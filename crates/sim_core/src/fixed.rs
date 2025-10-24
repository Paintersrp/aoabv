/// Upper bound for the water meter (0.0 - 1.0 scaled by 10_000).
pub const WATER_MAX: u16 = 10_000;

/// Upper bound for the soil meter (0.0 - 1.0 scaled by 10_000).
pub const SOIL_MAX: u16 = 10_000;

/// Upper bound for snow/ice albedo values represented in milli-units.
pub const ALBEDO_MAX: u16 = 1_000;

/// Upper bound for freshwater flux pulses represented in tenths of millimetres.
pub const FRESHWATER_FLUX_MAX: u16 = 2_000;

/// Clamp an integer value to a bounded `u16` range.
pub fn clamp_u16(value: i32, min: u16, max: u16) -> u16 {
    debug_assert!(min <= max);
    value.clamp(min as i32, max as i32) as u16
}

/// Clamp an integer value to a bounded `i16` range.
pub fn clamp_i16(value: i32, min: i16, max: i16) -> i16 {
    debug_assert!(min <= max);
    value.clamp(min as i32, max as i32) as i16
}

/// Clamp a biome index to the valid `[0, u8::MAX]` range.
pub fn clamp_biome_index(value: i32) -> u8 {
    value.clamp(u8::MIN as i32, u8::MAX as i32) as u8
}

/// Apply a signed delta to a resource meter, returning the clamped value.
pub fn commit_resource_delta(current: u16, delta: i32, max: u16) -> u16 {
    let next = i32::from(current) + delta;
    clamp_u16(next, 0, max)
}

/// Clamp hazard meters to the water range bounds.
pub fn clamp_hazard_meter(value: u16) -> u16 {
    clamp_u16(i32::from(value), 0, WATER_MAX)
}

/// Convert a resource level to a `[0.0, 1.0]` scalar using the provided maximum.
pub fn resource_ratio(value: u16, max: u16) -> f64 {
    let max = if max == 0 { 1 } else { max };
    let clamped = value.min(max);
    f64::from(clamped) / f64::from(max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn clamp_u16_never_exits_bounds(value in -50_000i32..50_000i32) {
            let clamped = clamp_u16(value, 0, WATER_MAX);
            prop_assert!(clamped <= WATER_MAX);
            prop_assert!(i32::from(clamped) >= 0);
        }

        #[test]
        fn clamp_i16_never_exits_bounds(value in -50_000i32..50_000i32) {
            let clamped = clamp_i16(value, -5_000, 5_000);
            prop_assert!(clamped <= 5_000);
            prop_assert!(clamped >= -5_000);
        }

        #[test]
        fn clamp_biome_index_never_exits_bounds(value in -50_000i32..50_000i32) {
            let clamped = clamp_biome_index(value);
            prop_assert!(clamped <= u8::MAX);
        }

        #[test]
        fn commit_resource_delta_never_exits_bounds(
            current in 0u16..=WATER_MAX,
            delta in -50_000i32..50_000i32,
        ) {
            let clamped = commit_resource_delta(current, delta, WATER_MAX);
            prop_assert!(clamped <= WATER_MAX);
            prop_assert!(i32::from(clamped) >= 0);
        }

        #[test]
        fn clamp_hazard_meter_never_exits_bounds(value in 0u16..=u16::MAX) {
            let clamped = clamp_hazard_meter(value);
            prop_assert!(clamped <= WATER_MAX);
        }
    }
}
