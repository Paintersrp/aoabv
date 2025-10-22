/// Upper bound for water/soil metrics (0.0 - 1.0 scaled by 10_000).
pub const RESOURCE_MAX: u16 = 10_000;

/// Apply a signed delta to a bounded resource value, clamping to `[0, RESOURCE_MAX]`.
pub fn apply_resource_delta(current: u16, delta: i32) -> u16 {
    let value = current as i32 + delta;
    clamp_resource(value)
}

/// Clamp an integer value to the valid resource domain.
pub fn clamp_resource(value: i32) -> u16 {
    value.clamp(0, RESOURCE_MAX as i32) as u16
}

/// Convert a resource level to a `[0.0, 1.0]` scalar.
pub fn resource_ratio(value: u16) -> f64 {
    f64::from(value) / f64::from(RESOURCE_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn apply_resource_delta_never_exits_bounds(current in 0u16..=RESOURCE_MAX, delta in -20_000i32..20_000i32) {
            let next = apply_resource_delta(current, delta);
            prop_assert!(next <= RESOURCE_MAX);
            prop_assert!(next >= 0);
        }

        #[test]
        fn clamp_resource_bounded(value in -50_000i32..50_000i32) {
            let clamped = clamp_resource(value);
            prop_assert!(clamped <= RESOURCE_MAX);
            prop_assert!(clamped >= 0);
        }
    }
}
