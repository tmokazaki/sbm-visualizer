//! Analytical baseline controllers for collision avoidance benchmarking.
//!
//! Grounded in Section IV-C1 of Luna et al. (2026):
//! - **No-Action**: Passive unguided orbital propagation ($\mathbf{a}_t = \mathbf{0}$)
//! - **Risk-Aware Rule-Based Controller**: Multi-factor decision rule blending spatial distance and TTC (Eqs. 9–10)
//! - **Impulsive $\Delta v$ Planner**: Finite-duration lateral acceleration planning targeting safety clearance (Eqs. 11–12)

use crate::conjunction::{assess_conjunction, find_most_critical_debris};
use crate::types::{Action3D, DebrisObject, SatelliteState, Vector3D};

/// Operational context provided to controllers at each decision epoch.
#[derive(Debug, Clone)]
pub struct ControllerContext<'a> {
    pub satellite: &'a SatelliteState,
    pub debris_field: &'a [DebrisObject],
    pub target_debris_idx: Option<usize>,
    pub collision_threshold_m: f64,
    pub safe_buffer_m: f64,
    pub max_thrust: f64,
    pub dt_seconds: f64,
}

/// Common trait implemented by all avoidance controllers.
pub trait AvoidanceController: Send + Sync {
    /// Resets any internal planner state at episode start.
    fn reset(&mut self);

    /// Computes commanded 3D continuous action $\mathbf{a} \in [-1, 1]^3$ for the current state.
    fn compute_action(&mut self, ctx: &ControllerContext<'_>) -> Action3D;
}

/// Baseline 1: Passive unguided coasting baseline (Zero commanded thrust).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoActionController;

impl AvoidanceController for NoActionController {
    fn reset(&mut self) {}

    fn compute_action(&mut self, _ctx: &ControllerContext<'_>) -> Action3D {
        Action3D::zero()
    }
}

/// Baseline 2: Risk-Aware Rule-Based Controller.
///
/// Implements Equations (9) and (10) of Luna et al. (2026):
/// $$\rho_i = \sigma\left(\alpha_d \frac{r_{\text{eff}}}{d_i} + \alpha_\tau \frac{\tau_{\text{crit}} - \tau_i}{\tau_{\text{crit}}}\right)$$
/// $$\mathbf{T}_t = T_{\max} \rho_{i^*} \left[\hat{\mathbf{u}}_{i^*} + \beta \hat{\mathbf{v}}_{\text{rel}, i^*}\right]$$
/// where $\beta = 0.35$, $\hat{\mathbf{u}}_{i^*}$ points directly away from the most threatening debris,
/// and $\hat{\mathbf{v}}_{\text{rel}}$ damps closing velocity.
#[derive(Debug, Clone, Copy)]
pub struct RiskAwareRuleBasedController {
    pub beta_damping: f64,
}

impl Default for RiskAwareRuleBasedController {
    fn default() -> Self {
        Self {
            beta_damping: 0.35,
        }
    }
}

impl AvoidanceController for RiskAwareRuleBasedController {
    fn reset(&mut self) {}

