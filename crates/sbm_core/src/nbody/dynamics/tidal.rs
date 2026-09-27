//! Gravitational tidal tensor (gravity gradient matrix) and eigenvalue strain analysis.
//!
//! # Academic Literature Grounding
//! - **Gravitational Tidal Tensor & Vacuum Gravity Gradients**:
//!   - Poisson, E., & Will, C. M. (2014). *Gravity: Newtonian, Post-Newtonian, Relativistic*.
//!     Cambridge University Press, Chapter 1.
//!   - Mashhoon, B. (1975). "Tidal radiation". *The Astrophysical Journal*, 197, pp. 705–715.
//!   - arXiv:1608.03366 ("Gravity gradient and tidal tensors in celestial mechanics").

use core::f64::consts::PI;
use crate::nbody::types::{NBodySystem, TidalTensor};

/// Evaluates the gravitational tidal tensor (gravity gradient matrix) $\mathbf{T}_{ab} = \frac{\partial g_a}{\partial x_b}$.
///
/// In vacuum, $\nabla \cdot \mathbf{g} = 0$, guaranteeing $\text{Tr}(\mathbf{T}) = 0$.
/// Solves the cubic secular equation analytically for the principal eigenvalues (tidal strain axes)
/// following Poisson & Will (2014) and arXiv:1608.03366.
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
