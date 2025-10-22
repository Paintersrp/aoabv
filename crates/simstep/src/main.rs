use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use sim_core::io::seed::SeedDocument;
use sim_core::Simulation;

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

    /// Optional path to write cause-code NDJSON alongside frames.
    #[arg(long)]
    cause_log: Option<PathBuf>,

    /// Optional override for the world seed.
    #[arg(long)]
    world_seed: Option<u64>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let seed_doc = SeedDocument::load_from_path(&args.seed)
        .with_context(|| format!("failed to read seed {:?}", args.seed))?;
    let mut simulation = Simulation::from_seed_document(seed_doc, args.world_seed)?;

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
        let outputs = simulation.tick();
        let line = outputs.frame.to_ndjson()?;
        writer.write_all(line.as_bytes())?;
        if let Some(writer) = cause_writer.as_mut() {
            for cause in outputs.causes {
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
