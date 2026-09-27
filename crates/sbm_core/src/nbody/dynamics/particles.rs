//! Relative centric test particle acceleration and swarm generation.
//!
//! # Academic Literature Grounding
//! - **Non-Inertial Reference Frames & d'Alembert Reflex Perturbations**:
//!   - Danby, J. M. A. (1988). *Fundamentals of Celestial Mechanics*, 2nd ed., Willmann-Bell, Chapter 11.
//!   - Roy, A. E. (2005). *Orbital Motion*, 4th ed., Institute of Physics Publishing, Chapter 5.
//! - **Oblate Zonal Harmonics ($J_2$) Geopotential Gradient**:
//!   - Kaula, W. M. (1966). *Theory of Satellite Geodesy: Use of Artificial Satellites for Geodesy*, Blaisdell Publishing Co.
//!   - Kozai, Y. (1959). "The motion of a close earth satellite", *The Astronomical Journal*, 64(9), pp. 367–377.
//!   - Vallado, D. A. (2013). *Fundamentals of Astrodynamics and Applications*, 4th ed., Microcosm Press, Chapter 8.
//! - **Third-Body Gravitational Perturbations (Lunisolar / Planetary)**:
//!   - Kozai, Y. (1962). "Secular perturbations of asteroids with high inclination and eccentricity", *The Astronomical Journal*, 67, pp. 591–598.
//!   - Battin, R. H. (1999). *An Introduction to the Mathematics and Methods of Astrodynamics*, AIAA Education Series, Chapter 10.
//! - **Equations of Motion**:
//!   $$\ddot{\mathbf{r}} = -\frac{\mu_0 \mathbf{r}}{\|\mathbf{r}\|^3} + \mathbf{a}_{J2}(\mathbf{r}) + \sum_{k \neq 0} \mu_k \left[ \frac{\mathbf{r}_k - \mathbf{r}}{\|\mathbf{r}_k - \mathbf{r}\|^3} - \frac{\mathbf{r}_k}{\|\mathbf{r}_k\|^3} \right]$$
//!   where the first term is central Keplerian gravity, the second is central body oblateness ($J_2$),
//!   and the summation represents third-body direct pull and d'Alembert reflex acceleration.

use core::f64::consts::PI;
use crate::nbody::types::{
    CentricPerturbationMode, CentricTestParticle, NBodySystem, ParticleAccelerationBreakdown,
    ASTRONOMICAL_UNIT_M, G_STANDARD,
};

/// Returns the equatorial radius $R_0$ in meters and dimensionless $J_2$ zonal harmonic coefficient
/// for standard celestial bodies based on IERS 2010, Lunar Prospector, and Juno gravity models.
///
/// References:
/// - Earth: IERS Conventions (2010), Petit & Luzum (eds.), IERS Technical Note No. 36.
/// - Moon: Konopliv et al. (2001), *Icarus*, 150(1), pp. 1–18.
/// - Jupiter: Folkner et al. (2017), *Geophysical Research Letters*, 44(10), pp. 4694–4700.
/// - Mars: Konopliv et al. (2011), *Icarus*, 211(1), pp. 401–419.
/// - Venus: Konopliv et al. (1999), *Icarus*, 139(1), pp. 3–18.
/// - Saturn: Iess et al. (2019), *Science*, 364(6445), eaat2965.
/// - Sun: Rozelot et al. (2009), *Solar Physics*, 256(1), pp. 1–15.
pub fn get_body_j2_parameters(body_name: &str) -> (f64, f64) {
    match body_name.to_ascii_lowercase().as_str() {
        "earth" => (6_378_137.0, 1.082_626_68e-3), // IERS 2010 / WGS84
        "moon" => (1_737_400.0, 2.03e-4),          // Lunar Prospector
        "jupiter" => (71_492_000.0, 1.4736e-2),     // Juno gravity
        "mars" => (3_396_200.0, 1.960_45e-3),      // MRO
        "venus" => (6_051_800.0, 4.458e-6),        // Magellan
        "saturn" => (60_268_000.0, 1.6297e-2),     // Cassini
        "sun" => (696_340_000.0, 2.2e-7),          // Solar oblateness
        _ => (0.0, 0.0),
    }
}

