use serde::{Deserialize, Serialize};

/// Hazard gauges for a region.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hazards {
    pub drought: u16,
    pub flood: u16,
}

impl Default for Hazards {
    fn default() -> Self {
        Self {
            drought: 0,
            flood: 0,
        }
    }
}

/// Region level state tracked by the simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Region {
    pub id: u32,
    pub x: u32,
    pub y: u32,
    pub elevation_m: i32,
    pub latitude_deg: f64,
    pub biome: u8,
    pub water: u16,
    pub soil: u16,
    pub temperature_tenths_c: i16,
    pub precipitation_mm: u16,
    pub hazards: Hazards,
}

impl Region {
    pub fn index(&self) -> usize {
        self.id as usize
    }
}

/// Global world state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub tick: u64,
    pub seed: u64,
    pub width: u32,
    pub height: u32,
    pub regions: Vec<Region>,
}

impl World {
    pub fn new(seed: u64, width: u32, height: u32, regions: Vec<Region>) -> Self {
        Self {
            tick: 0,
            seed,
            width,
            height,
            regions,
        }
    }

    pub fn region_key(index: usize) -> String {
        format!("r:{}", index)
    }

    pub fn region_index_from_key(key: &str) -> Option<usize> {
        key.strip_prefix("r:").and_then(|v| v.parse::<usize>().ok())
    }
}
