use crate::fixed::{resource_ratio, WATER_MAX};
use crate::rng::Stream;
use crate::world::World;

use super::{HUMIDITY_NOISE_FRACTION, HUMIDITY_TENTHS_MAX, PRECIP_MAX_MM};

const INSOLATION_REFERENCE_TENTHS: f64 = 16_000.0;

pub(super) fn sample(world: &World, stream: &Stream) -> Vec<i32> {
    let mut humidity = Vec::with_capacity(world.regions.len());
    for (index, region) in world.regions.iter().enumerate() {
        debug_assert_eq!(
            region.index(),
            index,
            "region id {} does not match index {}",
            region.id,
            index
        );

        let mut region_rng = stream.derive(index as u64);
        let water_ratio = resource_ratio(region.water, WATER_MAX);
        let capped_precip = i32::from(region.precipitation_mm).clamp(0, PRECIP_MAX_MM);
        let precip_ratio = f64::from(capped_precip) / f64::from(PRECIP_MAX_MM);
        let insolation_tenths = world
            .climate
            .last_insolation_tenths
            .get(index)
            .copied()
            .unwrap_or(0);
        let insolation_ratio =
            (f64::from(insolation_tenths) / INSOLATION_REFERENCE_TENTHS).clamp(0.0, 1.0);
        let transport_driver =
            0.45 * water_ratio + 0.4 * precip_ratio + 0.15 * (1.0 - insolation_ratio);
        let jitter = region_rng.next_signed_unit() * HUMIDITY_NOISE_FRACTION;
        let ratio = (transport_driver + jitter).clamp(0.0, 1.0);
        let humidity_tenths = (ratio * f64::from(HUMIDITY_TENTHS_MAX)).round() as i32;
        humidity.push(humidity_tenths.clamp(0, HUMIDITY_TENTHS_MAX));
    }
    humidity
}
