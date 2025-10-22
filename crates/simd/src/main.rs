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
use sim_core::io::seed::SeedDocument;
use sim_core::Simulation;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "simd", about = "Ages of a Borrowed Voice streaming daemon")]
struct Args {
    /// Path to the seed JSON document.
    #[arg(long)]
    seed: PathBuf,

    /// Optional override for the world seed.
    #[arg(long)]
    world_seed: Option<u64>,

    /// Address to bind (defaults to 127.0.0.1).
    #[arg(long, default_value = "127.0.0.1")]
    bind: String,

    /// Port to listen on for WebSocket clients.
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Milliseconds to sleep between ticks.
    #[arg(long, default_value_t = 250u64)]
    tick_ms: u64,
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let args = Args::parse();

    let seed_doc = SeedDocument::load_from_path(&args.seed)
        .with_context(|| format!("failed to load seed from {:?}", args.seed))?;
    let mut simulation = Simulation::from_seed_document(seed_doc, args.world_seed)?;

    let (tx, _rx) = broadcast::channel::<String>(128);
    let state = AppState { tx: tx.clone() };
    let sim_handle = Arc::new(Mutex::new(simulation));

    // Spawn ticking task.
    let tick_tx = tx.clone();
    let tick_handle = Arc::clone(&sim_handle);
    tokio::spawn(async move {
        loop {
            let mut guard = tick_handle.lock().await;
            let outputs = guard.tick();
            drop(guard);

            if let Ok(line) = outputs.frame.to_ndjson() {
                if tick_tx.send(line).is_err() {
                    tracing::trace!("no subscribers for frame t={}", outputs.frame.t);
                }
            }
            for cause in outputs.causes {
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
