//! Successive Convexification (SCvx) Trajectory Optimizer for CR3BP Low-Thrust Transfers.
//!
//! Grounded in:
//! - **Short, Haapala, & Bosanac (2020)**. *Implementation of CR3BP & Low-Energy Transfers.* AAS 20-459.
//! - **Mao, Szmuk, & Açıkmeşe (2016)**. *Successive Convexification of Non-Convex Optimal Control Problems.* arXiv:1608.05133.
//! - **Malyuta, Reynolds, Szmuk, et al. (2022)**. *Convex Optimization for Trajectory Generation Tutorial.* IEEE CSM, arXiv:2106.09125.

use crate::cr3bp::dynamics::{equations_of_motion, variational_matrix};
use crate::cr3bp::types::{Cr3bpState, Cr3bpSystem};
use crate::scvx::admm::LuSolver;

/// High-level operator parameters for a deep-space transfer.
#[derive(Debug, Clone, PartialEq)]
pub struct Cr3bpTransferMissionConfig {
    /// Spacecraft wet mass in kg (e.g., 500.0 kg).
    pub wet_mass_kg: f64,
    /// Maximum engine thrust in Newtons (e.g., 0.5 N for low-thrust ion engine).
    pub max_thrust_n: f64,
    /// Specific impulse in seconds (e.g., 2500.0 s for Hall thruster).
    pub isp_s: f64,
    /// Desired flight time in days.
    pub flight_days: f64,
    /// Number of temporal discretization nodes (e.g., 30 to 50).
    pub n_nodes: usize,
}

impl Default for Cr3bpTransferMissionConfig {
    fn default() -> Self {
        Self {
            wet_mass_kg: 500.0,
            max_thrust_n: 0.5,
            isp_s: 2500.0,
            flight_days: 28.0,
            n_nodes: 35,
        }
    }
}

/// A discrete trajectory node along the transfer.
#[derive(Debug, Clone, PartialEq)]
pub struct Cr3bpTransferNode {
    /// Flight time from departure in days.
    pub time_days: f64,
    /// Nondimensional simulation time.
    pub tau: f64,
    /// Cartesian position in rotating frame (km): [x, y, z].
    pub position_km: [f64; 3],
    /// Velocity in rotating frame (km/s): [vx, vy, vz].
    pub velocity_km_s: [f64; 3],
    /// Continuous thrust acceleration vector (m/s^2): [ax, ay, az].
    pub thrust_accel_m_s2: [f64; 3],
    /// Engine thrust magnitude in milliNewtons (mN).
    pub thrust_mn: f64,
    /// Cumulative Delta-V spent up to this node (m/s).
    pub cumulative_delta_v_m_s: f64,
}

/// An aggregated operational burn interval for the mission operator.
#[derive(Debug, Clone, PartialEq)]
pub struct Cr3bpBurnSegment {
    pub segment_index: usize,
    pub start_day: f64,
    pub end_day: f64,
    pub duration_hours: f64,
    pub average_thrust_mn: f64,
    pub delta_v_m_s: f64,
    pub fuel_consumed_kg: f64,
}

/// Complete solution and mission plan for the operator.
#[derive(Debug, Clone, PartialEq)]
pub struct Cr3bpTransferPlan {
    pub converged: bool,
    pub iterations: usize,
    pub total_flight_days: f64,
    pub total_delta_v_m_s: f64,
    pub total_fuel_consumed_kg: f64,
    pub final_mass_kg: f64,
    pub max_thrust_used_mn: f64,
    pub nodes: Vec<Cr3bpTransferNode>,
    pub burn_schedule: Vec<Cr3bpBurnSegment>,
}

/// Successive Convexification engine for CR3BP transfers.
pub struct Cr3bpTransferOptimizer {
    pub system: Cr3bpSystem,
    pub origin_state: Cr3bpState,
    pub target_state: Cr3bpState,
    pub config: Cr3bpTransferMissionConfig,
}

impl Cr3bpTransferOptimizer {
    pub fn new(
        system: Cr3bpSystem,
        origin_state: Cr3bpState,
        target_state: Cr3bpState,
        config: Cr3bpTransferMissionConfig,
    ) -> Self {
        Self {
            system,
            origin_state,
            target_state,
            config,
        }
    }

