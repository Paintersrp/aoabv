use std::cmp::Ordering;

use serde::ser::{SerializeMap, SerializeStruct};
use serde::{Deserialize, Serialize};

use crate::cause::Entry;
use crate::world::World;

#[derive(Clone, Debug, Default)]
pub struct Diff {
    pub biome: Vec<BiomeChange>,
    pub water: Vec<ResourceDelta>,
    pub soil: Vec<ResourceDelta>,
    pub insolation: Vec<ScalarValue>,
    pub tide_envelope: Vec<ScalarValue>,
    pub elevation: Vec<ScalarValue>,
    pub temperature: Vec<ScalarValue>,
    pub precipitation: Vec<ScalarValue>,
    pub humidity: Vec<ScalarValue>,
    pub albedo: Vec<ScalarValue>,
    pub freshwater_flux: Vec<ScalarValue>,
    pub ice_mass: Vec<ScalarValue>,
    pub hazards: Vec<HazardEvent>,
    pub causes: Vec<Entry>,
    pub diag_energy: Option<DiagEnergy>,
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

    pub fn record_insolation(&mut self, region_index: usize, value: i32) {
        Self::set_scalar_value(&mut self.insolation, region_index as u32, value);
    }

    pub fn record_tide_envelope(&mut self, region_index: usize, value: i32) {
        Self::set_scalar_value(&mut self.tide_envelope, region_index as u32, value);
    }

    pub fn record_elevation(&mut self, region_index: usize, value: i32) {
        Self::set_scalar_value(&mut self.elevation, region_index as u32, value);
    }

    pub fn record_temperature(&mut self, region_index: usize, value: i32) {
        Self::set_scalar_value(&mut self.temperature, region_index as u32, value);
    }

    pub fn record_precipitation(&mut self, region_index: usize, value: i32) {
        Self::set_scalar_value(&mut self.precipitation, region_index as u32, value);
    }

    pub fn record_humidity(&mut self, region_index: usize, value: i32) {
        Self::set_scalar_value(&mut self.humidity, region_index as u32, value);
    }

    pub fn record_albedo(&mut self, region_index: usize, value: i32) {
        Self::set_scalar_value(&mut self.albedo, region_index as u32, value);
    }

    pub fn record_freshwater_flux(&mut self, region_index: usize, value: i32) {
        Self::set_scalar_value(&mut self.freshwater_flux, region_index as u32, value);
    }

