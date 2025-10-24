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
    pub albedo_milli: u16,
    pub freshwater_flux_tenths_mm: u16,
    pub ice_mass_kilotons: u32,
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
    pub climate: ClimateState,
}

impl World {
    pub fn new(seed: u64, width: u32, height: u32, regions: Vec<Region>) -> Self {
        let climate = ClimateState::from_regions(&regions);
        Self {
            tick: 0,
            seed,
            width,
            height,
            regions,
            climate,
        }
    }

    pub fn region_key(index: usize) -> String {
        format!("r:{}", index)
    }

    pub fn region_index_from_key(key: &str) -> Option<usize> {
        key.strip_prefix("r:").and_then(|v| v.parse::<usize>().ok())
    }
}

/// Slow-changing climate coordination state carried between ticks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClimateState {
    pub temperature_baseline_tenths: Vec<i16>,
    pub last_albedo_milli: Vec<i32>,
    pub last_insolation_tenths: Vec<i32>,
}

impl ClimateState {
    pub fn from_regions(regions: &[Region]) -> Self {
        let temperature_baseline_tenths = vec![0; regions.len()];
        let last_albedo_milli = regions
            .iter()
            .map(|region| i32::from(region.albedo_milli))
            .collect();
        let last_insolation_tenths = vec![0; regions.len()];
        Self {
            temperature_baseline_tenths,
            last_albedo_milli,
            last_insolation_tenths,
        }
    }

    pub fn ensure_region_capacity(&mut self, region_count: usize) {
        if self.temperature_baseline_tenths.len() < region_count {
            self.temperature_baseline_tenths.resize(region_count, 0);
        }
        if self.last_albedo_milli.len() < region_count {
            self.last_albedo_milli.resize(region_count, 0);
        }
        if self.last_insolation_tenths.len() < region_count {
            self.last_insolation_tenths.resize(region_count, 0);
        }
    }
}
