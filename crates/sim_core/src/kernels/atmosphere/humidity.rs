use crate::fixed::{resource_ratio, WATER_MAX};
use crate::rng::Stream;
use crate::world::World;

use super::HUMIDITY_NOISE_FRACTION;

pub(super) fn sample(world: &World, stream: &Stream) -> Vec<f64> {
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
        let base_ratio = resource_ratio(region.water, WATER_MAX);
        let jitter = region_rng.next_signed_unit() * HUMIDITY_NOISE_FRACTION;
        let ratio = (base_ratio + jitter).clamp(0.0, 1.0);
        humidity.push(ratio);
    }
    humidity
}
