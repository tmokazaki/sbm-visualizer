//! High-precision gravitational dynamics, relativistic post-Newtonian equations, and conservation laws.
//!
//! # Academic Literature Grounding
//! - **1PN General Relativity & Post-Newtonian Dynamics**:
//!   - Einstein, A., Infeld, L., & Hoffmann, B. (1938). "The Gravitational Equations and the Problem of Motion".
//!     *Annals of Mathematics*, 39(1), pp. 65–100.
//!   - Will, C. M. (2014). "The Confrontation between General Relativity and Experiment".
//!     *Living Reviews in Relativity*, 17(4), Section 3.1.
//! - **Symplectic Integration & Conservation Invariants**:
//!   - Yoshida, H. (1990). "Construction of higher order symplectic integrators".
//!     *Physics Letters A*, 150(5–7), pp. 262–268. DOI: [10.1016/0375-9601(90)90092-3](https://doi.org/10.1016/0375-9601(90)90092-3).
//!   - Makino, J., & Aarseth, S. J. (1992). "A Polynomial Approximation for N-body Simulations: Hermite Scheme".
//!     *Publications of the Astronomical Society of Japan (PASJ)*, 44, pp. 141–151.

use crate::nbody::types::{
    CelestialBody, ConservationMetrics, NBodySystem, PairwiseForce, SPEED_OF_LIGHT,
};

/// Computes Cartesian gravitational acceleration vectors for all bodies in the system.
///
/// Includes pairwise Newtonian interactions and 1PN General Relativistic post-Newtonian
/// corrections based on the Einstein-Infeld-Hoffmann (EIH) formulation (Einstein et al. 1938; Will 2014):
///
/// $$\mathbf{a}_{i,\text{1PN}} = \sum_{j \neq i} \frac{G M_j}{c^2 r_{ij}^3} \left[ \left( \frac{4 G M_j}{r_{ij}} - v_i^2 \right) \mathbf{r}_{ij} + 4 (\mathbf{r}_{ij} \cdot \mathbf{v}_i) \mathbf{v}_i \right]$$
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
