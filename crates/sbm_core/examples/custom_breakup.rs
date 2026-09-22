//! Example showing how to use the sbm_core library in an external Rust project.

#![deny(clippy::print_stdout, clippy::print_stderr)]

use sbm_core::prelude::*;
use tracing::info;

fn main() -> Result<(), BreakupError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    info!("=== Custom Breakup Scenario via sbm_simple_engine Library ===");

    // Example: Hypervelocity collision between a 1500 kg remote sensing satellite
    // and a 2.5 kg orbital debris fragment at 12 km/s relative speed
    let engine = BreakupEngine::builder()
        .target_mass(1500.0)
        .projectile_mass(2.5)
        .impact_speed(12_000.0) // 12 km/s
        .target_type(ObjectType::Spacecraft)
        .projectile_type(ObjectType::Spacecraft)
        .breakup_type(BreakupType::Collision)
        .num_fragments(500)
        .build()?;

    let result = engine.simulate(2026);

    info!(specific_energy_kj_per_kg = result.specific_energy_kj_per_kg, "Specific impact energy");
    info!(
        outcome = if result.is_catastrophic { "Catastrophic Disruption" } else { "Cratering / Non-catastrophic" },
        destroyed_mass_kg = result.destroyed_mass_kg,
        remnant_mass_kg = result.remnant_mass_kg,
        "Collision outcome"
    );
    info!(
        total_fragments = result.fragments.len(),
        theoretical_yield_1cm = result.physical_yield_1cm,
        theoretical_yield_10cm = result.ssn_trackable_yield_10cm,
        "Fragment yield"
    );

    info!("--- Top 3 Heaviest Shards ---");
    for f in result.top_heaviest(3) {
        info!(
            id = f.id,
            mass_kg = f.mass_kg,
            lc_m = f.size_m,
            speed_mps = f.speed_mps,
            ballistic_coeff = f.ballistic_coefficient(None),
            "Heavy shard"
        );
    }

    Ok(())
}
