//! Unified CLI application for SBM Astrodynamics:
//! - NASA EVOLVE 4.0 Breakup Simulation (`breakup`)
//! - Clohessy-Wiltshire RPO Target Maneuver Planning (`rpo`)
//! - CR3BP Low-Energy & Multi-Body Transfer Planning (`transfer`)
//! - PPO Satellite Collision Avoidance Training & Benchmarking (`avoidance-train`, `avoidance-eval`)

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub mod commands;

use commands::{
    run_avoidance_eval_cli, run_avoidance_train_cli, run_breakup_cli, run_rpo_cli,
    run_transfer_cli,
};
use std::env;
use tracing::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args: Vec<String> = env::args().collect();
    let subcommand = args.get(1).map(|s| s.as_str()).unwrap_or("breakup");

    match subcommand {
        "rpo" => run_rpo_cli(&args[2..])?,
        "transfer" => run_transfer_cli(&args[2..])?,
        "avoidance-train" => run_avoidance_train_cli(&args[2..])?,
        "avoidance-eval" => run_avoidance_eval_cli(&args[2..])?,
        "help" | "--help" | "-h" => print_usage(),
        "breakup" => run_breakup_cli(&args)?,
        _ => run_breakup_cli(&args)?,
    }

    Ok(())
}

fn print_usage() {
    info!("============================================================");
    info!(" SBM Astrodynamics & Mission Planning CLI");
    info!("============================================================");
    info!("Usage: sbm_simple_engine <COMMAND> [OPTIONS]");
    info!("");
    info!("COMMANDS:");
    info!("  breakup          Execute NASA EVOLVE 4.0 debris collision simulation (default)");
    info!("  rpo              Plan Clohessy-Wiltshire RPO target maneuvers");
    info!("  transfer         Compute CR3BP invariant manifold low-energy transfer");
    info!("  avoidance-train  Train autonomous PPO satellite avoidance agent (Luna et al. 2026)");
    info!("  avoidance-eval   Run deterministic benchmark reproduction (Tables 2, 3, 4)");
    info!("  help             Show this usage guide");
    info!("");
    info!("AVOIDANCE OPTIONS:");
    info!("  --episodes <N>   Evaluation episodes (default: 1000)");
    info!("  --seed <seed>    Base random seed (default: 12345)");
    info!("  --timesteps <N>  Training timesteps (default: 1000000)");
    info!("  --save <path>    Model output binary path (default: models/ppo_avoidance_model.bin)");
    info!("  --model <path>   Path to model binary (default: embedded pre-trained weights)");
    info!("============================================================");
}
