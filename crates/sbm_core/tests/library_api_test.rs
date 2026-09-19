//! Integration tests verifying library consumption from an external crate perspective.

use sbm_core::prelude::*;

#[test]
fn test_library_builder_and_simulation_collision() {
    let engine = BreakupEngine::builder()
        .target_mass(2000.0)
        .projectile_mass(50.0)
        .impact_speed(14_000.0) // 14 km/s
        .target_type(ObjectType::Spacecraft)
        .projectile_type(ObjectType::RocketBody)
        .breakup_type(BreakupType::Collision)
        .num_fragments(300)
        .build()
        .expect("Engine build failed");

    let result = engine.simulate(42);

    // Ep = 0.5 * 50 * 14000^2 / 2000 = 2450 kJ/kg >= 40 kJ/kg
    assert!(result.is_catastrophic);
    assert_eq!(result.destroyed_mass_kg, 2050.0);
    assert_eq!(result.remnant_mass_kg, 0.0);
    assert_eq!(result.fragments.len(), 300);

    // Verify mass conservation
    let total_mass = result.total_fragment_mass();
    assert!((total_mass - 2050.0).abs() < 1e-6);

    // Verify fragment methods
    for f in &result.fragments {
        assert!(f.size_m > 0.0);
        assert!(f.cross_section_m2 > 0.0);
        assert!(f.am_ratio > 0.0);
        assert!(f.mass_kg > 0.0);
        assert!(f.speed_mps > 0.0);

        let bc = f.ballistic_coefficient(None);
        assert!((bc - 0.5 * 2.2 * f.am_ratio).abs() < 1e-9);

        let ke = f.kinetic_energy_joules();
        assert!((ke - 0.5 * f.mass_kg * f.speed_mps.powi(2)).abs() < 1e-6);
    }
}

#[test]
fn test_library_cratering_remnant() {
    let engine = BreakupEngine::builder()
        .target_mass(5000.0)
        .projectile_mass(0.5) // 500 g pellet
        .impact_speed(7000.0) // 7 km/s -> Ep = 0.5 * 0.5 * 49e6 / 5000 = 2.45 kJ/kg < 40 kJ/kg
        .num_fragments(150)
        .build()
        .expect("Build failed");

    let result = engine.simulate(99);
    assert!(!result.is_catastrophic);
    // Destroyed mass = 0.5 kg * 7 km/s = 3.5 kg
    assert_eq!(result.destroyed_mass_kg, 3.5);
    assert_eq!(result.remnant_mass_kg, 4996.5);
    assert_eq!(result.fragments.len(), 150);

    let total_frag_mass = result.total_fragment_mass();
    assert!((total_frag_mass - 3.5).abs() < 1e-6);
}

#[test]
fn test_library_explosion_simulation() {
    let engine = BreakupEngine::builder()
        .target_mass(800.0)
        .projectile_mass(1.0)
        .breakup_type(BreakupType::Explosion)
        .explosion_scaling(1.2)
        .target_type(ObjectType::RocketBody)
        .num_fragments(200)
        .build()
        .expect("Build failed");

    let result = engine.simulate(777);
    assert!(result.is_catastrophic);
    assert_eq!(result.fragments.len(), 200);

    // Yields computed via Eq. (3)
    assert!(result.physical_yield_1cm > 0.0);
    assert!(result.ssn_trackable_yield_10cm > 0.0);
}

#[test]
fn test_library_builder_error_handling() {
    // Negative target mass
    let err1 = BreakupEngine::builder().target_mass(-100.0).build();
    assert!(matches!(err1, Err(BreakupError::InvalidMass { parameter: "target_mass", .. })));

    // Zero fragments
    let err2 = BreakupEngine::builder().num_fragments(0).build();
    assert!(matches!(err2, Err(BreakupError::InvalidFragmentCount { value: 0 })));

    // Invalid power-law exponent
    let err3 = BreakupEngine::builder().size_skew(0.5).build();
    assert!(matches!(err3, Err(BreakupError::InvalidPowerLawExponent { .. })));

    // Invalid size range
    let err4 = BreakupEngine::builder().min_size(5.0).max_size(2.0).build();
    assert!(matches!(err4, Err(BreakupError::InvalidSizeRange { .. })));
}

#[test]
fn test_custom_rng_support() {
    // Custom RNG implementing RngSource
    struct DummyRng {
        counter: u64,
    }

    impl RngSource for DummyRng {
        fn next_f64(&mut self) -> f64 {
            self.counter = (self.counter.wrapping_mul(6364136223846793005)).wrapping_add(1);
            ((self.counter >> 11) as f64) / ((1u64 << 53) as f64)
        }
    }

    let engine = BreakupEngine::new(1000.0, 100.0, 10_000.0).unwrap();
    let mut dummy = DummyRng { counter: 42 };
    let result = engine.simulate_with_rng(&mut dummy);
    assert_eq!(result.fragments.len(), 1500);
}

#[test]
fn test_json_export_structure() {
    let engine = BreakupEngine::builder()
        .target_mass(500.0)
        .projectile_mass(50.0)
        .impact_speed(8000.0)
        .num_fragments(10)
        .build()
        .unwrap();

    let result = engine.simulate(1);
    let json_str = result.to_fragments_json();
    assert!(json_str.starts_with("[\n"));
    assert!(json_str.ends_with("]\n"));
    assert!(json_str.contains("\"id\":0"));
    assert!(json_str.contains("\"size\":"));
    assert!(json_str.contains("\"area\":"));
    assert!(json_str.contains("\"am_ratio\":"));
    assert!(json_str.contains("\"chi\":"));
    assert!(json_str.contains("\"mass\":"));
    assert!(json_str.contains("\"speed\":"));
}
