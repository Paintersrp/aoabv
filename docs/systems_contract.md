# Systems contract — simulation NDJSON

## Frame schema

Each NDJSON line emitted by `simd`/`simstep` serialises the following structure:

```json
{
  "t": 12,
  "world": {"width": 64, "height": 32},
  "diff": {
    "biome": {"r:42": 3},
    "elevation": {"r:42": 120},
    "insolation": {"r:42": 1380},
    "temp": {"r:42": 184},
    "precip": {"r:42": 3200},
    "humidity": {"r:42": 540},
    "soil": {"r:42": -40},
    "tide_envelope": {"r:42": -35},
    "albedo": {"r:42": 910},
    "freshwater_flux": {"r:42": 240},
    "ice_mass": {"r:42": 12500},
    "water": {"r:42": 120}
  },
  "diagnostics": {"energy_balance": -1, "albedo_anomaly_milli": -45},
  "highlights": [
    {"type": "hazard_flag", "region": 42, "info": {"kind": "drought", "level": 0.43}}
  ],
  "chronicle": ["Region 42 faces an extended dry spell."],
  "era_end": false
}
```

* `t` — Tick counter (`u64`).
* `world` — Snapshot of viewer metadata. Width/height describe the fixed grid dimensions for interpreting region indices.
* `diff` — Sparse update maps keyed by `"r:<index>"`. Values are integers (biome codes) or signed scalars and deltas (`water`, `soil`, `insolation`, `tide_envelope`, `elevation`, `temp`, `precip`). No additional keys are permitted.
  * `water` / `soil` — Signed deltas against the current meters (range -10_000..=10_000 before clamping). Values are applied using the clamping helpers in [`fixed.rs`](../crates/sim_core/src/fixed.rs).
  * `insolation` — Instantaneous top-of-atmosphere irradiance in watts per square metre, integer scaled (0..=2_000 for v0.0 prototypes).
  * `tide_envelope` — Deterministic tide offset envelope, signed millimetres relative to mean sea level (-500..=500).
  * `elevation` — Absolute terrain height in metres stored as `i32`. Initial seeds clamp sampled terrain to 0..=3_000 m, but kernels may push values negative for bathymetry adjustments.
  * `temp` — Deterministic air temperature in tenths of °C (-500..=500) derived from energy balance each tick.
  * `precip` — Total precipitation per tick in whole millimetres (0..=5_000) after humidity/orographic adjustments.
  * `precip_extreme` — Rolling precipitation anomaly index expressed in whole millimetres (positive for spikes, negative for lulls).
  * `albedo` — Snow/ice albedo in milli-units (100..=1_000). Values represent instantaneous surface reflectivity.
  * `freshwater_flux` — Meltwater discharge in tenths of millimetres per tick (0..=2_000).
  * `permafrost_active` — Active-layer depth in centimetres (signed, tenths) used for permafrost accounting.
  * `melt_pulse` — Cryosphere melt pulses in tenths of millimetres (signed, zero omitted when quiescent).
  * `heatwave_idx` — Rolling heatwave severity index in tenths of °C anomaly (zero omitted when stable).
  * `humidity` — Instantaneous atmospheric humidity in tenths of a percent (0..=1_000).
  * `ice_mass` — Regional cryosphere storage in kilotons (integer, 0..≈200_000).
  * `diag_climate` — Global climate diagnostic vector; currently emits a single `r:0` entry representing the composite stability index in tenths.
* `highlights` — Inspector hints. Hazard insight is surfaced exclusively via `{type:"hazard_flag", info:{kind, level}}` entries.
* `chronicle` — Ordered list of short factual sentences per tick.
* `era_end` — `true` once the long-term arc for the seed finishes (unused in v0.0).

When present, `diagnostics` captures global climate bookkeeping for the current tick:

* `energy_balance` — Mean temperature baseline adjustment (tenths of °C) scheduled for the next tick.
* `albedo_anomaly_milli` — Mean albedo anomaly in milli-units across regions that triggered reconciliation.
* `diag_climate` entries remain in the `diff` block; they are **not** duplicated here.

### Command checklist

Run the following workspace commands before submitting a change:

1. `cargo build -p simd -p simstep`
2. `cargo test -p sim_core`
3. `cargo run -p simstep -- --seed-file ./testdata/seeds/seed_wet_equator.json --ticks 20 --out ./target/tmp.ndjson`

## Seed schema

```json
{
  "name": "wet_equator",
  "width": 64,
  "height": 32,
  "elevation_noise": {"octaves": 3, "freq": 0.015, "amp": 1.0, "seed": 123},
  "humidity_bias": {"equator": 0.3, "poles": -0.2}
}
```

* `freq` influences the pseudo-noise frequency (currently informational only but preserved for forward compatibility).
* Repository fixtures `seed_wet_equator.json` and `seed_shard_continents.json` follow this schema.
* The realised world stores `tick`, `seed`, `width`, `height`, and a `regions` array containing deterministic coordinates and climate state (biome, water, soil, temperature, precipitation, albedo, freshwater flux, hazards).

## Cause log schema

Cause codes are emitted as standalone NDJSON lines when using the `--cause-log` option of `simstep`:

```json
{"target": "region:42/water", "code": "drought_flag", "note": "level=1800"}
```

Codes must appear in [`docs/cause_codes.md`](cause_codes.md). When adding new fields to frames or seeds, update this contract file and bump the viewer accordingly.
