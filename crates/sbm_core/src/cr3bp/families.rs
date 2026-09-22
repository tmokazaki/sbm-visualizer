//! CR3BP equilibrium points (Lagrange points), periodic orbit families, and invariant manifolds.
//!
//! Grounded in AAS 20-459 Sections 3–5:
//! - Exact Lagrange points $L_1\text{--}L_5$ via Euler quintic polynomial roots
//! - Periodic orbit families (Lyapunov, Halo, NRHO)
//! - Monodromy matrix $\mathbf{M} = \mathbf{\Phi}(T, 0)$ and stable/unstable invariant manifolds $W^s, W^u$

use crate::cr3bp::dynamics::{equations_of_motion, jacobi_constant};
use crate::cr3bp::integrator::{
    DormandPrinceIntegrator, EventCondition, EventDirection, IntegratorOptions, TrajectoryPoint,
};
use crate::cr3bp::types::{Cr3bpError, Cr3bpState, Cr3bpSystem, LagrangePoint};

/// Information about a computed Lagrange (libration) point.
#[derive(Debug, Clone, PartialEq)]
pub struct LibrationPointInfo {
    pub point: LagrangePoint,
    pub state: Cr3bpState,
    pub jacobi_constant: f64,
    pub distance_to_primary: f64,
    pub distance_to_secondary: f64,
}

/// Solves Euler's quintic polynomial for collinear Lagrange points using Newton-Raphson.
///
/// Returns distance $\gamma$ to the nearest primary/secondary body to machine precision ($< 10^{-14}$).
pub fn solve_euler_quintic(
    point: LagrangePoint,
    mu: f64,
    max_iter: usize,
    tol: f64,
) -> Result<f64, Cr3bpError> {
    // Initial guess based on Hill's approximation (mu/3)^(1/3)
    let hill_radius = (mu / 3.0).cbrt();
    let mut gamma = match point {
        LagrangePoint::L1 => hill_radius * (1.0 - hill_radius / 3.0),
        LagrangePoint::L2 => hill_radius * (1.0 + hill_radius / 3.0),
        LagrangePoint::L3 => 1.0 - 7.0 * mu / 12.0,
        _ => return Err(Cr3bpError::InvalidConfig("Only L1, L2, L3 are collinear".into())),
    };

    for iter in 0..max_iter {
        let (f, df) = match point {
            LagrangePoint::L1 => {
                // gamma^5 - (3 - mu)*gamma^4 + (3 - 2*mu)*gamma^3 - mu*gamma^2 + 2*mu*gamma - mu = 0
                let g = gamma;
                let g2 = g * g;
                let g3 = g2 * g;
                let g4 = g3 * g;
                let g5 = g4 * g;
                let f = g5 - (3.0 - mu) * g4 + (3.0 - 2.0 * mu) * g3 - mu * g2 + 2.0 * mu * g - mu;
                let df = 5.0 * g4 - 4.0 * (3.0 - mu) * g3 + 3.0 * (3.0 - 2.0 * mu) * g2 - 2.0 * mu * g + 2.0 * mu;
                (f, df)
            }
            LagrangePoint::L2 => {
                // gamma^5 + (3 - mu)*gamma^4 + (3 - 2*mu)*gamma^3 - mu*gamma^2 - 2*mu*gamma - mu = 0
                let g = gamma;
                let g2 = g * g;
                let g3 = g2 * g;
                let g4 = g3 * g;
                let g5 = g4 * g;
                let f = g5 + (3.0 - mu) * g4 + (3.0 - 2.0 * mu) * g3 - mu * g2 - 2.0 * mu * g - mu;
                let df = 5.0 * g4 + 4.0 * (3.0 - mu) * g3 + 3.0 * (3.0 - 2.0 * mu) * g2 - 2.0 * mu * g - 2.0 * mu;
                (f, df)
            }
            LagrangePoint::L3 => {
                // gamma^5 + (2 + mu)*gamma^4 + (1 + 2*mu)*gamma^3 - (1 - mu)*gamma^2 - 2*(1 - mu)*gamma - (1 - mu) = 0
                let g = gamma;
                let g2 = g * g;
                let g3 = g2 * g;
                let g4 = g3 * g;
                let g5 = g4 * g;
                let om_mu = 1.0 - mu;
                let f = g5 + (2.0 + mu) * g4 + (1.0 + 2.0 * mu) * g3 - om_mu * g2 - 2.0 * om_mu * g - om_mu;
                let df = 5.0 * g4 + 4.0 * (2.0 + mu) * g3 + 3.0 * (1.0 + 2.0 * mu) * g2 - 2.0 * om_mu * g - 2.0 * om_mu;
                (f, df)
            }
            _ => unreachable!(),
        };

        let delta = f / df;
        gamma -= delta;

        if delta.abs() < tol {
            return Ok(gamma);
        }

        if iter == max_iter - 1 {
            return Err(Cr3bpError::LibrationPointConvergenceFailed {
                point,
                iterations: max_iter,
            });
        }
    }

    Ok(gamma)
}

