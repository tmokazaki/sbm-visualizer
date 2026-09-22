//! Unified CLI application for SBM Astrodynamics:
//! - NASA EVOLVE 4.0 Breakup Simulation (`breakup`)
//! - Clohessy-Wiltshire RPO Target Maneuver Planning (`rpo`)
//! - CR3BP Low-Energy & Multi-Body Transfer Planning (`transfer`)

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub mod commands;

use commands::{run_breakup_cli, run_rpo_cli, run_transfer_cli};
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
    info!("  breakup   Execute NASA EVOLVE 4.0 debris collision simulation (default)");
    info!("  rpo       Plan Clohessy-Wiltshire RPO target maneuvers");
    info!("  transfer  Compute CR3BP invariant manifold low-energy transfer");
    info!("  help      Show this usage guide");
    info!("");
    info!("RPO OPTIONS:");
    info!("  --mode <nmc|two-impulse|vbar|rbar>  RPO maneuver profile (default: two-impulse)");
    info!("  --target <iss|sso|geo>             Reference target orbit (default: iss)");
    info!("  --x <meters>                       Initial radial offset (default: -100.0)");
    info!("  --y <meters>                       Initial in-track offset (default: -500.0)");
    info!("  --z <meters>                       Initial cross-track offset (default: 0.0)");
    info!("  --duration <seconds>               Transfer duration (default: 1800.0)");
    info!("  --radial <meters>                  NMC radial semi-axis (default: 100.0)");
    info!("============================================================");
}
