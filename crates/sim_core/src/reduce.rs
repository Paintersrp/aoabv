use crate::diff::Diff;
use crate::fixed::{
    clamp_biome_index, clamp_hazard_meter, clamp_i16, clamp_u16, commit_resource_delta, ALBEDO_MAX,
    FRESHWATER_FLUX_MAX, SOIL_MAX, WATER_MAX,
};
use crate::world::World;

const TEMP_MIN_TENTHS_C: i16 = -500;
const TEMP_MAX_TENTHS_C: i16 = 500;
const PRECIP_MAX_MM: u16 = 5_000;

pub fn apply(world: &mut World, mut diff: Diff) {
    world.climate.ensure_region_capacity(world.regions.len());
    diff.biome.sort_by_key(|change| change.region);
    diff.water.sort_by_key(|delta| delta.region);
    diff.soil.sort_by_key(|delta| delta.region);
    diff.insolation.sort_by_key(|value| value.region);
    diff.tide_envelope.sort_by_key(|value| value.region);
    diff.elevation.sort_by_key(|value| value.region);
    diff.temperature.sort_by_key(|value| value.region);
    diff.temperature_baseline.sort_by_key(|value| value.region);
    diff.precipitation.sort_by_key(|value| value.region);
    diff.precip_extreme.sort_by_key(|value| value.region);
    diff.humidity.sort_by_key(|value| value.region);
    diff.albedo.sort_by_key(|value| value.region);
    diff.permafrost_active.sort_by_key(|value| value.region);
    diff.freshwater_flux.sort_by_key(|value| value.region);
    diff.melt_pulse.sort_by_key(|value| value.region);
    diff.ice_mass.sort_by_key(|value| value.region);
    diff.heatwave_idx.sort_by_key(|value| value.region);
    diff.diag_climate.sort_by_key(|value| value.region);
    diff.hazards.sort_by_key(|hazard| hazard.region);

    for change in diff.biome {
        if let Some(region) = world.regions.get_mut(change.region as usize) {
            region.biome = clamp_biome_index(change.biome);
        }
    }

    for value in &diff.insolation {
        if let Some(slot) = world
            .climate
            .last_insolation_tenths
            .get_mut(value.region as usize)
        {
            *slot = value.value;
        }
    }

    for delta in diff.water {
        if let Some(region) = world.regions.get_mut(delta.region as usize) {
            region.water = commit_resource_delta(region.water, delta.delta, WATER_MAX);
        }
    }

    for delta in diff.soil {
        if let Some(region) = world.regions.get_mut(delta.region as usize) {
            region.soil = commit_resource_delta(region.soil, delta.delta, SOIL_MAX);
        }
    }

    for value in diff.elevation {
        if let Some(region) = world.regions.get_mut(value.region as usize) {
            region.elevation_m = value.value;
        }
    }

    for value in diff.temperature {
        if let Some(region) = world.regions.get_mut(value.region as usize) {
            region.temperature_tenths_c =
                clamp_i16(value.value, TEMP_MIN_TENTHS_C, TEMP_MAX_TENTHS_C);
        }
    }

    for value in diff.temperature_baseline {
        if let Some(slot) = world
            .climate
            .temperature_baseline_tenths
            .get_mut(value.region as usize)
        {
            *slot = clamp_i16(value.value, TEMP_MIN_TENTHS_C, TEMP_MAX_TENTHS_C);
        }
    }

    for value in diff.precipitation {
        if let Some(region) = world.regions.get_mut(value.region as usize) {
            region.precipitation_mm = clamp_u16(value.value, 0, PRECIP_MAX_MM);
        }
    }

    for value in diff.albedo {
        if let Some(region) = world.regions.get_mut(value.region as usize) {
            region.albedo_milli = clamp_u16(value.value, 0, ALBEDO_MAX);
        }
    }

    for value in diff.freshwater_flux {
        if let Some(region) = world.regions.get_mut(value.region as usize) {
            region.freshwater_flux_tenths_mm = clamp_u16(value.value, 0, FRESHWATER_FLUX_MAX);
        }
    }

    for value in diff.ice_mass {
        if let Some(region) = world.regions.get_mut(value.region as usize) {
            region.ice_mass_kilotons = value.value.max(0) as u32;
        }
    }

    for hazard in diff.hazards {
        if let Some(region) = world.regions.get_mut(hazard.region as usize) {
            region.hazards.drought = clamp_hazard_meter(hazard.drought);
            region.hazards.flood = clamp_hazard_meter(hazard.flood);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{BiomeChange, HazardEvent, ResourceDelta, ScalarValue};
    use crate::world::{Hazards, Region};
    use proptest::prelude::*;

    fn test_world() -> World {
        let regions = vec![
            Region {
                id: 0,
                x: 0,
                y: 0,
                elevation_m: 0,
                latitude_deg: 0.0,
                biome: 1,
                water: 1_000,
                soil: 9_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 350,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 1,
                x: 1,
                y: 0,
                elevation_m: 0,
                latitude_deg: 10.0,
                biome: 2,
                water: 5_000,
                soil: 100,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 360,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 2,
                x: 0,
                y: 1,
                elevation_m: 0,
                latitude_deg: -10.0,
                biome: 3,
                water: 9_900,
                soil: 6_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 370,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
            Region {
                id: 3,
                x: 1,
                y: 1,
                elevation_m: 0,
                latitude_deg: 20.0,
                biome: 4,
                water: 100,
                soil: 5_000,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: 380,
                freshwater_flux_tenths_mm: 0,
                ice_mass_kilotons: 0,
                hazards: Hazards::default(),
            },
        ];

        World::new(0, 2, 2, regions)
    }

    #[test]
    fn apply_sorts_entries_and_clamps_values() {
        let mut unsorted_diff = Diff::default();
        unsorted_diff.biome = vec![
            BiomeChange {
                region: 2,
                biome: 999,
            },
            BiomeChange {
                region: 0,
                biome: -5,
            },
            BiomeChange {
                region: 3,
                biome: 128,
            },
            BiomeChange {
                region: 1,
                biome: 42,
            },
        ];
        unsorted_diff.water = vec![
            ResourceDelta {
                region: 3,
                delta: -200,
            },
            ResourceDelta {
                region: 0,
                delta: 12_000,
            },
            ResourceDelta {
                region: 2,
                delta: 200,
            },
            ResourceDelta {
                region: 1,
                delta: -6_000,
            },
        ];
        unsorted_diff.soil = vec![
            ResourceDelta {
                region: 1,
                delta: 200,
            },
            ResourceDelta {
                region: 0,
                delta: -9_500,
            },
            ResourceDelta {
                region: 3,
                delta: -200,
            },
            ResourceDelta {
                region: 2,
                delta: 5_000,
            },
        ];
        unsorted_diff.insolation = vec![
            ScalarValue {
                region: 2,
                value: 200,
            },
            ScalarValue {
                region: 0,
                value: 150,
            },
            ScalarValue {
                region: 3,
                value: 50,
            },
            ScalarValue {
                region: 1,
                value: 175,
            },
        ];
        unsorted_diff.tide_envelope = vec![
            ScalarValue {
                region: 1,
                value: 30,
            },
            ScalarValue {
                region: 3,
                value: 60,
            },
            ScalarValue {
                region: 0,
                value: 20,
            },
            ScalarValue {
                region: 2,
                value: 40,
            },
        ];
        unsorted_diff.elevation = vec![
            ScalarValue {
                region: 2,
                value: 1_500,
            },
            ScalarValue {
                region: 0,
                value: -250,
            },
            ScalarValue {
                region: 3,
                value: 75,
            },
            ScalarValue {
                region: 1,
                value: 40,
            },
        ];
        unsorted_diff.temperature = vec![
            ScalarValue {
                region: 3,
                value: 1_000,
            },
            ScalarValue {
                region: 0,
                value: 150,
            },
            ScalarValue {
                region: 2,
                value: 375,
            },
            ScalarValue {
                region: 1,
                value: -700,
            },
        ];
        unsorted_diff.precipitation = vec![
            ScalarValue {
                region: 2,
                value: 6_000,
            },
            ScalarValue {
                region: 0,
                value: -50,
            },
            ScalarValue {
                region: 3,
                value: 4_500,
            },
            ScalarValue {
                region: 1,
                value: 200,
            },
        ];
        unsorted_diff.hazards = vec![
            HazardEvent {
                region: 3,
                drought: 15_000,
                flood: 200,
            },
            HazardEvent {
                region: 0,
                drought: 5,
                flood: 700,
            },
            HazardEvent {
                region: 2,
                drought: 65_000,
                flood: 65_535,
            },
            HazardEvent {
                region: 1,
                drought: 250,
                flood: 12_000,
            },
        ];

        let mut sorted_diff = unsorted_diff.clone();
        sorted_diff.biome.sort_by_key(|change| change.region);
        sorted_diff.water.sort_by_key(|delta| delta.region);
        sorted_diff.soil.sort_by_key(|delta| delta.region);
        sorted_diff.insolation.sort_by_key(|value| value.region);
        sorted_diff.tide_envelope.sort_by_key(|value| value.region);
        sorted_diff.elevation.sort_by_key(|value| value.region);
        sorted_diff.temperature.sort_by_key(|value| value.region);
        sorted_diff.precipitation.sort_by_key(|value| value.region);
        sorted_diff.hazards.sort_by_key(|hazard| hazard.region);

        let mut world_from_unsorted = test_world();
        let mut world_from_sorted = test_world();

        apply(&mut world_from_unsorted, unsorted_diff);
        apply(&mut world_from_sorted, sorted_diff);

        for (left, right) in world_from_unsorted
            .regions
            .iter()
            .zip(world_from_sorted.regions.iter())
        {
            assert_eq!(left.id, right.id);
            assert_eq!(left.biome, right.biome);
            assert_eq!(left.water, right.water);
            assert_eq!(left.soil, right.soil);
            assert_eq!(left.temperature_tenths_c, right.temperature_tenths_c);
            assert_eq!(left.precipitation_mm, right.precipitation_mm);
            assert_eq!(left.hazards.drought, right.hazards.drought);
            assert_eq!(left.hazards.flood, right.hazards.flood);
        }

        let region0 = &world_from_unsorted.regions[0];
        assert_eq!(region0.biome, 0);
        assert_eq!(region0.water, crate::fixed::WATER_MAX);
        assert_eq!(region0.soil, 0);
        assert_eq!(region0.elevation_m, -250);
        assert_eq!(region0.temperature_tenths_c, 150);
        assert_eq!(region0.precipitation_mm, 0);
        assert_eq!(region0.hazards.drought, 5);
        assert_eq!(region0.hazards.flood, 700);

        let region1 = &world_from_unsorted.regions[1];
        assert_eq!(region1.biome, 42);
        assert_eq!(region1.water, 0);
        assert_eq!(region1.soil, 300);
        assert_eq!(region1.elevation_m, 40);
        assert_eq!(region1.temperature_tenths_c, -500);
        assert_eq!(region1.precipitation_mm, 200);
        assert_eq!(region1.hazards.drought, 250);
        assert_eq!(region1.hazards.flood, crate::fixed::WATER_MAX);

        let region2 = &world_from_unsorted.regions[2];
        assert_eq!(region2.biome, u8::MAX);
        assert_eq!(region2.water, crate::fixed::WATER_MAX);
        assert_eq!(region2.soil, crate::fixed::SOIL_MAX);
        assert_eq!(region2.elevation_m, 1_500);
        assert_eq!(region2.temperature_tenths_c, 375);
        assert_eq!(region2.precipitation_mm, 5_000);
        assert_eq!(region2.hazards.drought, crate::fixed::WATER_MAX);
        assert_eq!(region2.hazards.flood, crate::fixed::WATER_MAX);

        let region3 = &world_from_unsorted.regions[3];
        assert_eq!(region3.biome, 128);
        assert_eq!(region3.water, 0);
        assert_eq!(region3.soil, 4_800);
        assert_eq!(region3.temperature_tenths_c, 500);
        assert_eq!(region3.precipitation_mm, 4_500);
        assert_eq!(region3.hazards.drought, crate::fixed::WATER_MAX);
        assert_eq!(region3.hazards.flood, 200);
    }

    proptest! {
        #[test]
        fn apply_is_order_independent_for_scalar_vectors(values in proptest::collection::vec(-4_000i32..4_000, 4)) {
            let regions = [3u32, 1, 2, 0];
            let mut unsorted = Diff::default();
            unsorted.insolation = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| ScalarValue { region: *region, value: *value })
                .collect();
            unsorted.tide_envelope = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| ScalarValue { region: *region, value: value - 25 })
                .collect();
            unsorted.elevation = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| ScalarValue { region: *region, value: value + 200 })
                .collect();
            unsorted.temperature = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| ScalarValue { region: *region, value: *value })
                .collect();
            unsorted.temperature_baseline = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| ScalarValue { region: *region, value: value / 2 })
                .collect();
            unsorted.precipitation = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| ScalarValue { region: *region, value: value.abs() })
                .collect();
            unsorted.precip_extreme = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| {
                    let extreme = if *value == 0 { 1 } else { *value };
                    ScalarValue { region: *region, value: extreme }
                })
                .collect();
            unsorted.humidity = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| ScalarValue { region: *region, value: value + 50 })
                .collect();
            unsorted.albedo = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| {
                    let albedo = (value.abs() % 900) + 100;
                    ScalarValue { region: *region, value: albedo }
                })
                .collect();
            unsorted.permafrost_active = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| ScalarValue { region: *region, value: value - 10 })
                .collect();
            unsorted.freshwater_flux = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| ScalarValue { region: *region, value: value.abs() })
                .collect();
            unsorted.melt_pulse = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| {
                    let melt = if *value == 0 { 3 } else { value.abs() };
                    ScalarValue { region: *region, value: melt }
                })
                .collect();
            unsorted.ice_mass = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| ScalarValue { region: *region, value: (value.abs() + 50) })
                .collect();
            unsorted.heatwave_idx = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| {
                    let anomaly = if *value == 0 { 2 } else { *value };
                    ScalarValue { region: *region, value: anomaly }
                })
                .collect();
            unsorted.diag_climate = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| ScalarValue { region: *region, value: *value })
                .collect();

            unsorted.water = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| ResourceDelta {
                    region: *region,
                    delta: if *value == 0 { 1 } else { *value },
                })
                .collect();
            unsorted.soil = regions
                .iter()
                .zip(values.iter())
                .map(|(region, value)| ResourceDelta {
                    region: *region,
                    delta: if *value == 0 { -1 } else { -*value },
                })
                .collect();

            let mut sorted = unsorted.clone();
            sorted.insolation.sort_by_key(|value| value.region);
            sorted.tide_envelope.sort_by_key(|value| value.region);
            sorted.elevation.sort_by_key(|value| value.region);
            sorted.temperature.sort_by_key(|value| value.region);
            sorted.temperature_baseline.sort_by_key(|value| value.region);
            sorted.precipitation.sort_by_key(|value| value.region);
            sorted.precip_extreme.sort_by_key(|value| value.region);
            sorted.humidity.sort_by_key(|value| value.region);
            sorted.albedo.sort_by_key(|value| value.region);
            sorted.permafrost_active.sort_by_key(|value| value.region);
            sorted.freshwater_flux.sort_by_key(|value| value.region);
            sorted.melt_pulse.sort_by_key(|value| value.region);
            sorted.ice_mass.sort_by_key(|value| value.region);
            sorted.heatwave_idx.sort_by_key(|value| value.region);
            sorted.diag_climate.sort_by_key(|value| value.region);
            sorted.water.sort_by_key(|delta| delta.region);
            sorted.soil.sort_by_key(|delta| delta.region);

            let mut world_unsorted = test_world();
            let mut world_sorted = test_world();

            apply(&mut world_unsorted, unsorted);
            apply(&mut world_sorted, sorted);

            for (left, right) in world_unsorted
                .regions
                .iter()
                .zip(world_sorted.regions.iter())
            {
                assert_eq!(left.biome, right.biome);
                assert_eq!(left.water, right.water);
                assert_eq!(left.soil, right.soil);
                assert_eq!(left.temperature_tenths_c, right.temperature_tenths_c);
                assert_eq!(left.precipitation_mm, right.precipitation_mm);
                assert_eq!(left.albedo_milli, right.albedo_milli);
                assert_eq!(
                    left.freshwater_flux_tenths_mm,
                    right.freshwater_flux_tenths_mm
                );
                assert_eq!(left.ice_mass_kilotons, right.ice_mass_kilotons);
            }

            assert_eq!(
                world_unsorted.climate.temperature_baseline_tenths,
                world_sorted.climate.temperature_baseline_tenths
            );
            assert_eq!(
                world_unsorted.climate.last_insolation_tenths,
                world_sorted.climate.last_insolation_tenths
            );
        }
    }
}
