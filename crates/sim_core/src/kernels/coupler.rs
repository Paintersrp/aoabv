use std::cell::RefCell;
use std::ptr::NonNull;

use anyhow::{anyhow, Result};

use crate::cause::{Code, Entry};
use crate::diff::Diff;
use crate::world::World;

pub const STAGE: &str = "kernel:climate/coupler";
pub const CHRONICLE_LINE: &str =
    "Cryosphere shifts rebalanced atmospheric energy baselines across the globe.";

const BASELINE_LIMIT_TENTHS: i32 = 120;

thread_local! {
    static CONTEXT: RefCell<Option<NonNull<World>>> = RefCell::new(None);
}

pub fn reconcile_with_world(
    world: &mut World,
    atmos_diff: &Diff,
    cryo_diff: &Diff,
) -> Result<Diff> {
    CONTEXT.with(|ctx| {
        let mut guard = ctx.borrow_mut();
        debug_assert!(guard.is_none(), "coupler context should be empty");
        *guard = NonNull::new(world as *mut World);
    });
    let result = reconcile(atmos_diff, cryo_diff);
    CONTEXT.with(|ctx| {
        let mut guard = ctx.borrow_mut();
        *guard = None;
    });
    result
}

pub fn reconcile(atmos_diff: &Diff, cryo_diff: &Diff) -> Result<Diff> {
    CONTEXT.with(|ctx| -> Result<Diff> {
        let ptr = {
            let guard = ctx.borrow();
            guard
                .as_ref()
                .copied()
                .ok_or_else(|| anyhow!("coupler world context missing"))?
        };
        // SAFETY: `reconcile_with_world` guarantees exclusive access for the
        // duration of this call. The pointer remains valid until we clear the
        // context after `reconcile` returns.
        let world = unsafe { ptr.as_ptr().as_mut().expect("world pointer") };
        Ok(reconcile_inner(world, atmos_diff, cryo_diff))
    })
}

fn reconcile_inner(world: &mut World, _atmos_diff: &Diff, cryo_diff: &Diff) -> Diff {
    let mut diff = Diff::default();
    if cryo_diff.albedo.is_empty() {
        return diff;
    }

    let region_count = world.regions.len();
    if region_count == 0 {
        return diff;
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

        let raw_adjust = (-anomaly as f64 / 120.0).round() as i32;
        let bounded_adjust = raw_adjust.clamp(-1, 1);
        let baseline_slot = world
            .climate
            .temperature_baseline_tenths
            .get_mut(index)
            .expect("baseline state sized");
        let previous = i32::from(*baseline_slot);
        let updated =
            (previous + bounded_adjust).clamp(-BASELINE_LIMIT_TENTHS, BASELINE_LIMIT_TENTHS);
        if updated != previous {
            *baseline_slot = updated as i16;
            diff.record_temperature_baseline(index, updated);
            total_adjust += i64::from(updated - previous);
        }
    }

    if adjusted_regions > 0 && (!diff.temperature_baseline.is_empty() || total_anomaly != 0) {
        let mean_anomaly = (total_anomaly as f64 / adjusted_regions as f64).round() as i32;
        let mean_adjust = (total_adjust as f64 / adjusted_regions as f64).round() as i32;
        diff.record_diagnostic("albedo_anomaly_milli", mean_anomaly);
        diff.record_diagnostic("energy_balance", mean_adjust);
        diff.record_cause(Entry::new(
            "climate:coupler",
            Code::AlbedoFeedback,
            Some(format!("mean_milli={}", mean_anomaly)),
        ));
        diff.record_cause(Entry::new(
            "climate:coupler",
            Code::EnergyBalanceAdjustment,
            Some(format!("mean_tenths={}", mean_adjust)),
        ));
    }

    diff
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::Diff as KernelDiff;
    use crate::kernels::atmosphere;
    use crate::rng::Stream;
    use crate::world::{Hazards, Region};

    fn seed_world() -> World {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 0,
                latitude_deg: 45.0,
                biome: 2,
                water: 5_000,
                soil: 5_000,
                temperature_tenths_c: 20,
                precipitation_mm: 400,
                albedo_milli: 300,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 100,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 10,
                latitude_deg: 65.0,
                biome: 1,
                water: 4_000,
                soil: 4_500,
                temperature_tenths_c: -40,
                precipitation_mm: 600,
                albedo_milli: 500,
                freshwater_flux_tenths_mm: 50,
                ice_mass_kilotons: 2_500,
                hazards: Hazards::default(),
            },
        ];
        World::new(777, 2, 1, regions)
    }

    #[test]
    fn reconcile_tracks_baseline_offsets() {
        let mut world = seed_world();
        let mut cryo_diff = KernelDiff::default();
        cryo_diff.record_albedo(0, 360);
        cryo_diff.record_albedo(1, 620);
        let atmos_diff = KernelDiff::default();

        let mut world_copy = world.clone();
        // Apply cryosphere effect to both worlds before reconciling, matching tick order.
        world.regions[0].albedo_milli = 360;
        world.regions[1].albedo_milli = 620;
        world_copy.regions[0].albedo_milli = 360;
        world_copy.regions[1].albedo_milli = 620;

        let coupler_diff =
            reconcile_with_world(&mut world, &atmos_diff, &cryo_diff).expect("reconcile succeeds");
        assert!(coupler_diff.diagnostics.contains_key("energy_balance"));
        assert!(!coupler_diff.temperature_baseline.is_empty());
        assert!(coupler_diff
            .causes
            .iter()
            .any(|cause| cause.code == Code::EnergyBalanceAdjustment));
        assert!(coupler_diff
            .causes
            .iter()
            .any(|cause| cause.code == Code::AlbedoFeedback));

        for entry in &coupler_diff.temperature_baseline {
            let region_index = entry.region as usize;
            let new_baseline = world
                .climate
                .temperature_baseline_tenths
                .get(region_index)
                .copied()
                .expect("baseline entry exists");
            assert_eq!(new_baseline as i32, entry.value);
        }

        // Atmosphere should reflect baseline adjustments on the next tick within
        // a tenth-degree tolerance.
        let baseline_world = world.clone();
        let control_world = world_copy;
        let mut rng = Stream::from(world.seed, atmosphere::STAGE, 3);
        let mut rng_control = Stream::from(control_world.seed, atmosphere::STAGE, 3);
        let baseline_run = atmosphere::update(&baseline_world, &mut rng)
            .expect("baseline update succeeds")
            .diff;
        let control_run = atmosphere::update(&control_world, &mut rng_control)
            .expect("control update succeeds")
            .diff;

        for region in 0..baseline_world.regions.len() {
            let base_temp = baseline_run
                .temperature
                .iter()
                .find(|value| value.region as usize == region)
                .map(|value| value.value);
            let control_temp = control_run
                .temperature
                .iter()
                .find(|value| value.region as usize == region)
                .map(|value| value.value);
            if let (Some(base), Some(control)) = (base_temp, control_temp) {
                assert!(
                    (base - control).abs() <= 1,
                    "baseline adjustment exceeds tolerance: {} vs {}",
                    base,
                    control
                );
            }
        }
    }
}
