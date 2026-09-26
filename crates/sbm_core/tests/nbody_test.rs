use sbm_core::nbody::{
    compute_accelerations, compute_conservation_metrics, compute_laplace_resonance_metrics,
    compute_trojan_libration_deg, create_preset, extract_osculating_elements,
    propagate_trajectory, step_hermite4, step_leapfrog, step_system, CelestialBody,
    IntegratorType, PresetId, ASTRONOMICAL_UNIT_M, G_STANDARD, JULIAN_DAY_S,
};

#[test]
fn test_yoshida4_energy_conservation_figure8() {
    let mut system = create_preset(PresetId::FigureEight);
    system.integrator = IntegratorType::Yoshida4th;

    // Characteristic velocity and length scale in preset
    let m = 1.0e24;
    let r_scale = 1.0e8;
    let v_scale = (G_STANDARD * m / r_scale).sqrt();
    let period_s = 6.3259 * (r_scale / v_scale);

    let initial_metrics = compute_conservation_metrics(&system);
    let e0 = initial_metrics.total_energy_j;
    assert!(e0.abs() > 1e15, "Figure-8 must have non-zero energy");

    // Integrate for 2 full periods with dt = period / 500
    let dt = period_s / 500.0;
    let steps = 1000;
    for _ in 0..steps {
        step_system(&mut system, dt);
    }

    let final_metrics = compute_conservation_metrics(&system);
    let rel_err = ((final_metrics.total_energy_j - e0) / e0).abs();

    assert!(
        rel_err < 1e-6,
        "Yoshida 4th must conserve energy in Figure-8: rel_err = {:e}",
        rel_err
    );
}

#[test]
fn test_yoshida6_precision_higher_than_yoshida4() {
    let mut sys4 = create_preset(PresetId::FigureEight);
    sys4.integrator = IntegratorType::Yoshida4th;

    let mut sys6 = create_preset(PresetId::FigureEight);
    sys6.integrator = IntegratorType::Yoshida6th;

    let m = 1.0e24;
    let r_scale = 1.0e8;
    let v_scale = (G_STANDARD * m / r_scale).sqrt();
    let period_s = 6.3259 * (r_scale / v_scale);

    let dt = period_s / 100.0;
    for _ in 0..200 {
        step_system(&mut sys4, dt);
        step_system(&mut sys6, dt);
    }

    let m4 = compute_conservation_metrics(&sys4);
    let m6 = compute_conservation_metrics(&sys6);

    // Both should conserve energy tightly
    assert!(m4.relative_energy_error < 1e-4);
    assert!(m6.relative_energy_error < 1e-4);
}

#[test]
fn test_linear_and_angular_momentum_conservation() {
    let mut system = create_preset(PresetId::SolarSystemJpl);
    system.integrator = IntegratorType::Yoshida4th;

    let m0 = compute_conservation_metrics(&system);
    let initial_p_mag = m0.linear_momentum_magnitude;
    let initial_l_mag = m0.angular_momentum_magnitude;

    // Advance 30 days in steps of 0.5 days
    let dt = 0.5 * JULIAN_DAY_S;
    for _ in 0..60 {
        step_system(&mut system, dt);
    }

    let m_final = compute_conservation_metrics(&system);

    // Linear momentum magnitude should remain tiny or invariant
    let p_drift = (m_final.linear_momentum_magnitude - initial_p_mag).abs();
    assert!(
        p_drift < 1e20,
        "Linear momentum drift must be negligible: {:e}",
        p_drift
    );

    // Angular momentum relative drift should be < 1e-5
    let l_rel_drift = ((m_final.angular_momentum_magnitude - initial_l_mag) / initial_l_mag).abs();
    assert!(
        l_rel_drift < 1e-5,
        "Angular momentum relative error must be < 1e-5: {:e}",
        l_rel_drift
    );
}

#[test]
fn test_jpl_solar_system_earth_orbital_period_and_radius() {
    let system = create_preset(PresetId::SolarSystemJpl);
    let sun = &system.bodies[0];
    let earth = &system.bodies[3];

    let elements = extract_osculating_elements(earth, sun, G_STANDARD)
        .expect("Earth osculating elements must be computable");

    // Earth semi-major axis should be ~1.0 AU within 3%
    let a_au = elements.semi_major_axis_m / ASTRONOMICAL_UNIT_M;
    assert!(
        (a_au - 1.0).abs() < 0.03,
        "Earth semi-major axis should be ~1.0 AU, got {}",
        a_au
    );

    // Orbital period should be ~365.25 days within 3%
    let p_days = elements.period_s / JULIAN_DAY_S;
    assert!(
        (p_days - 365.25).abs() < 12.0,
        "Earth period should be ~365.25 days, got {}",
        p_days
    );
}

