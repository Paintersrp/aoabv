use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::{routing::get, Router};
use tokio::net::TcpListener;

async fn healthcheck() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr: SocketAddr = "0.0.0.0:3000".parse().expect("valid socket address");
    let app = Router::new().route("/health", get(healthcheck));

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind TCP listener on {addr}"))?;

    axum::serve(listener, app.into_make_service())
        .await
        .context("failed while serving axum application")?;

    Ok(())
}
