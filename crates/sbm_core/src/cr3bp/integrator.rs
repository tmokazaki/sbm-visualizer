//! Adaptive numerical integrators, event detection, and variational STM propagation for CR3BP.
//!
//! Grounded in AAS 20-459 Sections 3–5:
//! - High-precision adaptive Dormand-Prince 5(4) / Fehlberg integration
//! - Event detection for Poincaré sections and hyperplanes (e.g. $\Sigma: x = 1-\mu$, $y=0$)
//! - Maximum Jacobi constant variation (Eq. 13)
//! - Orbit closure measure (Eq. 14)

use crate::cr3bp::dynamics::{equations_of_motion, jacobi_constant, state_and_stm_derivatives};
use crate::cr3bp::types::{Cr3bpError, Cr3bpState, Cr3bpSystem};

/// Direction of zero-crossing for event detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventDirection {
    /// Trigger when crossing with increasing value ($g$ goes negative to positive).
    Positive,
    /// Trigger when crossing with decreasing value ($g$ goes positive to negative).
    Negative,
    /// Trigger on any zero-crossing.
    Either,
}

/// Event condition for stopping or recording during numerical propagation.
#[derive(Debug, Clone, PartialEq)]
pub enum EventCondition {
    /// Crossing of an $x$-hyperplane: $g(\mathbf{x}) = x - x_{target} = 0$.
    /// Crucial for Moon hyperplane $\Sigma: x = 1-\mu$ in AAS 20-459 Section 5.
    PlaneX { x_target: f64, direction: EventDirection },
    /// Crossing of the $y=0$ plane: $g(\mathbf{x}) = y = 0$.
    /// Crucial for symmetric periodic orbit differential corrections.
    PlaneY { y_target: f64, direction: EventDirection },
    /// Crossing of the $z=0$ plane: $g(\mathbf{x}) = z = 0$.
    PlaneZ { z_target: f64, direction: EventDirection },
}

impl EventCondition {
    /// Evaluates the event scalar function $g(\mathbf{x})$. Zero indicates the event condition.
    pub fn evaluate(&self, state: &Cr3bpState) -> f64 {
        match self {
            Self::PlaneX { x_target, .. } => state.x - x_target,
            Self::PlaneY { y_target, .. } => state.y - y_target,
            Self::PlaneZ { z_target, .. } => state.z - z_target,
        }
    }

    /// Checks if a transition from $g_{prev}$ to $g_{curr}$ constitutes a valid event crossing.
    pub fn is_triggered(&self, g_prev: f64, g_curr: f64) -> bool {
        let dir = match self {
            Self::PlaneX { direction, .. } => *direction,
            Self::PlaneY { direction, .. } => *direction,
            Self::PlaneZ { direction, .. } => *direction,
        };

        match dir {
            EventDirection::Positive => g_prev < 0.0 && g_curr >= 0.0,
            EventDirection::Negative => g_prev > 0.0 && g_curr <= 0.0,
            EventDirection::Either => (g_prev < 0.0 && g_curr >= 0.0) || (g_prev > 0.0 && g_curr <= 0.0),
        }
    }
}

/// Configuration options for the adaptive numerical integrator.
#[derive(Debug, Clone, PartialEq)]
pub struct IntegratorOptions {
    /// Relative tolerance for step size control. Default: $10^{-12}$.
    pub rel_tol: f64,
    /// Absolute tolerance for step size control. Default: $10^{-12}$.
    pub abs_tol: f64,
    /// Initial nondimensional step size. Default: $10^{-4}$.
    pub initial_step: f64,
    /// Minimum allowed step size. Default: $10^{-14}$.
    pub min_step: f64,
    /// Maximum allowed step size. Default: $0.1$.
    pub max_step: f64,
    /// Maximum number of integration steps before aborting. Default: 500,000.
    pub max_steps: usize,
}