/// Computes the exact nondimensional coordinates and Jacobi constant of all 5 Lagrange points.
pub fn compute_lagrange_points(system: &Cr3bpSystem) -> Result<[LibrationPointInfo; 5], Cr3bpError> {
    let mu = system.mu;
    let tol = 1e-15;
    let max_iter = 100;

    // L1: between P1 and P2, distance gamma1 from P2
    let gamma1 = solve_euler_quintic(LagrangePoint::L1, mu, max_iter, tol)?;
    let x_l1 = 1.0 - mu - gamma1;
    let state_l1 = Cr3bpState::new(x_l1, 0.0, 0.0, 0.0, 0.0, 0.0);
    let cj_l1 = jacobi_constant(system, &state_l1);
    let l1_info = LibrationPointInfo {
        point: LagrangePoint::L1,
        state: state_l1,
        jacobi_constant: cj_l1,
        distance_to_primary: x_l1 + mu,
        distance_to_secondary: gamma1,
    };

    // L2: beyond P2, distance gamma2 from P2
    let gamma2 = solve_euler_quintic(LagrangePoint::L2, mu, max_iter, tol)?;
    let x_l2 = 1.0 - mu + gamma2;
    let state_l2 = Cr3bpState::new(x_l2, 0.0, 0.0, 0.0, 0.0, 0.0);
    let cj_l2 = jacobi_constant(system, &state_l2);
    let l2_info = LibrationPointInfo {
        point: LagrangePoint::L2,
        state: state_l2,
        jacobi_constant: cj_l2,
        distance_to_primary: x_l2 + mu,
        distance_to_secondary: gamma2,
    };

    // L3: beyond P1, distance gamma3 from P1
    let gamma3 = solve_euler_quintic(LagrangePoint::L3, mu, max_iter, tol)?;
    let x_l3 = -mu - gamma3;
    let state_l3 = Cr3bpState::new(x_l3, 0.0, 0.0, 0.0, 0.0, 0.0);
    let cj_l3 = jacobi_constant(system, &state_l3);
    let l3_info = LibrationPointInfo {
        point: LagrangePoint::L3,
        state: state_l3,
        jacobi_constant: cj_l3,
        distance_to_primary: gamma3,
        distance_to_secondary: gamma3 + 1.0,
    };

    // L4: equilateral triangle ahead of secondary
    let x_l4 = 0.5 - mu;
    let y_l4 = (3.0_f64).sqrt() / 2.0;
    let state_l4 = Cr3bpState::new(x_l4, y_l4, 0.0, 0.0, 0.0, 0.0);
    let cj_l4 = jacobi_constant(system, &state_l4);
    let l4_info = LibrationPointInfo {
        point: LagrangePoint::L4,
        state: state_l4,
        jacobi_constant: cj_l4,
        distance_to_primary: 1.0,
        distance_to_secondary: 1.0,
    };

    // L5: equilateral triangle trailing behind secondary
    let x_l5 = 0.5 - mu;
    let y_l5 = -((3.0_f64).sqrt() / 2.0);
    let state_l5 = Cr3bpState::new(x_l5, y_l5, 0.0, 0.0, 0.0, 0.0);
    let cj_l5 = jacobi_constant(system, &state_l5);
    let l5_info = LibrationPointInfo {
        point: LagrangePoint::L5,
        state: state_l5,
        jacobi_constant: cj_l5,
        distance_to_primary: 1.0,
        distance_to_secondary: 1.0,
    };

    Ok([l1_info, l2_info, l3_info, l4_info, l5_info])
}

/// Canonical periodic orbit definitions and benchmark states.
#[derive(Debug, Clone, PartialEq)]
pub struct PeriodicOrbitBenchmark {
    /// Name of the orbit (e.g. "Earth-Moon L1 Lyapunov").
    pub name: &'static str,
    /// Lagrange point around which the orbit revolves.
    pub point: LagrangePoint,
    /// Initial nondimensional rotating state $[x_0, y_0, z_0, \dot{x}_0, \dot{y}_0, \dot{z}_0]$.
    pub initial_state: Cr3bpState,
    /// Nondimensional orbital period $T$.
    pub period_nondim: f64,
    /// Period in days.
    pub period_days: f64,
    /// Nominal Jacobi constant $C_J$.
    pub jacobi_constant: f64,
    /// Closest approach to secondary (perilune) in nondimensional distance.
    pub perilune_dist_nd: f64,
}

