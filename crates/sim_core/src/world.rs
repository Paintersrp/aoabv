use std::collections::VecDeque;

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
    #[serde(skip)]
    pub temperature_maxima: Vec<VecDeque<i16>>,
    #[serde(skip)]
    pub precipitation_peaks: Vec<VecDeque<u16>>,
    pub sea_level_equivalent_mm: i32,
}

pub(crate) const EXTREME_WINDOW: usize = 6; // TODO(agents): rationale

impl ClimateState {
    pub fn from_regions(regions: &[Region]) -> Self {
        let temperature_baseline_tenths = vec![0; regions.len()];
        let last_albedo_milli = regions
            .iter()
            .map(|region| i32::from(region.albedo_milli))
            .collect();
        let last_insolation_tenths = vec![0; regions.len()];
        let mut temperature_maxima = Vec::with_capacity(regions.len());
        let mut precipitation_peaks = Vec::with_capacity(regions.len());
        for _ in regions {
            temperature_maxima.push(Self::new_temperature_window());
            precipitation_peaks.push(Self::new_precipitation_window());
        }
        Self {
            temperature_baseline_tenths,
            last_albedo_milli,
            last_insolation_tenths,
            temperature_maxima,
            precipitation_peaks,
            sea_level_equivalent_mm: 0,
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
        if self.temperature_maxima.len() < region_count {
            let missing = region_count - self.temperature_maxima.len();
            self.temperature_maxima
                .extend((0..missing).map(|_| Self::new_temperature_window()));
        }
        if self.precipitation_peaks.len() < region_count {
            let missing = region_count - self.precipitation_peaks.len();
            self.precipitation_peaks
                .extend((0..missing).map(|_| Self::new_precipitation_window()));
        }
    }

    pub fn sea_level_equivalent_mm(&self) -> i32 {
        self.sea_level_equivalent_mm
    }

    pub fn add_sea_level_equivalent_mm(&mut self, delta_mm: i32) {
        if delta_mm == 0 {
            return;
        }
        self.sea_level_equivalent_mm = self.sea_level_equivalent_mm.saturating_add(delta_mm);
    }

    fn new_temperature_window() -> VecDeque<i16> {
        VecDeque::from(vec![0; EXTREME_WINDOW])
    }

    fn new_precipitation_window() -> VecDeque<u16> {
        VecDeque::from(vec![0; EXTREME_WINDOW])
    }
}

#[cfg(test)]
mod tests {
    use super::{ClimateState, Region, EXTREME_WINDOW};

    #[test]
    fn sea_level_accumulator_saturates_and_tracks_delta() {
        let regions = vec![Region {
            id: 0,
            x: 0,
            y: 0,
            elevation_m: 0,
            latitude_deg: 0.0,
            biome: 0,
            water: 0,
            soil: 0,
            temperature_tenths_c: 0,
            precipitation_mm: 0,
            albedo_milli: 0,
            freshwater_flux_tenths_mm: 0,
            ice_mass_kilotons: 0,
            hazards: crate::world::Hazards::default(),
        }];

        let mut climate = ClimateState::from_regions(&regions);
        climate.add_sea_level_equivalent_mm(12);
        assert_eq!(climate.sea_level_equivalent_mm(), 12);

        climate.add_sea_level_equivalent_mm(-2);
        assert_eq!(climate.sea_level_equivalent_mm(), 10);

        climate.add_sea_level_equivalent_mm(i32::MAX);
        assert_eq!(climate.sea_level_equivalent_mm(), i32::MAX);
    }

    #[test]
    fn extreme_windows_initialize_and_resize_with_zeros() {
        let mut regions = Vec::new();
        for id in 0..3 {
            regions.push(Region {
                id,
                x: id,
                y: 0,
                elevation_m: 0,
                latitude_deg: 0.0,
                biome: 0,
                water: 0,
                soil: 0,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 0,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: crate::world::Hazards::default(),
            });
        }

        let mut climate = ClimateState::from_regions(&regions);

        for window in &climate.temperature_maxima {
            assert_eq!(window.len(), EXTREME_WINDOW);
            assert!(window.iter().all(|value| *value == 0));
        }

        for window in &climate.precipitation_peaks {
            assert_eq!(window.len(), EXTREME_WINDOW);
            assert!(window.iter().all(|value| *value == 0));
        }

        regions.push(Region {
            id: 3,
            x: 3,
            y: 0,
            elevation_m: 0,
            latitude_deg: 0.0,
            biome: 0,
            water: 0,
            soil: 0,
            temperature_tenths_c: 0,
            precipitation_mm: 0,
            albedo_milli: 0,
            freshwater_flux_tenths_mm: 0,
            ice_mass_kilotons: 0,
            hazards: crate::world::Hazards::default(),
        });

        climate.ensure_region_capacity(regions.len());

        assert_eq!(climate.temperature_maxima.len(), regions.len());
        assert_eq!(climate.precipitation_peaks.len(), regions.len());
        assert!(climate
            .temperature_maxima
            .last()
            .unwrap()
            .iter()
            .all(|v| *v == 0));
        assert!(climate
            .precipitation_peaks
            .last()
            .unwrap()
            .iter()
            .all(|v| *v == 0));
    }
}
