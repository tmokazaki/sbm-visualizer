//! Convenient re-exports of core avoidance framework types and controllers.

pub use crate::baselines::{
    AvoidanceController, ImpulsivePlannerController, NoActionController,
    RiskAwareRuleBasedController,
};
pub use crate::conjunction::{assess_conjunction, find_most_critical_debris};
pub use crate::curriculum::{CurriculumScheduler, CurriculumStage, CurriculumStageName};
pub use crate::dynamics::{
    apply_thrust_step, compute_total_gravitational_acceleration,
    moon_position_earth_centered, step_dynamics, sun_position_earth_centered,
};
pub use crate::env::{SatelliteAvoidanceEnv, SimpleRng, StepInfo};
pub use crate::eval::{
    evaluate_controller, evaluate_ppo_model, generate_comparative_telemetry,
    generate_real_tle_conjunction, generate_training_visualizer_data,
    record_controller_episode, record_ppo_episode, run_full_paper_benchmark,
    PolicyEvaluationSummary,
};
pub use crate::nn::{ActorCritic, LinearLayer};
pub use crate::ppo::{PpoHyperparameters, PpoTrainer, RolloutBuffer};
pub use crate::reward::calculate_step_reward;
pub use crate::serialize::{
    load_model_checkpoint, load_policy_weights_raw, save_model_checkpoint,
};
pub use crate::types::{
    Action3D, AvoidanceConfig, ConjunctionMetrics, DebrisObject, EpisodeTelemetryRecord,
    EpisodeTrajectorySnapshot, RealWorldConjunctionScenario, RewardBreakdown,
    RewardCoefficients, SatelliteState, TerminationReason, TrainingProgressPoint,
    TrainingVisualizerData, Vector3D, ACTION_DIM, DEFAULT_COLLISION_DISTANCE_M,
    DEFAULT_SAFE_BUFFER_M, EARTH_RADIUS_M, G0_STANDARD, MAX_DEBRIS_COUNT,
    MAX_THRUST_ACCELERATION, MU_EARTH, MU_MOON, MU_SUN, OBSERVATION_DIM,
    SPECIFIC_IMPULSE_S,
};
pub use crate::{load_default_pretrained_model, PRETRAINED_POLICY_WEIGHTS_BIN};

