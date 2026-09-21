//! # AutoOrbit: Physics-Informed Satellite Orbit Prediction Engine
//!
//! Grounded in the KDD 2026 paper:
//! > **Donghao Zhang, Siyuan Liang, Boyuan Yang, Tao Qi, Shangguang Wang, & Qing Li (2026).**  
//! > *AutoOrbit: Physics-Informed Satellite Orbit Prediction.*  
//! > In Proceedings of the 32nd ACM SIGKDD Conference on Knowledge Discovery and Data Mining (KDD '26),
//! > August 09–13, 2026, Jeju Island, Republic of Korea. ACM, New York, NY, USA, 12 pages.
//! > DOI: [10.1145/3770855.3818960](https://doi.org/10.1145/3770855.3818960)
//!
//! AutoOrbit addresses the critical limitations of both physics-based (HPOP, SGP4) and purely
//! data-driven (LSTM, Informer) models onboard satellites by introducing a **3-level hierarchical decomposition**:
//!
//! 1. **Global Orbital Structure (Component A)**:
//!    Reconstructs a stable reference orbit $s_{ref}(t)$ via phase-averaging across the ground-track recurrence period
//!    $T_{rec}$ (Eq. 1). The model predicts small residual deviations $r_{res}(t) = s_{obs}(t) - s_{ref}(t)$ (Eqs. 2–3),
//!    preventing long-horizon divergence under noisy GNSS inputs.
//!
//! 2. **Local Orbital Dynamics (Components B & C)**:
//!    Employs a 1D Fourier Neural Operator (FNO1d) with low-frequency mode truncation ($k_{max}$) to capture
//!    periodic spatiotemporal dynamics. Incorporates an acceleration-level physics loss (Eqs. 8–9) via a 4th-order
//!    finite difference scheme to regularize local motion against Earth gravity, $J_2\text{--}J_4$ harmonics,
//!    and atmospheric drag.
//!
//! 3. **Discrete Maneuver Events (Component D)**:
//!    Applies Gaussian Variational Equations (GVEs, Eqs. 10–12) to compute instantaneous jumps in Keplerian elements
//!    $(\Delta a, \Delta e, \Delta \omega, \Delta i, \Delta \Omega)$ from RAC impulsive velocity increments
//!    $[\Delta v_r, \Delta v_a, \Delta v_c]^T$, followed by analytical Keplerian propagation with $O(H)$ complexity.

pub mod fno;
pub mod maneuver;
pub mod physics;
pub mod predictor;
pub mod reference_orbit;
pub mod types;

// Re-export primary types and functions for ergonomic library use
pub use fno::{adaptive_avg_pool1d, Activation, AutoregressiveFnoModel, Fno1dLayer, LinearWeights, SpectralWeights};
pub use maneuver::{
    apply_maneuver_correction, cartesian_to_keplerian, compute_gve_element_deltas,
    keplerian_to_cartesian, solve_keplers_equation,
};
pub use physics::{
    compute_predicted_acceleration_4th_order, compute_theoretical_acceleration,
    evaluate_trajectory_physics_consistency, SpacecraftPhysicalParams,
};
pub use predictor::AutoOrbitPredictor;
pub use reference_orbit::ReferenceOrbit;
pub use types::{
    AutoOrbitError, KeplerianElements, ManeuverImpulse, NormalizationStats, PredictionHorizon,
    PredictionMetrics, StateVector, EARTH_MU, EARTH_RADIUS_M, J2, J3, J4,
};
