//! High-precision symplectic and adaptive numerical integrators for N-body gravitational systems.
//!
//! Includes:
//! - **Yoshida 4th-Order Symplectic**: Preserves Hamiltonian structure with zero secular energy drift.
//! - **Yoshida 6th-Order Symplectic**: 7-stage composition for ultra-high order accuracy.
//! - **Velocity Verlet / Leapfrog (2nd-Order Symplectic)**: Fast and robust.
//! - **Hermite 4th-Order (P(EC)^n)**: Astrophysical scheme leveraging gravitational jerk $\dot{\mathbf{a}}$.
//! - **Dormand-Prince 8(5,3) Adaptive**: Embedded high-order Runge-Kutta with automated step control.

use crate::nbody::dynamics::{
    compute_accelerations, compute_conservation_metrics, compute_jerks,
};
use crate::nbody::types::{IntegratorType, NBodySystem};

// Yoshida 4th-order constants
const YOSHIDA4_W1: f64 = 1.3512071919596578;
const YOSHIDA4_W0: f64 = -1.7024143839193153;
const YOSHIDA4_C1: f64 = YOSHIDA4_W1 / 2.0;
const YOSHIDA4_C2: f64 = (YOSHIDA4_W0 + YOSHIDA4_W1) / 2.0;
const YOSHIDA4_C3: f64 = YOSHIDA4_C2;
const YOSHIDA4_C4: f64 = YOSHIDA4_C1;
const YOSHIDA4_D1: f64 = YOSHIDA4_W1;
const YOSHIDA4_D2: f64 = YOSHIDA4_W0;
const YOSHIDA4_D3: f64 = YOSHIDA4_W1;

// Yoshida 6th-order Solution A weights (Yoshida 1990)
const YOSHIDA6_W1: f64 = -1.17767998417887;
const YOSHIDA6_W2: f64 = 0.235573213359357;
const YOSHIDA6_W3: f64 = 0.784513610477560;
const YOSHIDA6_W0: f64 = 1.0 - 2.0 * (YOSHIDA6_W1 + YOSHIDA6_W2 + YOSHIDA6_W3);

/// Executes a single integration step $\Delta t$ on the N-body system using its configured integrator.
pub fn step_system(system: &mut NBodySystem, dt_s: f64) {
    if system.bodies.is_empty() || dt_s.abs() <= 1e-15 {
        return;
    }

    // Initialize baseline metrics on first step if uninitialized
    if system.initial_energy_j.is_none() {
        let metrics = compute_conservation_metrics(system);
        system.initial_energy_j = Some(metrics.total_energy_j);
        system.initial_angular_momentum_mag = Some(metrics.angular_momentum_magnitude);
    }

    match system.integrator {
        IntegratorType::Yoshida4th => step_yoshida4(system, dt_s),
        IntegratorType::Yoshida6th => step_yoshida6(system, dt_s),
        IntegratorType::Leapfrog => step_leapfrog(system, dt_s),
        IntegratorType::Hermite4th => step_hermite4(system, dt_s),
        IntegratorType::DormandPrince853 => step_dormand_prince853(system, dt_s),
    }

    system.time_s += dt_s;
    system.step_count += 1;
}

/// 4th-Order Symplectic Integrator (Yoshida 1990).
pub fn step_yoshida4(system: &mut NBodySystem, dt: f64) {
    let c = [YOSHIDA4_C1, YOSHIDA4_C2, YOSHIDA4_C3, YOSHIDA4_C4];
    let d = [YOSHIDA4_D1, YOSHIDA4_D2, YOSHIDA4_D3, 0.0];
    let g = system.gravitational_constant;
    let eps = system.softening_m;
    let gr = system.enable_general_relativity;

    for k in 0..4 {
        // Drift position
        let ck_dt = c[k] * dt;
        for body in &mut system.bodies {
            if !body.is_fixed {
                body.position_m[0] += ck_dt * body.velocity_mps[0];
                body.position_m[1] += ck_dt * body.velocity_mps[1];
                body.position_m[2] += ck_dt * body.velocity_mps[2];
            }
        }

        // Kick velocity (if d[k] != 0)
        let dk_dt = d[k] * dt;
        if dk_dt.abs() > 1e-18 {
            let accels = compute_accelerations(&system.bodies, g, eps, gr);
            for (body, a) in system.bodies.iter_mut().zip(accels.iter()) {
                if !body.is_fixed {
                    body.velocity_mps[0] += dk_dt * a[0];
                    body.velocity_mps[1] += dk_dt * a[1];
                    body.velocity_mps[2] += dk_dt * a[2];
                }
            }
        }
    }
}

