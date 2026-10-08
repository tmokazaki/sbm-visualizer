//! Astrodynamic orbital propagation, 3-body gravitational fields, and Tsiolkovsky propulsion.
//!
//! Grounded in Section IV-A and Algorithms 1 & 2 of Luna et al. (2026):
//! - **Algorithm 1**: Total Gravitational Acceleration (Earth point-mass + Moon + Sun perturbations)
//! - **Equations (1)–(3)**: Continuous equations of motion and semi-implicit Euler integration
//! - **Algorithm 2 / Equation (4)**: Instantaneous rocket equation propellant consumption

use crate::types::{
    Action3D, SatelliteState, Vector3D, G0_STANDARD, MU_EARTH, MU_MOON, MU_SUN,
    SPECIFIC_IMPULSE_S,
};

/// Computes the Earth-centered orbital position of the Moon at elapsed simulation time $t$ (seconds).
///
/// Uses an analytical inclined circular orbit representation with mean semi-major axis
/// $a_{\text{Moon}} = 384,400\text{ km}$, orbital period $T_{\text{Moon}} = 27.32166\text{ days}$,
/// and inclination $i_{\text{Moon}} = 5.145^\circ$.
#[inline]
pub fn moon_position_earth_centered(elapsed_seconds: f64) -> Vector3D {
    const A_MOON: f64 = 384_400_000.0; // meters
    const PERIOD_MOON_S: f64 = 27.32166 * 86400.0;
    const INC_MOON_RAD: f64 = 5.145 * core::f64::consts::PI / 180.0;

    let mean_anomaly = (2.0 * core::f64::consts::PI / PERIOD_MOON_S) * elapsed_seconds;
    let cos_m = mean_anomaly.cos();
    let sin_m = mean_anomaly.sin();

    Vector3D::new(
        A_MOON * cos_m,
        A_MOON * sin_m * INC_MOON_RAD.cos(),
        A_MOON * sin_m * INC_MOON_RAD.sin(),
    )
}

/// Computes the Earth-centered orbital position of the Sun at elapsed simulation time $t$ (seconds).
///
/// Uses an analytical ecliptic representation with mean distance $1\text{ AU} = 149,597,870,700\text{ m}$,
/// sidereal year period $T_\odot = 365.25636\text{ days}$, and obliquity of ecliptic $\epsilon = 23.4393^\circ$.
#[inline]
pub fn sun_position_earth_centered(elapsed_seconds: f64) -> Vector3D {
    const A_SUN: f64 = 149_597_870_700.0; // meters (1 AU)
    const PERIOD_SUN_S: f64 = 365.25636 * 86400.0;
    const OBLIQUITY_RAD: f64 = 23.4393 * core::f64::consts::PI / 180.0;

    let mean_anomaly = (2.0 * core::f64::consts::PI / PERIOD_SUN_S) * elapsed_seconds;
    let cos_s = mean_anomaly.cos();
    let sin_s = mean_anomaly.sin();

    Vector3D::new(
        A_SUN * cos_s,
        A_SUN * sin_s * OBLIQUITY_RAD.cos(),
        A_SUN * sin_s * OBLIQUITY_RAD.sin(),
    )
}

/// Computes total gravitational acceleration acting on a satellite at position $\mathbf{r}_{\text{sat}}$.
///
/// Implements Algorithm 1 of Luna et al. (2026):
/// $$\mathbf{a}_{\text{total}} = -\frac{\mu_{\oplus}}{\|\mathbf{r}_{\text{sat}}\|^3}\mathbf{r}_{\text{sat}}
/// + \mu_{\text{Moon}}\frac{\mathbf{r}_{\text{rel, Moon}}}{\|\mathbf{r}_{\text{rel, Moon}}\|^3}
/// + \mu_{\odot}\frac{\mathbf{r}_{\text{rel, Sun}}}{\|\mathbf{r}_{\text{rel, Sun}}\|^3}$$
#[inline]
pub fn compute_total_gravitational_acceleration(
    sat_pos: Vector3D,
    elapsed_seconds: f64,
) -> Vector3D {
    // 1. Earth point-mass central gravity
    let r_sat_norm = sat_pos.norm();
    let r_sat_norm3 = (r_sat_norm * r_sat_norm * r_sat_norm).max(1e-12);
    let a_earth = sat_pos * (-MU_EARTH / r_sat_norm3);

    // 2. Moon gravitational acceleration in Earth-centered frame
    let r_moon_ec = moon_position_earth_centered(elapsed_seconds);
    let r_rel_moon = r_moon_ec - sat_pos;
    let r_moon_norm = r_rel_moon.norm();
    let r_moon_norm3 = (r_moon_norm * r_moon_norm * r_moon_norm).max(1e-12);
    let a_moon = r_rel_moon * (MU_MOON / r_moon_norm3);

    // 3. Sun gravitational acceleration in Earth-centered frame
    let r_sun_ec = sun_position_earth_centered(elapsed_seconds);
    let r_rel_sun = r_sun_ec - sat_pos;
    let r_sun_norm = r_rel_sun.norm();
    let r_sun_norm3 = (r_sun_norm * r_sun_norm * r_sun_norm).max(1e-12);
    let a_sun = r_rel_sun * (MU_SUN / r_sun_norm3);

    a_earth + a_moon + a_sun
}

