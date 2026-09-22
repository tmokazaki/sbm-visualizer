//! Self-contained, zero-dependency KKT convex subproblem solver for Successive Convexification (SCvx).
//!
//! Grounded in:
//! - **Mao, Y., Szmuk, M., & Açıkmeşe, B. (2016)**.
//!   *Successive Convexification of Non-Convex Optimal Control Problems and Its Convergence Properties.*
//!   arXiv:1608.05133 (Section II, Problem 2).
//! - **Malyuta, D., Reynolds, T. P., Szmuk, M., et al. (2022)**.
//!   *Convex Optimization for Trajectory Generation: A Tutorial on Generating Dynamically Feasible Trajectories Reliably and Efficiently.*
//!   IEEE Control Systems Magazine, 42(5), 40–113 (arXiv:2106.09125).
//!
//! Solves the convex subproblem at each succession:
//! - Exact linearized dynamics equality constraints: $\mathbf{s}_{k+1} - (\mathbf{I} + \Delta t \mathbf{A}_k)\mathbf{s}_k - \Delta t \mathbf{B}_k \mathbf{u}_k = \Delta t \mathbf{r}_k + \boldsymbol{\nu}_k$
//! - Exact boundary conditions: $\mathbf{s}_0 = \mathbf{s}_{\text{init}}$ and $\mathbf{s}_N = \mathbf{s}_{\text{target}}$
//! - Exact control saturation constraint: $\|\mathbf{u}_k\|_2 \le T_{\max}$ solved via Projected ADMM.
//! - Quadratic trust region regularization: $\min \frac{1}{2\Delta_k^2} \|\mathbf{s} - \bar{\mathbf{s}}\|^2$.

/// In-place LU decomposition with partial pivoting for repeated fast back-substitutions.
#[derive(Debug, Clone)]
pub struct LuSolver {
    lu: Vec<Vec<f64>>,
    pivots: Vec<usize>,
}

impl LuSolver {
    #[allow(clippy::needless_range_loop)]
    pub fn decompose(mut a: Vec<Vec<f64>>) -> Result<Self, String> {
        let n = a.len();
        let mut pivots = vec![0; n];
        for col in 0..n {
            let mut max_row = col;
            let mut max_val = a[col][col].abs();
            for row in (col + 1)..n {
                let val = a[row][col].abs();
                if val > max_val {
                    max_val = val;
                    max_row = row;
                }
            }

            if max_val < 1e-15 {
                return Err(format!("Singular matrix during LU decomposition at col {}", col));
            }

            pivots[col] = max_row;
            if max_row != col {
                a.swap(col, max_row);
            }

            let pivot = a[col][col];
            for row in (col + 1)..n {
                let factor = a[row][col] / pivot;
                a[row][col] = factor;
                for j in (col + 1)..n {
                    a[row][j] -= factor * a[col][j];
                }
            }
        }

        Ok(Self { lu: a, pivots })
    }

    #[allow(clippy::needless_range_loop)]
    pub fn solve(&self, b: &[f64]) -> Vec<f64> {
        let n = b.len();
        let mut y = b.to_vec();
        for col in 0..n {
            let p = self.pivots[col];
            if p != col {
                y.swap(col, p);
            }
        }

        // Forward solve: L * y' = y (L has unit diagonal)
        for row in 0..n {
            for col in 0..row {
                y[row] -= self.lu[row][col] * y[col];
            }
        }

        // Back solve: U * x = y'
        let mut x = vec![0.0; n];
        for row in (0..n).rev() {
            let mut sum = y[row];
            for col in (row + 1)..n {
                sum -= self.lu[row][col] * x[col];
            }
            x[row] = sum / self.lu[row][row];
        }

        x
    }
}

/// Solves a linear system $\mathbf{M} \mathbf{x} = \mathbf{b}$ via Gaussian elimination with partial pivoting.
pub fn solve_linear_system(a: &mut [Vec<f64>], b: &mut [f64]) -> Result<Vec<f64>, String> {
    let lu = LuSolver::decompose(a.to_vec())?;
    Ok(lu.solve(b))
}

