mod humidity;
mod orography;
mod precipitation;
pub(crate) mod seasonality;

use anyhow::Result;

use crate::diff::Diff;
use crate::rng::{stream_label, Stream};
use crate::schedule::KernelRun;
use crate::world::World;

pub const STAGE: &str = "kernel:atmosphere";

const TEMP_MIN_TENTHS_C: i32 = -500;
const TEMP_MAX_TENTHS_C: i32 = 500;
const PRECIP_MIN_MM: i32 = 0;
const PRECIP_MAX_MM: i32 = 5_000;
const HADLEY_LATITUDE_MAX: f64 = 30.0;
const MONSOON_HUMIDITY_THRESHOLD: f64 = 0.6;
const MONSOON_STRENGTH_THRESHOLD: f64 = 0.25;
const LAPSE_RATE_C_PER_KM: f64 = 6.5;
const HUMIDITY_TEMP_BONUS: f64 = 10.0;
const OROGRAPHIC_LIFT_THRESHOLD_KM: f64 = 0.25;
const HUMIDITY_TENTHS_MAX: i32 = 1_000;
const HUMIDITY_NOISE_FRACTION: f64 = 0.03;
const PRECIP_MULTIPLIER_MIN: f64 = 0.2;
const PRECIP_MULTIPLIER_MAX: f64 = 3.0;
const RAIN_SHADOW_MAX: f64 = 0.75;
const PI: f64 = 3.14159265358979323846264338327950288;
const TAU: f64 = 6.28318530717958647692528676655900577;
pub(crate) const SEASON_PERIOD_TICKS: u64 = 4;
pub(crate) const SEASONAL_INSOLATION_AMPLITUDE: f64 = 0.18;
const HADLEY_DRIFT_MAX_DEGREES: f64 = 5.0;
const SEASONAL_SCALAR_EPSILON: f64 = 1e-9;

