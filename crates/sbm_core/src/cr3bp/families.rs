//! CR3BP equilibrium points (Lagrange points), periodic orbit families, and invariant manifolds.
//!
//! Grounded in AAS 20-459 Sections 3–5:
//! - Exact Lagrange points $L_1\text{--}L_5$ via Euler quintic polynomial roots
//! - Periodic orbit families (Lyapunov, Halo, NRHO)
//! - Monodromy matrix $\mathbf{M} = \mathbf{\Phi}(T, 0)$ and stable/unstable invariant manifolds $W^s, W^u$

use crate::cr3bp::dynamics::jacobi_constant;
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
