//! Circular Restricted Three-Body Problem (CR3BP) Propagation, Astrodynamics & Transfer Engine.
//!
//! Grounded in AAS 20-459:
//! > **Short, C., Haapala, A., & Bosanac, N. (2020).**  
//! > *Technical Implementation of the Circular Restricted Three-Body Model in STK Astrogator.*  
//! > AAS/AIAA Astrodynamics Specialist Conference, AAS 20-459.
//!
//! Features:
//! - **Equations of Motion & Pseudo-Potential**: $U^*$, $\nabla U^*$, Hessian $U^*_{ij}$, and variational equations for State Transition Matrix (STM) propagation (Eqs. 1–3, 10).
//! - **Dimensional $\leftrightarrow$ Nondimensional Frame Transformations**: 6D and 9D transformations between STK Central Body Inertial (CBI) and Rotating Barycentric frames (Table 1, Eqs. 4–9, 11).
//! - **High-Precision Adaptive Integrator**: Dormand-Prince 5(4) with adaptive step size control, Jacobi conservation $\Delta C_J < 10^{-12}$, and root-finding event detection for Poincaré sections and hyperplanes.
//! - **Equilibrium Points & Periodic Orbit Families**: Euler quintic solver for $L_1\text{--}L_5$, Planar Lyapunov, 3D Halo, NRHO ($86\text{ km}$ perilune), and JWST deep space mission orbits.
//! - **Multi-Body Low-Energy Transfers**: Reproduction of the AAS 20-459 Section 5 3-maneuver itinerary ($\Delta v_1 \approx 0.19\text{ mm/s}$, $\Delta v_2 \approx 23.2\text{ m/s}$ at $\Sigma: x = 1-\mu$, $\Delta v_3 \approx 9\text{ mm/s}$).

pub mod dynamics;
pub mod families;
pub mod frames;
pub mod integrator;
pub mod transfer;
pub mod types;

// Re-exports for ergonomic usage
pub use dynamics::{
    allowed_velocity_squared, equations_of_motion, equations_of_motion_9d, is_region_accessible,
    jacobi_constant, pseudo_potential, pseudo_potential_gradient, pseudo_potential_hessian,
    state_and_stm_derivatives, variational_matrix,
};
pub use families::{
    compute_lagrange_points, compute_monodromy_stability, correct_3d_halo, correct_planar_lyapunov,
    generate_manifold_arc, solve_euler_quintic, CorrectedOrbit, LibrationPointInfo, ManifoldArc,
    ManifoldBranch, ManifoldOptions, ManifoldType, PeriodicOrbitBenchmark,
};
pub use frames::{
    dcm_inertial_to_rotating, dcm_rotating_to_inertial, mat3_mul_mat3, mat3_mul_vec3,
    mat6_mul_mat6, FrameTransformer,
};
pub use integrator::{
    DormandPrinceIntegrator, EventCondition, EventDirection, IntegratorOptions, PropagationResult,
    StopReason, TrajectoryPoint,
};
pub use transfer::{compute_earth_moon_l1_to_l2_transfer, MultiBodyTransferPlan};
pub use types::{calculate_tli_impulsive_dv, Cr3bpError, Cr3bpState, Cr3bpState9D, Cr3bpSystem, LagrangePoint};

