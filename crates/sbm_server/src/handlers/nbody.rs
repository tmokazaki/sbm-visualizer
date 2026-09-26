//! HTTP handlers for N-body astronomy simulations and dynamical presets.

use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use sbm_core::nbody::{
    compute_conservation_metrics, create_preset, propagate_trajectory, CelestialBody,
    IntegratorType, NBodySystem, PresetId,
};

use crate::dto::{
    BodyDto, PresetSummaryDto, SimulateNBodyRequest, SimulateNBodyResponse, TrajectorySnapshotDto,
};

/// Returns available astronomical N-body presets.
pub async fn get_nbody_presets() -> impl IntoResponse {
    let presets = vec![
        PresetSummaryDto {
            id: "inner_solar_system_jupiter".to_string(),
            name: "Sun-Venus-Earth-Moon-Mars-Jupiter (Gravitational Force Focus)".to_string(),
            description: "Inner Solar System and Jupiter for high-fidelity gravitational force, tidal tensor, and Hill sphere analysis."
                .to_string(),
            body_count: 6,
            primary_body: "Sun".to_string(),
        },
        PresetSummaryDto {
            id: "solar_system".to_string(),
            name: "Solar System (JPL J2000)".to_string(),
            description: "Sun, 8 major planets, Pluto, and Earth's Moon from NASA JPL Horizons ephemerides."
                .to_string(),
            body_count: 11,
            primary_body: "Sun".to_string(),
        },
        PresetSummaryDto {
            id: "laplace_resonance".to_string(),
            name: "Jovian Laplace Resonance (4:2:1)".to_string(),
            description: "Jupiter with Io, Europa, and Ganymede locked in the 4:2:1 Laplace mean motion resonance."
                .to_string(),
            body_count: 5,
            primary_body: "Jupiter".to_string(),
        },
        PresetSummaryDto {
            id: "figure_eight".to_string(),
            name: "Figure-8 Three-Body Choreography".to_string(),
            description: "Exact periodic zero-angular-momentum 3-body solution (Chenciner & Montgomery 2000)."
                .to_string(),
            body_count: 3,
            primary_body: "Barycenter".to_string(),
        },
        PresetSummaryDto {
            id: "trojans".to_string(),
            name: "Sun-Jupiter Trojans (L4/L5 Resonance)".to_string(),
            description: "Primary, secondary, and Trojan asteroid camps librating at equilateral Lagrange points."
                .to_string(),
            body_count: 6,
            primary_body: "Sun".to_string(),
        },
        PresetSummaryDto {
            id: "mercury_gr".to_string(),
            name: "Relativistic Mercury Precession (1PN)".to_string(),
            description: "Sun-Mercury system with 1PN General Relativity showing secular perihelion advance."
                .to_string(),
            body_count: 2,
            primary_body: "Sun".to_string(),
        },
        PresetSummaryDto {
            id: "pythagorean".to_string(),
            name: "Pythagorean Three-Body Problem (Burrau 1913)".to_string(),
            description: "Masses 3, 4, 5 forming a right triangle at rest; classic chaotic benchmark."
                .to_string(),
            body_count: 3,
            primary_body: "Barycenter".to_string(),
        },
        PresetSummaryDto {
            id: "trappist1".to_string(),
            name: "TRAPPIST-1 Resonant Chain".to_string(),
            description: "Red dwarf star with 7 transiting exoplanets in a continuous resonant chain."
                .to_string(),
            body_count: 8,
            primary_body: "TRAPPIST-1".to_string(),
        },
    ];

    Json(presets)
}

/// Executes a high-precision N-body simulation forward in time.
pub async fn simulate_nbody_handler(
    Json(payload): Json<SimulateNBodyRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut system = if let Some(ref pid) = payload.preset_id {
        match pid.to_lowercase().as_str() {
            "inner_solar_system_jupiter" | "inner_focus_jupiter" | "inner" => {
                create_preset(PresetId::InnerSolarSystemJupiter)
            }
            "solar_system" | "solarsystem" => create_preset(PresetId::SolarSystemJpl),
            "laplace_resonance" | "laplace" => create_preset(PresetId::LaplaceResonance),
            "figure_eight" | "figure8" => create_preset(PresetId::FigureEight),
            "trojans" | "trojan" => create_preset(PresetId::SunJupiterTrojans),
            "mercury_gr" | "mercury" => create_preset(PresetId::RelativisticMercury),
            "pythagorean" => create_preset(PresetId::Pythagorean3Body),
            "trappist1" | "trappist" => create_preset(PresetId::Trappist1Chain),
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Unknown preset identifier: {}", pid),
                ))
            }
        }
    } else if let Some(custom) = payload.custom_bodies {
        let mut sys = NBodySystem::new();
        for b in custom {
            let mut body = CelestialBody::new(
                b.id,
                b.name,
                b.mass_kg,
                b.radius_km,
                b.position_m,
                b.velocity_mps,
                b.color_hex,
            );
            body.is_fixed = b.is_fixed;
            sys.add_body(body);
        }
        sys
    } else {
        create_preset(PresetId::SolarSystemJpl)
    };

    if let Some(ref itype) = payload.integrator {
        match itype.to_lowercase().as_str() {
            "yoshida6" | "yoshida6th" => system.integrator = IntegratorType::Yoshida6th,
            "leapfrog" | "verlet" => system.integrator = IntegratorType::Leapfrog,
            "hermite4" | "hermite" => system.integrator = IntegratorType::Hermite4th,
            "dop853" | "dormandprince" => system.integrator = IntegratorType::DormandPrince853,
            _ => system.integrator = IntegratorType::Yoshida4th,
        }
    }

    if let Some(gr) = payload.enable_gr {
        system.enable_general_relativity = gr;
    }

    if let Some(soft) = payload.softening_m {
        system.softening_m = soft;
    }

    let initial_metrics = compute_conservation_metrics(&system);
    let total_duration = payload.total_duration_s.clamp(0.1, 1e10);
    let output_step = payload.output_step_s.clamp(0.01, total_duration);
    let max_sub_step = (output_step / 10.0).max(1e-4);

    let snapshots = propagate_trajectory(&mut system, total_duration, output_step, max_sub_step);
    let final_metrics = compute_conservation_metrics(&system);

    let bodies_dto = system
        .bodies
        .iter()
        .map(|b| BodyDto {
            id: b.id,
            name: b.name.clone(),
            mass_kg: b.mass_kg,
            radius_km: b.radius_km,
            position_m: b.position_m,
            velocity_mps: b.velocity_mps,
            color_hex: b.color_hex.clone(),
            is_fixed: b.is_fixed,
        })
        .collect();

    let snapshots_dto = snapshots
        .into_iter()
        .map(|s| TrajectorySnapshotDto {
            time_s: s.time_s,
            positions_m: s.positions_m,
            velocities_mps: s.velocities_mps,
            relative_energy_error: s.relative_energy_error,
        })
        .collect();

    Ok(Json(SimulateNBodyResponse {
        bodies: bodies_dto,
        snapshots: snapshots_dto,
        initial_energy_j: initial_metrics.total_energy_j,
        final_energy_j: final_metrics.total_energy_j,
        relative_energy_error: final_metrics.relative_energy_error,
        linear_momentum_magnitude: final_metrics.linear_momentum_magnitude,
        angular_momentum_magnitude: final_metrics.angular_momentum_magnitude,
    }))
}
