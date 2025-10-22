use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::diff::{Diff, HazardEvent};
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
    pub water: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub soil: BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub hazards: Vec<HazardEvent>,
}

impl FrameDiff {
    fn is_empty(&self) -> bool {
        self.biome.is_empty()
            && self.water.is_empty()
            && self.soil.is_empty()
            && self.hazards.is_empty()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Frame {
    pub t: u64,
    #[serde(skip_serializing_if = "FrameDiff::is_empty", default)]
    pub diff: FrameDiff,
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
) -> Frame {
    let mut frame_diff = FrameDiff::default();
    for change in diff.biome {
        frame_diff
            .biome
            .insert(World::region_key(change.region as usize), change.biome);
    }
    for delta in diff.water {
        frame_diff
            .water
            .insert(World::region_key(delta.region as usize), delta.delta);
    }
    for delta in diff.soil {
        frame_diff
            .soil
            .insert(World::region_key(delta.region as usize), delta.delta);
    }
    frame_diff.hazards = diff.hazards;

    Frame {
        t,
        diff: frame_diff,
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
