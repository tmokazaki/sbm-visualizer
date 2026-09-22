//! # NASA EVOLVE 4.0 Standard Breakup Model (SBM)
//!
//! Grounded in the peer-reviewed specification:
//! > **Johnson, N. L., Krisko, P. H., Liou, J.-C., & Anz-Meador, P. D. (2001).**  
//! > *NASA's new breakup model of EVOLVE 4.0.*  
//! > Advances in Space Research, 28(9), pp. 1377–1387.

pub mod engine;
pub mod error;
pub mod math;
pub mod rng;
pub mod sampling;
pub mod types;

// Re-export core types for ergonomic usage within the breakup module
pub use engine::{BreakupEngine, BreakupEngineBuilder, SimpleBreakupEngine};
pub use error::BreakupError;
pub use math::{
    ballistic_coefficient, collision_destroyed_mass, cross_sectional_area, cumulative_fragment_count,
    is_catastrophic_collision, specific_impact_energy, CATASTROPHIC_THRESHOLD_KJ_PER_KG,
    DEFAULT_DRAG_COEFFICIENT,
};
pub use rng::{RngSource, SimpleRng};
pub use sampling::{
    sample_am_ratio, sample_delta_v, sample_direction_cone, sample_direction_isotropic,
    sample_gaussian_mixture,
};
pub use types::{BreakupResult, BreakupType, Fragment, ObjectType};
