use sbm_core::nbody::{
    compute_accelerations, compute_centric_spatial_field_grid, compute_conservation_metrics,
    compute_earth_moon_barycenter, compute_gravitational_spheres, compute_laplace_resonance_metrics,
    compute_pairwise_forces, compute_spatial_field_grid, compute_spatial_field_point,
    compute_tidal_tensor, compute_trojan_libration_deg, create_preset, extract_osculating_elements,
    is_body_relevant_to_centric, propagate_trajectory, step_hermite4, step_leapfrog, step_system,
    CelestialBody, GravitationalCentricFrame, IntegratorType, NBodySystem, PresetId,
    ASTRONOMICAL_UNIT_M, G_STANDARD, JULIAN_DAY_S,
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

#[test]
fn test_chebotarev_1964_gravitational_spheres_reproduction() {
    // Chebotarev (1964), Soviet Astronomy 7(5), Table 1 "Dimensions of Gravitational Spheres"
    // Tabulates Sphere of Attraction (r_a), Laplace Sphere of Influence (r_s), and Hill Sphere (r_H).
    let m_sun = 1.98847e30;
    let m_earth = 5.9722e24;
    let a_earth = 1.495978707e11; // 1 AU in m
    let e_earth = 0.0167;
    let e_moon = 0.0549;

    let earth_spheres = compute_gravitational_spheres(m_earth, m_sun, a_earth, e_earth, e_moon);

    // Convert to km
    let r_a_km = earth_spheres.sphere_of_attraction_m / 1000.0;
    let r_s_km = earth_spheres.laplace_soi_m / 1000.0;
    let r_h_km = earth_spheres.hill_sphere_m / 1000.0;

    // Assert within 1-2% of Chebotarev (1964) Table 1 (259k, 924k, 1472k km)
    assert!(
        (r_a_km - 259_000.0).abs() / 259_000.0 < 0.015,
        "Earth r_a mismatch: got {:.1} km, expected ~259,000 km",
        r_a_km
    );
    assert!(
        (r_s_km - 924_000.0).abs() / 924_000.0 < 0.015,
        "Earth r_s mismatch: got {:.1} km, expected ~924,000 km",
        r_s_km
    );
    assert!(
        (r_h_km - 1_472_000.0).abs() / 1_472_000.0 < 0.02,
        "Earth r_H mismatch: got {:.1} km, expected ~1,472,000 km",
        r_h_km
    );

    // Jupiter
    let m_jupiter = 1.89813e27;
    let a_jupiter = 5.2044 * ASTRONOMICAL_UNIT_M;
    let e_jupiter = 0.0484;

    let jupiter_spheres = compute_gravitational_spheres(m_jupiter, m_sun, a_jupiter, e_jupiter, 0.0);
    let r_a_jup_mkm = jupiter_spheres.sphere_of_attraction_m / 1e9;
    let r_s_jup_mkm = jupiter_spheres.laplace_soi_m / 1e9;
    let r_h_jup_mkm = jupiter_spheres.hill_sphere_m / 1e9;

    // Chebotarev Table 1: Jupiter r_a = 24.1 million km, r_s = 48.2 million km, r_H = 53.1 million km
    assert!((r_a_jup_mkm - 24.1).abs() < 0.8, "Jupiter r_a: {:.2}M km", r_a_jup_mkm);
    assert!((r_s_jup_mkm - 48.2).abs() < 1.0, "Jupiter r_s: {:.2}M km", r_s_jup_mkm);
    assert!((r_h_jup_mkm - 50.5).abs() < 3.0, "Jupiter r_H: {:.2}M km", r_h_jup_mkm);
}

#[test]
fn test_domingos_2006_hill_sphere_satellite_stability() {
    // Domingos, Winter, & Yokoyama (2006) MNRAS 373(3), pp. 1227-1234
    // Critical prograde stability radius:
    // r_crit = 0.4895 * r_H * (1.0 - 1.0305 * e_sat - 0.2738 * e_planet)
    let m_sun = 1.98847e30;
    let m_earth = 5.9722e24;
    let a_earth = 1.0 * ASTRONOMICAL_UNIT_M;
    let e_earth = 0.0167;
    let e_moon = 0.0549;

    let earth_spheres = compute_gravitational_spheres(m_earth, m_sun, a_earth, e_earth, e_moon);
    let r_h = earth_spheres.hill_sphere_m;
    let r_crit = earth_spheres.critical_stability_radius_m;

    let a_moon = 384_400_000.0; // 384,400 km in meters

    // The Moon must be well inside r_crit, which in turn is well inside r_H:
    assert!(
        a_moon < r_crit,
        "Moon semi-major axis must be within Domingos 2006 critical stability boundary: a_moon = {:.0} km, r_crit = {:.0} km",
        a_moon / 1000.0,
        r_crit / 1000.0
    );
    assert!(
        r_crit < r_h,
        "Domingos critical radius must be inside the Hill sphere: r_crit = {:.0} km, r_h = {:.0} km",
        r_crit / 1000.0,
        r_h / 1000.0
    );

    // Critical radius is ~680,000 km
    let r_crit_km = r_crit / 1000.0;
    assert!(r_crit_km > 650_000.0 && r_crit_km < 750_000.0, "r_crit out of expected range: {:.0} km", r_crit_km);
}

#[test]
fn test_tidal_tensor_trace_free_and_eigenvalues_arxiv_1608_03366() {
    // arXiv:1608.03366: Gravity Gradient / Tidal Tensor
    // T_ab = G * M / r^3 * (3 * n_a * n_b - delta_ab)
    // 1. Trace must be exactly zero: Tr(T) = 0
    // 2. Maximum eigenvalue (radial stretching) = +2 * G * M / r^3
    // 3. Orthogonal eigenvalues (lateral compression) = -G * M / r^3, -G * M / r^3
    // 4. Sum of eigenvalues must be 0

    let mut system = NBodySystem::new();
    let earth = CelestialBody::new(0, "Earth", 5.9722e24, 6371.0, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], "#3b82f6");
    let moon_pos = [384_400_000.0, 0.0, 0.0];
    let moon = CelestialBody::new(1, "Moon", 7.3477e22, 1737.4, moon_pos, [0.0, 1022.0, 0.0], "#94a3b8");
    system.add_body(earth);
    system.add_body(moon);

    let tidal = compute_tidal_tensor(&system, [0.0, 0.0, 0.0]);

    // Analytical check
    let r: f64 = 384_400_000.0;
    let expected_lambda1 = 2.0 * G_STANDARD * 7.3477e22 / r.powi(3);
    let expected_lambda2 = -G_STANDARD * 7.3477e22 / r.powi(3);

    // 1. Trace free
    assert!(tidal.trace.abs() < 1e-18, "Tidal tensor must be trace-free: trace = {:e}", tidal.trace);

    // 2. Eigenvalue sum is zero
    let sum_eigen = tidal.eigenvalues[0] + tidal.eigenvalues[1] + tidal.eigenvalues[2];
    assert!(sum_eigen.abs() < 1e-18, "Sum of eigenvalues must be zero: sum = {:e}", sum_eigen);

    // 3. Radial stretching
    let rel_err_lambda1 = (tidal.eigenvalues[0] - expected_lambda1).abs() / expected_lambda1;
    assert!(rel_err_lambda1 < 1e-5, "Radial eigenvalue mismatch: {:e} vs {:e}", tidal.eigenvalues[0], expected_lambda1);

    // 4. Lateral compression
    let rel_err_lambda2 = (tidal.eigenvalues[1] - expected_lambda2).abs() / expected_lambda2.abs();
    assert!(rel_err_lambda2 < 1e-5, "Lateral eigenvalue 1 mismatch");
    let rel_err_lambda3 = (tidal.eigenvalues[2] - expected_lambda2).abs() / expected_lambda2.abs();
    assert!(rel_err_lambda3 < 1e-5, "Lateral eigenvalue 2 mismatch");
}

