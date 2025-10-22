use serde::ser::{SerializeMap, SerializeStruct};
use serde::{Deserialize, Serialize};

use crate::cause::Cause;
use crate::world::World;

#[derive(Clone, Debug, Default)]
pub struct Diff {
    pub biome: Vec<BiomeChange>,
    pub water: Vec<ResourceDelta>,
    pub soil: Vec<ResourceDelta>,
    pub hazards: Vec<HazardEvent>,
    pub causes: Vec<Cause>,
}

impl Diff {
    pub fn record_biome(&mut self, region_index: usize, biome: u8) {
        self.set_biome_value(region_index as u32, biome as i32);
    }

    pub fn record_water_delta(&mut self, region_index: usize, delta: i32) {
        if delta == 0 {
            return;
        }
        Self::insert_delta(&mut self.water, region_index as u32, delta);
    }

    pub fn record_soil_delta(&mut self, region_index: usize, delta: i32) {
        if delta == 0 {
            return;
        }
        Self::insert_delta(&mut self.soil, region_index as u32, delta);
    }

    pub fn record_hazard(&mut self, region_index: usize, drought: u16, flood: u16) {
        let region = region_index as u32;
        match self.hazards.binary_search_by_key(&region, |h| h.region) {
            Ok(idx) => {
                self.hazards[idx].drought = drought;
                self.hazards[idx].flood = flood;
            }
            Err(idx) => self.hazards.insert(
                idx,
                HazardEvent {
                    region,
                    drought,
                    flood,
                },
            ),
        }
    }

    pub fn record_cause(&mut self, cause: Cause) {
        self.causes.push(cause);
    }

    pub fn extend_causes<I>(&mut self, causes: I)
    where
        I: IntoIterator<Item = Cause>,
    {
        self.causes.extend(causes);
    }

    pub fn merge(&mut self, other: &Diff) {
        for change in &other.biome {
            self.set_biome_value(change.region, change.biome);
        }
        for delta in &other.water {
            Self::insert_delta(&mut self.water, delta.region, delta.delta);
        }
        for delta in &other.soil {
            Self::insert_delta(&mut self.soil, delta.region, delta.delta);
        }
        for hazard in &other.hazards {
            self.record_hazard(hazard.region as usize, hazard.drought, hazard.flood);
        }
        self.causes.extend(other.causes.iter().cloned());
    }

    pub fn take_causes(&mut self) -> Vec<Cause> {
        std::mem::take(&mut self.causes)
    }

    pub fn is_empty(&self) -> bool {
        self.biome.is_empty()
            && self.water.is_empty()
            && self.soil.is_empty()
            && self.hazards.is_empty()
            && self.causes.is_empty()
    }

    fn set_biome_value(&mut self, region: u32, biome: i32) {
        match self
            .biome
            .binary_search_by_key(&region, |change| change.region)
        {
            Ok(idx) => self.biome[idx].biome = biome,
            Err(idx) => self.biome.insert(idx, BiomeChange { region, biome }),
        }
    }

    fn insert_delta(target: &mut Vec<ResourceDelta>, region: u32, delta: i32) {
        match target.binary_search_by_key(&region, |entry| entry.region) {
            Ok(idx) => {
                let entry = &mut target[idx];
                entry.delta += delta;
                if entry.delta == 0 {
                    target.remove(idx);
                }
            }
            Err(idx) => target.insert(idx, ResourceDelta { region, delta }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BiomeChange {
    pub region: u32,
    pub biome: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceDelta {
    pub region: u32,
    pub delta: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HazardEvent {
    pub region: u32,
    pub drought: u16,
    pub flood: u16,
}

impl Serialize for Diff {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut field_count = 0;
        if !self.biome.is_empty() {
            field_count += 1;
        }
        if !self.water.is_empty() {
            field_count += 1;
        }
        if !self.soil.is_empty() {
            field_count += 1;
        }
        if !self.hazards.is_empty() {
            field_count += 1;
        }
        let mut state = serializer.serialize_struct("Diff", field_count)?;
        if !self.biome.is_empty() {
            state.serialize_field("biome", &BiomeChanges(&self.biome))?;
        }
        if !self.water.is_empty() {
            state.serialize_field("water", &ResourceDeltas(&self.water))?;
        }
        if !self.soil.is_empty() {
            state.serialize_field("soil", &ResourceDeltas(&self.soil))?;
        }
        if !self.hazards.is_empty() {
            state.serialize_field("hazards", &self.hazards)?;
        }
        state.end()
    }
}

struct BiomeChanges<'a>(&'a [BiomeChange]);

impl<'a> Serialize for BiomeChanges<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for change in self.0 {
            let key = World::region_key(change.region as usize);
            map.serialize_entry(&key, &change.biome)?;
        }
        map.end()
    }
}

struct ResourceDeltas<'a>(&'a [ResourceDelta]);

impl<'a> Serialize for ResourceDeltas<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for delta in self.0 {
            let key = World::region_key(delta.region as usize);
            map.serialize_entry(&key, &delta.delta)?;
        }
        map.end()
    }
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
