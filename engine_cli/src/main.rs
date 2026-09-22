//! Unified CLI application for SBM Astrodynamics:
//! - NASA EVOLVE 4.0 Breakup Simulation (`breakup`)
//! - Clohessy-Wiltshire RPO Target Maneuver Planning (`rpo`)
//! - CR3BP Low-Energy & Multi-Body Transfer Planning (`transfer`)

#![deny(clippy::print_stdout, clippy::print_stderr)]

use sbm_core::cr3bp::{compute_earth_moon_l1_to_l2_transfer, Cr3bpSystem};
use sbm_core::prelude::*;
use sbm_core::rpo::{
    plan_glideslope_rbar, plan_glideslope_vbar, plan_natural_motion_circumnavigation,
    plan_two_impulse_transfer, RelativeState, TargetOrbit,
};
use std::env;
use std::fs::File;
use std::io::Write;
use tracing::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args: Vec<String> = env::args().collect();
    let subcommand = args.get(1).map(|s| s.as_str()).unwrap_or("breakup");

    match subcommand {
        "rpo" => run_rpo_cli(&args[2..])?,
        "transfer" => run_transfer_cli(&args[2..])?,
        "help" | "--help" | "-h" => print_usage(),
        "breakup" => run_breakup_cli(&args)?,
        _ => run_breakup_cli(&args)?,
    }

    Ok(())
}

fn print_usage() {
    info!("============================================================");
    info!(" SBM Astrodynamics & Mission Planning CLI");
    info!("============================================================");
    info!("Usage: sbm_simple_engine <COMMAND> [OPTIONS]");
    info!("");
    info!("COMMANDS:");
    info!("  breakup   Execute NASA EVOLVE 4.0 debris collision simulation (default)");
    info!("  rpo       Plan Clohessy-Wiltshire RPO target maneuvers");
    info!("  transfer  Compute CR3BP invariant manifold low-energy transfer");
    info!("  help      Show this usage guide");
    info!("");
    info!("RPO OPTIONS:");
    info!("  --mode <nmc|two-impulse|vbar|rbar>  RPO maneuver profile (default: two-impulse)");
    info!("  --target <iss|sso|geo>             Reference target orbit (default: iss)");
    info!("  --x <meters>                       Initial radial offset (default: -100.0)");
    info!("  --y <meters>                       Initial in-track offset (default: -500.0)");
    info!("  --z <meters>                       Initial cross-track offset (default: 0.0)");
    info!("  --duration <seconds>               Transfer duration (default: 1800.0)");
    info!("  --radial <meters>                  NMC radial semi-axis (default: 100.0)");
    info!("============================================================");
}

