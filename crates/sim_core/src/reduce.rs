use crate::diff::Diff;
use crate::fixed::{apply_resource_delta, clamp_resource};
use crate::world::World;

pub fn apply_diff(world: &mut World, diff: &Diff) {
    for change in &diff.biome {
        if let Some(region) = world.regions.get_mut(change.region as usize) {
            region.biome = change.biome.clamp(0, u8::MAX as i32) as u8;
        }
    }

    for delta in &diff.water {
        if let Some(region) = world.regions.get_mut(delta.region as usize) {
            region.water = apply_resource_delta(region.water, delta.delta);
        }
    }

    for delta in &diff.soil {
        if let Some(region) = world.regions.get_mut(delta.region as usize) {
            region.soil = apply_resource_delta(region.soil, delta.delta);
        }
    }

    for hazard in &diff.hazards {
        if let Some(region) = world.regions.get_mut(hazard.region as usize) {
            region.hazards.drought = clamp_resource(hazard.drought as i32);
            region.hazards.flood = clamp_resource(hazard.flood as i32);
        }
    }
}