pub fn update(world: &World, rng: &mut Stream) -> Result<KernelRun> {
    if world.regions.is_empty() {
        return Ok(KernelRun::new(Diff::default()));
    }

    let seasonal = seasonality::compute(world);

    let moisture_stream = rng.derive(stream_label("CLIMATE.atmo_moisture"));
    let orography_stream = rng.derive(stream_label("CLIMATE.atmo_orography"));
    let commit_stream = rng.derive(stream_label("CLIMATE.atmo_precip_commit"));

    let mut humidity = humidity::sample(world, &moisture_stream);
    let orography = orography::apply(world, &orography_stream, &mut humidity);
    let precipitation = precipitation::commit(
        world,
        humidity.as_slice(),
        &seasonal,
        &orography,
        &commit_stream,
    );

    Ok(KernelRun {
        diff: precipitation.diff,
        chronicle: precipitation.chronicle,
        highlights: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::seasonality;
    use super::*;
    use crate::cause::Code;
    use crate::fixed::WATER_MAX;
    use crate::io::frame::make_frame;
    use crate::rng::Stream;
    use crate::world::{Hazards, Region, World};
    use proptest::prelude::*;

    #[test]
    fn atmosphere_records_energy_balance_and_causes() {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 200,
                latitude_deg: 10.0,
                biome: 0,
                water: 9_500,
                soil: 8_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 400,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 2_400,
                latitude_deg: 10.0,
                biome: 0,
                water: 9_000,
                soil: 8_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 360,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 2,
                x: 2,
                y: 0,
                elevation_m: 100,
                latitude_deg: 10.0,
                biome: 0,
                water: 9_200,
                soil: 8_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 380,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
        ];
        let mut world = World::new(7, 3, 1, regions);
        world.tick = 2;
        world.climate.last_insolation_tenths.fill(12_800);
        for region in &mut world.regions {
            region.precipitation_mm = 1_200;
        }
        let mut rng = Stream::from(world.seed, STAGE, 1);

        let run = update(&world, &mut rng).expect("atmosphere update succeeds");
        let diff = run.diff;

        assert!(!diff.temperature.is_empty(), "temperature map populated");
        assert!(
            !diff.precipitation.is_empty(),
            "precipitation map populated"
        );
        assert_eq!(diff.humidity.len(), world.regions.len());
        assert!(diff
            .causes
            .iter()
            .any(|entry| entry.code == Code::HadleyCell));
        assert!(diff
            .causes
            .iter()
            .any(|entry| entry.code == Code::SeasonalShift));
        assert!(diff
            .causes
            .iter()
            .any(|entry| entry.code == Code::HadleyDrift));
        assert!(diff
            .causes
            .iter()
            .any(|entry| entry.code == Code::OrographicLift));
        assert!(diff
            .causes
            .iter()
            .any(|entry| entry.code == Code::RainShadow));
        assert!(diff
            .causes
            .iter()
            .any(|entry| entry.code == Code::MonsoonOnset));
        assert!(diff
            .causes
            .iter()
            .any(|entry| entry.code == Code::HumidityTransport));
        assert_eq!(run.chronicle.len(), 1);
    }

    #[test]
    fn atmosphere_update_is_deterministic() {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 300,
                latitude_deg: 15.0,
                biome: 0,
                water: 6_500,
                soil: 5_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 360,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 1_800,
                latitude_deg: 28.0,
                biome: 0,
                water: 8_000,
                soil: 5_200,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 360,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 2,
                x: 2,
                y: 0,
                elevation_m: 120,
                latitude_deg: 35.0,
                biome: 0,
                water: 7_500,
                soil: 5_400,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 360,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
        ];
        let mut world = World::new(11, 3, 1, regions);
        world.tick = 7;
        world.climate.last_insolation_tenths = vec![12_400, 12_550, 12_700];
        for (i, region) in world.regions.iter_mut().enumerate() {
            region.precipitation_mm = 900 + (i as u16) * 75;
        }

        let mut rng_a = Stream::from(world.seed, STAGE, 4);
        let mut rng_b = Stream::from(world.seed, STAGE, 4);

        let run_a = update(&world, &mut rng_a).expect("first pass succeeds");
        let run_b = update(&world, &mut rng_b).expect("second pass succeeds");

        assert_eq!(run_a.diff.temperature, run_b.diff.temperature);
        assert_eq!(run_a.diff.precipitation, run_b.diff.precipitation);
        assert_eq!(run_a.diff.humidity, run_b.diff.humidity);
        assert_eq!(run_a.diff.causes, run_b.diff.causes);
        assert_eq!(run_a.chronicle, run_b.chronicle);
        assert!(run_a
            .diff
            .causes
            .iter()
            .any(|entry| entry.code == Code::HumidityTransport));
    }

    #[test]
    fn seasonal_scalar_matches_quarter_cycle() {
        let checkpoints = [(0, 0.0), (1, 1.0), (2, 0.0), (3, -1.0), (4, 0.0)];
        for (tick, expected) in checkpoints {
            let actual = seasonality::scalar_for_tick(tick);
            assert!(
                (actual - expected).abs() < 2e-4,
                "tick {} expected {:.3} got {:.6}",
                tick,
                expected,
                actual
            );
        }
        assert!((seasonality::scalar_for_tick(0) - seasonality::scalar_for_tick(4)).abs() < 1e-9);
    }

    #[test]
    fn seasonal_outputs_reproduce_for_identical_seed_and_tick() {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 150,
                latitude_deg: 12.0,
                biome: 0,
                water: 8_200,
                soil: 6_400,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 360,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 1_200,
                latitude_deg: 24.0,
                biome: 0,
                water: 7_900,
                soil: 6_100,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 355,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
        ];

        let mut world_a = World::new(19, 2, 1, regions.clone());
        world_a.tick = 5;
        world_a.climate.last_insolation_tenths = vec![12_200, 12_260];

        let world_b = world_a.clone();

        let mut rng_a = Stream::from(world_a.seed, STAGE, 3);
        let mut rng_b = Stream::from(world_b.seed, STAGE, 3);

        let run_a = update(&world_a, &mut rng_a).expect("first pass reproducible");
        let run_b = update(&world_b, &mut rng_b).expect("second pass reproducible");

        assert_eq!(run_a.diff.temperature, run_b.diff.temperature);
        assert_eq!(run_a.diff.precipitation, run_b.diff.precipitation);
        assert_eq!(run_a.diff.humidity, run_b.diff.humidity);
        assert_eq!(run_a.diff.causes, run_b.diff.causes);
        assert_eq!(run_a.chronicle, run_b.chronicle);
    }

    proptest! {
        #[test]
        fn humidity_diff_within_bounds(waters in prop::collection::vec(0u16..=WATER_MAX, 1..5)) {
            let regions: Vec<Region> = waters.iter().enumerate().map(|(i, water)| Region {
                id: i as u32,
                x: i as u32,
                y: 0,
                elevation_m: 200 + (i as i32 * 150),
                latitude_deg: 5.0 + (i as f64 * 4.0),
                biome: 0,
                water: *water,
                soil: 5_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 350,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            }).collect();

            let width = regions.len() as u32;
            let world = World::new(29, width.max(1), 1, regions);
            let mut rng = Stream::from(world.seed, STAGE, 2);
            let diff = update(&world, &mut rng)
                .expect("atmosphere update succeeds")
                .diff;

            for value in diff.humidity {
                prop_assert!(value.value >= 0);
                prop_assert!(value.value <= HUMIDITY_TENTHS_MAX);
            }
        }
    }

    #[test]
    fn humidity_diff_region_keys_are_prefixed() {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 100,
                latitude_deg: 8.0,
                biome: 0,
                water: 8_500,
                soil: 6_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 360,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 1_400,
                latitude_deg: 8.0,
                biome: 0,
                water: 8_800,
                soil: 6_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 360,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
        ];
        let world = World::new(47, 2, 1, regions);
        let mut rng = Stream::from(world.seed, STAGE, 3);
        let diff = update(&world, &mut rng)
            .expect("atmosphere update succeeds")
            .diff;
        let frame = make_frame(
            world.tick,
            diff,
            Vec::new(),
            Vec::new(),
            false,
            world.width,
            world.height,
        );

        assert!(!frame.diff.humidity.is_empty(), "humidity diff populated");
        for key in frame.diff.humidity.keys() {
            assert!(key.starts_with("r:"), "key {} missing region prefix", key);
        }
    }

    #[test]
    fn temperature_and_precip_within_bounds() {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 50,
                latitude_deg: -18.0,
                biome: 0,
                water: 9_800,
                soil: 7_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 340,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 3_200,
                latitude_deg: 32.0,
                biome: 0,
                water: 5_500,
                soil: 6_200,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 360,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 2,
                x: 2,
                y: 0,
                elevation_m: 400,
                latitude_deg: 58.0,
                biome: 0,
                water: 6_700,
                soil: 6_400,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 360,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
        ];
        let mut world = World::new(19, 3, 1, regions);
        world.tick = 5;
        let mut rng = Stream::from(world.seed, STAGE, 5);
        let diff = update(&world, &mut rng)
            .expect("atmosphere update succeeds")
            .diff;

        for value in &diff.temperature {
            assert!(
                (TEMP_MIN_TENTHS_C..=TEMP_MAX_TENTHS_C).contains(&value.value),
                "temperature {} out of bounds",
                value.value
            );
        }

        for value in &diff.precipitation {
            assert!(
                (PRECIP_MIN_MM..=PRECIP_MAX_MM).contains(&value.value),
                "precipitation {} out of bounds",
                value.value
            );
        }
    }
}
