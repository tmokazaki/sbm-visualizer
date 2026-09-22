//! Periodic orbit correction and invariant manifold Data Transfer Objects.

use serde::{Deserialize, Serialize};

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
