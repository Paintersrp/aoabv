#!/usr/bin/env python3
"""
AoABV Datafetch v0.1 — catalog validator and deterministic manifest builder.

- Parses YAML code blocks from docs/data_catalog.md
- Validates required fields
- Emits data/manifest/data_manifest.json with stable sort order
- Supports "plan" (dry-run) and "manifest" modes
- "--sample" restricts to entries where sample == true
- Pure stdlib; no network I/O performed in v0.1

Determinism:
- Output JSON uses sorted keys and a stable dataset ordering by id.
- Manifest includes a SHA256 over normalized metadata for traceability.

Usage:
  python tools/datafetch/datafetch.py validate [--sample]
  python tools/datafetch/datafetch.py plan [--sample] --out -
  python tools/datafetch/datafetch.py manifest [--sample] --out data/manifest/data_manifest.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from typing import Any, Dict, List

CATALOG_PATH = "docs/data_catalog.md"

YAML_RE = re.compile(r"```yaml(.*?)```", re.DOTALL | re.MULTILINE)


# very small YAML parser for key: value and simple lists; avoids pyyaml dependency
def parse_simple_yaml_block(text: str) -> Dict[str, Any]:
    data: Dict[str, Any] = {}
    current_list_key: str | None = None
    in_access = False

    for raw in text.strip().splitlines():
        line = raw.rstrip("\n")
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue

        if re.match(r"^\s+-\s", line):
            if current_list_key is None:
                raise ValueError("List item without an active key")
            item = line.strip()[1:].strip()
            data[current_list_key].append(_parse_value(item))
            continue

        if in_access and line.startswith("  "):
            if ":" not in stripped:
                raise ValueError(f"Malformed access line: {line}")
            key, val = stripped.split(":", 1)
            data.setdefault("access", {})[key.strip()] = _parse_value(val.strip())
            continue

        if ":" not in stripped:
            raise ValueError(f"Cannot parse line: {line}")

        key, val = stripped.split(":", 1)
        key = key.strip()
        val = val.strip()

        if val == "":
            if key == "access":
                data[key] = {}
                in_access = True
                current_list_key = None
            else:
                data[key] = []
                current_list_key = key
                in_access = False
            continue

        parsed = _parse_value(val)
        if isinstance(parsed, list):
            data[key] = parsed
        else:
            data[key] = parsed
        current_list_key = None
        in_access = False

    return data


def _parse_value(val: str) -> Any:
    if val.startswith("[") and val.endswith("]"):
        inner = val[1:-1].strip()
        if not inner:
            return []
        items = [item.strip() for item in inner.split(",")]
        return [_parse_value(item) for item in items]
    if (val.startswith("\"") and val.endswith("\"")) or (val.startswith("'") and val.endswith("'")):
        val = val[1:-1]
    lower = val.lower()
    if lower == "true":
        return True
    if lower == "false":
        return False
    return val


REQUIRED_FIELDS = [
    "id",
    "provider",
    "product",
    "version",
    "format",
    "access",
    "spatial_resolution",
    "temporal_coverage",
    "variables",
    "license",
    "sample",
    "notes",
]

ACCESS_FIELDS = {"method", "url", "auth"}


def load_catalog(sample_only: bool) -> List[Dict[str, Any]]:
    with open(CATALOG_PATH, "r", encoding="utf-8") as f:
        md = f.read()
    blocks = YAML_RE.findall(md)
    datasets: List[Dict[str, Any]] = []
    for block in blocks:
        entry = parse_simple_yaml_block(block)
        for field in REQUIRED_FIELDS:
            if field not in entry:
                raise SystemExit(f"Dataset missing required field '{field}': {entry.get('id', '<unknown>')}")
        access = entry.get("access")
        if not isinstance(access, dict) or ACCESS_FIELDS - set(access):
            missing = ", ".join(sorted(ACCESS_FIELDS - set(access or {})))
            raise SystemExit(f"Dataset '{entry.get('id', '<unknown>')}' missing access field(s): {missing}")
        if not isinstance(entry.get("format"), list) or not entry["format"]:
            raise SystemExit(f"Dataset '{entry['id']}' must define at least one format")
        if not isinstance(entry.get("variables"), list) or not entry["variables"]:
            raise SystemExit(f"Dataset '{entry['id']}' must define at least one variable")

        entry["id"] = str(entry["id"]).strip()
        if sample_only and not entry.get("sample", False):
            continue
        datasets.append(entry)

    datasets.sort(key=lambda d: d["id"])
    return datasets


def normalized_metadata_bytes(dataset: Dict[str, Any]) -> bytes:
    keep_keys = [
        "id",
        "provider",
        "product",
        "version",
        "format",
        "access",
        "spatial_resolution",
        "temporal_coverage",
        "variables",
        "license",
        "sample",
        "notes",
    ]
    keep = {key: dataset[key] for key in keep_keys if key in dataset}
    as_json = json.dumps(keep, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
    return as_json.encode("utf-8")


def dataset_hash(dataset: Dict[str, Any]) -> str:
    return hashlib.sha256(normalized_metadata_bytes(dataset)).hexdigest()


def cmd_validate(args: argparse.Namespace) -> None:
    datasets = load_catalog(args.sample)
    ids = [d["id"] for d in datasets]
    if len(ids) != len(set(ids)):
        raise SystemExit("Duplicate dataset ids detected.")
    for dataset_id in ids:
        if not re.match(r"^[a-z0-9][a-z0-9-]*$", dataset_id):
            raise SystemExit(f"Invalid id format: {dataset_id}")
    print(
        f"OK: {len(datasets)} dataset(s) validated{' (sample mode)' if args.sample else ''}."
    )


def cmd_plan(args: argparse.Namespace) -> None:
    datasets = load_catalog(args.sample)
    plan = []
    for entry in datasets:
        plan.append(
            {
                "id": entry["id"],
                "would_fetch": {
                    "method": entry["access"]["method"],
                    "url": entry["access"]["url"],
                    "auth": entry["access"]["auth"],
                },
                "hash": dataset_hash(entry),
            }
        )
    out_stream = sys.stdout if args.out == "-" else open(args.out, "w", encoding="utf-8")
    try:
        json.dump(plan, out_stream, indent=2, sort_keys=True, ensure_ascii=False)
        if out_stream is sys.stdout:
            out_stream.write("\n")
        else:
            print(f"Wrote plan → {args.out}")
    finally:
        if out_stream is not sys.stdout:
            out_stream.close()


def cmd_manifest(args: argparse.Namespace) -> None:
    datasets = load_catalog(args.sample)
    manifest = {
        "version": "0.1",
        "generated_by": "aoabv-datafetch-v0.1",
        "sample_mode": bool(args.sample),
        "datasets": [
            {
                "id": entry["id"],
                "provider": entry["provider"],
                "product": entry["product"],
                "version": entry["version"],
                "format": entry["format"],
                "access": entry["access"],
                "spatial_resolution": entry["spatial_resolution"],
                "temporal_coverage": entry["temporal_coverage"],
                "variables": entry["variables"],
                "license": entry["license"],
                "sample": entry["sample"],
                "notes": entry.get("notes", ""),
                "meta_sha256": dataset_hash(entry),
            }
            for entry in datasets
        ],
    }
    manifest["datasets"].sort(key=lambda item: item["id"])
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as fh:
        json.dump(manifest, fh, indent=2, sort_keys=True, ensure_ascii=False)
        fh.write("\n")
    print(f"Wrote manifest → {args.out}")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate_parser = subparsers.add_parser("validate", help="validate catalog entries")
    validate_parser.add_argument("--sample", action="store_true")
    validate_parser.set_defaults(func=cmd_validate)

    plan_parser = subparsers.add_parser("plan", help="dry-run plan (no network)")
    plan_parser.add_argument("--sample", action="store_true")
    plan_parser.add_argument("--out", default="-", help="'-' for stdout or a file path")
    plan_parser.set_defaults(func=cmd_plan)

    manifest_parser = subparsers.add_parser("manifest", help="emit deterministic manifest json")
    manifest_parser.add_argument("--sample", action="store_true")
    manifest_parser.add_argument("--out", default="data/manifest/data_manifest.json")
    manifest_parser.set_defaults(func=cmd_manifest)

    return parser


def main() -> None:
    parser = build_parser()
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