impl Default for IntegratorOptions {
    fn default() -> Self {
        Self {
            rel_tol: 1e-12,
            abs_tol: 1e-12,
            initial_step: 1e-4,
            min_step: 1e-14,
            max_step: 0.1,
            max_steps: 500_000,
        }
    }
}

/// A point along the propagated trajectory.
#[derive(Debug, Clone, PartialEq)]
pub struct TrajectoryPoint {
    /// Nondimensional time $t$.
    pub t: f64,
    /// Nondimensional state $[x, y, z, v_x, v_y, v_z]$.
    pub state: Cr3bpState,
    /// Jacobi constant at this state $C_J(t)$.
    pub jacobi_constant: f64,
}

/// Summary result of a numerical propagation.
#[derive(Debug, Clone, PartialEq)]
pub struct PropagationResult {
    /// Full recorded trajectory points.
    pub trajectory: Vec<TrajectoryPoint>,
    /// Final nondimensional state.
    pub final_state: Cr3bpState,
    /// Final time reached.
    pub final_time: f64,
    /// Final 6x6 State Transition Matrix $\mathbf{\Phi}(t_f, t_0)$ (if STM propagation was requested).
    pub final_stm: Option<[[f64; 6]; 6]>,
    /// Maximum variation of the Jacobi constant along the orbit (Equation 13 in AAS 20-459):
    ///
    /// $$\text{Max}(\Delta C_J) = \max_i |C_{J, i} - C_{J, 0}|$$
    pub max_jacobi_variation: f64,
    /// Orbit closure measure (Equation 14 in AAS 20-459):
    ///
    /// $$|\Delta X| = \|\mathbf{x}(t_f) - \mathbf{x}(t_0)\|_2$$
    pub closure_norm: f64,
    /// Number of accepted steps.
    pub steps_accepted: usize,
    /// Number of rejected steps.
    pub steps_rejected: usize,
    /// Reason propagation stopped.
    pub stop_reason: StopReason,
}

/// Reason why numerical propagation terminated.
#[derive(Debug, Clone, PartialEq)]
pub enum StopReason {
    /// Reached target integration duration.
    TargetTimeReached,
    /// Reached specified event condition.
    EventTriggered(EventCondition),
    /// Integration hit maximum allowed steps limit.
    StepLimitExceeded,
    /// Spacecraft hit collision radius.
    Collision(String),
}

/// Dormand-Prince 5(4) Butcher tableau coefficients.
const _DP_C: [f64; 7] = [0.0, 1.0 / 5.0, 3.0 / 10.0, 4.0 / 5.0, 8.0 / 9.0, 1.0, 1.0];
const DP_A21: f64 = 1.0 / 5.0;
const DP_A31: f64 = 3.0 / 40.0;
const DP_A32: f64 = 9.0 / 40.0;
const DP_A41: f64 = 44.0 / 45.0;
const DP_A42: f64 = -56.0 / 15.0;
const DP_A43: f64 = 32.0 / 9.0;
const DP_A51: f64 = 19372.0 / 6561.0;
const DP_A52: f64 = -25360.0 / 2187.0;
const DP_A53: f64 = 64448.0 / 6561.0;
const DP_A54: f64 = -212.0 / 729.0;
const DP_A61: f64 = 9017.0 / 3168.0;
const DP_A62: f64 = -355.0 / 33.0;
const DP_A63: f64 = 46732.0 / 5247.0;
const DP_A64: f64 = 49.0 / 176.0;
const DP_A65: f64 = -5103.0 / 18656.0;
const _DP_A71: f64 = 35.0 / 384.0;
const _DP_A73: f64 = 500.0 / 1113.0;
const _DP_A74: f64 = 125.0 / 192.0;
const _DP_A75: f64 = -2187.0 / 6784.0;
const _DP_A76: f64 = 11.0 / 84.0;

