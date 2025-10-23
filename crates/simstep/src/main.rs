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
    #[arg(long = "seed-file", value_name = "PATH")]
    seed_file: PathBuf,

    /// Override the world seed used when building the initial world state.
    #[arg(long, value_name = "NUMBER", conflicts_with = "world_seed")]
    seed: Option<u64>,

    /// Backwards-compatible alias for `--seed`.
    #[arg(long = "world-seed", value_name = "NUMBER", conflicts_with = "seed")]
    world_seed: Option<u64>,

    /// Number of ticks to execute.
    #[arg(long)]
    ticks: u64,

    /// Output NDJSON file path.
    #[arg(long)]
    out: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let seed = Seed::load_from_path(&args.seed_file)
        .with_context(|| format!("failed to read seed {:?}", args.seed_file))?;
    let mut world = build_world(&seed, args.seed.or(args.world_seed));

    let file =
        File::create(&args.out).with_context(|| format!("failed to create {:?}", args.out))?;
    let mut writer = BufWriter::new(file);

    for _ in 0..args.ticks {
        let next_tick = world.tick + 1;
        let seed = world.seed;
        let (diff, chronicle) = tick_once(&mut world, seed, next_tick)?;
        let highlights = collect_highlights(&world, &diff);

        let width = world.width;
        let height = world.height;
        let frame = make_frame(next_tick, diff, highlights, chronicle, false, width, height);
        let line = frame.to_ndjson()?;
        writer.write_all(line.as_bytes())?;
    }

    writer.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Args;
    use clap::{error::ErrorKind, Parser};

    #[test]
    fn requires_seed_file() {
        let err =
            Args::try_parse_from(["simstep", "--ticks", "8", "--out", "out.ndjson"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn rejects_both_seed_flags() {
        let err = Args::try_parse_from([
            "simstep",
            "--seed-file",
            "seed.json",
            "--ticks",
            "1",
            "--out",
            "out.ndjson",
            "--seed",
            "1",
            "--world-seed",
            "2",
        ])
        .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }
}
