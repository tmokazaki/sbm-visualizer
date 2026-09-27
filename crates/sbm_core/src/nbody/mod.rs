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
    compute_accelerations, compute_centric_particle_acceleration, compute_centric_spatial_field_grid,
    compute_conservation_metrics, compute_earth_moon_barycenter, compute_gravitational_spheres,
    compute_jerks, compute_laplace_resonance_metrics, compute_pairwise_forces,
    compute_shadow_cone_geometry, compute_spatial_field_grid, compute_spatial_field_point,
    compute_tidal_tensor, compute_trojan_libration_deg, compute_vis_viva_normalized_kinetic,
    compute_vis_viva_speed, evaluate_eclipse_state, extract_osculating_elements,
    generate_centric_test_particles, is_body_relevant_to_centric,
};
pub use integrator::{
    propagate_trajectory, step_dormand_prince853, step_hermite4, step_leapfrog, step_system,
    step_yoshida4, step_yoshida6, TrajectorySnapshot,
};
pub use presets::{create_inner_solar_system_jupiter, create_preset, PresetId};
pub use types::{
    BodyFieldContribution, CelestialBody, CentricTestParticle, ConservationMetrics,
    EclipseState, GravitationalCentricFrame, GravitationalSphereRadii, IntegratorType, NBodySystem,
    OsculatingElements, PairwiseForce, ResonanceMetrics, ShadowConeGeometry, SpatialFieldPoint,
    TidalTensor, ASTRONOMICAL_UNIT_M, G_STANDARD, JULIAN_DAY_S, JULIAN_YEAR_S, SPEED_OF_LIGHT,
};

