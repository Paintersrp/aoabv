use std::collections::HashSet;

use serde_json::Value;

use sim_core::cause::Code;
use sim_core::kernels::astronomy::{self, STAGE};
use sim_core::rng::Stream;
use sim_core::world::{Hazards, Region, World};

fn sample_world() -> World {
    let regions = vec![
        Region {
            id: 0,
            x: 0,
            y: 0,
            elevation_m: 120,
            latitude_deg: -10.0,
            biome: 2,
            water: 4_800,
            soil: 5_200,
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
            elevation_m: 450,
            latitude_deg: 12.5,
            biome: 3,
            water: 4_600,
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
            x: 0,
            y: 1,
            elevation_m: 1_100,
            latitude_deg: 44.0,
            biome: 4,
            water: 4_400,
            soil: 4_900,
            temperature_tenths_c: 0,
            precipitation_mm: 0,
            albedo_milli: 400,
            freshwater_flux_tenths_mm: 0,
            ice_mass_kilotons: 0,
            hazards: Hazards::default(),
        },
    ];
    World::new(42, 2, 2, regions)
}

fn assert_numbers_are_integers(value: &Value) {
    match value {
        Value::Number(number) => {
            assert!(number.as_i64().is_some(), "expected integer, got {number}");
        }
        Value::Array(elements) => {
            for element in elements {
                assert_numbers_are_integers(element);
            }
        }
        Value::Object(map) => {
            for element in map.values() {
                assert_numbers_are_integers(element);
            }
        }
        _ => {}
    }
}

#[test]
fn astronomy_diff_is_repeatable_and_integral() {
    let world = sample_world();
    let tick = 17;

    let mut rng_first = Stream::from(world.seed, STAGE, tick);
    let run_first = astronomy::update(&world, &mut rng_first).expect("astronomy update succeeds");
    let diff_first = run_first.diff;
    let chron_first = run_first.chronicle;

    let mut rng_second = Stream::from(world.seed, STAGE, tick);
    let run_second = astronomy::update(&world, &mut rng_second).expect("astronomy update succeeds");
    let diff_second = run_second.diff;
    let chron_second = run_second.chronicle;

    let diff_json_first = serde_json::to_value(&diff_first).expect("serialize diff");
    let diff_json_second = serde_json::to_value(&diff_second).expect("serialize diff");

    assert_eq!(
        diff_json_first, diff_json_second,
        "diff should be deterministic"
    );
    assert_eq!(
        chron_first, chron_second,
        "chronicle should be deterministic"
    );

    assert_numbers_are_integers(&diff_json_first);

    let valid_codes: HashSet<Code> = HashSet::from([
        Code::ObliquityShift,
        Code::PrecessionPhase,
        Code::SolarCyclePeak,
        Code::InsolationGradient,
        Code::TideNeap,
        Code::TideSpring,
    ]);

    assert!(
        !diff_first.causes.is_empty(),
        "astronomy kernel should emit cause entries"
    );
    for cause in &diff_first.causes {
        assert!(
            valid_codes.contains(&cause.code),
            "unexpected cause code {:?}",
            cause.code
        );
    }
}
