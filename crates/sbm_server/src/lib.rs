//! Local-First Native Astrodynamics Daemon & Deep Space Mission Planning API.
//!
//! Provides high-performance local endpoints and serves the 3D Interactive Cockpit UI.

#![deny(clippy::print_stdout, clippy::print_stderr)]

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use sbm_core::cr3bp::{
    calculate_tli_impulsive_dv, compute_earth_moon_l1_to_l2_transfer, compute_lagrange_points,
    correct_3d_halo, correct_planar_lyapunov, generate_manifold_arc, CorrectedOrbit, Cr3bpState,
    Cr3bpSystem, DormandPrinceIntegrator, IntegratorOptions, ManifoldBranch, ManifoldOptions,
    ManifoldType,
};

use sbm_core::scvx::{Cr3bpTransferMissionConfig, Cr3bpTransferOptimizer};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tower_http::cors::{Any, CorsLayer};
use tracing::error;

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
        // Web Cockpit UI route
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
        .layer(cors)
        .with_state(state)
}

// ---------------------------------------------------------------------------
// HTML UI Handlers
// ---------------------------------------------------------------------------

fn resolve_html_path(root: &std::path::Path, filename: &str) -> PathBuf {
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

// ---------------------------------------------------------------------------
// API Data Transfer Objects (DTOs)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub engine: &'static str,
    pub capabilities: Vec<&'static str>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemInfoResponse {
    pub name: String,
    pub mu: f64,
    pub l_star_km: f64,
    pub t_star_days: f64,
    pub v_star_ms: f64,
    pub lagrange_points: Vec<LagrangePointDto>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LagrangePointDto {
    pub point: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub jacobi_constant: f64,
    pub distance_to_primary_km: f64,
    pub distance_to_secondary_km: f64,
}

#[derive(Deserialize, Debug)]
pub struct CorrectLyapunovRequest {
    pub system: Option<String>,
    pub x0: f64,
    pub vy0_guess: f64,
    pub max_iter: Option<usize>,
    pub tol: Option<f64>,
}

#[derive(Deserialize, Debug)]
pub struct CorrectHaloRequest {
    pub system: Option<String>,
    pub z0: f64,
    pub x0_guess: f64,
    pub vy0_guess: f64,
    pub max_iter: Option<usize>,
    pub tol: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OrbitResponse {
    pub success: bool,
    pub initial_state: [f64; 6],
    pub period_nondim: f64,
    pub period_days: f64,
    pub jacobi_constant: f64,
    pub stability_index: f64,
    pub lambda_unstable: f64,
    pub lambda_stable: f64,
    pub eigenvector_unstable: [f64; 6],
    pub eigenvector_stable: [f64; 6],
    pub iterations: usize,
    pub residual: f64,
    pub trajectory_points: Vec<[f64; 4]>, // [t, x, y, z]
}

#[derive(Deserialize, Debug)]
pub struct ManifoldRequest {
    pub system: Option<String>,
    pub initial_state: [f64; 6],
    pub period_nondim: f64,
    pub eigenvector_unstable: [f64; 6],
    pub eigenvector_stable: [f64; 6],
    pub manifold_type: String, // "unstable" or "stable"
    pub branch: String,        // "positive" or "negative"
    pub orbit_phase: f64,      // 0.0 to 1.0
    pub epsilon_dist: Option<f64>,
    pub t_span: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ManifoldResponse {
    pub success: bool,
    pub manifold_type: String,
    pub branch: String,
    pub orbit_phase: f64,
    pub flight_time_days: f64,
    pub jacobi_constant: f64,
    pub final_state: [f64; 6],
    pub trajectory_points: Vec<[f64; 4]>, // [t, x, y, z]
}

// ---------------------------------------------------------------------------
// REST API Handlers
// ---------------------------------------------------------------------------

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: "0.2.0",
        engine: "CR3BP High-Precision Astrodynamics Engine (AAS 20-459)",
        capabilities: vec![
            "Euler Quintic Lagrange Root Solving (L1-L5)",
            "Symplectic State Transition Matrix (STM) Variational Integration",
            "Single-Shooting Planar Lyapunov Orbit Differential Correction",
            "Two-Variable 3D Halo Orbit Differential Correction",
            "Monodromy Matrix Stability & Floquet Eigendecomposition",
            "Invariant Manifold Tube Generation (W^u, W^s)",
            "AAS 20-459 Low-Energy Multi-Body Transfer Engine",
        ],
    })
}

pub fn resolve_system(name: &str) -> Cr3bpSystem {
    match name.to_lowercase().as_str() {
        "sun_earth" => Cr3bpSystem::sun_earth(),
        "sun_jupiter" => Cr3bpSystem::sun_jupiter(),
        _ => Cr3bpSystem::earth_moon(),
    }
}

