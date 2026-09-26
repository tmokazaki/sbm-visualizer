//! High-Precision N-Body Gravitational Dynamics and Astronomical Mechanics.
//!
//! Provides symplectic integrators (Yoshida 4th, Yoshida 6th, Leapfrog), 1PN General
//! Relativistic corrections, Hamiltonian conservation tracking, osculating Keplerian
//! element extraction, and canonical astronomical systems (JPL Solar System, Jovian
//! Laplace resonance, Figure-8 choreography, Sun-Jupiter Trojans, Pythagorean problem).

pub mod dynamics;
pub mod integrator;
pub mod presets;
pub mod types;

pub use dynamics::{
    compute_accelerations, compute_conservation_metrics, compute_jerks,
    compute_laplace_resonance_metrics, compute_trojan_libration_deg, extract_osculating_elements,
};
pub use integrator::{
    propagate_trajectory, step_dormand_prince853, step_hermite4, step_leapfrog, step_system,
    step_yoshida4, step_yoshida6, TrajectorySnapshot,
};
pub use presets::{create_preset, PresetId};
pub use types::{
    CelestialBody, ConservationMetrics, IntegratorType, NBodySystem, OsculatingElements,
    ResonanceMetrics, ASTRONOMICAL_UNIT_M, G_STANDARD, JULIAN_DAY_S, JULIAN_YEAR_S,
    SPEED_OF_LIGHT,
};