/// Applies commanded thrust acceleration and burns propellant according to the Tsiolkovsky equation.
///
/// Implements Algorithm 2 of Luna et al. (2026):
/// 1. Clamps action $\mathbf{a} \in [-1, 1]^3$, maps to thrust acceleration $\mathbf{a}_{\text{thrust}} = \mathbf{a}_{\text{cmd}} \cdot T_{\max}$.
/// 2. If fuel is available, integrates velocity $\Delta v = \|\mathbf{a}_{\text{thrust}}\|\Delta t$.
/// 3. Propellant consumption follows:
///    $$m_{\text{consumed}} = m_{\text{sat}} \left(1 - \exp\left(-\frac{\Delta v}{I_{sp} g_0}\right)\right)$$
/// 4. Reduces fuel mass $m_{\text{fuel}}$ and wet mass $m_{\text{sat}}$.
///
/// Returns `(actual_acceleration, delta_v, fuel_consumed)`.
#[inline]
pub fn apply_thrust_step(
    satellite: &mut SatelliteState,
    action: Action3D,
    max_thrust: f64,
    dt_seconds: f64,
) -> (Vector3D, f64, f64) {
    if satellite.fuel_mass_kg <= 0.0 {
        // Thrusters cannot fire when fuel is depleted
        return (Vector3D::zero(), 0.0, 0.0);
    }

    let a_cmd = Vector3D::new(
        action.ax.clamp(-1.0, 1.0),
        action.ay.clamp(-1.0, 1.0),
        action.az.clamp(-1.0, 1.0),
    );
    let a_thrust = a_cmd * max_thrust;
    let delta_v = a_thrust.norm() * dt_seconds;

    if delta_v <= 0.0 {
        return (Vector3D::zero(), 0.0, 0.0);
    }

    // Tsiolkovsky instantaneous approximation
    let ve = SPECIFIC_IMPULSE_S * G0_STANDARD;
    let exponent = -delta_v / ve;
    let fuel_consumed_candidate = satellite.total_mass_kg * (1.0 - exponent.exp());

    // Cap consumption to remaining fuel
    let (fuel_consumed, actual_accel, actual_dv) = if fuel_consumed_candidate >= satellite.fuel_mass_kg {
        let fuel_consumed = satellite.fuel_mass_kg;
        // Adjust achievable delta-v
        let dv_achievable = -ve * (1.0 - fuel_consumed / satellite.total_mass_kg).ln();
        let scale = if delta_v > 1e-12 { (dv_achievable / delta_v).clamp(0.0, 1.0) } else { 0.0 };
        (fuel_consumed, a_thrust * scale, dv_achievable)
    } else {
        (fuel_consumed_candidate, a_thrust, delta_v)
    };

    satellite.fuel_mass_kg = (satellite.fuel_mass_kg - fuel_consumed).max(0.0);
    satellite.total_mass_kg = (satellite.total_mass_kg - fuel_consumed).max(0.0);

    (actual_accel, actual_dv, fuel_consumed)
}

/// Integrates satellite state by one discrete time step $\Delta t$ using semi-implicit Euler.
///
/// Implements Equation (3) of Luna et al. (2026):
/// $$\mathbf{v}_{t+1} = \mathbf{v}_t + \Delta t \left(\mathbf{a}_{\text{gravity}} + \mathbf{a}_{\text{thrust}}\right)$$
/// $$\mathbf{r}_{t+1} = \mathbf{r}_t + \Delta t \mathbf{v}_{t+1}$$
#[inline]
pub fn step_dynamics(
    satellite: &mut SatelliteState,
    action: Action3D,
    elapsed_seconds: f64,
    max_thrust: f64,
    dt_seconds: f64,
) -> (f64, f64) {
    // 1. Apply thrust & propellant consumption
    let (a_thrust, delta_v, fuel_consumed) = apply_thrust_step(satellite, action, max_thrust, dt_seconds);

    // 2. Gravitational acceleration from Earth, Moon, and Sun
    let a_gravity = compute_total_gravitational_acceleration(satellite.position, elapsed_seconds);

    // 3. Semi-implicit Euler integration
    let a_total = a_gravity + a_thrust;
    satellite.velocity = satellite.velocity + a_total * dt_seconds;
    satellite.position = satellite.position + satellite.velocity * dt_seconds;

    (delta_v, fuel_consumed)
}
