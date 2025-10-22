use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use sim_core::io::frame::make_frame;
use sim_core::io::seed::{build_world, Seed};
use sim_core::{collect_highlights, tick_once};

#[derive(Parser, Debug)]
#[command(
    name = "simstep",
    about = "Batch runner for deterministic NDJSON frames"
)]
struct Args {
    /// Path to the seed JSON document.
    #[arg(long)]
    seed: PathBuf,

    /// Number of ticks to execute.
    #[arg(long)]
    ticks: u64,

    /// Output NDJSON file path.
    #[arg(long)]
    out: PathBuf,

    /// Optional override for the world seed.
    #[arg(long)]
    world_seed: Option<u64>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let seed = Seed::load_from_path(&args.seed)
        .with_context(|| format!("failed to read seed {:?}", args.seed))?;
    let mut world = build_world(&seed, args.world_seed);

    let file =
        File::create(&args.out).with_context(|| format!("failed to create {:?}", args.out))?;
    let mut writer = BufWriter::new(file);

    for _ in 0..args.ticks {
        let next_tick = world.tick + 1;
        let seed = world.seed;
        let (diff, chronicle) = tick_once(&mut world, seed, next_tick)?;
        let highlights = collect_highlights(&world, &diff);

        let frame = make_frame(next_tick, diff, highlights, chronicle, false);
        let line = frame.to_ndjson()?;
        writer.write_all(line.as_bytes())?;
    }

    writer.flush()?;

    Ok(())
}
