//! High-precision gravitational dynamics, relativistic post-Newtonian equations, and conservation laws.

use core::f64::consts::PI;
use crate::nbody::types::{
    CelestialBody, ConservationMetrics, NBodySystem, OsculatingElements, ResonanceMetrics,
    SPEED_OF_LIGHT,
};

/// Computes Cartesian gravitational acceleration vectors for all bodies in the system.
///
/// Includes pairwise Newtonian interactions and optional 1PN General Relativistic
/// post-Newtonian corrections.
pub fn compute_accelerations(
    bodies: &[CelestialBody],
    g: f64,
    softening_m: f64,
    enable_gr: bool,
) -> Vec<[f64; 3]> {
    let n = bodies.len();
    let mut accels = vec![[0.0, 0.0, 0.0]; n];
    let eps2 = softening_m * softening_m;
    let c2 = SPEED_OF_LIGHT * SPEED_OF_LIGHT;

    for i in 0..n {
        if bodies[i].is_fixed {
            continue;
        }

        let pi = bodies[i].position_m;
        let vi = bodies[i].velocity_mps;
        let vi_sq = vi[0] * vi[0] + vi[1] * vi[1] + vi[2] * vi[2];

        for (j, other) in bodies.iter().enumerate() {
            if i == j {
                continue;
            }

            let pj = other.position_m;
            let dx = pj[0] - pi[0];
            let dy = pj[1] - pi[1];
            let dz = pj[2] - pi[2];
            let r2 = dx * dx + dy * dy + dz * dz;
            let dist_soft_sq = r2 + eps2;
            let dist = dist_soft_sq.sqrt();

            if dist <= 1e-12 {
                continue;
            }

            // Newtonian gravity: a = G * M_j / dist_soft^3 * r_ij
            let denom = dist_soft_sq * dist;
            let gm_j = g * other.mass_kg;
            let factor = gm_j / denom;

            accels[i][0] += factor * dx;
            accels[i][1] += factor * dy;
            accels[i][2] += factor * dz;

            // 1PN General Relativistic correction (Schwarzschild / EIH post-Newtonian):
            // a_1PN = (G * M_j / (c^2 * r^3)) * [(4 * G * M_j / r - v_i^2) * r_ij + 4 * (r_ij . v_i) * v_i]
            if enable_gr && other.mass_kg > 0.0 {
                let r = r2.sqrt();
                if r > 1e-9 {
                    let rdotv = dx * vi[0] + dy * vi[1] + dz * vi[2];
                    let r3 = r * r * r;
                    let gr_factor = gm_j / (c2 * r3);
                    let term_pos = 4.0 * gm_j / r - vi_sq;

                    accels[i][0] += gr_factor * (term_pos * dx + 4.0 * rdotv * vi[0]);
                    accels[i][1] += gr_factor * (term_pos * dy + 4.0 * rdotv * vi[1]);
                    accels[i][2] += gr_factor * (term_pos * dz + 4.0 * rdotv * vi[2]);
                }
            }
        }
    }

    accels
}

/// Computes gravitational jerk (time derivative of acceleration $\dot{\mathbf{a}}$) for Hermite integration.
pub fn compute_jerks(bodies: &[CelestialBody], g: f64, softening_m: f64) -> Vec<[f64; 3]> {
    let n = bodies.len();
    let mut jerks = vec![[0.0, 0.0, 0.0]; n];
    let eps2 = softening_m * softening_m;

    for i in 0..n {
        if bodies[i].is_fixed {
            continue;
        }

        let pi = bodies[i].position_m;
        let vi = bodies[i].velocity_mps;

        for (j, other) in bodies.iter().enumerate() {
            if i == j {
                continue;
            }

            let pj = other.position_m;
            let vj = other.velocity_mps;

            let rx = pj[0] - pi[0];
            let ry = pj[1] - pi[1];
            let rz = pj[2] - pi[2];

            let vx = vj[0] - vi[0];
            let vy = vj[1] - vi[1];
            let vz = vj[2] - vi[2];

            let r2 = rx * rx + ry * ry + rz * rz;
            let dist_soft_sq = r2 + eps2;
            let dist = dist_soft_sq.sqrt();

            if dist <= 1e-12 {
                continue;
            }

            let gm_j = g * other.mass_kg;
            let rdotv = rx * vx + ry * vy + rz * vz;
            let inv_r3 = 1.0 / (dist_soft_sq * dist);
            let inv_r5 = 1.0 / (dist_soft_sq * dist_soft_sq * dist);

            jerks[i][0] += gm_j * (vx * inv_r3 - 3.0 * rdotv * rx * inv_r5);
            jerks[i][1] += gm_j * (vy * inv_r3 - 3.0 * rdotv * ry * inv_r5);
            jerks[i][2] += gm_j * (vz * inv_r3 - 3.0 * rdotv * rz * inv_r5);
        }
    }

    jerks
}

