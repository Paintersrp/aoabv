#!/usr/bin/env bash
# Deterministic checks for data catalog plumbing.
set -euo pipefail

py="python3"
tool="tools/datafetch/datafetch.py"
manifest="data/manifest/data_manifest.json"

echo "== validate (sample mode) =="
$py "$tool" validate --sample

echo "== plan (sample mode) =="
$py "$tool" plan --sample --out /tmp/plan1.json
$py "$tool" plan --sample --out /tmp/plan2.json
diff -u /tmp/plan1.json /tmp/plan2.json >/dev/null && echo "Plan is stable."

echo "== manifest (sample mode) =="
$py "$tool" manifest --sample --out "$manifest"

sha1=$(/usr/bin/env sha256sum "$manifest" | awk '{print $1}')
$py "$tool" manifest --sample --out "$manifest"
sha2=$(/usr/bin/env sha256sum "$manifest" | awk '{print $1}')

if [ "$sha1" != "$sha2" ]; then
  echo "✖ Manifest SHA changed across identical runs" >&2
  exit 1
fi
echo "Manifest SHA stable: $sha1"
