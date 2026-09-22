//! Rendezvous & Proximity Operations (RPO) trajectory planning handlers.

use axum::{http::StatusCode, Json};
use sbm_core::rpo::{
    plan_glideslope_rbar, plan_glideslope_vbar, plan_natural_motion_circumnavigation,
    plan_two_impulse_transfer, RelativeState, TargetOrbit,
};

use crate::dto::{RpoBurnDto, RpoPlanRequest, RpoPlanResponse};

pub async fn plan_rpo_handler(
    Json(req): Json<RpoPlanRequest>,
) -> Result<Json<RpoPlanResponse>, (StatusCode, String)> {
    let orbit = match req.target_orbit.as_deref().unwrap_or("iss").to_lowercase().as_str() {
        "iss" => TargetOrbit::iss(),
        "sso" => TargetOrbit::sso_700km(),
        "geo" => TargetOrbit::geo(),
        _ => {
            let alt = req.target_altitude_km.unwrap_or(400.0);
            TargetOrbit::circular(alt)
        }
    };

    let mode = req.mode.as_deref().unwrap_or("two_impulse").to_lowercase();

    match mode.as_str() {
        "nmc" | "inspection" => {
            let radial_m = req.nmc_radial_amplitude_m.unwrap_or(100.0);
            let cross_track_m = req.nmc_cross_track_amplitude_m.unwrap_or(50.0);
            let plan = plan_natural_motion_circumnavigation(&orbit, None, radial_m, cross_track_m, 0.0, 60)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("{:?}", e)))?;

            let burns = vec![RpoBurnDto {
                time_s: plan.insertion_maneuver.time_s,
                delta_v_mps: plan.insertion_maneuver.delta_v_mps,
                magnitude_mps: plan.insertion_maneuver.magnitude_mps,
                description: plan.insertion_maneuver.description,
            }];

            Ok(Json(RpoPlanResponse {
                success: true,
                mode: "Natural Motion Circumnavigation".to_string(),
                target_orbit: format!("{:.1} km circular", orbit.altitude_km),
                mean_motion_rad_s: orbit.mean_motion_rad_s,
                orbital_period_s: orbit.period_s,
                total_delta_v_mps: plan.insertion_maneuver.magnitude_mps,
                burns,
                trajectory_points: plan.trajectory_points,
            }))
        }
        "vbar" | "v_bar" => {
            let start_y = req.glideslope_start_m.unwrap_or(-400.0);
            let end_y = req.glideslope_end_m.unwrap_or(-15.0);
            let duration_s = req.duration_s.unwrap_or(1800.0);
            let hops = req.num_hops.unwrap_or(3);

            let plan = plan_glideslope_vbar(&orbit, start_y, end_y, duration_s, hops)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("{:?}", e)))?;

            let burns = plan.burns.into_iter().map(|b| RpoBurnDto {
                time_s: b.time_s,
                delta_v_mps: b.delta_v_mps,
                magnitude_mps: b.magnitude_mps,
                description: b.description,
            }).collect();

            Ok(Json(RpoPlanResponse {
                success: true,
                mode: "V-Bar Along-Track Glideslope".to_string(),
                target_orbit: format!("{:.1} km circular", orbit.altitude_km),
                mean_motion_rad_s: orbit.mean_motion_rad_s,
                orbital_period_s: orbit.period_s,
                total_delta_v_mps: plan.total_delta_v_mps,
                burns,
                trajectory_points: plan.trajectory_points,
            }))
        }
        "rbar" | "r_bar" => {
            let start_x = req.glideslope_start_m.unwrap_or(-200.0);
            let end_x = req.glideslope_end_m.unwrap_or(-10.0);
            let duration_s = req.duration_s.unwrap_or(1200.0);

            let plan = plan_glideslope_rbar(&orbit, start_x, end_x, duration_s)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("{:?}", e)))?;

            let burns = plan.burns.into_iter().map(|b| RpoBurnDto {
                time_s: b.time_s,
                delta_v_mps: b.delta_v_mps,
                magnitude_mps: b.magnitude_mps,
                description: b.description,
            }).collect();

            Ok(Json(RpoPlanResponse {
                success: true,
                mode: "R-Bar Radial Fail-Safe Approach".to_string(),
                target_orbit: format!("{:.1} km circular", orbit.altitude_km),
                mean_motion_rad_s: orbit.mean_motion_rad_s,
                orbital_period_s: orbit.period_s,
                total_delta_v_mps: plan.total_delta_v_mps,
                burns,
                trajectory_points: plan.trajectory_points,
            }))
        }
        _ => {
            let s = req.initial_state.unwrap_or([-100.0, -500.0, 0.0, 0.0, 0.0, 0.0]);
            let initial_state = RelativeState::new(s[0], s[1], s[2], s[3], s[4], s[5]);
            let target_pos = req.target_position.unwrap_or([0.0, -30.0, 0.0]);
            let target_vel = req.target_velocity.unwrap_or([0.0, 0.0, 0.0]);
            let duration_s = req.duration_s.unwrap_or(1800.0);

            let plan = plan_two_impulse_transfer(
                &initial_state,
                target_pos,
                target_vel,
                duration_s,
                &orbit,
                60,
            )
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("{:?}", e)))?;

            let burns = vec![
                RpoBurnDto {
                    time_s: plan.dv1.time_s,
                    delta_v_mps: plan.dv1.delta_v_mps,
                    magnitude_mps: plan.dv1.magnitude_mps,
                    description: plan.dv1.description,
                },
                RpoBurnDto {
                    time_s: plan.dv2.time_s,
                    delta_v_mps: plan.dv2.delta_v_mps,
                    magnitude_mps: plan.dv2.magnitude_mps,
                    description: plan.dv2.description,
                },
            ];

            Ok(Json(RpoPlanResponse {
                success: true,
                mode: "Two-Impulse Targeted Rendezvous".to_string(),
                target_orbit: format!("{:.1} km circular", orbit.altitude_km),
                mean_motion_rad_s: orbit.mean_motion_rad_s,
                orbital_period_s: orbit.period_s,
                total_delta_v_mps: plan.total_delta_v_mps,
                burns,
                trajectory_points: plan.trajectory_points,
            }))
        }
    }
}