// 5th order solution coefficients (b)
const DP_B1: f64 = 35.0 / 384.0;
const DP_B3: f64 = 500.0 / 1113.0;
const DP_B4: f64 = 125.0 / 192.0;
const DP_B5: f64 = -2187.0 / 6784.0;
const DP_B6: f64 = 11.0 / 84.0;

// Error estimation coefficients (b - b_hat)
const DP_E1: f64 = 71.0 / 57600.0;
const DP_E3: f64 = -71.0 / 16695.0;
const DP_E4: f64 = 71.0 / 1920.0;
const DP_E5: f64 = -17253.0 / 339200.0;
const DP_E6: f64 = 22.0 / 525.0;
const DP_E7: f64 = -1.0 / 40.0;

/// High-precision Dormand-Prince 5(4) adaptive numerical integrator.
pub struct DormandPrinceIntegrator<'a> {
    pub system: &'a Cr3bpSystem,
    pub options: IntegratorOptions,
}

impl<'a> DormandPrinceIntegrator<'a> {
    /// Creates a new integrator for the specified CR3BP system with given options.
    pub fn new(system: &'a Cr3bpSystem, options: IntegratorOptions) -> Self {
        Self { system, options }
    }

    /// Single adaptive step for 6D state vector.
    ///
    /// Returns `(next_state, next_time, actual_h, next_suggested_h, accepted)`.
    pub fn step_6d(
        &self,
        t: f64,
        y: &[f64; 6],
        h: f64,
    ) -> ([f64; 6], f64, f64, bool) {
        let state = Cr3bpState::from_array(*y);

        // Stage 1
        let k1 = equations_of_motion(self.system, &state);

        // Stage 2
        let mut y2 = [0.0; 6];
        for i in 0..6 {
            y2[i] = y[i] + h * DP_A21 * k1[i];
        }
        let k2 = equations_of_motion(self.system, &Cr3bpState::from_array(y2));

        // Stage 3
        let mut y3 = [0.0; 6];
        for i in 0..6 {
            y3[i] = y[i] + h * (DP_A31 * k1[i] + DP_A32 * k2[i]);
        }
        let k3 = equations_of_motion(self.system, &Cr3bpState::from_array(y3));

        // Stage 4
        let mut y4 = [0.0; 6];
        for i in 0..6 {
            y4[i] = y[i] + h * (DP_A41 * k1[i] + DP_A42 * k2[i] + DP_A43 * k3[i]);
        }
        let k4 = equations_of_motion(self.system, &Cr3bpState::from_array(y4));

        // Stage 5
        let mut y5 = [0.0; 6];
        for i in 0..6 {
            y5[i] = y[i] + h * (DP_A51 * k1[i] + DP_A52 * k2[i] + DP_A53 * k3[i] + DP_A54 * k4[i]);
        }
        let k5 = equations_of_motion(self.system, &Cr3bpState::from_array(y5));

        // Stage 6
        let mut y6 = [0.0; 6];
        for i in 0..6 {
            y6[i] = y[i]
                + h * (DP_A61 * k1[i]
                    + DP_A62 * k2[i]
                    + DP_A63 * k3[i]
                    + DP_A64 * k4[i]
                    + DP_A65 * k5[i]);
        }
        let k6 = equations_of_motion(self.system, &Cr3bpState::from_array(y6));

        // 5th order solution
        let mut y_next = [0.0; 6];
        for i in 0..6 {
            y_next[i] = y[i]
                + h * (DP_B1 * k1[i] + DP_B3 * k3[i] + DP_B4 * k4[i] + DP_B5 * k5[i] + DP_B6 * k6[i]);
        }

        // Stage 7 for error estimation
        let k7 = equations_of_motion(self.system, &Cr3bpState::from_array(y_next));

        // Error norm
        let mut err_sum = 0.0;
        for i in 0..6 {
            let err_i = h
                * (DP_E1 * k1[i]
                    + DP_E3 * k3[i]
                    + DP_E4 * k4[i]
                    + DP_E5 * k5[i]
                    + DP_E6 * k6[i]
                    + DP_E7 * k7[i]);
            let tol_i = self.options.abs_tol + self.options.rel_tol * y[i].abs().max(y_next[i].abs());
            let ratio = err_i / tol_i;
            err_sum += ratio * ratio;
        }
        let err_norm = (err_sum / 6.0).sqrt();

        // Step size adjustment factor
        let safety = 0.9;
        let fac_min = 0.2;
        let fac_max = 5.0;

        let scale = if err_norm > 0.0 {
            safety * (1.0 / err_norm).powf(0.2)
        } else {
            fac_max
        };
        let fac = scale.clamp(fac_min, fac_max);
        let next_h = (h * fac).clamp(self.options.min_step, self.options.max_step);

        if err_norm <= 1.0 {
            (y_next, t + h, next_h, true)
        } else {
            (*y, t, next_h, false)
        }
    }

