//! Local-First Native Astrodynamics Daemon & Deep Space Mission Planning API.
//!
//! Binary entry point for launching the local Axum server.

#![deny(clippy::print_stdout, clippy::print_stderr)]

use sbm_server::create_app;
use std::{net::SocketAddr, path::PathBuf};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for clean logs
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sbm_server=info,tower_http=info".into()),
        )
        .init();

    let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    info!("Starting SBM Astrodynamics Local Daemon at: {:?}", workspace_root);

    let app = create_app(workspace_root);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    info!("============================================================");
    info!("   Deep-Space Mission Planning Cockpit Local Server Running  ");
    info!("   Web Interface: http://127.0.0.1:{}", port);
    info!("   Health Check:  http://127.0.0.1:{}/api/v1/health", port);
    info!("============================================================");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
