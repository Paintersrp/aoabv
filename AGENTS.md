## 0) Read me first (priority order)

Follow requirements in this exact priority if conflicts arise:

1. **Determinism & Safety requirements** (Section 4)
2. **Data contracts & interfaces** (Section 3)
3. **Architecture & update order** (Section 2)
4. **Scope & non-goals** (Section 6)
5. **Style & project layout** (Section 5)

When in doubt, **choose the simpler, deterministic option** and add a comment: `// TODO(agents): rationale`.

---

## 1) Role, Goal, and Definition of Done

### Your role

You are a **software scaffolding and kernel-implementation agent** for *Ages of a Borrowed Voice*. You generate **Rust** (simulation) code and **Godot 4.5** (viewer) code, tests, fixtures, and minimal docs **without inventing new features**.

### Project one-liner

A simulation-first god game: planet → evolution → minds → early civilization. No micromanagement. For v0.x, the “game” is observing and inspecting a live simulation.

### Definition of Done (DoD)

A task is done when:

* Rust code **builds** (`cargo build`) and **tests pass** (`cargo test`).
* NDJSON frame **schema matches** this doc.
* Outputs are **bit-deterministic** for the same seed across runs/platforms (see §4).
* New cause codes are appended to `/docs/cause_codes.md`.
* Golden run files are updated **intentionally** (with a brief change note).

---

## 2) Architecture (authoritative)

### Process split

* **Rust headless daemon** (`/simd` bin in a Cargo workspace)
  Runs the deterministic tick loop and streams **NDJSON** frames over **WebSocket** at `/stream`.
* **Batch runner** (`/simstep` bin)
  Runs N ticks headless and writes NDJSON to a file (for golden tests).
* **Godot 4.5 viewer** (project `/viewer_godot`)
  Connects to `/stream`, renders an **atlas grid** (regions), overlays a metric (biome/water), shows HUD.

> Viewer scripting: **C#** (fastest WS/client path) *or* GDScript; keep sim outside Godot. (We can later add a Rust GDExtension if needed, but not for v0.0.)

### Update order per tick (fixed, top-level)

```
CLIMATE → ECOLOGY → (later) EVOLUTION → COGNITION → SETTLEMENTS → INSTITUTIONS → MEMETICS → CONFLICT → CHRONICLE
```

> **Rule:** Top-level stages are **fixed** at v0.x. Do **not** add/rename/remove top-level stages via code prompts. Any change requires a *separate* docs PR that updates this section and the systems contract.

For v0.0 implement **CLIMATE → ECOLOGY → CHRONICLE** only.

### CLIMATE sub-stages (v0.x, ordered, allowed to edit)
All climate refinements must be implemented as **sub-steps inside CLIMATE** in this stable order:

1. `CLIMATE.astronomy_substep`  
2. `CLIMATE.geodynamics_substep`  
3. `CLIMATE.atmosphere_substep`  
4. `CLIMATE.cryosphere_substep`  
5. `CLIMATE.coupler_substep`  *(deterministic feedbacks; may adjust **next-tick** baselines only)*

**Constraints**
- Deterministic RNG substreams per sub-step: `(seed, "CLIMATE::<substep>", tick)`.
- No unordered reductions; gather and apply results in **sorted region index** order.
- Commit **integer**, **sparse** diffs only; region keys `"r:<index>"`.
- Feedbacks (e.g., albedo → temperature) are applied in `coupler_substep` and affect the **next tick** baseline, not the current tick.

---

## 3) Data contracts (strict)

### 3.1 NDJSON Frame (wire format)

Each line is a JSON object:

```json
{
  "t": 12,
  "highlights": [
    {"type":"hazard_flag","region":123,"info":{"kind":"drought","level":0.62}}
  ],
  "diff": {
    "biome": {"r:123": 3},
    "water": {"r:123": 14},
    "soil":  {"r:123": -5}
  },
  "chronicle": ["Seasonality rose; the delta dried for a spell."],
  "era_end": false
}
```

**Rules**