fn run_rpo_cli(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut mode = "two-impulse".to_string();
    let mut target = "iss".to_string();
    let mut x0 = -100.0;
    let mut y0 = -500.0;
    let mut z0 = 0.0;
    let mut duration_s = 1800.0;
    let mut radial_m = 100.0;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" if i + 1 < args.len() => {
                mode = args[i + 1].clone();
                i += 1;
            }
            "--target" if i + 1 < args.len() => {
                target = args[i + 1].clone();
                i += 1;
            }
            "--x" if i + 1 < args.len() => {
                x0 = args[i + 1].parse().unwrap_or(-100.0);
                i += 1;
            }
            "--y" if i + 1 < args.len() => {
                y0 = args[i + 1].parse().unwrap_or(-500.0);
                i += 1;
            }
            "--z" if i + 1 < args.len() => {
                z0 = args[i + 1].parse().unwrap_or(0.0);
                i += 1;
            }
            "--duration" if i + 1 < args.len() => {
                duration_s = args[i + 1].parse().unwrap_or(1800.0);
                i += 1;
            }
            "--radial" if i + 1 < args.len() => {
                radial_m = args[i + 1].parse().unwrap_or(100.0);
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    let orbit = match target.to_lowercase().as_str() {
        "sso" => TargetOrbit::sso_700km(),
        "geo" => TargetOrbit::geo(),
        _ => TargetOrbit::iss(),
    };

    info!("============================================================");
    info!(" CLOHESSY-WILTSHIRE RPO TARGET MANEUVER PLANNER (CLI)");
    info!("============================================================");
    info!(
        target = target,
        altitude_km = orbit.altitude_km,
        mean_motion_rad_s = orbit.mean_motion_rad_s,
        period_min = orbit.period_s / 60.0,
        "Target orbit configured"
    );

    match mode.to_lowercase().as_str() {
        "nmc" | "inspection" => {
            let plan = plan_natural_motion_circumnavigation(&orbit, None, radial_m, 50.0, 0.0, 60)
                .map_err(|e| format!("{:?}", e))?;

            info!(
                radial_amplitude_m = plan.radial_amplitude_m,
                along_track_amplitude_m = plan.along_track_amplitude_m,
                cross_track_amplitude_m = plan.cross_track_amplitude_m,
                period_min = plan.period_s / 60.0,
                "Passive Natural Motion Circumnavigation planned"
            );
            info!(
                burn_time_s = plan.insertion_maneuver.time_s,
                delta_v_mps = ?plan.insertion_maneuver.delta_v_mps,
                magnitude_mps = plan.insertion_maneuver.magnitude_mps,
                "Insertion Maneuver (Drift-Free 2:1 Ellipse)"
            );
            info!(
                total_delta_v_mps = plan.insertion_maneuver.magnitude_mps,
                "RPO Plan Complete - Zero continuous propellant drift"
            );
        }
        "vbar" => {
            let plan = plan_glideslope_vbar(&orbit, y0, -15.0, duration_s, 3)
                .map_err(|e| format!("{:?}", e))?;

            info!(
                start_y_m = plan.start_distance_m,
                end_y_m = plan.end_distance_m,
                duration_min = plan.duration_s / 60.0,
                burns_count = plan.burns.len(),
                total_delta_v_mps = plan.total_delta_v_mps,
                "V-Bar Multi-Hop Glideslope planned"
            );
            for (idx, b) in plan.burns.iter().enumerate() {
                info!(
                    burn_idx = idx + 1,
                    time_min = b.time_s / 60.0,
                    magnitude_mps = b.magnitude_mps,
                    desc = b.description,
                    "Glideslope Maneuver"
                );
            }
        }
        "rbar" => {
            let plan = plan_glideslope_rbar(&orbit, x0, -10.0, duration_s)
                .map_err(|e| format!("{:?}", e))?;

            info!(
                start_x_m = plan.start_distance_m,
                end_x_m = plan.end_distance_m,
                duration_min = plan.duration_s / 60.0,
                total_delta_v_mps = plan.total_delta_v_mps,
                "R-Bar Radial Passive-Abort Approach planned"
            );
        }
        _ => {
            let init_state = RelativeState::new(x0, y0, z0, 0.0, 0.0, 0.0);
            let target_pos = [0.0, -30.0, 0.0];
            let target_vel = [0.0, 0.0, 0.0];

            let plan = plan_two_impulse_transfer(
                &init_state,
                target_pos,
                target_vel,
                duration_s,
                &orbit,
                50,
            )
            .map_err(|e| format!("{:?}", e))?;

            info!(
                departure_pos = ?[x0, y0, z0],
                target_pos = ?target_pos,
                duration_min = duration_s / 60.0,
                "Two-Impulse Targeted Transfer Planned"
            );
            info!(
                time_s = plan.dv1.time_s,
                delta_v_mps = ?plan.dv1.delta_v_mps,
                magnitude_mps = plan.dv1.magnitude_mps,
                desc = plan.dv1.description,
                "Maneuver 1 (Departure)"
            );
            info!(
                time_s = plan.dv2.time_s,
                delta_v_mps = ?plan.dv2.delta_v_mps,
                magnitude_mps = plan.dv2.magnitude_mps,
                desc = plan.dv2.description,
                "Maneuver 2 (Arrival Braking)"
            );
            info!(
                total_delta_v_mps = plan.total_delta_v_mps,
                "Total Maneuver Budget"
            );
        }
    }
    info!("============================================================");

    Ok(())
}

