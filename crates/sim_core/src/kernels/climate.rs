use crate::cause::{Code, Entry};
use crate::diff::{DiagEnergy, Diff};
use crate::fixed::{resource_ratio, WATER_MAX};
use crate::rng::Stream;
use crate::world::{Region, World};
use anyhow::{ensure, Result};

pub const STAGE: &str = "kernel:climate";
pub const ALBEDO_RECONCILE_STAGE: &str = "kernel:climate/albedo_reconcile";
const BASELINE_LIMIT_TENTHS: i32 = 120;

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

pub fn albedo_reconcile(world: &mut World) -> Result<Diff> {
    let mut diff = Diff::default();
    let region_count = world.regions.len();
    if region_count == 0 {
        return Ok(diff);
    }

    world.climate.ensure_region_capacity(region_count);

    let mut total_anomaly = 0i64;
    let mut total_adjust = 0i64;
    let mut adjusted_regions = 0usize;

    for (index, region) in world.regions.iter().enumerate() {
        let current_albedo = i32::from(region.albedo_milli);
        let slot = world
            .climate
            .last_albedo_milli
            .get_mut(index)
            .expect("climate state sized to regions");
        let previous_albedo = if *slot == 0 { current_albedo } else { *slot };
        *slot = current_albedo;
        let anomaly = current_albedo - previous_albedo;
        if anomaly == 0 {
            continue;
        }

        adjusted_regions += 1;
        total_anomaly += i64::from(anomaly);

        let raw_adjust = (-anomaly as f64 / 15.0).round() as i32;
        let bounded_adjust = raw_adjust.clamp(-BASELINE_LIMIT_TENTHS, BASELINE_LIMIT_TENTHS);
        let slot = world
            .climate
            .temperature_baseline_tenths
            .get_mut(index)
            .expect("climate state sized");
        let previous = i32::from(*slot);
        let updated =
            (previous + bounded_adjust).clamp(-BASELINE_LIMIT_TENTHS, BASELINE_LIMIT_TENTHS);
        *slot = updated as i16;
        total_adjust += i64::from(updated - previous);
    }

    if adjusted_regions > 0 {
        let mean_anomaly = (total_anomaly as f64 / adjusted_regions as f64).round() as i32;
        let mean_adjust = (total_adjust as f64 / adjusted_regions as f64).round() as i32;
        diff.record_diag_energy(DiagEnergy {
            albedo_anomaly_milli: mean_anomaly,
            temp_adjust_tenths: mean_adjust,
        });
        diff.record_cause(Entry::new(
            "climate:albedo_reconcile",
            Code::AlbedoFeedback,
            Some(format!("mean_milli={}", mean_anomaly)),
        ));
        diff.record_cause(Entry::new(
            "climate:albedo_reconcile",
            Code::EnergyBalanceAdjustment,
            Some(format!("mean_tenths={}", mean_adjust)),
        ));
    }

    Ok(diff)
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
            Some(format!("lift_km={:.3}", orographic_lift)),
        ));
    }

    Ok(diff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernels::atmosphere;
    use crate::rng::Stream;
    use crate::world::{Hazards, Region, World};

    #[test]
    fn biome_classification_varies_by_latitude() {
        struct BeltCase {
            latitude: f64,
            /// Allowed biome tiers for this latitude band (wettest to driest).
            allowed_biomes: &'static [u8],
            label: &'static str,
        }

        // Documented expectations for maintainability. Keep in sync with
        // `classify_biome` whenever biome tiers change.
        //
        // Latitude → biome ladder (wet → dry):
        // * equatorial (<15°): rainforest (5) → steppe (3) → desert (4)
        // * subtropical (<30°): rainforest (5) → savannah/temperate mix (2) → desert (4)
        // * temperate (<45°): temperate forest (2) → boreal/grassland mix (1) → steppe (3)
        // * subpolar (<60°): boreal mix (1) → polar tundra (0)
        // * polar (≥60°): polar tundra/ice (0)
        let belt_cases = [
            BeltCase {
                latitude: 0.0,
                allowed_biomes: &[5, 3, 4],
                label: "equatorial",
            },
            BeltCase {
                latitude: 20.0,
                allowed_biomes: &[5, 2, 4],
                label: "subtropical",
            },
            BeltCase {
                latitude: 35.0,
                allowed_biomes: &[2, 1, 3],
                label: "temperate",
            },
            BeltCase {
                latitude: 50.0,
                allowed_biomes: &[1, 0],
                label: "subpolar",
            },
            BeltCase {
                latitude: 70.0,
                allowed_biomes: &[0],
                label: "polar",
            },
        ];

        const TEST_SEED: u64 = 0xA5A5_F0F0_A5A5_F0F0;
        const TEST_TICK: u64 = 7;

        let regions: Vec<Region> = belt_cases
            .iter()
            .enumerate()
            .map(|(i, case)| Region {
                id: i as u32,
                x: i as u32,
                y: 0,
                elevation_m: 100,
                latitude_deg: case.latitude,
                biome: u8::MAX, // ensure every case records a biome diff
                water: 5_000,
                soil: 5_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 400,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            })
            .collect();

        let world = World::new(TEST_SEED, belt_cases.len() as u32, 1, regions);
        // Fixed RNG seed + tick ensure deterministic seasonal shifts across runs.
        let mut rng = Stream::from(TEST_SEED, STAGE, TEST_TICK);
        let diff = update(&world, &mut rng).expect("climate update should succeed");

        assert_eq!(diff.biome.len(), belt_cases.len());

        for (index, case) in belt_cases.iter().enumerate() {
            let belt = LatitudeBelt::from_latitude(case.latitude);
            let change = diff
                .biome
                .iter()
                .find(|entry| entry.region as usize == index)
                .expect("expected biome change for test region");
            let biome = change.biome as u8;

            assert!(
                case.allowed_biomes.contains(&biome),
                "{} belt produced biome {}, expected one of {:?}",
                case.label,
                biome,
                case.allowed_biomes
            );

            if matches!(belt, LatitudeBelt::Equatorial) {
                assert_ne!(
                    biome, 0,
                    "equatorial regions should never resolve to the polar biome"
                );
            }

            if matches!(belt, LatitudeBelt::Polar) {
                assert_eq!(
                    biome, 0,
                    "polar regions should be able to resolve to the polar biome"
                );
            }
        }
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
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 400,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
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
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 400,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
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
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 400,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
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
            .and_then(|note| note.strip_prefix("lift_km="))
            .and_then(|value| value.parse::<f64>().ok())
            .expect("lift note to include lift_km= prefix with numeric value");
        assert!(lift_note > 0.0, "expected positive lift, got {}", lift_note);
    }

    #[test]
    fn orographic_lift_cause_is_deterministic() {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 200,
                latitude_deg: 10.0,
                biome: 1,
                water: 4_800,
                soil: 5_200,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 380,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 1_800,
                latitude_deg: 12.0,
                biome: 1,
                water: 4_900,
                soil: 5_100,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 380,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 2,
                x: 0,
                y: 1,
                elevation_m: 300,
                latitude_deg: 8.0,
                biome: 1,
                water: 5_000,
                soil: 5_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 380,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 3,
                x: 1,
                y: 1,
                elevation_m: 350,
                latitude_deg: 9.5,
                biome: 1,
                water: 4_950,
                soil: 5_050,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 380,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
        ];

        let world = World::new(99, 2, 2, regions);
        let mut rng_a = Stream::from(world.seed, STAGE, 4);
        let mut rng_b = Stream::from(world.seed, STAGE, 4);

        let diff_a = update(&world, &mut rng_a).expect("first run succeeds");
        let diff_b = update(&world, &mut rng_b).expect("second run succeeds");

        assert_eq!(diff_a.causes, diff_b.causes);
    }

    #[test]
    fn albedo_reconcile_emits_diag_and_defers_temperature() {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 100,
                latitude_deg: 70.0,
                biome: 0,
                water: 5_500,
                soil: 5_000,
                temperature_tenths_c: -120,
                precipitation_mm: 400,
                albedo_milli: 720,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 12_000,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 50,
                latitude_deg: 10.0,
                biome: 0,
                water: 6_000,
                soil: 5_500,
                temperature_tenths_c: 40,
                precipitation_mm: 500,
                albedo_milli: 360,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 500,
                hazards: Hazards::default(),
            },
        ];
        let mut world_with = World::new(777, 2, 1, regions.clone());
        world_with.climate.last_albedo_milli[0] =
            i32::from(world_with.regions[0].albedo_milli) - 150;
        let initial_temperature = world_with.regions[0].temperature_tenths_c;

        let diff = albedo_reconcile(&mut world_with).expect("reconcile succeeds");
        let diag = diff.diag_energy.clone().expect("diag energy recorded");
        assert!(diff
            .causes
            .iter()
            .any(|entry| entry.code == Code::AlbedoFeedback));
        assert!(diff
            .causes
            .iter()
            .any(|entry| entry.code == Code::EnergyBalanceAdjustment));
        assert_ne!(
            diag.temp_adjust_tenths, 0,
            "temperature adjustment captured"
        );
        assert_ne!(diag.albedo_anomaly_milli, 0, "albedo anomaly captured");
        assert!(
            diff.temperature.is_empty(),
            "no same-tick temperature diffs"
        );
        assert_eq!(
            world_with.regions[0].temperature_tenths_c, initial_temperature,
            "world temperatures remain unchanged"
        );
        let baseline_shift = world_with.climate.temperature_baseline_tenths[0];
        assert_ne!(baseline_shift, 0, "baseline shift applied for next tick");
        assert_eq!(
            diag.temp_adjust_tenths,
            i32::from(baseline_shift),
            "diag reports applied baseline shift"
        );

        let world_control = World::new(777, 2, 1, regions);

        let mut rng_with = Stream::from(world_with.seed, atmosphere::STAGE, 2);
        let mut rng_without = Stream::from(world_control.seed, atmosphere::STAGE, 2);
        let diff_with =
            atmosphere::update(&world_with, &mut rng_with).expect("atmosphere with baseline");
        let diff_without = atmosphere::update(&world_control, &mut rng_without)
            .expect("atmosphere without baseline");

        let temp_with = diff_with
            .temperature
            .iter()
            .find(|value| value.region == 0)
            .map(|value| value.value)
            .expect("temperature diff for region 0");
        let temp_without = diff_without
            .temperature
            .iter()
            .find(|value| value.region == 0)
            .map(|value| value.value)
            .expect("temperature diff for region 0 control");

        assert_eq!(
            temp_with - temp_without,
            i32::from(baseline_shift),
            "baseline shift applies on subsequent tick"
        );
    }
}