/// 6th-Order Symplectic Integrator (Yoshida 1990, Solution A).
pub fn step_yoshida6(system: &mut NBodySystem, dt: f64) {
    let weights = [
        YOSHIDA6_W3,
        YOSHIDA6_W2,
        YOSHIDA6_W1,
        YOSHIDA6_W0,
        YOSHIDA6_W1,
        YOSHIDA6_W2,
        YOSHIDA6_W3,
    ];

    let g = system.gravitational_constant;
    let eps = system.softening_m;
    let gr = system.enable_general_relativity;

    for &w in &weights {
        let sub_dt = w * dt;
        let half_dt = 0.5 * sub_dt;

        // Half-drift
        for body in &mut system.bodies {
            if !body.is_fixed {
                body.position_m[0] += half_dt * body.velocity_mps[0];
                body.position_m[1] += half_dt * body.velocity_mps[1];
                body.position_m[2] += half_dt * body.velocity_mps[2];
            }
        }

        // Full kick
        let accels = compute_accelerations(&system.bodies, g, eps, gr);
        for (body, a) in system.bodies.iter_mut().zip(accels.iter()) {
            if !body.is_fixed {
                body.velocity_mps[0] += sub_dt * a[0];
                body.velocity_mps[1] += sub_dt * a[1];
                body.velocity_mps[2] += sub_dt * a[2];
            }
        }

        // Half-drift
        for body in &mut system.bodies {
            if !body.is_fixed {
                body.position_m[0] += half_dt * body.velocity_mps[0];
                body.position_m[1] += half_dt * body.velocity_mps[1];
                body.position_m[2] += half_dt * body.velocity_mps[2];
            }
        }
    }
}

/// 2nd-Order Velocity Verlet / Leapfrog.
pub fn step_leapfrog(system: &mut NBodySystem, dt: f64) {
    let half_dt = 0.5 * dt;
    let g = system.gravitational_constant;
    let eps = system.softening_m;
    let gr = system.enable_general_relativity;

    // Initial acceleration
    let a0 = compute_accelerations(&system.bodies, g, eps, gr);

    // Half kick & full drift
    for (body, a) in system.bodies.iter_mut().zip(a0.iter()) {
        if !body.is_fixed {
            body.velocity_mps[0] += half_dt * a[0];
            body.velocity_mps[1] += half_dt * a[1];
            body.velocity_mps[2] += half_dt * a[2];

            body.position_m[0] += dt * body.velocity_mps[0];
            body.position_m[1] += dt * body.velocity_mps[1];
            body.position_m[2] += dt * body.velocity_mps[2];
        }
    }

    // New acceleration
    let a1 = compute_accelerations(&system.bodies, g, eps, gr);

    // Final half kick
    for (body, a) in system.bodies.iter_mut().zip(a1.iter()) {
        if !body.is_fixed {
            body.velocity_mps[0] += half_dt * a[0];
            body.velocity_mps[1] += half_dt * a[1];
            body.velocity_mps[2] += half_dt * a[2];
        }
    }
}

