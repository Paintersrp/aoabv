use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::json;
use sim_core::io::frame::make_frame;
use sim_core::io::seed::{build_world, Seed};
use sim_core::tick_once;
use sim_core::world::World;

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

    /// Optional path to emit per-tick global metrics as NDJSON.
    #[arg(long = "emit-metrics", value_name = "PATH")]
    emit_metrics: Option<PathBuf>,
}

struct GlobalMeans {
    temp_c: f64,
    albedo: f64,
    humidity_pct: f64,
    precip_native: f64,
}

fn compute_global_means(
    world: &World,
    humidity_cache: &[i32],
    region_order: &[usize],
) -> GlobalMeans {
    if region_order.is_empty() {
        return GlobalMeans {
            temp_c: 0.0,
            albedo: 0.0,
            humidity_pct: 0.0,
            precip_native: 0.0,
        };
    }

    let mut temp_sum: i128 = 0;
    let mut albedo_sum: i128 = 0;
    let mut humidity_sum: i128 = 0;
    let mut precip_sum: i128 = 0;

    for &index in region_order {
        if let Some(region) = world.regions.get(index) {
            temp_sum += i128::from(region.temperature_tenths_c);
            albedo_sum += i128::from(region.albedo_milli);
            precip_sum += i128::from(region.precipitation_mm);
            let humidity_value = humidity_cache.get(index).copied().unwrap_or(0);
            humidity_sum += i128::from(humidity_value);
        }
    }

    let count = region_order.len() as f64;
    // TODO(agents): Equal-weight means avoid grid geometry assumptions for v0.2.
    GlobalMeans {
        temp_c: temp_sum as f64 / (count * 10.0),
        albedo: albedo_sum as f64 / (count * 1_000.0),
        humidity_pct: humidity_sum as f64 / (count * 10.0),
        precip_native: precip_sum as f64 / count,
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let seed = Seed::load_from_path(&args.seed_file)
        .with_context(|| format!("failed to read seed {:?}", args.seed_file))?;
    let mut world = build_world(&seed, args.seed.or(args.world_seed));

    let frame_file =
        File::create(&args.out).with_context(|| format!("failed to create {:?}", args.out))?;
    let mut frame_writer = BufWriter::new(frame_file);

    let mut metrics_writer = if let Some(path) = &args.emit_metrics {
        let file = File::create(path)
            .with_context(|| format!("failed to create metrics file at {:?}", path))?;
        Some(BufWriter::new(file))
    } else {
        None
    };

    let mut humidity_cache = vec![0i32; world.regions.len()];
    let mut region_order: Vec<usize> = (0..world.regions.len()).collect();
    region_order.sort_by_key(|&idx| world.regions[idx].id);

    for _ in 0..args.ticks {
        let next_tick = world.tick + 1;
        let seed = world.seed;
        let (diff, chronicle, highlights) = tick_once(&mut world, seed, next_tick)?;

        if let Some(writer) = metrics_writer.as_mut() {
            for value in &diff.humidity {
                let index = value.region as usize;
                if let Some(slot) = humidity_cache.get_mut(index) {
                    *slot = value.value;
                }
            }

            let means = compute_global_means(&world, &humidity_cache, &region_order);
            let diag_energy = diff.diagnostics.get("energy_balance").copied().unwrap_or(0);
            let metrics_line = json!({
                "t": next_tick,
                "global": {
                    "temp_c": means.temp_c,
                    "albedo": means.albedo,
                    "humidity_pct": means.humidity_pct,
                    "precip_native": means.precip_native,
                    "diag_energy_tenths": diag_energy as f64,
                }
            });
            let serialized = serde_json::to_string(&metrics_line)?;
            writer.write_all(serialized.as_bytes())?;
            writer.write_all(b"\n")?;
        }

        let width = world.width;
        let height = world.height;
        let frame = make_frame(next_tick, diff, highlights, chronicle, false, width, height);
        let line = frame.to_ndjson()?;
        frame_writer.write_all(line.as_bytes())?;
    }

    frame_writer.flush()?;
    if let Some(writer) = metrics_writer.as_mut() {
        writer.flush()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{tick_once, Args};
    use clap::{error::ErrorKind, Parser};
    use sim_core::io::frame::make_frame;
    use sim_core::io::seed::{build_world, Seed};

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

    #[test]
    fn paired_runs_are_deterministic_over_200_ticks() {
        let seed_json = r#"{
            "name": "determinism",
            "width": 4,
            "height": 2,
            "elevation_noise": {"octaves": 1, "freq": 0.1, "amp": 1.0, "seed": 3},
            "humidity_bias": {"equator": 0.2, "poles": -0.2}
        }"#;
        let seed: Seed = serde_json::from_str(seed_json).expect("seed parses");

        let run_once = || {
            let mut world = build_world(&seed, Some(1_234_567));
            let mut lines = Vec::new();
            for _ in 0..200 {
                let next_tick = world.tick + 1;
                let seed_value = world.seed;
                let (diff, chronicle, highlights) =
                    tick_once(&mut world, seed_value, next_tick).expect("tick succeeds");
                let frame = make_frame(
                    next_tick,
                    diff,
                    highlights,
                    chronicle,
                    false,
                    world.width,
                    world.height,
                );
                lines.push(frame.to_ndjson().expect("frame serializes"));
            }
            lines
        };

        let first = run_once();
        let second = run_once();
        assert_eq!(first, second);
    }
}
