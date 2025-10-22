use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use sim_core::io::frame::make_frame;
use sim_core::io::seed::{build_world, Humidity, Noise, Seed};
use sim_core::Simulation;

#[derive(Parser, Debug)]
#[command(
    name = "simstep",
    about = "Batch runner for deterministic NDJSON frames"
)]
struct Args {
    /// Path to the seed JSON document.
    #[arg(long = "seed")]
    seed_path: Option<PathBuf>,

    /// Optional seed name override when constructing from CLI parameters.
    #[arg(long)]
    seed_name: Option<String>,

    /// World width when constructing from CLI parameters.
    #[arg(long)]
    width: Option<u32>,

    /// World height when constructing from CLI parameters.
    #[arg(long)]
    height: Option<u32>,

    /// Elevation noise octaves when constructing from CLI parameters.
    #[arg(long = "noise-octaves")]
    noise_octaves: Option<u8>,

    /// Elevation noise frequency when constructing from CLI parameters.
    #[arg(long = "noise-freq")]
    noise_freq: Option<f64>,

    /// Elevation noise amplitude when constructing from CLI parameters.
    #[arg(long = "noise-amp")]
    noise_amp: Option<f64>,

    /// Elevation noise RNG seed when constructing from CLI parameters.
    #[arg(long = "noise-seed")]
    noise_seed: Option<u64>,

    /// Equatorial humidity bias when constructing from CLI parameters.
    #[arg(long = "humidity-equator")]
    humidity_equator: Option<f64>,

    /// Polar humidity bias when constructing from CLI parameters.
    #[arg(long = "humidity-poles")]
    humidity_poles: Option<f64>,

    /// Number of ticks to execute.
    #[arg(long)]
    ticks: u64,

    /// Output NDJSON file path.
    #[arg(long)]
    out: PathBuf,

    /// Optional path to write cause-code NDJSON alongside frames.
    #[arg(long)]
    cause_log: Option<PathBuf>,

    /// Optional override for the world seed.
    #[arg(long)]
    world_seed: Option<u64>,
}

fn load_seed(args: &Args) -> Result<Seed> {
    if let Some(path) = &args.seed_path {
        return Seed::load_from_path(path)
            .with_context(|| format!("failed to read seed {:?}", path));
    }

    let width = args
        .width
        .context("--width is required when --seed is not provided")?;
    let height = args
        .height
        .context("--height is required when --seed is not provided")?;
    let noise_octaves = args
        .noise_octaves
        .context("--noise-octaves is required when --seed is not provided")?;
    let noise_freq = args
        .noise_freq
        .context("--noise-freq is required when --seed is not provided")?;
    let noise_amp = args
        .noise_amp
        .context("--noise-amp is required when --seed is not provided")?;
    let noise_seed = args
        .noise_seed
        .context("--noise-seed is required when --seed is not provided")?;
    let humidity_equator = args
        .humidity_equator
        .context("--humidity-equator is required when --seed is not provided")?;
    let humidity_poles = args
        .humidity_poles
        .context("--humidity-poles is required when --seed is not provided")?;

    Ok(Seed {
        name: args.seed_name.clone().unwrap_or_else(|| "cli".to_string()),
        width,
        height,
        noise: Noise {
            octaves: noise_octaves,
            freq: noise_freq,
            amp: noise_amp,
            seed: noise_seed,
        },
        humidity: Humidity {
            equator: humidity_equator,
            poles: humidity_poles,
        },
    })
}

fn main() -> Result<()> {
    let args = Args::parse();

    let seed = load_seed(&args)?;
    let world = build_world(&seed, args.world_seed);
    let mut simulation = Simulation::from_world(world);

    let file =
        File::create(&args.out).with_context(|| format!("failed to create {:?}", args.out))?;
    let mut writer = BufWriter::new(file);

    let mut cause_writer = if let Some(path) = &args.cause_log {
        let file =
            File::create(path).with_context(|| format!("failed to create cause log {:?}", path))?;
        Some(BufWriter::new(file))
    } else {
        None
    };

    for _ in 0..args.ticks {
        let outputs = simulation.tick()?;
        let sim_core::TickOutputs {
            t,
            diff,
            highlights,
            chronicle,
            era_end,
            causes,
        } = outputs;

        let frame = make_frame(t, diff, highlights, chronicle, era_end);
        let line = frame.to_ndjson()?;
        writer.write_all(line.as_bytes())?;
        if let Some(writer) = cause_writer.as_mut() {
            for cause in causes {
                let json = serde_json::to_string(&cause)?;
                writer.write_all(json.as_bytes())?;
                writer.write_all(b"\n")?;
            }
        }
    }

    writer.flush()?;
    if let Some(writer) = cause_writer.as_mut() {
        writer.flush()?;
    }

    Ok(())
}
