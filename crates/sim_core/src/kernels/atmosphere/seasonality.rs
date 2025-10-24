use crate::world::World;

use super::{
    HADLEY_DRIFT_MAX_DEGREES, HADLEY_LATITUDE_MAX, PI, SEASONAL_INSOLATION_AMPLITUDE,
    SEASONAL_SCALAR_EPSILON, SEASON_PERIOD_TICKS, TAU,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct SeasonalityContext {
    pub scalar: f64,
    pub insolation_bias: f64,
    pub hadley_lat_shift: f64,
}

pub(super) fn compute(world: &World) -> SeasonalityContext {
    let scalar = seasonal_scalar(world.tick + 1);
    let insolation_bias = (1.0 + SEASONAL_INSOLATION_AMPLITUDE * scalar).clamp(
        1.0 - SEASONAL_INSOLATION_AMPLITUDE,
        1.0 + SEASONAL_INSOLATION_AMPLITUDE,
    );
    let hadley_lat_shift = HADLEY_DRIFT_MAX_DEGREES * scalar;

    SeasonalityContext {
        scalar,
        insolation_bias,
        hadley_lat_shift,
    }
}

pub(super) fn hadley_strength(latitude_deg: f64) -> f64 {
    if latitude_deg.abs() >= HADLEY_LATITUDE_MAX {
        0.0
    } else {
        1.0 - latitude_deg.abs() / HADLEY_LATITUDE_MAX
    }
}

pub(super) fn insolation_factor(latitude_deg: f64) -> f64 {
    let closeness = (90.0 - latitude_deg.abs()).max(0.0) / 90.0;
    closeness.powf(0.85)
}

pub(super) fn has_seasonal_variation(value: f64) -> bool {
    value.abs() > SEASONAL_SCALAR_EPSILON
}

pub(crate) fn scalar_for_tick(tick: u64) -> f64 {
    seasonal_scalar(tick)
}

fn wrap_angle(mut angle: f64) -> f64 {
    angle %= TAU;
    if angle > PI {
        angle -= TAU;
    } else if angle < -PI {
        angle += TAU;
    }
    angle
}

fn sin_series(angle: f64) -> f64 {
    let x = wrap_angle(angle);
    let x2 = x * x;
    let x3 = x * x2;
    let x5 = x3 * x2;
    let x7 = x5 * x2;
    let x9 = x7 * x2;
    let x11 = x9 * x2;
    let x13 = x11 * x2;
    x - x3 / 6.0 + x5 / 120.0 - x7 / 5_040.0 + x9 / 362_880.0 - x11 / 39_916_800.0
        + x13 / 6_227_020_800.0
}

fn seasonal_scalar(tick: u64) -> f64 {
    if SEASON_PERIOD_TICKS == 0 {
        return 0.0;
    }
    let tick_in_cycle = (tick % SEASON_PERIOD_TICKS) as f64;
    let phase = (tick_in_cycle / SEASON_PERIOD_TICKS as f64) * TAU;
    sin_series(phase)
}
