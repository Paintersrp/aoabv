use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::fixed::clamp_resource;
use crate::rng::SplitMix64;
use crate::world::{HazardLevels, Region, World};

#[derive(Clone, Debug, Deserialize)]
pub struct SeedDocument {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub elevation_noise: ElevationNoise,
    pub humidity_bias: HumidityBias,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ElevationNoise {
    pub octaves: u8,
    pub freq: f64,
    pub amp: f64,
    pub seed: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HumidityBias {
    pub equator: f64,
    pub poles: f64,
}

pub struct SeedRealization {
    pub world: World,
}

impl SeedDocument {
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let file =
            File::open(path).with_context(|| format!("failed to open seed file {:?}", path))?;
        Self::from_reader(BufReader::new(file))
    }

    pub fn from_reader<R: Read>(reader: R) -> Result<Self> {
        Ok(serde_json::from_reader(reader).context("invalid seed json")?)
    }

    pub fn realize(&self, world_seed_override: Option<u64>) -> Result<SeedRealization> {
        let world_seed = world_seed_override.unwrap_or(self.elevation_noise.seed);
        let mut regions = Vec::with_capacity((self.width * self.height) as usize);
        let mut id: u32 = 0;
        for y in 0..self.height {
            for x in 0..self.width {
                let latitude = latitude_from_grid(y, self.height);
                let elevation = sample_elevation(world_seed, &self.elevation_noise, x, y);
                let (water, soil) =
                    initial_resources(world_seed, &self.humidity_bias, latitude, elevation, x, y);
                regions.push(Region {
                    id,
                    x,
                    y,
                    elevation_m: elevation,
                    latitude_deg: latitude,
                    biome: 0,
                    water,
                    soil,
                    hazards: HazardLevels::default(),
                });
                id += 1;
            }
        }

        Ok(SeedRealization {
            world: World::new(world_seed, self.width, self.height, regions),
        })
    }
}

fn latitude_from_grid(y: u32, height: u32) -> f64 {
    let ratio = (f64::from(y) + 0.5) / f64::from(height);
    90.0 - ratio * 180.0
}

fn sample_elevation(seed: u64, noise: &ElevationNoise, x: u32, y: u32) -> f64 {
    let mut octave = 0;
    let mut amplitude = noise.amp;
    let mut total = 0.0;
    while octave < noise.octaves {
        let mut rng = SplitMix64::new(
            seed ^ noise.seed ^ ((x as u64) << 16) ^ ((y as u64) << 32) ^ octave as u64,
        );
        let sample = rng.next_signed_unit();
        total += sample * amplitude * 500.0;
        amplitude *= 0.5;
        octave += 1;
    }
    (total + 500.0).clamp(0.0, 3000.0)
}

fn initial_resources(
    seed: u64,
    humidity: &HumidityBias,
    latitude_deg: f64,
    elevation_m: f64,
    x: u32,
    y: u32,
) -> (u16, u16) {
    let mut rng = SplitMix64::new(seed ^ ((x as u64) << 12) ^ ((y as u64) << 20));
    let latitude_ratio = (latitude_deg.abs() / 90.0).clamp(0.0, 1.0);
    let bias = humidity.equator + (humidity.poles - humidity.equator) * latitude_ratio;
    let base = (0.55 + bias).clamp(0.05, 0.95);
    let elevation_penalty = (elevation_m / 3_000.0).clamp(0.0, 1.0) * 0.3;
    let noise = rng.next_signed_unit() * 0.05;
    let water = clamp_resource(((base - elevation_penalty + noise) * 10_000.0) as i32);
    let soil_base = (base - 0.1).clamp(0.05, 0.9);
    let soil_noise = rng.next_signed_unit() * 0.04;
    let soil =
        clamp_resource(((soil_base - elevation_penalty * 0.5 + soil_noise) * 10_000.0) as i32);
    (water, soil)
}
