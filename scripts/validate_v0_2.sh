#!/usr/bin/env bash
# v0.2 verification run (no CI hook). Deterministic, offline.
set -euo pipefail

OUT_METRICS="${OUT_METRICS:-/tmp/aoabv_v02_metrics.ndjson}"
OUT_FRAMES="${OUT_FRAMES:-/dev/null}"
ASSERT_OUT="${ASSERT_OUT:-/tmp/v02_assertions.json}"
SEED="${SEED:-42}"
TICKS="${TICKS:-3600}"
SKIP_INITIAL="${SKIP_INITIAL:-360}"

cleanup() {
  if [[ -n "${OUT_FRAMES:-}" && "$OUT_FRAMES" != "/dev/null" ]]; then
    rm -f "$OUT_FRAMES"
  fi
}
trap cleanup EXIT

echo "== simstep (seed=$SEED ticks=$TICKS) =="
cargo run --release -p simstep -- --seed-file ./testdata/seeds/seed_wet_equator.json --seed "$SEED" --ticks "$TICKS" --out "$OUT_FRAMES" --emit-metrics "$OUT_METRICS"

echo "== validate v0.2 =="
python3 tools/validate/validate_v0_2.py \
  --metrics "$OUT_METRICS" \
  --targets data/reference/targets_v0_2.csv \
  --skip-initial "$SKIP_INITIAL" \
  --assertions-out "$ASSERT_OUT"

echo "Assertions written to $ASSERT_OUT"
