use crate::cause::{Code, Entry};
use crate::diff::Diff;
use crate::rng::Stream;
use crate::schedule::KernelRun;
use crate::world::World;
use anyhow::{ensure, Result};

mod classification;
mod diagnostics;

pub const STAGE: &str = "kernel:climate";
pub const CORE_STAGE: &str = "kernel:climate/core";
pub fn update(world: &World, rng: &mut Stream) -> Result<KernelRun> {
    let mut diff = Diff::default();
    let mut chronicle = Vec::new();

    for (index, region) in world.regions.iter().enumerate() {
        ensure!(
            region.index() == index,
            "region id {} does not match index {}",
            region.id,
            index
        );

        let belt = classification::LatitudeBelt::from_latitude(region.latitude_deg);
        let mut region_rng = rng.derive(region.index() as u64);
        let seasonal_shift = region_rng.next_signed_unit();
        let dryness = classification::dryness_score(region, seasonal_shift);
        let biome = classification::classify_biome(&belt, dryness);
        let orographic_lift = diagnostics::orographic_lift_indicator(world, region);

        if biome != region.biome {
            diff.record_biome(region.index(), biome);
            chronicle.push(format!(
                "Region {} shifted toward a {} biome.",
                region.id,
                classification::biome_label(biome)
            ));
        }

        diff.record_cause(Entry::new(
            format!("region:{}/biome", region.id),
            Code::LatitudeBelt,
            Some(format!("{}", belt.label())),
        ));
        diff.record_cause(Entry::new(
            format!("region:{}/biome", region.id),
            Code::SeasonalShift,
            Some(format!("{:.3}", seasonal_shift)),
        ));
        diff.record_cause(Entry::new(
            format!("region:{}/biome", region.id),
            Code::OrographicLift,
            Some(format!("lift_km={:.3}", orographic_lift)),
        ));
    }

    Ok(KernelRun {
        diff,
        chronicle,
        highlights: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Stream;
    use crate::world::{Hazards, Region, World};

    use super::classification::LatitudeBelt;

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
        let run = update(&world, &mut rng).expect("climate update should succeed");
        let diff = run.diff;

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
        let diff = update(&world, &mut rng).unwrap().diff;
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

        let run_a = update(&world, &mut rng_a).expect("first run succeeds");
        let run_b = update(&world, &mut rng_b).expect("second run succeeds");

        assert_eq!(run_a.diff.causes, run_b.diff.causes);
        assert_eq!(run_a.chronicle, run_b.chronicle);
    }
}
