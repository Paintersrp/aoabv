use crate::diff::Diff;
use crate::fixed::{clamp_u16, SOIL_MAX, WATER_MAX};
use crate::world::World;

pub fn apply_diff(world: &mut World, diff: &Diff) {
    for change in &diff.biome {
        if let Some(region) = world.regions.get_mut(change.region as usize) {
            region.biome = change.biome.clamp(0, u8::MAX as i32) as u8;
        }
    }

    for delta in &diff.water {
        if let Some(region) = world.regions.get_mut(delta.region as usize) {
            let updated = region.water as i32 + delta.delta;
            region.water = clamp_u16(updated, 0, WATER_MAX);
        }
    }

    for delta in &diff.soil {
        if let Some(region) = world.regions.get_mut(delta.region as usize) {
            let updated = region.soil as i32 + delta.delta;
            region.soil = clamp_u16(updated, 0, SOIL_MAX);
        }
    }

    for hazard in &diff.hazards {
        if let Some(region) = world.regions.get_mut(hazard.region as usize) {
            region.hazards.drought = clamp_u16(hazard.drought as i32, 0, WATER_MAX);
            region.hazards.flood = clamp_u16(hazard.flood as i32, 0, WATER_MAX);
        }
    }
}
