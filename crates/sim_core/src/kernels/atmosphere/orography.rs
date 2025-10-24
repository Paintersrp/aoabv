use crate::rng::Stream;
use crate::world::World;

use super::{
    HUMIDITY_TENTHS_MAX, OROGRAPHIC_LIFT_THRESHOLD_KM, PRECIP_MULTIPLIER_MAX,
    PRECIP_MULTIPLIER_MIN, RAIN_SHADOW_MAX,
};

#[derive(Debug)]
pub(super) struct OrographyEffects {
    pub precip_multipliers: Vec<f64>,
    pub lift_gradients: Vec<f64>,
    pub lift_multipliers: Vec<f64>,
    pub rain_shadow_factors: Vec<f64>,
}

pub(super) fn apply(
    world: &World,
    stream: &Stream,
    humidity_tenths: &mut [i32],
) -> OrographyEffects {
    let total_regions = world.regions.len();
    let mut precip_multipliers = vec![1.0f64; total_regions];
    let mut lift_gradients = vec![0.0f64; total_regions];
    let mut lift_multipliers = vec![1.0f64; total_regions];
    let mut rain_shadow_factors = vec![0.0f64; total_regions];
    let tenths_max = f64::from(HUMIDITY_TENTHS_MAX);

    for (index, region) in world.regions.iter().enumerate() {
        let (wind_dx, wind_dy) = prevailing_wind(region.latitude_deg);
        if wind_dx == 0 && wind_dy == 0 {
            continue;
        }

        let mut effect_rng = stream.derive(index as u64);
        let lift_jitter = effect_rng.next_f64();
        let shadow_jitter = effect_rng.next_f64();

        let upwind_x = region.x as i32 - wind_dx;
        let upwind_y = region.y as i32 - wind_dy;
        if let Some(upwind_index) = region_index_at(world, upwind_x, upwind_y) {
            let upwind = &world.regions[upwind_index];
            let gradient_km = f64::from(region.elevation_m - upwind.elevation_m) / 1_000.0;
            if gradient_km >= OROGRAPHIC_LIFT_THRESHOLD_KM {
                let random_factor = 0.85 + lift_jitter * 0.3;
                let lift = gradient_km * 0.25 * random_factor;
                let mut humidity_ratio = f64::from(humidity_tenths[index]) / tenths_max;
                humidity_ratio = (humidity_ratio + lift).clamp(0.0, 1.0);
                humidity_tenths[index] =
                    ((humidity_ratio * tenths_max).round() as i32).clamp(0, HUMIDITY_TENTHS_MAX);
                let multiplier = (1.0 + lift * 0.8).clamp(1.0, PRECIP_MULTIPLIER_MAX);
                precip_multipliers[index] *= multiplier;
                lift_gradients[index] = gradient_km;
                lift_multipliers[index] = precip_multipliers[index];

                let downwind_x = region.x as i32 + wind_dx;
                let downwind_y = region.y as i32 + wind_dy;
                if let Some(downwind_index) = region_index_at(world, downwind_x, downwind_y) {
                    let dryness_base = gradient_km * (0.18 + shadow_jitter * 0.12);
                    let dryness = dryness_base.clamp(0.0, RAIN_SHADOW_MAX);
                    let mut downwind_ratio =
                        f64::from(humidity_tenths[downwind_index]) / tenths_max;
                    downwind_ratio = (downwind_ratio * (1.0 - dryness)).clamp(0.0, 1.0);
                    humidity_tenths[downwind_index] = ((downwind_ratio * tenths_max).round()
                        as i32)
                        .clamp(0, HUMIDITY_TENTHS_MAX);
                    let rain_multiplier = (1.0 - dryness * 0.65).clamp(PRECIP_MULTIPLIER_MIN, 1.0);
                    precip_multipliers[downwind_index] *= rain_multiplier;
                    rain_shadow_factors[downwind_index] =
                        rain_shadow_factors[downwind_index].max(dryness);
                }
            }
        }
    }

    OrographyEffects {
        precip_multipliers,
        lift_gradients,
        lift_multipliers,
        rain_shadow_factors,
    }
}

fn prevailing_wind(latitude_deg: f64) -> (i32, i32) {
    let abs_lat = latitude_deg.abs();
    if abs_lat < 30.0 {
        (-1, 0)
    } else if abs_lat < 60.0 {
        (1, 0)
    } else {
        (-1, 0)
    }
}

fn region_index_at(world: &World, x: i32, y: i32) -> Option<usize> {
    if x < 0 || y < 0 {
        return None;
    }
    let (width, height) = (world.width as i32, world.height as i32);
    if x >= width || y >= height {
        return None;
    }
    let idx = (y as usize) * (world.width as usize) + (x as usize);
    if idx < world.regions.len() {
        let region = &world.regions[idx];
        if region.x as i32 == x && region.y as i32 == y {
            return Some(idx);
        }
    }
    world
        .regions
        .iter()
        .enumerate()
        .find(|(_, region)| region.x as i32 == x && region.y as i32 == y)
        .map(|(index, _)| index)
}
