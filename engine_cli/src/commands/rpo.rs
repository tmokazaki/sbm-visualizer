//! Clohessy-Wiltshire Rendezvous & Proximity Operations CLI command.

use sbm_core::rpo::{
    plan_glideslope_rbar, plan_glideslope_vbar, plan_natural_motion_circumnavigation,
    plan_two_impulse_transfer, RelativeState, TargetOrbit,
};
use tracing::info;

pub fn run_rpo_cli(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
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
