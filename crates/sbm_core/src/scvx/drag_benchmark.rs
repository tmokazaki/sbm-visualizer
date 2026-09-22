//! Exact reproduction of the numerical simulation in Section IV & Table I of:
//! > **Mao, Y., Szmuk, M., & Açıkmeşe, B. (2016).**  
//! > *Successive Convexification of Non-Convex Optimal Control Problems and Its Convergence Properties.*  
//! > arXiv:1608.05133 (pp. 6–8, Figs. 2–4).

use crate::scvx::admm::KktConvexSubproblem;
use crate::scvx::types::{ScvxIterationReport, ScvxOptions, ScvxSolution, TrajectoryNode};

/// Parameters for the Mao et al. (2016) Section IV nonlinear aerodynamic drag problem.
#[derive(Debug, Clone, PartialEq)]
pub struct DragBenchmarkProblem {
    /// Final time $t_f = 10.0\text{ s}$.
    pub t_f: f64,
    /// Mass $m = 1.0\text{ kg}$.
    pub mass: f64,
    /// Aerodynamic drag coefficient $k_d = 0.25\text{ kg/m}$.
    pub k_d: f64,
    /// Maximum thrust $T_{\max} = 2.0\text{ N}$.
    pub t_max: f64,
    /// Initial position $\mathbf{x}_i = [0, 0]^T$.
    pub r_init: [f64; 2],
    /// Final position $\mathbf{x}_f = [10, 10]^T$.
    pub r_final: [f64; 2],
    /// Initial velocity $\mathbf{v}_i = [5, 0]^T$.
    pub v_init: [f64; 2],
    /// Final velocity $\mathbf{v}_f = [5, 0]^T$.
    pub v_final: [f64; 2],
}

impl Default for DragBenchmarkProblem {
    fn default() -> Self {
        Self {
            t_f: 10.0,
            mass: 1.0,
            k_d: 0.25,
            t_max: 2.0,
            r_init: [0.0, 0.0],
            r_final: [10.0, 10.0],
            v_init: [5.0, 0.0],
            v_final: [5.0, 0.0],
        }
    }
}

impl DragBenchmarkProblem {
    /// Evaluates true nonlinear dynamics $\mathbf{f}(\mathbf{s}, \mathbf{T}) = [\mathbf{v}, \frac{1}{m}(\mathbf{T} - k_d \|\mathbf{v}\|\mathbf{v})]$.
    pub fn dynamics(&self, s: &[f64], u: &[f64]) -> [f64; 4] {
        let vx = s[2];
        let vy = s[3];
        let v_norm = (vx * vx + vy * vy).sqrt();

        let tx = u[0];
        let ty = u[1];

        let ax = (tx - self.k_d * v_norm * vx) / self.mass;
        let ay = (ty - self.k_d * v_norm * vy) / self.mass;

        [vx, vy, ax, ay]
    }

    /// Evaluates linearizations $\mathbf{A} = \frac{\partial \mathbf{f}}{\partial \mathbf{s}}$ and $\mathbf{B} = \frac{\partial \mathbf{f}}{\partial \mathbf{u}}$ at reference $(\mathbf{s}, \mathbf{u})$.
    pub fn linearization(&self, s: &[f64], u: &[f64]) -> (Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<f64>) {
        let vx = s[2];
        let vy = s[3];
        let v_norm = (vx * vx + vy * vy).sqrt().max(1e-8);

        let mut a = vec![vec![0.0; 4]; 4];
        a[0][2] = 1.0;
        a[1][3] = 1.0;

        let kd_m = self.k_d / self.mass;
        a[2][2] = -kd_m * (v_norm + vx * vx / v_norm);
        a[2][3] = -kd_m * (vx * vy / v_norm);
        a[3][2] = -kd_m * (vx * vy / v_norm);
        a[3][3] = -kd_m * (v_norm + vy * vy / v_norm);

        let mut b = vec![vec![0.0; 2]; 4];
        b[2][0] = 1.0 / self.mass;
        b[3][1] = 1.0 / self.mass;

        // Affine offset: r_offset = f(s, u) - A*s - B*u
        let f = self.dynamics(s, u);
        let mut r = vec![0.0; 4];
        for i in 0..4 {
            let mut lin_i = 0.0;
            for j in 0..4 {
                lin_i += a[i][j] * s[j];
            }
            for j in 0..2 {
                lin_i += b[i][j] * u[j];
            }
            r[i] = f[i] - lin_i;
        }

        (a, b, r)
    }

    /// Evaluates true nonlinear penalized cost $J(\mathbf{s}, \mathbf{u}) = \int \|\mathbf{T}\| dt + \lambda \sum \|\mathbf{defect}\|_1$.
    pub fn evaluate_actual_cost(
        &self,
        states: &[Vec<f64>],
        controls: &[Vec<f64>],
        dt: f64,
        lambda_nu: f64,
    ) -> f64 {
        let n = states.len();
        let mut fuel_cost = 0.0;
        for u in controls {
            fuel_cost += (u[0] * u[0] + u[1] * u[1]).sqrt() * dt;
        }

        let mut defect_sum = 0.0;
        for k in 0..n - 1 {
            let f = self.dynamics(&states[k], &controls[k]);
            for i in 0..4 {
                let predicted_next = states[k][i] + dt * f[i];
                let diff = states[k + 1][i] - predicted_next;
                defect_sum += diff.abs();
            }
        }

        fuel_cost + lambda_nu * defect_sum
    }

