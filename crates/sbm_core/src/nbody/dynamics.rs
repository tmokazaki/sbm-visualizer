//! High-precision gravitational dynamics, relativistic post-Newtonian equations, and conservation laws.

use core::f64::consts::PI;
use crate::nbody::types::{
    BodyFieldContribution, CelestialBody, CentricTestParticle, ConservationMetrics,
    GravitationalSphereRadii, NBodySystem, OsculatingElements, PairwiseForce, ResonanceMetrics,
    SpatialFieldPoint, TidalTensor, ASTRONOMICAL_UNIT_M, G_STANDARD, SPEED_OF_LIGHT,
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

/// Evaluates pairwise gravitational force contributions acting on a designated target body.
pub fn compute_pairwise_forces(system: &NBodySystem, focus_index: usize) -> Vec<PairwiseForce> {
    let n = system.bodies.len();
    if focus_index >= n {
        return Vec::new();
    }
    let target = &system.bodies[focus_index];
    let mut forces = Vec::new();
    let mut total_f = 0.0;
    let eps2 = system.softening_m * system.softening_m;

    for (j, body_j) in system.bodies.iter().enumerate() {
        if j == focus_index {
            continue;
        }
        let dx = body_j.position_m[0] - target.position_m[0];
        let dy = body_j.position_m[1] - target.position_m[1];
        let dz = body_j.position_m[2] - target.position_m[2];
        let r2 = dx * dx + dy * dy + dz * dz;
        let dist_soft_sq = r2 + eps2;
        let dist = dist_soft_sq.sqrt();
        if dist <= 1e-12 {
            continue;
        }
        let f_mag = (system.gravitational_constant * target.mass_kg * body_j.mass_kg) / dist_soft_sq;
        let fx = f_mag * (dx / dist);
        let fy = f_mag * (dy / dist);
        let fz = f_mag * (dz / dist);

        total_f += f_mag;
        forces.push(PairwiseForce {
            source_id: body_j.id,
            source_name: body_j.name.clone(),
            force_vector_n: [fx, fy, fz],
            magnitude_n: f_mag,
            fraction_of_total: 0.0,
        });
    }

    if total_f > 1e-15 {
        for f in &mut forces {
            f.fraction_of_total = f.magnitude_n / total_f;
        }
    }
    forces
}

/// Computes the planetary gravitational domains of dominance:
/// - Sphere of Attraction ($r_a = a \sqrt{m/M_\odot}$)
/// - Laplace Sphere of Influence ($r_s = a (m/M_\odot)^{2/5}$)
/// - Hill Sphere ($r_H = a(1-e) \sqrt[3]{m/(3M_\odot)}$)
/// - Domingos et al. (2006) Critical Satellite Stability Radius ($r_{\text{crit}} \approx 0.4895 r_H$).
pub fn compute_gravitational_spheres(
    body_mass_kg: f64,
    primary_mass_kg: f64,
    semimajor_axis_m: f64,
    eccentricity: f64,
    sat_eccentricity: f64,
) -> GravitationalSphereRadii {
    let mass_ratio = body_mass_kg / primary_mass_kg;
    let sphere_of_attraction_m = semimajor_axis_m * mass_ratio.sqrt();
    let laplace_soi_m = semimajor_axis_m * mass_ratio.powf(0.4);
    let hill_sphere_m = semimajor_axis_m * (1.0 - eccentricity) * (mass_ratio / 3.0).cbrt();
    let critical_stability_radius_m = 0.4895 * hill_sphere_m * (1.0 - 1.0305 * sat_eccentricity - 0.2738 * eccentricity);

    GravitationalSphereRadii {
        sphere_of_attraction_m,
        laplace_soi_m,
        hill_sphere_m,
        critical_stability_radius_m,
    }
}

/// Evaluates the gravitational tidal tensor (gravity gradient matrix) $\mathbf{T}_{ab} = \frac{\partial g_a}{\partial x_b}$.
///
/// In vacuum, $\nabla \cdot \mathbf{g} = 0$, guaranteeing $\text{Tr}(\mathbf{T}) = 0$.
/// Solves the cubic secular equation analytically for the principal eigenvalues (tidal strain axes).
pub fn compute_tidal_tensor(system: &NBodySystem, point_m: [f64; 3]) -> TidalTensor {
    let mut matrix = [[0.0; 3]; 3];
    let eps2 = system.softening_m * system.softening_m;

    for b in &system.bodies {
        let rx = b.position_m[0] - point_m[0];
        let ry = b.position_m[1] - point_m[1];
        let rz = b.position_m[2] - point_m[2];
        let r2 = rx * rx + ry * ry + rz * rz;
        let dist_soft_sq = r2 + eps2;
        let dist = dist_soft_sq.sqrt();
        if dist <= 1e-12 {
            continue;
        }
        let gm = system.gravitational_constant * b.mass_kg;
        let inv_r3 = 1.0 / (dist_soft_sq * dist);
        let inv_r5 = 1.0 / (dist_soft_sq * dist_soft_sq * dist);

        let r_vec = [rx, ry, rz];
        for a in 0..3 {
            for b_idx in 0..3 {
                let delta = if a == b_idx { 1.0 } else { 0.0 };
                matrix[a][b_idx] += gm * (3.0 * r_vec[a] * r_vec[b_idx] * inv_r5 - delta * inv_r3);
            }
        }
    }

    let trace = matrix[0][0] + matrix[1][1] + matrix[2][2];

    let sum_sq: f64 = matrix.iter().flat_map(|row| row.iter()).map(|&v| v * v).sum();
    let q = sum_sq / 6.0;

    let det = matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]);

    let mut eigenvalues = [0.0; 3];
    if q > 1e-30 {
        let r_val = det * 0.5;
        let arg = (r_val / q.powf(1.5)).clamp(-1.0, 1.0);
        let theta = arg.acos() / 3.0;
        let sqrt_q = q.sqrt();

        let e1 = 2.0 * sqrt_q * theta.cos();
        let e2 = 2.0 * sqrt_q * (theta - 2.0 * PI / 3.0).cos();
        let e3 = 2.0 * sqrt_q * (theta + 2.0 * PI / 3.0).cos();

        let mut e_arr = [e1, e2, e3];
        e_arr.sort_by(|a, b| b.partial_cmp(a).unwrap_or(core::cmp::Ordering::Equal));
        eigenvalues = e_arr;
    }

    let max_strain_eotvos = eigenvalues[0].abs().max(eigenvalues[2].abs()) * 1e9;

    TidalTensor {
        matrix,
        trace,
        eigenvalues,
        max_strain_eotvos,
    }
}

