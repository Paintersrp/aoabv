use crate::fixed::{resource_ratio, WATER_MAX};
use crate::world::Region;

#[derive(Debug, Clone, Copy)]
pub(super) enum LatitudeBelt {
    Equatorial,
    Subtropical,
    Temperate,
    Subpolar,
    Polar,
}

impl LatitudeBelt {
    pub(super) fn from_latitude(latitude: f64) -> Self {
        let lat = latitude.abs();
        if lat < 15.0 {
            Self::Equatorial
        } else if lat < 30.0 {
            Self::Subtropical
        } else if lat < 45.0 {
            Self::Temperate
        } else if lat < 60.0 {
            Self::Subpolar
        } else {
            Self::Polar
        }
    }

    pub(super) fn label(&self) -> &'static str {
        match self {
            Self::Equatorial => "equatorial",
            Self::Subtropical => "subtropical",
            Self::Temperate => "temperate",
            Self::Subpolar => "subpolar",
            Self::Polar => "polar",
        }
    }
}

pub(super) fn classify_biome(belt: &LatitudeBelt, dryness: f64) -> u8 {
    let dryness = dryness.clamp(0.0, 1.0);
    match belt {
        LatitudeBelt::Equatorial => {
            if dryness < 0.35 {
                5 // tropical rainforest
            } else if dryness < 0.65 {
                3 // dry steppe
            } else {
                4 // desert
            }
        }
        LatitudeBelt::Subtropical => {
            if dryness < 0.3 {
                5
            } else if dryness < 0.6 {
                2 // savannah / temperate mix
            } else {
                4
            }
        }
        LatitudeBelt::Temperate => {
            if dryness < 0.25 {
                2 // temperate forest
            } else if dryness < 0.6 {
                1 // boreal/grassland mix
            } else {
                3
            }
        }
        LatitudeBelt::Subpolar => {
            if dryness < 0.4 {
                1
            } else {
                0 // polar tundra
            }
        }
        LatitudeBelt::Polar => 0,
    }
}

pub(super) fn biome_label(biome: u8) -> &'static str {
    match biome {
        5 => "rainforest",
        4 => "desert",
        3 => "steppe",
        2 => "temperate",
        1 => "boreal",
        _ => "polar",
    }
}

pub(super) fn dryness_score(region: &Region, seasonal_shift: f64) -> f64 {
    let moisture = resource_ratio(region.water, WATER_MAX);
    let elevation = (f64::from(region.elevation_m) / 3_000.0).clamp(0.0, 1.0);
    let baseline = 1.0 - moisture;
    (baseline * 0.6 + elevation * 0.3 + seasonal_shift * 0.1).clamp(0.0, 1.0)
}