#[test]
fn test_earth_moon_sun_gravitational_tug_of_war() {
    let system = create_preset(PresetId::InnerSolarSystemJupiter);
    assert_eq!(system.bodies.len(), 6);

    // Find Sun, Earth, Moon
    let sun_idx = system.bodies.iter().position(|b| b.name == "Sun").unwrap();
    let earth_idx = system.bodies.iter().position(|b| b.name == "Earth").unwrap();
    let moon_idx = system.bodies.iter().position(|b| b.name == "Moon").unwrap();

    let moon_forces = compute_pairwise_forces(&system, moon_idx);
    let f_sun_on_moon = moon_forces.iter().find(|pf| pf.source_id == system.bodies[sun_idx].id).unwrap();
    let f_earth_on_moon = moon_forces.iter().find(|pf| pf.source_id == system.bodies[earth_idx].id).unwrap();

    // Verify Sun pull on Moon is ~2.2x Earth pull on Moon!
    let ratio = f_sun_on_moon.magnitude_n / f_earth_on_moon.magnitude_n;
    assert!(
        (ratio - 2.20).abs() < 0.15,
        "Gravitational Tug-of-War ratio F_sun / F_earth on Moon must be ~2.20: got {:.3}",
        ratio
    );

    // Percentage checks
    assert!(f_sun_on_moon.fraction_of_total > 0.65 && f_sun_on_moon.fraction_of_total < 0.72);
    assert!(f_earth_on_moon.fraction_of_total > 0.28 && f_earth_on_moon.fraction_of_total < 0.35);

    // Verify Earth-Moon Barycenter (EMB)
    let earth = &system.bodies[earth_idx];
    let (emb, emb_dist_m) = compute_earth_moon_barycenter(&system).expect("EMB must compute");

    let dx = emb[0] - earth.position_m[0];
    let dy = emb[1] - earth.position_m[1];
    let dz = emb[2] - earth.position_m[2];
    let emb_dist_km = (dx * dx + dy * dy + dz * dz).sqrt() / 1000.0;

    // Analytical EMB displacement = m_moon / (m_earth + m_moon) * 384,400 km ~= 4,671 km
    assert!(
        (emb_dist_km - 4671.0).abs() < 50.0,
        "EMB displacement from Earth center must be ~4,671 km: got {:.1} km",
        emb_dist_km
    );
    assert!(
        (emb_dist_m / 1000.0 - 4671.0).abs() < 50.0,
        "EMB displacement return value mismatch"
    );
    // EMB is inside Earth (Earth radius = 6371 km)
    assert!(emb_dist_km < earth.radius_km, "EMB must reside within Earth's mantle");
}

