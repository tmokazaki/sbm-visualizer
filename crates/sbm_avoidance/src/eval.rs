//! Deterministic benchmark evaluation harness reproducing Tables 2, 3, and 4.
//!
//! Grounded in Section V and Section IV-C of Luna et al. (2026):
//! - Runs 1,000 deterministic seeded episodes (base seed 12345)
//! - Evaluates PPO policy, Impulsive $\Delta v$, Risk-Aware Rule-Based, and No-Action baselines
//! - Calculates statistical averages for Success Rate, Collision Rate, $\Delta v$, Fuel, and Reward

use crate::baselines::{AvoidanceController, ImpulsivePlannerController, NoActionController, RiskAwareRuleBasedController};
use crate::env::SatelliteAvoidanceEnv;
use crate::nn::ActorCritic;
use crate::types::{Action3D, AvoidanceConfig, RewardBreakdown, TerminationReason};
use serde::{Deserialize, Serialize};

/// Summary metrics container for a policy evaluated across multiple episodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluationSummary {
    pub policy_name: String,
    pub total_episodes: usize,
    pub collisions: usize,
    pub successes: usize,
    pub fuel_depletions: usize,
    pub timeouts: usize,
    pub collision_rate_pct: f64,
    pub success_rate_pct: f64,
    pub avg_reward: f64,
    pub avg_steps: f64,
    pub avg_delta_v: f64,
    pub avg_min_distance_m: f64,
    pub avg_fuel_used_kg: f64,
    pub mean_reward_breakdown: RewardBreakdown,
}

/// Evaluates a controller over deterministic seeded trials.
pub fn evaluate_controller<C: AvoidanceController>(
    controller: &mut C,
    name: &str,
    config: &AvoidanceConfig,
    episodes: usize,
    base_seed: u64,
    max_steps: usize,
) -> PolicyEvaluationSummary {
    let mut collisions = 0;
    let mut successes = 0;
    let mut fuel_depletions = 0;
    let mut timeouts = 0;

    let mut total_rewards = 0.0;
    let mut total_steps = 0;
    let mut total_delta_v = 0.0;
    let mut total_min_distance = 0.0;
    let mut total_fuel_used = 0.0;
    let mut total_breakdown = RewardBreakdown::default();

    let mut env = SatelliteAvoidanceEnv::new(config.clone());

    for ep_idx in 0..episodes {
        let ep_seed = base_seed.wrapping_add(ep_idx as u64);
        controller.reset();
        env.reset(ep_seed);

        let initial_fuel = env.satellite.fuel_mass_kg;
        let mut steps = 0;
        let mut ep_reward = 0.0;
        let mut ep_dv = 0.0;
        let mut term_reason = None;

        while steps < max_steps {
            let ctx = crate::baselines::ControllerContext {
                satellite: &env.satellite,
                debris_field: &env.debris_field,
                target_debris_idx: env.target_debris_idx,
                collision_threshold_m: env.config.collision_distance_m,
                safe_buffer_m: env.config.safe_zone_buffer_m,
                max_thrust: env.config.max_thrust,
                dt_seconds: env.config.dt_seconds,
            };
            let action = controller.compute_action(&ctx);

            let (_obs, reward, done, info) = env.step(action);
            steps += 1;
            ep_reward += reward;
            ep_dv += info.step_delta_v;

            if done {
                term_reason = info.termination_reason;
                break;
            }
        }

        if term_reason.is_none() && steps >= max_steps {
            term_reason = Some(TerminationReason::Success);
        }

        match term_reason {
            Some(TerminationReason::Collision) => collisions += 1,
            Some(TerminationReason::FuelDepleted) => fuel_depletions += 1,
            Some(TerminationReason::MaxSteps) => timeouts += 1,
            Some(TerminationReason::Success) | Some(TerminationReason::OrbitComplete) | None => successes += 1,
        }

        total_rewards += ep_reward;
        total_steps += steps;
        total_delta_v += ep_dv;
        total_min_distance += env.min_observed_distance_m;
        total_fuel_used += (initial_fuel - env.satellite.fuel_mass_kg).max(0.0);
        total_breakdown.add(&env.cumulative_reward_breakdown);
    }

    let n = episodes.max(1) as f64;
    let mean_breakdown = RewardBreakdown {
        survival: total_breakdown.survival / n,
        distance: total_breakdown.distance / n,
        closing: total_breakdown.closing / n,
        milestone: total_breakdown.milestone / n,
        dv_penalty: total_breakdown.dv_penalty / n,
        smooth_penalty: total_breakdown.smooth_penalty / n,
        cumulative_penalty: total_breakdown.cumulative_penalty / n,
        collision_penalty: total_breakdown.collision_penalty / n,
        total: total_breakdown.total / n,
    };

    PolicyEvaluationSummary {
        policy_name: name.to_string(),
        total_episodes: episodes,
        collisions,
        successes,
        fuel_depletions,
        timeouts,
        collision_rate_pct: (collisions as f64 / n) * 100.0,
        success_rate_pct: (successes as f64 / n) * 100.0,
        avg_reward: total_rewards / n,
        avg_steps: total_steps as f64 / n,
        avg_delta_v: total_delta_v / n,
        avg_min_distance_m: total_min_distance / n,
        avg_fuel_used_kg: total_fuel_used / n,
        mean_reward_breakdown: mean_breakdown,
    }
}

