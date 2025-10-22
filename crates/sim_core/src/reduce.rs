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