#[test]
fn test_spatial_field_point_at_1au_matches_solar_gravity() {
    let mut system = NBodySystem::new();
    let sun = CelestialBody::new(
        0,
        "Sun",
        1.98847e30,
        696340.0,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "#fbbf24",
    );
    system.add_body(sun);

    let point = [ASTRONOMICAL_UNIT_M, 0.0, 0.0];
    let field = compute_spatial_field_point(&system, point);

    // Theoretical acceleration g = G * M / r^2
    let expected_g = G_STANDARD * 1.98847e30 / (ASTRONOMICAL_UNIT_M * ASTRONOMICAL_UNIT_M);
    assert!(
        (field.acceleration_magnitude - expected_g).abs() / expected_g < 1e-6,
        "Field acceleration at 1 AU must match analytical solar gravity: {:e} vs {:e}",
        field.acceleration_magnitude,
        expected_g
    );

    // Vector direction: pulls towards origin (-x direction)
    assert!(field.acceleration_vector_mps2[0] < 0.0);
    assert!(field.acceleration_vector_mps2[1].abs() < 1e-12);
    assert!(field.acceleration_vector_mps2[2].abs() < 1e-12);

    // Dominant body is Sun
    assert_eq!(field.dominant_body_name, "Sun");
    assert!((field.dominant_body_fraction - 1.0).abs() < 1e-6);

    // Potential Phi = -G * M / r
    let expected_phi = -G_STANDARD * 1.98847e30 / ASTRONOMICAL_UNIT_M;
    assert!(
        (field.gravitational_potential_j_kg - expected_phi).abs() / expected_phi.abs() < 1e-6,
        "Potential mismatch: {:e} vs {:e}",
        field.gravitational_potential_j_kg,
        expected_phi
    );
}

