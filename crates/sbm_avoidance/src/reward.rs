//! Fuel-efficient multi-objective reward shaping and termination engine.
//!
//! Grounded in Algorithm 3 and Section IV-B2 of Luna et al. (2026):
//! - **Survival Reward**: Positive feedback for sustaining orbit
//! - **Distance Shaping**: Proximity buffer safety incentive and coast bonus
//! - **Closing Speed & Projected Miss**: Rewards geometry widening at lookahead
//! - **Context-Aware $\Delta v$ Penalty**: Allows aggressive thrusting only in imminent danger
//! - **Smoothness Penalty**: Dampens control jitter
//! - **Cumulative $\Delta v$ Soft Cap**: Accelerating penalty above $60\text{ m/s}$

use crate::types::{RewardBreakdown, RewardCoefficients};

/// Soft threshold for cumulative velocity expenditure in m/s ($60.0\text{ m/s}$).
pub const CUMULATIVE_DELTA_V_SOFT_CAP: f64 = 60.0;

/// Exponent applied to cumulative delta-v overages ($1.3$).
pub const CUMULATIVE_DELTA_V_EXPONENT: f64 = 1.3;

/// Contextual kinematics and history inputs for Algorithm 3 reward evaluation.
#[derive(Debug, Clone, Copy)]
pub struct RewardContext {
    pub min_distance_m: f64,
    pub step_delta_v: f64,
    pub action_change_norm: f64,
    pub cumulative_delta_v: f64,
    pub step: usize,
    pub closing_speed_mps: Option<f64>,
    pub prev_closing_speed_mps: Option<f64>,
    pub projected_miss_m: Option<f64>,
    pub collision_threshold_m: f64,
    pub safe_zone_buffer_m: f64,
}

/// Evaluates step reward components and terminal collision status according to Algorithm 3.
pub fn calculate_step_reward(
    ctx: &RewardContext,
    coeffs: &RewardCoefficients,
) -> (RewardBreakdown, bool) {
    let mut breakdown = RewardBreakdown::default();
    let extended_safe_threshold = ctx.collision_threshold_m + ctx.safe_zone_buffer_m;

    // 1. Collision terminal check
    if ctx.min_distance_m <= ctx.collision_threshold_m {
        breakdown.collision_penalty = -coeffs.collision_penalty;
        breakdown.total = breakdown.collision_penalty;
        return (breakdown, true);
    }

    // 2. Survival baseline tick
    breakdown.survival = coeffs.survival_reward;

    // 3. Distance shaping & coasting bonus
    if ctx.min_distance_m > extended_safe_threshold {
        let dist_excess = ctx.min_distance_m - extended_safe_threshold;
        let capped_excess = dist_excess.min(8000.0);
        let dist_inc = coeffs.distance_shaping_coeff * capped_excess;
        breakdown.distance += dist_inc;

        if ctx.step_delta_v < 0.005 {
            breakdown.distance += coeffs.coast_bonus;
        }
    } else {
        let gap = extended_safe_threshold - ctx.min_distance_m;
        let capped_gap = gap.min(8000.0);
        let dist_dec = 0.00015 * capped_gap;
        breakdown.distance -= dist_dec;
    }

    // 4. Closing speed improvement & projected miss distance
    if let (Some(curr_closing), Some(prev_closing)) = (ctx.closing_speed_mps, ctx.prev_closing_speed_mps) {
        if prev_closing > 0.0 && curr_closing < prev_closing {
            let improvement = (prev_closing - curr_closing) / prev_closing.max(1e-6);
            let closing_inc = 0.5 * improvement; // bounded in [0, 0.5]
            breakdown.closing += closing_inc;
        }
    }

    if let Some(proj_miss) = ctx.projected_miss_m {
        if proj_miss > ctx.collision_threshold_m {
            let gain = proj_miss - ctx.collision_threshold_m;
            let proj_inc = coeffs.projected_miss_reward * gain.min(15000.0);
            breakdown.distance += proj_inc;
        }
    }

    // 5. Context-aware Delta-V expenditure cost
    if ctx.step_delta_v > 0.0 {
        let mut dv_cost = coeffs.delta_v_linear_cost * ctx.step_delta_v
            + coeffs.delta_v_quadratic_cost * (ctx.step_delta_v * ctx.step_delta_v);

        // Relax cost when maneuvering within safety buffer
        if ctx.min_distance_m <= extended_safe_threshold {
            dv_cost *= 0.6;
        }
        breakdown.dv_penalty -= dv_cost;

        // Quadratic penalty on large impulses (> 0.2 m/s)
        if ctx.step_delta_v > 0.2 {
            let over = ctx.step_delta_v - 0.2;
            let extra = coeffs.large_burn_penalty * (over * over);
            breakdown.dv_penalty -= extra;
        }

        // Proximity burn incentive when dangerously close
        if ctx.min_distance_m < 1.15 * ctx.collision_threshold_m && ctx.step_delta_v > 0.05 {
            let incentive = 0.3 * (ctx.step_delta_v / 0.15).min(1.0);
            breakdown.closing += incentive;
        }
    }

    // 6. Action smoothness penalty (penalize jitter)
    if ctx.action_change_norm > 0.0 && ctx.step_delta_v < 0.25 {
        let dec = coeffs.smoothness_penalty * (ctx.action_change_norm * ctx.action_change_norm);
        breakdown.smooth_penalty -= dec;
    }

    // 7. Soft cap on cumulative delta-v
    if ctx.cumulative_delta_v > CUMULATIVE_DELTA_V_SOFT_CAP {
        let over = ctx.cumulative_delta_v - CUMULATIVE_DELTA_V_SOFT_CAP;
        let dec = coeffs.cumulative_dv_penalty * over.powf(CUMULATIVE_DELTA_V_EXPONENT);
        breakdown.cumulative_penalty -= dec;
    }

    // 8. Long episode milestone survival bonus (every 100 steps)
    if ctx.step > 0 && ctx.step.is_multiple_of(100) {
        breakdown.milestone += coeffs.milestone_reward;
    }

    breakdown.total = breakdown.survival
        + breakdown.distance
        + breakdown.closing
        + breakdown.milestone
        + breakdown.dv_penalty
        + breakdown.smooth_penalty
        + breakdown.cumulative_penalty
        + breakdown.collision_penalty;

    (breakdown, false)
}
