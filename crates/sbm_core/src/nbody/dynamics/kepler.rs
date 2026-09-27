//! Osculating Keplerian orbital mechanics, Vis-Viva energy, and orbital resonance metrics.

use core::f64::consts::PI;
use crate::nbody::types::{
    CelestialBody, EclipseState, OsculatingElements, ResonanceMetrics, SatelliteOrbitalTelemetry,
    G_STANDARD,
};

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

/// Computes comprehensive orbital elements and eclipse telemetry for a tracked satellite or test particle.
pub fn compute_satellite_orbital_telemetry(
    id: u64,
    name: &str,
    anchor: &CelestialBody,
    rel_pos_m: [f64; 3],
    rel_vel_m_s: [f64; 3],
    eclipse_state: EclipseState,
) -> SatelliteOrbitalTelemetry {
    let anchor_radius_m = anchor.radius_km * 1e3;
    let anchor_mass_kg = anchor.mass_kg;
    let anchor_name = &anchor.name;
    let r_mag = (rel_pos_m[0] * rel_pos_m[0] + rel_pos_m[1] * rel_pos_m[1] + rel_pos_m[2] * rel_pos_m[2]).sqrt().max(1.0);
    let v_mag = (rel_vel_m_s[0] * rel_vel_m_s[0] + rel_vel_m_s[1] * rel_vel_m_s[1] + rel_vel_m_s[2] * rel_vel_m_s[2]).sqrt();
    let mu = G_STANDARD * anchor_mass_kg;

    let specific_energy = 0.5 * v_mag * v_mag - mu / r_mag;
    let semi_major_axis_m = if specific_energy.abs() > 1e-12 {
        -mu / (2.0 * specific_energy)
    } else {
        r_mag
    };

    // Specific angular momentum h = r x v
    let hx = rel_pos_m[1] * rel_vel_m_s[2] - rel_pos_m[2] * rel_vel_m_s[1];
    let hy = rel_pos_m[2] * rel_vel_m_s[0] - rel_pos_m[0] * rel_vel_m_s[2];
    let hz = rel_pos_m[0] * rel_vel_m_s[1] - rel_pos_m[1] * rel_vel_m_s[0];
    let h_mag = (hx * hx + hy * hy + hz * hz).sqrt().max(1e-12);

    // Eccentricity vector e = (v x h) / mu - r / |r|
    let vxh_x = rel_vel_m_s[1] * hz - rel_vel_m_s[2] * hy;
    let vxh_y = rel_vel_m_s[2] * hx - rel_vel_m_s[0] * hz;
    let vxh_z = rel_vel_m_s[0] * hy - rel_vel_m_s[1] * hx;

    let ex = vxh_x / mu - rel_pos_m[0] / r_mag;
    let ey = vxh_y / mu - rel_pos_m[1] / r_mag;
    let ez = vxh_z / mu - rel_pos_m[2] / r_mag;
    let eccentricity = (ex * ex + ey * ey + ez * ez).sqrt();

    let inc_deg = (hz / h_mag).clamp(-1.0, 1.0).acos().to_degrees();

    let periapsis_radius_m = semi_major_axis_m * (1.0 - eccentricity);
    let apoapsis_radius_m = semi_major_axis_m * (1.0 + eccentricity);
    let periapsis_altitude_m = periapsis_radius_m - anchor_radius_m;
    let apoapsis_altitude_m = apoapsis_radius_m - anchor_radius_m;
    let current_altitude_m = r_mag - anchor_radius_m;

    let orbital_period_s = if semi_major_axis_m > 0.0 {
        2.0 * PI * (semi_major_axis_m.powi(3) / mu).sqrt()
    } else {
        f64::INFINITY
    };

    SatelliteOrbitalTelemetry {
        id,
        name: name.to_string(),
        anchor_body: anchor_name.to_string(),
        semi_major_axis_m,
        eccentricity,
        inclination_deg: inc_deg,
        periapsis_radius_m,
        apoapsis_radius_m,
        periapsis_altitude_m,
        apoapsis_altitude_m,
        current_radius_m: r_mag,
        current_altitude_m,
        current_speed_m_s: v_mag,
        orbital_period_s,
        eclipse_state,
    }
}
