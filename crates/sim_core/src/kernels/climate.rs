use crate::cause::{Code, Entry};
use crate::diff::Diff;
use crate::fixed::{resource_ratio, WATER_MAX};
use crate::rng::Stream;
use crate::world::{Region, World};
use anyhow::{ensure, Result};

pub const STAGE: &str = "kernel:climate";

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
    let moisture = resource_ratio(region.water, WATER_MAX);
    let elevation = (f64::from(region.elevation_m) / 3_000.0).clamp(0.0, 1.0);
    let baseline = 1.0 - moisture;
    (baseline * 0.6 + elevation * 0.3 + seasonal_shift * 0.1).clamp(0.0, 1.0)
}

fn orographic_lift_indicator(world: &World, region: &Region) -> f64 {
    let width = world.width as i32;
    let height = world.height as i32;
    let x = region.x as i32;
    let y = region.y as i32;
    let mut sum = 0_i64;
    let mut count = 0_i32;
    const OFFSETS: &[(i32, i32)] = &[(-1, 0), (1, 0), (0, -1), (0, 1)];
    for (dx, dy) in OFFSETS {
        let nx = x + dx;
        let ny = y + dy;
        if nx < 0 || nx >= width || ny < 0 || ny >= height {
            continue;
        }
        let neighbor_index = (ny * width + nx) as usize;
        if let Some(neighbor) = world.regions.get(neighbor_index) {
            sum += i64::from(neighbor.elevation_m);
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    let neighbor_mean = sum as f64 / f64::from(count);
    ((f64::from(region.elevation_m) - neighbor_mean) / 1_000.0).max(0.0)
}
pub fn update(world: &World, rng: &mut Stream) -> Result<Diff> {
    let mut diff = Diff::default();

    for (index, region) in world.regions.iter().enumerate() {
        ensure!(
            region.index() == index,
            "region id {} does not match index {}",
            region.id,
            index
        );
        let belt = LatitudeBelt::from_latitude(region.latitude_deg);
        let mut region_rng = rng.derive(region.index() as u64);
        let seasonal_shift = region_rng.next_signed_unit();
        let dryness = dryness_score(region, seasonal_shift);
        let biome = classify_biome(&belt, dryness);
        let orographic_lift = orographic_lift_indicator(world, region);
        if biome != region.biome {
            diff.record_biome(region.index(), biome);
        }
        diff.record_cause(Entry::new(
            format!("region:{}/biome", region.id),
            Code::LatitudeBelt,
            Some(format!("{}", belt.label())),
        ));
        diff.record_cause(Entry::new(
            format!("region:{}/biome", region.id),
            Code::SeasonalityVariance,
            Some(format!("{:.3}", seasonal_shift)),
        ));
        diff.record_cause(Entry::new(
            format!("region:{}/biome", region.id),
            Code::OrographicLift,
            Some(format!("{:.3}", orographic_lift)),
        ));
    }

    Ok(diff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Stream;
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
        let mut rng = Stream::from(world.seed, STAGE, 1);
        let diff = update(&world, &mut rng).unwrap();
        assert!(diff.biome.len() >= 3);
    }

    #[test]
    fn orographic_lift_cause_for_elevated_regions() {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 200,
                latitude_deg: 0.0,
                biome: 0,
                water: 5_000,
                soil: 5_000,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 1_800,
                latitude_deg: 0.0,
                biome: 0,
                water: 5_000,
                soil: 5_000,
                hazards: Hazards::default(),
            },
            Region {
                id: 2,
                x: 2,
                y: 0,
                elevation_m: 200,
                latitude_deg: 0.0,
                biome: 0,
                water: 5_000,
                soil: 5_000,
                hazards: Hazards::default(),
            },
        ];
        let world = World::new(17, 3, 1, regions);
        let mut rng = Stream::from(world.seed, STAGE, 1);
        let diff = update(&world, &mut rng).unwrap();
        let target = "region:1/biome";
        let lift_entry = diff
            .causes
            .iter()
            .find(|entry| entry.code == Code::OrographicLift && entry.target == target)
            .expect("orographic lift entry for elevated region");
        let lift_note = lift_entry
            .note
            .as_ref()
            .and_then(|note| note.parse::<f64>().ok())
            .expect("lift note to be numeric");
        assert!(lift_note > 0.0, "expected positive lift, got {}", lift_note);
    }
}
