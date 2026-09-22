//! Successive Convexification (SCvx) transfer optimization and OEM export handlers.

use axum::{http::StatusCode, Json};
use sbm_core::cr3bp::{calculate_tli_impulsive_dv, Cr3bpState, Cr3bpSystem};
use sbm_core::scvx::{Cr3bpTransferMissionConfig, Cr3bpTransferOptimizer};

use crate::dto::{
    Cr3bpBurnSegmentDto, Cr3bpTransferNodeDto, ExportOemRequest, ExportOemResponse,
    OptimizeTransferRequest, OptimizeTransferResponse,
};
use crate::handlers::system::resolve_system;

/// Resolves an origin or destination preset name or custom state array into a `Cr3bpState`.
pub fn resolve_orbit_state(
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
    let system = resolve_system(req.system.as_deref().unwrap_or("earth_moon"));

    let origin_state = resolve_orbit_state(
        req.origin_state,
        req.origin_preset.as_deref(),
        "earth_moon_l1_halo",
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