    /// Single adaptive step for 42D state + STM vector.
    pub fn step_42d(
        &self,
        t: f64,
        y: &[f64; 42],
        h: f64,
    ) -> ([f64; 42], f64, f64, bool) {
        let k1 = state_and_stm_derivatives(self.system, y);

        let mut y2 = [0.0; 42];
        for i in 0..42 {
            y2[i] = y[i] + h * DP_A21 * k1[i];
        }
        let k2 = state_and_stm_derivatives(self.system, &y2);

        let mut y3 = [0.0; 42];
        for i in 0..42 {
            y3[i] = y[i] + h * (DP_A31 * k1[i] + DP_A32 * k2[i]);
        }
        let k3 = state_and_stm_derivatives(self.system, &y3);

        let mut y4 = [0.0; 42];
        for i in 0..42 {
            y4[i] = y[i] + h * (DP_A41 * k1[i] + DP_A42 * k2[i] + DP_A43 * k3[i]);
        }
        let k4 = state_and_stm_derivatives(self.system, &y4);

        let mut y5 = [0.0; 42];
        for i in 0..42 {
            y5[i] = y[i] + h * (DP_A51 * k1[i] + DP_A52 * k2[i] + DP_A53 * k3[i] + DP_A54 * k4[i]);
        }
        let k5 = state_and_stm_derivatives(self.system, &y5);

        let mut y6 = [0.0; 42];
        for i in 0..42 {
            y6[i] = y[i]
                + h * (DP_A61 * k1[i]
                    + DP_A62 * k2[i]
                    + DP_A63 * k3[i]
                    + DP_A64 * k4[i]
                    + DP_A65 * k5[i]);
        }
        let k6 = state_and_stm_derivatives(self.system, &y6);

        let mut y_next = [0.0; 42];
        for i in 0..42 {
            y_next[i] = y[i]
                + h * (DP_B1 * k1[i] + DP_B3 * k3[i] + DP_B4 * k4[i] + DP_B5 * k5[i] + DP_B6 * k6[i]);
        }

        let k7 = state_and_stm_derivatives(self.system, &y_next);

        // Error norm computed over position and velocity (first 6 elements)
        let mut err_sum = 0.0;
        for i in 0..6 {
            let err_i = h
                * (DP_E1 * k1[i]
                    + DP_E3 * k3[i]
                    + DP_E4 * k4[i]
                    + DP_E5 * k5[i]
                    + DP_E6 * k6[i]
                    + DP_E7 * k7[i]);
            let tol_i = self.options.abs_tol + self.options.rel_tol * y[i].abs().max(y_next[i].abs());
            let ratio = err_i / tol_i;
            err_sum += ratio * ratio;
        }
        let err_norm = (err_sum / 6.0).sqrt();

        let safety = 0.9;
        let fac_min = 0.2;
        let fac_max = 5.0;

        let scale = if err_norm > 0.0 {
            safety * (1.0 / err_norm).powf(0.2)
        } else {
            fac_max
        };
        let fac = scale.clamp(fac_min, fac_max);
        let next_h = (h * fac).clamp(self.options.min_step, self.options.max_step);

        if err_norm <= 1.0 {
            (y_next, t + h, next_h, true)
        } else {
            (*y, t, next_h, false)
        }
    }

