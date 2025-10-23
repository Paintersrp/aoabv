use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use clap::Parser;
use sim_core::cause::Entry;
use sim_core::io::frame::make_frame;
use sim_core::io::seed::{build_world, Humidity, Noise, Seed};
use sim_core::{collect_highlights, tick_once};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "simd", about = "Ages of a Borrowed Voice streaming daemon")]
struct Args {
    /// JSON seed document describing the initial world configuration.
    #[arg(long = "seed-file", value_name = "PATH")]
    seed_file: Option<PathBuf>,

    /// Override or supply the world seed for deterministic generation.
    #[arg(long, value_name = "NUMBER", conflicts_with = "world_seed")]
    seed: Option<u64>,

    /// Backwards-compatible alias for `--seed`.
    #[arg(long = "world-seed", value_name = "NUMBER", conflicts_with = "seed")]
    world_seed: Option<u64>,

    /// Procedural world width when not using a JSON seed file.
    #[arg(long, requires_all = ["height", "seed"], conflicts_with = "seed_file")]
    width: Option<u32>,

    /// Procedural world height when not using a JSON seed file.
    #[arg(long, requires_all = ["width", "seed"], conflicts_with = "seed_file")]
    height: Option<u32>,

    /// Target frames per second for ticking the simulation.
    #[arg(long, default_value_t = 4u32, value_parser = clap::value_parser!(u32).range(1..=60))]
    fps: u32,

    /// Address to bind (defaults to 127.0.0.1).
    #[arg(long, default_value = "127.0.0.1")]
    bind: String,

    /// Port to listen on for WebSocket clients.
    #[arg(long, default_value_t = 8787)]
    port: u16,
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<String>,
}

fn load_seed(args: &Args) -> Result<Seed> {
    if let Some(path) = &args.seed_file {
        return Seed::load_from_path(path)
            .with_context(|| format!("failed to load seed from {:?}", path));
    }

    let width = args
        .width
        .context("--width is required when --seed-file is absent")?;
    let height = args
        .height
        .context("--height is required when --seed-file is absent")?;
    let seed = args
        .seed
        .or(args.world_seed)
        .context("--seed is required when procedurally generating a world")?;

    // TODO(agents): rationale - use fixed procedural defaults when no seed file is supplied.
    Ok(Seed {
        name: format!("cli-{}", seed),
        width,
        height,
        noise: Noise {
            octaves: 3,
            freq: 0.02,
            amp: 1.0,
            seed,
        },
        humidity: Humidity {
            equator: 0.3,
            poles: -0.2,
        },
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let args = Args::parse();

    let seed = load_seed(&args)?;
    let frame_period = Duration::from_secs_f64(1.0 / f64::from(args.fps));
    let world_seed_override = args.seed.or(args.world_seed);
    let world = build_world(&seed, world_seed_override);

    let (tx, _rx) = broadcast::channel::<String>(128);
    let state = AppState { tx: tx.clone() };
    let world_handle = Arc::new(Mutex::new(world));

    // Spawn ticking task.
    let tick_tx = tx.clone();
    let tick_handle = Arc::clone(&world_handle);
    tokio::spawn(async move {
        loop {
            let tick_result: Result<(String, Vec<Entry>, u64), anyhow::Error> = {
                let mut world = tick_handle.lock().await;
                let next_tick = world.tick + 1;
                let seed = world.seed;

                match tick_once(&mut world, seed, next_tick) {
                    Ok((diff, chronicle)) => {
                        let highlights = collect_highlights(&world, &diff);
                        let causes = diff.causes.clone();
                        let width = world.width;
                        let height = world.height;
                        let frame = make_frame(
                            next_tick, diff, highlights, chronicle, false, width, height,
                        );
                        match frame.to_ndjson() {
                            Ok(line) => Ok((line, causes, next_tick)),
                            Err(err) => Err(err.into()),
                        }
                    }
                    Err(err) => Err(err),
                }
            };

            let (line, causes, t) = match tick_result {
                Ok(result) => result,
                Err(err) => {
                    error!(?err, "tick failed");
                    break;
                }
            };

            if tick_tx.send(line).is_err() {
                tracing::trace!("no subscribers for frame t={}", t);
            }
            for cause in causes {
                info!(target = "cause", %cause.code, %cause.target, note = ?cause.note);
            }

            sleep(frame_period).await;
        }
    });

    let app = Router::new()
        .route("/stream", get(ws_handler))
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", args.bind, args.port)
        .parse()
        .with_context(|| format!("invalid bind address {}:{}", args.bind, args.port))?;

    info!(%addr, "starting simd");
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind {}", addr))?;
    axum::serve(listener, app.into_make_service())
        .await
        .context("server error")?;
    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move { handle_socket(socket, state.tx.subscribe()).await })
}

async fn handle_socket(mut socket: WebSocket, mut rx: broadcast::Receiver<String>) {
    while let Ok(line) = rx.recv().await {
        if socket.send(Message::Text(line.clone())).await.is_err() {
            error!("websocket client disconnected");
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Args;
    use clap::{error::ErrorKind, Parser};

    #[test]
    fn rejects_conflicting_seed_aliases() {
        let err = Args::try_parse_from([
            "simd",
            "--seed-file",
            "seed.json",
            "--seed",
            "1",
            "--world-seed",
            "2",
        ])
        .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn rejects_procedural_without_height() {
        let err = Args::try_parse_from(["simd", "--seed", "4", "--width", "64"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn rejects_width_with_seed_file() {
        let err = Args::try_parse_from(["simd", "--seed-file", "seed.json", "--width", "64"])
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }
}
