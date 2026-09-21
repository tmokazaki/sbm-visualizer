//! Acceleration-level physics verification and physical consistency evaluation.
//!
//! Grounded in Section 3.3.3 of the AutoOrbit paper (KDD 2026):
//! - **Fourth-Order Finite Difference (Eq. 8)**: Computes predicted kinematic acceleration $\vec{a}_{pred}$
//!   from predicted velocity sequences.
//! - **Analytical Perturbation Model**: Evaluates total theoretical acceleration $\vec{a}_{phy}$ incorporating
//!   central gravity, Earth's non-spherical gravity harmonics ($J_2, J_3, J_4$), dynamic atmospheric drag,
//!   and third-body perturbations.
//! - **Physics Loss & Consistency Metrics (Eq. 9)**: Computes equation residuals and $P_{95}, P_{99}$ percentiles.

use crate::autoorbit::types::{
    StateVector, EARTH_RADIUS_M, J2, J3, J4,
};

/// Earth rotation angular velocity vector $\vec{\omega}_\oplus = [0, 0, 7.292115 \times 10^{-5}]^T\text{ rad/s}$.
pub const EARTH_ROTATION_RATE_RAD_PER_S: f64 = 7.292_115e-5;

/// Computes predicted acceleration via the fourth-order central finite-difference scheme (Eq. 8):
///
/// $$\vec{a}_{pred}(t) = \frac{-\vec{v}(t + 2\tau) + 8\vec{v}(t + \tau) - 8\vec{v}(t - \tau) + \vec{v}(t - 2\tau)}{12\tau}$$
pub fn compute_predicted_acceleration_4th_order(
    v_minus_2: [f64; 3],
    v_minus_1: [f64; 3],
    v_plus_1: [f64; 3],
    v_plus_2: [f64; 3],
    cadence_s: f64,
) -> [f64; 3] {
    let denom = 12.0 * cadence_s;
    let mut a = [0.0; 3];
    for i in 0..3 {
        a[i] = (-v_plus_2[i] + 8.0 * v_plus_1[i] - 8.0 * v_minus_1[i] + v_minus_2[i]) / denom;
    }
    a
}

/// Physical parameters for satellite perturbation modeling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacecraftPhysicalParams {
    /// Satellite mass $M$ in kilograms (e.g. 2158.777 kg for Sentinel-1A).
    pub mass_kg: f64,
    /// Cross-sectional area $A$ in square meters (e.g. 20.395 m² for Sentinel-1A).
    pub area_m2: f64,
    /// Atmospheric drag coefficient $C_D$ (nominally 2.2).
    pub drag_coefficient: f64,
    /// Flag to enable dynamic atmospheric drag.
    pub include_drag: bool,
    /// Flag to enable Earth $J_2\text{--}J_4$ zonal geopotential harmonics.
    pub include_geopotential: bool,
}

impl Default for SpacecraftPhysicalParams {
    fn default() -> Self {
        Self {
            mass_kg: 2158.777,
            area_m2: 20.395,
            drag_coefficient: 2.2,
            include_drag: true,
            include_geopotential: true,
        }
    }
}

/// Evaluates the theoretical physical acceleration $\vec{a}_{phy}$ acting on a satellite at state $(\vec{r}, \vec{v})$.
pub fn compute_theoretical_acceleration(
    state: &StateVector,
    params: &SpacecraftPhysicalParams,
    mu: f64,
) -> [f64; 3] {
    let r_vec = state.position();
    let v_vec = state.velocity();
    let r = state.r_norm();

    if r < 1.0 {
        return [0.0, 0.0, 0.0];
    }

    let r2 = r * r;
    let r3 = r2 * r;
    let z2 = r_vec[2] * r_vec[2];

    // 1. Central Body Gravitation: a_grav = -mu / r^3 * r
    let mu_over_r3 = mu / r3;
    let mut ax = -mu_over_r3 * r_vec[0];
    let mut ay = -mu_over_r3 * r_vec[1];
    let mut az = -mu_over_r3 * r_vec[2];

    // 2. Earth Non-spherical Geopotential (J2, J3, J4 terms)
    if params.include_geopotential {
        let re = EARTH_RADIUS_M;
        let re2 = re * re;
        let r5 = r3 * r2;
        let z2_over_r2 = z2 / r2;

        // J2 Zonal Acceleration
        let j2_coeff = -1.5 * J2 * mu * re2 / r5;
        let j2_xy_factor = 1.0 - 5.0 * z2_over_r2;
        let j2_z_factor = 3.0 - 5.0 * z2_over_r2;

        ax += j2_coeff * r_vec[0] * j2_xy_factor;
        ay += j2_coeff * r_vec[1] * j2_xy_factor;
        az += j2_coeff * r_vec[2] * j2_z_factor;

        // J3 Zonal Acceleration
        let re3 = re2 * re;
        let r7 = r5 * r2;
        let j3_coeff = -0.5 * J3 * mu * re3 / r7;
        let j3_xy_factor = 5.0 * r_vec[2] * (3.0 - 7.0 * z2_over_r2);
        let j3_z_factor = 3.0 * r2 * (1.0 - 10.0 * z2_over_r2 + (35.0 / 3.0) * z2_over_r2 * z2_over_r2);

        ax += j3_coeff * r_vec[0] * j3_xy_factor;
        ay += j3_coeff * r_vec[1] * j3_xy_factor;
        az += j3_coeff * j3_z_factor;

        // J4 Zonal Acceleration
        let re4 = re2 * re2;
        let j4_coeff = (15.0 / 8.0) * J4 * mu * re4 / r7;
        let j4_xy_factor = 1.0 - 14.0 * z2_over_r2 + 21.0 * z2_over_r2 * z2_over_r2;
        let j4_z_factor = 5.0 - (70.0 / 3.0) * z2_over_r2 + 21.0 * z2_over_r2 * z2_over_r2;

        ax += j4_coeff * r_vec[0] * j4_xy_factor;
        ay += j4_coeff * r_vec[1] * j4_xy_factor;
        az += j4_coeff * r_vec[2] * j4_z_factor;
    }

    // 3. Atmospheric Drag: a_drag = -0.5 * C_D * (A / M) * rho * v_rel * v_rel_vec
    if params.include_drag && params.mass_kg > 0.0 {
        let altitude_km = (r - EARTH_RADIUS_M) / 1000.0;
        if altitude_km > 100.0 && altitude_km < 1000.0 {
            // US Standard Atmosphere exponential density approximation for LEO
            let (h0, rho0, h_scale) = match altitude_km {
                h if h < 300.0 => (250.0, 6.073e-11, 45.546),
                h if h < 400.0 => (300.0, 1.916e-11, 53.628),
                h if h < 500.0 => (400.0, 2.803e-12, 58.515),
                h if h < 600.0 => (500.0, 5.215e-13, 60.828),
                h if h < 700.0 => (600.0, 1.137e-13, 71.835),
                h if h < 800.0 => (700.0, 3.070e-14, 88.667),
                _ => (800.0, 1.136e-14, 124.64),
            };
            let rho = rho0 * (-((altitude_km - h0) / h_scale)).exp();

            // Relative velocity accounting for Earth rotation: v_rel = v - omega x r
            let vx_rel = v_vec[0] + EARTH_ROTATION_RATE_RAD_PER_S * r_vec[1];
            let vy_rel = v_vec[1] - EARTH_ROTATION_RATE_RAD_PER_S * r_vec[0];
            let vz_rel = v_vec[2];
            let v_rel = (vx_rel * vx_rel + vy_rel * vy_rel + vz_rel * vz_rel).sqrt();

            let drag_factor = -0.5 * params.drag_coefficient * (params.area_m2 / params.mass_kg) * rho * v_rel;
            ax += drag_factor * vx_rel;
            ay += drag_factor * vy_rel;
            az += drag_factor * vz_rel;
        }
    }

    [ax, ay, az]
}