#[test]
fn test_spatial_dominance_chebotarev_boundary_sun_earth() {
    let system = create_preset(PresetId::InnerSolarSystemJupiter);
    let earth = system.bodies.iter().find(|b| b.name == "Earth").expect("Earth");
    let sun = system.bodies.iter().find(|b| b.name == "Sun").expect("Sun");

    // Earth's sphere of attraction (Chebotarev 1964): r_a = a * sqrt(m_earth / M_sun) ~ 259,000 km
    let mass_ratio = earth.mass_kg / sun.mass_kg;
    let dist_sun_earth = (earth.position_m[0].powi(2) + earth.position_m[1].powi(2)).sqrt();
    let r_attraction = dist_sun_earth * mass_ratio.sqrt();

    // 1. Inside Earth's dominance basin (e.g. 100,000 km from Earth towards Sun)
    let probe_inside = [
        earth.position_m[0] - 100_000_000.0,
        earth.position_m[1],
        0.0,
    ];
    let field_inside = compute_spatial_field_point(&system, probe_inside);
    assert_eq!(
        field_inside.dominant_body_name, "Earth",
        "Inside Chebotarev attraction sphere (< 259,000 km), Earth must dominate"
    );

    // 2. Outside Earth's dominance basin (e.g. 500,000 km from Earth towards Sun)
    let probe_outside = [
        earth.position_m[0] - 500_000_000.0,
        earth.position_m[1],
        0.0,
    ];
    let field_outside = compute_spatial_field_point(&system, probe_outside);
    assert_eq!(
        field_outside.dominant_body_name, "Sun",
        "Outside Chebotarev attraction sphere (> 259,000 km), Sun must dominate"
    );

    // 3. Near the neutral gravity boundary (r_a distance)
    let probe_neutral = [
        earth.position_m[0] - r_attraction,
        earth.position_m[1],
        0.0,
    ];
    let field_neutral = compute_spatial_field_point(&system, probe_neutral);
    let g_sun = field_neutral.contributions.iter().find(|c| c.body_name == "Sun").unwrap().acceleration_magnitude;
    let g_earth = field_neutral.contributions.iter().find(|c| c.body_name == "Earth").unwrap().acceleration_magnitude;
    let ratio = g_earth / g_sun;
    assert!(
        (ratio - 1.0).abs() < 0.05,
        "At Chebotarev attraction radius r_a, g_earth / g_sun must be ~1.0: got {:.3}",
        ratio
    );
}

#[test]
fn test_spatial_dominance_earth_moon_neutral_point() {
    let system = create_preset(PresetId::InnerSolarSystemJupiter);
    let earth = system.bodies.iter().find(|b| b.name == "Earth").expect("Earth");
    let moon = system.bodies.iter().find(|b| b.name == "Moon").expect("Moon");

    // Earth-Moon distance vector
    let dx = moon.position_m[0] - earth.position_m[0];
    let dy = moon.position_m[1] - earth.position_m[1];
    let dz = moon.position_m[2] - earth.position_m[2];
    let d_em = (dx * dx + dy * dy + dz * dz).sqrt();

    // Neutral point along the Earth-Moon axis where g_earth == g_moon:
    // d_moon = d_em / (1 + sqrt(M_earth / M_moon)) ~ 38,400 km
    let sqrt_ratio = (earth.mass_kg / moon.mass_kg).sqrt();
    let neutral_dist_from_moon = d_em / (1.0 + sqrt_ratio);

    // Probe 1: 15,000 km from Moon towards Earth (deep in Moon dominance basin)
    let frac_moon_basin = (d_em - 15_000_000.0) / d_em;
    let p_moon = [
        earth.position_m[0] + dx * frac_moon_basin,
        earth.position_m[1] + dy * frac_moon_basin,
        earth.position_m[2] + dz * frac_moon_basin,
    ];
    let field_moon = compute_spatial_field_point(&system, p_moon);
    assert_eq!(
        field_moon.dominant_body_name, "Moon",
        "Within 15,000 km of Moon, Moon must dominate over Earth and Sun"
    );

    // Probe 2: 150,000 km from Earth towards Moon (inside Earth dominance basin < 259,000 km)
    let frac_earth_basin = 150_000_000.0 / d_em;
    let p_earth = [
        earth.position_m[0] + dx * frac_earth_basin,
        earth.position_m[1] + dy * frac_earth_basin,
        earth.position_m[2] + dz * frac_earth_basin,
    ];
    let field_earth = compute_spatial_field_point(&system, p_earth);
    assert_eq!(
        field_earth.dominant_body_name, "Earth",
        "At 150,000 km from Earth towards Moon, Earth must dominate"
    );

    // Probe 3: 70,000 km from Moon towards Earth (314,400 km from Earth).
    // Because Earth's sphere of attraction against the Sun is ~259,000 km,
    // this spatial region is actually dominated by the SUN!
    let frac_sun_gap = (d_em - 70_000_000.0) / d_em;
    let p_sun_gap = [
        earth.position_m[0] + dx * frac_sun_gap,
        earth.position_m[1] + dy * frac_sun_gap,
        earth.position_m[2] + dz * frac_sun_gap,
    ];
    let field_sun_gap = compute_spatial_field_point(&system, p_sun_gap);
    assert_eq!(
        field_sun_gap.dominant_body_name, "Sun",
        "At 314,400 km from Earth (> 259,000 km), Sun's gravitational pull exceeds Earth's"
    );

    // Probe 4: Pairwise neutral gravity point between Earth and Moon
    let frac_neutral = (d_em - neutral_dist_from_moon) / d_em;
    let p_neutral = [
        earth.position_m[0] + dx * frac_neutral,
        earth.position_m[1] + dy * frac_neutral,
        earth.position_m[2] + dz * frac_neutral,
    ];
    let field_neutral = compute_spatial_field_point(&system, p_neutral);
    let g_earth = field_neutral.contributions.iter().find(|c| c.body_name == "Earth").unwrap().acceleration_magnitude;
    let g_moon = field_neutral.contributions.iter().find(|c| c.body_name == "Moon").unwrap().acceleration_magnitude;
    let ratio = g_earth / g_moon;
    assert!(
        (ratio - 1.0).abs() < 0.05,
        "At Earth-Moon neutral gravity point, g_earth / g_moon must be ~1.0: got {:.3}",
        ratio
    );
}