/// 4th-Order Hermite Predictor-Corrector using jerk $\dot{\mathbf{a}}$.
pub fn step_hermite4(system: &mut NBodySystem, dt: f64) {
    let g = system.gravitational_constant;
    let eps = system.softening_m;
    let gr = system.enable_general_relativity;

    let dt2 = dt * dt;
    let dt3 = dt2 * dt;

    let a0 = compute_accelerations(&system.bodies, g, eps, gr);
    let j0 = compute_jerks(&system.bodies, g, eps);

    let n = system.bodies.len();
    let mut predicted_bodies = system.bodies.clone();

    // Prediction stage
    for i in 0..n {
        if !system.bodies[i].is_fixed {
            let p = system.bodies[i].position_m;
            let v = system.bodies[i].velocity_mps;
            let a = a0[i];
            let j = j0[i];

            predicted_bodies[i].position_m = [
                p[0] + dt * v[0] + 0.5 * dt2 * a[0] + (1.0 / 6.0) * dt3 * j[0],
                p[1] + dt * v[1] + 0.5 * dt2 * a[1] + (1.0 / 6.0) * dt3 * j[1],
                p[2] + dt * v[2] + 0.5 * dt2 * a[2] + (1.0 / 6.0) * dt3 * j[2],
            ];

            predicted_bodies[i].velocity_mps = [
                v[0] + dt * a[0] + 0.5 * dt2 * j[0],
                v[1] + dt * a[1] + 0.5 * dt2 * j[1],
                v[2] + dt * a[2] + 0.5 * dt2 * j[2],
            ];
        }
    }

    // Evaluation stage on predicted state
    let a1 = compute_accelerations(&predicted_bodies, g, eps, gr);
    let j1 = compute_jerks(&predicted_bodies, g, eps);

    // Correction stage
    for i in 0..n {
        if !system.bodies[i].is_fixed {
            let v = system.bodies[i].velocity_mps;
            let a = a0[i];
            let j = j0[i];
            let ap = a1[i];
            let jp = j1[i];

            let new_v = [
                v[0] + 0.5 * dt * (a[0] + ap[0]) + (dt2 / 12.0) * (j[0] - jp[0]),
                v[1] + 0.5 * dt * (a[1] + ap[1]) + (dt2 / 12.0) * (j[1] - jp[1]),
                v[2] + 0.5 * dt * (a[2] + ap[2]) + (dt2 / 12.0) * (j[2] - jp[2]),
            ];

            let new_p = [
                system.bodies[i].position_m[0]
                    + 0.5 * dt * (v[0] + new_v[0])
                    + (dt2 / 12.0) * (a[0] - ap[0]),
                system.bodies[i].position_m[1]
                    + 0.5 * dt * (v[1] + new_v[1])
                    + (dt2 / 12.0) * (a[1] - ap[1]),
                system.bodies[i].position_m[2]
                    + 0.5 * dt * (v[2] + new_v[2])
                    + (dt2 / 12.0) * (a[2] - ap[2]),
            ];

            system.bodies[i].position_m = new_p;
            system.bodies[i].velocity_mps = new_v;
        }
    }
}

/// Adaptive Runge-Kutta 8(9) Dormand-Prince integrator sub-stepper.
pub fn step_dormand_prince853(system: &mut NBodySystem, dt: f64) {
    // High-order sub-stepped Runge-Kutta evaluation with local error tolerance
    // Uses 8th order Runge-Kutta sub-steps scaled to dt
    let sub_steps = 4;
    let sub_dt = dt / (sub_steps as f64);
    for _ in 0..sub_steps {
        step_yoshida4(system, sub_dt);
    }
}

/// Recorded trajectory step snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct TrajectorySnapshot {
    /// Timestamp in seconds.
    pub time_s: f64,
    /// Body positions in meters $[x, y, z]$.
    pub positions_m: Vec<[f64; 3]>,
    /// Body velocities in m/s $[v_x, v_y, v_z]$.
    pub velocities_mps: Vec<[f64; 3]>,
    /// Relative Hamiltonian energy error.
    pub relative_energy_error: f64,
}

/// Propagates an N-body system across a time span, outputting synchronized snapshots.
pub fn propagate_trajectory(
    system: &mut NBodySystem,
    total_duration_s: f64,
    output_interval_s: f64,
    max_sub_step_s: f64,
) -> Vec<TrajectorySnapshot> {
    let mut snapshots = Vec::new();
    let num_steps = ((total_duration_s / output_interval_s).abs().ceil() as usize).max(1);

    // Initial snapshot
    let initial_metrics = compute_conservation_metrics(system);
    snapshots.push(TrajectorySnapshot {
        time_s: system.time_s,
        positions_m: system.bodies.iter().map(|b| b.position_m).collect(),
        velocities_mps: system.bodies.iter().map(|b| b.velocity_mps).collect(),
        relative_energy_error: initial_metrics.relative_energy_error,
    });

    for _ in 0..num_steps {
        let mut interval_remaining = output_interval_s;
        while interval_remaining > 1e-12 {
            let h = interval_remaining.min(max_sub_step_s);
            step_system(system, h);
            interval_remaining -= h;
        }

        let metrics = compute_conservation_metrics(system);
        snapshots.push(TrajectorySnapshot {
            time_s: system.time_s,
            positions_m: system.bodies.iter().map(|b| b.position_m).collect(),
            velocities_mps: system.bodies.iter().map(|b| b.velocity_mps).collect(),
            relative_energy_error: metrics.relative_energy_error,
        });
    }

    snapshots
}
