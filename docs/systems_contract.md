# Systems contract — simulation NDJSON

## Frame schema

Each NDJSON line emitted by `simd`/`simstep` serialises the following structure:

```json
{
  "t": 12,
  "diff": {
    "biome": {"r:42": 3},
    "water": {"r:42": 120},
    "soil":  {"r:42": -40},
    "hazards": [{"region": 42, "drought": 1200, "flood": 0}]
  },
  "highlights": [
    {"type": "hazard_flag", "region": 42, "info": {"kind": "drought", "level": 0.43}}
  ],
  "chronicle": ["Region 42 faces an extended dry spell."],
  "era_end": false
}
```

* `t` — Tick counter (`u64`).
* `diff` — Sparse update maps keyed by `"r:<index>"`. Values are integers (biome codes) or signed deltas (water/soil). Hazards emit full snapshots.
* `highlights` — Inspector hints. Every hazard highlight uses `{type:"hazard_flag", info:{kind, level}}`.
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
* The realised world stores `tick`, `seed`, `width`, `height`, and a `regions` array containing deterministic coordinates and climate state.

## Cause log schema

Cause codes are emitted as standalone NDJSON lines when using the `--cause-log` option of `simstep`:

```json
{"target": "region:42/water", "code": "drought_flag", "note": "level=1800"}
```

Codes must appear in [`docs/cause_codes.md`](cause_codes.md). When adding new fields to frames or seeds, update this contract file and bump the viewer accordingly.