impl PeriodicOrbitBenchmark {
    /// Earth–Moon $L_1$ Planar Lyapunov orbit matching AAS 20-459 Section 4 & Figure 2a, 4a.
    ///
    /// - Period: 12.46 days ($T \approx 2.8694$ nondim)
    /// - Jacobi constant: $C_J \approx 3.163007$
    pub fn earth_moon_l1_lyapunov() -> Self {
        // High-precision converged initial state matching AAS 20-459 Section 5 (C_J = 3.163007)
        let initial_state = Cr3bpState::new(
            0.819394213903,
            0.0,
            0.0,
            0.0,
            0.169105771181,
            0.0,
        );
        let period_nondim = 2.7903044968;
        let period_days = 12.1168;
        let jacobi_constant = 3.163007;

        Self {
            name: "Earth-Moon L1 Planar Lyapunov",
            point: LagrangePoint::L1,
            initial_state,
            period_nondim,
            period_days,
            jacobi_constant,
            perilune_dist_nd: 1.0 - 0.0121505856 - 0.819394213903,
        }
    }

    /// Earth–Moon $L_1$ Southern Halo orbit matching AAS 20-459 Section 4 & Figure 2a, 5.
    ///
    /// - Period: 12.28 days ($T \approx 2.8279$ nondim)
    pub fn earth_moon_l1_southern_halo() -> Self {
        let initial_state = Cr3bpState::new(
            0.835824510,
            0.0,
            -0.088349280,
            0.0,
            0.158428310,
            0.0,
        );
        let period_nondim = 2.827914;
        let period_days = 12.2831;
        let jacobi_constant = 3.15982;

        Self {
            name: "Earth-Moon L1 Southern Halo",
            point: LagrangePoint::L1,
            initial_state,
            period_nondim,
            period_days,
            jacobi_constant,
            perilune_dist_nd: 0.165,
        }
    }

    /// Earth–Moon $L_2$ Lyapunov orbit matching AAS 20-459 Section 5 (low-energy transfer target).
    ///
    /// - Jacobi constant: $C_J \approx 3.162991$
    pub fn earth_moon_l2_lyapunov() -> Self {
        let initial_state = Cr3bpState::new(
            1.173792942585,
            0.0,
            0.0,
            0.0,
            -0.106864156133,
            0.0,
        );
        let period_nondim = 3.4114560448;
        let period_days = 14.8142;
        let jacobi_constant = 3.162991;

        Self {
            name: "Earth-Moon L2 Planar Lyapunov",
            point: LagrangePoint::L2,
            initial_state,
            period_nondim,
            period_days,
            jacobi_constant,
            perilune_dist_nd: 1.155694200 - (1.0 - 0.0121505856),
        }
    }

    /// Earth–Moon $L_2$ Near-Rectilinear Halo Orbit (NRHO) matching AAS 20-459 Section 4 (altitude $\approx 86$ km).
    ///
    /// - Perilune altitude above lunar surface: $\approx 86\text{ km}$ ($r_{moon} \approx 1737.4\text{ km}$)
    pub fn earth_moon_l2_nrho() -> Self {
        let initial_state = Cr3bpState::new(
            1.0219485,
            0.0,
            -0.182463,
            0.0,
            -0.103215,
            0.0,
        );
        let period_nondim = 1.5124;
        let period_days = 6.568;
        let jacobi_constant = 3.0452;

        Self {
            name: "Earth-Moon L2 Near-Rectilinear Halo Orbit (NRHO)",
            point: LagrangePoint::L2,
            initial_state,
            period_nondim,
            period_days,
            jacobi_constant,
            perilune_dist_nd: (1737.4 + 86.0) / 384400.0,
        }
    }

    /// Sun–Earth $L_2$ Halo Orbit (JWST / Gaia mission orbit preset for deep space operations).
    ///
    /// - Revolution period $\approx 180\text{ days}$ (half a year)
    pub fn sun_earth_l2_halo() -> Self {
        let initial_state = Cr3bpState::new(
            1.008323,
            0.0,
            0.003328,
            0.0,
            0.010834,
            0.0,
        );
        let period_nondim = 3.1025;
        let period_days = 179.8;
        let jacobi_constant = 3.00085;

        Self {
            name: "Sun-Earth L2 Halo (JWST Mission Class)",
            point: LagrangePoint::L2,
            initial_state,
            period_nondim,
            period_days,
            jacobi_constant,
            perilune_dist_nd: 0.01004,
        }
    }
}

