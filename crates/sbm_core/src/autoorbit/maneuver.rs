//! Real-time orbital maneuver correction via Gaussian Variational Equations (GVEs).
//!
//! Grounded in Section 3.4 of the AutoOrbit paper (KDD 2026):
//! - **Equations (10)–(12)**: Instantaneous jumps in Keplerian orbital elements $(\Delta a, \Delta e, \Delta \omega)$
//!   induced by impulsive thrust increments $\Delta \vec{v} = [\Delta v_r, \Delta v_a, \Delta v_c]^T$ in the RAC/RIC frame.
//! - **Analytical Keplerian Propagation**: Propagates modified elements across the prediction horizon with $O(H)$ complexity,
//!   eliminating numerical ODE integration overhead.

use crate::autoorbit::types::{AutoOrbitError, KeplerianElements, ManeuverImpulse, StateVector};

/// Converts a Cartesian state vector $(\vec{r}, \vec{v})$ in ECI coordinates to classical Keplerian elements.
pub fn cartesian_to_keplerian(
    state: &StateVector,
    mu: f64,
) -> Result<KeplerianElements, AutoOrbitError> {
    let r_vec = state.position();
    let v_vec = state.velocity();

    let r = state.r_norm();
    let v = state.v_norm();

    if r < 1.0 {
        return Err(AutoOrbitError::InvalidConfiguration(
            "Position radius cannot be near zero".to_string(),
        ));
    }

    // Specific angular momentum h = r x v
    let hx = r_vec[1] * v_vec[2] - r_vec[2] * v_vec[1];
    let hy = r_vec[2] * v_vec[0] - r_vec[0] * v_vec[2];
    let hz = r_vec[0] * v_vec[1] - r_vec[1] * v_vec[0];
    let h = (hx * hx + hy * hy + hz * hz).sqrt();

    if h < 1e-6 {
        return Err(AutoOrbitError::InvalidConfiguration(
            "Rectilinear motion (angular momentum h ~ 0)".to_string(),
        ));
    }

    // Node vector n = k x h = [-hy, hx, 0]
    let nx = -hy;
    let ny = hx;
    let n = (nx * nx + ny * ny).sqrt();

    // Eccentricity vector e_vec = ((v^2 - mu/r) * r - (r . v) * v) / mu
    let r_dot_v = r_vec[0] * v_vec[0] + r_vec[1] * v_vec[1] + r_vec[2] * v_vec[2];
    let v2_minus_mu_r = v * v - mu / r;

    let ex = (v2_minus_mu_r * r_vec[0] - r_dot_v * v_vec[0]) / mu;
    let ey = (v2_minus_mu_r * r_vec[1] - r_dot_v * v_vec[1]) / mu;
    let ez = (v2_minus_mu_r * r_vec[2] - r_dot_v * v_vec[2]) / mu;
    let e = (ex * ex + ey * ey + ez * ez).sqrt();

    // Semi-major axis a: 1/a = 2/r - v^2/mu
    let inv_a = 2.0 / r - (v * v) / mu;
    if inv_a <= 1e-12 || e >= 1.0 {
        let a = if inv_a.abs() > 1e-12 { 1.0 / inv_a } else { 0.0 };
        return Err(AutoOrbitError::NonEllipticOrbit {
            semi_major_axis: a,
            eccentricity: e,
        });
    }
    let a = 1.0 / inv_a;

    // Inclination i = acos(hz / h)
    let inc = (hz / h).clamp(-1.0, 1.0).acos();

    let pi = core::f64::consts::PI;
    let two_pi = 2.0 * pi;

    let is_equatorial = n <= 1e-10;
    let is_circular = e <= 1e-8;

    let (raan, omega, true_anomaly) = match (is_equatorial, is_circular) {
        (true, true) => {
            let lambda = r_vec[1].atan2(r_vec[0]).rem_euclid(two_pi);
            (0.0, 0.0, lambda)
        }
        (true, false) => {
            let varpi = ey.atan2(ex).rem_euclid(two_pi);
            let mut nu = ((ex * r_vec[0] + ey * r_vec[1]) / (e * r)).clamp(-1.0, 1.0).acos();
            if r_dot_v < 0.0 {
                nu = two_pi - nu;
            }
            (0.0, varpi, nu)
        }
        (false, true) => {
            let raan = ny.atan2(nx).rem_euclid(two_pi);
            let mut u = ((nx * r_vec[0] + ny * r_vec[1]) / (n * r)).clamp(-1.0, 1.0).acos();
            if r_vec[2] < 0.0 {
                u = two_pi - u;
            }
            (raan, 0.0, u)
        }
        (false, false) => {
            let raan = ny.atan2(nx).rem_euclid(two_pi);
            let n_dot_e = nx * ex + ny * ey;
            let mut w = (n_dot_e / (n * e)).clamp(-1.0, 1.0).acos();
            if ez < 0.0 {
                w = two_pi - w;
            }
            let e_dot_r = ex * r_vec[0] + ey * r_vec[1] + ez * r_vec[2];
            let mut nu = (e_dot_r / (e * r)).clamp(-1.0, 1.0).acos();
            if r_dot_v < 0.0 {
                nu = two_pi - nu;
            }
            (raan, w, nu)
        }
    };

    Ok(KeplerianElements::new(a, e, inc, raan, omega, true_anomaly))
}

