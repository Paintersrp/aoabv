#!/usr/bin/env python3
"""
AoABV v0.2 validator: lightweight checks against Earth-like ranges.

Inputs:
  - metrics NDJSON from simstep: one JSON per line with keys:
      {"t": int, "global": {
          "temp_c": float,
          "albedo": float,
          "humidity_pct": float,
          "precip_native": float,
          "diag_energy_tenths": float
      }}
  - targets CSV with rows: metric,min,max,notes

Outputs:
  - metrics summary printed to stdout
  - assertions JSON written to --assertions-out (list of {metric, pass, value, min, max, notes})

No external deps. Deterministic: stable sorting of ticks and keys.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from typing import Any, Dict, List


def load_targets(path: str) -> Dict[str, Dict[str, float]]:
    targets: Dict[str, Dict[str, float]] = {}
    with open(path, "r", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            metric = row.get("metric", "").strip()
            if not metric or metric.startswith("#"):
                continue
            targets[metric] = {
                "min": float(row["min"]),
                "max": float(row["max"]),
                "notes": row.get("notes", ""),
            }
    return targets


def load_metrics_ndjson(path: str, skip_initial: int) -> List[Dict[str, Any]]:
    entries: List[Dict[str, Any]] = []
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            entries.append(json.loads(line))
    entries.sort(key=lambda obj: obj.get("t", 0))
    if skip_initial > 0:
        entries = [obj for obj in entries if obj.get("t", 0) >= skip_initial]
    return entries


def mean(values: List[float]) -> float:
    if not values:
        return math.nan
    return sum(values) / float(len(values))


def percentile(values: List[float], q: float) -> float:
    if not values:
        return math.nan
    ordered = sorted(values)
    index = int(round((q / 100.0) * (len(ordered) - 1)))
    index = max(0, min(len(ordered) - 1, index))
    return ordered[index]


def summarise(metrics: List[Dict[str, Any]]) -> Dict[str, float]:
    temps = [entry["global"]["temp_c"] for entry in metrics if "global" in entry]
    albedo = [entry["global"]["albedo"] for entry in metrics if "global" in entry]
    humidity = [entry["global"]["humidity_pct"] for entry in metrics if "global" in entry]
    precip = [entry["global"]["precip_native"] for entry in metrics if "global" in entry]
    energy = [entry["global"]["diag_energy_tenths"] for entry in metrics if "global" in entry]

    return {
        "global.temp_c_mean": mean(temps),
        "global.albedo_mean": mean(albedo),
        "global.humidity_pct_mean": mean(humidity),
        "global.precip_native_mean": mean(precip),
        "global.diag_energy_abs_mean_tenths": mean([abs(value) for value in energy]),
        "global.temp_c_p95": percentile(temps, 95),
        "global.precip_native_p99": percentile(precip, 99),
    }


def evaluate(summary: Dict[str, float], targets: Dict[str, Dict[str, float]]) -> List[Dict[str, Any]]:
    results: List[Dict[str, Any]] = []
    for metric in sorted(targets.keys()):
        spec = targets[metric]
        value = summary.get(metric, math.nan)
        passed = spec["min"] <= value <= spec["max"]
        results.append(
            {
                "metric": metric,
                "pass": bool(passed),
                "value": value,
                "min": spec["min"],
                "max": spec["max"],
                "notes": spec.get("notes", ""),
            }
        )
    return results


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--metrics", required=True, help="metrics NDJSON from simstep --emit-metrics")
    parser.add_argument("--targets", required=True, help="targets CSV (metric,min,max,notes)")
    parser.add_argument(
        "--skip-initial",
        type=int,
        default=0,
        help="skip ticks below this threshold when computing statistics",
    )
    parser.add_argument("--assertions-out", required=True)
    args = parser.parse_args()

    targets = load_targets(args.targets)
    metrics = load_metrics_ndjson(args.metrics, args.skip_initial)
    if not metrics:
        print("No metrics found.", file=sys.stderr)
        sys.exit(2)

    summary = summarise(metrics)
    for key in sorted(summary.keys()):
        value = summary[key]
        if math.isnan(value):
            print(f"{key}=nan")
        else:
            print(f"{key}={value:.6f}")

    assertions = evaluate(summary, targets)
    with open(args.assertions_out, "w", encoding="utf-8") as handle:
        json.dump(assertions, handle, indent=2, sort_keys=True)

    failures = [entry for entry in assertions if not entry["pass"]]
    if failures:
        print("\nValidation FAIL:", file=sys.stderr)
        for entry in failures:
            value = entry["value"]
            print(
                f" - {entry['metric']}: {value:.6f} not in [{entry['min']}, {entry['max']}]",
                file=sys.stderr,
            )
        sys.exit(1)

    print("\nValidation PASS")
    sys.exit(0)


if __name__ == "__main__":
    main()