    pub fn record_ice_mass(&mut self, region_index: usize, value: i32) {
        Self::set_scalar_value(&mut self.ice_mass, region_index as u32, value);
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

    /// Insert a cause entry while maintaining a deterministic ordering by
    /// `(target, code, note)`.
    pub fn record_cause(&mut self, cause: Entry) {
        let position =
            self.causes
                .binary_search_by(|existing| match existing.target.cmp(&cause.target) {
                    Ordering::Equal => match existing.code.cmp(&cause.code) {
                        Ordering::Equal => existing.note.cmp(&cause.note),
                        other => other,
                    },
                    other => other,
                });
        match position {
            Ok(idx) => self.causes.insert(idx + 1, cause),
            Err(idx) => self.causes.insert(idx, cause),
        }
    }

    pub fn extend_causes<I>(&mut self, causes: I)
    where
        I: IntoIterator<Item = Entry>,
    {
        for cause in causes {
            self.record_cause(cause);
        }
    }

    pub fn record_diag_energy(&mut self, diag: DiagEnergy) {
        self.diag_energy = Some(diag);
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
        for scalar in &other.insolation {
            Self::set_scalar_value(&mut self.insolation, scalar.region, scalar.value);
        }
        for scalar in &other.tide_envelope {
            Self::set_scalar_value(&mut self.tide_envelope, scalar.region, scalar.value);
        }
        for scalar in &other.elevation {
            Self::set_scalar_value(&mut self.elevation, scalar.region, scalar.value);
        }
        for scalar in &other.temperature {
            Self::set_scalar_value(&mut self.temperature, scalar.region, scalar.value);
        }
        for scalar in &other.precipitation {
            Self::set_scalar_value(&mut self.precipitation, scalar.region, scalar.value);
        }
        for scalar in &other.humidity {
            Self::set_scalar_value(&mut self.humidity, scalar.region, scalar.value);
        }
        for scalar in &other.albedo {
            Self::set_scalar_value(&mut self.albedo, scalar.region, scalar.value);
        }
        for scalar in &other.freshwater_flux {
            Self::set_scalar_value(&mut self.freshwater_flux, scalar.region, scalar.value);
        }
        for scalar in &other.ice_mass {
            Self::set_scalar_value(&mut self.ice_mass, scalar.region, scalar.value);
        }
        for hazard in &other.hazards {
            self.record_hazard(hazard.region as usize, hazard.drought, hazard.flood);
        }
        for cause in other.causes.iter().cloned() {
            self.record_cause(cause);
        }
        if let Some(diag) = &other.diag_energy {
            self.diag_energy = Some(diag.clone());
        }
    }

    pub fn take_causes(&mut self) -> Vec<Entry> {
        std::mem::take(&mut self.causes)
    }

    pub fn is_empty(&self) -> bool {
        self.biome.is_empty()
            && self.water.is_empty()
            && self.soil.is_empty()
            && self.insolation.is_empty()
            && self.tide_envelope.is_empty()
            && self.elevation.is_empty()
            && self.temperature.is_empty()
            && self.precipitation.is_empty()
            && self.humidity.is_empty()
            && self.albedo.is_empty()
            && self.freshwater_flux.is_empty()
            && self.ice_mass.is_empty()
            && self.hazards.is_empty()
            && self.causes.is_empty()
            && self.diag_energy.is_none()
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

    fn set_scalar_value(target: &mut Vec<ScalarValue>, region: u32, value: i32) {
        match target.binary_search_by_key(&region, |entry| entry.region) {
            Ok(idx) => target[idx].value = value,
            Err(idx) => target.insert(idx, ScalarValue { region, value }),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalarValue {
    pub region: u32,
    pub value: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagEnergy {
    pub albedo_anomaly_milli: i32,
    pub temp_adjust_tenths: i32,
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
        if !self.insolation.is_empty() {
            field_count += 1;
        }
        if !self.tide_envelope.is_empty() {
            field_count += 1;
        }
        if !self.elevation.is_empty() {
            field_count += 1;
        }
        if !self.temperature.is_empty() {
            field_count += 1;
        }
        if !self.precipitation.is_empty() {
            field_count += 1;
        }
        if !self.humidity.is_empty() {
            field_count += 1;
        }
        if !self.albedo.is_empty() {
            field_count += 1;
        }
        if !self.freshwater_flux.is_empty() {
            field_count += 1;
        }
        if !self.ice_mass.is_empty() {
            field_count += 1;
        }
        if !self.hazards.is_empty() {
            field_count += 1;
        }
        if self.diag_energy.is_some() {
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
        if !self.insolation.is_empty() {
            state.serialize_field("insolation", &ScalarValues(&self.insolation))?;
        }
        if !self.tide_envelope.is_empty() {
            state.serialize_field("tide_envelope", &ScalarValues(&self.tide_envelope))?;
        }
        if !self.elevation.is_empty() {
            state.serialize_field("elevation", &ScalarValues(&self.elevation))?;
        }
        if !self.temperature.is_empty() {
            state.serialize_field("temp", &ScalarValues(&self.temperature))?;
        }
        if !self.precipitation.is_empty() {
            state.serialize_field("precip", &ScalarValues(&self.precipitation))?;
        }
        if !self.humidity.is_empty() {
            state.serialize_field("humidity", &ScalarValues(&self.humidity))?;
        }
        if !self.albedo.is_empty() {
            state.serialize_field("albedo", &ScalarValues(&self.albedo))?;
        }
        if !self.freshwater_flux.is_empty() {
            state.serialize_field("freshwater_flux", &ScalarValues(&self.freshwater_flux))?;
        }
        if !self.ice_mass.is_empty() {
            state.serialize_field("ice_mass", &ScalarValues(&self.ice_mass))?;
        }
        if !self.hazards.is_empty() {
            state.serialize_field("hazards", &self.hazards)?;
        }
        if let Some(diag) = &self.diag_energy {
            state.serialize_field("diag_energy", diag)?;
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

struct ScalarValues<'a>(&'a [ScalarValue]);

impl<'a> Serialize for ScalarValues<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for value in self.0 {
            let key = World::region_key(value.region as usize);
            map.serialize_entry(&key, &value.value)?;
        }
        map.end()
    }
}