fn run_transfer_cli(_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let sys = Cr3bpSystem::earth_moon();
    info!("============================================================");
    info!(" CR3BP LOW-ENERGY INVARIANT MANIFOLD TRANSFER (CLI)");
    info!("============================================================");
    info!(
        system = "Earth-Moon",
        mass_ratio_mu = sys.mu,
        characteristic_length_km = sys.l_star,
        characteristic_time_days = sys.t_star,
        characteristic_velocity_mps = sys.v_star,
        "System parameters loaded"
    );

    let transfer = compute_earth_moon_l1_to_l2_transfer(&sys, None)
        .map_err(|e| format!("{:?}", e))?;

    info!(
        departure_orbit = "Earth-Moon L1 Southern Lyapunov",
        target_orbit = "Earth-Moon L2 Southern Lyapunov",
        dv1_mps = transfer.dv1_ms,
        dv2_mps = transfer.dv2_ms,
        dv3_mps = transfer.dv3_ms,
        total_delta_v_mps = transfer.total_dv_ms,
        pos_match_error_m = transfer.position_match_error_m,
        vel_match_error_mps = transfer.velocity_match_error_ms,
        "Low-energy heteroclinic transfer evaluated (AAS 20-459)"
    );
    info!("============================================================");

    Ok(())
}

fn run_breakup_cli(_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    info!("============================================================");
    info!(" NASA Standard Breakup Model - Simple Engine CLI");
    info!("============================================================");

    let target_m = 1000.0;    // kg
    let proj_m = 100.0;       // kg
    let impact_v = 10_000.0;  // 10 km/s (10,000 m/s)

    let engine = BreakupEngine::builder()
        .target_mass(target_m)
        .projectile_mass(proj_m)
        .impact_speed(impact_v)
        .target_type(ObjectType::Spacecraft)
        .projectile_type(ObjectType::Spacecraft)
        .breakup_type(BreakupType::Collision)
        .num_fragments(1500)
        .build()?;

    let result = engine.simulate(42);

    info!(target_mass_kg = target_m, target_type = ?engine.target_type, "Configured target");
    info!(projectile_mass_kg = proj_m, projectile_type = ?engine.projectile_type, "Configured projectile");
    info!(impact_velocity_km_s = impact_v / 1000.0, specific_energy_kj_per_kg = result.specific_energy_kj_per_kg, "Collision parameters");
    info!(
        outcome = if result.is_catastrophic { "CATASTROPHIC (Total breakup)" } else { "PARTIAL (Crater/Remnant)" },
        destroyed_mass_kg = result.destroyed_mass_kg,
        remnant_mass_kg = result.remnant_mass_kg,
        "Breakup regime evaluated"
    );
    info!(
        yield_1cm = result.physical_yield_1cm,
        yield_10cm_ssn = result.ssn_trackable_yield_10cm,
        sampled_fragments = result.fragments.len(),
        "Debris cloud population generated"
    );

    info!("--- TOP 3 HEAVIEST FRAGMENTS (Core chunks) ---");
    for f in result.top_heaviest(3) {
        info!(
            id = f.id,
            size_m = f.size_m,
            area_m2 = f.cross_section_m2,
            am_ratio = f.am_ratio,
            mass_kg = f.mass_kg,
            speed_mps = f.speed_mps,
            band = f.contour_band,
            "Heavy fragment"
        );
    }

    info!("--- TOP 3 FASTEST FRAGMENTS (Light shards) ---");
    for f in result.top_fastest(3) {
        info!(
            id = f.id,
            size_cm = f.size_m * 100.0,
            area_m2 = f.cross_section_m2,
            am_ratio = f.am_ratio,
            mass_kg = f.mass_kg,
            speed_mps = f.speed_mps,
            band = f.contour_band,
            "Fast fragment"
        );
    }

    let mut json_file = File::create("fragments_output.json").expect("Failed to create file");
    json_file
        .write_all(result.to_fragments_json().as_bytes())
        .expect("Failed to write JSON output");

    info!(
        output_file = "fragments_output.json",
        fragments_count = result.fragments.len(),
        "Successfully exported fragments"
    );
    info!("============================================================");

    Ok(())
}