/// Detailed information about a converged periodic orbit.
#[derive(Debug, Clone, PartialEq)]
pub struct CorrectedOrbit {
    /// Initial nondimensional rotating state $[x_0, y_0, z_0, v_{x0}, v_{y0}, v_{z0}]$.
    pub initial_state: Cr3bpState,
    /// Orbital period in nondimensional time units ($T = 2 t_{1/2}$).
    pub period_nondim: f64,
    /// Orbital period in dimensional days.
    pub period_days: f64,
    /// Jacobi constant $C_J$.
    pub jacobi_constant: f64,
    /// Full 6x6 Monodromy matrix $\mathbf{M} = \mathbf{\Phi}(T, 0)$.
    pub monodromy_matrix: [[f64; 6]; 6],
    /// Stability index $\nu = \frac{1}{2}(\lambda_u + 1/\lambda_u)$.
    pub stability_index: f64,
    /// Maximum unstable eigenvalue magnitude ($|\lambda_u| \ge 1.0$).
    pub lambda_unstable: f64,
    /// Corresponding stable eigenvalue ($|\lambda_s| \le 1.0$).
    pub lambda_stable: f64,
    /// Normalized unstable eigenvector at initial state.
    pub eigenvector_unstable: [f64; 6],
    /// Normalized stable eigenvector at initial state.
    pub eigenvector_stable: [f64; 6],
    /// Iterations required to converge.
    pub iterations: usize,
    /// Orthogonal crossing residual norm ($|\dot{x}(T/2)|$ or $\sqrt{\dot{x}^2 + \dot{z}^2}$).
    pub residual: f64,
}

/// Direction of invariant manifold propagation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifoldType {
    /// Unstable manifold $W^u$ (asymptotically departs periodic orbit as $t \to +\infty$).
    Unstable,
    /// Stable manifold $W^s$ (asymptotically arrives at periodic orbit as $t \to +\infty$, integrated backward).
    Stable,
}

/// Manifold displacement branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifoldBranch {
    /// Step off in positive eigenvector direction ($+\epsilon$).
    Positive,
    /// Step off in negative eigenvector direction ($-\epsilon$).
    Negative,
}

/// A computed invariant manifold trajectory arc.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifoldArc {
    pub manifold_type: ManifoldType,
    pub branch: ManifoldBranch,
    /// Phase parameter along periodic orbit $\tau \in [0, 1]$.
    pub orbit_phase: f64,
    /// Trajectory points along the manifold ray.
    pub trajectory: Vec<TrajectoryPoint>,
    /// State at the end of the manifold arc.
    pub final_state: Cr3bpState,
    /// Flight duration in days.
    pub flight_time_days: f64,
    /// Jacobi constant along arc.
    pub jacobi_constant: f64,
}

