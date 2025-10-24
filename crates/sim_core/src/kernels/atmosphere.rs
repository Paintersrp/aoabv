use anyhow::Result;

use crate::cause::{Code, Entry};
use crate::diff::Diff;
use crate::fixed::{resource_ratio, WATER_MAX};
use crate::rng::Stream;
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

fn compute_temperature_tenths(latitude_deg: f64, elevation_m: i32, humidity_ratio: f64) -> i32 {
    let insolation = insolation_factor(latitude_deg);
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
) -> i32 {
    let insolation = insolation_factor(latitude_deg);
    let elevation_km = f64::from(elevation_m.max(0)) / 1_000.0;
    let lift_bonus = (elevation_km * 260.0).min(700.0);
    let convective = 1_000.0 + 2_200.0 * humidity_ratio * insolation;
    let hadley_bonus = 1_200.0 * hadley_strength * humidity_ratio;
    let humidity_penalty = (1.0 - humidity_ratio).max(0.0) * 700.0;
    let thin_air_penalty = elevation_km.powf(1.15) * 120.0;
    let precip = convective + hadley_bonus + lift_bonus - humidity_penalty - thin_air_penalty;
    precip.round() as i32
}

pub fn update(world: &World, rng: &mut Stream) -> Result<Diff> {
    let _ = rng;
    let mut diff = Diff::default();

    for (index, region) in world.regions.iter().enumerate() {
        debug_assert_eq!(
            region.index(),
            index,
            "region id {} does not match index {}",
            region.id,
            index
        );

        let humidity_ratio = resource_ratio(region.water, WATER_MAX);
        let hadley = hadley_strength(region.latitude_deg);
        let temperature_tenths =
            compute_temperature_tenths(region.latitude_deg, region.elevation_m, humidity_ratio)
                .clamp(TEMP_MIN_TENTHS_C, TEMP_MAX_TENTHS_C);
        if i32::from(region.temperature_tenths_c) != temperature_tenths {
            diff.record_temperature(index, temperature_tenths);
        }

        let precip_mm = compute_precip_mm(
            region.latitude_deg,
            region.elevation_m,
            humidity_ratio,
            hadley,
        )
        .clamp(PRECIP_MIN_MM, PRECIP_MAX_MM);
        if u16::from(region.precipitation_mm) != precip_mm as u16 {
            diff.record_precipitation(index, precip_mm);
        }

        if hadley > 0.0 {
            diff.record_cause(Entry::new(
                format!("region:{}/temperature", region.id),
                Code::HadleyCell,
                Some(format!("strength={:.2}", hadley)),
            ));
        }

        let elevation_km = f64::from(region.elevation_m.max(0)) / 1_000.0;
        if elevation_km >= OROGRAPHIC_LIFT_THRESHOLD_KM {
            diff.record_cause(Entry::new(
                format!("region:{}/precip", region.id),
                Code::OrographicLift,
                Some(format!("lift_km={:.2}", elevation_km)),
            ));
        }

        let monsoon_strength = hadley * humidity_ratio;
        if hadley > MONSOON_STRENGTH_THRESHOLD && humidity_ratio >= MONSOON_HUMIDITY_THRESHOLD {
            diff.record_cause(Entry::new(
                format!("region:{}/precip", region.id),
                Code::MonsoonOnset,
                Some(format!("intensity={:.2}", monsoon_strength)),
            ));
        }
    }

    Ok(diff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Hazards, Region, World};

    #[test]
    fn atmosphere_records_energy_balance_and_causes() {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 2_000,
                latitude_deg: 12.0,
                biome: 0,
                water: 9_000,
                soil: 8_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 400,
                freshwater_flux_tenths_mm: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 100,
                latitude_deg: 48.0,
                biome: 0,
                water: 4_500,
                soil: 4_500,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 360,
                freshwater_flux_tenths_mm: 0,
                hazards: Hazards::default(),
            },
        ];
        let world = World::new(7, 2, 1, regions);
        let mut rng = Stream::from(world.seed, STAGE, 1);

        let diff = update(&world, &mut rng).expect("atmosphere update succeeds");

        assert!(!diff.temperature.is_empty(), "temperature map populated");
        assert!(
            !diff.precipitation.is_empty(),
            "precipitation map populated"
        );
        assert!(diff
            .causes
            .iter()
            .any(|entry| entry.code == Code::HadleyCell));
        assert!(diff
            .causes
            .iter()
            .any(|entry| entry.code == Code::OrographicLift));
        assert!(diff
            .causes
            .iter()
            .any(|entry| entry.code == Code::MonsoonOnset));
    }
}
