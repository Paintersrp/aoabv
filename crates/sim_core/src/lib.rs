pub mod cause;
pub mod diff;
pub mod fixed;
pub mod io;
pub mod kernels;
pub mod reduce;
pub mod rng;
pub mod schedule;
pub mod world;

use anyhow::{ensure, Result};
use diff::Diff;
use io::frame::Highlight;
use kernels::{astronomy, atmosphere, climate, cryosphere, ecology, geodynamics};
use reduce::apply;
use rng::{stream_label, Stream};
use schedule::run_kernel;
use world::World;

/// Execute a single deterministic simulation tick.
///
/// This function orchestrates the kernel update order and commits their diffs to the
/// provided [`World`]. The returned tuple captures all changes applied during the
/// tick alongside the chronicle snippets and highlights surfaced by the kernels.
pub fn tick_once(
    world: &mut World,
    seed: u64,
    tick: u64,
) -> Result<(Diff, Vec<String>, Vec<Highlight>)> {
    ensure!(
        tick == world.tick + 1,
        "tick_once called with out-of-order tick: current={} requested={}",
        world.tick,
        tick
    );

    let mut aggregate_diff = Diff::default();
    let mut chronicle = Vec::new();
    let mut highlights = Vec::new();

    let climate_stage_rng = Stream::from(seed, climate::STAGE, tick);

    // Astronomy kernel establishes irradiance and tide envelopes.
    let astronomy_run = run_kernel(
        world,
        &mut aggregate_diff,
        &climate_stage_rng,
        astronomy::STAGE,
        |world, rng| astronomy::update(&*world, rng),
    )?;
    chronicle.extend(astronomy_run.chronicle);
    highlights.extend(astronomy_run.highlights);

    // Geodynamics kernel adjusts topography before climate updates.
    let geodynamics_run = run_kernel(
        world,
        &mut aggregate_diff,
        &climate_stage_rng,
        geodynamics::STAGE,
        |world, rng| geodynamics::update(&*world, rng),
    )?;
    chronicle.extend(geodynamics_run.chronicle);
    highlights.extend(geodynamics_run.highlights);

    // Atmospheric energy balance precedes climate classification.
    let atmosphere_run = run_kernel(
        world,
        &mut aggregate_diff,
        &climate_stage_rng,
        atmosphere::STAGE,
        |world, rng| atmosphere::update(&*world, rng),
    )?;
    if !atmosphere_run.chronicle.is_empty() {
        chronicle.push("Hadley belt drifted northward under seasonal tilt.".to_string());
    }
    highlights.extend(atmosphere_run.highlights);

    let cryosphere_run = run_kernel(
        world,
        &mut aggregate_diff,
        &climate_stage_rng,
        cryosphere::STAGE,
        |world, rng| cryosphere::update(world, rng),
    )?;
    chronicle.extend(cryosphere_run.chronicle);
    highlights.extend(cryosphere_run.highlights);

    let albedo_reconcile_diff = climate::albedo_reconcile(world)?;
    aggregate_diff.merge(&albedo_reconcile_diff);
    apply(world, albedo_reconcile_diff);

    let climate_run = run_kernel(
        world,
        &mut aggregate_diff,
        &climate_stage_rng,
        climate::CORE_STAGE,
        |world, rng| climate::update(&*world, rng),
    )?;
    chronicle.extend(climate_run.chronicle);
    highlights.extend(climate_run.highlights);

    // Ecology kernel uses the climate-updated world state.
    let ecology_run = run_kernel(
        world,
        &mut aggregate_diff,
        &climate_stage_rng,
        ecology::STAGE,
        |world, rng| ecology::update(&*world, rng),
    )?;
    chronicle.extend(ecology_run.chronicle);
    highlights.extend(ecology_run.highlights);

    // Chronicle stream reserved for downstream narrative kernels.
    let mut chronicle_rng = climate_stage_rng.derive(stream_label("kernel:chronicle"));
    let _ = chronicle_rng.next_u64();

    world.tick = tick;

    Ok((aggregate_diff, chronicle, highlights))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::seed::{build_world, Seed};

    #[test]
    fn tick_advances_world() {
        let seed_json = r#"{
            "name": "test",
            "width": 2,
            "height": 1,
            "elevation_noise": {"octaves": 1, "freq": 0.1, "amp": 1.0, "seed": 42},
            "humidity_bias": {"equator": 0.2, "poles": -0.2}
        }"#;
        let seed: Seed = serde_json::from_str(seed_json).unwrap();
        let mut world = build_world(&seed, Some(777));
        let prev_tick = world.tick;
        let next_tick = prev_tick + 1;
        let seed = world.seed;
        let (_diff, _chronicle, _highlights) = tick_once(&mut world, seed, next_tick).unwrap();
        assert_eq!(world.tick, next_tick);
    }
}