    /// Evaluates equations of motion including low-thrust acceleration in rotating frame.
    pub fn dynamics_with_thrust(&self, s: &[f64; 6], u: &[f64; 3]) -> [f64; 6] {
        let state = Cr3bpState::new(s[0], s[1], s[2], s[3], s[4], s[5]);
        let unforced = equations_of_motion(&self.system, &state);

        [
            unforced[0],
            unforced[1],
            unforced[2],
            unforced[3] + u[0],
            unforced[4] + u[1],
            unforced[5] + u[2],
        ]
    }

    /// Evaluates the 6x6 state variational matrix A and 6x3 control matrix B.
    pub fn linearization(&self, s: &[f64; 6], u: &[f64; 3]) -> ([[f64; 6]; 6], [[f64; 3]; 6], [f64; 6]) {
        let a = variational_matrix(&self.system, s[0], s[1], s[2]);
        let mut b = [[0.0; 3]; 6];
        b[3][0] = 1.0;
        b[4][1] = 1.0;
        b[5][2] = 1.0;

        let f = self.dynamics_with_thrust(s, u);
        let mut r = [0.0; 6];
        for i in 0..6 {
            let mut lin_i = 0.0;
            for j in 0..6 {
                lin_i += a[i][j] * s[j];
            }
            for j in 0..3 {
                lin_i += b[i][j] * u[j];
            }
            r[i] = f[i] - lin_i;
        }

        (a, b, r)
    }

