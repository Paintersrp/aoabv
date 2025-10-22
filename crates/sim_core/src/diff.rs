use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::world::World;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Diff {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub biome: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub water: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub soil: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hazards: Vec<HazardDiff>,
}

impl Diff {
    pub fn record_biome(&mut self, region_index: usize, biome: u8) {
        let key = World::region_key(region_index);
        self.biome.insert(key, biome as i32);
    }

    pub fn record_water_delta(&mut self, region_index: usize, delta: i32) {
        if delta == 0 {
            return;
        }
        let key = World::region_key(region_index);
        *self.water.entry(key).or_insert(0) += delta;
    }

    pub fn record_soil_delta(&mut self, region_index: usize, delta: i32) {
        if delta == 0 {
            return;
        }
        let key = World::region_key(region_index);
        *self.soil.entry(key).or_insert(0) += delta;
    }

    pub fn record_hazard(&mut self, region_index: usize, drought: u16, flood: u16) {
        let hazard = HazardDiff {
            region: region_index as u32,
            drought,
            flood,
        };
        if let Some(existing) = self.hazards.iter_mut().find(|h| h.region == hazard.region) {
            existing.drought = hazard.drought;
            existing.flood = hazard.flood;
        } else {
            self.hazards.push(hazard);
            self.hazards.sort_by_key(|h| h.region);
        }
    }

    pub fn merge(&mut self, other: &Diff) {
        for (key, value) in &other.biome {
            self.biome.insert(key.clone(), *value);
        }
        for (key, delta) in &other.water {
            *self.water.entry(key.clone()).or_insert(0) += delta;
        }
        for (key, delta) in &other.soil {
            *self.soil.entry(key.clone()).or_insert(0) += delta;
        }
        for hazard in &other.hazards {
            self.record_hazard(hazard.region as usize, hazard.drought, hazard.flood);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.biome.is_empty()
            && self.water.is_empty()
            && self.soil.is_empty()
            && self.hazards.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HazardDiff {
    pub region: u32,
    pub drought: u16,
    pub flood: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Highlight {
    #[serde(rename = "type")]
    pub kind: String,
    pub region: u32,
    pub info: HighlightInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HighlightInfo {
    pub kind: String,
    pub level: f32,
}

impl Highlight {
    pub fn hazard(region: u32, kind: &str, level: f32) -> Self {
        Self {
            kind: "hazard_flag".to_string(),
            region,
            info: HighlightInfo {
                kind: kind.to_string(),
                level,
            },
        }
    }
}