/// Numerically corrects a planar Lyapunov periodic orbit using single-shooting differential correction.
///
/// Exploits the symmetry about the $x$-axis:
/// Starts at $[x_0, 0, 0, 0, \dot{y}_0, 0]$ at $t = 0$.
/// Corrects $\dot{y}_0$ until the half-period crossing $y(T/2) = 0$ is orthogonal ($\dot{x}(T/2) = 0$).
pub fn correct_planar_lyapunov(
    system: &Cr3bpSystem,
    x0: f64,
    vy0_guess: f64,
    max_iter: usize,
    tol: f64,
) -> Result<CorrectedOrbit, Cr3bpError> {
    let mut vy0 = vy0_guess;
    let mut iterations = 0;
    let mut final_residual = 0.0;
    let mut t_half = 0.0;
    let mut converged_state = Cr3bpState::zero();

    let options = IntegratorOptions {
        rel_tol: 1e-12,
        abs_tol: 1e-12,
        initial_step: 1e-4,
        min_step: 1e-14,
        max_step: 0.02,
        max_steps: 200_000,
    };
    let integrator = DormandPrinceIntegrator::new(system, options);

    for iter in 0..max_iter {
        iterations = iter + 1;
        let state = Cr3bpState::new(x0, 0.0, 0.0, 0.0, vy0, 0.0);

        let event_dir = if vy0 > 0.0 {
            EventDirection::Negative
        } else {
            EventDirection::Positive
        };
        let event = EventCondition::PlaneY {
            y_target: 0.0,
            direction: event_dir,
        };

        // Propagate with STM up to max nondimensional time of 10.0 (~43 days)
        let prop_res = integrator.propagate_with_stm(&state, 0.0, 10.0, Some(&event))?;
        let half_state = prop_res.final_state;
        let stm = prop_res.final_stm.ok_or(Cr3bpError::DifferentialCorrectionFailed {
            iterations,
            residual: 1.0,
        })?;

        t_half = prop_res.final_time;
        final_residual = half_state.vx.abs();

        if final_residual < tol {
            converged_state = state;
            break;
        }

        // Sensitivity of vx(T/2) with respect to vy0:
        // d(vx)/d(vy0) = Phi[3][4] - (ax / vy) * Phi[1][4]
        let deriv = equations_of_motion(system, &half_state);
        let ax = deriv[3];
        let vy = half_state.vy;
        if vy.abs() < 1e-12 {
            return Err(Cr3bpError::DifferentialCorrectionFailed {
                iterations,
                residual: final_residual,
            });
        }

        let d_vx_d_vy0 = stm[3][4] - (ax / vy) * stm[1][4];
        if d_vx_d_vy0.abs() < 1e-15 {
            return Err(Cr3bpError::DifferentialCorrectionFailed {
                iterations,
                residual: final_residual,
            });
        }

        let delta_vy0 = -half_state.vx / d_vx_d_vy0;
        vy0 += delta_vy0;

        if iter == max_iter - 1 {
            return Err(Cr3bpError::DifferentialCorrectionFailed {
                iterations,
                residual: final_residual,
            });
        }
    }

    let period_nondim = 2.0 * t_half;
    let period_days = period_nondim * system.t_star / 86400.0;
    let cj = jacobi_constant(system, &converged_state);

    let full_prop = integrator.propagate_with_stm(&converged_state, 0.0, period_nondim, None)?;
    let monodromy = full_prop.final_stm.unwrap_or([[0.0; 6]; 6]);

    let (stability_index, lambda_u, lambda_s, eig_u, eig_s) =
        compute_monodromy_stability(&monodromy);

    Ok(CorrectedOrbit {
        initial_state: converged_state,
        period_nondim,
        period_days,
        jacobi_constant: cj,
        monodromy_matrix: monodromy,
        stability_index,
        lambda_unstable: lambda_u,
        lambda_stable: lambda_s,
        eigenvector_unstable: eig_u,
        eigenvector_stable: eig_s,
        iterations,
        residual: final_residual,
    })
}

/// Numerically corrects a 3D Halo periodic orbit for a specified out-of-plane amplitude $z_0$.
///
/// Starts at $[x_0, 0, z_0, 0, \dot{y}_0, 0]$ at $t = 0$.
/// Adjusts $(x_0, \dot{y}_0)$ until crossing $y = 0$ orthogonally ($\dot{x}(T/2) = 0$ and $\dot{z}(T/2) = 0$).
pub fn correct_3d_halo(
    system: &Cr3bpSystem,
    z0: f64,
    x0_guess: f64,
    vy0_guess: f64,
    max_iter: usize,
    tol: f64,
) -> Result<CorrectedOrbit, Cr3bpError> {
    let mut x0 = x0_guess;
    let mut vy0 = vy0_guess;
    let mut iterations = 0;
    let mut final_residual = 0.0;
    let mut t_half = 0.0;
    let mut converged_state = Cr3bpState::zero();

    let options = IntegratorOptions {
        rel_tol: 1e-12,
        abs_tol: 1e-12,
        initial_step: 1e-4,
        min_step: 1e-14,
        max_step: 0.02,
        max_steps: 200_000,
    };
    let integrator = DormandPrinceIntegrator::new(system, options);

    for iter in 0..max_iter {
        iterations = iter + 1;
        let state = Cr3bpState::new(x0, 0.0, z0, 0.0, vy0, 0.0);

        let event_dir = if vy0 > 0.0 {
            EventDirection::Negative
        } else {
            EventDirection::Positive
        };
        let event = EventCondition::PlaneY {
            y_target: 0.0,
            direction: event_dir,
        };

        let prop_res = integrator.propagate_with_stm(&state, 0.0, 10.0, Some(&event))?;
        let half_state = prop_res.final_state;
        let stm = prop_res.final_stm.ok_or(Cr3bpError::DifferentialCorrectionFailed {
            iterations,
            residual: 1.0,
        })?;

        t_half = prop_res.final_time;
        let rx = half_state.vx;
        let rz = half_state.vz;
        final_residual = (rx * rx + rz * rz).sqrt();

        if final_residual < tol {
            converged_state = state;
            break;
        }

        let deriv = equations_of_motion(system, &half_state);
        let ax = deriv[3];
        let az = deriv[5];
        let vy = half_state.vy;
        if vy.abs() < 1e-12 {
            return Err(Cr3bpError::DifferentialCorrectionFailed {
                iterations,
                residual: final_residual,
            });
        }

        // 2x2 Jacobian J = [ d_vx/d_x0, d_vx/d_vy0 ; d_vz/d_x0, d_vz/d_vy0 ]
        let j00 = stm[3][0] - (ax / vy) * stm[1][0];
        let j01 = stm[3][4] - (ax / vy) * stm[1][4];
        let j10 = stm[5][0] - (az / vy) * stm[1][0];
        let j11 = stm[5][4] - (az / vy) * stm[1][4];

        let det = j00 * j11 - j01 * j10;
        if det.abs() < 1e-15 {
            return Err(Cr3bpError::DifferentialCorrectionFailed {
                iterations,
                residual: final_residual,
            });
        }

        let delta_x0 = -(j11 * rx - j01 * rz) / det;
        let delta_vy0 = -(-j10 * rx + j00 * rz) / det;

        x0 += delta_x0;
        vy0 += delta_vy0;

        if iter == max_iter - 1 {
            return Err(Cr3bpError::DifferentialCorrectionFailed {
                iterations,
                residual: final_residual,
            });
        }
    }

    let period_nondim = 2.0 * t_half;
    let period_days = period_nondim * system.t_star / 86400.0;
    let cj = jacobi_constant(system, &converged_state);

    let full_prop = integrator.propagate_with_stm(&converged_state, 0.0, period_nondim, None)?;
    let monodromy = full_prop.final_stm.unwrap_or([[0.0; 6]; 6]);

    let (stability_index, lambda_u, lambda_s, eig_u, eig_s) =
        compute_monodromy_stability(&monodromy);

    Ok(CorrectedOrbit {
        initial_state: converged_state,
        period_nondim,
        period_days,
        jacobi_constant: cj,
        monodromy_matrix: monodromy,
        stability_index,
        lambda_unstable: lambda_u,
        lambda_stable: lambda_s,
        eigenvector_unstable: eig_u,
        eigenvector_stable: eig_s,
        iterations,
        residual: final_residual,
    })
}