pub async fn get_system_info(Path(name): Path<String>) -> Result<Json<SystemInfoResponse>, StatusCode> {
    let system = resolve_system(&name);
    let pts = compute_lagrange_points(&system).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let lagrange_points = pts
        .iter()
        .map(|lp| LagrangePointDto {
            point: lp.point.to_string(),
            x: lp.state.x,
            y: lp.state.y,
            z: lp.state.z,
            jacobi_constant: lp.jacobi_constant,
            distance_to_primary_km: lp.distance_to_primary * system.l_star / 1000.0,
            distance_to_secondary_km: lp.distance_to_secondary * system.l_star / 1000.0,
        })
        .collect();

    Ok(Json(SystemInfoResponse {
        name,
        mu: system.mu,
        l_star_km: system.l_star / 1000.0,
        t_star_days: system.t_star / 86400.0,
        v_star_ms: system.v_star,
        lagrange_points,
    }))
}

pub async fn correct_lyapunov_handler(
    Json(req): Json<CorrectLyapunovRequest>,
) -> Result<Json<OrbitResponse>, (StatusCode, String)> {
    let system = resolve_system(req.system.as_deref().unwrap_or("earth_moon"));
    let max_iter = req.max_iter.unwrap_or(20);
    let tol = req.tol.unwrap_or(1e-10);

    let corrected = correct_planar_lyapunov(&system, req.x0, req.vy0_guess, max_iter, tol)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Sample dense orbit trajectory points for 3D scene rendering
    let opt = IntegratorOptions {
        rel_tol: 1e-11,
        abs_tol: 1e-11,
        initial_step: 1e-3,
        min_step: 1e-14,
        max_step: 0.01,
        max_steps: 50_000,
    };
    let integrator = DormandPrinceIntegrator::new(&system, opt);
    let prop = integrator
        .propagate_6d(&corrected.initial_state, 0.0, corrected.period_nondim, None)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let trajectory_points = prop
        .trajectory
        .into_iter()
        .map(|pt| [pt.t, pt.state.x, pt.state.y, pt.state.z])
        .collect();

    Ok(Json(OrbitResponse {
        success: true,
        initial_state: corrected.initial_state.to_array(),
        period_nondim: corrected.period_nondim,
        period_days: corrected.period_days,
        jacobi_constant: corrected.jacobi_constant,
        stability_index: corrected.stability_index,
        lambda_unstable: corrected.lambda_unstable,
        lambda_stable: corrected.lambda_stable,
        eigenvector_unstable: corrected.eigenvector_unstable,
        eigenvector_stable: corrected.eigenvector_stable,
        iterations: corrected.iterations,
        residual: corrected.residual,
        trajectory_points,
    }))
}

pub async fn correct_halo_handler(
    Json(req): Json<CorrectHaloRequest>,
) -> Result<Json<OrbitResponse>, (StatusCode, String)> {
    let system = resolve_system(req.system.as_deref().unwrap_or("earth_moon"));
    let max_iter = req.max_iter.unwrap_or(20);
    let tol = req.tol.unwrap_or(1e-8);

    let corrected = correct_3d_halo(&system, req.z0, req.x0_guess, req.vy0_guess, max_iter, tol)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let opt = IntegratorOptions {
        rel_tol: 1e-11,
        abs_tol: 1e-11,
        initial_step: 1e-3,
        min_step: 1e-14,
        max_step: 0.01,
        max_steps: 50_000,
    };
    let integrator = DormandPrinceIntegrator::new(&system, opt);
    let prop = integrator
        .propagate_6d(&corrected.initial_state, 0.0, corrected.period_nondim, None)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let trajectory_points = prop
        .trajectory
        .into_iter()
        .map(|pt| [pt.t, pt.state.x, pt.state.y, pt.state.z])
        .collect();

    Ok(Json(OrbitResponse {
        success: true,
        initial_state: corrected.initial_state.to_array(),
        period_nondim: corrected.period_nondim,
        period_days: corrected.period_days,
        jacobi_constant: corrected.jacobi_constant,
        stability_index: corrected.stability_index,
        lambda_unstable: corrected.lambda_unstable,
        lambda_stable: corrected.lambda_stable,
        eigenvector_unstable: corrected.eigenvector_unstable,
        eigenvector_stable: corrected.eigenvector_stable,
        iterations: corrected.iterations,
        residual: corrected.residual,
        trajectory_points,
    }))
}