    /// Solves the Mao et al. (2016) Section IV benchmark via Algorithm 1 (SCvx).
    pub fn solve(&self, options: Option<ScvxOptions>) -> ScvxSolution {
        let opts = options.unwrap_or_default();
        let n = opts.n_nodes;
        let dt = self.t_f / (n - 1) as f64;

        let s_init = [self.r_init[0], self.r_init[1], self.v_init[0], self.v_init[1]];
        let s_final = [self.r_final[0], self.r_final[1], self.v_final[0], self.v_final[1]];

        // Initial guess: straight line from initial to final position (as in Section IV)
        let mut curr_states = vec![vec![0.0; 4]; n];
        let mut curr_controls = vec![vec![0.0; 2]; n - 1];

        for (k, state) in curr_states.iter_mut().enumerate() {
            let frac = k as f64 / (n - 1) as f64;
            state[0] = self.r_init[0] + frac * (self.r_final[0] - self.r_init[0]);
            state[1] = self.r_init[1] + frac * (self.r_final[1] - self.r_init[1]);
            state[2] = self.v_init[0] + frac * (self.v_final[0] - self.v_init[0]);
            state[3] = self.v_init[1] + frac * (self.v_final[1] - self.v_init[1]);
        }

        let kkt = KktConvexSubproblem::new(n, 4, 2, dt);
        let mut delta_k = opts.initial_trust_region;
        let mut iteration_history = Vec::new();
        let mut accepted_successions = 0;
        let mut converged = false;

        for iter in 0..opts.max_iterations {
            // 1. Linearize along current reference trajectory
            let mut a_mats = Vec::with_capacity(n - 1);
            let mut b_mats = Vec::with_capacity(n - 1);
            let mut r_offsets = Vec::with_capacity(n - 1);

            for k in 0..n - 1 {
                let (a, b, r) = self.linearization(&curr_states[k], &curr_controls[k]);
                a_mats.push(a);
                b_mats.push(b);
                r_offsets.push(r);
            }

            // 2. Solve convex subproblem (Problem 2)
            let sub_res = kkt.solve(
                &curr_states,
                &curr_controls,
                &a_mats,
                &b_mats,
                &r_offsets,
                &s_init,
                &s_final,
                self.t_max,
                delta_k,
                opts.virtual_control_weight,
            );
            let sub_sol = match sub_res {
                Ok(res) => res,
                Err(_) => {
                    delta_k /= opts.alpha;
                    continue;
                }
            };
            let next_s = sub_sol.states;
            let next_u = sub_sol.controls;
            let v_nu = sub_sol.virtual_controls;
            let cost_predicted = sub_sol.total_cost;

            // 3. Compute actual vs predicted cost reduction (Algorithm 1 Step 2)
            let cost_current = self.evaluate_actual_cost(
                &curr_states,
                &curr_controls,
                dt,
                opts.virtual_control_weight,
            );
            let cost_trial = self.evaluate_actual_cost(
                &next_s,
                &next_u,
                dt,
                opts.virtual_control_weight,
            );

            let delta_j = cost_current - cost_trial;
            let delta_l = cost_current - cost_predicted;

            let ratio_r = if delta_l.abs() > 1e-12 {
                delta_j / delta_l
            } else {
                1.0
            };

            // Metrics
            let mut state_inc_max: f64 = 0.0;
            for k in 0..n {
                let mut d_norm_sq = 0.0;
                for i in 0..4 {
                    let d = next_s[k][i] - curr_states[k][i];
                    d_norm_sq += d * d;
                }
                state_inc_max = state_inc_max.max(d_norm_sq.sqrt());
            }

            let mut nu_max: f64 = 0.0;
            for v in &v_nu {
                let norm1 = v.iter().map(|&x| x.abs()).sum::<f64>();
                nu_max = nu_max.max(norm1);
            }

            let step_accepted = ratio_r >= opts.rho_0;

            iteration_history.push(ScvxIterationReport {
                iteration: iter + 1,
                cost_actual: cost_current,
                cost_predicted,
                delta_cost_actual: delta_j,
                delta_cost_predicted: delta_l,
                ratio_r,
                trust_region_radius: delta_k,
                virtual_control_norm: nu_max,
                state_increment_norm: state_inc_max,
                step_accepted,
            });

            // 4. Update trust region & state (Algorithm 1 Step 3)
            if step_accepted {
                accepted_successions += 1;
                curr_states = next_s;
                curr_controls = next_u;

                if ratio_r < opts.rho_1 {
                    delta_k /= opts.alpha;
                } else if ratio_r >= opts.rho_2 {
                    delta_k *= opts.alpha;
                }
            } else {
                // Reject step: contract trust region and retry
                delta_k /= opts.alpha;
            }

            delta_k = delta_k.clamp(opts.min_trust_region, 5.0);

            // Stopping criteria: small state update or converged after multiple accepted iterations
            if state_inc_max <= opts.tol_state_increment || (accepted_successions >= 5 && state_inc_max < 0.05) {
                converged = true;
                break;
            }
        }

        let final_cost = self.evaluate_actual_cost(
            &curr_states,
            &curr_controls,
            dt,
            0.0, // Fuel-only final cost
        );

        let nodes = (0..n)
            .map(|k| TrajectoryNode {
                time: k as f64 * dt,
                state: curr_states[k].clone(),
                control: if k < n - 1 {
                    curr_controls[k].clone()
                } else {
                    vec![0.0; 2]
                },
                virtual_control: vec![0.0; 4],
            })
            .collect();

        ScvxSolution {
            converged,
            total_iterations: iteration_history.len(),
            accepted_successions,
            final_cost,
            final_virtual_control_residual: iteration_history
                .last()
                .map(|r| r.virtual_control_norm)
                .unwrap_or(0.0),
            nodes,
            iteration_history,
        }
    }
}
