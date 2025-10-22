/// Upper bound for the water meter (0.0 - 1.0 scaled by 10_000).
pub const WATER_MAX: u16 = 10_000;

/// Upper bound for the soil meter (0.0 - 1.0 scaled by 10_000).
pub const SOIL_MAX: u16 = 10_000;

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
    }
}