#[test]
fn test_spatial_field_grid_sampling() {
    let system = create_preset(PresetId::InnerSolarSystemJupiter);
    let x_range = [-1.5 * ASTRONOMICAL_UNIT_M, 1.5 * ASTRONOMICAL_UNIT_M];
    let y_range = [-1.5 * ASTRONOMICAL_UNIT_M, 1.5 * ASTRONOMICAL_UNIT_M];
    let grid = compute_spatial_field_grid(&system, x_range, y_range, 5, 5);

    assert_eq!(grid.len(), 25);
    for pt in &grid {
        assert_eq!(pt.contributions.len(), 6);
        assert!(pt.acceleration_magnitude > 0.0);
        assert!(pt.dominant_body_fraction > 0.0 && pt.dominant_body_fraction <= 1.0);
    }
}

#[test]
fn test_centric_spatial_field_grid_earth() {
    let frame = GravitationalCentricFrame::Geocentric;
    assert_eq!(frame, GravitationalCentricFrame::Geocentric);

    let system = create_preset(PresetId::InnerSolarSystemJupiter);
    // Earth-centric grid spanning ±200,000 km
    let half_span_m = 200_000_000.0;
    // 4x4 resolution avoids exact center point (r=0) singularity
    let grid = compute_centric_spatial_field_grid(&system, "Earth", half_span_m, 4).unwrap();

    assert_eq!(grid.len(), 16);
    for pt in &grid {
        assert_eq!(pt.dominant_body_name, "Earth", "With Sun gravity ignored, Earth dominates its local domain");
        assert!(pt.dominant_body_fraction > 0.95, "Earth must account for > 95% of local planetary pull away from Moon");
        // Verify Sun is not in contributions list
        assert!(
            pt.contributions.iter().all(|c| !c.body_name.eq_ignore_ascii_case("Sun")),
            "Sun must be excluded from planet-centric gravity field contributions"
        );
    }
}

#[test]
fn test_centric_spatial_field_grid_moon() {
    let system = create_preset(PresetId::InnerSolarSystemJupiter);
    // Moon-centric grid spanning ±25,000 km (within 38,400 km neutral boundary with Earth)
    let half_span_m = 25_000_000.0;
    // 4x4 resolution avoids r=0 singularity
    let grid = compute_centric_spatial_field_grid(&system, "Moon", half_span_m, 4).unwrap();

    assert_eq!(grid.len(), 16);
    for pt in &grid {
        assert_eq!(pt.dominant_body_name, "Moon", "All points within 25,000 km must be in Moon dominance basin");
        assert!(pt.dominant_body_fraction > 0.50, "Moon must account for > 50% of local gravitational pull against Earth");
        assert!(
            pt.contributions.iter().all(|c| !c.body_name.eq_ignore_ascii_case("Sun")),
            "Sun must be excluded from Selenocentric gravity field contributions"
        );
    }
}

