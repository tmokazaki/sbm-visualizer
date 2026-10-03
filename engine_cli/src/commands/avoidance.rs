//! Command-line interface for PPO satellite avoidance training and benchmark reproduction.
//!
//! Grounded in Luna et al. (2026):
//! - `avoidance-train`: Full RL training loop with 3-stage curriculum and PPO Adam updates
//! - `avoidance-eval`: Deterministic 1,000-trial benchmark reproducing Tables 2, 3, and 4

use sbm_avoidance::prelude::*;
use std::error::Error;
use std::time::Instant;
use tracing::info;

/// Executes the PPO training loop subcommand.
pub fn run_avoidance_train_cli(args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut total_timesteps = 1_000_000usize;
    let mut seed = 42u64;
    let mut save_path = "models/ppo_avoidance_model.bin".to_string();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--timesteps" if i + 1 < args.len() => {
                total_timesteps = args[i + 1].parse()?;
                i += 1;
            }
            "--seed" if i + 1 < args.len() => {
                seed = args[i + 1].parse()?;
                i += 1;
            }
            "--save" if i + 1 < args.len() => {
                save_path = args[i + 1].clone();
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    info!("============================================================");
    info!(" SBM Avoidance: Proximal Policy Optimization (PPO) Training");
    info!(" Paper: Luna et al. (2026), IEEE Access (DOI: 10.1109/ACCESS.2026.3655237)");
    info!("============================================================");
    info!("  Total Timesteps:  {}", total_timesteps);
    info!("  Random Seed:      {}", seed);
    info!("  Target Save Path: {}", save_path);
    info!("============================================================");

    let start_time = Instant::now();
    let config = AvoidanceConfig {
        reward: RewardCoefficients::training(),
        ..Default::default()
    };

    let mut env = SatelliteAvoidanceEnv::new(config);
    let params = PpoHyperparameters::default();
    let mut trainer = PpoTrainer::new(seed, params);
    let mut rng = SimpleRng::new(seed);

    info!("Starting PPO curriculum training loop...");
    let mut last_log_step = 0;

    while trainer.global_step < total_timesteps {
        let mean_ep_reward = trainer.train_iteration(&mut env, &mut rng);
        let stage = trainer.curriculum.get_stage(trainer.global_step);

        if trainer.global_step - last_log_step >= 20_000 || trainer.global_step >= total_timesteps {
            last_log_step = trainer.global_step;
            let elapsed_sec = start_time.elapsed().as_secs_f64();
            let steps_per_sec = trainer.global_step as f64 / elapsed_sec.max(1e-3);

            info!(
                "Step: {:7}/{} ({:5.1}%) | Stage: {:?} | Collision Prob: {:.2} | Mean Ep Reward: {:10.2} | Speed: {:6.0} steps/s",
                trainer.global_step,
                total_timesteps,
                (trainer.global_step as f64 / total_timesteps as f64) * 100.0,
                stage.name,
                stage.collision_probability,
                mean_ep_reward,
                steps_per_sec,
            );
        }
    }

    if let Some(parent) = std::path::Path::new(&save_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    save_model_checkpoint(&trainer.model, &save_path)
        .map_err(|e| format!("Failed to save checkpoint to '{}': {}", save_path, e))?;

    let total_elapsed = start_time.elapsed().as_secs_f64();
    info!("============================================================");
    info!(" Training completed in {:.2} seconds!", total_elapsed);
    info!(" Model saved successfully to: {}", save_path);
    info!("============================================================");

    Ok(())
}

/// Executes the deterministic 1,000-trial benchmark evaluation subcommand.
pub fn run_avoidance_eval_cli(args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut episodes = 1000usize;
    let mut base_seed = 12345u64;
    let mut model_path: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--episodes" if i + 1 < args.len() => {
                episodes = args[i + 1].parse()?;
                i += 1;
            }
            "--seed" if i + 1 < args.len() => {
                base_seed = args[i + 1].parse()?;
                i += 1;
            }
            "--model" if i + 1 < args.len() => {
                model_path = Some(args[i + 1].clone());
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    info!("============================================================");
    info!(" SBM Avoidance: Deterministic Benchmark Reproduction");
    info!(" Paper: Luna et al. (2026), IEEE Access (DOI: 10.1109/ACCESS.2026.3655237)");
    info!("============================================================");
    info!("  Evaluation Episodes: {}", episodes);
    info!("  Base Random Seed:    {}", base_seed);
    if let Some(ref path) = model_path {
        info!("  Model Source:        File ({})", path);
    } else {
        info!("  Model Source:        Embedded Pre-Trained Weights (Luna et al. 2026)");
    }
    info!("============================================================");

    let model = if let Some(ref path) = model_path {
        load_model_checkpoint(path)?
    } else {
        load_default_pretrained_model()?
    };

    let config = AvoidanceConfig {
        reward: RewardCoefficients::evaluation(),
        ..Default::default()
    };

    let start_time = Instant::now();
    info!("Running deterministic benchmark across 4 controllers...");

    let summaries = run_full_paper_benchmark(&model, &config, episodes, base_seed);
    let elapsed = start_time.elapsed().as_secs_f64();

    info!("=========================================================================================================");
    info!(" TABLE 2: Agent Performance ({}-run evaluation)", episodes);
    info!("=========================================================================================================");
    info!(
        "{:<12} | {:>14} | {:>10} | {:>12} | {:>9} | {:>12} | {:>14} | {:>13}",
        "Policy", "Collision Rate", "Collisions", "Success Rate", "Avg Steps", "Avg Δv (m/s)", "Avg Min Dist(m)", "Fuel Used(kg)"
    );
    info!("---------------------------------------------------------------------------------------------------------");

    for s in &summaries {
        info!(
            "{:<12} | {:>13.2}% | {:>10} | {:>11.2}% | {:>9.2} | {:>12.3} | {:>14.2} | {:>13.3}",
            s.policy_name,
            s.collision_rate_pct,
            s.collisions,
            s.success_rate_pct,
            s.avg_steps,
            s.avg_delta_v,
            s.avg_min_distance_m,
            s.avg_fuel_used_kg
        );
    }
    info!("=========================================================================================================");

    info!("");
    info!("==================================================================================");
    info!(" TABLE 3: Termination counts ({}-run evaluation)", episodes);
    info!("==================================================================================");
    info!("{:<12} | {:>10} | {:>10} | {:>15} | {:>10}", "Policy", "Success", "Collision", "Fuel Depleted", "Timeout");
    info!("----------------------------------------------------------------------------------");
    for s in &summaries {
        info!(
            "{:<12} | {:>10} | {:>10} | {:>15} | {:>10}",
            s.policy_name, s.successes, s.collisions, s.fuel_depletions, s.timeouts
        );
    }
    info!("==================================================================================");

    info!("");
    info!("=================================================================================================================");
    info!(" TABLE 4: Mean reward components per episode ({}-run evaluation)", episodes);
    info!("=================================================================================================================");
    info!(
        "{:<12} | {:>10} | {:>12} | {:>8} | {:>9} | {:>12} | {:>15} | {:>12}",
        "Policy", "Survival", "Distance", "Closing", "Milestone", "Δv Penalty", "Cumulative Pen.", "Total Reward"
    );
    info!("-----------------------------------------------------------------------------------------------------------------");
    for s in &summaries {
        let b = &s.mean_reward_breakdown;
        info!(
            "{:<12} | {:>10.2} | {:>12.2} | {:>8.2} | {:>9.2} | {:>12.2} | {:>15.2} | {:>12.2}",
            s.policy_name, b.survival, b.distance, b.closing, b.milestone, b.dv_penalty, b.cumulative_penalty, b.total
        );
    }
    info!("=================================================================================================================");
    info!("Benchmark completed in {:.2} seconds ({:.1} episodes/sec).", elapsed, episodes as f64 * 4.0 / elapsed.max(1e-3));

    Ok(())
}
