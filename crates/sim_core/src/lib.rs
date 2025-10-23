pub mod cause;
pub mod diff;
pub mod fixed;
pub mod io;
pub mod kernels;
pub mod reduce;
pub mod rng;
pub mod world;

use anyhow::{ensure, Result};
use diff::Diff;
use fixed::WATER_MAX;
use io::frame::Highlight;
use kernels::{
    astronomy, climate,
    ecology::{self, DROUGHT_ALERT_THRESHOLD, FLOOD_ALERT_THRESHOLD},
};
use reduce::apply;
use rng::Stream;
use world::World;

/// Execute a single deterministic simulation tick.
///
/// This function orchestrates the kernel update order and commits their diffs to the
/// provided [`World`]. The returned [`Diff`] captures all changes applied during the
/// tick, while the [`Vec<String>`] contains chronicle notes summarising notable events.
pub fn tick_once(world: &mut World, seed: u64, tick: u64) -> Result<(Diff, Vec<String>)> {
    ensure!(
        tick == world.tick + 1,
        "tick_once called with out-of-order tick: current={} requested={}",
        world.tick,
        tick
    );

    let mut aggregate_diff = Diff::default();
    let mut chronicle = Vec::new();

    // Astronomy kernel establishes irradiance and tide envelopes.
    let mut astronomy_rng = Stream::from(seed, astronomy::STAGE, tick);
    let (astronomy_diff, mut astronomy_chronicle) = astronomy::update(world, &mut astronomy_rng)?;
    chronicle.append(&mut astronomy_chronicle);
    aggregate_diff.merge(&astronomy_diff);
    apply(world, astronomy_diff);

    // Climate kernel.
    let mut climate_rng = Stream::from(seed, climate::STAGE, tick);
    let climate_diff = climate::update(world, &mut climate_rng)?;
    for change in &climate_diff.biome {
        if let Some(region) = world.regions.get(change.region as usize) {
            chronicle.push(format!(
                "Region {} shifted biome to {}",
                region.id, change.biome
            ));
        }
    }
    aggregate_diff.merge(&climate_diff);
    apply(world, climate_diff);

    // Ecology kernel uses the climate-updated world state.
    let mut ecology_rng = Stream::from(seed, ecology::STAGE, tick);
    let ecology_diff = ecology::update(world, &mut ecology_rng)?;
    for hazard in &ecology_diff.hazards {
        if let Some(region) = world.regions.get(hazard.region as usize) {
            if hazard.drought > DROUGHT_ALERT_THRESHOLD {
                chronicle.push(format!("Region {} faces an extended dry spell.", region.id));
            } else if hazard.flood > FLOOD_ALERT_THRESHOLD {
                chronicle.push(format!("Region {} endures seasonal floods.", region.id));
            }
        }
    }
    aggregate_diff.merge(&ecology_diff);
    apply(world, ecology_diff);

    world.tick = tick;

    Ok((aggregate_diff, chronicle))
}

/// Derive visual highlights for a frame from the applied diff.
pub fn collect_highlights(world: &World, diff: &Diff) -> Vec<Highlight> {
    let mut highlights = Vec::new();
    for hazard in &diff.hazards {
        if let Some(region) = world.regions.get(hazard.region as usize) {
            if hazard.drought > DROUGHT_ALERT_THRESHOLD {
                highlights.push(Highlight::hazard(
                    region.id,
                    "drought",
                    hazard.drought as f32 / WATER_MAX as f32,
                ));
            } else if hazard.flood > FLOOD_ALERT_THRESHOLD {
                highlights.push(Highlight::hazard(
                    region.id,
                    "flood",
                    hazard.flood as f32 / WATER_MAX as f32,
                ));
            }
        }
    }
    highlights
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
        let (_diff, _chronicle) = tick_once(&mut world, seed, next_tick).unwrap();
        assert_eq!(world.tick, next_tick);
    }
}
