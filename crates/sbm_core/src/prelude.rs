//! Convenient re-exports for library consumers.

pub use crate::engine::{BreakupEngine, BreakupEngineBuilder, SimpleBreakupEngine};
pub use crate::error::BreakupError;
pub use crate::math::{
    ballistic_coefficient, collision_destroyed_mass, cross_sectional_area, cumulative_fragment_count,
    is_catastrophic_collision, specific_impact_energy, CATASTROPHIC_THRESHOLD_KJ_PER_KG,
    DEFAULT_DRAG_COEFFICIENT,
};
pub use crate::rng::{RngSource, SimpleRng};
pub use crate::sampling::{
    sample_am_ratio, sample_delta_v, sample_direction_cone, sample_direction_isotropic,
    sample_gaussian_mixture,
};
pub use crate::types::{BreakupResult, BreakupType, Fragment, ObjectType};
pub use crate::autoorbit::{
    apply_maneuver_correction, cartesian_to_keplerian, compute_gve_element_deltas,
    keplerian_to_cartesian, AutoOrbitError, AutoOrbitPredictor, KeplerianElements, ManeuverImpulse,
    NormalizationStats, PredictionHorizon, PredictionMetrics, ReferenceOrbit, StateVector,
};
pub use crate::cr3bp::{
    compute_earth_moon_l1_to_l2_transfer, compute_lagrange_points, equations_of_motion,
    equations_of_motion_9d, is_region_accessible, jacobi_constant, pseudo_potential,
    pseudo_potential_gradient, pseudo_potential_hessian, Cr3bpError, Cr3bpState, Cr3bpState9D,
    Cr3bpSystem, DormandPrinceIntegrator, EventCondition, EventDirection, FrameTransformer,
    IntegratorOptions, LagrangePoint, LibrationPointInfo, MultiBodyTransferPlan,
    PeriodicOrbitBenchmark, PropagationResult, StopReason, TrajectoryPoint,
};