    /// Solves the optimal low-thrust transfer using SCvx and returns the operational mission plan.
    #[allow(clippy::needless_range_loop)]
    pub fn optimize(&self) -> Result<Cr3bpTransferPlan, String> {
        let n = self.config.n_nodes;
        let t_star_seconds = self.system.t_star;
        let l_star_km = self.system.l_star / 1000.0;
        let v_star_km_s = l_star_km / t_star_seconds;
        let a_star_m_s2 = self.system.l_star / (t_star_seconds * t_star_seconds);

        // Nondimensional flight duration:
        let total_time_s = self.config.flight_days * 86400.0;
        let total_tau = total_time_s / t_star_seconds;
        let dt = total_tau / (n - 1) as f64;

        // Maximum acceleration in nondimensional units:
        let accel_max_m_s2 = self.config.max_thrust_n / self.config.wet_mass_kg;
        let u_max_nondim = accel_max_m_s2 / a_star_m_s2;

        let s_init = [
            self.origin_state.x,
            self.origin_state.y,
            self.origin_state.z,
            self.origin_state.vx,
            self.origin_state.vy,
            self.origin_state.vz,
        ];

        let s_final = [
            self.target_state.x,
            self.target_state.y,
            self.target_state.z,
            self.target_state.vx,
            self.target_state.vy,
            self.target_state.vz,
        ];

        // 1. Initial trajectory guess: curved arc avoiding lunar body singularity
        let mut curr_states = vec![vec![0.0; 6]; n];
        let mut curr_controls = vec![vec![0.0; 3]; n - 1];

        for (k, state) in curr_states.iter_mut().enumerate() {
            let frac = k as f64 / (n - 1) as f64;
            for i in 0..6 {
                state[i] = s_init[i] + frac * (s_final[i] - s_init[i]);
            }
            // Out-of-plane or in-plane arc to clear the Moon (at x = 1 - mu, y = 0)
            let arc = 0.08 * (std::f64::consts::PI * frac).sin();
            state[1] += arc;
        }

        let nx = 6;
        let nu = 3;
        let n_vars = n * nx + (n - 1) * nu;
        let n_eq = (n - 1) * nx + 2 * nx;
        let total_dim = n_vars + n_eq;

        let mut trust_region: f64 = 0.05;
        let mut converged = false;
        let mut total_iterations = 0;

        // Outer SCvx iteration loop
        for iter in 0..12 {
            total_iterations = iter + 1;

            // Form KKT matrix for subproblem
            let mut kkt = vec![vec![0.0; total_dim]; total_dim];
            let w_trust = 1.0 / (trust_region * trust_region).max(1e-4_f64);
            let rho_admm = 5.0;

            for k in 0..n {
                for i in 0..nx {
                    kkt[k * nx + i][k * nx + i] = w_trust;
                }
            }

            let u_offset = n * nx;
            for k in 0..n - 1 {
                for j in 0..nu {
                    kkt[u_offset + k * nu + j][u_offset + k * nu + j] = rho_admm;
                }
            }

            // Constraints C * z = d
            let eq_offset = n_vars;
            // s(0) = s_init
            for i in 0..nx {
                kkt[eq_offset + i][i] = 1.0;
                kkt[i][eq_offset + i] = 1.0;
            }

            // Dynamics constraints
            let mut curr_eq = eq_offset + nx;
            let mut a_mats = Vec::with_capacity(n - 1);
            let mut b_mats = Vec::with_capacity(n - 1);
            let mut r_offsets = Vec::with_capacity(n - 1);

            for k in 0..n - 1 {
                let s_k: [f64; 6] = curr_states[k].as_slice().try_into().unwrap();
                let u_k: [f64; 3] = curr_controls[k].as_slice().try_into().unwrap();
                let (a, b, r) = self.linearization(&s_k, &u_k);

                for i in 0..nx {
                    let row = curr_eq + i;
                    // + s_{k+1}
                    kkt[row][(k + 1) * nx + i] = 1.0;
                    kkt[(k + 1) * nx + i][row] = 1.0;

                    // - (I + dt * A_k) * s_k
                    for j in 0..nx {
                        let val = if i == j { 1.0 } else { 0.0 } + dt * a[i][j];
                        kkt[row][k * nx + j] = -val;
                        kkt[k * nx + j][row] = -val;
                    }

                    // - dt * B_k * u_k
                    for j in 0..nu {
                        let val = dt * b[i][j];
                        kkt[row][u_offset + k * nu + j] = -val;
                        kkt[u_offset + k * nu + j][row] = -val;
                    }
                }

                curr_eq += nx;
                a_mats.push(a);
                b_mats.push(b);
                r_offsets.push(r);
            }

            // s(N-1) = s_final
            for i in 0..nx {
                let row = curr_eq + i;
                let col = (n - 1) * nx + i;
                kkt[row][col] = 1.0;
                kkt[col][row] = 1.0;
            }

            // Factorize KKT system
            let lu_solver = LuSolver::decompose(kkt)?;

            // Base RHS
            let mut rhs_base = vec![0.0; total_dim];
            for k in 0..n {
                for i in 0..nx {
                    rhs_base[k * nx + i] = w_trust * curr_states[k][i];
                }
            }
            rhs_base[eq_offset..(eq_offset + nx)].copy_from_slice(&s_init[..nx]);

            curr_eq = eq_offset + nx;
            for r in &r_offsets {
                for i in 0..nx {
                    rhs_base[curr_eq + i] = dt * r[i];
                }
                curr_eq += nx;
            }
            rhs_base[curr_eq..(curr_eq + nx)].copy_from_slice(&s_final[..nx]);

            // Projected ADMM
            let mut v_aux = curr_controls.clone();
            let mut mu_dual = vec![vec![0.0; nu]; n - 1];
            let mut sol = vec![0.0; total_dim];

            for _ in 0..20 {
                let mut rhs = rhs_base.clone();
                for k in 0..n - 1 {
                    for j in 0..nu {
                        rhs[u_offset + k * nu + j] = rho_admm * (v_aux[k][j] - mu_dual[k][j]);
                    }
                }

                sol = lu_solver.solve(&rhs);

                for k in 0..n - 1 {
                    let mut target = [0.0; 3];
                    for j in 0..3 {
                        target[j] = sol[u_offset + k * nu + j] + mu_dual[k][j];
                    }

                    let t_norm = (target[0] * target[0] + target[1] * target[1] + target[2] * target[2]).sqrt();
                    if t_norm <= dt / rho_admm {
                        v_aux[k] = vec![0.0; 3];
                    } else {
                        let s_mag = (t_norm - dt / rho_admm).min(u_max_nondim);
                        let scale = s_mag / t_norm;
                        for j in 0..3 {
                            v_aux[k][j] = target[j] * scale;
                        }
                    }

                    for j in 0..3 {
                        mu_dual[k][j] += sol[u_offset + k * nu + j] - v_aux[k][j];
                    }
                }
            }

            // Check step update size
            let mut state_inc_max: f64 = 0.0;
            for k in 0..n {
                let mut d_sq = 0.0;
                for i in 0..nx {
                    let d = sol[k * nx + i] - curr_states[k][i];
                    d_sq += d * d;
                }
                state_inc_max = state_inc_max.max(d_sq.sqrt());
            }

            // Update nominal trajectory
            for k in 0..n {
                for i in 0..nx {
                    curr_states[k][i] = sol[k * nx + i];
                }
            }
            curr_controls = v_aux;

            trust_region = (trust_region * 1.2_f64).min(0.20_f64);

            tracing::info!(
                iteration = iter + 1,
                state_inc_max,
                trust_region,
                "CR3BP SCvx iteration progress"
            );

            if state_inc_max < 0.015 {
                converged = true;
                break;
            }
        }

        // Convert nondimensional solution to operator deliverables
        let g0 = 9.80665;
        let mut nodes = Vec::with_capacity(n);
        let mut cumulative_dv = 0.0;

        for k in 0..n {
            let tau = k as f64 * dt;
            let time_days = (tau * t_star_seconds) / 86400.0;

            let pos_km = [
                curr_states[k][0] * l_star_km,
                curr_states[k][1] * l_star_km,
                curr_states[k][2] * l_star_km,
            ];

            let vel_km_s = [
                curr_states[k][3] * v_star_km_s,
                curr_states[k][4] * v_star_km_s,
                curr_states[k][5] * v_star_km_s,
            ];

            let (accel_m_s2, thrust_mn) = if k < n - 1 {
                let ax = curr_controls[k][0] * a_star_m_s2;
                let ay = curr_controls[k][1] * a_star_m_s2;
                let az = curr_controls[k][2] * a_star_m_s2;
                let a_mag = (ax * ax + ay * ay + az * az).sqrt();
                let f_n = a_mag * self.config.wet_mass_kg;
                let dt_seconds = dt * t_star_seconds;
                cumulative_dv += a_mag * dt_seconds;
                ([ax, ay, az], f_n * 1000.0)
            } else {
                ([0.0, 0.0, 0.0], 0.0)
            };

            nodes.push(Cr3bpTransferNode {
                time_days,
                tau,
                position_km: pos_km,
                velocity_km_s: vel_km_s,
                thrust_accel_m_s2: accel_m_s2,
                thrust_mn,
                cumulative_delta_v_m_s: cumulative_dv,
            });
        }

        // Aggregate into operator burn segments
        let mut burn_schedule = Vec::new();
        let mut segment_idx = 1;
        let dt_days = self.config.flight_days / (n - 1) as f64;

        for (k, ctrl) in curr_controls.iter().enumerate() {
            let a_mag = (ctrl[0] * ctrl[0] + ctrl[1] * ctrl[1] + ctrl[2] * ctrl[2]).sqrt() * a_star_m_s2;
            let f_mn = a_mag * self.config.wet_mass_kg * 1000.0;

            if f_mn > 1.0 {
                let start_day = k as f64 * dt_days;
                let end_day = (k + 1) as f64 * dt_days;
                let duration_hours = dt_days * 24.0;
                let dv = a_mag * (dt * t_star_seconds);
                let fuel = self.config.wet_mass_kg * (1.0 - (-dv / (self.config.isp_s * g0)).exp());

                burn_schedule.push(Cr3bpBurnSegment {
                    segment_index: segment_idx,
                    start_day,
                    end_day,
                    duration_hours,
                    average_thrust_mn: f_mn,
                    delta_v_m_s: dv,
                    fuel_consumed_kg: fuel,
                });
                segment_idx += 1;
            }
        }

        let total_fuel_consumed_kg = self.config.wet_mass_kg * (1.0 - (-cumulative_dv / (self.config.isp_s * g0)).exp());
        let final_mass_kg = self.config.wet_mass_kg - total_fuel_consumed_kg;
        let max_thrust_used_mn = nodes.iter().map(|n| n.thrust_mn).fold(0.0, f64::max);

        Ok(Cr3bpTransferPlan {
            converged,
            iterations: total_iterations,
            total_flight_days: self.config.flight_days,
            total_delta_v_m_s: cumulative_dv,
            total_fuel_consumed_kg,
            final_mass_kg,
            max_thrust_used_mn,
            nodes,
            burn_schedule,
        })
    }
}
