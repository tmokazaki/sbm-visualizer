//! Space debris collision avoidance REST API handlers.
//!
//! Grounded in Luna et al. (2026):
//! - Deterministic encounter simulation (PPO, Impulsive, Rule-based, No-action)
//! - Multi-policy comparative overlay telemetry
//! - 1,000-trial benchmark metrics reproducing Tables 2, 3, and 4
//! - Training curriculum learning curves and milestone checkpoint replays
//! - Real-world CelesTrak TLE conjunction scenario screening and evasive maneuvers

use axum::{extract::Query, http::StatusCode, Json};
use sbm_avoidance::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Request payload for single or comparative encounter simulation.
#[derive(Debug, Clone, Deserialize)]
pub struct SimulateAvoidanceRequest {
    /// Random seed for encounter generation (default: 12345).
    pub seed: Option<u64>,
    /// Controller policy: "comparative" (all 4), "ppo", "impulsive", "rule_based", or "no_action".
    pub policy: Option<String>,
    /// Maximum simulation steps (default: 1000).
    pub max_steps: Option<usize>,
}

/// Simulation response envelope holding one or more trajectory telemetry records.
#[derive(Debug, Clone, Serialize)]
pub struct SimulateAvoidanceResponse {
    /// True if multiple policies are returned for comparative visual overlay.
    pub comparative: bool,
    /// Trajectory telemetry records.
    pub episodes: Vec<EpisodeTelemetryRecord>,
}

/// Query parameters for benchmark reproduction.
#[derive(Debug, Clone, Deserialize)]
pub struct BenchmarkQueryParams {
    /// Number of evaluation episodes (default: 100, max: 1000).
    pub episodes: Option<usize>,
    /// Base seed (default: 12345).
    pub seed: Option<u64>,
}

/// Query parameters for real-world TLE conjunction scenarios.
#[derive(Debug, Clone, Deserialize)]
pub struct RealScenarioQueryParams {
    /// Satellite preset: "ISS", "SENTINEL-1A", "STARLINK".
    pub satellite: Option<String>,
    /// Debris swarm preset: "COSMOS_2251", "FENGYUN_1C", "COSMOS_1408".
    pub debris: Option<String>,
}

/// Simulates an encounter episode and streams/returns 3D telemetry.
pub async fn simulate_avoidance_handler(
    Json(req): Json<SimulateAvoidanceRequest>,
) -> Result<Json<SimulateAvoidanceResponse>, (StatusCode, String)> {
    let seed = req.seed.unwrap_or(12345);
    let max_steps = req.max_steps.unwrap_or(1000).min(2000);
    let policy = req.policy.as_deref().unwrap_or("comparative").to_lowercase();

    let model = load_default_pretrained_model()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load pre-trained policy: {}", e)))?;

    let config = AvoidanceConfig {
        reward: RewardCoefficients::evaluation(),
        ..Default::default()
    };

    match policy.as_str() {
        "all" | "comparative" | "overlay" => {
            let episodes = generate_comparative_telemetry(&model, &config, seed, max_steps);
            Ok(Json(SimulateAvoidanceResponse {
                comparative: true,
                episodes,
            }))
        }
        "impulsive" | "impulse" => {
            let mut ctrl = ImpulsivePlannerController::default();
            let ep = record_controller_episode(&mut ctrl, "Impulsive", &config, seed, max_steps);
            Ok(Json(SimulateAvoidanceResponse {
                comparative: false,
                episodes: vec![ep],
            }))
        }
        "rule" | "rule_based" | "rule-based" => {
            let mut ctrl = RiskAwareRuleBasedController::default();
            let ep = record_controller_episode(&mut ctrl, "Rule-based", &config, seed, max_steps);
            Ok(Json(SimulateAvoidanceResponse {
                comparative: false,
                episodes: vec![ep],
            }))
        }
        "no_action" | "noaction" | "no-action" | "ballistic" => {
            let mut ctrl = NoActionController;
            let ep = record_controller_episode(&mut ctrl, "No-action", &config, seed, max_steps);
            Ok(Json(SimulateAvoidanceResponse {
                comparative: false,
                episodes: vec![ep],
            }))
        }
        _ => {
            let ep = record_ppo_episode(&model, &config, seed, max_steps);
            Ok(Json(SimulateAvoidanceResponse {
                comparative: false,
                episodes: vec![ep],
            }))
        }
    }
}

/// Runs deterministic benchmark and returns Tables 2, 3, and 4 metrics.
pub async fn benchmark_avoidance_handler(
    Query(params): Query<BenchmarkQueryParams>,
) -> Result<Json<Vec<PolicyEvaluationSummary>>, (StatusCode, String)> {
    let episodes = params.episodes.unwrap_or(100).min(1000);
    let seed = params.seed.unwrap_or(12345);

    let model = load_default_pretrained_model()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load pre-trained policy: {}", e)))?;

    let config = AvoidanceConfig {
        reward: RewardCoefficients::evaluation(),
        ..Default::default()
    };

    let summaries = run_full_paper_benchmark(&model, &config, episodes, seed);
    Ok(Json(summaries))
}

static TRAINING_DATA_CACHE: OnceLock<TrainingVisualizerData> = OnceLock::new();
static REAL_SCENARIO_CACHE: OnceLock<Mutex<HashMap<(String, String), RealWorldConjunctionScenario>>> = OnceLock::new();

/// Returns curriculum learning progress curves (0 to 1M steps) and 4 milestone checkpoint replays.
pub async fn training_data_handler() -> Result<Json<TrainingVisualizerData>, (StatusCode, String)> {
    if let Some(cached) = TRAINING_DATA_CACHE.get() {
        return Ok(Json(cached.clone()));
    }

    let model = load_default_pretrained_model()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load pre-trained policy: {}", e)))?;

    let config = AvoidanceConfig {
        reward: RewardCoefficients::evaluation(),
        ..Default::default()
    };

    let data = generate_training_visualizer_data(&model, &config);
    let _ = TRAINING_DATA_CACHE.set(data.clone());
    Ok(Json(data))
}

/// Returns a real-world TLE satellite conjunction scenario (unmaneuvered vs PPO autonomous evasive burn).
pub async fn real_scenario_handler(
    Query(params): Query<RealScenarioQueryParams>,
) -> Result<Json<RealWorldConjunctionScenario>, (StatusCode, String)> {
    let sat = params.satellite.as_deref().unwrap_or("ISS");
    let deb = params.debris.as_deref().unwrap_or("COSMOS_2251");
    let key = (sat.to_string(), deb.to_string());

    let cache_map = REAL_SCENARIO_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(lock) = cache_map.lock() {
        if let Some(cached) = lock.get(&key) {
            return Ok(Json(cached.clone()));
        }
    }

    let model = load_default_pretrained_model()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load pre-trained policy: {}", e)))?;

    let scenario = generate_real_tle_conjunction(&model, sat, deb);
    if let Ok(mut lock) = cache_map.lock() {
        lock.insert(key, scenario.clone());
    }
    Ok(Json(scenario))
}
