//! CLI application for the NASA EVOLVE 4.0 Standard Breakup Model.

#![deny(clippy::print_stdout, clippy::print_stderr)]

use sbm_core::prelude::*;
use std::fs::File;
use std::io::Write;
use tracing::info;

fn main() -> Result<(), BreakupError> {
    // Initialize tracing subscriber for structured, leveled logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    info!("============================================================");
    info!(" NASA Standard Breakup Model - Simple Engine CLI");
    info!("============================================================");

    let target_m = 1000.0;    // kg
    let proj_m = 100.0;       // kg
    let impact_v = 10_000.0;  // 10 km/s (10,000 m/s)

    let engine = BreakupEngine::builder()
        .target_mass(target_m)
        .projectile_mass(proj_m)
        .impact_speed(impact_v)
        .target_type(ObjectType::Spacecraft)
        .projectile_type(ObjectType::Spacecraft)
        .breakup_type(BreakupType::Collision)
        .num_fragments(1500)
        .build()?;

    let result = engine.simulate(42);

    info!(target_mass_kg = target_m, target_type = ?engine.target_type, "Configured target");
    info!(projectile_mass_kg = proj_m, projectile_type = ?engine.projectile_type, "Configured projectile");
    info!(impact_velocity_km_s = impact_v / 1000.0, specific_energy_kj_per_kg = result.specific_energy_kj_per_kg, "Collision parameters");
    info!(
        outcome = if result.is_catastrophic { "CATASTROPHIC (Total breakup)" } else { "PARTIAL (Crater/Remnant)" },
        destroyed_mass_kg = result.destroyed_mass_kg,
        remnant_mass_kg = result.remnant_mass_kg,
        "Breakup regime evaluated"
    );
    info!(
        yield_1cm = result.physical_yield_1cm,
        yield_10cm_ssn = result.ssn_trackable_yield_10cm,
        sampled_fragments = result.fragments.len(),
        "Debris cloud population generated"
    );

    // Log top 3 heaviest fragments
    info!("--- TOP 3 HEAVIEST FRAGMENTS (Core chunks) ---");
    for f in result.top_heaviest(3) {
        info!(
            id = f.id,
            size_m = f.size_m,
            area_m2 = f.cross_section_m2,
            am_ratio = f.am_ratio,
            mass_kg = f.mass_kg,
            speed_mps = f.speed_mps,
            band = f.contour_band,
            "Heavy fragment"
        );
    }

    // Log top 3 fastest fragments
    info!("--- TOP 3 FASTEST FRAGMENTS (Light shards) ---");
    for f in result.top_fastest(3) {
        info!(
            id = f.id,
            size_cm = f.size_m * 100.0,
            area_m2 = f.cross_section_m2,
            am_ratio = f.am_ratio,
            mass_kg = f.mass_kg,
            speed_mps = f.speed_mps,
            band = f.contour_band,
            "Fast fragment"
        );
    }

    // Export to JSON for inspection or feeding to visualizers
    let mut json_file = File::create("fragments_output.json").expect("Failed to create file");
    json_file
        .write_all(result.to_fragments_json().as_bytes())
        .expect("Failed to write JSON output");

    info!(
        output_file = "fragments_output.json",
        fragments_count = result.fragments.len(),
        "Successfully exported fragments"
    );
    info!("============================================================");

    Ok(())
}
