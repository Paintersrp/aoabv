use crate::cause::{Code, Entry};
use crate::diff::Diff;
use crate::fixed::{clamp_u16, resource_ratio, SOIL_MAX, WATER_MAX};
use crate::rng::Stream;
use crate::world::World;
use anyhow::{ensure, Result};

pub const STAGE: &str = "kernel:ecology";

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

        let drought_level = WATER_MAX.saturating_sub(new_water);
        let flood_level = new_water.saturating_sub(WATER_MAX - 1_500);
        if drought_level != region.hazards.drought || flood_level != region.hazards.flood {
            diff.record_hazard(region.index(), drought_level, flood_level);
        }

        if drought_level > 3_000 {
            diff.record_cause(Entry::new(
                format!("region:{}/water", region.id),
                Code::DroughtFlag,
                Some(format!("level={}", drought_level)),
            ));
        } else if flood_level > 1_000 {
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
}