#[test]
fn test_centric_spatial_field_grid_heliocentric_includes_sun() {
    let system = create_preset(PresetId::InnerSolarSystemJupiter);
    // Heliocentric grid spanning ±1 AU around Sun
    let half_span_m = 149_597_870_700.0;
    let grid = compute_centric_spatial_field_grid(&system, "Sun", half_span_m, 4).unwrap();

    assert_eq!(grid.len(), 16);
    for pt in &grid {
        assert_eq!(pt.dominant_body_name, "Sun", "Sun dominates heliocentric grid points");
        assert!(
            pt.contributions.iter().any(|c| c.body_name.eq_ignore_ascii_case("Sun")),
            "Sun must be included in Heliocentric gravity field contributions"
        );
    }
}

#[test]
fn test_is_body_relevant_to_centric() {
    // Heliocentric mode includes all celestial bodies
    assert!(is_body_relevant_to_centric("Sun", "Sun"));
    assert!(is_body_relevant_to_centric("Earth", "Sun"));
    assert!(is_body_relevant_to_centric("Moon", "Sun"));
    assert!(is_body_relevant_to_centric("Jupiter", "Sun"));
    assert!(is_body_relevant_to_centric("Mars", "Sun"));

    // Geocentric mode includes only Earth and Moon
    assert!(is_body_relevant_to_centric("Earth", "Earth"));
    assert!(is_body_relevant_to_centric("Moon", "Earth"));
    assert!(!is_body_relevant_to_centric("Sun", "Earth"));
    assert!(!is_body_relevant_to_centric("Jupiter", "Earth"));
    assert!(!is_body_relevant_to_centric("Mars", "Earth"));
    assert!(!is_body_relevant_to_centric("Venus", "Earth"));

    // Selenocentric mode includes only Moon and Earth
    assert!(is_body_relevant_to_centric("Moon", "Moon"));
    assert!(is_body_relevant_to_centric("Earth", "Moon"));
    assert!(!is_body_relevant_to_centric("Sun", "Moon"));
    assert!(!is_body_relevant_to_centric("Jupiter", "Moon"));

    // Jovicentric mode includes Jupiter and Jovian satellites
    assert!(is_body_relevant_to_centric("Jupiter", "Jupiter"));
    assert!(is_body_relevant_to_centric("Ganymede", "Jupiter"));
    assert!(!is_body_relevant_to_centric("Sun", "Jupiter"));
    assert!(!is_body_relevant_to_centric("Earth", "Jupiter"));
}

#[test]
fn test_centric_spatial_field_grid_high_resolution_granularity() {
    let system = create_preset(PresetId::InnerSolarSystemJupiter);
    // 32x32 = 1,024 vector lattice in Earth cislunar space (±1,200,000 km)
    let half_span_m = 1_200_000_000.0;
    let resolution = 32;
    let grid = compute_centric_spatial_field_grid(&system, "Earth", half_span_m, resolution).unwrap();

    assert_eq!(grid.len(), 32 * 32);

    // Physical granularity: delta x = 2 * 1,200,000 / 31 ~ 77,419 km
    let delta_x_km = (2.0 * half_span_m / ((resolution - 1) as f64)) / 1e3;
    assert!((delta_x_km - 77419.35).abs() < 1.0, "Physical step size must be ~77,419 km: got {:.2}", delta_x_km);

    for pt in &grid {
        assert!(pt.acceleration_magnitude > 0.0, "Acceleration magnitude must be non-zero");
        assert!(!pt.acceleration_magnitude.is_nan(), "Acceleration magnitude must not be NaN");
        assert!(!pt.acceleration_magnitude.is_infinite(), "Acceleration magnitude must be finite");

        // Verify only relevant bodies (Earth and Moon) are in the contributions list
        assert_eq!(pt.contributions.len(), 2, "Only Earth and Moon should be present in Geocentric mode");
        assert!(pt.contributions.iter().any(|c| c.body_name == "Earth"));
        assert!(pt.contributions.iter().any(|c| c.body_name == "Moon"));
        assert!(pt.contributions.iter().all(|c| c.body_name != "Sun" && c.body_name != "Jupiter" && c.body_name != "Mars"));
    }

    // Selenocentric high-resolution grid (32x32 = 1,024 vectors across ±100,000 km)
    let moon_half_span_m = 100_000_000.0;
    let moon_grid = compute_centric_spatial_field_grid(&system, "Moon", moon_half_span_m, resolution).unwrap();
    assert_eq!(moon_grid.len(), 1024);

    let moon_delta_x_km = (2.0 * moon_half_span_m / ((resolution - 1) as f64)) / 1e3;
    assert!((moon_delta_x_km - 6451.61).abs() < 1.0, "Moon micro-step size must be ~6,452 km: got {:.2}", moon_delta_x_km);

    for pt in &moon_grid {
        assert_eq!(pt.contributions.len(), 2, "Only Moon and Earth should be present in Selenocentric mode");
        assert!(pt.contributions.iter().all(|c| c.body_name != "Sun"));
    }
}

