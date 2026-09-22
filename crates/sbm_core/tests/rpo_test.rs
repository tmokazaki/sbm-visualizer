//! Comprehensive tests for Rendezvous and Proximity Operations (RPO) and CW relative dynamics.

use sbm_core::rpo::{
    cw_state_transition_matrix, plan_glideslope_rbar, plan_glideslope_vbar,
    plan_natural_motion_circumnavigation, plan_two_impulse_transfer, propagate_cw, RelativeState,
    TargetOrbit,
};

#[test]
fn test_cw_stm_identity_at_t0() {
    let omega = 0.00113; // ~400 km LEO orbit mean motion
    let phi0 = cw_state_transition_matrix(omega, 0.0);

    for (r, row) in phi0.iter().enumerate() {
        for (c, &val) in row.iter().enumerate() {
            let expected = if r == c { 1.0 } else { 0.0 };
            assert!(
                (val - expected).abs() < 1e-12,
                "CW-STM not identity at [{}, {}]: got {}, expected {}",
                r,
                c,
                val,
                expected
            );
        }
    }
}

#[test]
fn test_cw_stm_drift_free_nmc_closed_orbit() {
    let orbit = TargetOrbit::circular(400.0); // 400 km LEO (ISS orbit)
    let omega = orbit.mean_motion_rad_s;
    let period = orbit.period_s;

    // NMC closed relative orbit with 2:1 aspect ratio:
    // Radial amplitude A = 100 m, In-track amplitude = 200 m
    // Drift-free condition: vy0 = -2 * omega * x0
    let x0 = -100.0;
    let y0 = 0.0;
    let z0 = 50.0;
    let vx0 = 0.0;
    let vy0 = -2.0 * omega * x0; // = 200 * omega
    let vz0 = 0.0;

    let initial_state = RelativeState::new(x0, y0, z0, vx0, vy0, vz0);

    // Propagate for one complete orbital period T
    let state_after_one_orbit = propagate_cw(&initial_state, period, omega);

    // Position and velocity must match initial state with sub-millimeter precision
    let pos_diff = (state_after_one_orbit - initial_state).r_norm();
    let vel_diff = (state_after_one_orbit - initial_state).v_norm();

    assert!(
        pos_diff < 1e-6,
        "NMC orbit must close after 1 period: pos error = {:.2e} m",
        pos_diff
    );
    assert!(
        vel_diff < 1e-9,
        "NMC velocity must match after 1 period: vel error = {:.2e} m/s",
        vel_diff
    );
}

#[test]
fn test_two_impulse_targeted_rendezvous() {
    let orbit = TargetOrbit::circular(500.0);
    let omega = orbit.mean_motion_rad_s;

    // Chaser starts 2 km behind and 300 m below target
    let initial_state = RelativeState::new(-300.0, -2000.0, 50.0, 0.05, 0.20, -0.02);

    // Target waypoint: 50 m on V-bar ahead of target
    let target_pos = [0.0, -50.0, 0.0];
    let desired_vel = [0.0, 0.0, 0.0]; // Stationkeeping stop
    let duration_s = 1800.0; // 30 minutes

    let plan = plan_two_impulse_transfer(
        &initial_state,
        target_pos,
        desired_vel,
        duration_s,
        &orbit,
        50,
    )
    .expect("Two-impulse transfer must succeed");

    assert!(plan.success);
    assert!(plan.total_delta_v_mps > 0.0 && plan.total_delta_v_mps < 20.0);
    assert!(plan.trajectory_points.len() == 50);

    // Verify propagation from departure state reaches target position exactly
    let post_dv1_state = RelativeState::new(
        initial_state.x,
        initial_state.y,
        initial_state.z,
        initial_state.vx + plan.dv1.delta_v_mps[0],
        initial_state.vy + plan.dv1.delta_v_mps[1],
        initial_state.vz + plan.dv1.delta_v_mps[2],
    );

    let arrival_state = propagate_cw(&post_dv1_state, duration_s, omega);
    let target_pos_error = ((arrival_state.x - target_pos[0]).powi(2)
        + (arrival_state.y - target_pos[1]).powi(2)
        + (arrival_state.z - target_pos[2]).powi(2))
    .sqrt();

    assert!(
        target_pos_error < 1e-6,
        "Target position error too large: {:.2e} m",
        target_pos_error
    );

    // Verify arrival brake dv2 zeroes out the velocity relative to target
    let final_rel_vel = [
        arrival_state.vx + plan.dv2.delta_v_mps[0],
        arrival_state.vy + plan.dv2.delta_v_mps[1],
        arrival_state.vz + plan.dv2.delta_v_mps[2],
    ];
    let final_vel_mag = (final_rel_vel[0].powi(2) + final_rel_vel[1].powi(2) + final_rel_vel[2].powi(2)).sqrt();
    assert!(
        final_vel_mag < 1e-6,
        "Arrival velocity residual too large: {:.2e} m/s",
        final_vel_mag
    );
}

#[test]
fn test_natural_motion_circumnavigation_flyaround() {
    let orbit = TargetOrbit::iss();

    let plan = plan_natural_motion_circumnavigation(&orbit, None, 150.0, 75.0, 0.0, 60)
        .expect("NMC plan must succeed");

    assert!(plan.success);
    assert_eq!(plan.radial_amplitude_m, 150.0);
    assert_eq!(plan.along_track_amplitude_m, 300.0); // 2:1 ratio
    assert_eq!(plan.cross_track_amplitude_m, 75.0);
    assert_eq!(plan.trajectory_points.len(), 60);

    // Verify periodic closure
    let p_start = plan.trajectory_points.first().unwrap();
    let p_end = plan.trajectory_points.last().unwrap();
    let close_err = ((p_end[1] - p_start[1]).powi(2)
        + (p_end[2] - p_start[2]).powi(2)
        + (p_end[3] - p_start[3]).powi(2))
    .sqrt();

    assert!(close_err < 1e-3, "NMC path must close: error = {:.2e} m", close_err);
}

#[test]
fn test_vbar_and_rbar_glideslopes() {
    let orbit = TargetOrbit::sso_700km();

    // 1. V-Bar Multi-Hop Glideslope
    let vbar_plan = plan_glideslope_vbar(&orbit, -2000.0, -100.0, 3600.0, 4)
        .expect("V-Bar plan must succeed");

    assert!(vbar_plan.success);
    assert_eq!(vbar_plan.burns.len(), 8); // 4 hops * 2 burns each
    assert!(vbar_plan.total_delta_v_mps > 0.0 && vbar_plan.total_delta_v_mps < 15.0);

    // 2. R-Bar Radial Approach
    let rbar_plan = plan_glideslope_rbar(&orbit, -1000.0, -50.0, 1800.0)
        .expect("R-Bar plan must succeed");

    assert!(rbar_plan.success);
    assert_eq!(rbar_plan.burns.len(), 2);
    assert!(rbar_plan.total_delta_v_mps > 0.0);
}
