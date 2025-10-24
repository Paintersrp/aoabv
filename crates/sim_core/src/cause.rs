use serde::{Deserialize, Serialize};

/// Canonical cause code for simulation diagnostics.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    LatitudeBelt,
    OrographicLift,
    SeasonalityVariance,
    HadleyCell,
    HadleyDrift,
    MonsoonOnset,
    RainShadow,
    HumidityTransport,
    EnergyBalanceAdjustment,
    OrogenyBelt,
    VolcanicAerosolPulse,
    SubsidenceDeltas,
    CmeEvent,
    InsolationGradient,
    ObliquityShift,
    PrecessionPhase,
    SolarCyclePeak,
    TideNeap,
    TideSpring,
    SoilFertilityLow,
    DroughtFlag,
    FloodFlag,
    AlbedoFeedback,
    GlacierMassBalance,
    FreshwaterPulse,
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
            Code::HadleyCell => "hadley_cell",
            Code::HadleyDrift => "hadley_drift",
            Code::MonsoonOnset => "monsoon_onset",
            Code::RainShadow => "rain_shadow",
            Code::HumidityTransport => "humidity_transport",
            Code::EnergyBalanceAdjustment => "energy_balance_adjustment",
            Code::OrogenyBelt => "orogeny_belt",
            Code::VolcanicAerosolPulse => "volcanic_aerosol_pulse",
            Code::SubsidenceDeltas => "subsidence_deltas",
            Code::CmeEvent => "cme_event",
            Code::InsolationGradient => "insolation_gradient",
            Code::ObliquityShift => "obliquity_shift",
            Code::PrecessionPhase => "precession_phase",
            Code::SolarCyclePeak => "solar_cycle_peak",
            Code::TideNeap => "tide_neap",
            Code::TideSpring => "tide_spring",
            Code::SoilFertilityLow => "soil_fertility_low",
            Code::DroughtFlag => "drought_flag",
            Code::FloodFlag => "flood_flag",
            Code::AlbedoFeedback => "albedo_feedback",
            Code::GlacierMassBalance => "glacier_mass_balance",
            Code::FreshwaterPulse => "freshwater_pulse",
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