/// Evaluates the Earth-Moon Barycenter (EMB) position and displacement from Earth's center in meters.
pub fn compute_earth_moon_barycenter(system: &NBodySystem) -> Result<([f64; 3], f64), &'static str> {
    let earth = system.bodies.iter().find(|b| b.name.eq_ignore_ascii_case("earth"))
        .ok_or("Earth not found in system")?;
    let moon = system.bodies.iter().find(|b| b.name.eq_ignore_ascii_case("moon"))
        .ok_or("Moon not found in system")?;

    let total_mass = earth.mass_kg + moon.mass_kg;
    let emb = [
        (earth.position_m[0] * earth.mass_kg + moon.position_m[0] * moon.mass_kg) / total_mass,
        (earth.position_m[1] * earth.mass_kg + moon.position_m[1] * moon.mass_kg) / total_mass,
        (earth.position_m[2] * earth.mass_kg + moon.position_m[2] * moon.mass_kg) / total_mass,
    ];

    let dx = emb[0] - earth.position_m[0];
    let dy = emb[1] - earth.position_m[1];
    let dz = emb[2] - earth.position_m[2];
    let displacement_m = (dx * dx + dy * dy + dz * dz).sqrt();

    Ok((emb, displacement_m))
}

/// Evaluates the complete gravitational field state at an arbitrary spatial coordinate $\mathbf{r} = [x, y, z]$.
///
/// Computes:
/// - Net Newtonian gravitational acceleration vector $\mathbf{g}(\mathbf{r}) = \sum_{j=1}^N \frac{G m_j (\mathbf{r}_j - \mathbf{r})}{\|\mathbf{r}_j - \mathbf{r}\|^3}$
/// - Gravitational potential $\Phi(\mathbf{r}) = -\sum_{j=1}^N \frac{G m_j}{\|\mathbf{r}_j - \mathbf{r}\|}$
/// - Dominant gravitational body basin ($\arg\max_j \|\mathbf{g}_j(\mathbf{r})\|$)
/// - Individual body contributions (Tug-of-War breakdown at this spatial point)
/// - Gravitational tidal tensor $\mathbf{T}_{ab}(\mathbf{r}) = \partial g_a / \partial x_b$ and principal strain eigenvalues
pub fn compute_spatial_field_point(system: &NBodySystem, point_m: [f64; 3]) -> SpatialFieldPoint {
    let mut net_accel = [0.0, 0.0, 0.0];
    let mut total_potential = 0.0;
    let mut contributions = Vec::with_capacity(system.bodies.len());
    let mut scalar_sum = 0.0;
    let eps2 = system.softening_m * system.softening_m;

    let mut max_body_accel = -1.0;
    let mut dominant_id = 0;
    let mut dominant_name = String::new();

    for b in &system.bodies {
        let dx = b.position_m[0] - point_m[0];
        let dy = b.position_m[1] - point_m[1];
        let dz = b.position_m[2] - point_m[2];
        let r2 = dx * dx + dy * dy + dz * dz;
        let dist = r2.sqrt();
        let dist_soft_sq = r2 + eps2;
        let dist_soft = dist_soft_sq.sqrt();

        let (ax, ay, az, a_mag, phi) = if dist_soft > 1e-12 {
            let gm = system.gravitational_constant * b.mass_kg;
            let denom = dist_soft_sq * dist_soft;
            let factor = gm / denom;
            let ax = factor * dx;
            let ay = factor * dy;
            let az = factor * dz;
            let a_mag = (ax * ax + ay * ay + az * az).sqrt();
            let phi = -gm / dist_soft;
            (ax, ay, az, a_mag, phi)
        } else {
            (0.0, 0.0, 0.0, 0.0, 0.0)
        };

        net_accel[0] += ax;
        net_accel[1] += ay;
        net_accel[2] += az;
        total_potential += phi;
        scalar_sum += a_mag;

        if a_mag > max_body_accel {
            max_body_accel = a_mag;
            dominant_id = b.id;
            dominant_name = b.name.clone();
        }

        contributions.push(BodyFieldContribution {
            body_id: b.id,
            body_name: b.name.clone(),
            body_color: b.color_hex.clone(),
            acceleration_vector_mps2: [ax, ay, az],
            acceleration_magnitude: a_mag,
            gravitational_potential_j_kg: phi,
            fraction_of_total: 0.0,
            distance_m: dist,
        });
    }

    if scalar_sum > 1e-30 {
        for c in &mut contributions {
            c.fraction_of_total = c.acceleration_magnitude / scalar_sum;
        }
    }

    let dominant_body_fraction = if scalar_sum > 1e-30 {
        (max_body_accel / scalar_sum).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let net_mag = (net_accel[0] * net_accel[0] + net_accel[1] * net_accel[1] + net_accel[2] * net_accel[2]).sqrt();
    let tidal_tensor = compute_tidal_tensor(system, point_m);

    SpatialFieldPoint {
        position_m: point_m,
        acceleration_vector_mps2: net_accel,
        acceleration_magnitude: net_mag,
        gravitational_potential_j_kg: total_potential,
        dominant_body_id: dominant_id,
        dominant_body_name: dominant_name,
        dominant_body_fraction,
        contributions,
        tidal_tensor,
    }
}

/// Samples the gravitational field across a 2D bounding box on the orbital plane ($z = 0$).
///
/// Useful for generating gravitational dominance basin maps and vector field lattices.
pub fn compute_spatial_field_grid(
    system: &NBodySystem,
    x_range_m: [f64; 2],
    y_range_m: [f64; 2],
    resolution_x: usize,
    resolution_y: usize,
) -> Vec<SpatialFieldPoint> {
    let nx = resolution_x.max(2);
    let ny = resolution_y.max(2);
    let mut grid = Vec::with_capacity(nx * ny);

    let dx = (x_range_m[1] - x_range_m[0]) / ((nx - 1) as f64);
    let dy = (y_range_m[1] - y_range_m[0]) / ((ny - 1) as f64);

    for j in 0..ny {
        let y = y_range_m[0] + (j as f64) * dy;
        for i in 0..nx {
            let x = x_range_m[0] + (i as f64) * dx;
            grid.push(compute_spatial_field_point(system, [x, y, 0.0]));
        }
    }

    grid
}

/// Determines if a celestial body is relevant to the active gravitational centric frame.
/// In planet-centric mode, unindependent external celestial bodies (such as distant planets or the Sun)
/// are filtered out so that only the local gravitational subsystem is computed.
pub fn is_body_relevant_to_centric(body_name: &str, centric_body_name: &str) -> bool {
    if centric_body_name.eq_ignore_ascii_case("Sun") {
        return true;
    }
    if centric_body_name.eq_ignore_ascii_case("Earth") {
        return body_name.eq_ignore_ascii_case("Earth") || body_name.eq_ignore_ascii_case("Moon");
    }
    if centric_body_name.eq_ignore_ascii_case("Moon") {
        return body_name.eq_ignore_ascii_case("Moon") || body_name.eq_ignore_ascii_case("Earth");
    }
    if centric_body_name.eq_ignore_ascii_case("Jupiter") {
        return body_name.eq_ignore_ascii_case("Jupiter")
            || body_name.eq_ignore_ascii_case("Io")
            || body_name.eq_ignore_ascii_case("Europa")
            || body_name.eq_ignore_ascii_case("Ganymede")
            || body_name.eq_ignore_ascii_case("Callisto");
    }
    body_name.eq_ignore_ascii_case(centric_body_name)
}

/// Computes a high-resolution gravitational field grid centered dynamically on a specific celestial body.
///
/// Pins the center of the sampling bounding box to the target body's position $[x_0, y_0, z_0]$
/// and spans $[x_0 \pm \text{half\_span\_m}, y_0 \pm \text{half\_span\_m}]$.
///
/// In planet-centric gravity modes (non-Heliocentric), the Sun's gravity is ignored and
/// unindependent external bodies are removed to isolate the planet's local gravity well and satellite interactions.
pub fn compute_centric_spatial_field_grid(
    system: &NBodySystem,
    center_body_name: &str,
    half_span_m: f64,
    resolution: usize,
) -> Result<Vec<SpatialFieldPoint>, String> {
    let center_body = system
        .bodies
        .iter()
        .find(|b| b.name.eq_ignore_ascii_case(center_body_name))
        .ok_or_else(|| format!("Body '{}' not found in system", center_body_name))?;

    let cx = center_body.position_m[0];
    let cy = center_body.position_m[1];
    let cz = center_body.position_m[2];

    let x_range = [cx - half_span_m, cx + half_span_m];
    let y_range = [cy - half_span_m, cy + half_span_m];

    let nx = resolution.max(2);
    let ny = resolution.max(2);
    let mut grid = Vec::with_capacity(nx * ny);

    let dx = (x_range[1] - x_range[0]) / ((nx - 1) as f64);
    let dy = (y_range[1] - y_range[0]) / ((ny - 1) as f64);

    let active_system;
    let sys_ref = if !center_body_name.eq_ignore_ascii_case("Sun") {
        active_system = NBodySystem {
            bodies: system
                .bodies
                .iter()
                .filter(|b| is_body_relevant_to_centric(&b.name, center_body_name))
                .cloned()
                .collect(),
            ..system.clone()
        };
        &active_system
    } else {
        system
    };

    for j in 0..ny {
        let y = y_range[0] + (j as f64) * dy;
        for i in 0..nx {
            let x = x_range[0] + (i as f64) * dx;
            grid.push(compute_spatial_field_point(sys_ref, [x, y, cz]));
        }
    }

    Ok(grid)
}

/// Evaluates the net gravitational acceleration acting on a test particle in the non-inertial
/// reference frame centered on the active centric body.
///
/// In accordance with celestial mechanics, the relative acceleration is given by:
/// $$ \ddot{\mathbf{r}}_{\text{rel}} = -\frac{G M_0 \mathbf{r}_{\text{rel}}}{(\|\mathbf{r}_{\text{rel}}\|^2 + \epsilon^2)^{3/2}} + \sum_{k \neq 0} G M_k \left( \frac{\mathbf{r}_k - \mathbf{r}_{\text{rel}}}{(\|\mathbf{r}_k - \mathbf{r}_{\text{rel}}\|^2 + \epsilon^2)^{3/2}} - \frac{\mathbf{r}_k}{(\|\mathbf{r}_k\|^2 + \epsilon^2)^{3/2}} \right) $$
/// where $M_0$ is the mass of the centric anchor body, and the sum over $k$ includes all relevant
/// gravitational perturbers (such as the Moon in Geocentric frame) while ignoring the Sun and
/// distant unindependent bodies.
pub fn compute_centric_particle_acceleration(
    system: &NBodySystem,
    center_body_name: &str,
    rel_pos_m: [f64; 3],
) -> Result<[f64; 3], String> {
    let center_body = system
        .bodies
        .iter()
        .find(|b| b.name.eq_ignore_ascii_case(center_body_name))
        .ok_or_else(|| format!("Body '{}' not found in system", center_body_name))?;

    let eps2 = system.softening_m * system.softening_m;

    // 1. Primary Central Gravitational Pull
    let rx = rel_pos_m[0];
    let ry = rel_pos_m[1];
    let rz = rel_pos_m[2];
    let r2 = rx * rx + ry * ry + rz * rz;
    let dist_soft_sq = r2 + eps2;
    let dist_soft = dist_soft_sq.sqrt();

    let mut ax = 0.0;
    let mut ay = 0.0;
    let mut az = 0.0;

    if dist_soft > 1e-6 {
        let mu0 = G_STANDARD * center_body.mass_kg;
        let factor0 = -mu0 / (dist_soft_sq * dist_soft);
        ax += factor0 * rx;
        ay += factor0 * ry;
        az += factor0 * rz;
    }

    // 2. Perturber Tidal & Indirect Reflex Accelerations
    let is_heliocentric = center_body_name.eq_ignore_ascii_case("Sun");
    let c_pos = center_body.position_m;

    for b in &system.bodies {
        if b.name.eq_ignore_ascii_case(center_body_name) {
            continue;
        }
        if !is_heliocentric {
            if b.name.eq_ignore_ascii_case("Sun") {
                continue;
            }
            if !is_body_relevant_to_centric(&b.name, center_body_name) {
                continue;
            }
        }

        let mu_k = G_STANDARD * b.mass_kg;

        // Relative position of perturber body k from center body
        let rk_x = b.position_m[0] - c_pos[0];
        let rk_y = b.position_m[1] - c_pos[1];
        let rk_z = b.position_m[2] - c_pos[2];
        let rk2 = rk_x * rk_x + rk_y * rk_y + rk_z * rk_z;
        let rk_soft_sq = rk2 + eps2;
        let rk_soft = rk_soft_sq.sqrt();

        // Vector from particle to perturber body k: delta = r_k - r_rel
        let delta_x = rk_x - rx;
        let delta_y = rk_y - ry;
        let delta_z = rk_z - rz;
        let delta2 = delta_x * delta_x + delta_y * delta_y + delta_z * delta_z;
        let delta_soft_sq = delta2 + eps2;
        let delta_soft = delta_soft_sq.sqrt();

        // Direct acceleration
        if delta_soft > 1e-6 {
            let f_dir = mu_k / (delta_soft_sq * delta_soft);
            ax += f_dir * delta_x;
            ay += f_dir * delta_y;
            az += f_dir * delta_z;
        }

        // Indirect d'Alembert reflex acceleration of centric origin
        if rk_soft > 1e-6 {
            let f_ind = -mu_k / (rk_soft_sq * rk_soft);
            ax += f_ind * rk_x;
            ay += f_ind * rk_y;
            az += f_ind * rk_z;
        }
    }

    Ok([ax, ay, az])
}

/// Generates initial conditions for a swarm of test particles in Keplerian orbits around the centric body.
pub fn generate_centric_test_particles(
    system: &NBodySystem,
    center_body_name: &str,
    count: usize,
    seed: u64,
) -> Result<Vec<CentricTestParticle>, String> {
    let center_body = system
        .bodies
        .iter()
        .find(|b| b.name.eq_ignore_ascii_case(center_body_name))
        .ok_or_else(|| format!("Body '{}' not found in system", center_body_name))?;

    let mu = G_STANDARD * center_body.mass_kg;
    let (r_min, r_max) = if center_body_name.eq_ignore_ascii_case("Earth") {
        (15_000_000.0, 350_000_000.0) // 15,000 km to 350,000 km
    } else if center_body_name.eq_ignore_ascii_case("Moon") {
        (2_500_000.0, 35_000_000.0)   // 2,500 km to 35,000 km
    } else if center_body_name.eq_ignore_ascii_case("Jupiter") {
        (500_000_000.0, 15_000_000_000.0) // 500k km to 15M km
    } else {
        (0.4 * ASTRONOMICAL_UNIT_M, 4.5 * ASTRONOMICAL_UNIT_M)
    };

    let mut particles = Vec::with_capacity(count);
    let mut rng_state = seed ^ 0x9e3779b97f4a7c15;

    // Linear Congruential Generator helper
    let mut next_f64 = || {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((rng_state >> 11) as f64) / ((1u64 << 53) as f64)
    };

    for i in 0..count {
        let frac = (i as f64 + next_f64() * 0.5) / (count as f64);
        let r = r_min + frac * (r_max - r_min);
        let theta = next_f64() * 2.0 * PI;
        let inc = (next_f64() - 0.5) * 0.35; // ±10 degrees

        let x = r * theta.cos() * inc.cos();
        let y = r * theta.sin() * inc.cos();
        let z = r * inc.sin();

        // Circular velocity with slight random perturbation
        let v_circ = (mu / r).sqrt();
        let ecc_factor = 0.92 + next_f64() * 0.16; // 0.92 to 1.08
        let speed = v_circ * ecc_factor;

        let vx = -speed * theta.sin() * inc.cos();
        let vy = speed * theta.cos() * inc.cos();
        let vz = speed * (next_f64() - 0.5) * 0.1;

        particles.push(CentricTestParticle {
            id: (i + 1) as u64,
            anchor_body_name: center_body.name.clone(),
            rel_position_m: [x, y, z],
            rel_velocity_m_s: [vx, vy, vz],
            orbital_radius_m: r,
        });
    }

    Ok(particles)
}

/// Computes Vis-Viva orbital speed for a given radial distance $r$, semi-major axis $a$,
/// and standard gravitational parameter $\mu = G M$:
/// $$ v(r) = \sqrt{\mu \left(\frac{2}{r} - \frac{1}{a}\right)} $$
pub fn compute_vis_viva_speed(mu: f64, r: f64, a: f64) -> f64 {
    if r <= 0.0 || a <= 0.0 || mu <= 0.0 {
        return 0.0;
    }
    let term = 2.0 / r - 1.0 / a;
    if term <= 0.0 {
        0.0
    } else {
        (mu * term).sqrt()
    }
}

/// Evaluates normalized Vis-Viva kinetic parameter $\tau \in [0.0, 1.0]$ across an orbit's
/// periapsis-to-apoapsis velocity domain:
/// $$ \tau = \frac{v(r) - v_{\min}}{v_{\max} - v_{\min}} $$
/// where $\tau = 1.0$ at periapsis (maximum velocity) and $\tau = 0.0$ at apoapsis (minimum velocity).
pub fn compute_vis_viva_normalized_kinetic(mu: f64, r: f64, a: f64, e: f64) -> f64 {
    if r <= 0.0 || a <= 0.0 || mu <= 0.0 {
        return 0.5;
    }
    let e_clamped = e.clamp(0.0, 0.999);
    if e_clamped < 1e-5 {
        return 0.5;
    }

    let r_p = a * (1.0 - e_clamped);
    let r_a = a * (1.0 + e_clamped);

    let v_min = compute_vis_viva_speed(mu, r_a, a);
    let v_max = compute_vis_viva_speed(mu, r_p, a);

    if (v_max - v_min).abs() < 1e-9 {
        return 0.5;
    }

    let v_r = compute_vis_viva_speed(mu, r, a);
    ((v_r - v_min) / (v_max - v_min)).clamp(0.0, 1.0)
}
