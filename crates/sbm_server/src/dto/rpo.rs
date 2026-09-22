//! Rendezvous & Proximity Operations (RPO) Data Transfer Objects.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct RpoPlanRequest {
    pub target_orbit: Option<String>,
    pub target_altitude_km: Option<f64>,
    pub mode: Option<String>,
    pub initial_state: Option<[f64; 6]>,
    pub target_position: Option<[f64; 3]>,
    pub target_velocity: Option<[f64; 3]>,
    pub duration_s: Option<f64>,
    pub nmc_radial_amplitude_m: Option<f64>,
    pub nmc_cross_track_amplitude_m: Option<f64>,
    pub glideslope_start_m: Option<f64>,
    pub glideslope_end_m: Option<f64>,
    pub num_hops: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpoPlanResponse {
    pub success: bool,
    pub mode: String,
    pub target_orbit: String,
    pub mean_motion_rad_s: f64,
    pub orbital_period_s: f64,
    pub total_delta_v_mps: f64,
    pub burns: Vec<RpoBurnDto>,
    pub trajectory_points: Vec<[f64; 4]>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpoBurnDto {
    pub time_s: f64,
    pub delta_v_mps: [f64; 3],
    pub magnitude_mps: f64,
    pub description: String,
}