#[test]
fn test_laplace_resonance_4_2_1() {
    let system = create_preset(PresetId::LaplaceResonance);
    let jupiter = &system.bodies[0];
    let io = &system.bodies[1];
    let europa = &system.bodies[2];
    let ganymede = &system.bodies[3];

    let el_io = extract_osculating_elements(io, jupiter, G_STANDARD).unwrap();
    let el_eu = extract_osculating_elements(europa, jupiter, G_STANDARD).unwrap();
    let el_ga = extract_osculating_elements(ganymede, jupiter, G_STANDARD).unwrap();

    let r_eu_io = el_eu.period_s / el_io.period_s;
    let r_ga_eu = el_ga.period_s / el_eu.period_s;

    // Europa period / Io period ~ 2.0 (within 2%)
    assert!(
        (r_eu_io - 2.0).abs() < 0.05,
        "Europa/Io period ratio should be ~2.0, got {}",
        r_eu_io
    );

    // Ganymede period / Europa period ~ 2.0 (within 2%)
    assert!(
        (r_ga_eu - 2.0).abs() < 0.05,
        "Ganymede/Europa period ratio should be ~2.0, got {}",
        r_ga_eu
    );

    // Laplace angle phi_L = lambda_1 - 3*lambda_2 + 2*lambda_3 librates around 180 deg
    let res_metrics = compute_laplace_resonance_metrics(&system.bodies)
        .expect("Laplace metrics must be available");
    assert!(
        (res_metrics.resonant_angle_deg - 180.0).abs() < 20.0,
        "Laplace angle must be near 180 deg, got {}",
        res_metrics.resonant_angle_deg
    );
}

#[test]
fn test_sun_jupiter_trojan_lagrange_angles() {
    let system = create_preset(PresetId::SunJupiterTrojans);
    let sun = &system.bodies[0];
    let jupiter = &system.bodies[1];
    let achilles = &system.bodies[2];
    let patroclus = &system.bodies[4];

    let angle_l4 = compute_trojan_libration_deg(sun, jupiter, achilles);
    let angle_l5 = compute_trojan_libration_deg(sun, jupiter, patroclus);

    // L4 should be +60 deg
    assert!(
        (angle_l4 - 60.0).abs() < 5.0,
        "Achilles should be near +60 deg (L4), got {}",
        angle_l4
    );

    // L5 should be -60 deg
    assert!(
        (angle_l5 - (-60.0)).abs() < 5.0,
        "Patroclus should be near -60 deg (L5), got {}",
        angle_l5
    );
}

#[test]
fn test_relativistic_mercury_precession_computation() {
    let mut system_gr = create_preset(PresetId::RelativisticMercury);
    system_gr.enable_general_relativity = true;

    let mut system_newton = create_preset(PresetId::RelativisticMercury);
    system_newton.enable_general_relativity = false;

    let a_gr = compute_accelerations(
        &system_gr.bodies,
        system_gr.gravitational_constant,
        0.0,
        true,
    );
    let a_newton = compute_accelerations(
        &system_newton.bodies,
        system_newton.gravitational_constant,
        0.0,
        false,
    );

    // Acceleration on Mercury in GR must have a positive radial/orbital correction compared to pure Newton
    let diff_x = a_gr[1][0] - a_newton[1][0];
    let diff_y = a_gr[1][1] - a_newton[1][1];
    let diff_mag = (diff_x * diff_x + diff_y * diff_y).sqrt();

    assert!(
        diff_mag > 1e-12,
        "1PN General Relativity must produce non-zero acceleration correction: {:e}",
        diff_mag
    );
}

#[test]
fn test_trajectory_propagation_output_snapshots() {
    let mut system = create_preset(PresetId::FigureEight);
    let snapshots = propagate_trajectory(&mut system, 100.0, 10.0, 1.0);

    assert_eq!(snapshots.len(), 11, "Should record 11 snapshots (t=0 to t=100 every 10s)");
    assert_eq!(snapshots[0].time_s, 0.0);
    assert!((snapshots[10].time_s - 100.0).abs() < 1e-6);
}

#[test]
fn test_osculating_elements_circular_and_inclined() {
    let primary = CelestialBody::new(
        0,
        "Primary",
        1.98847e30,
        696340.0,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "#fff",
    );

    let r = 1.0 * ASTRONOMICAL_UNIT_M;
    let v = (G_STANDARD * primary.mass_kg / r).sqrt();

    // Orbit inclined at 45 deg
    let inc = 45.0_f64.to_radians();
    let body = CelestialBody::new(
        1,
        "TestSat",
        1000.0,
        1.0,
        [r, 0.0, 0.0],
        [0.0, v * inc.cos(), v * inc.sin()],
        "#38bdf8",
    );

    let elem = extract_osculating_elements(&body, &primary, G_STANDARD)
        .expect("Elements must be extractable");

    assert!((elem.semi_major_axis_m - r).abs() / r < 1e-4);
    assert!(elem.eccentricity < 1e-4);
    assert!((elem.inclination_rad - inc).abs() < 1e-4);
}

#[test]
fn test_leapfrog_and_hermite4_steps() {
    let mut sys_lf = create_preset(PresetId::SolarSystemJpl);
    sys_lf.integrator = IntegratorType::Leapfrog;
    step_leapfrog(&mut sys_lf, 3600.0);
    assert!(sys_lf.bodies[3].speed() > 0.0);

    let mut sys_herm = create_preset(PresetId::SolarSystemJpl);
    sys_herm.integrator = IntegratorType::Hermite4th;
    step_hermite4(&mut sys_herm, 3600.0);
    assert!(sys_herm.bodies[3].speed() > 0.0);
}
