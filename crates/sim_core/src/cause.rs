use serde::{Deserialize, Serialize};

/// Canonical cause code for simulation diagnostics.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    LatitudeBelt,
    OrographicLift,
    SeasonalityVariance,
    SoilFertilityLow,
    DroughtFlag,
    FloodFlag,
    EraEnd,
    StagnationWarning,
    CollapseWarning,
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Code::LatitudeBelt => "latitude_belt",
            Code::OrographicLift => "orographic_lift",
            Code::SeasonalityVariance => "seasonality_variance",
            Code::SoilFertilityLow => "soil_fertility_low",
            Code::DroughtFlag => "drought_flag",
            Code::FloodFlag => "flood_flag",
            Code::EraEnd => "era_end",
            Code::StagnationWarning => "stagnation_warning",
            Code::CollapseWarning => "collapse_warning",
        };
        f.write_str(label)
    }
}

/// Structured cause entry used for diagnostics and auditing.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    pub target: String,
    pub code: Code,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Entry {
    pub fn new<T: Into<String>>(target: T, code: Code, note: Option<String>) -> Self {
        Self {
            target: target.into(),
            code,
            note,
        }
    }
}