/// Solution of an SCvx convex subproblem.
#[derive(Debug, Clone)]
pub struct SubproblemSolution {
    pub states: Vec<Vec<f64>>,
    pub controls: Vec<Vec<f64>>,
    pub virtual_controls: Vec<Vec<f64>>,
    pub total_cost: f64,
}

/// Solves the convex subproblem at each succession via KKT system solution and Projected ADMM.
pub struct KktConvexSubproblem {
    pub n_nodes: usize,
    pub state_dim: usize,
    pub control_dim: usize,
    pub dt: f64,
}

impl KktConvexSubproblem {
    pub fn new(n_nodes: usize, state_dim: usize, control_dim: usize, dt: f64) -> Self {
        Self {
            n_nodes,
            state_dim,
            control_dim,
            dt,
        }
    }

    /// Solves the linearized subproblem with exact control saturation bound $T_{\max}$ via Projected ADMM.
    #[allow(clippy::too_many_arguments, clippy::needless_range_loop)]
    pub fn solve(
        &self,
        ref_states: &[Vec<f64>],
        ref_controls: &[Vec<f64>],
        a_matrices: &[Vec<Vec<f64>>],
        b_matrices: &[Vec<Vec<f64>>],
        r_offsets: &[Vec<f64>],
        s_init: &[f64],
        s_target: &[f64],
        u_max: f64,
        trust_region: f64,
        lambda_nu: f64,
    ) -> Result<SubproblemSolution, String> {
        let n = self.n_nodes;
        let nx = self.state_dim;
        let nu = self.control_dim;
        let dt = self.dt;

        let n_vars = n * nx + (n - 1) * nu;
        let n_eq = (n - 1) * nx + 2 * nx; // dynamics + s(0) + s(N-1)
        let total_dim = n_vars + n_eq;

        let mut kkt = vec![vec![0.0; total_dim]; total_dim];

        // 1. Hessian H:
        // Minimize 1/2 * (1/delta^2) * ||s - s_ref||^2 + 1/2 * rho_admm * ||u - v + mu||^2
        let w_trust = 1.0 / (trust_region * trust_region).max(1e-4);
        let rho_admm = 5.0;

        for k in 0..n {
            for i in 0..nx {
                let idx = k * nx + i;
                kkt[idx][idx] = w_trust;
            }
        }

        let u_offset = n * nx;
        for k in 0..n - 1 {
            for j in 0..nu {
                let idx = u_offset + k * nu + j;
                kkt[idx][idx] = rho_admm;
            }
        }

        // 2. Linear equality constraints C * z = d:
        let eq_offset = n_vars;

        // Boundary s(0) = s_init
        for i in 0..nx {
            let row = eq_offset + i;
            let col = i;
            kkt[row][col] = 1.0;
            kkt[col][row] = 1.0;
        }

        // Dynamics: s_{k+1} - (I + dt*A_k)*s_k - dt*B_k*u_k = dt*r_k
        let mut curr_eq = eq_offset + nx;
        for k in 0..n - 1 {
            for i in 0..nx {
                let row = curr_eq + i;

                // + s_{k+1}
                let col_next = (k + 1) * nx + i;
                kkt[row][col_next] = 1.0;
                kkt[col_next][row] = 1.0;

                // - (I + dt * A_k) * s_k
                for j in 0..nx {
                    let col_curr = k * nx + j;
                    let val = if i == j { 1.0 } else { 0.0 } + dt * a_matrices[k][i][j];
                    kkt[row][col_curr] = -val;
                    kkt[col_curr][row] = -val;
                }

                // - dt * B_k * u_k
                for j in 0..nu {
                    let col_u = u_offset + k * nu + j;
                    let val = dt * b_matrices[k][i][j];
                    kkt[row][col_u] = -val;
                    kkt[col_u][row] = -val;
                }
            }
            curr_eq += nx;
        }

        // Boundary s(N-1) = s_target
        for i in 0..nx {
            let row = curr_eq + i;
            let col = (n - 1) * nx + i;
            kkt[row][col] = 1.0;
            kkt[col][row] = 1.0;
        }

        // Pre-factorize KKT system once for all ADMM iterations
        let lu_solver = LuSolver::decompose(kkt)?;

        // Base RHS (trust region state target + equality constraint rhs)
        let mut rhs_base = vec![0.0; total_dim];
        for k in 0..n {
            for i in 0..nx {
                rhs_base[k * nx + i] = w_trust * ref_states[k][i];
            }
        }

        rhs_base[eq_offset..(eq_offset + nx)].copy_from_slice(&s_init[..nx]);

        curr_eq = eq_offset + nx;
        for r_offset in r_offsets.iter().take(n - 1) {
            for i in 0..nx {
                rhs_base[curr_eq + i] = dt * r_offset[i];
            }
            curr_eq += nx;
        }

        rhs_base[curr_eq..(curr_eq + nx)].copy_from_slice(&s_target[..nx]);

        // ADMM initialization:
        let mut v_aux = ref_controls.to_vec();
        if v_aux.len() != n - 1 {
            v_aux = vec![vec![0.0; nu]; n - 1];
        }
        let mut mu_dual = vec![vec![0.0; nu]; n - 1];

        let mut final_sol = vec![0.0; total_dim];

        // Projected ADMM loop (25 iterations for high precision)
        for _ in 0..25 {
            // 1. Update RHS with ADMM proximal terms
            let mut rhs = rhs_base.clone();
            for k in 0..n - 1 {
                for j in 0..nu {
                    let idx = u_offset + k * nu + j;
                    rhs[idx] = rho_admm * (v_aux[k][j] - mu_dual[k][j]);
                }
            }

            // 2. Solve linear system via fast back-substitution
            final_sol = lu_solver.solve(&rhs);

            // 3. Project auxiliary control v onto ||v||_2 <= u_max with L2 shrinkage
            for k in 0..n - 1 {
                let mut target = vec![0.0; nu];
                for j in 0..nu {
                    target[j] = final_sol[u_offset + k * nu + j] + mu_dual[k][j];
                }

                let t_norm = (target[0] * target[0] + target[1] * target[1]).sqrt();
                if t_norm <= dt / rho_admm {
                    v_aux[k][0] = 0.0;
                    v_aux[k][1] = 0.0;
                } else {
                    let s_mag = (t_norm - dt / rho_admm).min(u_max);
                    let scale = s_mag / t_norm;
                    v_aux[k][0] = target[0] * scale;
                    v_aux[k][1] = target[1] * scale;
                }

                // 4. Update dual variable mu
                for j in 0..nu {
                    let u_sol_kj = final_sol[u_offset + k * nu + j];
                    mu_dual[k][j] += u_sol_kj - v_aux[k][j];
                }
            }
        }

        // Extract states and controls
        let mut next_s = vec![vec![0.0; nx]; n];
        for k in 0..n {
            for i in 0..nx {
                next_s[k][i] = final_sol[k * nx + i];
            }
        }

        let next_u = v_aux;
        let mut v_nu = vec![vec![0.0; nx]; n - 1];
        let mut fuel_cost = 0.0;

        for k in 0..n - 1 {
            let u_norm = (next_u[k][0] * next_u[k][0] + next_u[k][1] * next_u[k][1]).sqrt();
            fuel_cost += u_norm * dt;

            // Virtual defect: difference between raw linear solve and constrained projected control
            for i in 0..nx {
                let mut diff = 0.0;
                for j in 0..nu {
                    let u_raw_kj = final_sol[u_offset + k * nu + j];
                    diff += dt * b_matrices[k][i][j] * (u_raw_kj - next_u[k][j]);
                }
                v_nu[k][i] = diff;
            }
        }

        let v_cost: f64 = v_nu
            .iter()
            .map(|v| v.iter().map(|&x| x.abs()).sum::<f64>())
            .sum::<f64>();
        let total_cost = fuel_cost + lambda_nu * v_cost;

        Ok(SubproblemSolution {
            states: next_s,
            controls: next_u,
            virtual_controls: v_nu,
            total_cost,
        })
    }
}
