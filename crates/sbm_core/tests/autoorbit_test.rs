//! Integration tests for the AutoOrbit physics-informed satellite orbit prediction engine.
//!
//! Grounded in the KDD 2026 paper:
//! > **Zhang, D., Liang, S., Yang, B., Qi, T., Wang, S., & Li, Q. (2026).**  
//! > *AutoOrbit: Physics-Informed Satellite Orbit Prediction.*  
//! > In Proceedings of the 32nd ACM SIGKDD Conference on Knowledge Discovery and Data Mining (KDD '26).
//! > DOI: 10.1145/3770855.3818960

use sbm_core::autoorbit::{
    apply_maneuver_correction, cartesian_to_keplerian, compute_gve_element_deltas,
    compute_theoretical_acceleration, keplerian_to_cartesian, AutoOrbitPredictor,
    KeplerianElements, ManeuverImpulse, PredictionHorizon, SpacecraftPhysicalParams,
    StateVector, EARTH_MU,
};

#[test]
fn test_cartesian_keplerian_inclined_eccentric_roundtrip() {
    // Highly inclined (sun-synchronous ~98.2 deg), slightly eccentric LEO orbit
    let inc = 98.18 * core::f64::consts::PI / 180.0;
    let omega = 45.0 * core::f64::consts::PI / 180.0;
    let raan = 120.0 * core::f64::consts::PI / 180.0;
    let nu = 30.0 * core::f64::consts::PI / 180.0;
    let a = 7_070_000.0;
    let e = 0.0012;

    let elements = KeplerianElements::new(a, e, inc, raan, omega, nu);
    let cart = keplerian_to_cartesian(&elements, EARTH_MU);

    // Convert back to Keplerian
    let recovered = cartesian_to_keplerian(&cart, EARTH_MU).expect("Valid orbit");

    assert!((recovered.semi_major_axis_m - a).abs() < 1e-3, "a mismatch");
    assert!((recovered.eccentricity - e).abs() < 1e-6, "e mismatch");
    assert!((recovered.inclination_rad - inc).abs() < 1e-6, "inc mismatch");
    assert!((recovered.raan_rad - raan).abs() < 1e-5, "raan mismatch");
    assert!((recovered.arg_periapsis_rad - omega).abs() < 1e-5, "omega mismatch");
    assert!((recovered.true_anomaly_rad - nu).abs() < 1e-5, "nu mismatch");
}

#[test]
fn test_gve_equations_radial_and_crosstrack_components() {
    let a = 7_000_000.0;
    let e = 0.01;
    let inc = 45.0 * core::f64::consts::PI / 180.0;
    let elements = KeplerianElements::new(a, e, inc, 0.0, 0.0, 0.0);

    // 1. Radial thrust impulse: Delta vr = 1.0 m/s
    // Eq. 12: Delta omega = 1 / (e v) * [-cos(nu) * dvr]
    // At nu = 0, cos(0) = 1 -> Delta omega = -1 / (e v) * dvr < 0
    let impulse_radial = ManeuverImpulse::new(1.0, 0.0, 0.0, 0.0);
    let (da_r, de_r, d_omega_r, d_inc_r, _) = compute_gve_element_deltas(&elements, &impulse_radial, EARTH_MU);
    assert!(da_r.abs() < 1e-5, "Radial thrust at periapsis should produce zero Delta a");
    assert!(de_r.abs() < 1e-5, "Radial thrust at periapsis should produce zero Delta e");
    assert!(d_omega_r < 0.0, "Radial thrust at periapsis should rotate periapsis backward");
    assert_eq!(d_inc_r, 0.0, "Radial thrust produces zero plane change");

    // 2. Cross-track thrust impulse: Delta vc = 1.0 m/s
    // Produces inclination change Delta inc = r cos(u) / h * dvc
    let impulse_normal = ManeuverImpulse::new(0.0, 0.0, 1.0, 0.0);
    let (da_c, de_c, _, d_inc_c, _) = compute_gve_element_deltas(&elements, &impulse_normal, EARTH_MU);
    assert_eq!(da_c, 0.0);
    assert_eq!(de_c, 0.0);
    assert!(d_inc_c > 0.0, "Positive cross-track impulse raises inclination");
}