    /// Propagates a 6D state forward or backward until `t_end` or until `event` is triggered.
    pub fn propagate_6d(
        &self,
        initial_state: &Cr3bpState,
        t_start: f64,
        t_end: f64,
        event: Option<&EventCondition>,
    ) -> Result<PropagationResult, Cr3bpError> {
        let mut t = t_start;
        let mut y = initial_state.to_array();
        let initial_cj = jacobi_constant(self.system, initial_state);

        let mut trajectory = Vec::new();
        trajectory.push(TrajectoryPoint {
            t,
            state: *initial_state,
            jacobi_constant: initial_cj,
        });

        let mut max_cj_diff = 0.0;
        let mut steps_accepted = 0;
        let mut steps_rejected = 0;
        let direction = if t_end >= t_start { 1.0 } else { -1.0 };
        let mut h = self.options.initial_step * direction;

        let mut prev_g = event.map(|ev| ev.evaluate(initial_state));
        let mut stop_reason = StopReason::TargetTimeReached;

        for step_idx in 0..self.options.max_steps {
            // Check if we reached target time
            if (direction > 0.0 && t >= t_end) || (direction < 0.0 && t <= t_end) {
                stop_reason = StopReason::TargetTimeReached;
                break;
            }

            // Cap step size to not overshoot t_end
            if (direction > 0.0 && t + h > t_end) || (direction < 0.0 && t + h < t_end) {
                h = t_end - t;
            }

            let (y_next, t_next, next_h_mag, accepted) = self.step_6d(t, &y, h);

            if accepted {
                steps_accepted += 1;
                let next_state = Cr3bpState::from_array(y_next);

                // Event detection
                if let (Some(ev), Some(g_p)) = (event, prev_g) {
                    let g_c = ev.evaluate(&next_state);
                    if ev.is_triggered(g_p, g_c) {
                        // Refine event crossing using bisection
                        let (t_event, state_event) = self.bisect_event_6d(
                            t,
                            &Cr3bpState::from_array(y),
                            t_next,
                            &next_state,
                            ev,
                        );
                        let cj_event = jacobi_constant(self.system, &state_event);
                        let cj_diff = (cj_event - initial_cj).abs();
                        if cj_diff > max_cj_diff {
                            max_cj_diff = cj_diff;
                        }

                        trajectory.push(TrajectoryPoint {
                            t: t_event,
                            state: state_event,
                            jacobi_constant: cj_event,
                        });

                        let closure = (state_event - *initial_state).r_norm();
                        return Ok(PropagationResult {
                            trajectory,
                            final_state: state_event,
                            final_time: t_event,
                            final_stm: None,
                            max_jacobi_variation: max_cj_diff,
                            closure_norm: closure,
                            steps_accepted,
                            steps_rejected,
                            stop_reason: StopReason::EventTriggered(ev.clone()),
                        });
                    }
                    prev_g = Some(g_c);
                }

                t = t_next;
                y = y_next;

                let cj = jacobi_constant(self.system, &next_state);
                let diff = (cj - initial_cj).abs();
                if diff > max_cj_diff {
                    max_cj_diff = diff;
                }

                trajectory.push(TrajectoryPoint {
                    t,
                    state: next_state,
                    jacobi_constant: cj,
                });

                h = next_h_mag * direction;
            } else {
                steps_rejected += 1;
                h = next_h_mag * direction;
            }

            if step_idx == self.options.max_steps - 1 {
                stop_reason = StopReason::StepLimitExceeded;
            }
        }

        let final_state = Cr3bpState::from_array(y);
        let closure = (final_state - *initial_state).r_norm();

        Ok(PropagationResult {
            trajectory,
            final_state,
            final_time: t,
            final_stm: None,
            max_jacobi_variation: max_cj_diff,
            closure_norm: closure,
            steps_accepted,
            steps_rejected,
            stop_reason,
        })
    }

