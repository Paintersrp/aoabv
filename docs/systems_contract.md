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
    "soil": {"r:42": -40},
    "tide_envelope": {"r:42": -35},
    "water": {"r:42": 120}
  },
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
* `highlights` — Inspector hints. Hazard insight is surfaced exclusively via `{type:"hazard_flag", info:{kind, level}}` entries.
* `chronicle` — Ordered list of short factual sentences per tick.
* `era_end` — `true` once the long-term arc for the seed finishes (unused in v0.0).

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
* The realised world stores `tick`, `seed`, `width`, `height`, and a `regions` array containing deterministic coordinates and climate state (biome, water, soil, temperature, precipitation, hazards).

## Cause log schema

Cause codes are emitted as standalone NDJSON lines when using the `--cause-log` option of `simstep`:

```json
{"target": "region:42/water", "code": "drought_flag", "note": "level=1800"}
```

Codes must appear in [`docs/cause_codes.md`](cause_codes.md). When adding new fields to frames or seeds, update this contract file and bump the viewer accordingly.
