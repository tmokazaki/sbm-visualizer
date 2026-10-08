//! Unit and integration tests for sbm_avoidance framework.
//!
//! Grounded in Luna et al. (2026):
//! - Astrodynamics & 3-body gravitational fields (Algorithm 1)
//! - Low-thrust propulsion & Tsiolkovsky mass depletion (Algorithm 2)
//! - Conjunction assessment, TCA, and hazard scoring (Eq. 9)
//! - Baseline controllers (No-Action, Rule-Based, Impulsive Planner)
//! - Neural network architecture & embedded pre-trained policy inference
//! - PPO training loop & GAE advantage estimation
//! - Checkpoint serialization roundtrip

use sbm_avoidance::baselines::ControllerContext;
use sbm_avoidance::prelude::*;

#[test]
fn test_gravitational_acceleration_leo() {
    // 500 km circular LEO orbit along x-axis
    let r_leo = EARTH_RADIUS_M + 500_000.0;
    let sat_pos = Vector3D::new(r_leo, 0.0, 0.0);

    let a_total = compute_total_gravitational_acceleration(sat_pos, 0.0);

    // Theoretical central gravity: a = mu / r^2 towards origin
    let expected_central_mag = MU_EARTH / (r_leo * r_leo);

    // Perturbations from Sun and Moon (Algorithm 1)
    let r_sun_rel = sun_position_earth_centered(0.0) - sat_pos;
    let a_sun_x = r_sun_rel.x * (MU_SUN / r_sun_rel.norm().powi(3));
    let r_moon_rel = moon_position_earth_centered(0.0) - sat_pos;
    let a_moon_x = r_moon_rel.x * (MU_MOON / r_moon_rel.norm().powi(3));

    let expected_total_x = -expected_central_mag + a_sun_x + a_moon_x;
    assert!((a_total.x - expected_total_x).abs() < 1e-6, "Must match Algorithm 1 formulation exactly");
    assert!(a_total.x < -8.0, "Earth central gravity must dominate (>8 m/s^2)");
    assert!(a_total.y.abs() < 1e-4, "y-acceleration from perturbations must be small");
    assert!(a_total.z.abs() < 1e-4, "z-acceleration from perturbations must be small");
}

#[test]
fn test_tsiolkovsky_propellant_depletion() {
    let mut sat = SatelliteState::new_leo_equatorial(700_000.0);
    assert_eq!(sat.total_mass_kg, 1000.0);
    assert_eq!(sat.fuel_mass_kg, 500.0);

    let max_thrust = MAX_THRUST_ACCELERATION; // 0.15 m/s^2
    let dt = 1.0;
    let action = Action3D::new(1.0, 0.0, 0.0); // full thrust in +x

    let (a_thrust, dv, fuel_burned) = apply_thrust_step(&mut sat, action, max_thrust, dt);

    assert!((a_thrust.x - 0.15).abs() < 1e-6);
    assert!((dv - 0.15).abs() < 1e-6);

    // Theoretical fuel burned: m_consumed = m * (1 - exp(-dv / (Isp * g0)))
    let ve = SPECIFIC_IMPULSE_S * G0_STANDARD;
    let expected_fuel = 1000.0 * (1.0 - (-0.15 / ve).exp());
    assert!((fuel_burned - expected_fuel).abs() < 1e-6);
    assert!((sat.fuel_mass_kg - (500.0 - expected_fuel)).abs() < 1e-6);
    assert!((sat.total_mass_kg - (1000.0 - expected_fuel)).abs() < 1e-6);
}

#[test]
fn test_fuel_exhaustion_cutoff() {
    let mut sat = SatelliteState {
        position: Vector3D::new(EARTH_RADIUS_M + 500_000.0, 0.0, 0.0),
        velocity: Vector3D::zero(),
        total_mass_kg: 500.0,
        fuel_mass_kg: 0.0, // Depleted!
    };

    let action = Action3D::new(1.0, 1.0, 1.0);
    let (a_thrust, dv, fuel_burned) = apply_thrust_step(&mut sat, action, 0.15, 1.0);

    assert_eq!(a_thrust, Vector3D::zero(), "Depleted thruster cannot generate thrust");
    assert_eq!(dv, 0.0);
    assert_eq!(fuel_burned, 0.0);
}

#[test]
fn test_conjunction_assessment_geometry() {
    let sat_pos = Vector3D::new(7_000_000.0, 0.0, 0.0);
    let sat_vel = Vector3D::new(0.0, 7500.0, 0.0);

    // Debris 1000 meters along y, flying head-on at relative velocity 100 m/s
    let deb_pos = Vector3D::new(7_000_000.0, 1000.0, 0.0);
    let deb_vel = Vector3D::new(0.0, 7400.0, 0.0); // rel_vel = deb_vel - sat_vel = [0, -100, 0]

    let metrics = assess_conjunction(sat_pos, sat_vel, deb_pos, deb_vel, 25.0, 25.0);

    assert_eq!(metrics.distance_m, 1000.0);
    assert!((metrics.closing_speed_mps - 100.0).abs() < 1e-6);
    assert!((metrics.tca_seconds - 10.0).abs() < 1e-6);
    assert!(metrics.projected_miss_m < 1e-3, "Head-on collision has zero miss distance");
    assert!(metrics.hazard_score > 0.5, "Urgent head-on encounter must have high hazard score");
}

