use crate::cause::{Code, Entry};
use crate::diff::Diff;
use crate::fixed::{clamp_hazard_meter, clamp_u16, resource_ratio, SOIL_MAX, WATER_MAX};
use crate::rng::Stream;
use crate::world::World;
use anyhow::{ensure, Result};

pub const STAGE: &str = "kernel:ecology";

/// Hazard level required before emitting alerts or highlights for droughts.
pub const DROUGHT_ALERT_THRESHOLD: u16 = 2_000;
/// Hazard level required before emitting alerts or highlights for floods.
pub const FLOOD_ALERT_THRESHOLD: u16 = 600;

/// Blend the previous hazard gauge toward the new target with a per-tick half-life.
///
/// Each invocation halves the difference between the stored gauge and the incoming
/// target while rounding away from zero, yielding deterministic exponential decay
/// without floating point noise or stalls at low magnitudes.
fn blend_hazard(previous: u16, target: u16) -> u16 {
    if previous == target {
        return clamp_hazard_meter(previous);
    }

    let prev = i32::from(previous);
    let goal = i32::from(target);
    let diff = goal - prev;
    let step = if diff > 0 {
        (diff + 1) / 2
    } else {
        -(((-diff) + 1) / 2)
    };
    let blended = (prev + step).max(0);
    clamp_hazard_meter(blended as u16)
}

struct BiomeProfile {
    water_target: f64,
    soil_target: f64,
}

fn profile_for_biome(biome: u8) -> BiomeProfile {
    match biome {
        5 => BiomeProfile {
            water_target: 0.85,
            soil_target: 0.75,
        },
        4 => BiomeProfile {
            water_target: 0.2,
            soil_target: 0.25,
        },
        3 => BiomeProfile {
            water_target: 0.35,
            soil_target: 0.4,
        },
        2 => BiomeProfile {
            water_target: 0.55,
            soil_target: 0.55,
        },
        1 => BiomeProfile {
            water_target: 0.4,
            soil_target: 0.45,
        },
        _ => BiomeProfile {
            water_target: 0.25,
            soil_target: 0.3,
        },
    }
}

