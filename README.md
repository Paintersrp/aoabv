# Ages of a Borrowed Voice — Simulation Workspace

This repository hosts the deterministic Rust simulation kernels for *Ages of a Borrowed Voice*. The workspace is organised as a triad of crates:

* `sim_core` — pure kernels, world model, diff reducer, and IO contracts.
* `simd` — long-lived daemon that streams NDJSON frames over WebSocket.
* `simstep` — batch runner that executes a fixed number of ticks and emits NDJSON to disk for golden tests.

The viewer (Godot 4.5) will connect to the daemon and consume the `sim_core` frame schema. The data contract for NDJSON frames, seeds, and cause codes is documented in [`/docs`](docs/).

## Getting started

1. Install the pinned Rust toolchain (`rustup show active-toolchain` should report `1.76.0`).
2. Fetch dependencies and build:
   ```bash
   cargo build -p simd -p simstep
   # or: make build
   ```
3. Run the batch runner to reproduce the golden fixtures:
   ```bash
   cargo run -p simstep -- --seed ./testdata/seeds/wet_equator.json --ticks 8 --out ./target/tmp.ndjson
   diff -u ./target/tmp.ndjson ./testdata/golden/wet_equator_8ticks.ndjson
   # or: make golden
   ```
4. Start the streaming daemon:
   ```bash
   cargo run -p simd -- --seed ./testdata/seeds/wet_equator.json --port 8787
   # or: make simd
   ```
   The daemon serves a WebSocket endpoint at `ws://localhost:8787/stream`. Each message is a single NDJSON line conforming to §3 of `AGENTS.md`.

## Determinism contract

All kernels derive deterministic substreams from the world seed, tick, and stage identifier. Given identical inputs the NDJSON output is byte-for-byte identical across runs and platforms. Property tests enforce water/soil bounds and clamping behaviour, and the golden fixtures are regenerated exclusively through `simstep`.

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
