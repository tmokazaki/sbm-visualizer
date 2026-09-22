//! Local-First Native Astrodynamics Daemon & Deep Space Mission Planning API.
//!
//! Provides high-performance local REST endpoints and serves interactive 3D Web cockpits.

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub mod dto;
pub mod handlers;
pub mod routes;

// Router and application state
pub use routes::{create_app, AppState};

// Strongly-typed Data Transfer Objects
pub use dto::{
    CorrectHaloRequest, CorrectLyapunovRequest, Cr3bpBurnSegmentDto, Cr3bpTransferNodeDto,
    ExportOemRequest, ExportOemResponse, HealthResponse, LagrangePointDto, ManifoldRequest,
    ManifoldResponse, OptimizeTransferRequest, OptimizeTransferResponse, OrbitResponse,
    RpoBurnDto, RpoPlanRequest, RpoPlanResponse, SystemInfoResponse,
};

// Decoupled Endpoint Handlers
pub use handlers::{
    correct_halo_handler, correct_lyapunov_handler, export_oem_handler,
    generate_manifold_handler, get_system_info, get_transfer_benchmark, health_check,
    optimize_transfer_handler, plan_rpo_handler, resolve_orbit_state, resolve_system,
};
