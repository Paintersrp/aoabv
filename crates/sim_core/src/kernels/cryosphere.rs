use anyhow::Result;

use crate::cause::{Code, Entry};
use crate::diff::Diff;
use crate::fixed::{ALBEDO_MAX, FRESHWATER_FLUX_MAX};
use crate::rng::Stream;
use crate::schedule::KernelRun;
use crate::world::World;

pub const STAGE: &str = "kernel:cryosphere";
pub const CHRONICLE_LINE: &str = "Active layer deepened; surface darkened slightly.";
pub const SNOWMELT_CHRONICLE_LINE: &str = "Warm spell released highland snow into streams.";

const ALBEDO_FLOOR: i32 = 100;
const ALBEDO_MAX_I32: i32 = ALBEDO_MAX as i32;
const FRESHWATER_FLUX_MAX_I32: i32 = FRESHWATER_FLUX_MAX as i32;
const ICE_ACCUM_PER_MM: f64 = 6.5;
const ICE_MASS_SATURATION_KT: f64 = 60_000.0;
const ICE_MASS_MAX_KT: f64 = 200_000.0;
const SNOWPACK_CAPTURE_RATIO: f32 = 0.6; // TODO(agents): rationale
const COLD_DEGREE_DAY_ACCUM_MM: f32 = 1.4; // TODO(agents): rationale
const WARM_DEGREE_DAY_MELT_MM: f32 = 4.8; // TODO(agents): rationale
const RAIN_ON_SNOW_MELT_MM: f32 = 0.12; // TODO(agents): rationale
const SNOWPACK_MAX_MM: f32 = 4_500.0; // TODO(agents): rationale
const MELT_PULSE_CLAMP_MM: i32 = 1_000;
const PERMAFROST_ACTIVE_TABLE: &[(i16, i32)] = &[
    (-400, 30),
    (-250, 55),
    (-150, 80),
    (-50, 110),
    (50, 160),
    (150, 210),
    (250, 260),
    (i16::MAX, 300),
]; // TODO(agents): rationale

fn active_layer_depth(temp_tenths: i16) -> i32 {
    let mut depth = PERMAFROST_ACTIVE_TABLE
        .iter()
        .find(|(threshold, _)| temp_tenths <= *threshold)
        .map(|(_, depth)| *depth)
        .unwrap_or(0);
    depth = depth.clamp(0, 300);
    depth
}

