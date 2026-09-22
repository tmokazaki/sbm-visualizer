//! Clohessy-Wiltshire (Hill) Relative Orbital Mechanics & RPO Maneuver Planning.
//!
//! Grounded in:
//! > **Clohessy, W. H., & Wiltshire, R. S. (1960).**  
//! > *Terminal Guidance System for Satellite Rendezvous.*  
//! > Journal of the Aerospace Sciences, 27(9), pp. 653–658.
//!
//! And standard operational rendezvous targeting:
//! > **Fehse, W. (2003).**  
//! > *Automated Rendezvous and Docking of Spacecraft.*  
//! > Cambridge Aerospace Series, Cambridge University Press.

use crate::rpo::types::{
    GlideslopeApproachPlan, NmcInspectionPlan, RelativeState, RpoError, RpoManeuverDto,
    TargetOrbit, TwoImpulseTransferPlan,
};

/// Computes the exact 6x6 Clohessy-Wiltshire (CW) State Transition Matrix (STM).
///
/// $$\mathbf{\Phi}(t) = \begin{bmatrix} \boldsymbol{\Phi}_{rr}(t) & \boldsymbol{\Phi}_{rv}(t) \\ \boldsymbol{\Phi}_{vr}(t) & \boldsymbol{\Phi}_{vv}(t) \end{bmatrix}$$
pub fn cw_state_transition_matrix(omega: f64, t: f64) -> [[f64; 6]; 6] {
    let tau = omega * t;
    let s = tau.sin();
    let c = tau.cos();
    let inv_w = if omega.abs() > 1e-12 { 1.0 / omega } else { 0.0 };

    let mut phi = [[0.0; 6]; 6];

    // Phi_rr (Position-to-Position)
    phi[0][0] = 4.0 - 3.0 * c;
    phi[0][1] = 0.0;
    phi[0][2] = 0.0;
    phi[1][0] = 6.0 * (s - tau);
    phi[1][1] = 1.0;
    phi[1][2] = 0.0;
    phi[2][0] = 0.0;
    phi[2][1] = 0.0;
    phi[2][2] = c;

    // Phi_rv (Velocity-to-Position)
    phi[0][3] = s * inv_w;
    phi[0][4] = 2.0 * (1.0 - c) * inv_w;
    phi[0][5] = 0.0;
    phi[1][3] = 2.0 * (c - 1.0) * inv_w;
    phi[1][4] = (4.0 * s - 3.0 * tau) * inv_w;
    phi[1][5] = 0.0;
    phi[2][3] = 0.0;
    phi[2][4] = 0.0;
    phi[2][5] = s * inv_w;

    // Phi_vr (Position-to-Velocity)
    phi[3][0] = 3.0 * omega * s;
    phi[3][1] = 0.0;
    phi[3][2] = 0.0;
    phi[4][0] = 6.0 * omega * (c - 1.0);
    phi[4][1] = 0.0;
    phi[4][2] = 0.0;
    phi[5][0] = 0.0;
    phi[5][1] = 0.0;
    phi[5][2] = -omega * s;

    // Phi_vv (Velocity-to-Velocity)
    phi[3][3] = c;
    phi[3][4] = 2.0 * s;
    phi[3][5] = 0.0;
    phi[4][3] = -2.0 * s;
    phi[4][4] = 4.0 * c - 3.0;
    phi[4][5] = 0.0;
    phi[5][3] = 0.0;
    phi[5][4] = 0.0;
    phi[5][5] = c;

    phi
}

/// Propagates a relative state vector analytically across time step $\Delta t$ using the CW-STM.
pub fn propagate_cw(state: &RelativeState, dt: f64, omega: f64) -> RelativeState {
    let phi = cw_state_transition_matrix(omega, dt);
    let s = state.to_array();

    let mut out = [0.0; 6];
    for i in 0..6 {
        let mut sum = 0.0;
        for j in 0..6 {
            sum += phi[i][j] * s[j];
        }
        out[i] = sum;
    }

    RelativeState::from_array(out)
}