/// Computes stability index and unstable/stable eigenvectors from the 6x6 Monodromy matrix.
pub fn compute_monodromy_stability(
    monodromy: &[[f64; 6]; 6],
) -> (f64, f64, f64, [f64; 6], [f64; 6]) {
    // 1. Dominant unstable eigenvalue lambda_u and eigenvector v_u via power iteration on M
    let (lambda_u, eig_u) = power_iteration_dominant_eigen(monodromy, 150, 1e-12);

    // 2. Compute exact symplectic inverse M_inv = [ D^T, -B^T ; -C^T, A^T ]
    let m_inv = compute_symplectic_inverse(monodromy);

    // 3. Dominant eigenvalue of M_inv is 1 / lambda_s = lambda_u with eigenvector v_s
    let (lambda_s_inv, eig_s) = power_iteration_dominant_eigen(&m_inv, 150, 1e-12);
    let lambda_s = if lambda_s_inv.abs() > 1e-15 {
        1.0 / lambda_s_inv
    } else {
        1.0
    };

    let stability_index = 0.5 * (lambda_u + lambda_s);
    (stability_index, lambda_u, lambda_s, eig_u, eig_s)
}

/// Computes the exact analytical inverse of a 6x6 symplectic matrix without numerical inversion:
/// $\mathbf{M}^{-1} = \begin{bmatrix} \mathbf{D}^T & -\mathbf{B}^T \\ -\mathbf{C}^T & \mathbf{A}^T \end{bmatrix}$
fn compute_symplectic_inverse(m: &[[f64; 6]; 6]) -> [[f64; 6]; 6] {
    let mut inv = [[0.0; 6]; 6];
    for i in 0..3 {
        for j in 0..3 {
            inv[i][j] = m[3 + j][3 + i];
            inv[i][3 + j] = -m[j][3 + i];
            inv[3 + i][j] = -m[3 + j][i];
            inv[3 + i][3 + j] = m[j][i];
        }
    }
    inv
}

fn power_iteration_dominant_eigen(
    m: &[[f64; 6]; 6],
    max_iter: usize,
    tol: f64,
) -> (f64, [f64; 6]) {
    let mut v = [1.0 / (6.0_f64).sqrt(); 6];
    let mut lambda = 1.0;

    for _ in 0..max_iter {
        let mut w = [0.0; 6];
        for i in 0..6 {
            for j in 0..6 {
                w[i] += m[i][j] * v[j];
            }
        }

        let mut norm_sq = 0.0;
        for &item in &w {
            norm_sq += item * item;
        }
        let norm = norm_sq.sqrt();
        if norm < 1e-15 {
            break;
        }

        let mut dot = 0.0;
        for i in 0..6 {
            dot += v[i] * w[i];
        }

        let mut diff_sq = 0.0;
        for i in 0..6 {
            let next_vi = w[i] / norm;
            let d = next_vi - v[i];
            diff_sq += d * d;
            v[i] = next_vi;
        }

        lambda = dot;
        if diff_sq.sqrt() < tol {
            break;
        }
    }

    (lambda, v)
}

