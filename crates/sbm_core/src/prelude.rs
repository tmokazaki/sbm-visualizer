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
