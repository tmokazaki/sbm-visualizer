//! System and health check Data Transfer Objects.

use serde::{Deserialize, Serialize};

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
