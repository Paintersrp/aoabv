use crate::diff::Diff;
use crate::fixed::{apply_resource_delta, clamp_resource};
use crate::world::World;

pub fn apply_diff(world: &mut World, diff: &Diff) {
    for (key, value) in &diff.biome {
        if let Some(index) = World::region_index_from_key(key) {
            if let Some(region) = world.regions.get_mut(index) {
                region.biome = (*value).clamp(0, u8::MAX as i32) as u8;
            }
        }
    }

    for (key, delta) in &diff.water {
        if let Some(index) = World::region_index_from_key(key) {
            if let Some(region) = world.regions.get_mut(index) {
                region.water = apply_resource_delta(region.water, *delta);
            }
        }
    }

    for (key, delta) in &diff.soil {
        if let Some(index) = World::region_index_from_key(key) {
            if let Some(region) = world.regions.get_mut(index) {
                region.soil = apply_resource_delta(region.soil, *delta);
            }
        }
    }

    for hazard in &diff.hazards {
        if let Some(region) = world.regions.get_mut(hazard.region as usize) {
            region.hazards.drought = clamp_resource(hazard.drought as i32);
            region.hazards.flood = clamp_resource(hazard.flood as i32);
        }
    }
}