/// Converts classical Keplerian orbital elements $(a, e, i, \Omega, \omega, \nu)$ back to Cartesian ECI state vector.
pub fn keplerian_to_cartesian(elements: &KeplerianElements, mu: f64) -> StateVector {
    let a = elements.semi_major_axis_m;
    let e = elements.eccentricity;
    let inc = elements.inclination_rad;
    let raan = elements.raan_rad;
    let omega = elements.arg_periapsis_rad;
    let nu = elements.true_anomaly_rad;

    let p = a * (1.0 - e * e);
    let r = p / (1.0 + e * nu.cos());

    // Position and velocity in the Perifocal (PQW) orbital plane frame
    let cos_nu = nu.cos();
    let sin_nu = nu.sin();
    let r_pqw = [r * cos_nu, r * sin_nu, 0.0];

    let sqrt_mu_p = (mu / p.max(1.0)).sqrt();
    let v_pqw = [-sqrt_mu_p * sin_nu, sqrt_mu_p * (e + cos_nu), 0.0];

    // Transformation from PQW to ECI: R = Rz(-Omega) * Rx(-inc) * Rz(-omega)
    let cos_o = raan.cos();
    let sin_o = raan.sin();
    let cos_i = inc.cos();
    let sin_i = inc.sin();
    let cos_w = omega.cos();
    let sin_w = omega.sin();

    let p11 = cos_o * cos_w - sin_o * sin_w * cos_i;
    let p12 = -cos_o * sin_w - sin_o * cos_w * cos_i;
    let p21 = sin_o * cos_w + cos_o * sin_w * cos_i;
    let p22 = -sin_o * sin_w + cos_o * cos_w * cos_i;
    let p31 = sin_w * sin_i;
    let p32 = cos_w * sin_i;

    let x = p11 * r_pqw[0] + p12 * r_pqw[1];
    let y = p21 * r_pqw[0] + p22 * r_pqw[1];
    let z = p31 * r_pqw[0] + p32 * r_pqw[1];

    let vx = p11 * v_pqw[0] + p12 * v_pqw[1];
    let vy = p21 * v_pqw[0] + p22 * v_pqw[1];
    let vz = p31 * v_pqw[0] + p32 * v_pqw[1];

    StateVector::new(x, y, z, vx, vy, vz)
}

/// Solves Kepler's transcendental equation $M = E - e \sin E$ for Eccentric Anomaly $E$ via Newton-Raphson.
pub fn solve_keplers_equation(
    mean_anomaly: f64,
    eccentricity: f64,
    tolerance: f64,
    max_iter: usize,
) -> Result<f64, AutoOrbitError> {
    let two_pi = 2.0 * core::f64::consts::PI;
    let m = ((mean_anomaly % two_pi) + two_pi) % two_pi;

    // Initial estimate for E
    let mut e_anom = if eccentricity < 0.8 {
        m
    } else {
        core::f64::consts::PI
    };

    for _iter in 0..max_iter {
        let f = e_anom - eccentricity * e_anom.sin() - m;
        if f.abs() < tolerance {
            return Ok(e_anom);
        }
        let f_prime = 1.0 - eccentricity * e_anom.cos();
        if f_prime.abs() < 1e-14 {
            break;
        }
        e_anom -= f / f_prime;
    }

    Err(AutoOrbitError::KeplerConvergenceFailed {
        mean_anomaly,
        iterations: max_iter,
    })
}

