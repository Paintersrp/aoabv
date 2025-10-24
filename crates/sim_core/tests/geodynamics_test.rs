use serde_json::Value;

use sim_core::cause::Code;
use sim_core::kernels::geodynamics::{self, STAGE};
use sim_core::rng::Stream;
use sim_core::world::{Hazards, Region, World};

const MIN_ELEVATION_M: i32 = -1_000;
const MAX_ELEVATION_M: i32 = 4_000;

fn sample_world() -> World {
    let regions = vec![
        Region {
            id: 0,
            x: 0,
            y: 0,
            elevation_m: 1_200,
            latitude_deg: -5.0,
            biome: 3,
            water: 5_000,
            soil: 5_000,
            temperature_tenths_c: 0,
            precipitation_mm: 0,
            albedo_milli: 450,
            freshwater_flux_tenths_mm: 0,
            ice_mass_kilotons: 0,
            hazards: Hazards::default(),
        },
        Region {
            id: 1,
            x: 1,
            y: 0,
            elevation_m: 900,
            latitude_deg: 15.0,
            biome: 4,
            water: 5_100,
            soil: 4_900,
            temperature_tenths_c: 0,
            precipitation_mm: 0,
            albedo_milli: 420,
            freshwater_flux_tenths_mm: 0,
            ice_mass_kilotons: 0,
            hazards: Hazards::default(),
        },
        Region {
            id: 2,
            x: 0,
            y: 1,
            elevation_m: 350,
            latitude_deg: 32.5,
            biome: 2,
            water: 4_950,
            soil: 5_050,
            temperature_tenths_c: 0,
            precipitation_mm: 0,
            albedo_milli: 410,
            freshwater_flux_tenths_mm: 0,
            ice_mass_kilotons: 0,
            hazards: Hazards::default(),
        },
        Region {
            id: 3,
            x: 1,
            y: 1,
            elevation_m: 60,
            latitude_deg: 48.0,
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
    ];
    World::new(99, 2, 2, regions)
}

fn serialize_diff(diff: &sim_core::diff::Diff) -> Value {
    serde_json::to_value(diff).expect("serialize diff")
}

#[test]
fn geodynamics_outputs_are_deterministic_for_seed_and_tick() {
    let world = sample_world();
    let tick = 512;

    let mut rng_first = Stream::from(world.seed, STAGE, tick);
    let (diff_first, chron_first) =
        geodynamics::update(&world, &mut rng_first).expect("geodynamics update succeeds");

    let mut rng_second = Stream::from(world.seed, STAGE, tick);
    let (diff_second, chron_second) =
        geodynamics::update(&world, &mut rng_second).expect("geodynamics update succeeds");

    assert_eq!(serialize_diff(&diff_first), serialize_diff(&diff_second));
    assert_eq!(chron_first, chron_second);
}

#[test]
fn geodynamics_elevation_adjustments_remain_bounded() {
    let world = sample_world();

    let mut triggered = None;
    for tick in 1..=20_000 {
        let mut rng = Stream::from(world.seed, STAGE, tick);
        let (diff, _chronicle) =
            geodynamics::update(&world, &mut rng).expect("geodynamics update succeeds");
        if !diff.elevation.is_empty() {
            triggered = Some(diff);
            break;
        }
    }

    let diff = triggered.expect("expected volcanic event within search window");
    for scalar in &diff.elevation {
        assert!(
            (MIN_ELEVATION_M..=MAX_ELEVATION_M).contains(&scalar.value),
            "elevation {} out of bounds",
            scalar.value
        );
    }
}

#[test]
fn geodynamics_handles_event_hits_and_misses() {
    let world = sample_world();

    let mut miss_tick = None;
    let mut hit_tick = None;
    let mut hit_diff = None;
    let mut hit_chronicle = None;
    for tick in 1..=20_000 {
        let mut rng = Stream::from(world.seed, STAGE, tick);
        let (diff, chronicle) =
            geodynamics::update(&world, &mut rng).expect("geodynamics update succeeds");
        if diff.elevation.is_empty() {
            if miss_tick.is_none() {
                miss_tick = Some((tick, diff.clone(), chronicle.clone()));
            }
        } else if hit_tick.is_none() {
            hit_tick = Some(tick);
            hit_diff = Some(diff);
            hit_chronicle = Some(chronicle);
        }
        if miss_tick.is_some() && hit_tick.is_some() {
            break;
        }
    }

    let (miss_tick, miss_diff, miss_chronicle) =
        miss_tick.expect("expected to observe a no-event tick");
    assert!(miss_diff.elevation.is_empty());
    assert!(miss_diff.causes.is_empty());
    assert!(miss_chronicle.is_empty());

    let hit_tick = hit_tick.expect("expected to observe a volcanic event");
    let hit_diff = hit_diff.expect("captured diff for event");
    let hit_chronicle = hit_chronicle.expect("captured chronicle for event");

    assert!(!hit_diff.elevation.is_empty());
    assert!(!hit_chronicle.is_empty());
    assert!(hit_diff
        .causes
        .iter()
        .any(|entry| matches!(entry.code, Code::OrogenyBelt | Code::SubsidenceDeltas)));
    assert!(hit_diff
        .causes
        .iter()
        .any(|entry| entry.code == Code::VolcanicAerosolPulse));

    // Determinism: rerun the hit tick and ensure it matches cached results.
    let mut rng = Stream::from(world.seed, STAGE, hit_tick);
    let (repeat_diff, repeat_chronicle) =
        geodynamics::update(&world, &mut rng).expect("geodynamics update succeeds");
    assert_eq!(serialize_diff(&repeat_diff), serialize_diff(&hit_diff));
    assert_eq!(repeat_chronicle, hit_chronicle);

    // Ensure the no-hit tick differs from the event tick.
    assert_ne!(miss_tick, hit_tick);
}
