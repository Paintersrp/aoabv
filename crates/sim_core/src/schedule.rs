use anyhow::Result;

use crate::diff::Diff;
use crate::io::frame::Highlight;
use crate::reduce::apply;
use crate::rng::{stream_label, Stream};
use crate::world::World;

#[derive(Clone, Debug)]
pub struct KernelRun {
    pub diff: Diff,
    pub chronicle: Vec<String>,
    pub highlights: Vec<Highlight>,
}

impl KernelRun {
    pub fn new(diff: Diff) -> Self {
        Self {
            diff,
            chronicle: Vec::new(),
            highlights: Vec::new(),
        }
    }
}

pub fn run_kernel<F>(
    world: &mut World,
    aggregate_diff: &mut Diff,
    parent_stream: &Stream,
    stage_label: &str,
    mut runner: F,
) -> Result<KernelRun>
where
    F: FnMut(&mut World, &mut Stream) -> Result<KernelRun>,
{
    let mut kernel_rng = parent_stream.derive(stream_label(stage_label));
    let run = runner(world, &mut kernel_rng)?;
    aggregate_diff.merge(&run.diff);
    apply(world, run.diff.clone());
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cause::{Code, Entry};
    use crate::world::{Hazards, Region};

    fn seed_world() -> World {
        let region = Region {
            id: 0,
            x: 0,
            y: 0,
            elevation_m: 0,
            latitude_deg: 12.0,
            biome: 2,
            water: 5_000,
            soil: 4_000,
            temperature_tenths_c: 0,
            precipitation_mm: 0,
            albedo_milli: 300,
            freshwater_flux_tenths_mm: 0,
            ice_mass_kilotons: 0,
            hazards: Hazards::default(),
        };
        World::new(777, 1, 1, vec![region])
    }

    #[test]
    fn run_kernel_commits_and_accumulates_diffs() {
        let mut world = seed_world();
        let mut aggregate = Diff::default();
        let parent_stream = Stream::from(world.seed, "stage:test", 1);

        let first_run = run_kernel(
            &mut world,
            &mut aggregate,
            &parent_stream,
            "kernel:first",
            |world, rng| {
                // consume an RNG sample to ensure derived streams are stable across runs
                let _ = rng.next_u64();

                assert_eq!(world.regions[0].biome, 2);
                let mut diff = Diff::default();
                diff.record_biome(0, 5);
                diff.record_water_delta(0, -250);
                diff.record_hazard(0, 5_500, 0);
                diff.record_cause(Entry::new("region:0/water", Code::DroughtFlag, None));

                let mut run = KernelRun::new(diff);
                run.chronicle.push("first pass".to_string());
                run.highlights.push(Highlight::hazard(0, "drought", 0.55));
                Ok(run)
            },
        )
        .expect("first kernel run succeeds");

        assert_eq!(world.regions[0].biome, 5);
        assert_eq!(world.regions[0].water, 4_750);
        assert_eq!(world.regions[0].hazards.drought, 5_500);
        assert_eq!(aggregate.biome.len(), 1);
        assert_eq!(aggregate.biome[0].biome, 5);
        assert_eq!(aggregate.water[0].delta, -250);
        assert_eq!(aggregate.hazards[0].drought, 5_500);
        assert_eq!(first_run.chronicle, vec!["first pass".to_string()]);
        assert_eq!(first_run.highlights.len(), 1);
        assert_eq!(first_run.highlights[0].kind, "hazard_flag");

        let second_run = run_kernel(
            &mut world,
            &mut aggregate,
            &parent_stream,
            "kernel:second",
            |world, rng| {
                let _ = rng.next_u64();

                assert_eq!(world.regions[0].water, 4_750);

                let mut diff = Diff::default();
                diff.record_water_delta(0, 100);
                diff.record_soil_delta(0, -200);
                diff.record_hazard(0, 6_000, 200);
                diff.record_cause(Entry::new("region:0/water", Code::FloodFlag, None));

                let mut run = KernelRun::new(diff);
                run.chronicle.push("second pass".to_string());
                Ok(run)
            },
        )
        .expect("second kernel run succeeds");

        assert_eq!(world.regions[0].water, 4_850);
        assert_eq!(world.regions[0].soil, 3_800);
        assert_eq!(world.regions[0].hazards.drought, 6_000);
        assert_eq!(world.regions[0].hazards.flood, 200);
        assert_eq!(aggregate.water[0].delta, -150);
        assert_eq!(aggregate.soil[0].delta, -200);
        assert_eq!(aggregate.hazards[0].drought, 6_000);
        assert_eq!(aggregate.hazards[0].flood, 200);
        assert_eq!(aggregate.causes.len(), 2);
        assert!(aggregate
            .causes
            .iter()
            .any(|cause| cause.code == Code::DroughtFlag));
        assert!(aggregate
            .causes
            .iter()
            .any(|cause| cause.code == Code::FloodFlag));
        assert_eq!(second_run.chronicle, vec!["second pass".to_string()]);
    }
}