/// Computes the central body $J_2$ oblate zonal harmonic perturbative acceleration:
///
/// $$ a_{J2, x} = -\frac{3}{2} J_2 \frac{\mu R_0^2}{r^5} x \left( 1 - 5 \frac{z^2}{r^2} \right) $$
/// $$ a_{J2, y} = -\frac{3}{2} J_2 \frac{\mu R_0^2}{r^5} y \left( 1 - 5 \frac{z^2}{r^2} \right) $$
/// $$ a_{J2, z} = -\frac{3}{2} J_2 \frac{\mu R_0^2}{r^5} z \left( 3 - 5 \frac{z^2}{r^2} \right) $$
///
/// References:
/// - Kaula, W. M. (1966). *Theory of Satellite Geodesy*, Chapter 3.
/// - Vallado, D. A. (2013). *Fundamentals of Astrodynamics and Applications*, Chapter 8.
pub fn compute_j2_acceleration(rel_pos_m: [f64; 3], mu: f64, r_eq_m: f64, j2: f64) -> [f64; 3] {
    let rx = rel_pos_m[0];
    let ry = rel_pos_m[1];
    let rz = rel_pos_m[2];
    let r2 = rx * rx + ry * ry + rz * rz;
    let r = r2.sqrt();

    if r < 1e-3 || j2.abs() < 1e-15 || r_eq_m <= 0.0 || mu <= 0.0 {
        return [0.0, 0.0, 0.0];
    }

    let r5 = r2 * r2 * r;
    let re2 = r_eq_m * r_eq_m;
    let z2_over_r2 = (rz * rz) / r2;
    let coeff = -1.5 * j2 * mu * re2 / r5;

    let ax = coeff * rx * (1.0 - 5.0 * z2_over_r2);
    let ay = coeff * ry * (1.0 - 5.0 * z2_over_r2);
    let az = coeff * rz * (3.0 - 5.0 * z2_over_r2);

    [ax, ay, az]
}

/// Determines if a celestial body acts as a relevant third-body perturber for a centric frame.
pub fn is_body_relevant_perturber(body_name: &str, centric_body_name: &str) -> bool {
    if body_name.eq_ignore_ascii_case(centric_body_name) {
        return false;
    }
    if centric_body_name.eq_ignore_ascii_case("Sun") {
        return true;
    }
    // Sun is always a major tidal perturber for any planet/moon in the solar system
    if body_name.eq_ignore_ascii_case("Sun") {
        return true;
    }
    if centric_body_name.eq_ignore_ascii_case("Earth") {
        return body_name.eq_ignore_ascii_case("Moon");
    }
    if centric_body_name.eq_ignore_ascii_case("Moon") {
        return body_name.eq_ignore_ascii_case("Earth");
    }
    if centric_body_name.eq_ignore_ascii_case("Jupiter") {
        return body_name.eq_ignore_ascii_case("Io")
            || body_name.eq_ignore_ascii_case("Europa")
            || body_name.eq_ignore_ascii_case("Ganymede")
            || body_name.eq_ignore_ascii_case("Callisto");
    }
    false
}

