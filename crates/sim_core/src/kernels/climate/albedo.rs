use crate::cause::{Code, Entry};
use crate::diff::{DiagEnergy, Diff};
use crate::world::World;
use anyhow::Result;

const BASELINE_LIMIT_TENTHS: i32 = 120;

pub(super) fn reconcile(world: &mut World) -> Result<Diff> {
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