    fn compute_action(&mut self, ctx: &ControllerContext<'_>) -> Action3D {
        if ctx.debris_field.is_empty() {
            return Action3D::zero();
        }

        // Use target debris if designated, else find debris with max risk score rho
        let (crit_idx, metrics) = if let Some(idx) = ctx.target_debris_idx {
            if idx < ctx.debris_field.len() {
                let m = assess_conjunction(
                    ctx.satellite.position,
                    ctx.satellite.velocity,
                    ctx.debris_field[idx].position,
                    ctx.debris_field[idx].velocity,
                    ctx.debris_field[idx].radius_m,
                    ctx.collision_threshold_m,
                );
                (idx, m)
            } else {
                find_most_critical_debris(ctx.satellite, ctx.debris_field, ctx.collision_threshold_m)
                    .unwrap_or((0, assess_conjunction(
                        ctx.satellite.position, ctx.satellite.velocity,
                        ctx.debris_field[0].position, ctx.debris_field[0].velocity,
                        ctx.debris_field[0].radius_m, ctx.collision_threshold_m,
                    )))
            }
        } else {
            find_most_critical_debris(ctx.satellite, ctx.debris_field, ctx.collision_threshold_m)
                .unwrap_or((0, assess_conjunction(
                    ctx.satellite.position, ctx.satellite.velocity,
                    ctx.debris_field[0].position, ctx.debris_field[0].velocity,
                    ctx.debris_field[0].radius_m, ctx.collision_threshold_m,
                )))
        };

        if metrics.hazard_score < 0.05 {
            return Action3D::zero();
        }

        let deb = &ctx.debris_field[crit_idx];
        let rel_pos = deb.position - ctx.satellite.position;
        let rel_vel = deb.velocity - ctx.satellite.velocity;

        // Away unit vector (pointing from debris to satellite)
        let away_dir = (-rel_pos).normalize_or_zero();

        // Relative velocity direction (opposing approach)
        let vel_dir = (-rel_vel).normalize_or_zero();

        // Blended thrust direction: away + beta * vel_dir
        let blended_dir = away_dir + vel_dir * self.beta_damping;
        let blended_unit = blended_dir.normalize_or_zero();

        // Scale commanded action by hazard risk score rho
        let cmd = blended_unit * metrics.hazard_score;

        Action3D::new(cmd.x, cmd.y, cmd.z)
    }
}

/// Baseline 3: Impulsive $\Delta v$ Planner.
///
/// Implements Equations (11) and (12) of Luna et al. (2026):
/// Predicts $t^* = -\frac{\mathbf{r}_{\text{rel}}\cdot\mathbf{v}_{\text{rel}}}{\|\mathbf{v}_{\text{rel}}\|^2}$,
/// estimates required lateral clearance $d_{\text{req}} = \max(0, d_{\text{safe}} - d_{pm})$,
/// and commands constant lateral acceleration:
/// $$a_\perp = \frac{2 d_{\text{req}}}{(t^*)^2}$$
/// perpendicular to $\mathbf{v}_{\text{rel}}$, sustained over $\lceil t^* / \Delta t \rceil$ steps.
#[derive(Debug, Clone)]
pub struct ImpulsivePlannerController {
    plan_action: Action3D,
    steps_remaining: usize,
}

impl Default for ImpulsivePlannerController {
    fn default() -> Self {
        Self {
            plan_action: Action3D::zero(),
            steps_remaining: 0,
        }
    }
}

impl AvoidanceController for ImpulsivePlannerController {
    fn reset(&mut self) {
        self.plan_action = Action3D::zero();
        self.steps_remaining = 0;
    }

    fn compute_action(&mut self, ctx: &ControllerContext<'_>) -> Action3D {
        // Sustain planned maneuver if ongoing
        if self.steps_remaining > 0 {
            self.steps_remaining -= 1;
            return self.plan_action;
        }

        if ctx.debris_field.is_empty() {
            return Action3D::zero();
        }

        // Select target debris
        let tgt_idx = ctx.target_debris_idx.unwrap_or(0).min(ctx.debris_field.len() - 1);
        let deb = &ctx.debris_field[tgt_idx];

        let r0 = ctx.satellite.position - deb.position;
        let v_rel = ctx.satellite.velocity - deb.velocity;
        let v_sq = v_rel.norm_squared();

        if v_sq < 1e-12 {
            let away = r0.normalize_or_zero();
            self.plan_action = Action3D::new(away.x * 0.5, away.y * 0.5, away.z * 0.5);
            self.steps_remaining = (5.0 / ctx.dt_seconds.max(1e-6)).ceil() as usize;
            return self.plan_action;
        }

        // 1. Predict TCA t*
        let t_star = (-r0.dot(&v_rel) / v_sq).clamp(0.0, 1200.0);
        if t_star <= 0.0 {
            return Action3D::zero();
        }

        // 2. Perpendicular displacement vector and current miss distance
        let v_hat = v_rel.normalize_or_zero();
        let r_perp = r0 - v_hat * r0.dot(&v_hat);
        let miss_now = r_perp.norm();

        // 3. Required clearance
        let clearance_needed = ctx.collision_threshold_m + ctx.safe_buffer_m;
        let d_req = (clearance_needed - miss_now).max(0.0);
        if d_req <= 0.0 {
            return Action3D::zero();
        }

        // 4. Constant lateral acceleration: a_req = 2 * d_req / t*^2
        let a_req = 2.0 * d_req / (t_star * t_star).max(1e-6);
        let mag = (a_req / ctx.max_thrust.max(1e-6)).clamp(0.0, 1.0);

        // 5. Normal direction perpendicular to v_rel aligned with r_perp
        let normal_dir = if r_perp.norm() > 1e-9 {
            r_perp.normalize_or_zero()
        } else {
            let trial = if v_hat.x.abs() < 0.9 {
                Vector3D::new(1.0, 0.0, 0.0)
            } else {
                Vector3D::new(0.0, 1.0, 0.0)
            };
            v_hat.cross(&trial).normalize_or_zero()
        };

        let action_vec = normal_dir * mag;
        let action = Action3D::new(action_vec.x, action_vec.y, action_vec.z);

        let steps_needed = (t_star / ctx.dt_seconds.max(1e-6)).ceil() as usize;
        self.plan_action = action;
        self.steps_remaining = steps_needed.max(1);

        action
    }
}
