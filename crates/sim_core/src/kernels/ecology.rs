use crate::cause::{Code, Entry};
use crate::diff::{Diff, Highlight};
use crate::fixed::{apply_resource_delta, resource_ratio, RESOURCE_MAX};
use crate::rng::StageRng;
use crate::world::World;

pub struct EcologyOutput {
    pub diff: Diff,
    pub highlights: Vec<Highlight>,
    pub chronicle: Vec<String>,
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

pub fn run(world: &World, rng: &mut StageRng) -> EcologyOutput {
    let mut diff = Diff::default();
    let mut highlights = Vec::new();
    let mut chronicle = Vec::new();

    for region in &world.regions {
        let mut region_rng = rng.fork_region(region.index());
        let profile = profile_for_biome(region.biome);
        let water_ratio = resource_ratio(region.water);
        let soil_ratio = resource_ratio(region.soil);

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

        let new_water = apply_resource_delta(region.water, water_delta);
        let new_soil = apply_resource_delta(region.soil, soil_delta);

        let drought_level = RESOURCE_MAX.saturating_sub(new_water);
        let flood_level = new_water.saturating_sub(RESOURCE_MAX - 1_500);
        if drought_level != region.hazards.drought || flood_level != region.hazards.flood {
            diff.record_hazard(region.index(), drought_level, flood_level);
        }

        if drought_level > 3_000 {
            highlights.push(Highlight::hazard(
                region.id,
                "drought",
                drought_level as f32 / RESOURCE_MAX as f32,
            ));
            chronicle.push(format!("Region {} faces an extended dry spell.", region.id));
            diff.record_cause(Entry::new(
                format!("region:{}/water", region.id),
                Code::DroughtFlag,
                Some(format!("level={}", drought_level)),
            ));
        } else if flood_level > 1_000 {
            highlights.push(Highlight::hazard(
                region.id,
                "flood",
                flood_level as f32 / RESOURCE_MAX as f32,
            ));
            chronicle.push(format!("Region {} endures seasonal floods.", region.id));
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

    EcologyOutput {
        diff,
        highlights,
        chronicle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::ProjectRng;
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
        let mut rng = ProjectRng::new(world.seed).stage(crate::rng::Stage::Ecology, 1);
        let output = run(&world, &mut rng);
        let water_delta = output
            .diff
            .water
            .first()
            .map(|delta| delta.delta)
            .unwrap_or(0);
        assert!(water_delta.is_positive());
    }

    proptest! {
        #[test]
        fn ecology_diff_keeps_resources_within_bounds(
            water in 0u16..=RESOURCE_MAX,
            soil in 0u16..=RESOURCE_MAX,
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
            let mut rng = crate::rng::ProjectRng::new(world.seed).stage(crate::rng::Stage::Ecology, 1);
            let output = run(&world, &mut rng);
            let water_delta = output
                .diff
                .water
                .first()
                .map(|delta| delta.delta)
                .unwrap_or(0);
            let soil_delta = output
                .diff
                .soil
                .first()
                .map(|delta| delta.delta)
                .unwrap_or(0);
            let next_water = apply_resource_delta(water, water_delta);
            let next_soil = apply_resource_delta(soil, soil_delta);
            prop_assert!(next_water <= RESOURCE_MAX);
            prop_assert!(next_soil <= RESOURCE_MAX);
            prop_assert!(next_water >= 0);
            prop_assert!(next_soil >= 0);
        }
    }
}