#[test]
fn test_maneuver_correction_prevents_unmodeled_thrust_divergence() {
    let initial_state = StateVector::new(7_070_000.0, 0.0, 0.0, 0.0, 7500.0, 0.0);
    let impulse = ManeuverImpulse::new(0.0, 1.0, 0.0, 0.0); // 1.0 m/s along-track burn

    let num_steps = 60;
    let cadence_s = 10.0;

    // Propagate post-maneuver trajectory with GVE analytical correction
    let post_maneuver_traj = apply_maneuver_correction(&initial_state, &impulse, num_steps, cadence_s, EARTH_MU)
        .expect("Maneuver correction succeeds");

    assert_eq!(post_maneuver_traj.len(), num_steps);

    // Propagate unmodeled passive trajectory (assuming no maneuver)
    let passive_impulse = ManeuverImpulse::new(0.0, 0.0, 0.0, 0.0);
    let passive_traj = apply_maneuver_correction(&initial_state, &passive_impulse, num_steps, cadence_s, EARTH_MU)
        .expect("Passive propagation succeeds");

    // After 10 minutes (600s), an unmodeled 1.0 m/s burn drifts by Delta r ~ 600m to 1km
    let end_post = post_maneuver_traj.last().unwrap();
    let end_passive = passive_traj.last().unwrap();

    let dx = end_post.x - end_passive.x;
    let dy = end_post.y - end_passive.y;
    let dz = end_post.z - end_passive.z;
    let pos_divergence = (dx * dx + dy * dy + dz * dz).sqrt();

    // The unmodeled divergence exceeds 500m
    assert!(
        pos_divergence > 500.0,
        "Expected significant maneuver drift (>500m), got {:.1}m",
        pos_divergence
    );
}

#[test]
fn test_physics_acceleration_j2_effect() {
    let r = 7_000_000.0;
    let equatorial_state = StateVector::new(r, 0.0, 0.0, 0.0, 7500.0, 0.0);
    let polar_state = StateVector::new(0.0, 0.0, r, 7500.0, 0.0, 0.0);

    let params = SpacecraftPhysicalParams {
        include_drag: false,
        include_geopotential: true,
        ..Default::default()
    };

    let a_eq = compute_theoretical_acceleration(&equatorial_state, &params, EARTH_MU);
    let a_polar = compute_theoretical_acceleration(&polar_state, &params, EARTH_MU);

    let a_eq_mag = (a_eq[0] * a_eq[0] + a_eq[1] * a_eq[1] + a_eq[2] * a_eq[2]).sqrt();
    let a_pol_mag = (a_polar[0] * a_polar[0] + a_polar[1] * a_polar[1] + a_polar[2] * a_polar[2]).sqrt();

    // Due to Earth's oblateness (J2), equatorial gravity is slightly higher than polar gravity at identical distance r
    let diff = a_eq_mag - a_pol_mag;
    assert!(diff > 0.01 && diff < 0.05, "J2 differential was {:.4} m/s^2", diff);
}

#[test]
fn test_autoorbit_predictor_evaluates_paper_accuracies() {
    let predictor = AutoOrbitPredictor::sentinel_1a_preset();

    // Generate observations spanning 128 time steps
    let mut obs = Vec::with_capacity(128);
    for step in 0..128 {
        let state = predictor.reference_orbit.state_at_step(step);
        // Inject nominal GNSS noise (~10m 1-sigma per paper Table 6)
        obs.push(state.add_noise([8.0, -6.0, 4.0], [0.05, -0.03, 0.02]));
    }

    // Predict across horizons from Table 2
    let horizons = [
        (PredictionHorizon::HalfHour, 150.0),
        (PredictionHorizon::OneHour, 200.0),
        (PredictionHorizon::FourHours, 280.0),
    ];

    for (horizon, max_pos_threshold) in horizons {
        let pred = predictor.predict(&obs, 128, horizon, None).expect("Prediction ok");
        let steps = horizon.steps_at_cadence(10.0);
        assert_eq!(pred.len(), steps);

        let mut truth = Vec::with_capacity(steps);
        for s in 0..steps {
            truth.push(predictor.reference_orbit.state_at_step(128 + s));
        }

        let metrics = predictor.evaluate(&pred, &truth);
        assert!(
            metrics.position_error_m < max_pos_threshold,
            "Horizon {:?} had pos error {:.2}m, exceeding threshold {:.1}m",
            horizon,
            metrics.position_error_m,
            max_pos_threshold
        );
    }
}