/// Evaluates total mechanical energy, momentum, and barycentric state.
pub fn compute_conservation_metrics(system: &NBodySystem) -> ConservationMetrics {
    let bodies = &system.bodies;
    let n = bodies.len();
    let eps2 = system.softening_m * system.softening_m;

    let mut kinetic_energy_j = 0.0;
    let mut potential_energy_j = 0.0;
    let mut linear_momentum_kg_mps = [0.0, 0.0, 0.0];
    let mut angular_momentum_kg_m2_s = [0.0, 0.0, 0.0];
    let mut total_mass = 0.0;
    let mut weighted_pos = [0.0, 0.0, 0.0];
    let mut weighted_vel = [0.0, 0.0, 0.0];

    for i in 0..n {
        let m = bodies[i].mass_kg;
        total_mass += m;

        let v = bodies[i].velocity_mps;
        let v2 = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
        kinetic_energy_j += 0.5 * m * v2;

        linear_momentum_kg_mps[0] += m * v[0];
        linear_momentum_kg_mps[1] += m * v[1];
        linear_momentum_kg_mps[2] += m * v[2];

        let r = bodies[i].position_m;
        weighted_pos[0] += m * r[0];
        weighted_pos[1] += m * r[1];
        weighted_pos[2] += m * r[2];

        weighted_vel[0] += m * v[0];
        weighted_vel[1] += m * v[1];
        weighted_vel[2] += m * v[2];

        // Angular momentum: L = r x (m * v)
        let lx = m * (r[1] * v[2] - r[2] * v[1]);
        let ly = m * (r[2] * v[0] - r[0] * v[2]);
        let lz = m * (r[0] * v[1] - r[1] * v[0]);
        angular_momentum_kg_m2_s[0] += lx;
        angular_momentum_kg_m2_s[1] += ly;
        angular_momentum_kg_m2_s[2] += lz;

        // Gravitational potential energy: - sum_{i < j} G * m_i * m_j / r_ij
        for other in bodies.iter().skip(i + 1) {
            let dx = other.position_m[0] - r[0];
            let dy = other.position_m[1] - r[1];
            let dz = other.position_m[2] - r[2];
            let dist = (dx * dx + dy * dy + dz * dz + eps2).sqrt();
            if dist > 1e-12 {
                potential_energy_j -= system.gravitational_constant * m * other.mass_kg / dist;
            }
        }
    }

    let total_energy_j = kinetic_energy_j + potential_energy_j;
    let initial_energy = system.initial_energy_j.unwrap_or(total_energy_j);
    let relative_energy_error = if initial_energy.abs() > 1e-15 {
        ((total_energy_j - initial_energy) / initial_energy).abs()
    } else {
        0.0
    };

    let p_mag = (linear_momentum_kg_mps[0].powi(2)
        + linear_momentum_kg_mps[1].powi(2)
        + linear_momentum_kg_mps[2].powi(2))
    .sqrt();

    let l_mag = (angular_momentum_kg_m2_s[0].powi(2)
        + angular_momentum_kg_m2_s[1].powi(2)
        + angular_momentum_kg_m2_s[2].powi(2))
    .sqrt();

    let initial_l = system.initial_angular_momentum_mag.unwrap_or(l_mag);
    let relative_angular_momentum_error = if initial_l > 1e-15 {
        ((l_mag - initial_l) / initial_l).abs()
    } else {
        0.0
    };

    let barycenter_position_m = if total_mass > 1e-15 {
        [
            weighted_pos[0] / total_mass,
            weighted_pos[1] / total_mass,
            weighted_pos[2] / total_mass,
        ]
    } else {
        [0.0, 0.0, 0.0]
    };

    let barycenter_velocity_mps = if total_mass > 1e-15 {
        [
            weighted_vel[0] / total_mass,
            weighted_vel[1] / total_mass,
            weighted_vel[2] / total_mass,
        ]
    } else {
        [0.0, 0.0, 0.0]
    };

    ConservationMetrics {
        kinetic_energy_j,
        potential_energy_j,
        total_energy_j,
        relative_energy_error,
        linear_momentum_kg_mps,
        linear_momentum_magnitude: p_mag,
        angular_momentum_kg_m2_s,
        angular_momentum_magnitude: l_mag,
        relative_angular_momentum_error,
        barycenter_position_m,
        barycenter_velocity_mps,
    }
}

