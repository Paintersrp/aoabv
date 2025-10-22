# Simulation agent notes

This repository contains only the deterministic simulation stack. The viewer lives in `viewer_godot/` and consumes frames from the streaming daemon (`simd`).

* `sim_core` owns the data contracts, kernels, and reducers. All kernels must be pure functions of the world snapshot and RNG substream.
* `simd` hosts a WebSocket endpoint at `/stream` and emits NDJSON lines one per tick.
* `simstep` is the batch runner used to generate golden NDJSON fixtures for regression tests.

## Quick commands

* `cargo fmt` — format Rust code.
* `cargo clippy -D warnings` — lint and keep deterministic APIs honest.
* `cargo test -p sim_core` — run unit + property tests.
* `cargo run -p simstep -- --seed-file ./testdata/seeds/run_seed_wet_equator.json --ticks 120 --out ./target/tmp.ndjson` — reproduce the golden run.

Ensure that every new cause code is reflected in [`docs/cause_codes.md`](cause_codes.md) and that wire changes are documented in [`docs/systems_contract.md`](systems_contract.md).
