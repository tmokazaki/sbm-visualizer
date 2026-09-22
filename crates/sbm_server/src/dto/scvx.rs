//! Successive Convexification (SCvx) transfer optimization and OEM export Data Transfer Objects.

use serde::{Deserialize, Serialize};

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
