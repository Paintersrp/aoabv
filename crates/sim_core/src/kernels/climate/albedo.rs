use anyhow::Result;

use crate::cause::{Code, Entry};
use crate::diff::{DiagEnergy, Diff};
use crate::world::World;

const BASELINE_LIMIT_TENTHS: i32 = 120;

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

#[cfg(test)]
mod tests {
    use super::albedo_reconcile;
    use crate::cause::Code;
    use crate::kernels::atmosphere;
    use crate::rng::Stream;
    use crate::world::{Hazards, Region, World};

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
        let diff_with = atmosphere::update(&world_with, &mut rng_with)
            .expect("atmosphere with baseline")
            .diff;
        let diff_without = atmosphere::update(&world_control, &mut rng_without)
            .expect("atmosphere without baseline")
            .diff;

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
