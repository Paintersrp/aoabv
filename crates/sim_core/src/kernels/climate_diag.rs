use anyhow::Result;

use crate::diff::Diff;
use crate::rng::Stream;
use crate::schedule::KernelRun;
use crate::world::World;

pub const STAGE: &str = "CLIMATE::climate_diag";
pub const CHRONICLE_LINE: &str = "Climate diagnostics stable; no anomalies detected.";

const DIAG_MIN: i32 = -1_000;
const DIAG_MAX: i32 = 1_000;

pub fn update(world: &World, _rng: &mut Stream) -> Result<KernelRun> {
    let mut diff = Diff::default();

    if world.regions.is_empty() {
        diff.record_diag_climate(0, 0);
        let mut run = KernelRun::new(diff);
        run.chronicle.push(CHRONICLE_LINE.to_string());
        return Ok(run);
    }

    let region_count = world.regions.len() as f64;
    let mean_temp = world
        .regions
        .iter()
        .map(|region| f64::from(region.temperature_tenths_c))
        .sum::<f64>()
        / region_count;
    let mean_precip = world
        .regions
        .iter()
        .map(|region| f64::from(region.precipitation_mm))
        .sum::<f64>()
        / region_count;
    let mean_water = world
        .regions
        .iter()
        .map(|region| f64::from(region.water))
        .sum::<f64>()
        / region_count;
    let mean_albedo = world
        .regions
        .iter()
        .map(|region| f64::from(region.albedo_milli))
        .sum::<f64>()
        / region_count;
    let sea_level = world.climate.sea_level_equivalent_mm() as f64;

    let composite = 0.45 * mean_temp
        + 0.25 * ((mean_precip - 1_500.0) / 5.0)
        + 0.15 * ((mean_water - 5_000.0) / 5.0)
        + 0.1 * ((mean_albedo - 450.0) / 2.0)
        + 0.05 * sea_level;

    let diag_value = composite.round() as i32;
    let clamped = diag_value.clamp(DIAG_MIN, DIAG_MAX);
    diff.record_diag_climate(0, clamped);

    let mut run = KernelRun::new(diff);
    run.chronicle.push(CHRONICLE_LINE.to_string());
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Stream;
    use crate::world::{Hazards, Region, World};

    fn seed_world(temp: i16, precip: u16, water: u16, albedo: u16) -> World {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 100,
                latitude_deg: 0.0,
                biome: 0,
                water,
                soil: 5_000,
                temperature_tenths_c: temp,
                precipitation_mm: precip,
                albedo_milli: albedo,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 200,
                latitude_deg: 15.0,
                biome: 1,
                water,
                soil: 4_000,
                temperature_tenths_c: temp,
                precipitation_mm: precip,
                albedo_milli: albedo,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
        ];
        World::new(42, 2, 1, regions)
    }

    #[test]
    fn update_emits_diagnostic_diff_and_chronicle() {
        let world = seed_world(180, 1_200, 7_500, 520);
        let mut rng = Stream::from(world.seed, STAGE, 1);

        let run = update(&world, &mut rng).expect("climate diag update succeeds");
        assert_eq!(run.chronicle, vec![CHRONICLE_LINE.to_string()]);
        assert_eq!(run.highlights.len(), 0);
        assert_eq!(run.diff.diag_climate.len(), 1);
        let entry = &run.diff.diag_climate[0];
        assert_eq!(entry.region, 0);

        let region_count = world.regions.len() as f64;
        let mean_temp = world
            .regions
            .iter()
            .map(|region| f64::from(region.temperature_tenths_c))
            .sum::<f64>()
            / region_count;
        let mean_precip = world
            .regions
            .iter()
            .map(|region| f64::from(region.precipitation_mm))
            .sum::<f64>()
            / region_count;
        let mean_water = world
            .regions
            .iter()
            .map(|region| f64::from(region.water))
            .sum::<f64>()
            / region_count;
        let mean_albedo = world
            .regions
            .iter()
            .map(|region| f64::from(region.albedo_milli))
            .sum::<f64>()
            / region_count;
        let expected = (0.45 * mean_temp
            + 0.25 * ((mean_precip - 1_500.0) / 5.0)
            + 0.15 * ((mean_water - 5_000.0) / 5.0)
            + 0.1 * ((mean_albedo - 450.0) / 2.0)
            + 0.05 * world.climate.sea_level_equivalent_mm() as f64)
            .round() as i32;
        assert_eq!(entry.value, expected.clamp(DIAG_MIN, DIAG_MAX));
    }

    #[test]
    fn diagnostic_value_clamped_to_bounds() {
        let mut world = seed_world(500, 5_000, 10_000, 900);
        world.climate.add_sea_level_equivalent_mm(40_000);
        let mut rng = Stream::from(world.seed, STAGE, 99);

        let run = update(&world, &mut rng).expect("climate diag update succeeds");
        let entry = &run.diff.diag_climate[0];
        assert_eq!(entry.value, DIAG_MAX);

        let mut world = seed_world(-500, 0, 100, 100);
        world.climate.add_sea_level_equivalent_mm(-40_000);
        let mut rng = Stream::from(world.seed, STAGE, 3);

        let run = update(&world, &mut rng).expect("climate diag update succeeds");
        let entry = &run.diff.diag_climate[0];
        assert_eq!(entry.value, DIAG_MIN);
    }
}
