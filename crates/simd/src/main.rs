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

    /// Optional override for the world seed.
    #[arg(long)]
    world_seed: Option<u64>,

    /// Address to bind (defaults to 127.0.0.1).
    #[arg(long, default_value = "127.0.0.1")]
    bind: String,

    /// Port to listen on for WebSocket clients.
    #[arg(long, default_value_t = 8787)]
    port: u16,

    /// Milliseconds to sleep between ticks.
    #[arg(long, default_value_t = 250u64)]
    tick_ms: u64,
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<String>,
}

fn load_seed(args: &Args) -> Result<Seed> {
    if let Some(path) = &args.seed_path {
        return Seed::load_from_path(path)
            .with_context(|| format!("failed to load seed from {:?}", path));
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

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let args = Args::parse();

    let seed = load_seed(&args)?;
    let world = build_world(&seed, args.world_seed);

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
                        let frame = make_frame(next_tick, diff, highlights, chronicle, false);
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

            sleep(Duration::from_millis(args.tick_ms)).await;
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
