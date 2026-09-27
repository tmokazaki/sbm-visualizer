//! Relative centric test particle acceleration and swarm generation.
//!
//! # Academic Literature Grounding
//! - **Non-Inertial Reference Frames & d'Alembert Reflex Perturbations**:
//!   - Danby, J. M. A. (1988). *Fundamentals of Celestial Mechanics*, 2nd ed., Willmann-Bell, Chapter 11.
//!   - Roy, A. E. (2005). *Orbital Motion*, 4th ed., Institute of Physics Publishing, Chapter 5.
//! - **Equations of Motion**:
//!   $$\ddot{\mathbf{r}} = -\frac{\mu_0 \mathbf{r}}{\|\mathbf{r}\|^3} + \sum_{k \neq 0} \mu_k \left[ \frac{\mathbf{r}_k - \mathbf{r}}{\|\mathbf{r}_k - \mathbf{r}\|^3} - \frac{\mathbf{r}_k}{\|\mathbf{r}_k\|^3} \right]$$
//!   where the second term represents direct perturbation and the third term is the d'Alembert reflex acceleration of the central body.

use core::f64::consts::PI;
use crate::nbody::dynamics::field::is_body_relevant_to_centric;
use crate::nbody::types::{
    CentricTestParticle, NBodySystem, ASTRONOMICAL_UNIT_M, G_STANDARD,
};

/// Evaluates the net gravitational acceleration acting on a test particle in the non-inertial
/// reference frame centered on the active centric body following Danby (1988) and Roy (2005).
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