/// Evaluates the net gravitational acceleration acting on a test particle in the non-inertial
/// reference frame centered on the active centric body with a detailed component breakdown.
///
/// Grounded in Danby (1988), Roy (2005), and Kaula (1966).
pub fn compute_centric_particle_acceleration_with_breakdown(
    system: &NBodySystem,
    center_body_name: &str,
    rel_pos_m: [f64; 3],
    mode: CentricPerturbationMode,
) -> Result<([f64; 3], ParticleAccelerationBreakdown), String> {
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

    let mu0 = G_STANDARD * center_body.mass_kg;
    if dist_soft > 1e-6 && mu0 > 0.0 {
        let factor0 = -mu0 / (dist_soft_sq * dist_soft);
        ax += factor0 * rx;
        ay += factor0 * ry;
        az += factor0 * rz;
    }
    let a_central_mag = (ax * ax + ay * ay + az * az).sqrt();

    // 2. Central Body J2 Oblateness Zonal Perturbation
    let mut a_j2_mag = 0.0;
    if mode == CentricPerturbationMode::FullPerturbed {
        let (r_eq_m, j2) = get_body_j2_parameters(center_body_name);
        if j2.abs() > 1e-15 && r_eq_m > 0.0 {
            let a_j2 = compute_j2_acceleration(rel_pos_m, mu0, r_eq_m, j2);
            ax += a_j2[0];
            ay += a_j2[1];
            az += a_j2[2];
            a_j2_mag = (a_j2[0] * a_j2[0] + a_j2[1] * a_j2[1] + a_j2[2] * a_j2[2]).sqrt();
        }
    }

    // 3. Third-Body Tidal & d'Alembert Reflex Accelerations
    let mut a_3rd_x = 0.0;
    let mut a_3rd_y = 0.0;
    let mut a_3rd_z = 0.0;
    let mut max_perturber_mag = 0.0;
    let mut dominant_perturber_name = "None".to_string();

    if mode == CentricPerturbationMode::ThirdBody || mode == CentricPerturbationMode::FullPerturbed {
        let c_pos = center_body.position_m;

        for b in &system.bodies {
            if !is_body_relevant_perturber(&b.name, center_body_name) {
                continue;
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

            let mut b_ax = 0.0;
            let mut b_ay = 0.0;
            let mut b_az = 0.0;

            // Direct acceleration
            if delta_soft > 1e-6 {
                let f_dir = mu_k / (delta_soft_sq * delta_soft);
                b_ax += f_dir * delta_x;
                b_ay += f_dir * delta_y;
                b_az += f_dir * delta_z;
            }

            // Indirect d'Alembert reflex acceleration of centric origin
            if rk_soft > 1e-6 {
                let f_ind = -mu_k / (rk_soft_sq * rk_soft);
                b_ax += f_ind * rk_x;
                b_ay += f_ind * rk_y;
                b_az += f_ind * rk_z;
            }

            let b_mag = (b_ax * b_ax + b_ay * b_ay + b_az * b_az).sqrt();
            if b_mag > max_perturber_mag {
                max_perturber_mag = b_mag;
                dominant_perturber_name = b.name.clone();
            }

            a_3rd_x += b_ax;
            a_3rd_y += b_ay;
            a_3rd_z += b_az;
        }

        ax += a_3rd_x;
        ay += a_3rd_y;
        az += a_3rd_z;
    }

    let a_third_body_mag = (a_3rd_x * a_3rd_x + a_3rd_y * a_3rd_y + a_3rd_z * a_3rd_z).sqrt();
    let a_total_mag = (ax * ax + ay * ay + az * az).sqrt();

    let breakdown = ParticleAccelerationBreakdown {
        a_central_mps2: a_central_mag,
        a_j2_mps2: a_j2_mag,
        a_third_body_mps2: a_third_body_mag,
        a_total_mps2: a_total_mag,
        dominant_perturber_name,
    };

    Ok(([ax, ay, az], breakdown))
}

/// Evaluates the net gravitational acceleration acting on a test particle in the non-inertial
/// reference frame centered on the active centric body following Danby (1988) and Roy (2005).
pub fn compute_centric_particle_acceleration(
    system: &NBodySystem,
    center_body_name: &str,
    rel_pos_m: [f64; 3],
) -> Result<[f64; 3], String> {
    compute_centric_particle_acceleration_with_breakdown(
        system,
        center_body_name,
        rel_pos_m,
        CentricPerturbationMode::FullPerturbed,
    )
    .map(|(acc, _)| acc)
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
