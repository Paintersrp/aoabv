# v0.2 Validation (Earth-like Sanity Checks)

This document explains the lightweight checks used to finalise v0.2. The flow avoids large
external datasets by validating **global means** and simple distribution statistics against
broad Earth-like ranges.

## Metrics & Units
- `temp_c_mean`: derived from `temp` (tenths °C) → °C.
- `albedo_mean`: derived from `albedo` (milli) → fraction [0,1].
- `humidity_pct_mean`: derived from `humidity` (tenths) → percent [0,100].
- `diag_energy_abs_mean_tenths`: absolute mean of `diag_energy` (tenths). Target is near-zero drift.

Precipitation is recorded in native per-tick units and reported for inspection. It is not yet
part of the v0.2 pass/fail criteria.

## Targets
See `data/reference/targets_v0_2.csv`. These bands are intentionally broad to catch gross biases
without requiring reanalysis datasets.

## How it works
- Run `simstep` with `--emit-metrics <path>`. The runner writes one NDJSON line per tick:
  `{"t": <tick>, "global": {"temp_c": ..., "albedo": ..., "humidity_pct": ..., "precip_native": ..., "diag_energy_tenths": ...}}`
- `tools/validate/validate_v0_2.py`:
  - discards the first `--skip-initial` ticks (spin-up),
  - computes global means and a few percentiles,
  - evaluates pass/fail against the CSV targets.

## Commands
```bash
make v0_2_validate
```

## Notes

* For v0.2 we use **equal-weight** means (each region counts equally). We will introduce
  **area weighting** once grid geometry is formalised in `/docs/systems_contract.md`.
* Precipitation is reported in native per-tick units and not enforced in v0.2 due to
  unknown tick duration. Thresholds will be added when the temporal cadence is standardised.
