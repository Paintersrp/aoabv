use crate::cause::Cause;
use crate::diff::{Diff, Highlight};
use crate::fixed::resource_ratio;
use crate::rng::StageRng;
use crate::world::{Region, World};

pub struct ClimateOutput {
    pub diff: Diff,
    pub highlights: Vec<Highlight>,
    pub chronicle: Vec<String>,
}

enum LatitudeBelt {
    Equatorial,
    Subtropical,
    Temperate,
    Subpolar,
    Polar,
}

impl LatitudeBelt {
    fn from_latitude(latitude: f64) -> Self {
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

    fn label(&self) -> &'static str {
        match self {
            Self::Equatorial => "equatorial",
            Self::Subtropical => "subtropical",
            Self::Temperate => "temperate",
            Self::Subpolar => "subpolar",
            Self::Polar => "polar",
        }
    }
}

fn classify_biome(belt: &LatitudeBelt, dryness: f64) -> u8 {
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

fn dryness_score(region: &Region, seasonal_shift: f64) -> f64 {
    let moisture = resource_ratio(region.water);
    let elevation = (f64::from(region.elevation_m) / 3_000.0).clamp(0.0, 1.0);
    let baseline = 1.0 - moisture;
    (baseline * 0.6 + elevation * 0.3 + seasonal_shift * 0.1).clamp(0.0, 1.0)
}

pub fn run(world: &World, rng: &mut StageRng) -> ClimateOutput {
    let mut diff = Diff::default();
    let highlights = Vec::new();
    let mut chronicle = Vec::new();

    for region in &world.regions {
        let belt = LatitudeBelt::from_latitude(region.latitude_deg);
        let mut region_rng = rng.fork_region(region.index());
        let seasonal_shift = region_rng.next_signed_unit();
        let dryness = dryness_score(region, seasonal_shift);
        let biome = classify_biome(&belt, dryness);
        if biome != region.biome {
            diff.record_biome(region.index(), biome);
            chronicle.push(format!("Region {} shifted biome to {}", region.id, biome));
        }
        diff.record_cause(Cause::new(
            format!("region:{}/biome", region.id),
            "latitude_belt",
            Some(format!("{}", belt.label())),
        ));
        diff.record_cause(Cause::new(
            format!("region:{}/biome", region.id),
            "seasonality_variance",
            Some(format!("{:.3}", seasonal_shift)),
        ));
    }

    ClimateOutput {
        diff,
        highlights,
        chronicle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::ProjectRng;
    use crate::world::{Hazards, Region, World};

    #[test]
    fn biome_classification_varies_by_latitude() {
        let regions: Vec<Region> = (0..5)
            .map(|i| Region {
                id: i,
                x: i,
                y: 0,
                elevation_m: 100,
                latitude_deg: -60.0 + f64::from(i) * 30.0,
                biome: 0,
                water: 5_000,
                soil: 5_000,
                hazards: Hazards::default(),
            })
            .collect();
        let world = World::new(11, 5, 1, regions);
        let mut rng = ProjectRng::new(world.seed).stage(crate::rng::Stage::Climate, 1);
        let output = run(&world, &mut rng);
        assert!(output.diff.biome.len() >= 3);
    }
}