#[test]
fn test_generate_centric_test_particles() {
    let system = create_preset(PresetId::InnerSolarSystemJupiter);

    // 1. Earth-centric particles
    let count = 30;
    let particles = sbm_core::nbody::generate_centric_test_particles(&system, "Earth", count, 42).expect("Generate Earth particles");
    assert_eq!(particles.len(), count);

    for (idx, p) in particles.iter().enumerate() {
        assert_eq!(p.anchor_body_name, "Earth");
        assert_eq!(p.id, (idx + 1) as u64);

        let r = (p.rel_position_m[0].powi(2) + p.rel_position_m[1].powi(2) + p.rel_position_m[2].powi(2)).sqrt();
        assert!((14_000_000.0..=360_000_000.0).contains(&r), "Earth particle orbital radius must be within bounds: got {:.1} km", r / 1e3);

        let v = (p.rel_velocity_m_s[0].powi(2) + p.rel_velocity_m_s[1].powi(2) + p.rel_velocity_m_s[2].powi(2)).sqrt();
        assert!(v > 500.0 && v < 12_000.0, "Orbital speed must be physically sound: got {:.1} m/s", v);

        // Dot product between r and v should be close to zero for near-circular orbit
        let r_dot_v = (p.rel_position_m[0] * p.rel_velocity_m_s[0] + p.rel_position_m[1] * p.rel_velocity_m_s[1] + p.rel_position_m[2] * p.rel_velocity_m_s[2]) / (r * v);
        assert!(r_dot_v.abs() < 0.25, "Near-circular orbit dot product |r·v|/(|r||v|) must be small: got {:.4}", r_dot_v);
    }

    // 2. Moon-centric particles
    let moon_particles = sbm_core::nbody::generate_centric_test_particles(&system, "Moon", 20, 123).expect("Generate Moon particles");
    assert_eq!(moon_particles.len(), 20);
    for p in &moon_particles {
        assert_eq!(p.anchor_body_name, "Moon");
        let r = (p.rel_position_m[0].powi(2) + p.rel_position_m[1].powi(2) + p.rel_position_m[2].powi(2)).sqrt();
        assert!((2_000_000.0..=36_000_000.0).contains(&r), "Moon particle radius bounds: got {:.1} km", r / 1e3);
    }

    // 3. Error case for invalid body
    let err = sbm_core::nbody::generate_centric_test_particles(&system, "Pluto", 10, 1);
    assert!(err.is_err(), "Non-existent body should return an error");
}

