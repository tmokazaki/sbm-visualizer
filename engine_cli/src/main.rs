//! CLI application for the NASA EVOLVE 4.0 Standard Breakup Model.

use std::fs::File;
use std::io::Write;
use sbm_core::prelude::*;

fn main() -> Result<(), BreakupError> {
    println!("============================================================");
    println!(" NASA Standard Breakup Model - Simple Engine CLI");
    println!("============================================================");

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

    println!("Target Mass:       {:.0} kg ({:?})", target_m, engine.target_type);
    println!("Projectile Mass:   {:.0} kg ({:?})", proj_m, engine.projectile_type);
    println!("Impact Velocity:   {:.1} km/s", impact_v / 1000.0);
    println!("Specific Energy:   {:.1} kJ/kg", result.specific_energy_kj_per_kg);
    println!(
        "Collision Outcome: {}",
        if result.is_catastrophic {
            "CATASTROPHIC (Total breakup)"
        } else {
            "PARTIAL (Crater/Remnant)"
        }
    );
    println!("Destroyed Mass:    {:.1} kg", result.destroyed_mass_kg);
    if !result.is_catastrophic {
        println!("Surviving Remnant: {:.1} kg", result.remnant_mass_kg);
    }
    println!(
        "Physical Yield:    {:.0} frags (>=1 cm), {:.0} frags (>=10 cm SSN)",
        result.physical_yield_1cm, result.ssn_trackable_yield_10cm
    );
    println!("Sampled Fragments: {} (Simulated points)", result.fragments.len());
    println!("------------------------------------------------------------");

    // Print top 3 heaviest fragments
    println!("TOP 3 HEAVIEST FRAGMENTS (Core chunks, stay near center):");
    for f in result.top_heaviest(3) {
        println!(
            "  #{} -> Size: {:.2} m, Area: {:.4} m^2, A/M: {:.4} m^2/kg, Mass: {:.1} kg, Speed: {:.0} m/s, Band: {}",
            f.id, f.size_m, f.cross_section_m2, f.am_ratio, f.mass_kg, f.speed_mps, f.contour_band
        );
    }

    // Print top 3 fastest fragments
    println!("\nTOP 3 FASTEST FRAGMENTS (Light shards, outer expanding bubble):");
    for f in result.top_fastest(3) {
        println!(
            "  #{} -> Size: {:.1} cm, Area: {:.6} m^2, A/M: {:.4} m^2/kg, Mass: {:.3} kg, Speed: {:.0} m/s, Band: {}",
            f.id, f.size_m * 100.0, f.cross_section_m2, f.am_ratio, f.mass_kg, f.speed_mps, f.contour_band
        );
    }

    // Export to JSON for inspection or feeding to visualizers
    let mut json_file = File::create("fragments_output.json").expect("Failed to create file");
    json_file
        .write_all(result.to_fragments_json().as_bytes())
        .expect("Failed to write JSON output");

    println!("\n[OK] Exported {} fragments to fragments_output.json", result.fragments.len());
    println!("============================================================");

    Ok(())
}
