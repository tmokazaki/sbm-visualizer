//! # NASA EVOLVE 4.0 Standard Breakup Model (SBM) Library
//!
//! `sbm_core` provides a pure, zero-dependency, high-throughput implementation
//! of the NASA EVOLVE 4.0 Standard Breakup Model for hypervelocity collisions and explosions
//! in Earth orbit.
//!
//! ## Mathematical Grounding
//! Based on the peer-reviewed specification:
//! > **Johnson, N. L., Krisko, P. H., Liou, J.-C., & Anz-Meador, P. D. (2001).**  
//! > *NASA's new breakup model of EVOLVE 4.0.*  
//! > Advances in Space Research, 28(9), 1377–1387.
//!
//! ## Key Features
//! - **Dual Event Regimes**: Hypervelocity collisions (catastrophic disruption & cratering) and explosive breakups.
//! - **Object-Specific Area-to-Mass ($A/M$) Distributions**: Bimodal Gaussian mixture distributions for Spacecraft and Rocket Bodies (Eqs. 5, 6, 7).
//! - **Radar Cross-Sectional Area ($A_x$) Formulation**: Continuous piecewise power-law scaling (Eqs. 8, 9).
//! - **Rigorous Mass Conservation**: Individual fragment masses derived via $M = A_x / (A/M)$ (Eq. 10) normalized to conserve exact destroyed mass.
//! - **Physically Conditioned Ejection Velocity ($\Delta v$)**: Log-normal distribution conditioned on $\chi = \log_{10}(A/M)$ (Eqs. 11, 12).
//! - **Zero External Dependencies**: Pure Rust standard library implementation, deterministic, thread-safe, and compatible with WebAssembly (`wasm32-unknown-unknown`) and embedded environments.
//!
//! ## Quick Start
//!
//! ```rust
//! use sbm_core::prelude::*;
//!
//! fn main() -> Result<(), BreakupError> {
//!     // 1. Configure the breakup scenario using the fluent builder
//!     let engine = BreakupEngine::builder()
//!         .target_mass(1000.0)             // 1000 kg satellite
//!         .projectile_mass(100.0)          // 100 kg impactor
//!         .impact_speed(10_000.0)          // 10 km/s relative speed
//!         .target_type(ObjectType::Spacecraft)
//!         .projectile_type(ObjectType::Spacecraft)
//!         .breakup_type(BreakupType::Collision)
//!         .num_fragments(1500)
//!         .build()?;
//!
//!     // 2. Run deterministic simulation with seed
//!     let result = engine.simulate(42);
//!
//!     // 3. Inspect physical outcomes via structured logging
//!     tracing::info!(
//!         catastrophic = result.is_catastrophic,
//!         destroyed_mass_kg = result.destroyed_mass_kg,
//!         yield_1cm = result.physical_yield_1cm,
//!         "Breakup simulation completed"
//!     );
//!
//!     // 4. Access individual fragments
//!     for fragment in result.top_heaviest(3) {
//!         tracing::info!(
//!             id = fragment.id,
//!             mass_kg = fragment.mass_kg,
//!             size_m = fragment.size_m,
//!             speed_mps = fragment.speed_mps,
//!             "Top fragment"
//!         );
//!     }
//!
//!     Ok(())
//! }
//! ```

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub mod autoorbit;
pub mod cr3bp;
pub mod engine;
pub mod error;
pub mod math;
pub mod prelude;
pub mod rng;
pub mod rpo;
pub mod sampling;

pub mod scvx;
pub mod types;

// Re-export common types at crate root for ergonomic usage
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