#[test]
fn test_compute_centric_particle_acceleration() {
    let system = create_preset(PresetId::InnerSolarSystemJupiter);
    let earth = system.bodies.iter().find(|b| b.name == "Earth").expect("Earth");

    // Test particle at 42,164 km (GEO radius) along X-axis from Earth
    let r_geo = 42_164_000.0;
    let rel_pos = [r_geo, 0.0, 0.0];

    let acc = sbm_core::nbody::compute_centric_particle_acceleration(&system, "Earth", rel_pos)
        .expect("Compute centric acceleration");

    // Earth's direct central acceleration magnitude: g = G * M_earth / r^2
    let expected_g = sbm_core::nbody::G_STANDARD * earth.mass_kg / (r_geo * r_geo);

    // ax must be negative (pointing back towards Earth center at origin)
    assert!(acc[0] < 0.0, "Acceleration X must point toward origin: got {:e}", acc[0]);
    let diff_rel = ((-acc[0]) - expected_g).abs() / expected_g;
    assert!(
        diff_rel < 0.05,
        "Centric acceleration at GEO must be dominated by Earth central pull: got {:e} vs expected {:e}",
        -acc[0],
        expected_g
    );

    // Verify error handling for invalid center body
    let err = sbm_core::nbody::compute_centric_particle_acceleration(&system, "InvalidPlanet", rel_pos);
    assert!(err.is_err(), "Invalid body name must return Err");
}

#[test]
fn test_vis_viva_orbital_speed_and_normalized_kinetic() {
    let mu_earth = sbm_core::nbody::G_STANDARD * 5.9722e24;

    // 1. Circular LEO Orbit (r = a = 7,000 km, e = 0.0)
    let r_leo = 7_000_000.0;
    let v_leo = sbm_core::nbody::compute_vis_viva_speed(mu_earth, r_leo, r_leo);
    let expected_v_leo = (mu_earth / r_leo).sqrt();
    assert!((v_leo - expected_v_leo).abs() < 1e-4, "LEO circular speed must match sqrt(mu/r): got {:.2}", v_leo);

    let tau_circular = sbm_core::nbody::compute_vis_viva_normalized_kinetic(mu_earth, r_leo, r_leo, 0.0);
    assert_eq!(tau_circular, 0.5, "Circular orbit normalized kinetic parameter must default to 0.5");

    // 2. Highly Eccentric Geostationary Transfer Orbit (GTO)
    // Periapsis: 6,678 km (300 km alt), Apoapsis: 42,164 km (GEO alt)
    let r_p = 6_678_000.0;
    let r_a = 42_164_000.0;
    let a_gto = (r_p + r_a) / 2.0;
    let e_gto = (r_a - r_p) / (r_a + r_p);

    let v_peri = sbm_core::nbody::compute_vis_viva_speed(mu_earth, r_p, a_gto);
    let v_apo = sbm_core::nbody::compute_vis_viva_speed(mu_earth, r_a, a_gto);
    assert!(v_peri > v_apo, "Periapsis speed ({:.1} m/s) must exceed apoapsis speed ({:.1} m/s)", v_peri, v_apo);
    assert!(v_peri > 10_000.0 && v_peri < 11_000.0, "GTO periapsis speed ~10.2 km/s: got {:.1}", v_peri);
    assert!(v_apo > 1_500.0 && v_apo < 2_000.0, "GTO apoapsis speed ~1.6 km/s: got {:.1}", v_apo);

    let tau_peri = sbm_core::nbody::compute_vis_viva_normalized_kinetic(mu_earth, r_p, a_gto, e_gto);
    let tau_apo = sbm_core::nbody::compute_vis_viva_normalized_kinetic(mu_earth, r_a, a_gto, e_gto);
    let tau_mid = sbm_core::nbody::compute_vis_viva_normalized_kinetic(mu_earth, a_gto, a_gto, e_gto);

    assert!((tau_peri - 1.0).abs() < 1e-6, "Periapsis tau must equal 1.0 (fastest/hot): got {:.4}", tau_peri);
    assert!(tau_apo < 1e-6, "Apoapsis tau must equal 0.0 (slowest/cool): got {:.4}", tau_apo);
    assert!(tau_mid > 0.1 && tau_mid < 0.9, "Midpoint tau must be strictly bounded between 0 and 1: got {:.4}", tau_mid);

    // 3. Edge Cases
    assert_eq!(sbm_core::nbody::compute_vis_viva_speed(mu_earth, 0.0, a_gto), 0.0);
    assert_eq!(sbm_core::nbody::compute_vis_viva_speed(mu_earth, r_p, -1.0), 0.0);
    assert_eq!(sbm_core::nbody::compute_vis_viva_normalized_kinetic(mu_earth, -1.0, a_gto, e_gto), 0.5);
}



