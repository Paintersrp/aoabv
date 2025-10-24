use crate::world::{Region, World};

pub(super) fn orographic_lift_indicator(world: &World, region: &Region) -> f64 {
    let width = world.width as i32;
    let height = world.height as i32;
    let x = region.x as i32;
    let y = region.y as i32;
    let mut sum = 0_i64;
    let mut count = 0_i32;
    const OFFSETS: &[(i32, i32)] = &[(-1, 0), (1, 0), (0, -1), (0, 1)];
    for (dx, dy) in OFFSETS {
        let nx = x + dx;
        let ny = y + dy;
        if nx < 0 || nx >= width || ny < 0 || ny >= height {
            continue;
        }
        let neighbor_index = (ny * width + nx) as usize;
        if let Some(neighbor) = world.regions.get(neighbor_index) {
            sum += i64::from(neighbor.elevation_m);
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    let neighbor_mean = sum as f64 / f64::from(count);
    ((f64::from(region.elevation_m) - neighbor_mean) / 1_000.0).max(0.0)
}