    /// Propagates 42D state and State Transition Matrix $\mathbf{\Phi}(t, t_0)$.
    pub fn propagate_with_stm(
        &self,
        initial_state: &Cr3bpState,
        t_start: f64,
        t_end: f64,
        event: Option<&EventCondition>,
    ) -> Result<PropagationResult, Cr3bpError> {
        let mut y42 = [0.0; 42];
        y42[..6].copy_from_slice(&initial_state.to_array());

        // Initialize STM as identity matrix
        for i in 0..6 {
            y42[6 + 6 * i + i] = 1.0;
        }

        let mut t = t_start;
        let initial_cj = jacobi_constant(self.system, initial_state);

        let mut trajectory = Vec::new();
        trajectory.push(TrajectoryPoint {
            t,
            state: *initial_state,
            jacobi_constant: initial_cj,
        });

        let mut max_cj_diff = 0.0;
        let mut steps_accepted = 0;
        let mut steps_rejected = 0;
        let direction = if t_end >= t_start { 1.0 } else { -1.0 };
        let mut h = self.options.initial_step * direction;

        let mut prev_g = event.map(|ev| ev.evaluate(initial_state));
        let mut stop_reason = StopReason::TargetTimeReached;

        for step_idx in 0..self.options.max_steps {
            if (direction > 0.0 && t >= t_end) || (direction < 0.0 && t <= t_end) {
                stop_reason = StopReason::TargetTimeReached;
                break;
            }

            if (direction > 0.0 && t + h > t_end) || (direction < 0.0 && t + h < t_end) {
                h = t_end - t;
            }

            let (y_next, t_next, next_h_mag, accepted) = self.step_42d(t, &y42, h);

            if accepted {
                steps_accepted += 1;
                let next_state = Cr3bpState::new(
                    y_next[0], y_next[1], y_next[2], y_next[3], y_next[4], y_next[5],
                );

                if let (Some(ev), Some(g_p)) = (event, prev_g) {
                    let g_c = ev.evaluate(&next_state);
                    if ev.is_triggered(g_p, g_c) {
                        // Refine event for 42D state
                        let (t_ev, y_ev) = self.bisect_event_42d(t, &y42, t_next, &y_next, ev);
                        let ev_state = Cr3bpState::new(
                            y_ev[0], y_ev[1], y_ev[2], y_ev[3], y_ev[4], y_ev[5],
                        );
                        let cj_ev = jacobi_constant(self.system, &ev_state);
                        let diff = (cj_ev - initial_cj).abs();
                        if diff > max_cj_diff {
                            max_cj_diff = diff;
                        }

                        let mut final_stm = [[0.0; 6]; 6];
                        for r in 0..6 {
                            for c in 0..6 {
                                final_stm[r][c] = y_ev[6 + 6 * r + c];
                            }
                        }

                        trajectory.push(TrajectoryPoint {
                            t: t_ev,
                            state: ev_state,
                            jacobi_constant: cj_ev,
                        });

                        let closure = (ev_state - *initial_state).r_norm();
                        return Ok(PropagationResult {
                            trajectory,
                            final_state: ev_state,
                            final_time: t_ev,
                            final_stm: Some(final_stm),
                            max_jacobi_variation: max_cj_diff,
                            closure_norm: closure,
                            steps_accepted,
                            steps_rejected,
                            stop_reason: StopReason::EventTriggered(ev.clone()),
                        });
                    }
                    prev_g = Some(g_c);
                }

                t = t_next;
                y42 = y_next;

                let cj = jacobi_constant(self.system, &next_state);
                let diff = (cj - initial_cj).abs();
                if diff > max_cj_diff {
                    max_cj_diff = diff;
                }

                trajectory.push(TrajectoryPoint {
                    t,
                    state: next_state,
                    jacobi_constant: cj,
                });

                h = next_h_mag * direction;
            } else {
                steps_rejected += 1;
                h = next_h_mag * direction;
            }

            if step_idx == self.options.max_steps - 1 {
                stop_reason = StopReason::StepLimitExceeded;
            }
        }

        let final_state = Cr3bpState::new(y42[0], y42[1], y42[2], y42[3], y42[4], y42[5]);
        let mut final_stm = [[0.0; 6]; 6];
        for r in 0..6 {
            for c in 0..6 {
                final_stm[r][c] = y42[6 + 6 * r + c];
            }
        }
        let closure = (final_state - *initial_state).r_norm();

        Ok(PropagationResult {
            trajectory,
            final_state,
            final_time: t,
            final_stm: Some(final_stm),
            max_jacobi_variation: max_cj_diff,
            closure_norm: closure,
            steps_accepted,
            steps_rejected,
            stop_reason,
        })
    }