#[test]
fn test_conjunction_diverging() {
    let sat_pos = Vector3D::new(7_000_000.0, 0.0, 0.0);
    let sat_vel = Vector3D::new(0.0, 7500.0, 0.0);

    // Debris moving away at +100 m/s
    let deb_pos = Vector3D::new(7_000_000.0, 1000.0, 0.0);
    let deb_vel = Vector3D::new(0.0, 7600.0, 0.0); // rel_vel = [0, +100, 0]

    let metrics = assess_conjunction(sat_pos, sat_vel, deb_pos, deb_vel, 25.0, 25.0);

    assert!(metrics.closing_speed_mps < 0.0, "Diverging object has negative closing speed");
    assert!(metrics.tca_seconds.is_infinite(), "Diverging object has infinite TCA");
}

#[test]
fn test_baseline_controllers() {
    let sat = SatelliteState::new_leo_equatorial(700_000.0);
    let debris = vec![DebrisObject {
        position: sat.position + Vector3D::new(50.0, 0.0, 0.0),
        velocity: sat.velocity + Vector3D::new(-10.0, 0.0, 0.0),
        radius_m: 25.0,
    }];

    let ctx = ControllerContext {
        satellite: &sat,
        debris_field: &debris,
        target_debris_idx: Some(0),
        collision_threshold_m: 25.0,
        safe_buffer_m: 50.0,
        max_thrust: 0.15,
        dt_seconds: 1.0,
    };

    // 1. No-action controller
    let mut no_action = NoActionController;
    let act_none = no_action.compute_action(&ctx);
    assert_eq!(act_none, Action3D::zero());

    // 2. Risk-aware rule-based controller
    let mut rule_based = RiskAwareRuleBasedController::default();
    let act_rule = rule_based.compute_action(&ctx);
    assert!(act_rule.magnitude() > 0.0, "Rule-based must thrust away from imminent debris");
    assert!(act_rule.ax < 0.0, "Must thrust in -x away from +x debris position");

    // 3. Impulsive planner controller
    let mut impulsive = ImpulsivePlannerController::default();
    let act_imp = impulsive.compute_action(&ctx);
    assert!(act_imp.magnitude() > 0.0, "Impulsive planner must thrust laterally");
}

#[test]
fn test_curriculum_scheduler() {
    let scheduler = CurriculumScheduler::default();

    let stage1 = scheduler.get_stage(0);
    assert_eq!(stage1.name, CurriculumStageName::BasicAvoidance);
    assert_eq!(stage1.collision_probability, 0.4);
    assert_eq!(stage1.debris_radius_m, 25.0);

    let stage2 = scheduler.get_stage(350_000);
    assert_eq!(stage2.name, CurriculumStageName::Intermediate);
    assert_eq!(stage2.collision_probability, 0.6);
    assert_eq!(stage2.debris_radius_m, 50.0);

    let stage3 = scheduler.get_stage(750_000);
    assert_eq!(stage3.name, CurriculumStageName::Advanced);
    assert_eq!(stage3.collision_probability, 1.0);
    assert_eq!(stage3.debris_radius_m, 100.0);
}

#[test]
fn test_pretrained_policy_inference() {
    let model = load_default_pretrained_model().expect("Pre-trained weights must load");
    let mut env = SatelliteAvoidanceEnv::new(AvoidanceConfig::default());
    let obs = env.reset(12345);

    let action = model.forward_actor_deterministic(&obs);
    assert_eq!(action.len(), ACTION_DIM);

    for &a in &action {
        assert!(a.is_finite());
        assert!((-1.0..=1.0).contains(&a), "Action {} out of [-1, 1] range", a);
    }

    let value = model.forward_critic(&obs);
    assert!(value.is_finite());
}

#[test]
fn test_model_serialization_roundtrip() {
    let original = load_default_pretrained_model().expect("Pre-trained weights must load");
    let temp_path = std::env::temp_dir().join("sbm_avoidance_test_model.bin");
    let temp_path_str = temp_path.to_str().unwrap();

    save_model_checkpoint(&original, temp_path_str).expect("Must save model checkpoint");
    let loaded = load_model_checkpoint(temp_path_str).expect("Must load model checkpoint");

    let obs = [0.123f32; OBSERVATION_DIM];
    let act_orig = original.forward_actor_deterministic(&obs);
    let act_load = loaded.forward_actor_deterministic(&obs);

    assert_eq!(act_orig, act_load, "Loaded model actions must match exactly");
    assert_eq!(original.forward_critic(&obs), loaded.forward_critic(&obs));

    let _ = std::fs::remove_file(temp_path);
}

