//! System parameters and health check handlers.

use axum::{extract::Path, http::StatusCode, Json};
use sbm_core::cr3bp::{compute_lagrange_points, Cr3bpSystem};

use crate::dto::{HealthResponse, LagrangePointDto, SystemInfoResponse};

/// Resolves a system name into a `Cr3bpSystem`.
pub fn resolve_system(name: &str) -> Cr3bpSystem {
    match name.to_lowercase().as_str() {
        "sun_earth" => Cr3bpSystem::sun_earth(),
        "sun_jupiter" => Cr3bpSystem::sun_jupiter(),
        _ => Cr3bpSystem::earth_moon(),
    }
}

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