pub async fn generate_manifold_handler(
    Json(req): Json<ManifoldRequest>,
) -> Result<Json<ManifoldResponse>, (StatusCode, String)> {
    let system = resolve_system(req.system.as_deref().unwrap_or("earth_moon"));

    let m_type = match req.manifold_type.to_lowercase().as_str() {
        "stable" => ManifoldType::Stable,
        _ => ManifoldType::Unstable,
    };

    let branch = match req.branch.to_lowercase().as_str() {
        "negative" => ManifoldBranch::Negative,
        _ => ManifoldBranch::Positive,
    };

    let dummy_orbit = CorrectedOrbit {
        initial_state: Cr3bpState::from_array(req.initial_state),
        period_nondim: req.period_nondim,
        period_days: req.period_nondim * system.t_star / 86400.0,
        jacobi_constant: 0.0,
        monodromy_matrix: [[0.0; 6]; 6],
        stability_index: 0.0,
        lambda_unstable: 1.0,
        lambda_stable: 1.0,
        eigenvector_unstable: req.eigenvector_unstable,
        eigenvector_stable: req.eigenvector_stable,
        iterations: 1,
        residual: 0.0,
    };

    let opts = ManifoldOptions {
        manifold_type: m_type,
        branch,
        orbit_phase: req.orbit_phase,
        epsilon_dist: req.epsilon_dist.unwrap_or(1e-5),
        t_span: req.t_span.unwrap_or(2.5),
    };

    let arc = generate_manifold_arc(&system, &dummy_orbit, &opts, None)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let trajectory_points = arc
        .trajectory
        .into_iter()
        .map(|pt| [pt.t, pt.state.x, pt.state.y, pt.state.z])
        .collect();

    Ok(Json(ManifoldResponse {
        success: true,
        manifold_type: format!("{:?}", arc.manifold_type),
        branch: format!("{:?}", arc.branch),
        orbit_phase: arc.orbit_phase,
        flight_time_days: arc.flight_time_days,
        jacobi_constant: arc.jacobi_constant,
        final_state: arc.final_state.to_array(),
        trajectory_points,
    }))
}

pub async fn get_transfer_benchmark() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let system = Cr3bpSystem::earth_moon();
    let plan = compute_earth_moon_l1_to_l2_transfer(&system, None)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let dep_pts: Vec<[f64; 4]> = plan
        .departure_arc
        .into_iter()
        .map(|pt| [pt.t, pt.state.x, pt.state.y, pt.state.z])
        .collect();
    let arr_pts: Vec<[f64; 4]> = plan
        .arrival_arc
        .into_iter()
        .map(|pt| [pt.t, pt.state.x, pt.state.y, pt.state.z])
        .collect();

    Ok(Json(serde_json::json!({
        "name": plan.name,
        "dv1_ms": plan.dv1_ms,
        "dv2_ms": plan.dv2_ms,
        "dv3_ms": plan.dv3_ms,
        "total_dv_ms": plan.total_dv_ms,
        "duration_days": plan.transfer_duration_days,
        "position_match_error_m": plan.position_match_error_m,
        "velocity_match_error_ms": plan.velocity_match_error_ms,
        "departure_arc": dep_pts,
        "arrival_arc": arr_pts,
    })))
}

