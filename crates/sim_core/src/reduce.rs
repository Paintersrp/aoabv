use crate::diff::Diff;
use crate::fixed::{
    clamp_biome_index, clamp_hazard_meter, commit_resource_delta, SOIL_MAX, WATER_MAX,
};
use crate::world::World;

pub fn apply(world: &mut World, mut diff: Diff) {
    diff.biome.sort_by_key(|change| change.region);
    diff.water.sort_by_key(|delta| delta.region);
    diff.soil.sort_by_key(|delta| delta.region);
    diff.hazards.sort_by_key(|hazard| hazard.region);

    for change in diff.biome {
        if let Some(region) = world.regions.get_mut(change.region as usize) {
            region.biome = clamp_biome_index(change.biome);
        }
    }

    for delta in diff.water {
        if let Some(region) = world.regions.get_mut(delta.region as usize) {
            region.water = commit_resource_delta(region.water, delta.delta, WATER_MAX);
        }
    }

    for delta in diff.soil {
        if let Some(region) = world.regions.get_mut(delta.region as usize) {
            region.soil = commit_resource_delta(region.soil, delta.delta, SOIL_MAX);
        }
    }

    for hazard in diff.hazards {
        if let Some(region) = world.regions.get_mut(hazard.region as usize) {
            region.hazards.drought = clamp_hazard_meter(hazard.drought);
            region.hazards.flood = clamp_hazard_meter(hazard.flood);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{BiomeChange, HazardEvent, ResourceDelta};
    use crate::world::{Hazards, Region};

    fn test_world() -> World {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 0,
                latitude_deg: 0.0,
                biome: 1,
                water: 1_000,
                soil: 9_000,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 0,
                latitude_deg: 10.0,
                biome: 2,
                water: 5_000,
                soil: 100,
                hazards: Hazards::default(),
            },
            Region {
                id: 2,
                x: 0,
                y: 1,
                elevation_m: 0,
                latitude_deg: -10.0,
                biome: 3,
                water: 9_900,
                soil: 6_000,
                hazards: Hazards::default(),
            },
            Region {
                id: 3,
                x: 1,
                y: 1,
                elevation_m: 0,
                latitude_deg: 20.0,
                biome: 4,
                water: 100,
                soil: 5_000,
                hazards: Hazards::default(),
            },
        ];

        World::new(0, 2, 2, regions)
    }

    #[test]
    fn apply_sorts_entries_and_clamps_values() {
        let mut unsorted_diff = Diff::default();
        unsorted_diff.biome = vec![
            BiomeChange {
                region: 2,
                biome: 999,
            },
            BiomeChange {
                region: 0,
                biome: -5,
            },
            BiomeChange {
                region: 3,
                biome: 128,
            },
            BiomeChange {
                region: 1,
                biome: 42,
            },
        ];
        unsorted_diff.water = vec![
            ResourceDelta {
                region: 3,
                delta: -200,
            },
            ResourceDelta {
                region: 0,
                delta: 12_000,
            },
            ResourceDelta {
                region: 2,
                delta: 200,
            },
            ResourceDelta {
                region: 1,
                delta: -6_000,
            },
        ];
        unsorted_diff.soil = vec![
            ResourceDelta {
                region: 1,
                delta: 200,
            },
            ResourceDelta {
                region: 0,
                delta: -9_500,
            },
            ResourceDelta {
                region: 3,
                delta: -200,
            },
            ResourceDelta {
                region: 2,
                delta: 5_000,
            },
        ];
        unsorted_diff.hazards = vec![
            HazardEvent {
                region: 3,
                drought: 15_000,
                flood: 200,
            },
            HazardEvent {
                region: 0,
                drought: 5,
                flood: 700,
            },
            HazardEvent {
                region: 2,
                drought: 65_000,
                flood: 65_535,
            },
            HazardEvent {
                region: 1,
                drought: 250,
                flood: 12_000,
            },
        ];

        let mut sorted_diff = unsorted_diff.clone();
        sorted_diff.biome.sort_by_key(|change| change.region);
        sorted_diff.water.sort_by_key(|delta| delta.region);
        sorted_diff.soil.sort_by_key(|delta| delta.region);
        sorted_diff.hazards.sort_by_key(|hazard| hazard.region);

        let mut world_from_unsorted = test_world();
        let mut world_from_sorted = test_world();

        apply(&mut world_from_unsorted, unsorted_diff);
        apply(&mut world_from_sorted, sorted_diff);

        for (left, right) in world_from_unsorted
            .regions
            .iter()
            .zip(world_from_sorted.regions.iter())
        {
            assert_eq!(left.id, right.id);
            assert_eq!(left.biome, right.biome);
            assert_eq!(left.water, right.water);
            assert_eq!(left.soil, right.soil);
            assert_eq!(left.hazards.drought, right.hazards.drought);
            assert_eq!(left.hazards.flood, right.hazards.flood);
        }

        let region0 = &world_from_unsorted.regions[0];
        assert_eq!(region0.biome, 0);
        assert_eq!(region0.water, crate::fixed::WATER_MAX);
        assert_eq!(region0.soil, 0);
        assert_eq!(region0.hazards.drought, 5);
        assert_eq!(region0.hazards.flood, 700);

        let region1 = &world_from_unsorted.regions[1];
        assert_eq!(region1.biome, 42);
        assert_eq!(region1.water, 0);
        assert_eq!(region1.soil, 300);
        assert_eq!(region1.hazards.drought, 250);
        assert_eq!(region1.hazards.flood, crate::fixed::WATER_MAX);

        let region2 = &world_from_unsorted.regions[2];
        assert_eq!(region2.biome, u8::MAX);
        assert_eq!(region2.water, crate::fixed::WATER_MAX);
        assert_eq!(region2.soil, crate::fixed::SOIL_MAX);
        assert_eq!(region2.hazards.drought, crate::fixed::WATER_MAX);
        assert_eq!(region2.hazards.flood, crate::fixed::WATER_MAX);

        let region3 = &world_from_unsorted.regions[3];
        assert_eq!(region3.biome, 128);
        assert_eq!(region3.water, 0);
        assert_eq!(region3.soil, 4_800);
        assert_eq!(region3.hazards.drought, crate::fixed::WATER_MAX);
        assert_eq!(region3.hazards.flood, 200);
    }
}