/// Computes instantaneous jumps in Keplerian orbital elements via Gaussian Variational Equations (GVEs).
///
/// Grounded in Equations (10)–(12) of Zhang et al. (2026):
///
/// $$\Delta a = \frac{2 a^2 v}{\mu} \left( e \sin\nu \cdot \Delta v_r + \frac{p}{r} \cdot \Delta v_a \right)$$
/// $$\Delta e = \frac{1}{v} [\sin\nu \cdot \Delta v_r + (e + \cos\nu) \cdot \Delta v_a]$$
/// $$\Delta \omega = \frac{1}{e v} [-\cos\nu \cdot \Delta v_r + (e + \cos\nu)\sin\nu \cdot \Delta v_a]$$
/// $$\Delta i = \frac{r \cos(\omega + \nu)}{h} \cdot \Delta v_c$$
/// $$\Delta \Omega = \frac{r \sin(\omega + \nu)}{h \sin i} \cdot \Delta v_c$$
pub fn compute_gve_element_deltas(
    elements: &KeplerianElements,
    impulse: &ManeuverImpulse,
    mu: f64,
) -> (f64, f64, f64, f64, f64) {
    let a = elements.semi_major_axis_m;
    let e = elements.eccentricity.max(1e-6); // Guard against zero division in arg of periapsis
    let inc = elements.inclination_rad;
    let omega = elements.arg_periapsis_rad;
    let nu = elements.true_anomaly_rad;

    let p = a * (1.0 - e * e);
    let r = elements.radius_m();
    let v = elements.speed_mps(mu).max(1.0);
    let h = (mu * p.max(1.0)).sqrt();

    let dvr = impulse.delta_vr;
    let dva = impulse.delta_va;
    let dvc = impulse.delta_vc;

    let sin_nu = nu.sin();
    let cos_nu = nu.cos();

    // Eq. 10: Semi-major axis variation Delta a
    let delta_a = (2.0 * a * a * v / mu) * (e * sin_nu * dvr + (p / r) * dva);

    // Eq. 11: Eccentricity variation Delta e
    let delta_e = (1.0 / v) * (sin_nu * dvr + (e + cos_nu) * dva);

    // Eq. 12: Argument of periapsis variation Delta omega
    let delta_omega = (1.0 / (e * v)) * (-cos_nu * dvr + (e + cos_nu) * sin_nu * dva);

    // Cross-track out-of-plane variations: Delta i and Delta Omega
    let u = omega + nu;
    let delta_inc = (r * u.cos() / h) * dvc;

    let sin_i = inc.sin().abs();
    let delta_raan = if sin_i > 1e-5 {
        (r * u.sin() / (h * sin_i)) * dvc
    } else {
        0.0
    };

    (delta_a, delta_e, delta_omega, delta_inc, delta_raan)
}

/// Converts a velocity increment in the Radial-Along-Track-Cross-Track (RAC) frame to ECI Cartesian coordinates.
pub fn rac_to_eci_delta_v(state: &StateVector, impulse: &ManeuverImpulse) -> [f64; 3] {
    let r_vec = state.position();
    let v_vec = state.velocity();
    let r = state.r_norm().max(1.0);

    // Radial unit vector r_hat = r / |r|
    let r_hat = [r_vec[0] / r, r_vec[1] / r, r_vec[2] / r];

    // Angular momentum vector h = r x v
    let hx = r_vec[1] * v_vec[2] - r_vec[2] * v_vec[1];
    let hy = r_vec[2] * v_vec[0] - r_vec[0] * v_vec[2];
    let hz = r_vec[0] * v_vec[1] - r_vec[1] * v_vec[0];
    let h = (hx * hx + hy * hy + hz * hz).sqrt().max(1e-6);

    // Cross-track unit vector c_hat = h / |h|
    let c_hat = [hx / h, hy / h, hz / h];

    // Along-track unit vector a_hat = c_hat x r_hat
    let a_hat = [
        c_hat[1] * r_hat[2] - c_hat[2] * r_hat[1],
        c_hat[2] * r_hat[0] - c_hat[0] * r_hat[2],
        c_hat[0] * r_hat[1] - c_hat[1] * r_hat[0],
    ];

    [
        impulse.delta_vr * r_hat[0] + impulse.delta_va * a_hat[0] + impulse.delta_vc * c_hat[0],
        impulse.delta_vr * r_hat[1] + impulse.delta_va * a_hat[1] + impulse.delta_vc * c_hat[1],
        impulse.delta_vr * r_hat[2] + impulse.delta_va * a_hat[2] + impulse.delta_vc * c_hat[2],
    ]
}