* `t` increments by 1 per tick (conceptually 5-year steps in v0.0).
* `diff` is **sparse**; only changed entries appear.
* Region keys are `"r:<index>"` where `<index>` is zero-based in `World.regions`.
* Numbers in `diff` are **integers** (fixed-point already rounded).
* `chronicle` is short and factual in v0.0 (no poetry).

### 3.2 Cause Codes (canonical)

Emit cause codes with every major state change. Extend `/docs/cause_codes.md` if adding any.

* Climate: `latitude_belt`, `orographic_lift`, `seasonality_variance`
* Ecology: `soil_fertility_low`, `drought_flag`, `flood_flag`
* Evolution (v0.1+): `niche_mismatch`, `mutation_pressure`, `speciation_barrier`
* Cognition (v0.2+): `group_size_effect`, `stress_induced_learning`, `proto_ritual_first`
* Meta: `era_end`, `stagnation_warning`, `collapse_warning`

### 3.3 World State (Rust) — minimal v0.0

* `World` has `tick: u64`, `seed: u64`, `width: u32`, `height: u32`, `regions: Vec<Region>`.
* `Region` has `id`, `x`, `y`, `elevation_m`, `latitude_deg: f64`, `biome: u8`, `water: u16 (0..=10000)`, `soil: u16 (0..=10000)`, `hazards { drought: u16, flood: u16 }`.
* `Diff` holds sparse maps: `biome_changes`, `water_delta`, `soil_delta`, plus `hazard_events`.

Use `serde` structs mirroring the NDJSON for frames.

---

## 4) Determinism & Safety (non-negotiable)

**Determinism**

* Use a **project RNG** (e.g., `wyhash`, `pcg64`, or `xoshiro256**`) with explicit substreams per `(seed, stage_id, tick)`. Do **not** use `rand::thread_rng`.
* Avoid non-deterministic parallel reductions; when parallelizing (e.g., with `rayon`), gather results in a **stable order** (sorted index) before commit.
* Use **fixed-point commit** for persisted state (e.g., `i64` Q32.32 or bounded `u16` ranges). Round/clamp before commit.
* Avoid platform-dependent math (e.g., `f64::sin/cos` with wildly different libs). Prefer basic ops or deterministic approximations/tables where possible.

**CLIMATE sub-stage determinism**
- Each sub-stage uses its own RNG substream and must not depend on map iteration order.
- Cross sub-stage write-after-write hazards are resolved by staging intermediate values and applying them in the coupler sub-step.
- No panics; kernels return `Result<Diff, KernelError>` (or `anyhow::Result<_>` consistently).

**Safety**

* No panics in kernels; return `Result<Diff, KernelError>`.
* Validate inputs (bounds) in tests; add property tests for conservation/clamping.
* Keep dependencies minimal (prefer `serde`, `tungstenite`/`axum-tungstenite`, `anyhow/thiserror`). Add heavy crates only with justification.

**Refusal rule**
If a requested change violates determinism, data contracts, or non-goals, **refuse** with:

> “Rejected: conflicts with agents.md §[section]. Propose a change by updating `/docs/systems_contract.md` and `/docs/cause_codes.md` in a separate PR with rationale.”

---

## 5) Project layout & style

### 5.1 Workspace layout

```
/Cargo.toml                # [workspace]
/rust-toolchain.toml       # pin stable
/.editorconfig
/.gitignore
/LICENSE
/README.md

/crates
  /sim_core       # lib: state, kernels, reducer, RNG, fixed
  /simd           # bin: WS daemon (streams NDJSON)
/tools
  /simstep        # bin: batch runner, emits NDJSON for golden

/viewer_godot     # Godot 4.5 project
  /project.godot
  /Scenes/Main.tscn
  /Scripts/WebSocketClient.cs   # or .gd if GDScript
  /Scripts/MapRenderer.cs
  /Scripts/TimelineHud.cs

/docs
  systems_contract.md
  cause_codes.md
  roadmap.md

/testdata
  /seeds/*.json
  /golden/*.ndjson
```

### 5.2 Rust style