/// Converts Cartesian state vectors $[x, y, z, v_x, v_y, v_z]$ relative to a primary body
/// into classical osculating Keplerian orbital elements $(a, e, i, \Omega, \omega, \nu)$.
pub fn extract_osculating_elements(
    body: &CelestialBody,
    primary: &CelestialBody,
    g: f64,
) -> Option<OsculatingElements> {
    let mu = g * (primary.mass_kg + body.mass_kg);
    if mu <= 1e-12 {
        return None;
    }

    let rx = body.position_m[0] - primary.position_m[0];
    let ry = body.position_m[1] - primary.position_m[1];
    let rz = body.position_m[2] - primary.position_m[2];

    let vx = body.velocity_mps[0] - primary.velocity_mps[0];
    let vy = body.velocity_mps[1] - primary.velocity_mps[1];
    let vz = body.velocity_mps[2] - primary.velocity_mps[2];

    let r = (rx * rx + ry * ry + rz * rz).sqrt();
    let v2 = vx * vx + vy * vy + vz * vz;

    if r <= 1e-12 {
        return None;
    }

    // Specific angular momentum h = r x v
    let hx = ry * vz - rz * vy;
    let hy = rz * vx - rx * vz;
    let hz = rx * vy - ry * vx;
    let h = (hx * hx + hy * hy + hz * hz).sqrt();

    if h <= 1e-12 {
        return None;
    }

    // Specific orbital energy E = v^2/2 - mu/r
    let energy = 0.5 * v2 - mu / r;

    // Semi-major axis a = -mu / (2 * E)
    let a = if energy.abs() > 1e-15 {
        -mu / (2.0 * energy)
    } else {
        f64::INFINITY
    };

    // Node vector n = k x h = [-hy, hx, 0]
    let nx = -hy;
    let ny = hx;
    let n = (nx * nx + ny * ny).sqrt();

    // Eccentricity vector e = ((v^2 - mu/r) * r - (r.v) * v) / mu
    let rdotv = rx * vx + ry * vy + rz * vz;
    let ex = ((v2 - mu / r) * rx - rdotv * vx) / mu;
    let ey = ((v2 - mu / r) * ry - rdotv * vy) / mu;
    let ez = ((v2 - mu / r) * rz - rdotv * vz) / mu;
    let e = (ex * ex + ey * ey + ez * ez).sqrt();

    // Inclination i = acos(hz / h)
    let inc = (hz / h).clamp(-1.0, 1.0).acos();

    // Longitude of ascending node Omega (RAAN)
    let raan = if n > 1e-12 {
        let mut val = ny.atan2(nx);
        if val < 0.0 {
            val += 2.0 * PI;
        }
        val
    } else {
        0.0
    };

    // Argument of periapsis omega
    let arg_periapsis = if n > 1e-12 && e > 1e-12 {
        let ndote = (nx * ex + ny * ey) / (n * e);
        let mut val = ndote.clamp(-1.0, 1.0).acos();
        if ez < 0.0 {
            val = 2.0 * PI - val;
        }
        val
    } else if e > 1e-12 {
        // Equatorial orbit
        let mut val = ey.atan2(ex);
        if val < 0.0 {
            val += 2.0 * PI;
        }
        val
    } else {
        0.0
    };

    // True anomaly nu
    let true_anomaly = if e > 1e-12 {
        let edotr = (ex * rx + ey * ry + ez * rz) / (e * r);
        let mut val = edotr.clamp(-1.0, 1.0).acos();
        if rdotv < 0.0 {
            val = 2.0 * PI - val;
        }
        val
    } else if n > 1e-12 {
        let ndotr = (nx * rx + ny * ry) / (n * r);
        let mut val = ndotr.clamp(-1.0, 1.0).acos();
        if rz < 0.0 {
            val = 2.0 * PI - val;
        }
        val
    } else {
        let mut val = ry.atan2(rx);
        if val < 0.0 {
            val += 2.0 * PI;
        }
        val
    };

    // Mean anomaly M
    let mean_anomaly = if e < 1.0 {
        // Eccentric anomaly E = 2 * atan(sqrt((1-e)/(1+e)) * tan(nu/2))
        let sin_half_nu = (true_anomaly * 0.5).sin();
        let cos_half_nu = (true_anomaly * 0.5).cos();
        let tan_half_e = ((1.0 - e) / (1.0 + e)).sqrt() * (sin_half_nu / cos_half_nu);
        let ecc_anomaly = 2.0 * tan_half_e.atan();
        let mut m = ecc_anomaly - e * ecc_anomaly.sin();
        if m < 0.0 {
            m += 2.0 * PI;
        }
        m
    } else {
        0.0
    };

    let period_s = if a > 0.0 && a.is_finite() {
        2.0 * PI * (a.powi(3) / mu).sqrt()
    } else {
        f64::INFINITY
    };

    let periapsis_m = a * (1.0 - e);
    let apoapsis_m = a * (1.0 + e);

    Some(OsculatingElements {
        semi_major_axis_m: a,
        eccentricity: e,
        inclination_rad: inc,
        raan_rad: raan,
        arg_periapsis_rad: arg_periapsis,
        true_anomaly_rad: true_anomaly,
        mean_anomaly_rad: mean_anomaly,
        period_s,
        periapsis_m,
        apoapsis_m,
    })
}

