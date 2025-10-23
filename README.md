# Ages of a Borrowed Voice — Simulation Workspace

## Project overview

This workspace contains the deterministic Rust simulation stack for *Ages of a Borrowed Voice* and the accompanying Godot 4.5 viewer shell.

* `sim_core` — pure kernels, world model, diff reducer, IO contracts, and determinism guarantees.
* `simd` — long-lived daemon that streams NDJSON frames over WebSocket at `/stream`.
* `simstep` — batch runner that executes a fixed number of ticks and emits NDJSON to disk for golden comparisons.
* `viewer_godot` — Godot 4.5 project that attaches to the daemon and visualises the grid.

The NDJSON frame, cause code, and systems contracts that bind these components together are captured in [`/docs`](docs/). Recent kernel integrations extend the frame payload with:

* `world` metadata describing the fixed grid dimensions so the viewer can position region indices deterministically.
* New sparse `diff` maps for `insolation`, `tide_envelope`, and `elevation` alongside the existing `biome`, `soil`, and `water` deltas.

See [`/docs/systems_contract.md`](docs/systems_contract.md) for the authoritative schema.

## Toolchain

Rust **1.76.0** is pinned via [`rust-toolchain.toml`](rust-toolchain.toml). Install it with `rustup` (the Makefile assumes the toolchain already exists) and ensure `rustfmt`/`clippy` components are available. Godot **4.5** (with .NET support for C# scripting) is required to open the viewer project.

## Build and run commands

### Build the Rust binaries

```bash
cargo build -p simd -p simstep
# or: make build
```

### Run the streaming daemon (`simd`)

```bash
cargo run -p simd -- --seed-file ./testdata/seeds/seed_wet_equator.json --port 8080
# or: make simd
```

The daemon exposes a WebSocket endpoint at `ws://localhost:8080/stream`, emitting one NDJSON frame per line that matches the systems contract.

### Run the batch runner / regenerate golden runs (`simstep`)

```bash
cargo run -p simstep -- \
  --seed-file ./testdata/seeds/seed_wet_equator.json \
  --ticks 20 \
  --out ./target/tmp.ndjson
diff -u ./target/tmp.ndjson ./testdata/golden/seed_wet_equator.ndjson
# or: make golden
```

Use these commands whenever regenerating golden fixtures; include a brief note in commit messages describing why they changed. When verifying the new NDJSON fields, inspect the diff output for the expected `world`, `insolation`, `tide_envelope`, and `elevation` entries.

### Deterministic verification ledger

Once diffs look correct, record the canonical hash so the verification log stays reproducible:

```bash
shasum -a 256 ./testdata/golden/seed_wet_equator.ndjson
```

Log the resulting digest and the command invocation in [`docs/v0.0_verification.md`](docs/v0.0_verification.md) to keep the deterministic workflow auditable.

### Launch the Godot viewer

```bash
godot4 --path ./viewer_godot
# or open the folder in the Godot editor UI
```

With the daemon running, select the main scene (`Main.tscn`) and press **Play** to connect to `ws://localhost:8080/stream` and render live frames.

## Assumptions & scope

* Seeds in [`/testdata/seeds`](testdata/seeds/) describe world dimensions and generator parameters for climate/ecology kernels. Default fixtures include `seed_wet_equator.json` for golden runs and `seed_shard_continents.json` for broader worldgen experiments.
* Golden NDJSON outputs in [`/testdata/golden`](testdata/golden/) are authoritative references for regression testing.
* The viewer is observational only in v0.0—no gameplay UI or input loops beyond connecting to the stream.

## Determinism expectations

Simulation stages derive deterministic RNG substreams from `(seed, stage_id, tick)`. Identical seeds and tick counts must yield byte-identical NDJSON across runs and supported platforms. Water and soil stay within `0..=10000`, diffs are sparse and index-sorted, and highlights include typed payloads. When modifying kernels, update golden runs through `simstep` and document the rationale.

## Documentation

* [`/docs/systems_contract.md`](docs/systems_contract.md) — authoritative wire/data model for frames, seeds, and world state.
* [`/docs/cause_codes.md`](docs/cause_codes.md) — canonical list of cause codes emitted by the simulation.
* [`CHANGELOG.md`](CHANGELOG.md) — release notes; update this file with each tagged release.
* [`/docs/roadmap.md`](docs/roadmap.md) — release timeline, milestones, and deferred features.
* [`/docs/v0.0_verification.md`](docs/v0.0_verification.md) — verification ledger for v0.0 runs, including command history, NDJSON field checks, and deterministic hashes.

## Repository layout

```
.
├── Cargo.toml              # Workspace definition
├── crates
│   ├── sim_core            # Library crate (kernels, reducers, IO)
│   ├── simd                # Streaming daemon binary
│   └── simstep             # Batch runner binary
├── docs                    # Data contracts and roadmap
├── testdata                # Seeds and golden NDJSON runs
└── viewer_godot            # (reserved for the viewer project)
```

## Contributing

* Run `cargo fmt`, `cargo clippy -D warnings`, and `cargo test -p sim_core` (or simply `make check`) before sending a PR.
* Update `/docs/systems_contract.md` and `/docs/cause_codes.md` when changing the wire schema or cause list.
* Include a brief change note in commit messages when regenerating golden files.

## License

[MIT](LICENSE)