pub fn update(world: &mut World, rng: &mut Stream) -> Result<KernelRun> {
    let mut diff = Diff::default();
    let mut chronicle = Vec::new();
    let mut ice_updates = 0usize;
    let mut freshwater_regions = 0usize;
    let mut snowmelt_regions = 0usize;
    let mut contributing_regions = 0usize;
    let mut total_melt_mm = 0.0;

    world.climate.ensure_region_capacity(world.regions.len());

    for index in 0..world.regions.len() {
        let region = &world.regions[index];
        debug_assert_eq!(
            region.index(),
            index,
            "region id {} does not match index {}",
            region.id,
            index
        );

        let temp_tenths = i32::from(region.temperature_tenths_c);
        let precip_mm_i32 = i32::from(region.precipitation_mm);
        let existing_albedo = i32::from(region.albedo_milli);
        let existing_flux = i32::from(region.freshwater_flux_tenths_mm);
        let existing_ice_mass = region.ice_mass_kilotons as f64;
        let mut snowpack_mm = world.climate.snowpack_mm[index] as f32;
        let previous_active_layer = world.climate.permafrost_active_cm[index];
        let baseline_offset = world
            .climate
            .temperature_baseline_tenths
            .get(index)
            .copied()
            .unwrap_or(0);
        let seasonal_temp = temp_tenths + i32::from(baseline_offset);
        let seasonal_temp_clamped =
            seasonal_temp.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
        let mut active_layer_cm = active_layer_depth(seasonal_temp_clamped);
        active_layer_cm = active_layer_cm.clamp(0, 300);
        let thaw_delta = active_layer_cm - previous_active_layer;
        world.climate.permafrost_active_cm[index] = active_layer_cm;
        if active_layer_cm != previous_active_layer {
            diff.record_permafrost_active(index, active_layer_cm);
            if thaw_delta > 0 {
                diff.record_cause(Entry::new(
                    format!("region:{}/permafrost", region.id),
                    Code::PermafrostThaw,
                    Some(format!("depth_cm={}", active_layer_cm)),
                ));
            }
        }

        let temp_c = temp_tenths as f32 / 10.0;
        let precip_mm = region.precipitation_mm as f32;
        let cold_degree_days = (-temp_c).max(0.0);
        let warm_degree_days = temp_c.max(0.0);

        let snow_accum = if temp_c <= 0.0 {
            precip_mm * SNOWPACK_CAPTURE_RATIO + cold_degree_days * COLD_DEGREE_DAY_ACCUM_MM
        } else {
            0.0
        };
        snowpack_mm = (snowpack_mm + snow_accum).clamp(0.0, SNOWPACK_MAX_MM);

        let potential_melt = if warm_degree_days > 0.0 {
            warm_degree_days * WARM_DEGREE_DAY_MELT_MM + precip_mm * RAIN_ON_SNOW_MELT_MM
        } else {
            0.0
        };
        let actual_melt = potential_melt
            .max(0.0)
            .min(snowpack_mm)
            .min(MELT_PULSE_CLAMP_MM as f32);
        snowpack_mm = (snowpack_mm - actual_melt).max(0.0);
        world.climate.snowpack_mm[index] = snowpack_mm.round() as i32;
        let melt_pulse_mm = actual_melt.round() as i32;
        let snowmelt_contribution_mm = actual_melt as f64;
        if melt_pulse_mm > 0 {
            diff.record_melt_pulse(index, melt_pulse_mm);
            diff.record_cause(Entry::new(
                format!("region:{}/snowmelt", region.id),
                Code::SnowmeltSurge,
                Some(format!("mm={}", melt_pulse_mm)),
            ));
            snowmelt_regions += 1;
        }

        let cold_degree_days = (-temp_tenths).max(0) as f64 / 10.0;
        let warm_degree_days = temp_tenths.max(0) as f64 / 10.0;

        let snowfall_input = (precip_mm_i32 as f64) * (0.02 + cold_degree_days / 120.0);
        let melt_variability = 6.0 + rng.next_signed_unit() * 1.5;
        let melt_output = warm_degree_days * melt_variability;
        let mass_balance = snowfall_input - melt_output;

        let latitude_weight = (region.latitude_deg.abs() / 90.0).clamp(0.0, 1.0);
        let ice_mass_delta = mass_balance * ICE_ACCUM_PER_MM;
        let mut next_ice_mass = (existing_ice_mass + ice_mass_delta).max(0.0);
        if next_ice_mass > ICE_MASS_MAX_KT {
            next_ice_mass = ICE_MASS_MAX_KT;
        }
        let next_ice_mass_i32 = next_ice_mass.round() as i32;

        if next_ice_mass_i32 != region.ice_mass_kilotons as i32 {
            diff.record_ice_mass(index, next_ice_mass_i32);
            ice_updates += 1;
            let delta_kt = next_ice_mass - existing_ice_mass;
            diff.record_cause(Entry::new(
                format!("region:{}/ice", region.id),
                Code::IceMassVariation,
                Some(format!("delta_kt={:+.1}", delta_kt)),
            ));
        }

        let coverage = if next_ice_mass <= 0.0 {
            0.0
        } else {
            (next_ice_mass / ICE_MASS_SATURATION_KT).min(1.0)
        };
        let albedo_noise = rng.next_signed_unit() * 10.0;
        let mut raw_albedo = (ALBEDO_FLOOR as f64
            + (ALBEDO_MAX_I32 - ALBEDO_FLOOR) as f64 * coverage
            + latitude_weight * 40.0
            + albedo_noise)
            .round() as i32;
        raw_albedo = raw_albedo.clamp(ALBEDO_FLOOR, ALBEDO_MAX_I32);
        let thaw_bias = (thaw_delta / 5).clamp(-20, 20);
        let biased_albedo = (raw_albedo - thaw_bias).clamp(ALBEDO_FLOOR, ALBEDO_MAX_I32);
        let mut next_albedo = existing_albedo + (biased_albedo - existing_albedo).clamp(-20, 20);
        next_albedo = next_albedo.clamp(ALBEDO_FLOOR, ALBEDO_MAX_I32);

        if next_albedo != existing_albedo {
            diff.record_albedo(index, next_albedo);
            diff.record_cause(Entry::new(
                format!("region:{}/albedo", region.id),
                Code::AlbedoFeedback,
                Some(format!("milli={}", next_albedo)),
            ));
        }

        if mass_balance.abs() >= 0.1 {
            diff.record_cause(Entry::new(
                format!("region:{}/glacier", region.id),
                Code::GlacierMassBalance,
                Some(format!("balance_mm={:.1}", mass_balance)),
            ));
        }

        let glacier_melt_mm = (-mass_balance).max(0.0);
        let melt_total_mm = glacier_melt_mm + snowmelt_contribution_mm;
        let freshwater_flux = (melt_total_mm * 10.0).round() as i32;
        let freshwater_clamped = freshwater_flux.clamp(0, FRESHWATER_FLUX_MAX_I32);
        if freshwater_clamped != existing_flux {
            diff.record_freshwater_flux(index, freshwater_clamped);
        }
        if freshwater_clamped > 0 {
            diff.record_cause(Entry::new(
                format!("region:{}/freshwater", region.id),
                Code::FreshwaterPulse,
                Some(format!("tenths_mm={}", freshwater_clamped)),
            ));
            freshwater_regions += 1;
        }

        let mut region_contributed = false;
        if glacier_melt_mm > 0.0 {
            total_melt_mm += glacier_melt_mm;
            region_contributed = true;
        }
        if snowmelt_contribution_mm > 0.0 {
            total_melt_mm += snowmelt_contribution_mm;
            region_contributed = true;
        }
        if region_contributed {
            contributing_regions += 1;
        }
    }

    let sea_level_delta_mm = total_melt_mm.round() as i32;
    if sea_level_delta_mm != 0 {
        world
            .climate
            .add_sea_level_equivalent_mm(sea_level_delta_mm);
        diff.record_cause(Entry::new(
            "world:sea_level",
            Code::SeaLevelContribution,
            Some(format!("mm={}", sea_level_delta_mm)),
        ));
    }

    if ice_updates > 0 || freshwater_regions > 0 || sea_level_delta_mm != 0 {
        chronicle.push(format!(
            "{} ({}, {} freshwater pulses, {} sea-level contributors).",
            CHRONICLE_LINE, ice_updates, freshwater_regions, contributing_regions
        ));
    } else {
        chronicle.push(CHRONICLE_LINE.to_string());
    }

    if snowmelt_regions > 0 {
        chronicle.push(SNOWMELT_CHRONICLE_LINE.to_string());
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

    #[test]
    fn cryosphere_updates_albedo_and_flux() {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 100,
                latitude_deg: 72.0,
                biome: 3,
                water: 6_000,
                soil: 5_500,
                temperature_tenths_c: -120,
                precipitation_mm: 800,
                albedo_milli: 500,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 2_000,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 50,
                latitude_deg: 12.0,
                biome: 2,
                water: 4_000,
                soil: 4_000,
                temperature_tenths_c: 180,
                precipitation_mm: 600,
                albedo_milli: 300,
                freshwater_flux_tenths_mm: 50,
                ice_mass_kilotons: 100,
                hazards: Hazards::default(),
            },
        ];
        let mut world = World::new(9, 2, 1, regions);
        world.climate.snowpack_mm[1] = 900;
        let mut rng = Stream::from(world.seed, STAGE, 1);

        let run = update(&mut world, &mut rng).expect("cryosphere update succeeds");
        let diff = run.diff;

        assert!(
            !diff.albedo.is_empty(),
            "cryosphere should emit albedo updates"
        );
        assert!(
            !diff.freshwater_flux.is_empty(),
            "cryosphere should emit freshwater flux updates"
        );
        assert!(
            !diff.permafrost_active.is_empty(),
            "cryosphere should emit permafrost depth updates"
        );
        assert!(
            diff.ice_mass
                .iter()
                .any(|entry| entry.region == 0 || entry.region == 1),
            "ice mass updates expected"
        );
        assert!(
            diff.causes
                .iter()
                .any(|entry| entry.code == Code::AlbedoFeedback),
            "albedo cause expected"
        );
        assert!(
            diff.causes
                .iter()
                .any(|entry| entry.code == Code::GlacierMassBalance),
            "mass balance cause expected"
        );
        assert!(
            diff.causes
                .iter()
                .any(|entry| entry.code == Code::FreshwaterPulse),
            "freshwater cause expected"
        );
        assert!(
            diff.causes
                .iter()
                .any(|entry| entry.code == Code::IceMassVariation),
            "ice variation cause expected"
        );
        assert!(
            diff.causes
                .iter()
                .any(|entry| entry.code == Code::PermafrostThaw),
            "permafrost thaw cause expected"
        );
        assert!(
            diff.melt_pulse
                .iter()
                .any(|entry| entry.region == 1 && entry.value > 0),
            "melt pulse should be recorded for warm region"
        );
        assert!(
            diff.causes
                .iter()
                .any(|entry| entry.code == Code::SnowmeltSurge),
            "snowmelt surge cause expected"
        );
        assert!(world.climate.sea_level_equivalent_mm() >= 0);
        assert!(!run.chronicle.is_empty());
        assert!(
            run.chronicle
                .iter()
                .any(|line| line == SNOWMELT_CHRONICLE_LINE),
            "snowmelt chronicle line should be included"
        );
        assert!(
            world.climate.snowpack_mm[1] < 900,
            "snowpack cache should decrease after melt"
        );
        for value in diff.permafrost_active {
            assert!(
                (0..=300).contains(&value.value),
                "permafrost depth {} out of range",
                value.value
            );
        }
    }

    #[test]
    fn cryosphere_reproducible_and_clamped() {
        let regions = vec![Region {
            id: 0,
            x: 0,
            y: 0,
            elevation_m: 0,
            latitude_deg: 80.0,
            biome: 0,
            water: 6_000,
            soil: 6_000,
            temperature_tenths_c: -150,
            precipitation_mm: 700,
            albedo_milli: 600,
            freshwater_flux_tenths_mm: 0,
            ice_mass_kilotons: 10_000,
            hazards: Hazards::default(),
        }];
        let mut world = World::new(42, 1, 1, regions);
        world.climate.snowpack_mm[0] = 1_200;
        let mut rng_a = Stream::from(world.seed, STAGE, 3);
        let mut rng_b = Stream::from(world.seed, STAGE, 3);

        let run_a = update(&mut world.clone(), &mut rng_a).expect("first run succeeds");
        let run_b = update(&mut world, &mut rng_b).expect("second run succeeds");

        let diff_a = run_a.diff;
        let diff_b = run_b.diff;

        let Diff {
            albedo: albedo_a,
            ice_mass: ice_mass_a,
            melt_pulse: melt_pulse_a,
            permafrost_active: permafrost_a,
            ..
        } = diff_a;
        let Diff {
            albedo: albedo_b,
            ice_mass: ice_mass_b,
            melt_pulse: melt_pulse_b,
            permafrost_active: permafrost_b,
            ..
        } = diff_b;

        assert_eq!(albedo_a, albedo_b, "albedo deterministic");
        assert_eq!(ice_mass_a, ice_mass_b, "ice mass deterministic");
        assert_eq!(melt_pulse_a, melt_pulse_b, "melt pulse deterministic");
        assert_eq!(permafrost_a, permafrost_b, "permafrost deterministic");
        assert_eq!(run_a.chronicle, run_b.chronicle);

        for scalar in &albedo_a {
            assert!(
                (ALBEDO_FLOOR..=ALBEDO_MAX_I32).contains(&scalar.value),
                "albedo {} out of range",
                scalar.value
            );
        }
        for scalar in &ice_mass_a {
            assert!(scalar.value >= 0, "ice mass must remain non-negative");
        }
        for scalar in &melt_pulse_a {
            assert!(
                (0..=MELT_PULSE_CLAMP_MM).contains(&scalar.value),
                "melt pulse {} out of bounds",
                scalar.value
            );
        }
        for scalar in &permafrost_a {
            assert!(
                (0..=300).contains(&scalar.value),
                "permafrost depth {} out of range",
                scalar.value
            );
        }
    }

    #[test]
    fn melt_updates_sea_level_accumulator() {
        let regions = vec![Region {
            id: 0,
            x: 0,
            y: 0,
            elevation_m: 0,
            latitude_deg: 75.0,
            biome: 0,
            water: 5_000,
            soil: 5_000,
            temperature_tenths_c: 120,
            precipitation_mm: 100,
            albedo_milli: 500,
            freshwater_flux_tenths_mm: 0,
            ice_mass_kilotons: 5_000,
            hazards: Hazards::default(),
        }];

        let mut world = World::new(5, 1, 1, regions);
        world.climate.snowpack_mm[0] = 800;
        let mut rng = Stream::from(world.seed, STAGE, 2);
        let run = update(&mut world, &mut rng).expect("cryosphere update succeeds");

        assert!(
            world.climate.sea_level_equivalent_mm() > 0,
            "sea level accumulator should record melt contributions"
        );
        assert!(
            run.diff
                .causes
                .iter()
                .any(|entry| entry.code == Code::SeaLevelContribution),
            "sea level cause should be emitted"
        );
        assert!(
            run.diff
                .causes
                .iter()
                .any(|entry| entry.code == Code::IceMassVariation),
            "ice mass variation cause should be emitted"
        );
        assert!(
            !run.diff.ice_mass.is_empty(),
            "ice mass diff should be recorded"
        );
    }

    #[test]
    fn snowpack_does_not_melt_when_persistently_cold() {
        let regions = vec![Region {
            id: 0,
            x: 0,
            y: 0,
            elevation_m: 0,
            latitude_deg: 68.0,
            biome: 0,
            water: 5_000,
            soil: 5_000,
            temperature_tenths_c: -220,
            precipitation_mm: 400,
            albedo_milli: 480,
            freshwater_flux_tenths_mm: 0,
            ice_mass_kilotons: 3_000,
            hazards: Hazards::default(),
        }];

        let mut world = World::new(7, 1, 1, regions);
        let mut rng = Stream::from(world.seed, STAGE, 4);

        for _ in 0..3 {
            let run = update(&mut world, &mut rng).expect("cryosphere update succeeds");
            assert!(run.diff.melt_pulse.is_empty(), "no melt pulses expected");
        }

        assert!(
            world.climate.snowpack_mm[0] > 0,
            "snowpack cache should accumulate under persistent cold"
        );
    }

    #[test]
    fn active_layer_lookup_is_deterministic() {
        let temps = [-360, -240, -120, -10, 80, 180, 320];
        let first: Vec<i32> = temps
            .iter()
            .map(|&t| active_layer_depth(t as i16))
            .collect();
        let second: Vec<i32> = temps
            .iter()
            .map(|&t| active_layer_depth(t as i16))
            .collect();
        assert_eq!(first, second, "lookup should be deterministic");
        for depth in first {
            assert!((0..=300).contains(&depth), "depth {} out of range", depth);
        }
    }

    #[test]
    fn albedo_change_is_capped_per_tick() {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 50,
                latitude_deg: 68.0,
                biome: 1,
                water: 5_800,
                soil: 5_400,
                temperature_tenths_c: -90,
                precipitation_mm: 500,
                albedo_milli: 520,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 3_200,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 10,
                latitude_deg: 40.0,
                biome: 2,
                water: 4_600,
                soil: 4_200,
                temperature_tenths_c: 110,
                precipitation_mm: 650,
                albedo_milli: 360,
                freshwater_flux_tenths_mm: 20,
                ice_mass_kilotons: 900,
                hazards: Hazards::default(),
            },
        ];
        let mut world = World::new(11, 2, 1, regions);
        let baseline_albedo: Vec<i32> = world
            .regions
            .iter()
            .map(|region| region.albedo_milli as i32)
            .collect();
        let mut rng = Stream::from(world.seed, STAGE, 5);
        let run = update(&mut world, &mut rng).expect("cryosphere update succeeds");

        for scalar in run.diff.albedo {
            let previous = baseline_albedo[scalar.region as usize];
            let delta = scalar.value - previous;
            assert!(delta.abs() <= 20, "albedo delta {} exceeds clamp", delta);
            assert!(
                (ALBEDO_FLOOR..=ALBEDO_MAX_I32).contains(&scalar.value),
                "albedo {} outside range",
                scalar.value
            );
        }
    }
}