/// Applies real-time maneuver correction to an initial Cartesian state and analytically propagates
/// the post-maneuver trajectory across $H$ time steps at the given cadence $\tau$ (seconds).
pub fn apply_maneuver_correction(
    pre_state: &StateVector,
    impulse: &ManeuverImpulse,
    num_steps: usize,
    cadence_s: f64,
    mu: f64,
) -> Result<Vec<StateVector>, AutoOrbitError> {
    if num_steps == 0 {
        return Ok(Vec::new());
    }

    // 1. Transform RAC impulse into ECI velocity increment
    let dv_eci = rac_to_eci_delta_v(pre_state, impulse);

    // 2. Instantaneous post-maneuver state at maneuver epoch tm
    let post_state_0 = StateVector::new(
        pre_state.x,
        pre_state.y,
        pre_state.z,
        pre_state.vx + dv_eci[0],
        pre_state.vy + dv_eci[1],
        pre_state.vz + dv_eci[2],
    );

    // 3. Compute post-maneuver Keplerian elements
    let post_elements = cartesian_to_keplerian(&post_state_0, mu)?;
    let a = post_elements.semi_major_axis_m;
    let e = post_elements.eccentricity;
    let inc = post_elements.inclination_rad;
    let raan = post_elements.raan_rad;
    let omega = post_elements.arg_periapsis_rad;
    let nu0 = post_elements.true_anomaly_rad;

    // 4. Initial mean anomaly M0 at epoch
    let tan_half_nu = (nu0 * 0.5).tan();
    let inv_factor = ((1.0 - e) / (1.0 + e).max(1e-12)).sqrt();
    let mut e0 = 2.0 * (inv_factor * tan_half_nu).atan();
    if e0 < 0.0 {
        e0 += 2.0 * core::f64::consts::PI;
    }
    let m0 = e0 - e * e0.sin();

    // 5. Mean motion of post-maneuver orbit
    let n = (mu / a.powi(3)).sqrt();

    let mut trajectory = Vec::with_capacity(num_steps);
    trajectory.push(post_state_0);

    for step in 1..num_steps {
        let dt = step as f64 * cadence_s;
        let m_t = m0 + n * dt;
        let e_t = solve_keplers_equation(m_t, e, 1e-10, 50)?;

        let tan_half_e = (e_t * 0.5).tan();
        let factor = ((1.0 + e) / (1.0 - e).max(1e-12)).sqrt();
        let mut nu_t = 2.0 * (factor * tan_half_e).atan();
        if nu_t < 0.0 {
            nu_t += 2.0 * core::f64::consts::PI;
        }

        let updated_elements = KeplerianElements::new(a, e, inc, raan, omega, nu_t);
        let cart_state = keplerian_to_cartesian(&updated_elements, mu);
        trajectory.push(cart_state);
    }

    Ok(trajectory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autoorbit::types::EARTH_MU;

    #[test]
    fn test_cartesian_keplerian_roundtrip() {
        // Typical LEO orbit: r ~ 7000 km, v ~ 7.5 km/s
        let initial_state = StateVector::new(7_000_000.0, 0.0, 0.0, 0.0, 7546.053, 0.0);
        let elements = cartesian_to_keplerian(&initial_state, EARTH_MU).expect("Valid orbit");

        assert!((elements.semi_major_axis_m - 7_000_000.0).abs() < 10.0);
        assert!(elements.eccentricity < 0.001);

        let roundtrip_state = keplerian_to_cartesian(&elements, EARTH_MU);
        assert!((roundtrip_state.x - initial_state.x).abs() < 1e-4);
        assert!((roundtrip_state.vy - initial_state.vy).abs() < 1e-4);
    }

    #[test]
    fn test_gve_along_track_thrust_increases_semi_major_axis() {
        // Positive along-track impulse Delta va > 0 should raise the orbit's semi-major axis Delta a > 0
        let initial_state = StateVector::new(7_000_000.0, 0.0, 0.0, 0.0, 7546.053, 0.0);
        let elements = cartesian_to_keplerian(&initial_state, EARTH_MU).expect("Valid orbit");

        let impulse = ManeuverImpulse::new(0.0, 1.0, 0.0, 0.0); // +1.0 m/s along-track
        let (da, _, _, _, _) = compute_gve_element_deltas(&elements, &impulse, EARTH_MU);

        // Theoretical Delta a = 2 a^2 v / mu * Delta va = 2 * (7e6)^2 * 7546 / 3.986e14 * 1.0 ~ 1855 m
        assert!(da > 1800.0 && da < 1900.0, "da was {}", da);
    }

    #[test]
    fn test_maneuver_propagation_continuity() {
        let pre_state = StateVector::new(7_000_000.0, 0.0, 0.0, 0.0, 7546.053, 0.0);
        let impulse = ManeuverImpulse::new(0.0, 0.5, 0.0, 0.0);
        let traj = apply_maneuver_correction(&pre_state, &impulse, 60, 10.0, EARTH_MU)
            .expect("Propagation succeeds");

        assert_eq!(traj.len(), 60);
        // Post-maneuver velocity should immediately reflect velocity increment
        let initial_v = pre_state.v_norm();
        let post_v = traj[0].v_norm();
        assert!((post_v - (initial_v + 0.5)).abs() < 0.1);
    }
}