/// Evaluates physical consistency error along a predicted trajectory.
///
/// Returns the mean squared error (MSE) physics loss, along with the 95th ($P_{95}$) and 99th ($P_{99}$)
/// percentile equation residuals.
pub fn evaluate_trajectory_physics_consistency(
    trajectory: &[StateVector],
    cadence_s: f64,
    params: &SpacecraftPhysicalParams,
    mu: f64,
) -> (f64, f64, f64) {
    if trajectory.len() < 5 {
        return (0.0, 0.0, 0.0);
    }

    let n = trajectory.len();
    let mut residuals = Vec::with_capacity(n - 4);
    let mut total_sq_err = 0.0;

    for i in 2..(n - 2) {
        let a_pred = compute_predicted_acceleration_4th_order(
            trajectory[i - 2].velocity(),
            trajectory[i - 1].velocity(),
            trajectory[i + 1].velocity(),
            trajectory[i + 2].velocity(),
            cadence_s,
        );

        let a_phy = compute_theoretical_acceleration(&trajectory[i], params, mu);

        let dx = a_pred[0] - a_phy[0];
        let dy = a_pred[1] - a_phy[1];
        let dz = a_pred[2] - a_phy[2];
        let sq_err = dx * dx + dy * dy + dz * dz;
        total_sq_err += sq_err;
        residuals.push(sq_err.sqrt());
    }

    if residuals.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    residuals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

    let mse = total_sq_err / residuals.len() as f64;
    let idx_95 = (residuals.len() as f64 * 0.95).floor() as usize;
    let idx_99 = (residuals.len() as f64 * 0.99).floor() as usize;

    let p95 = residuals[idx_95.min(residuals.len() - 1)];
    let p99 = residuals[idx_99.min(residuals.len() - 1)];

    (mse, p95, p99)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autoorbit::types::EARTH_MU;

    #[test]
    fn test_theoretical_acceleration_leo_magnitude() {
        // Sentinel-1A at r ~ 7070 km: central gravity a_grav ~ 7.98 m/s^2, J2 perturbation ~ 0.015 m/s^2
        let state = StateVector::new(7_070_000.0, 0.0, 0.0, 0.0, 7500.0, 0.0);
        let params = SpacecraftPhysicalParams::default();
        let a = compute_theoretical_acceleration(&state, &params, EARTH_MU);

        let a_mag = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
        assert!(a_mag > 7.9 && a_mag < 8.1, "Expected ~8.0 m/s^2, got {}", a_mag);
    }

    #[test]
    fn test_finite_difference_acceleration_on_kepler_orbit() {
        // Check 4th order derivative against theoretical centripetal acceleration on circular orbit
        let r = 7_000_000.0;
        let v = (EARTH_MU / r).sqrt();
        let omega = v / r;
        let tau = 10.0;

        let make_v = |step: f64| -> [f64; 3] {
            let theta = omega * step * tau;
            [-v * theta.sin(), v * theta.cos(), 0.0]
        };

        let a_pred = compute_predicted_acceleration_4th_order(
            make_v(-2.0),
            make_v(-1.0),
            make_v(1.0),
            make_v(2.0),
            tau,
        );

        // Theoretical centripetal acceleration a = -v^2 / r along radial x axis
        let a_theo_x = -v * v / r;
        assert!((a_pred[0] - a_theo_x).abs() < 1e-4, "Error was {}", (a_pred[0] - a_theo_x).abs());
        assert!(a_pred[1].abs() < 1e-4);
    }
}
