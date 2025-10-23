use anyhow::{ensure, Result};

use crate::cause::{Code, Entry};
use crate::diff::Diff;
use crate::rng::Stream;
use crate::world::World;

pub const STAGE: &str = "kernel:geodynamics";

const EVENT_DENOMINATOR: u64 = 1_000;
const MIN_ELEVATION_M: i32 = -1_000; // TODO(agents): rationale — extend seed clamp for bathymetry adjustments.
const MAX_ELEVATION_M: i32 = 4_000; // TODO(agents): rationale — allow moderate uplift beyond seed cap.

const NEIGHBOR_OFFSETS: &[(i32, i32)] = &[(-1, 0), (1, 0), (0, -1), (0, 1)];

pub fn update(world: &World, rng: &mut Stream) -> Result<(Diff, Vec<String>)> {
    let mut diff = Diff::default();
    let mut chronicle = Vec::new();

    let width = world.width as i32;
    let height = world.height as i32;

    for (index, region) in world.regions.iter().enumerate() {
        ensure!(
            region.index() == index,
            "region id {} does not match index {}",
            region.id,
            index
        );

        let mut region_rng = rng.derive(region.index() as u64);
        if region_rng.next_u64() % EVENT_DENOMINATOR != 0 {
            continue;
        }

        let uplift = region_rng.next_u64() & 1 == 0;
        let magnitude_m = (region_rng.next_f64() * 90.0 + 10.0).round() as i32;
        let primary_delta = if uplift { magnitude_m } else { -magnitude_m };
        let primary_new = clamp_elevation(region.elevation_m.saturating_add(primary_delta));

        diff.record_elevation(index, primary_new);

        let cause_code = if uplift {
            Code::OrogenyBelt
        } else {
            Code::SubsidenceDeltas
        };
        diff.record_cause(Entry::new(
            format!("region:{}/elevation", region.id),
            cause_code,
            Some(format!("delta_m={:+}", primary_delta)),
        ));

        let neighbor_delta = (primary_delta / 2).clamp(-50, 50);
        if neighbor_delta != 0 {
            for (dx, dy) in NEIGHBOR_OFFSETS {
                let nx = region.x as i32 + dx;
                let ny = region.y as i32 + dy;
                if nx < 0 || nx >= width || ny < 0 || ny >= height {
                    continue;
                }
                let neighbor_index = (ny * width + nx) as usize;
                if let Some(neighbor) = world.regions.get(neighbor_index) {
                    let neighbor_new =
                        clamp_elevation(neighbor.elevation_m.saturating_add(neighbor_delta));
                    diff.record_elevation(neighbor_index, neighbor_new);
                }
            }
        }

        let aerosol_tau = region_rng.next_f64() * 0.05 + 0.01;
        diff.record_cause(Entry::new(
            "world:atmosphere",
            Code::VolcanicAerosolPulse,
            Some(format!(
                "region={} optical_depth={:.3}",
                region.id, aerosol_tau
            )),
        ));

        let descriptor = if uplift { "uplift" } else { "collapse" };
        chronicle.push(format!(
            "Volcanic {} near region {} adjusted terrain by {:+} m.",
            descriptor, region.id, primary_delta
        ));
    }

    Ok((diff, chronicle))
}

fn clamp_elevation(value: i32) -> i32 {
    value.clamp(MIN_ELEVATION_M, MAX_ELEVATION_M)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Stream;
    use crate::world::{Hazards, Region, World};

    fn test_world() -> World {
        let regions = vec![Region {
            id: 0,
            x: 0,
            y: 0,
            elevation_m: 1200,
            latitude_deg: 0.0,
            biome: 0,
            water: 5_000,
            soil: 5_000,
            hazards: Hazards::default(),
        }];
        World::new(0, 1, 1, regions)
    }

    #[test]
    fn update_is_often_noop() {
        let world = test_world();
        let mut rng = Stream::from(world.seed, STAGE, 1);
        let (diff, chronicle) = update(&world, &mut rng).expect("geodynamics update succeeds");
        // Most ticks should be empty; ensure deterministic empty case allowed.
        assert!(diff.elevation.len() <= 5);
        assert!(chronicle.len() <= diff.elevation.len());
    }

    #[test]
    fn eventually_triggers_event() {
        let world = test_world();
        let mut triggered = None;
        for tick in 1..=5_000 {
            let mut rng = Stream::from(world.seed, STAGE, tick);
            let (diff, chronicle) = update(&world, &mut rng).expect("geodynamics update succeeds");
            if !diff.elevation.is_empty() {
                triggered = Some((tick, diff, chronicle));
                break;
            }
        }
        let (tick, diff, chronicle) = triggered.expect("event triggers within sample window");
        assert!(tick <= 5_000);
        assert!(!diff.elevation.is_empty());
        assert!(!chronicle.is_empty());
    }
}