/// Evaluates the Jovian Laplace orbital resonance angle:
/// $\phi_L = \lambda_{\text{Io}} - 3\lambda_{\text{Europa}} + 2\lambda_{\text{Ganymede}}$.
///
/// In exact resonance, this angle librates tightly around $180^\circ$.
pub fn compute_laplace_resonance_metrics(bodies: &[CelestialBody]) -> Option<ResonanceMetrics> {
    // Locate Jupiter, Io, Europa, Ganymede
    let jupiter = bodies.iter().find(|b| b.name.eq_ignore_ascii_case("jupiter"))?;
    let io = bodies.iter().find(|b| b.name.eq_ignore_ascii_case("io"))?;
    let europa = bodies.iter().find(|b| b.name.eq_ignore_ascii_case("europa"))?;
    let ganymede = bodies.iter().find(|b| b.name.eq_ignore_ascii_case("ganymede"))?;

    // Longitudinal polar angle in orbital plane relative to Jupiter
    let lambda = |b: &CelestialBody| -> f64 {
        let dx = b.position_m[0] - jupiter.position_m[0];
        let dy = b.position_m[1] - jupiter.position_m[1];
        let mut angle = dy.atan2(dx);
        if angle < 0.0 {
            angle += 2.0 * PI;
        }
        angle
    };

    let l1 = lambda(io);
    let l2 = lambda(europa);
    let l3 = lambda(ganymede);

    // phi_L = lambda_1 - 3*lambda_2 + 2*lambda_3
    let mut phi_rad = l1 - 3.0 * l2 + 2.0 * l3;
    // Normalize to [0, 2*pi)
    phi_rad = phi_rad.rem_euclid(2.0 * PI);
    let phi_deg = phi_rad.to_degrees();

    // Ratio of orbital frequencies
    let d1 = io.distance_from_origin();
    let d2 = europa.distance_from_origin();
    let ratio = if d1 > 1e-6 { (d2 / d1).powf(1.5) } else { 2.0 };

    Some(ResonanceMetrics {
        name: "Laplace Resonance (Jovian System)".to_string(),
        ratio_desc: "Io:Europa:Ganymede (4:2:1)".to_string(),
        resonant_angle_deg: phi_deg,
        frequency_ratio: ratio,
    })
}

/// Computes the angular separation of a Trojan asteroid from the primary-secondary axis
/// to evaluate libration around the $L_4$ ($+60^\circ$) or $L_5$ ($-60^\circ$) Lagrange points.
pub fn compute_trojan_libration_deg(
    primary: &CelestialBody,
    secondary: &CelestialBody,
    trojan: &CelestialBody,
) -> f64 {
    let px = secondary.position_m[0] - primary.position_m[0];
    let py = secondary.position_m[1] - primary.position_m[1];
    let theta_sec = py.atan2(px);

    let tx = trojan.position_m[0] - primary.position_m[0];
    let ty = trojan.position_m[1] - primary.position_m[1];
    let theta_trojan = ty.atan2(tx);

    let mut diff = (theta_trojan - theta_sec).to_degrees();
    while diff > 180.0 {
        diff -= 360.0;
    }
    while diff < -180.0 {
        diff += 360.0;
    }
    diff
}
