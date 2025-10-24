use anyhow::{ensure, Result};

use crate::cause::{Code, Entry};
use crate::diff::Diff;
use crate::kernels::atmosphere::{seasonality, SEASONAL_INSOLATION_AMPLITUDE};
use crate::rng::Stream;
use crate::schedule::KernelRun;
use crate::world::World;

pub const STAGE: &str = "kernel:astronomy";

const SOLAR_CONSTANT_WM2: f64 = 1_361.0;
const OBLIQUITY_BASE_DEG: f64 = 23.44;
const LAT_POWER: f64 = 0.8;
const TIDE_EQUATOR_METERS: f64 = 3.2;
const TIDE_POLE_METERS: f64 = 1.2;

fn lat_factor(latitude_deg: f64) -> f64 {
    let closeness = (90.0 - latitude_deg.abs()).max(0.0) / 90.0;
    closeness.powf(LAT_POWER)
}

fn to_tenths(value: f64) -> i32 {
    (value * 10.0).round() as i32
}

pub fn update(world: &World, rng: &mut Stream) -> Result<KernelRun> {
    let mut diff = Diff::default();
    let mut chronicle = Vec::new();

    let obliquity_delta = rng.next_signed_unit() * 0.35;
    let obliquity_deg = OBLIQUITY_BASE_DEG + obliquity_delta;
    let precession_phase = rng.next_f64() * 360.0;
    let solar_cycle_position = rng.next_f64();
    let solar_cycle_index = (solar_cycle_position * 25.0).floor() as u32;
    let solar_cycle_amplitude = 1.0 + (solar_cycle_position - 0.5) * 0.1;
    let lunar_phase = rng.next_f64();
    let lunar_wave = lunar_phase * 2.0 - 1.0;
    let seasonal_scalar = seasonality::scalar_for_tick(world.tick + 1);
    let seasonal_bias = (1.0 + SEASONAL_INSOLATION_AMPLITUDE * seasonal_scalar).clamp(
        1.0 - SEASONAL_INSOLATION_AMPLITUDE,
        1.0 + SEASONAL_INSOLATION_AMPLITUDE,
    );

    diff.record_cause(Entry::new(
        "world:astronomy",
        Code::ObliquityShift,
        Some(format!("delta_deg={:.2}", obliquity_delta)),
    ));
    diff.record_cause(Entry::new(
        "world:astronomy",
        Code::PrecessionPhase,
        Some(format!("phase_deg={:.1}", precession_phase)),
    ));
    diff.record_cause(Entry::new(
        "world:astronomy",
        Code::SolarCyclePeak,
        Some(format!("cycle_index={}", solar_cycle_index)),
    ));

    let equatorial_insolation =
        SOLAR_CONSTANT_WM2 * solar_cycle_amplitude * seasonal_bias * (0.35 + 0.65);

    for (index, region) in world.regions.iter().enumerate() {
        ensure!(
            region.index() == index,
            "region id {} does not match index {}",
            region.id,
            index
        );

        let lat_effect = lat_factor(region.latitude_deg);
        let insolation_wm2 = SOLAR_CONSTANT_WM2
            * solar_cycle_amplitude
            * seasonal_bias
            * (0.35 + 0.65 * lat_effect * (obliquity_deg / OBLIQUITY_BASE_DEG));
        diff.record_insolation(index, to_tenths(insolation_wm2));

        let delta_wm2 = (equatorial_insolation - insolation_wm2).abs();
        diff.record_cause(Entry::new(
            format!("region:{}/insolation", region.id),
            Code::InsolationGradient,
            Some(format!("delta_wm2={:.1}", delta_wm2)),
        ));

        let tide_lat_component =
            TIDE_POLE_METERS + (TIDE_EQUATOR_METERS - TIDE_POLE_METERS) * lat_effect;
        let tide_envelope_m = tide_lat_component * (1.0 + 0.25 * lunar_wave);
        diff.record_tide_envelope(index, to_tenths(tide_envelope_m));

        let tide_code = if lunar_wave >= 0.0 {
            Code::TideSpring
        } else {
            Code::TideNeap
        };
        diff.record_cause(Entry::new(
            format!("region:{}/tide", region.id),
            tide_code,
            Some(format!("phase={:.3}", lunar_wave)),
        ));
    }

    let tide_summary = if lunar_wave >= 0.0 {
        "Spring tides amplify coastal forces."
    } else {
        "Neap tides calm coastal forces."
    };
    chronicle.push(format!(
        "Axial tilt shifted by {:+.2}°, precession at {:.0}°, {}",
        obliquity_delta, precession_phase, tide_summary
    ));

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
    fn astronomy_update_populates_diff_and_chronicle() {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 0,
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
                elevation_m: 0,
                latitude_deg: 45.0,
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
        let world = World::new(0, 2, 1, regions);
        let mut rng = Stream::from(world.seed, STAGE, 1);

        let run = update(&world, &mut rng).expect("astronomy update succeeds");
        let diff = run.diff;
        let chronicle = run.chronicle;

        assert_eq!(diff.insolation.len(), 2);
        assert_eq!(diff.tide_envelope.len(), 2);
        assert!(!diff.causes.is_empty());
        assert_eq!(chronicle.len(), 1);
    }
}
