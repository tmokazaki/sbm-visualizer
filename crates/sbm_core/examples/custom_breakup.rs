//! Example showing how to use the sbm_core library in an external Rust project.

use sbm_core::prelude::*;

fn main() -> Result<(), BreakupError> {
    println!("=== Custom Breakup Scenario via sbm_simple_engine Library ===");

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

    println!("Specific Impact Energy: {:.2} kJ/kg", result.specific_energy_kj_per_kg);
    println!("Outcome: {}", if result.is_catastrophic { "Catastrophic Disruption" } else { "Cratering / Non-catastrophic" });
    println!("Destroyed Mass:         {:.2} kg", result.destroyed_mass_kg);
    println!("Surviving Remnant Mass: {:.2} kg", result.remnant_mass_kg);
    println!("Total Fragment Count:   {}", result.fragments.len());
    println!("Theoretical Yield (>=1cm):  {:.0}", result.physical_yield_1cm);
    println!("Theoretical Yield (>=10cm): {:.0}", result.ssn_trackable_yield_10cm);

    println!("\nTop 3 Heaviest Shards:");
    for f in result.top_heaviest(3) {
        println!("  #{} -> Mass: {:.3} kg, Lc: {:.2} m, Speed: {:.1} m/s, B*: {:.4} m^2/kg",
            f.id, f.mass_kg, f.size_m, f.speed_mps, f.ballistic_coefficient(None));
    }

    Ok(())
}
