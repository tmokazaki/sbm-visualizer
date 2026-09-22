//! HTTP router configuration and static visualizer serving.

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use std::path::{Path, PathBuf};
use tower_http::cors::{Any, CorsLayer};
use tracing::error;

use crate::handlers::{
    correct_halo_handler, correct_lyapunov_handler, export_oem_handler,
    generate_manifold_handler, get_system_info, get_transfer_benchmark, health_check,
    optimize_transfer_handler, plan_rpo_handler,
};

/// Shared server application state.
#[derive(Clone, Debug)]
pub struct AppState {
    pub workspace_root: PathBuf,
}

/// Builds and configures the Axum application router with all routes and CORS.
pub fn create_app(workspace_root: PathBuf) -> Router {
    let state = AppState { workspace_root };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Web Cockpit UI routes
        .route("/", get(serve_visualizer))
        .route("/cr3bp_deep_space_visualizer.html", get(serve_visualizer))
        .route("/index.html", get(serve_sbm_index))
        .route("/rpo", get(serve_rpo_visualizer))
        .route("/rpo_visualizer.html", get(serve_rpo_visualizer))
        // API routes
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/system/:name", get(get_system_info))
        .route("/api/v1/orbit/correct/lyapunov", post(correct_lyapunov_handler))
        .route("/api/v1/orbit/correct/halo", post(correct_halo_handler))
        .route("/api/v1/orbit/manifold", post(generate_manifold_handler))
        .route("/api/v1/transfer/benchmark", get(get_transfer_benchmark))
        .route("/api/v1/transfer/optimize", post(optimize_transfer_handler))
        .route("/api/v1/export/oem", post(export_oem_handler))
        .route("/api/v1/rpo/plan", post(plan_rpo_handler))
        .layer(cors)
        .with_state(state)
}

/// Resolves HTML file path looking at root, one level up, and two levels up.
pub fn resolve_html_path(root: &Path, filename: &str) -> PathBuf {
    let direct = root.join(filename);
    if direct.exists() {
        return direct;
    }
    // Check two levels up (if executed inside crates/sbm_server)
    let up2 = root.join("../../").join(filename);
    if up2.exists() {
        return up2;
    }
    // Check one level up
    let up1 = root.join("../").join(filename);
    if up1.exists() {
        return up1;
    }
    direct
}

pub async fn serve_visualizer(State(state): State<AppState>) -> Response {
    let file_path = resolve_html_path(&state.workspace_root, "cr3bp_deep_space_visualizer.html");
    match tokio::fs::read_to_string(&file_path).await {
        Ok(html_content) => Html(html_content).into_response(),
        Err(err) => {
            error!("Failed to read cr3bp_deep_space_visualizer.html: {:?}", err);
            (
                StatusCode::NOT_FOUND,
                format!("Visualizer HTML not found at: {:?}", file_path),
            )
                .into_response()
        }
    }
}

pub async fn serve_sbm_index(State(state): State<AppState>) -> Response {
    let file_path = resolve_html_path(&state.workspace_root, "index.html");
    match tokio::fs::read_to_string(&file_path).await {
        Ok(html_content) => Html(html_content).into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            format!("Index HTML not found: {:?}", err),
        )
            .into_response(),
    }
}

pub async fn serve_rpo_visualizer(State(state): State<AppState>) -> Response {
    let file_path = resolve_html_path(&state.workspace_root, "rpo_visualizer.html");
    match tokio::fs::read_to_string(&file_path).await {
        Ok(html_content) => Html(html_content).into_response(),
        Err(err) => {
            error!("Failed to read rpo_visualizer.html: {:?}", err);
            (
                StatusCode::NOT_FOUND,
                format!("RPO Visualizer HTML not found at: {:?}", file_path),
            )
                .into_response()
        }
    }
}