#[derive(Deserialize, Debug)]
pub struct OptimizeTransferRequest {
    pub system: Option<String>,
    pub origin_preset: Option<String>,
    pub origin_state: Option<[f64; 6]>,
    pub destination_preset: Option<String>,
    pub destination_state: Option<[f64; 6]>,
    pub leo_altitude_km: Option<f64>,
    pub spacecraft_wet_mass_kg: Option<f64>,
    pub max_thrust_n: Option<f64>,
    pub isp_s: Option<f64>,
    pub flight_days: Option<f64>,
    pub n_nodes: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cr3bpTransferNodeDto {
    pub time_days: f64,
    pub tau: f64,
    pub position_km: [f64; 3],
    pub velocity_km_s: [f64; 3],
    pub thrust_accel_m_s2: [f64; 3],
    pub thrust_mn: f64,
    pub cumulative_delta_v_m_s: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cr3bpBurnSegmentDto {
    pub segment_index: usize,
    pub start_day: f64,
    pub end_day: f64,
    pub duration_hours: f64,
    pub average_thrust_mn: f64,
    pub delta_v_m_s: f64,
    pub fuel_consumed_kg: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OptimizeTransferResponse {
    pub success: bool,
    pub converged: bool,
    pub iterations: usize,
    pub total_flight_days: f64,
    pub total_delta_v_m_s: f64,
    pub tli_impulsive_delta_v_m_s: Option<f64>,
    pub total_mission_delta_v_m_s: f64,
    pub total_fuel_consumed_kg: f64,
    pub final_mass_kg: f64,
    pub max_thrust_used_mn: f64,
    pub burn_schedule: Vec<Cr3bpBurnSegmentDto>,
    pub trajectory_nodes: Vec<Cr3bpTransferNodeDto>,
}

fn resolve_orbit_state(
    custom_state: Option<[f64; 6]>,
    preset_name: Option<&str>,
    fallback_preset: &str,
    system: &Cr3bpSystem,
) -> Cr3bpState {
    if let Some(arr) = custom_state {
        Cr3bpState::from_array(arr)
    } else {
        match preset_name.unwrap_or(fallback_preset).to_lowercase().as_str() {
            "earth_geo" | "geostationary_orbit" => Cr3bpState::earth_geostationary(system),
            "earth_gto_apogee" => Cr3bpState::earth_gto_apogee(system),
            "trans_lunar_injection" | "tli_staging" => Cr3bpState::trans_lunar_injection_apogee(system),
            "earth_moon_l1_halo" => Cr3bpState::new(0.8234, 0.0, 0.045, 0.0, 0.13, 0.0),
            "earth_moon_l1_lyapunov" => Cr3bpState::new(0.8369, 0.0, 0.0, 0.0, 0.12, 0.0),
            "low_lunar_orbit" => Cr3bpState::new(1.0 - system.mu + 0.0048, 0.0, 0.0, 0.0, 1.63, 0.0),
            "lunar_gateway_nrho" => Cr3bpState::new(1.025, 0.0, 0.18, 0.0, -0.22, 0.0),
            "earth_moon_l2_halo" => Cr3bpState::new(1.155, 0.0, 0.05, 0.0, -0.15, 0.0),
            "earth_moon_l2_lyapunov" => Cr3bpState::new(1.12, 0.0, 0.0, 0.0, -0.18, 0.0),
            "earth_moon_l4" => Cr3bpState::new(0.5 - system.mu, 0.8660254037844386, 0.0, 0.0, 0.0, 0.0),
            "earth_moon_l5" => Cr3bpState::new(0.5 - system.mu, -0.8660254037844386, 0.0, 0.0, 0.0, 0.0),
            "sun_earth_l2_halo" => Cr3bpState::new(1.0083, 0.0, 0.0035, 0.0, -0.015, 0.0),
            _ => Cr3bpState::new(0.8369, 0.0, 0.0, 0.0, 0.12, 0.0),
        }
    }
}

pub async fn optimize_transfer_handler(
    Json(req): Json<OptimizeTransferRequest>,
) -> Result<Json<OptimizeTransferResponse>, (StatusCode, String)> {
    let sys_name = req.system.as_deref().unwrap_or("Earth-Moon");
    let system = resolve_system(sys_name);

    let origin_state = resolve_orbit_state(
        req.origin_state,
        req.origin_preset.as_deref(),
        "earth_moon_l1_lyapunov",
        &system,
    );

    let target_state = resolve_orbit_state(
        req.destination_state,
        req.destination_preset.as_deref(),
        "earth_moon_l2_halo",
        &system,
    );

    let config = Cr3bpTransferMissionConfig {
        wet_mass_kg: req.spacecraft_wet_mass_kg.unwrap_or(450.0),
        max_thrust_n: req.max_thrust_n.unwrap_or(0.35),
        isp_s: req.isp_s.unwrap_or(2800.0),
        flight_days: req.flight_days.unwrap_or(14.0),
        n_nodes: req.n_nodes.unwrap_or(30),
    };

    let optimizer = Cr3bpTransferOptimizer::new(system.clone(), origin_state, target_state, config);
    let plan = optimizer.optimize()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let tli_dv = req.leo_altitude_km.map(|alt| {
        calculate_tli_impulsive_dv(&system, alt, None)
    }).or_else(|| {
        let p = req.origin_preset.as_deref().unwrap_or("").to_lowercase();
        if p == "trans_lunar_injection" || p == "tli_staging" {
            Some(calculate_tli_impulsive_dv(&system, 300.0, None))
        } else {
            None
        }
    });

    let total_mission_delta_v_m_s = plan.total_delta_v_m_s + tli_dv.unwrap_or(0.0);

    let nodes = plan.nodes.into_iter().map(|n| Cr3bpTransferNodeDto {
        time_days: n.time_days,
        tau: n.tau,
        position_km: n.position_km,
        velocity_km_s: n.velocity_km_s,
        thrust_accel_m_s2: n.thrust_accel_m_s2,
        thrust_mn: n.thrust_mn,
        cumulative_delta_v_m_s: n.cumulative_delta_v_m_s,
    }).collect();

    let burns = plan.burn_schedule.into_iter().map(|b| Cr3bpBurnSegmentDto {
        segment_index: b.segment_index,
        start_day: b.start_day,
        end_day: b.end_day,
        duration_hours: b.duration_hours,
        average_thrust_mn: b.average_thrust_mn,
        delta_v_m_s: b.delta_v_m_s,
        fuel_consumed_kg: b.fuel_consumed_kg,
    }).collect();

    Ok(Json(OptimizeTransferResponse {
        success: true,
        converged: plan.converged,
        iterations: plan.iterations,
        total_flight_days: plan.total_flight_days,
        total_delta_v_m_s: plan.total_delta_v_m_s,
        tli_impulsive_delta_v_m_s: tli_dv,
        total_mission_delta_v_m_s,
        total_fuel_consumed_kg: plan.total_fuel_consumed_kg,
        final_mass_kg: plan.final_mass_kg,
        max_thrust_used_mn: plan.max_thrust_used_mn,
        burn_schedule: burns,
        trajectory_nodes: nodes,
    }))
}

#[derive(Deserialize, Debug)]
pub struct ExportOemRequest {
    pub object_name: Option<String>,
    pub object_id: Option<String>,
    pub center_name: Option<String>,
    pub ref_frame: Option<String>,
    pub time_system: Option<String>,
    pub start_date: Option<String>,
    pub nodes: Vec<Cr3bpTransferNodeDto>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportOemResponse {
    pub success: bool,
    pub format: &'static str,
    pub oem_content: String,
    pub csv_content: String,
}

pub async fn export_oem_handler(
    Json(req): Json<ExportOemRequest>,
) -> Result<Json<ExportOemResponse>, (StatusCode, String)> {
    let obj_name = req.object_name.as_deref().unwrap_or("DEEP_SPACE_TRANSFER_VEHICLE");
    let obj_id = req.object_id.as_deref().unwrap_or("2026-999A");
    let center = req.center_name.as_deref().unwrap_or("EARTH-MOON BARYCENTER");
    let frame = req.ref_frame.as_deref().unwrap_or("EME2000");
    let time_sys = req.time_system.as_deref().unwrap_or("UTC");
    let start_date = req.start_date.as_deref().unwrap_or("2026-09-22T00:00:00.000");

    let mut oem = String::new();
    oem.push_str("CCSDS_OEM_VERS = 2.0\n");
    oem.push_str("CREATION_DATE  = 2026-09-22T12:00:00.000\n");
    oem.push_str("ORIGINATOR     = SBM_ASTRODYNAMICS_FLIGHT_PLANNER\n\n");
    oem.push_str("META_START\n");
    oem.push_str(&format!("OBJECT_NAME          = {}\n", obj_name));
    oem.push_str(&format!("OBJECT_ID            = {}\n", obj_id));
    oem.push_str(&format!("CENTER_NAME          = {}\n", center));
    oem.push_str(&format!("REF_FRAME            = {}\n", frame));
    oem.push_str(&format!("TIME_SYSTEM          = {}\n", time_sys));
    oem.push_str(&format!("START_TIME           = {}\n", start_date));
    let stop_days = req.nodes.last().map(|n| n.time_days).unwrap_or(0.0);
    oem.push_str(&format!("STOP_TIME_REL_DAYS   = {:.4}\n", stop_days));
    oem.push_str("META_STOP\n\n");

    let mut csv = String::new();
    csv.push_str("time_days,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s,thrust_mn,cumulative_dv_m_s\n");

    for n in &req.nodes {
        oem.push_str(&format!(
            "{:+010.4} {:+14.6} {:+14.6} {:+14.6} {:+12.6} {:+12.6} {:+12.6}\n",
            n.time_days,
            n.position_km[0],
            n.position_km[1],
            n.position_km[2],
            n.velocity_km_s[0],
            n.velocity_km_s[1],
            n.velocity_km_s[2],
        ));

        csv.push_str(&format!(
            "{:.4},{:.4},{:.4},{:.4},{:.6},{:.6},{:.6},{:.2},{:.2}\n",
            n.time_days,
            n.position_km[0],
            n.position_km[1],
            n.position_km[2],
            n.velocity_km_s[0],
            n.velocity_km_s[1],
            n.velocity_km_s[2],
            n.thrust_mn,
            n.cumulative_delta_v_m_s,
        ));
    }

    Ok(Json(ExportOemResponse {
        success: true,
        format: "CCSDS OEM v2.0 & RFC 4180 CSV",
        oem_content: oem,
        csv_content: csv,
    }))
}
