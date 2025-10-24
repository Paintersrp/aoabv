use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::diff::Diff;
use crate::world::World;

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

#[derive(Clone, Debug, Default, Serialize)]
pub struct FrameDiff {
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub biome: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub insolation: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub tide_envelope: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub elevation: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub temp: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub precip: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub precip_extreme: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub humidity: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub albedo: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub permafrost_active: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub freshwater_flux: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub melt_pulse: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub ice_mass: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub heatwave_idx: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub diag_climate: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub soil: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub water: BTreeMap<String, i32>,
}

impl FrameDiff {
    fn is_empty(&self) -> bool {
        self.biome.is_empty()
            && self.insolation.is_empty()
            && self.tide_envelope.is_empty()
            && self.elevation.is_empty()
            && self.temp.is_empty()
            && self.precip.is_empty()
            && self.precip_extreme.is_empty()
            && self.humidity.is_empty()
            && self.albedo.is_empty()
            && self.permafrost_active.is_empty()
            && self.freshwater_flux.is_empty()
            && self.melt_pulse.is_empty()
            && self.ice_mass.is_empty()
            && self.heatwave_idx.is_empty()
            && self.diag_climate.is_empty()
            && self.soil.is_empty()
            && self.water.is_empty()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct FrameWorldMeta {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct Frame {
    pub t: u64,
    pub world: FrameWorldMeta,
    #[serde(skip_serializing_if = "FrameDiff::is_empty", default)]
    pub diff: FrameDiff,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub diagnostics: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub highlights: Vec<Highlight>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub chronicle: Vec<String>,
    pub era_end: bool,
}

pub fn make_frame(
    t: u64,
    diff: Diff,
    highlights: Vec<Highlight>,
    chronicle: Vec<String>,
    era_end: bool,
    width: u32,
    height: u32,
) -> Frame {
    let mut frame_diff = FrameDiff::default();
    for change in diff.biome {
        frame_diff
            .biome
            .insert(World::region_key(change.region as usize), change.biome);
    }
    for value in diff.insolation {
        frame_diff
            .insolation
            .insert(World::region_key(value.region as usize), value.value);
    }
    for value in diff.tide_envelope {
        frame_diff
            .tide_envelope
            .insert(World::region_key(value.region as usize), value.value);
    }
    for value in diff.elevation {
        frame_diff
            .elevation
            .insert(World::region_key(value.region as usize), value.value);
    }
    for value in diff.temperature {
        frame_diff
            .temp
            .insert(World::region_key(value.region as usize), value.value);
    }
    for value in diff.precipitation {
        frame_diff
            .precip
            .insert(World::region_key(value.region as usize), value.value);
    }
    for value in diff.precip_extreme {
        frame_diff
            .precip_extreme
            .insert(World::region_key(value.region as usize), value.value);
    }
    for value in diff.humidity {
        frame_diff
            .humidity
            .insert(World::region_key(value.region as usize), value.value);
    }
    for value in diff.albedo {
        frame_diff
            .albedo
            .insert(World::region_key(value.region as usize), value.value);
    }
    for value in diff.permafrost_active {
        frame_diff
            .permafrost_active
            .insert(World::region_key(value.region as usize), value.value);
    }
    for value in diff.freshwater_flux {
        frame_diff
            .freshwater_flux
            .insert(World::region_key(value.region as usize), value.value);
    }
    for value in diff.melt_pulse {
        frame_diff
            .melt_pulse
            .insert(World::region_key(value.region as usize), value.value);
    }
    for value in diff.ice_mass {
        frame_diff
            .ice_mass
            .insert(World::region_key(value.region as usize), value.value);
    }
    for value in diff.heatwave_idx {
        frame_diff
            .heatwave_idx
            .insert(World::region_key(value.region as usize), value.value);
    }
    for value in diff.diag_climate {
        frame_diff
            .diag_climate
            .insert(World::region_key(value.region as usize), value.value);
    }
    for delta in diff.soil {
        frame_diff
            .soil
            .insert(World::region_key(delta.region as usize), delta.delta);
    }
    for delta in diff.water {
        frame_diff
            .water
            .insert(World::region_key(delta.region as usize), delta.delta);
    }

    Frame {
        t,
        diff: frame_diff,
        diagnostics: diff.diagnostics,
        world: FrameWorldMeta { width, height },
        highlights,
        chronicle,
        era_end,
    }
}

impl Frame {
    pub fn to_ndjson(&self) -> serde_json::Result<String> {
        let mut json = serde_json::to_string(self)?;
        json.push('\n');
        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn frame_diff_excludes_hazards_key() {
        let mut diff = Diff::default();
        diff.record_biome(0, 3);
        diff.record_water_delta(1, 5);
        diff.record_soil_delta(2, -7);
        diff.record_hazard(0, 4_500, 0);

        let frame = make_frame(1, diff, Vec::new(), Vec::new(), false, 8, 4);
        let json_line = frame.to_ndjson().expect("frame serializes");
        let value: serde_json::Value =
            serde_json::from_str(json_line.trim_end()).expect("valid json");
        let diff_value = value.get("diff").expect("diff field present");
        let diff_map = diff_value.as_object().expect("diff is object");

        for key in diff_map.keys() {
            assert!(key == "biome" || key == "water" || key == "soil" || key == "ice_mass");
        }
        assert!(!diff_map.contains_key("hazards"));
    }

    #[test]
    fn frame_diff_includes_ice_mass_map() {
        let mut diff = Diff::default();
        diff.record_ice_mass(0, 12_500);

        let frame = make_frame(2, diff, Vec::new(), Vec::new(), false, 4, 4);
        let json_line = frame.to_ndjson().expect("frame serializes");
        let value: serde_json::Value =
            serde_json::from_str(json_line.trim_end()).expect("valid json");
        let diff_value = value.get("diff").expect("diff field present");
        let ice_mass = diff_value
            .get("ice_mass")
            .expect("ice_mass field present")
            .as_object()
            .expect("ice_mass diff is object");
        assert_eq!(ice_mass.get("r:0").and_then(|v| v.as_i64()), Some(12_500));
    }

    #[test]
    fn frame_world_metadata_present() {
        let frame = make_frame(0, Diff::default(), Vec::new(), Vec::new(), false, 12, 6);
        let json_line = frame.to_ndjson().expect("frame serializes");
        let value: serde_json::Value =
            serde_json::from_str(json_line.trim_end()).expect("valid json");
        let world = value.get("world").expect("world metadata present");
        let world_map = world.as_object().expect("world is object");
        assert_eq!(world_map.get("width").and_then(|v| v.as_u64()), Some(12));
        assert_eq!(world_map.get("height").and_then(|v| v.as_u64()), Some(6));
    }

    #[test]
    fn frame_diff_serializes_scalar_maps() {
        let mut diff = Diff::default();
        diff.record_insolation(0, 12_345);
        diff.record_tide_envelope(1, -234);
        diff.record_elevation(2, 987);
        diff.record_temperature(3, 156);
        diff.record_precipitation(0, 2_345);
        diff.record_precip_extreme(1, 640);
        diff.record_albedo(1, 875);
        diff.record_freshwater_flux(2, 1_234);
        diff.record_heatwave_idx(0, 210);
        diff.record_diag_climate(0, -42);

        let frame = make_frame(5, diff, Vec::new(), Vec::new(), false, 8, 4);
        let json_line = frame.to_ndjson().expect("frame serializes");
        let value: serde_json::Value =
            serde_json::from_str(json_line.trim_end()).expect("valid json");
        let diff_value = value.get("diff").expect("diff field present");
        let diff_map = diff_value.as_object().expect("diff is object");

        let insolation = diff_map
            .get("insolation")
            .expect("insolation map present")
            .as_object()
            .expect("insolation is object");
        assert_eq!(insolation.get("r:0").and_then(|v| v.as_i64()), Some(12_345));

        let tide = diff_map
            .get("tide_envelope")
            .expect("tide map present")
            .as_object()
            .expect("tide is object");
        assert_eq!(tide.get("r:1").and_then(|v| v.as_i64()), Some(-234));

        let elevation = diff_map
            .get("elevation")
            .expect("elevation map present")
            .as_object()
            .expect("elevation is object");
        assert_eq!(elevation.get("r:2").and_then(|v| v.as_i64()), Some(987));

        let temp = diff_map
            .get("temp")
            .expect("temp map present")
            .as_object()
            .expect("temp is object");
        assert_eq!(temp.get("r:3").and_then(|v| v.as_i64()), Some(156));

        let precip = diff_map
            .get("precip")
            .expect("precip map present")
            .as_object()
            .expect("precip is object");
        assert_eq!(precip.get("r:0").and_then(|v| v.as_i64()), Some(2_345));

        let precip_extreme = diff_map
            .get("precip_extreme")
            .expect("precip_extreme map present")
            .as_object()
            .expect("precip_extreme is object");
        assert_eq!(
            precip_extreme.get("r:1").and_then(|v| v.as_i64()),
            Some(640)
        );

        let albedo = diff_map
            .get("albedo")
            .expect("albedo map present")
            .as_object()
            .expect("albedo is object");
        assert_eq!(albedo.get("r:1").and_then(|v| v.as_i64()), Some(875));

        let freshwater = diff_map
            .get("freshwater_flux")
            .expect("freshwater map present")
            .as_object()
            .expect("freshwater is object");
        assert_eq!(freshwater.get("r:2").and_then(|v| v.as_i64()), Some(1_234));

        let heatwave_idx = diff_map
            .get("heatwave_idx")
            .expect("heatwave_idx map present")
            .as_object()
            .expect("heatwave_idx is object");
        assert_eq!(heatwave_idx.get("r:0").and_then(|v| v.as_i64()), Some(210));

        let diag_climate = diff_map
            .get("diag_climate")
            .expect("diag_climate map present")
            .as_object()
            .expect("diag_climate is object");
        assert_eq!(diag_climate.get("r:0").and_then(|v| v.as_i64()), Some(-42));
    }

    #[test]
    fn frame_carries_diagnostics_map() {
        let mut diff = Diff::default();
        diff.record_diagnostic("energy_balance", -1);
        diff.record_diagnostic("albedo_anomaly_milli", -12);

        let frame = make_frame(5, diff, Vec::new(), Vec::new(), false, 4, 1);
        let json_line = frame.to_ndjson().expect("frame serializes");
        let value: serde_json::Value =
            serde_json::from_str(json_line.trim_end()).expect("valid json");
        let diagnostics = value
            .get("diagnostics")
            .expect("diagnostics present")
            .as_object()
            .expect("diagnostics is object");
        assert_eq!(
            diagnostics.get("energy_balance").and_then(|v| v.as_i64()),
            Some(-1)
        );
        assert_eq!(
            diagnostics
                .get("albedo_anomaly_milli")
                .and_then(|v| v.as_i64()),
            Some(-12)
        );
    }

    proptest! {
        #[test]
        fn frame_diff_region_keys_are_well_formed(values in proptest::collection::vec(-5_000i32..5_000, 4)) {
            let mut diff = Diff::default();
            let regions = [3usize, 1, 2, 0];
            for (offset, region) in regions.iter().enumerate() {
                let base = values[offset];
                diff.record_biome(*region, ((offset as u8) + 1) % 6);
                diff.record_insolation(*region, base);
                diff.record_tide_envelope(*region, base - 10);
                diff.record_elevation(*region, base + 200);
                diff.record_temperature(*region, base);
                diff.record_temperature_baseline(*region, base / 2);
                diff.record_precipitation(*region, base.abs());
                let extreme = if base == 0 { 1 } else { base };
                diff.record_precip_extreme(*region, extreme);
                diff.record_humidity(*region, base + 50);
                let albedo = (base.abs() % 900) + 100;
                diff.record_albedo(*region, albedo);
                diff.record_permafrost_active(*region, base - 25);
                diff.record_freshwater_flux(*region, base.abs());
                let melt = if base == 0 { 5 } else { base.abs() };
                diff.record_melt_pulse(*region, melt);
                diff.record_ice_mass(*region, (base.abs() + 100) as i32);
                diff.record_heatwave_idx(*region, extreme);
                diff.record_water_delta(*region, extreme);
                diff.record_soil_delta(*region, -extreme);
            }
            diff.record_diag_climate(0, values[0]);
            diff.record_diag_climate(1, values[1]);

            let frame = make_frame(0, diff, Vec::new(), Vec::new(), false, 2, 2);
            let json_line = frame.to_ndjson().expect("frame serializes");
            let value: serde_json::Value = serde_json::from_str(json_line.trim_end()).expect("valid json");
            if let Some(diff_value) = value.get("diff") {
                let diff_map = diff_value.as_object().expect("diff is object");
                for map_value in diff_map.values() {
                    let entries = map_value.as_object().expect("entries are maps");
                    for (key, entry_value) in entries {
                        prop_assert!(key.starts_with("r:"));
                        prop_assert!(key[2..].chars().all(|c| c.is_ascii_digit()));
                        prop_assert!(entry_value.as_i64().is_some());
                    }
                }
            }
        }
    }
}