/// Evaluates the PPO policy network (actor) over deterministic seeded trials.
pub fn evaluate_ppo_model(
    model: &ActorCritic,
    config: &AvoidanceConfig,
    episodes: usize,
    base_seed: u64,
    max_steps: usize,
) -> PolicyEvaluationSummary {
    let mut collisions = 0;
    let mut successes = 0;
    let mut fuel_depletions = 0;
    let mut timeouts = 0;

    let mut total_rewards = 0.0;
    let mut total_steps = 0;
    let mut total_delta_v = 0.0;
    let mut total_min_distance = 0.0;
    let mut total_fuel_used = 0.0;
    let mut total_breakdown = RewardBreakdown::default();

    let mut env = SatelliteAvoidanceEnv::new(config.clone());

    for ep_idx in 0..episodes {
        let ep_seed = base_seed.wrapping_add(ep_idx as u64);
        let mut obs = env.reset(ep_seed);

        let initial_fuel = env.satellite.fuel_mass_kg;
        let mut steps = 0;
        let mut ep_reward = 0.0;
        let mut ep_dv = 0.0;
        let mut term_reason = None;

        while steps < max_steps {
            let act_arr = model.forward_actor_deterministic(&obs);
            let action = Action3D::new(act_arr[0] as f64, act_arr[1] as f64, act_arr[2] as f64);

            let (next_obs, reward, done, info) = env.step(action);
            steps += 1;
            ep_reward += reward;
            ep_dv += info.step_delta_v;
            obs = next_obs;

            if done {
                term_reason = info.termination_reason;
                break;
            }
        }

        if term_reason.is_none() && steps >= max_steps {
            term_reason = Some(TerminationReason::Success);
        }

        match term_reason {
            Some(TerminationReason::Collision) => collisions += 1,
            Some(TerminationReason::FuelDepleted) => fuel_depletions += 1,
            Some(TerminationReason::MaxSteps) => timeouts += 1,
            Some(TerminationReason::Success) | Some(TerminationReason::OrbitComplete) | None => successes += 1,
        }

        total_rewards += ep_reward;
        total_steps += steps;
        total_delta_v += ep_dv;
        total_min_distance += env.min_observed_distance_m;
        total_fuel_used += (initial_fuel - env.satellite.fuel_mass_kg).max(0.0);
        total_breakdown.add(&env.cumulative_reward_breakdown);
    }

    let n = episodes.max(1) as f64;
    let mean_breakdown = RewardBreakdown {
        survival: total_breakdown.survival / n,
        distance: total_breakdown.distance / n,
        closing: total_breakdown.closing / n,
        milestone: total_breakdown.milestone / n,
        dv_penalty: total_breakdown.dv_penalty / n,
        smooth_penalty: total_breakdown.smooth_penalty / n,
        cumulative_penalty: total_breakdown.cumulative_penalty / n,
        collision_penalty: total_breakdown.collision_penalty / n,
        total: total_breakdown.total / n,
    };

    PolicyEvaluationSummary {
        policy_name: "PPO".to_string(),
        total_episodes: episodes,
        collisions,
        successes,
        fuel_depletions,
        timeouts,
        collision_rate_pct: (collisions as f64 / n) * 100.0,
        success_rate_pct: (successes as f64 / n) * 100.0,
        avg_reward: total_rewards / n,
        avg_steps: total_steps as f64 / n,
        avg_delta_v: total_delta_v / n,
        avg_min_distance_m: total_min_distance / n,
        avg_fuel_used_kg: total_fuel_used / n,
        mean_reward_breakdown: mean_breakdown,
    }
}

/// Executes full comparative benchmark reproducing Tables 2, 3, and 4 across all 4 policies.
pub fn run_full_paper_benchmark(
    model: &ActorCritic,
    config: &AvoidanceConfig,
    episodes: usize,
    base_seed: u64,
) -> Vec<PolicyEvaluationSummary> {
    let mut summaries = Vec::new();

    // 1. PPO Policy
    let ppo_summary = evaluate_ppo_model(model, config, episodes, base_seed, 1000);
    summaries.push(ppo_summary);

    // 2. Impulsive Delta-V Planner
    let mut imp = ImpulsivePlannerController::default();
    let imp_summary = evaluate_controller(&mut imp, "Impulsive", config, episodes, base_seed, 1000);
    summaries.push(imp_summary);

    // 3. Risk-Aware Rule-Based Controller
    let mut rule = RiskAwareRuleBasedController::default();
    let rule_summary = evaluate_controller(&mut rule, "Rule-based", config, episodes, base_seed, 1000);
    summaries.push(rule_summary);

    // 4. No-Action Baseline
    let mut noact = NoActionController;
    let noact_summary = evaluate_controller(&mut noact, "No-action", config, episodes, base_seed, 1000);
    summaries.push(noact_summary);

    summaries
}