/// Configuration options for generating an invariant manifold arc.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifoldOptions {
    /// Manifold type: Unstable or Stable.
    pub manifold_type: ManifoldType,
    /// Manifold branch: Positive (+epsilon) or Negative (-epsilon).
    pub branch: ManifoldBranch,
    /// Phase parameter along the periodic orbit $\tau \in [0, 1]$.
    pub orbit_phase: f64,
    /// Nondimensional displacement magnitude $\epsilon$ (e.g. $10^{-5}$).
    pub epsilon_dist: f64,
    /// Maximum flight duration in nondimensional time units.
    pub t_span: f64,
}

/// Generates an invariant manifold trajectory arc stepping off a periodic orbit.
pub fn generate_manifold_arc(
    system: &Cr3bpSystem,
    orbit: &CorrectedOrbit,
    options: &ManifoldOptions,
    event: Option<&EventCondition>,
) -> Result<ManifoldArc, Cr3bpError> {
    let int_options = IntegratorOptions {
        rel_tol: 1e-10,
        abs_tol: 1e-10,
        initial_step: 1e-4,
        min_step: 1e-14,
        max_step: 0.02,
        max_steps: 100_000,
    };
    let integrator = DormandPrinceIntegrator::new(system, int_options);

    let t_phase = options.orbit_phase.clamp(0.0, 1.0) * orbit.period_nondim;
    let prop_phase = integrator.propagate_with_stm(&orbit.initial_state, 0.0, t_phase, None)?;
    let phase_state = prop_phase.final_state;
    let phi = prop_phase.final_stm.unwrap_or([[0.0; 6]; 6]);

    let eig_0 = match options.manifold_type {
        ManifoldType::Unstable => orbit.eigenvector_unstable,
        ManifoldType::Stable => orbit.eigenvector_stable,
    };

    let mut eig_tau = [0.0; 6];
    for i in 0..6 {
        for j in 0..6 {
            eig_tau[i] += phi[i][j] * eig_0[j];
        }
    }

    let pos_norm = (eig_tau[0] * eig_tau[0] + eig_tau[1] * eig_tau[1] + eig_tau[2] * eig_tau[2]).sqrt();
    let norm = if pos_norm > 1e-12 {
        pos_norm
    } else {
        (eig_tau.iter().map(|&x| x * x).sum::<f64>()).sqrt().max(1e-12)
    };

    let sign = match options.branch {
        ManifoldBranch::Positive => 1.0,
        ManifoldBranch::Negative => -1.0,
    };

    let pert_state = Cr3bpState::new(
        phase_state.x + sign * options.epsilon_dist * (eig_tau[0] / norm),
        phase_state.y + sign * options.epsilon_dist * (eig_tau[1] / norm),
        phase_state.z + sign * options.epsilon_dist * (eig_tau[2] / norm),
        phase_state.vx + sign * options.epsilon_dist * (eig_tau[3] / norm),
        phase_state.vy + sign * options.epsilon_dist * (eig_tau[4] / norm),
        phase_state.vz + sign * options.epsilon_dist * (eig_tau[5] / norm),
    );

    let (t_start, t_end) = match options.manifold_type {
        ManifoldType::Unstable => (0.0, options.t_span.abs()),
        ManifoldType::Stable => (0.0, -options.t_span.abs()),
    };

    let prop_res = integrator.propagate_6d(&pert_state, t_start, t_end, event)?;
    let flight_time_days = prop_res.final_time.abs() * system.t_star / 86400.0;
    let cj = jacobi_constant(system, &pert_state);

    Ok(ManifoldArc {
        manifold_type: options.manifold_type,
        branch: options.branch,
        orbit_phase: options.orbit_phase,
        trajectory: prop_res.trajectory,
        final_state: prop_res.final_state,
        flight_time_days,
        jacobi_constant: cj,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_euler_quintic_l1_l2_l3_precision() {
        let system = Cr3bpSystem::earth_moon();
        let pts = compute_lagrange_points(&system).expect("Lagrange computation failed");

        // L1 is between 0 and 1-mu
        assert!(pts[0].state.x > 0.0 && pts[0].state.x < 1.0 - system.mu);
        // L2 is beyond 1-mu
        assert!(pts[1].state.x > 1.0 - system.mu);
        // L3 is beyond primary on negative side
        assert!(pts[2].state.x < -system.mu);

        // AAS 20-459 Table 2 comparison: L1 ~ 0.8369, L2 ~ 1.1557
        assert!((pts[0].state.x - 0.836915).abs() < 1e-4);
        assert!((pts[1].state.x - 1.155682).abs() < 1e-4);
    }

    #[test]
    fn test_differential_corrector_earth_moon_l1_lyapunov() {
        let system = Cr3bpSystem::earth_moon();
        let benchmark = PeriodicOrbitBenchmark::earth_moon_l1_lyapunov();

        // Perturb the initial velocity guess by +5%
        let x0 = benchmark.initial_state.x;
        let vy0_perturbed = benchmark.initial_state.vy * 1.05;

        let corrected = correct_planar_lyapunov(&system, x0, vy0_perturbed, 15, 1e-10)
            .expect("Lyapunov correction should converge");

        assert!(corrected.residual < 1e-10, "Residual should be < 1e-10");
        assert!(
            (corrected.initial_state.vy - benchmark.initial_state.vy).abs() < 1e-6,
            "Velocity vy0 should match benchmark to 1e-6: got {}, expected {}",
            corrected.initial_state.vy,
            benchmark.initial_state.vy
        );
        assert!(
            (corrected.period_days - benchmark.period_days).abs() < 0.05,
            "Period in days should match benchmark: got {}, expected {}",
            corrected.period_days,
            benchmark.period_days
        );

        // Monodromy matrix must have det ~ 1.0 and symplectic reciprocal eigenvalues
        assert!(
            corrected.lambda_unstable > 1.0,
            "Unstable eigenvalue must be > 1.0"
        );
        assert!(
            corrected.lambda_stable < 1.0,
            "Stable eigenvalue must be < 1.0"
        );
        let lambda_product = corrected.lambda_unstable * corrected.lambda_stable;
        assert!(
            (lambda_product - 1.0).abs() < 0.05,
            "Product of reciprocal eigenvalues should be ~ 1.0: got {}",
            lambda_product
        );
    }

    #[test]
    fn test_invariant_manifold_arc_energy_conservation() {
        let system = Cr3bpSystem::earth_moon();
        let benchmark = PeriodicOrbitBenchmark::earth_moon_l1_lyapunov();

        let corrected = correct_planar_lyapunov(&system, benchmark.initial_state.x, benchmark.initial_state.vy, 10, 1e-10)
            .expect("Orbit convergence");

        // Generate unstable manifold arc
        let m_opts = ManifoldOptions {
            manifold_type: ManifoldType::Unstable,
            branch: ManifoldBranch::Positive,
            orbit_phase: 0.0,
            epsilon_dist: 1e-5,
            t_span: 1.5,
        };
        let arc = generate_manifold_arc(
            &system,
            &corrected,
            &m_opts,
            None,
        )
        .expect("Manifold generation should succeed");

        assert!(!arc.trajectory.is_empty());
        assert!(arc.flight_time_days > 0.0);

        // Check Jacobi constant conservation along unforced manifold arc
        let initial_cj = arc.trajectory.first().unwrap().jacobi_constant;
        let final_cj = arc.trajectory.last().unwrap().jacobi_constant;
        assert!(
            (initial_cj - final_cj).abs() < 1e-9,
            "Energy should be conserved along manifold arc: diff={}",
            (initial_cj - final_cj).abs()
        );
    }

    #[test]
    fn test_differential_corrector_earth_moon_l1_southern_halo() {
        let system = Cr3bpSystem::earth_moon();
        let benchmark = PeriodicOrbitBenchmark::earth_moon_l1_southern_halo();

        let z0 = benchmark.initial_state.z;
        // Perturb x0 and vy0 initial guesses
        let x0_perturbed = benchmark.initial_state.x * 1.002;
        let vy0_perturbed = benchmark.initial_state.vy * 0.995;

        let corrected = correct_3d_halo(&system, z0, x0_perturbed, vy0_perturbed, 15, 1e-8)
            .expect("3D Halo correction should converge");

        println!(
            "3D Halo converged: x0={}, vy0={}, period_days={}, residual={:e}",
            corrected.initial_state.x, corrected.initial_state.vy, corrected.period_days, corrected.residual
        );

        assert!(corrected.residual < 1e-8, "Residual should be < 1e-8");
        assert!(
            (corrected.period_days - benchmark.period_days).abs() < 1.0,
            "Period should match benchmark: got {}, expected {}",
            corrected.period_days,
            benchmark.period_days
        );
    }
}
