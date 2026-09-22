//! Convenient re-exports for library consumers across all astrodynamics domains.

pub use crate::breakup::*;
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
pub use crate::rpo::{
    plan_glideslope_rbar, plan_glideslope_vbar, plan_natural_motion_circumnavigation,
    plan_two_impulse_transfer, GlideslopeApproachPlan, NmcInspectionPlan, RelativeState, RpoError,
    RpoManeuverDto, TargetOrbit, TwoImpulseTransferPlan,
};
pub use crate::scvx::{
    Cr3bpBurnSegment, Cr3bpTransferMissionConfig, Cr3bpTransferNode, Cr3bpTransferOptimizer,
    Cr3bpTransferPlan, ScvxOptions, ScvxSolution, TrajectoryNode,
};
