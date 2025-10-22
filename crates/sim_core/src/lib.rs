pub mod cause;
pub mod diff;
pub mod fixed;
pub mod io;
pub mod kernels;
pub mod reduce;
pub mod rng;
pub mod world;

use anyhow::Result;
use cause::Entry;
use diff::Diff;
use io::frame::Frame;
use io::seed::{SeedDocument, SeedRealization};
use kernels::{climate, ecology};
use reduce::apply_diff;
use rng::Stream;
use world::World;

/// Result of a single simulation tick.
pub struct TickOutputs {
    pub frame: Frame,
    pub causes: Vec<Entry>,
}

/// Deterministic simulation harness that owns the mutable [`World`].
pub struct Simulation {
    world: World,
}

impl Simulation {
    /// Construct the simulation from a parsed seed document and optional world seed override.
    pub fn from_seed_document(doc: SeedDocument, world_seed_override: Option<u64>) -> Result<Self> {
        let realization = doc.realize(world_seed_override)?;
        Ok(Self::new(realization))
    }

    /// Construct the simulation from a realised seed.
    pub fn new(realization: SeedRealization) -> Self {
        Self {
            world: realization.world,
        }
    }

    /// Access the current world snapshot.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Run a single deterministic tick, returning the NDJSON frame and causes emitted.
    pub fn tick(&mut self) -> TickOutputs {
        let next_tick = self.world.tick + 1;

        let mut aggregate_diff = Diff::default();
        let mut highlights = Vec::new();
        let mut chronicle = Vec::new();

        // Climate kernel.
        let mut climate_rng = Stream::from(self.world.seed, climate::STAGE, next_tick);
        let climate_output = climate::run(&self.world, &mut climate_rng);
        apply_diff(&mut self.world, &climate_output.diff);
        aggregate_diff.merge(&climate_output.diff);
        highlights.extend(climate_output.highlights.into_iter());
        chronicle.extend(climate_output.chronicle.into_iter());

        // Ecology kernel uses the climate-updated world state.
        let mut ecology_rng = Stream::from(self.world.seed, ecology::STAGE, next_tick);
        let ecology_output = ecology::run(&self.world, &mut ecology_rng);
        apply_diff(&mut self.world, &ecology_output.diff);
        aggregate_diff.merge(&ecology_output.diff);
        highlights.extend(ecology_output.highlights.into_iter());
        chronicle.extend(ecology_output.chronicle.into_iter());

        self.world.tick = next_tick;

        let mut diff_for_frame = aggregate_diff;
        let causes = diff_for_frame.take_causes();

        TickOutputs {
            frame: Frame {
                t: next_tick,
                diff: diff_for_frame,
                highlights,
                chronicle,
                era_end: false,
            },
            causes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::seed::SeedDocument;

    #[test]
    fn tick_advances_world() {
        let seed_json = r#"{
            "name": "test",
            "width": 2,
            "height": 1,
            "elevation_noise": {"octaves": 1, "freq": 0.1, "amp": 1.0, "seed": 42},
            "humidity_bias": {"equator": 0.2, "poles": -0.2}
        }"#;
        let doc: SeedDocument = serde_json::from_str(seed_json).unwrap();
        let mut sim = Simulation::from_seed_document(doc, Some(777)).unwrap();
        let prev_tick = sim.world.tick;
        let _ = sim.tick();
        assert_eq!(sim.world.tick, prev_tick + 1);
    }
}
