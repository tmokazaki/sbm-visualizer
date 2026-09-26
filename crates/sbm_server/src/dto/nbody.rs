//! Data transfer objects for N-body gravitational dynamics and astronomy simulation endpoints.

use serde::{Deserialize, Serialize};

/// Celestial body serialization DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyDto {
    pub id: usize,
    pub name: String,
    pub mass_kg: f64,
    pub radius_km: f64,
    pub position_m: [f64; 3],
    pub velocity_mps: [f64; 3],
    pub color_hex: String,
    pub is_fixed: bool,
}

/// Request to simulate an N-body system forward in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateNBodyRequest {
    pub preset_id: Option<String>,
    pub custom_bodies: Option<Vec<BodyDto>>,
    pub total_duration_s: f64,
    pub output_step_s: f64,
    pub integrator: Option<String>,
    pub enable_gr: Option<bool>,
    pub softening_m: Option<f64>,
}

/// Single trajectory step snapshot DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectorySnapshotDto {
    pub time_s: f64,
    pub positions_m: Vec<[f64; 3]>,
    pub velocities_mps: Vec<[f64; 3]>,
    pub relative_energy_error: f64,
}

/// Simulation result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateNBodyResponse {
    pub bodies: Vec<BodyDto>,
    pub snapshots: Vec<TrajectorySnapshotDto>,
    pub initial_energy_j: f64,
    pub final_energy_j: f64,
    pub relative_energy_error: f64,
    pub linear_momentum_magnitude: f64,
    pub angular_momentum_magnitude: f64,
}

/// Summary of an available astronomical preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetSummaryDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub body_count: usize,
    pub primary_body: String,
}
