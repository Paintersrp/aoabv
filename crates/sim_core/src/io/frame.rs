use serde::Serialize;

use crate::diff::{Diff, Highlight};

#[derive(Clone, Debug, Serialize)]
pub struct Frame {
    pub t: u64,
    pub diff: Diff,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub highlights: Vec<Highlight>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub chronicle: Vec<String>,
    pub era_end: bool,
}

impl Frame {
    pub fn to_ndjson(&self) -> serde_json::Result<String> {
        let mut json = serde_json::to_string(self)?;
        json.push('\n');
        Ok(json)
    }
}
