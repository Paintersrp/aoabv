use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::fixed::{clamp_u16, ALBEDO_MAX, FRESHWATER_FLUX_MAX, SOIL_MAX, WATER_MAX};
use crate::rng::Stream;
use crate::world::{Hazards, Region, World};

/// Parsed seed definition describing the deterministic initial world.
#[derive(Clone, Debug, Deserialize)]
pub struct Seed {
    pub name: String,
    pub width: u32,
    pub height: u32,
    #[serde(rename = "elevation_noise")]
    pub noise: Noise,
    #[serde(rename = "humidity_bias")]
    pub humidity: Humidity,
}

/// Multi-octave pseudo-noise configuration for elevation sampling.
#[derive(Clone, Debug, Deserialize)]
pub struct Noise {
    pub octaves: u8,
    pub freq: f64,
    pub amp: f64,
    pub seed: u64,
}

/// Deterministic humidity bias per latitude band.
#[derive(Clone, Debug, Deserialize)]
pub struct Humidity {
    pub equator: f64,
    pub poles: f64,
}

impl Seed {
    /// Load a seed JSON document from disk.
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let file =
            File::open(path).with_context(|| format!("failed to open seed file {:?}", path))?;
        Self::from_reader(BufReader::new(file))
    }

    /// Deserialize a seed document from an arbitrary reader.
    pub fn from_reader<R: Read>(reader: R) -> Result<Self> {
        Ok(serde_json::from_reader(reader).context("invalid seed json")?)
    }
}

/// Realise a [`World`] from the given seed description.
pub fn build_world(seed: &Seed, world_seed_override: Option<u64>) -> World {
    let world_seed = world_seed_override.unwrap_or(seed.noise.seed);
    let mut regions = Vec::with_capacity((seed.width * seed.height) as usize);
    let mut id: u32 = 0;
    for y in 0..seed.height {
        for x in 0..seed.width {
            let latitude = latitude_from_grid(y, seed.height);
            let elevation = sample_elevation(world_seed, &seed.noise, x, y);
            let (water, soil) =
                initial_resources(world_seed, &seed.humidity, latitude, elevation, x, y);
            let polar_factor = (latitude.abs() / 90.0).clamp(0.0, 1.0);
            let mut cryosphere_rng = Stream::from(world_seed, "seed:cryosphere", u64::from(id));
            let albedo_noise = cryosphere_rng.next_signed_unit() * 25.0;
            let albedo = clamp_u16(
                (300.0 + 500.0 * polar_factor + albedo_noise).round() as i32,
                0,
                ALBEDO_MAX,
            );
            let freshwater_flux = clamp_u16(0, 0, FRESHWATER_FLUX_MAX);

            regions.push(Region {
                id,
                x,
                y,
                elevation_m: elevation,
                latitude_deg: latitude,
                biome: 0,
                water,
                soil,
                temperature_tenths_c: 0,
                precipitation_mm: 0,
                albedo_milli: albedo,
                freshwater_flux_tenths_mm: freshwater_flux,
                hazards: Hazards::default(),
            });
            id += 1;
        }
    }

    World::new(world_seed, seed.width, seed.height, regions)
}

fn latitude_from_grid(y: u32, height: u32) -> f64 {
    let ratio = (f64::from(y) + 0.5) / f64::from(height);
    90.0 - ratio * 180.0
}

fn sample_elevation(seed: u64, noise: &Noise, x: u32, y: u32) -> i32 {
    let mut octave = 0;
    let mut amplitude = noise.amp;
    let mut total = 0.0;
    while octave < noise.octaves {
        let context = ((x as u64) << 32) ^ ((y as u64) << 16) ^ u64::from(octave);
        let mut rng = Stream::from(seed ^ noise.seed, "seed:elevation", context);
        let sample = rng.next_signed_unit();
        total += sample * amplitude * 500.0;
        amplitude *= 0.5;
        octave += 1;
    }
    (total + 500.0).clamp(0.0, 3_000.0).round() as i32
}

fn initial_resources(
    seed: u64,
    humidity: &Humidity,
    latitude_deg: f64,
    elevation_m: i32,
    x: u32,
    y: u32,
) -> (u16, u16) {
    let context = ((x as u64) << 32) ^ ((y as u64) << 16);
    let mut water_rng = Stream::from(seed, "seed:resources:water", context);
    let mut soil_rng = Stream::from(seed, "seed:resources:soil", context);
    let latitude_ratio = (latitude_deg.abs() / 90.0).clamp(0.0, 1.0);
    let bias = humidity.equator + (humidity.poles - humidity.equator) * latitude_ratio;
    let base = (0.55 + bias).clamp(0.05, 0.95);
    let elevation_penalty = (f64::from(elevation_m) / 3_000.0).clamp(0.0, 1.0) * 0.3;
    let noise = water_rng.next_signed_unit() * 0.05;
    let water = clamp_u16(
        ((base - elevation_penalty + noise) * 10_000.0).round() as i32,
        0,
        WATER_MAX,
    );
    let soil_base = (base - 0.1).clamp(0.05, 0.9);
    let soil_noise = soil_rng.next_signed_unit() * 0.04;
    let soil = clamp_u16(
        ((soil_base - elevation_penalty * 0.5 + soil_noise) * 10_000.0).round() as i32,
        0,
        SOIL_MAX,
    );
    (water, soil)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn repository_seeds_deserialize() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let seeds_dir = manifest_dir.join("../../testdata/seeds");
        for name in ["seed_wet_equator.json", "seed_shard_continents.json"] {
            let path = seeds_dir.join(name);
            let seed = Seed::load_from_path(&path)
                .unwrap_or_else(|err| panic!("failed to load {:?}: {}", path, err));
            assert!(seed.width > 0, "seed {:?} must define width", path);
            assert!(seed.height > 0, "seed {:?} must define height", path);
        }
    }
}