pub fn update(world: &World, rng: &mut Stream) -> Result<Diff> {
    let mut diff = Diff::default();

    for (index, region) in world.regions.iter().enumerate() {
        ensure!(
            region.index() == index,
            "region id {} does not match index {}",
            region.id,
            index
        );
        ensure!(
            region.water <= WATER_MAX,
            "region {} water {} exceeds WATER_MAX {}",
            region.id,
            region.water,
            WATER_MAX
        );
        ensure!(
            region.soil <= SOIL_MAX,
            "region {} soil {} exceeds SOIL_MAX {}",
            region.id,
            region.soil,
            SOIL_MAX
        );
        let mut region_rng = rng.derive(region.index() as u64);
        let profile = profile_for_biome(region.biome);
        let water_ratio = resource_ratio(region.water, WATER_MAX);
        let soil_ratio = resource_ratio(region.soil, SOIL_MAX);

        let water_drift = ((profile.water_target - water_ratio) * 200.0).round() as i32;
        let soil_drift = ((profile.soil_target - soil_ratio) * 150.0).round() as i32;
        let noise = (region_rng.next_signed_unit() * 25.0) as i32;

        let water_delta = (water_drift + noise).clamp(-180, 180);
        let noise_half = if noise >= 0 {
            noise / 2
        } else {
            (noise - 1) / 2
        };
        let soil_delta = (soil_drift + noise_half).clamp(-120, 120);

        if water_delta != 0 {
            diff.record_water_delta(region.index(), water_delta);
        }
        if soil_delta != 0 {
            diff.record_soil_delta(region.index(), soil_delta);
        }

        let new_water = clamp_u16(region.water as i32 + water_delta, 0, WATER_MAX);
        let new_soil = clamp_u16(region.soil as i32 + soil_delta, 0, SOIL_MAX);

        let drought_target = WATER_MAX.saturating_sub(new_water);
        let flood_target = new_water.saturating_sub(WATER_MAX - 1_500);
        let drought_level = blend_hazard(region.hazards.drought, drought_target);
        let flood_level = blend_hazard(region.hazards.flood, flood_target);
        if drought_level != region.hazards.drought || flood_level != region.hazards.flood {
            diff.record_hazard(region.index(), drought_level, flood_level);
        }

        if drought_level > DROUGHT_ALERT_THRESHOLD {
            diff.record_cause(Entry::new(
                format!("region:{}/water", region.id),
                Code::DroughtFlag,
                Some(format!("level={}", drought_level)),
            ));
        } else if flood_level > FLOOD_ALERT_THRESHOLD {
            diff.record_cause(Entry::new(
                format!("region:{}/water", region.id),
                Code::FloodFlag,
                Some(format!("level={}", flood_level)),
            ));
        }

        if new_soil < 2_500 {
            diff.record_cause(Entry::new(
                format!("region:{}/soil", region.id),
                Code::SoilFertilityLow,
                Some(format!("value={}", new_soil)),
            ));
        }
    }

    Ok(diff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Stream;
    use crate::{reduce, world};
    use proptest::prelude::*;

    #[test]
    fn ecology_moves_resources_toward_targets() {
        let world = crate::world::World::new(
            5,
            1,
            1,
            vec![crate::world::Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 100,
                latitude_deg: 0.0,
                biome: 5,
                water: 2_000,
                soil: 2_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 350,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: crate::world::Hazards::default(),
            }],
        );
        let mut rng = Stream::from(world.seed, STAGE, 1);
        let diff = update(&world, &mut rng).unwrap();
        let water_delta = diff.water.first().map(|delta| delta.delta).unwrap_or(0);
        assert!(water_delta.is_positive());
    }

    proptest! {
        #[test]
        fn ecology_diff_keeps_resources_within_bounds(
            water in 0u16..=WATER_MAX,
            soil in 0u16..=SOIL_MAX,
            biome in 0u8..=5
        ) {
            use crate::world::{Hazards, Region, World};
            let world = World::new(
                1,
                1,
                1,
                vec![Region {
                    id: 0,
                    x: 0,
                    y: 0,
                    elevation_m: 100,
                    latitude_deg: 0.0,
                    biome,
                    water,
                    soil,
                    temperature_tenths_c: 0,
                    precipitation_mm: 0,
                    albedo_milli: 350,
                    freshwater_flux_tenths_mm: 0,
                    ice_mass_kilotons: 0,
                    hazards: Hazards::default(),
                }],
            );
            let mut rng = Stream::from(world.seed, STAGE, 1);
            let diff = update(&world, &mut rng).unwrap();
            let water_delta = diff
                .water
                .first()
                .map(|delta| delta.delta)
                .unwrap_or(0);
            let soil_delta = diff
                .soil
                .first()
                .map(|delta| delta.delta)
                .unwrap_or(0);
            let next_water = clamp_u16(water as i32 + water_delta, 0, WATER_MAX);
            let next_soil = clamp_u16(soil as i32 + soil_delta, 0, SOIL_MAX);
            prop_assert!(next_water <= WATER_MAX);
            prop_assert!(next_soil <= SOIL_MAX);
            prop_assert!(i32::from(next_water) >= 0);
            prop_assert!(i32::from(next_soil) >= 0);
        }
    }

    #[test]
    fn hazard_gauges_decay_without_new_stressors() {
        let mut level = 6_000u16;
        let expected = [3_000, 1_500, 750, 375, 187, 93, 46, 23, 11, 5, 2, 1, 0];
        for &value in &expected {
            level = blend_hazard(level, 0);
            assert_eq!(level, value);
        }
        assert_eq!(blend_hazard(0, 6_000), 3_000);
        assert_eq!(blend_hazard(1, 0), 0);
    }

    #[test]
    fn flood_hazard_diff_records_decay() {
        let seed = find_zero_noise_seed().expect("seed for deterministic noise");
        let mut world = world::World::new(
            seed,
            1,
            1,
            vec![world::Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 10,
                latitude_deg: 0.0,
                biome: 5,
                water: 8_500,
                soil: 7_500,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 400,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: world::Hazards {
                    drought: 0,
                    flood: 6_000,
                },
            }],
        );

        let mut rng = Stream::from(world.seed, STAGE, 1);
        let expected_levels = [3_000, 1_500, 750, 375, 187, 93, 46, 23, 11, 5, 2, 1, 0];
        for &expected in &expected_levels {
            let diff = update(&world, &mut rng).expect("ecology update");
            let hazard = diff
                .hazards
                .iter()
                .find(|event| event.region == 0)
                .map(|event| event.flood);

            assert_eq!(hazard.unwrap_or(0), expected);
            reduce::apply(&mut world, diff);
            assert_eq!(world.regions[0].hazards.flood, expected);
        }
    }

    fn find_zero_noise_seed() -> Option<u64> {
        for seed in 0..10_000 {
            let stream = Stream::from(seed, STAGE, 1);
            let mut region_stream = stream.derive(0);
            let noise = (region_stream.next_signed_unit() * 25.0) as i32;
            if noise == 0 {
                return Some(seed);
            }
        }
        None
    }
}