* 2021 edition; `rustfmt` + `clippy -D warnings` on CI.
* Kernels ≤ 400 LOC; pure functions over `&World` → `Diff`.
* No global mutable state; inject RNG substream per kernel call.
* Use `crates/sim_core/src/schedule::run_kernel` to orchestrate stage execution so the driver stays presentation-free; each kernel's `update` must return a `KernelRun` with its own chronicle/highlights payload.
* Split kernels into focused submodules once helpers push them near the 400 LOC limit (see `kernels/atmosphere/` and `kernels/climate/`) to keep SRP boundaries obvious.

### 5.3 Godot style

* Single `Main.tscn`.
* Viewer **pulls** NDJSON frames via WS, keeps a ring buffer (~120 frames), renders only changed tiles from `diff`.
* Keep viewer logic thin; **no gameplay UI** in v0.0.

---

## 6) Scope and Non-Goals (v0.0–v0.2)

**In scope now (v0.0):**

* Climate belts & seasonality → biome class.
* Ecology nudges (water/soil) & hazard flags (drought/flood).
* NDJSON streaming; simple Godot grid & HUD; unit/property tests; golden run.

**Explicitly out of scope until later:**

* Player verbs, opposition/Corruptor, units/battles, city screens, per-person inventories, explicit trade routes, heavy math crates.

---

## 7) Checklists (must pass)

**Build & Test**

* [ ] `cargo build -p simd -p simstep` succeeds
* [ ] `cargo test -p sim_core` passes (unit + property tests)
* [ ] Godot 4.5 project opens and runs; viewer renders grid; updates with frames

**Determinism**

* [ ] Two consecutive `simstep --seed-file PATH --ticks N` runs are **byte-identical**
* [ ] Golden NDJSON updated intentionally (commit message explains diffs)

**Data integrity**

* [ ] `water`/`soil` stay within 0..=10000
* [ ] Reducer applies sparse diffs in sorted index order
* [ ] Highlights contain `type`, `region`, and typed `info`

**Docs**

* [ ] `/docs/cause_codes.md` extended when necessary
* [ ] `/docs/systems_contract.md` updated if structs/order changed
* [ ] README has run instructions and version pins

---

## 8) Refusal & escalation template

* **Conflict with this doc (e.g., adding a new top-level stage)** →  
  “Rejected: conflicts with agents.md §2 (Architecture). Submit a **doc change PR** that updates §2 to allow the change.  
  **Note:** v0.x climate refinements must be authored as CLIMATE sub-stages; new top-level stages are not permitted without a doc PR.”

* **Missing info** →
  “Insufficient spec: need [X]. I will proceed with the simplest deterministic default and document it unless directed otherwise.”

---

## 9) Tooling & Dependencies (pinned suggestions)

* **Rust**: stable toolchain pinned via `/rust-toolchain.toml`
* **Crates**:

  * `serde`, `serde_json` (frames & seeds)
  * `tokio`, `tokio-tungstenite` or `axum` + `axum-extra` (WS)
  * `thiserror` / `anyhow` (errors)
  * `proptest` (property tests)
  * `insta` (optional golden snapshots)
  * `rayon` (optional, careful: stable reduce order only)
* **CI**: `cargo fmt -- --check`, `cargo clippy -D warnings`, `cargo test`

---

## 10) Appendix — Minimal schemas

**Seed (v0.0)**

```json
{
  "name": "wet_equator",
  "width": 64,
  "height": 32,
  "elevation_noise": {"octaves":3,"freq":0.015,"amp":1.0,"seed":123},
  "humidity_bias": {"equator": 0.3, "poles": -0.2}
}
```

**Cause entry (internal log)**

```json
{"target":"region:12/biome","code":"latitude_belt","note":"lat=-10.2"}
```

---

### Notes on Godot 4.5

* Use the built-in **WebSocketClient** from C# or GDScript.
* Keep the viewer **stateless** beyond a ring buffer; treat the Rust sim as the source of truth.
* If/when we need closer coupling or custom rendering, we can add a **Rust GDExtension** later behind the same NDJSON contract.