#[test]
fn test_environment_reset_and_step() {
    let config = AvoidanceConfig::default();
    let mut env = SatelliteAvoidanceEnv::new(config);

    let obs = env.reset(12345);
    assert_eq!(obs.len(), OBSERVATION_DIM);

    let action = Action3D::new(0.5, -0.5, 0.0);
    let (next_obs, reward, done, info) = env.step(action);

    assert_eq!(next_obs.len(), OBSERVATION_DIM);
    assert!(reward.is_finite());
    assert!(!done, "First step should not terminate");
    assert!(info.step_delta_v > 0.0);
    assert!(info.remaining_fuel_kg < 500.0);
}

#[test]
fn test_ppo_training_iteration() {
    let config = AvoidanceConfig {
        reward: RewardCoefficients::training(),
        ..Default::default()
    };
    let mut env = SatelliteAvoidanceEnv::new(config);
    let params = PpoHyperparameters {
        n_steps: 100, // Small rollout for fast unit test
        n_epochs: 2,
        batch_size: 32,
        ..Default::default()
    };

    let mut trainer = PpoTrainer::new(42, params);
    let mut rng = SimpleRng::new(42);

    let mean_reward = trainer.train_iteration(&mut env, &mut rng);
    assert!(mean_reward.is_finite());
    assert_eq!(trainer.global_step, 100);
}

#[test]
fn test_record_episode_snapshots() {
    let model = load_default_pretrained_model().expect("Pre-trained model must load");
    let config = AvoidanceConfig {
        reward: RewardCoefficients::evaluation(),
        ..Default::default()
    };
    let seed = 12345u64;

    let record = record_ppo_episode(&model, &config, seed, 200);

    assert_eq!(record.policy_name, "PPO");
    assert_eq!(record.seed, seed);
    assert!(!record.snapshots.is_empty(), "Snapshots must not be empty");
    assert!(record.snapshots.len() <= 201);

    for s in &record.snapshots {
        assert!(s.sat_pos[0].is_finite());
        assert!(s.sat_pos[1].is_finite());
        assert!(s.sat_pos[2].is_finite());
        assert!(s.sat_fuel_kg >= 0.0 && s.sat_fuel_kg <= 500.0);
        assert!(s.cumulative_dv >= 0.0);
        assert!(s.min_distance_m >= 0.0);
    }
}

#[test]
fn test_comparative_overlay_seed_consistency() {
    let model = load_default_pretrained_model().expect("Pre-trained model must load");
    let config = AvoidanceConfig {
        reward: RewardCoefficients::evaluation(),
        ..Default::default()
    };
    let seed = 12345u64;

    let records = generate_comparative_telemetry(&model, &config, seed, 150);
    assert_eq!(records.len(), 4, "Must generate records for all 4 controllers");

    let ppo = &records[0];
    let imp = &records[1];
    let rule = &records[2];
    let noact = &records[3];

    assert_eq!(ppo.policy_name, "PPO");
    assert_eq!(imp.policy_name, "Impulsive");
    assert_eq!(rule.policy_name, "Rule-based");
    assert_eq!(noact.policy_name, "No-action");

    // All controllers must share identical initial debris field geometry for the same seed
    assert_eq!(ppo.initial_debris, imp.initial_debris);
    assert_eq!(ppo.initial_debris, rule.initial_debris);
    assert_eq!(ppo.initial_debris, noact.initial_debris);
    assert_eq!(ppo.target_debris_idx, imp.target_debris_idx);
}

#[test]
fn test_training_visualizer_data_generation() {
    let model = load_default_pretrained_model().expect("Pre-trained model must load");
    let config = AvoidanceConfig {
        reward: RewardCoefficients::evaluation(),
        ..Default::default()
    };

    let data = generate_training_visualizer_data(&model, &config);
    assert_eq!(data.curves.len(), 101, "Curriculum curves must have 101 points (0 to 1M steps)");
    assert_eq!(data.curves.first().unwrap().step, 0);
    assert_eq!(data.curves.last().unwrap().step, 1_000_000);

    // Collision rate must monotonically improve between stage boundaries
    assert!(data.curves[0].collision_rate_pct > data.curves[50].collision_rate_pct);
    assert!(data.curves[50].collision_rate_pct > data.curves[100].collision_rate_pct);

    assert_eq!(data.checkpoint_episodes.len(), 4, "Must provide 4 milestone checkpoint replays");
}

#[test]
fn test_real_tle_conjunction_generation() {
    let model = load_default_pretrained_model().expect("Pre-trained model must load");

    let scenario = generate_real_tle_conjunction(&model, "ISS", "COSMOS_2251");
    assert!(scenario.satellite_name.contains("ISS"));
    assert!(scenario.debris_catalog_name.contains("COSMOS"));

    // Ballistic unmaneuvered should collide or violate safety zone
    assert!(scenario.unmaneuvered_miss_distance_m <= 300.0, "Ballistic drift passes within collision threshold");
    // PPO should achieve safe clearance
    assert!(scenario.achieved_clearance_m > 300.0, "PPO maneuver clears collision sphere");
    assert!(scenario.propellant_used_kg > 0.0, "PPO burns fuel during evasive maneuver");
}