    /// High-precision bisection solver to locate event crossing to $< 10^{-12}$ tolerance.
    fn bisect_event_6d(
        &self,
        mut t_left: f64,
        state_left: &Cr3bpState,
        mut t_right: f64,
        _state_right: &Cr3bpState,
        event: &EventCondition,
    ) -> (f64, Cr3bpState) {
        let mut y_l = state_left.to_array();

        for _ in 0..60 {
            let dt = t_right - t_left;
            if dt.abs() < 1e-13 {
                break;
            }
            let t_mid = 0.5 * (t_left + t_right);
            let h_mid = t_mid - t_left;

            let (y_m, _, _, _) = self.step_6d(t_left, &y_l, h_mid);
            let state_m = Cr3bpState::from_array(y_m);
            let g_m = event.evaluate(&state_m);

            if g_m.abs() < 1e-13 {
                return (t_mid, state_m);
            }

            let g_l = event.evaluate(&Cr3bpState::from_array(y_l));
            if (g_l > 0.0 && g_m < 0.0) || (g_l < 0.0 && g_m > 0.0) {
                t_right = t_mid;
            } else {
                t_left = t_mid;
                y_l = y_m;
            }
        }

        let t_final = 0.5 * (t_left + t_right);
        (t_final, Cr3bpState::from_array(y_l))
    }

    /// Bisection solver for 42D state and STM.
    fn bisect_event_42d(
        &self,
        mut t_left: f64,
        y_left: &[f64; 42],
        mut t_right: f64,
        _y_right: &[f64; 42],
        event: &EventCondition,
    ) -> (f64, [f64; 42]) {
        let mut y_l = *y_left;

        for _ in 0..60 {
            let dt = t_right - t_left;
            if dt.abs() < 1e-13 {
                break;
            }
            let t_mid = 0.5 * (t_left + t_right);
            let h_mid = t_mid - t_left;

            let (y_m, _, _, _) = self.step_42d(t_left, &y_l, h_mid);
            let state_m = Cr3bpState::new(y_m[0], y_m[1], y_m[2], y_m[3], y_m[4], y_m[5]);
            let g_m = event.evaluate(&state_m);

            if g_m.abs() < 1e-13 {
                return (t_mid, y_m);
            }

            let state_l = Cr3bpState::new(y_l[0], y_l[1], y_l[2], y_l[3], y_l[4], y_l[5]);
            let g_l = event.evaluate(&state_l);
            if (g_l > 0.0 && g_m < 0.0) || (g_l < 0.0 && g_m > 0.0) {
                t_right = t_mid;
            } else {
                t_left = t_mid;
                y_l = y_m;
            }
        }

        let t_final = 0.5 * (t_left + t_right);
        (t_final, y_l)
    }
}
