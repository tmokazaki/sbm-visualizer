//! Gravitational dynamics submodules, equations of motion, and spatial fields.

pub mod core;
pub mod field;
pub mod kepler;
pub mod particles;
pub mod shadow;
pub mod tidal;

pub use self::core::{compute_accelerations, compute_jerks, compute_conservation_metrics, compute_pairwise_forces};
pub use self::field::{
    compute_centric_spatial_field_grid, compute_earth_moon_barycenter,
    compute_gravitational_spheres, compute_spatial_field_grid, compute_spatial_field_point,
    is_body_relevant_to_centric,
};
pub use self::kepler::{
    compute_laplace_resonance_metrics, compute_satellite_orbital_telemetry,
    compute_trojan_libration_deg, compute_vis_viva_normalized_kinetic, compute_vis_viva_speed,
    extract_osculating_elements,
};
pub use self::particles::{compute_centric_particle_acceleration, generate_centric_test_particles};
pub use self::shadow::{compute_shadow_cone_geometry, evaluate_eclipse_state};
pub use self::tidal::compute_tidal_tensor;
