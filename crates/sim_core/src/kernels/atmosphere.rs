use anyhow::Result;

use crate::cause::{Code, Entry};
use crate::diff::Diff;
use crate::fixed::{resource_ratio, WATER_MAX};
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
const SEASON_PERIOD_TICKS: f64 = 12.0;
const SEASONAL_INSOLATION_AMPLITUDE: f64 = 0.18;
const HADLEY_DRIFT_MAX_DEGREES: f64 = 5.0;
const SEASONAL_SCALAR_EPSILON: f64 = 1e-9;

fn wrap_angle(mut angle: f64) -> f64 {
    angle %= TAU;
    if angle > PI {
        angle -= TAU;
    } else if angle < -PI {
        angle += TAU;
    }
    angle
}

fn sin_series(angle: f64) -> f64 {
    let x = wrap_angle(angle);
    let x2 = x * x;
    let x3 = x * x2;
    let x5 = x3 * x2;
    let x7 = x5 * x2;
    let x9 = x7 * x2;
    let x11 = x9 * x2;
    let x13 = x11 * x2;
    x - x3 / 6.0 + x5 / 120.0 - x7 / 5_040.0 + x9 / 362_880.0 - x11 / 39_916_800.0
        + x13 / 6_227_020_800.0
}

fn seasonal_scalar(tick: u64) -> f64 {
    if SEASON_PERIOD_TICKS <= f64::EPSILON {
        return 0.0;
    }
    let phase = (tick as f64 / SEASON_PERIOD_TICKS) * TAU;
    sin_series(phase)
}

fn insolation_factor(latitude_deg: f64) -> f64 {
    let closeness = (90.0 - latitude_deg.abs()).max(0.0) / 90.0;
    closeness.powf(0.85)
}

fn hadley_strength(latitude_deg: f64) -> f64 {
    if latitude_deg.abs() >= HADLEY_LATITUDE_MAX {
        0.0
    } else {
        1.0 - latitude_deg.abs() / HADLEY_LATITUDE_MAX
    }
}

fn compute_temperature_tenths(
    latitude_deg: f64,
    elevation_m: i32,
    humidity_ratio: f64,
    insolation_bias: f64,
) -> i32 {
    let insolation = (insolation_factor(latitude_deg) * insolation_bias).clamp(0.0, 1.2);
    let base_temp_c = -25.0 + 60.0 * insolation;
    let lapse = (f64::from(elevation_m.max(0)) / 1_000.0) * LAPSE_RATE_C_PER_KM;
    let humidity_bonus = (humidity_ratio - 0.5) * HUMIDITY_TEMP_BONUS;
    ((base_temp_c - lapse + humidity_bonus) * 10.0).round() as i32
}

fn compute_precip_mm(
    latitude_deg: f64,
    elevation_m: i32,
    humidity_ratio: f64,
    hadley_strength: f64,
    insolation_bias: f64,
) -> i32 {
    let insolation = (insolation_factor(latitude_deg) * insolation_bias).clamp(0.0, 1.2);
    let elevation_km = f64::from(elevation_m.max(0)) / 1_000.0;
    let lift_bonus = (elevation_km * 260.0).min(700.0);
    let convective = 1_000.0 + 2_200.0 * humidity_ratio * insolation;
    let hadley_bonus = 1_200.0 * hadley_strength * humidity_ratio;
    let humidity_penalty = (1.0 - humidity_ratio).max(0.0) * 700.0;
    let thin_air_penalty = elevation_km.powf(1.15) * 120.0;
    let precip = convective + hadley_bonus + lift_bonus - humidity_penalty - thin_air_penalty;
    precip.round() as i32
}

fn prevailing_wind(latitude_deg: f64) -> (i32, i32) {
    let abs_lat = latitude_deg.abs();
    if abs_lat < 30.0 {
        // Trade winds (east to west).
        (-1, 0)
    } else if abs_lat < 60.0 {
        // Mid-latitude westerlies (west to east).
        (1, 0)
    } else {
        // Polar easterlies (east to west).
        (-1, 0)
    }
}

fn region_index_at(world: &World, x: i32, y: i32) -> Option<usize> {
    if x < 0 || y < 0 {
        return None;
    }
    let (width, height) = (world.width as i32, world.height as i32);
    if x >= width || y >= height {
        return None;
    }
    let idx = (y as usize) * (world.width as usize) + (x as usize);
    if idx < world.regions.len() {
        let region = &world.regions[idx];
        if region.x as i32 == x && region.y as i32 == y {
            return Some(idx);
        }
    }
    world
        .regions
        .iter()
        .enumerate()
        .find(|(_, region)| region.x as i32 == x && region.y as i32 == y)
        .map(|(index, _)| index)
}

