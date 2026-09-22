//! Periodic orbit correction and invariant manifold handlers.

use axum::{http::StatusCode, Json};
use sbm_core::cr3bp::{
    compute_earth_moon_l1_to_l2_transfer, correct_3d_halo, correct_planar_lyapunov,
    generate_manifold_arc, CorrectedOrbit, Cr3bpState, Cr3bpSystem, DormandPrinceIntegrator,
    IntegratorOptions, ManifoldBranch, ManifoldOptions, ManifoldType,
};

use crate::dto::{
    CorrectHaloRequest, CorrectLyapunovRequest, ManifoldRequest, ManifoldResponse, OrbitResponse,
};
use crate::handlers::system::resolve_system;

pub async fn correct_lyapunov_handler(
    Json(req): Json<CorrectLyapunovRequest>,
) -> Result<Json<OrbitResponse>, (StatusCode, String)> {
    let system = resolve_system(req.system.as_deref().unwrap_or("earth_moon"));
    let max_iter = req.max_iter.unwrap_or(20);
    let tol = req.tol.unwrap_or(1e-8);

    let corrected = correct_planar_lyapunov(&system, req.x0, req.vy0_guess, max_iter, tol)
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
