use anyhow::Result;

use crate::cause::{Code, Entry};
use crate::diff::Diff;
use crate::fixed::{ALBEDO_MAX, FRESHWATER_FLUX_MAX};
use crate::rng::Stream;
use crate::world::World;

pub const STAGE: &str = "kernel:cryosphere";
pub const CHRONICLE_LINE: &str = "Polar ice advanced; albedo brightened the poles.";

const ALBEDO_MIN: i32 = 0;
const ALBEDO_MAX_I32: i32 = ALBEDO_MAX as i32;
const FRESHWATER_FLUX_MAX_I32: i32 = FRESHWATER_FLUX_MAX as i32;

pub fn update(world: &World, rng: &mut Stream) -> Result<Diff> {
    let mut diff = Diff::default();

    for (index, region) in world.regions.iter().enumerate() {
        debug_assert_eq!(
            region.index(),
            index,
            "region id {} does not match index {}",
            region.id,
            index
        );

        let temp_tenths = i32::from(region.temperature_tenths_c);
        let precip_mm = i32::from(region.precipitation_mm);
        let existing_albedo = i32::from(region.albedo_milli);
        let existing_flux = i32::from(region.freshwater_flux_tenths_mm);

        let cold_degree_days = (-temp_tenths).max(0) as f64 / 10.0;
        let warm_degree_days = temp_tenths.max(0) as f64 / 10.0;

        let snowfall_input = (precip_mm as f64) * (0.02 + cold_degree_days / 120.0);
        let melt_variability = 6.0 + rng.next_signed_unit() * 1.5;
        let melt_output = warm_degree_days * melt_variability;
        let mass_balance = snowfall_input - melt_output;

        let latitude_weight = (region.latitude_deg.abs() / 90.0).clamp(0.0, 1.0);
        let baseline_albedo = 250.0 + 600.0 * latitude_weight;
        let albedo_adjustment = mass_balance * (0.4 + 0.3 * latitude_weight);
        let albedo_noise = rng.next_signed_unit() * 15.0;

        let mut next_albedo = (baseline_albedo + albedo_adjustment + albedo_noise).round() as i32;
        next_albedo = next_albedo.clamp(ALBEDO_MIN, ALBEDO_MAX_I32);

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

        let freshwater_flux = (mass_balance.max(0.0) * 10.0).round() as i32;
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
        }
    }

    Ok(diff)
}

#[cfg(test)]
mod tests {
    use super::*;
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
                hazards: Hazards::default(),
            },
        ];
        let world = World::new(9, 2, 1, regions);
        let mut rng = Stream::from(world.seed, STAGE, 1);

        let diff = update(&world, &mut rng).expect("cryosphere update succeeds");

        assert!(
            !diff.albedo.is_empty(),
            "cryosphere should emit albedo updates"
        );
        assert!(
            !diff.freshwater_flux.is_empty(),
            "cryosphere should emit freshwater flux updates"
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
    }
}