/// Inverts a 3x3 matrix analytically using Cramer's rule.
pub fn invert_3x3(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

    if det.abs() < 1e-14 {
        return None;
    }

    let inv_det = 1.0 / det;
    let mut inv = [[0.0; 3]; 3];

    inv[0][0] = (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det;
    inv[0][1] = (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_det;
    inv[0][2] = (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det;

    inv[1][0] = (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_det;
    inv[1][1] = (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det;
    inv[1][2] = (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_det;

    inv[2][0] = (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det;
    inv[2][1] = (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_det;
    inv[2][2] = (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det;

    Some(inv)
}

/// Plans an optimal Two-Impulse Targeted Rendezvous / Proximity Transfer (CW Targeting).
///
/// Solves for the departure impulse $\Delta \mathbf{v}_1$ and arrival brake impulse $\Delta \mathbf{v}_2$ to transfer
/// from initial state $\mathbf{x}_0 = [\mathbf{r}_0^T, \mathbf{v}_0^{-T}]^T$ to target relative state
/// $\mathbf{x}_f = [\mathbf{r}_f^T, \mathbf{v}_f^{+T}]^T$ in time $\Delta t$.
///
/// Grounded in:
/// $$\mathbf{r}_f = \boldsymbol{\Phi}_{rr}(\Delta t) \mathbf{r}_0 + \boldsymbol{\Phi}_{rv}(\Delta t) \mathbf{v}_0^+$$
/// $$\mathbf{v}_0^+ = \boldsymbol{\Phi}_{rv}(\Delta t)^{-1} \left( \mathbf{r}_f - \boldsymbol{\Phi}_{rr}(\Delta t) \mathbf{r}_0 \right)$$
/// $$\Delta \mathbf{v}_1 = \mathbf{v}_0^+ - \mathbf{v}_0^-$$
/// $$\mathbf{v}_f^- = \boldsymbol{\Phi}_{vr}(\Delta t) \mathbf{r}_0 + \boldsymbol{\Phi}_{vv}(\Delta t) \mathbf{v}_0^+$$
/// $$\Delta \mathbf{v}_2 = \mathbf{v}_f^+ - \mathbf{v}_f^-$$
pub fn plan_two_impulse_transfer(
    initial_state: &RelativeState,
    target_position_m: [f64; 3],
    desired_arrival_velocity_mps: [f64; 3],
    transfer_duration_s: f64,
    target_orbit: &TargetOrbit,
    num_trajectory_points: usize,
) -> Result<TwoImpulseTransferPlan, RpoError> {
    if transfer_duration_s <= 1.0 {
        return Err(RpoError::InvalidConfiguration(
            "Transfer duration must be positive and greater than 1 second".to_string(),
        ));
    }

    let omega = target_orbit.mean_motion_rad_s;
    if omega <= 0.0 {
        return Err(RpoError::ZeroTargetMeanMotion);
    }

    // Check for resonance singularity (multiples of orbital period)
    let tau = omega * transfer_duration_s;
    let period_fraction = tau / (2.0 * core::f64::consts::PI);
    if (period_fraction.round() - period_fraction).abs() < 1e-3 {
        return Err(RpoError::SingularTransferTime(format!(
            "Transfer duration {:.1} s is an integer multiple of orbital period (singular Phi_rv)",
            transfer_duration_s
        )));
    }

    let phi = cw_state_transition_matrix(omega, transfer_duration_s);

    let mut phi_rr = [[0.0; 3]; 3];
    let mut phi_rv = [[0.0; 3]; 3];
    let mut phi_vr = [[0.0; 3]; 3];
    let mut phi_vv = [[0.0; 3]; 3];

    for i in 0..3 {
        for j in 0..3 {
            phi_rr[i][j] = phi[i][j];
            phi_rv[i][j] = phi[i][3 + j];
            phi_vr[i][j] = phi[3 + i][j];
            phi_vv[i][j] = phi[3 + i][3 + j];
        }
    }

    let inv_phi_rv = invert_3x3(&phi_rv).ok_or_else(|| {
        RpoError::SingularTransferTime("Phi_rv block matrix is non-invertible".to_string())
    })?;

    // r_target - Phi_rr * r_0
    let r0 = initial_state.position();
    let mut phi_rr_r0 = [0.0; 3];
    for i in 0..3 {
        phi_rr_r0[i] = phi_rr[i][0] * r0[0] + phi_rr[i][1] * r0[1] + phi_rr[i][2] * r0[2];
    }

    let mut diff = [0.0; 3];
    for i in 0..3 {
        diff[i] = target_position_m[i] - phi_rr_r0[i];
    }

    // Required post-burn departure velocity: v0_plus = inv(Phi_rv) * diff
    let mut v0_plus = [0.0; 3];
    for i in 0..3 {
        v0_plus[i] = inv_phi_rv[i][0] * diff[0] + inv_phi_rv[i][1] * diff[1] + inv_phi_rv[i][2] * diff[2];
    }

    // Delta-v 1 (Departure burn)
    let v0_minus = initial_state.velocity();
    let dv1_vec = [
        v0_plus[0] - v0_minus[0],
        v0_plus[1] - v0_minus[1],
        v0_plus[2] - v0_minus[2],
    ];
    let dv1_mag = (dv1_vec[0] * dv1_vec[0] + dv1_vec[1] * dv1_vec[1] + dv1_vec[2] * dv1_vec[2]).sqrt();

    // Arrival velocity before braking: vf_minus = Phi_vr * r0 + Phi_vv * v0_plus
    let mut vf_minus = [0.0; 3];
    for i in 0..3 {
        vf_minus[i] = (phi_vr[i][0] * r0[0] + phi_vr[i][1] * r0[1] + phi_vr[i][2] * r0[2])
            + (phi_vv[i][0] * v0_plus[0] + phi_vv[i][1] * v0_plus[1] + phi_vv[i][2] * v0_plus[2]);
    }

    // Delta-v 2 (Arrival brake / insertion burn)
    let dv2_vec = [
        desired_arrival_velocity_mps[0] - vf_minus[0],
        desired_arrival_velocity_mps[1] - vf_minus[1],
        desired_arrival_velocity_mps[2] - vf_minus[2],
    ];
    let dv2_mag = (dv2_vec[0] * dv2_vec[0] + dv2_vec[1] * dv2_vec[1] + dv2_vec[2] * dv2_vec[2]).sqrt();

    let post_burn_state = RelativeState::new(
        initial_state.x,
        initial_state.y,
        initial_state.z,
        v0_plus[0],
        v0_plus[1],
        v0_plus[2],
    );

    // Sample dense trajectory points for 3D visualizer
    let n_pts = num_trajectory_points.max(20);
    let mut trajectory_points = Vec::with_capacity(n_pts);

    for step in 0..n_pts {
        let t_sec = transfer_duration_s * (step as f64) / ((n_pts - 1) as f64);
        let s_t = propagate_cw(&post_burn_state, t_sec, omega);
        trajectory_points.push([t_sec, s_t.x, s_t.y, s_t.z]);
    }

    let final_arrival_state = RelativeState::new(
        target_position_m[0],
        target_position_m[1],
        target_position_m[2],
        desired_arrival_velocity_mps[0],
        desired_arrival_velocity_mps[1],
        desired_arrival_velocity_mps[2],
    );

    Ok(TwoImpulseTransferPlan {
        success: true,
        transfer_duration_s,
        transfer_duration_min: transfer_duration_s / 60.0,
        dv1: RpoManeuverDto {
            time_s: 0.0,
            delta_v_mps: dv1_vec,
            magnitude_mps: dv1_mag,
            description: "Departure Injection Burn".to_string(),
        },
        dv2: RpoManeuverDto {
            time_s: transfer_duration_s,
            delta_v_mps: dv2_vec,
            magnitude_mps: dv2_mag,
            description: "Arrival Braking & Insertion Burn".to_string(),
        },
        total_delta_v_mps: dv1_mag + dv2_mag,
        initial_state: *initial_state,
        final_state: final_arrival_state,
        trajectory_points,
    })
}

/// Plans a Natural Motion Circumnavigation (NMC) passive inspection orbit.
///
/// Configures a closed, periodic elliptical relative orbit centered on the target satellite
/// requiring **zero propellant** for continuous 360-degree inspection:
///
/// In-plane drift-free condition:
/// $$\dot{y}_0 + 2 \omega x_0 = 0 \iff \dot{y}_0 = -2 \omega x_0$$
///
/// Produces a $2:1$ aspect ratio relative ellipse in the $x-y$ plane:
/// $$x(t) = -A \cos(\omega t + \alpha), \quad y(t) = 2 A \sin(\omega t + \alpha), \quad z(t) = B \cos(\omega t + \beta)$$
pub fn plan_natural_motion_circumnavigation(
    target_orbit: &TargetOrbit,
    current_state: Option<&RelativeState>,
    radial_amplitude_m: f64,
    cross_track_amplitude_m: f64,
    phase_rad: f64,
    num_trajectory_points: usize,
) -> Result<NmcInspectionPlan, RpoError> {
    let omega = target_orbit.mean_motion_rad_s;
    if omega <= 0.0 {
        return Err(RpoError::ZeroTargetMeanMotion);
    }

    let a_rad = radial_amplitude_m.abs();
    let b_cross = cross_track_amplitude_m.abs();
    let a_in_track = 2.0 * a_rad;

    // Evaluate initial drift-free state at epoch t = 0
    let x0 = -a_rad * phase_rad.cos();
    let y0 = a_in_track * phase_rad.sin();
    let z0 = b_cross * phase_rad.cos();

    let vx0 = a_rad * omega * phase_rad.sin();
    let vy0 = -2.0 * omega * x0; // Exact zero-drift condition: vy0 = 2 * omega * A * cos(phase)
    let vz0 = -b_cross * omega * phase_rad.sin();

    let nmc_state = RelativeState::new(x0, y0, z0, vx0, vy0, vz0);

    // If chaser is currently at an arbitrary state, compute insertion maneuver
    let insertion_burn = if let Some(chaser) = current_state {
        let dv = [
            vx0 - chaser.vx,
            vy0 - chaser.vy,
            vz0 - chaser.vz,
        ];
        let mag = (dv[0] * dv[0] + dv[1] * dv[1] + dv[2] * dv[2]).sqrt();
        RpoManeuverDto {
            time_s: 0.0,
            delta_v_mps: dv,
            magnitude_mps: mag,
            description: "NMC Insertion Maneuver".to_string(),
        }
    } else {
        RpoManeuverDto {
            time_s: 0.0,
            delta_v_mps: [0.0, 0.0, 0.0],
            magnitude_mps: 0.0,
            description: "NMC Nominal Injection".to_string(),
        }
    };

    let period = target_orbit.period_s;
    let n_pts = num_trajectory_points.max(60);
    let mut trajectory_points = Vec::with_capacity(n_pts);

    for step in 0..n_pts {
        let t_sec = period * (step as f64) / ((n_pts - 1) as f64);
        let s_t = propagate_cw(&nmc_state, t_sec, omega);
        trajectory_points.push([t_sec, s_t.x, s_t.y, s_t.z]);
    }

    Ok(NmcInspectionPlan {
        success: true,
        radial_amplitude_m: a_rad,
        along_track_amplitude_m: a_in_track,
        cross_track_amplitude_m: b_cross,
        period_s: period,
        insertion_maneuver: insertion_burn,
        initial_drift_free_state: nmc_state,
        trajectory_points,
    })
}

/// Plans a multi-hop or direct V-bar (along-track) glideslope approach.
///
/// Chaser approaches the target along the velocity vector ($\hat{\mathbf{y}}$).
pub fn plan_glideslope_vbar(
    target_orbit: &TargetOrbit,
    start_y_m: f64,
    end_y_m: f64,
    duration_s: f64,
    num_hops: usize,
) -> Result<GlideslopeApproachPlan, RpoError> {
    let hops = num_hops.max(1);
    let dt_per_hop = duration_s / (hops as f64);
    let dy_per_hop = (end_y_m - start_y_m) / (hops as f64);

    let mut burns = Vec::with_capacity(hops * 2);
    let mut trajectory_points = Vec::new();
    let mut current_state = RelativeState::new(0.0, start_y_m, 0.0, 0.0, 0.0, 0.0);
    let mut total_dv = 0.0;

    for h in 0..hops {
        let target_y = start_y_m + dy_per_hop * ((h + 1) as f64);
        let plan = plan_two_impulse_transfer(
            &current_state,
            [0.0, target_y, 0.0],
            [0.0, 0.0, 0.0],
            dt_per_hop,
            target_orbit,
            25,
        )?;

        total_dv += plan.total_delta_v_mps;
        burns.push(RpoManeuverDto {
            time_s: h as f64 * dt_per_hop,
            delta_v_mps: plan.dv1.delta_v_mps,
            magnitude_mps: plan.dv1.magnitude_mps,
            description: format!("V-Bar Hop #{} Departure", h + 1),
        });
        burns.push(RpoManeuverDto {
            time_s: (h + 1) as f64 * dt_per_hop,
            delta_v_mps: plan.dv2.delta_v_mps,
            magnitude_mps: plan.dv2.magnitude_mps,
            description: format!("V-Bar Hop #{} Braking & Hold", h + 1),
        });

        // Append trajectory
        for pt in plan.trajectory_points {
            trajectory_points.push([pt[0] + h as f64 * dt_per_hop, pt[1], pt[2], pt[3]]);
        }

        current_state = plan.final_state;
    }

    Ok(GlideslopeApproachPlan {
        success: true,
        approach_type: "V-Bar (Along-Track)".to_string(),
        start_distance_m: start_y_m,
        end_distance_m: end_y_m,
        duration_s,
        total_delta_v_mps: total_dv,
        burns,
        trajectory_points,
    })
}

/// Plans an R-bar (radial) glideslope approach from below (Earth side) to target.
///
/// Benefits from Earth gravity gradient deceleration, providing **passive abort safety**:
/// if thrusters shut down, the chaser naturally falls away and back from the target.
pub fn plan_glideslope_rbar(
    target_orbit: &TargetOrbit,
    start_x_m: f64,
    end_x_m: f64,
    duration_s: f64,
) -> Result<GlideslopeApproachPlan, RpoError> {
    let initial_state = RelativeState::new(start_x_m, 0.0, 0.0, 0.0, 0.0, 0.0);
    let plan = plan_two_impulse_transfer(
        &initial_state,
        [end_x_m, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        duration_s,
        target_orbit,
        50,
    )?;

    let burns = vec![
        RpoManeuverDto {
            time_s: 0.0,
            delta_v_mps: plan.dv1.delta_v_mps,
            magnitude_mps: plan.dv1.magnitude_mps,
            description: "R-Bar Injection Burn (from below)".to_string(),
        },
        RpoManeuverDto {
            time_s: duration_s,
            delta_v_mps: plan.dv2.delta_v_mps,
            magnitude_mps: plan.dv2.magnitude_mps,
            description: "R-Bar Terminal Braking & Stationkeeping".to_string(),
        },
    ];

    Ok(GlideslopeApproachPlan {
        success: true,
        approach_type: "R-Bar (Radial Fail-Safe)".to_string(),
        start_distance_m: start_x_m,
        end_distance_m: end_x_m,
        duration_s,
        total_delta_v_mps: plan.total_delta_v_mps,
        burns,
        trajectory_points: plan.trajectory_points,
    })
}