pub fn update(world: &World, rng: &mut Stream) -> Result<KernelRun> {
    let mut diff = Diff::default();
    let total_regions = world.regions.len();
    if total_regions == 0 {
        return Ok(KernelRun::new(diff));
    }

    let seasonal = seasonal_scalar(world.tick);
    let insolation_bias = (1.0 + SEASONAL_INSOLATION_AMPLITUDE * seasonal).clamp(
        1.0 - SEASONAL_INSOLATION_AMPLITUDE,
        1.0 + SEASONAL_INSOLATION_AMPLITUDE,
    );
    let hadley_lat_shift = HADLEY_DRIFT_MAX_DEGREES * seasonal;

    let moisture_stream = rng.derive(stream_label("CLIMATE.atmo_moisture"));
    let orography_stream = rng.derive(stream_label("CLIMATE.atmo_orography"));
    let commit_stream = rng.derive(stream_label("CLIMATE.atmo_precip_commit"));

    let mut humidity: Vec<f64> = Vec::with_capacity(total_regions);
    let mut precip_multipliers = vec![1.0f64; total_regions];
    let mut lift_gradients = vec![0.0f64; total_regions];
    let mut lift_multipliers = vec![1.0f64; total_regions];
    let mut rain_shadow_factors = vec![0.0f64; total_regions];

    for (index, region) in world.regions.iter().enumerate() {
        debug_assert_eq!(
            region.index(),
            index,
            "region id {} does not match index {}",
            region.id,
            index
        );

        let mut region_rng = moisture_stream.derive(index as u64);
        let base_ratio = resource_ratio(region.water, WATER_MAX);
        let jitter = region_rng.next_signed_unit() * HUMIDITY_NOISE_FRACTION;
        let ratio = (base_ratio + jitter).clamp(0.0, 1.0);
        humidity.push(ratio);
    }

    for (index, region) in world.regions.iter().enumerate() {
        let (wind_dx, wind_dy) = prevailing_wind(region.latitude_deg);
        if wind_dx == 0 && wind_dy == 0 {
            continue;
        }

        let mut effect_rng = orography_stream.derive(index as u64);
        let lift_jitter = effect_rng.next_f64();
        let shadow_jitter = effect_rng.next_f64();

        let upwind_x = region.x as i32 - wind_dx;
        let upwind_y = region.y as i32 - wind_dy;
        if let Some(upwind_index) = region_index_at(world, upwind_x, upwind_y) {
            let upwind = &world.regions[upwind_index];
            let gradient_km = f64::from(region.elevation_m - upwind.elevation_m) / 1_000.0;
            if gradient_km >= OROGRAPHIC_LIFT_THRESHOLD_KM {
                let random_factor = 0.85 + lift_jitter * 0.3;
                let lift = gradient_km * 0.25 * random_factor;
                humidity[index] = (humidity[index] + lift).clamp(0.0, 1.0);
                let multiplier = (1.0 + lift * 0.8).clamp(1.0, PRECIP_MULTIPLIER_MAX);
                precip_multipliers[index] *= multiplier;
                lift_gradients[index] = gradient_km;
                lift_multipliers[index] = precip_multipliers[index];

                let downwind_x = region.x as i32 + wind_dx;
                let downwind_y = region.y as i32 + wind_dy;
                if let Some(downwind_index) = region_index_at(world, downwind_x, downwind_y) {
                    let dryness_base = gradient_km * (0.18 + shadow_jitter * 0.12);
                    let dryness = dryness_base.clamp(0.0, RAIN_SHADOW_MAX);
                    humidity[downwind_index] =
                        (humidity[downwind_index] * (1.0 - dryness)).clamp(0.0, 1.0);
                    let rain_multiplier = (1.0 - dryness * 0.65).clamp(PRECIP_MULTIPLIER_MIN, 1.0);
                    precip_multipliers[downwind_index] *= rain_multiplier;
                    rain_shadow_factors[downwind_index] =
                        rain_shadow_factors[downwind_index].max(dryness);
                }
            }
        }
    }

    let mut chronicle = Vec::new();
    let mut monsoon_regions = 0usize;

    for (index, region) in world.regions.iter().enumerate() {
        let mut commit_rng = commit_stream.derive(index as u64);
        let humidity_ratio = humidity[index].clamp(0.0, 1.0);
        let humidity_tenths = (humidity_ratio * f64::from(HUMIDITY_TENTHS_MAX)).round() as i32;
        let humidity_tenths = humidity_tenths.clamp(0, HUMIDITY_TENTHS_MAX);
        diff.record_humidity(index, humidity_tenths);

        let effective_latitude = (region.latitude_deg - hadley_lat_shift).clamp(-90.0, 90.0);
        let hadley = hadley_strength(effective_latitude);
        let baseline_offset = world
            .climate
            .temperature_baseline_tenths
            .get(index)
            .copied()
            .unwrap_or(0);
        let mut temperature_tenths = compute_temperature_tenths(
            effective_latitude,
            region.elevation_m,
            humidity_ratio,
            insolation_bias,
        )
        .clamp(TEMP_MIN_TENTHS_C, TEMP_MAX_TENTHS_C);
        temperature_tenths = (temperature_tenths + i32::from(baseline_offset))
            .clamp(TEMP_MIN_TENTHS_C, TEMP_MAX_TENTHS_C);
        if i32::from(region.temperature_tenths_c) != temperature_tenths {
            diff.record_temperature(index, temperature_tenths);
        }

        let base_precip = compute_precip_mm(
            effective_latitude,
            region.elevation_m,
            humidity_ratio,
            hadley,
            insolation_bias,
        );
        let jitter = (commit_rng.next_f64() - 0.5) * 0.04;
        let scaled_precip =
            (f64::from(base_precip) * precip_multipliers[index] * (1.0 + jitter)).round() as i32;
        let precip_mm = scaled_precip.clamp(PRECIP_MIN_MM, PRECIP_MAX_MM);
        if u16::from(region.precipitation_mm) != precip_mm as u16 {
            diff.record_precipitation(index, precip_mm);
        }

        if seasonal.abs() > SEASONAL_SCALAR_EPSILON {
            diff.record_cause(Entry::new(
                format!("region:{}/temperature", region.id),
                Code::SeasonalityVariance,
                Some(format!("scalar={:.3}", seasonal)),
            ));
            diff.record_cause(Entry::new(
                format!("region:{}/precip", region.id),
                Code::SeasonalityVariance,
                Some(format!("scalar={:.3}", seasonal)),
            ));
        }

        if hadley_lat_shift.abs() > SEASONAL_SCALAR_EPSILON {
            diff.record_cause(Entry::new(
                format!("region:{}/temperature", region.id),
                Code::HadleyDrift,
                Some(format!("shift_deg={:.2}", hadley_lat_shift)),
            ));
            diff.record_cause(Entry::new(
                format!("region:{}/precip", region.id),
                Code::HadleyDrift,
                Some(format!("shift_deg={:.2}", hadley_lat_shift)),
            ));
        }

        if hadley > 0.0 {
            diff.record_cause(Entry::new(
                format!("region:{}/temperature", region.id),
                Code::HadleyCell,
                Some(format!("strength={:.2}", hadley)),
            ));
        }

        if lift_gradients[index] > 0.0 {
            diff.record_cause(Entry::new(
                format!("region:{}/precip", region.id),
                Code::OrographicLift,
                Some(format!(
                    "gradient_km={:.2};multiplier={:.2}",
                    lift_gradients[index], lift_multipliers[index]
                )),
            ));
        }

        if rain_shadow_factors[index] > 0.0 {
            diff.record_cause(Entry::new(
                format!("region:{}/precip", region.id),
                Code::RainShadow,
                Some(format!("shadow_factor={:.2}", rain_shadow_factors[index])),
            ));
        }

        let monsoon_strength = hadley * humidity_ratio;
        if hadley > MONSOON_STRENGTH_THRESHOLD && humidity_ratio >= MONSOON_HUMIDITY_THRESHOLD {
            diff.record_cause(Entry::new(
                format!("region:{}/precip", region.id),
                Code::MonsoonOnset,
                Some(format!("intensity={:.2}", monsoon_strength)),
            ));
            monsoon_regions += 1;
        }
    }

    let summary = if monsoon_regions > 0 {
        format!(
            "Hadley cells shifted {:+.1}°; monsoons intensified across {} regions.",
            hadley_lat_shift, monsoon_regions
        )
    } else {
        format!(
            "Hadley cells shifted {:+.1}°; seasonal scalar {:+.2}.",
            hadley_lat_shift, seasonal
        )
    };
    chronicle.push(summary);

    Ok(KernelRun {
        diff,
        chronicle,
        highlights: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::frame::make_frame;
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
        world.tick = 3;
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
            .any(|entry| entry.code == Code::SeasonalityVariance));
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

        let mut rng_a = Stream::from(world.seed, STAGE, 4);
        let mut rng_b = Stream::from(world.seed, STAGE, 4);

        let run_a = update(&world, &mut rng_a).expect("first pass succeeds");
        let run_b = update(&world, &mut rng_b).expect("second pass succeeds");

        assert_eq!(run_a.diff.temperature, run_b.diff.temperature);
        assert_eq!(run_a.diff.precipitation, run_b.diff.precipitation);
        assert_eq!(run_a.diff.humidity, run_b.diff.humidity);
        assert_eq!(run_a.diff.causes, run_b.diff.causes);
        assert_eq!(run_a.chronicle, run_b.chronicle);
    }

    #[test]
    fn seasonal_scalar_matches_expected_phases() {
        let checkpoints = [(0, 0.0), (3, 1.0), (6, 0.0), (9, -1.0), (12, 0.0)];
        for (tick, expected) in checkpoints {
            let actual = seasonal_scalar(tick);
            assert!(
                (actual - expected).abs() < 2e-4,
                "tick {} expected {:.3} got {:.6}",
                tick,
                expected,
                actual
            );
        }
        assert!((seasonal_scalar(0) - seasonal_scalar(12)).abs() < 1e-9);
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
