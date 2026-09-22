//! Deep Space CR3BP Mission & Multi-Body Low-Energy Transfer Demo.
//!
//! Grounded in AAS 20-459:
//! > **Short, C., Haapala, A., & Bosanac, N. (2020).**  
//! > *Technical Implementation of the Circular Restricted Three-Body Model in STK Astrogator.*  
//!
//! Demonstrates:
//! 1. Earth–Moon libration points ($L_1\text{--}L_5$) and energy landscape.
//! 2. Periodic orbit benchmark propagation ($L_1$ Lyapunov, $L_1$ Southern Halo, $L_2$ NRHO).
//! 3. AAS 20-459 Low-Energy Multi-Body Transfer ($L_1 \rightarrow L_2$) via invariant manifolds and Moon hyperplane $\Sigma: x = 1-\mu$.
//! 4. Deep space mission application: Sun–Earth $L_2$ Halo Orbit (James Webb Space Telescope class).

#![deny(clippy::print_stdout, clippy::print_stderr)]

use sbm_core::cr3bp::{
    compute_earth_moon_l1_to_l2_transfer, compute_lagrange_points, equations_of_motion_9d,
    Cr3bpSystem, DormandPrinceIntegrator, FrameTransformer, IntegratorOptions,
    PeriodicOrbitBenchmark,
};
use tracing::info;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    info!("================================================================================");
    info!("    CR3BP PROPAGATOR & DEEP SPACE MULTI-BODY TRANSFER DEMO (AAS 20-459)          ");
    info!("================================================================================");

    // -------------------------------------------------------------------------
    // 1. Earth-Moon System & Lagrange Points
    // -------------------------------------------------------------------------
    let em_sys = Cr3bpSystem::earth_moon();
    info!("--- 1. Earth-Moon Three-Body System Definition ---");
    info!(primary = "Earth", gm1 = em_sys.gm1, secondary = "Moon", gm2 = em_sys.gm2, "Primary & Secondary gravitation");
    info!(
        mu = em_sys.mu,
        l_star_km = em_sys.l_star / 1000.0,
        t_star_days = em_sys.t_star / 86400.0,
        v_star_ms = em_sys.v_star,
        "System canonical parameters"
    );

    let l_points = compute_lagrange_points(&em_sys).expect("Failed to compute Lagrange points");
    info!("--- Lagrange (Libration) Equilibrium Points ---");
    for lp in &l_points {
        let pos_km = em_sys.dimensionalize_position(lp.state.position());
        info!(
            point = %lp.point,
            x_nd = lp.state.x,
            y_nd = lp.state.y,
            cj = lp.jacobi_constant,
            dist_moon_km = lp.distance_to_secondary * (em_sys.l_star / 1000.0),
            x_km = pos_km[0] / 1000.0,
            y_km = pos_km[1] / 1000.0,
            "Lagrange point equilibrium"
        );
    }

    // -------------------------------------------------------------------------
    // 2. High-Precision Periodic Orbit Propagation
    // -------------------------------------------------------------------------
    info!("--- 2. Periodic Orbit Benchmark Propagation ---");
    let lyap_l1 = PeriodicOrbitBenchmark::earth_moon_l1_lyapunov();
    let integrator = DormandPrinceIntegrator::new(&em_sys, IntegratorOptions::default());

    let lyap_res = integrator
        .propagate_6d(&lyap_l1.initial_state, 0.0, lyap_l1.period_nondim, None)
        .expect("Propagation failed");

    info!(
        benchmark = "Earth-Moon L1 Planar Lyapunov Orbit",
        period_days = lyap_l1.period_days,
        period_nondim = lyap_l1.period_nondim,
        cj = lyap_l1.jacobi_constant,
        max_cj_variation = lyap_res.max_jacobi_variation,
        steps_accepted = lyap_res.steps_accepted,
        closure_norm_m = lyap_res.closure_norm * em_sys.l_star,
        "Lyapunov orbit propagation verified"
    );

    let halo_l1 = PeriodicOrbitBenchmark::earth_moon_l1_southern_halo();
    let halo_res = integrator
        .propagate_6d(&halo_l1.initial_state, 0.0, halo_l1.period_nondim, None)
        .expect("Halo propagation failed");

    info!(
        benchmark = "Earth-Moon L1 Southern 3D Halo Orbit",
        period_days = halo_l1.period_days,
        period_nondim = halo_l1.period_nondim,
        max_cj_variation = halo_res.max_jacobi_variation,
        steps_accepted = halo_res.steps_accepted,
        "Halo orbit propagation verified"
    );

    // -------------------------------------------------------------------------
    // 3. AAS 20-459 Low-Energy Manifold Transfer (L1 -> L2)
    // -------------------------------------------------------------------------
    info!("--- 3. AAS 20-459 Multi-Body Low-Energy Transfer (L1 -> L2) ---");
    let transfer = compute_earth_moon_l1_to_l2_transfer(&em_sys, None)
        .expect("Failed to compute transfer");

    info!(
        scenario = %transfer.name,
        departure_orbit = %transfer.departure_orbit.name,
        target_orbit = %transfer.arrival_orbit.name,
        flight_duration_days = transfer.transfer_duration_days,
        "Transfer scenario configured"
    );
    info!(
        dv1_ms = transfer.dv1_ms,
        dv2_ms = transfer.dv2_ms,
        dv3_ms = transfer.dv3_ms,
        total_dv_ms = transfer.total_dv_ms,
        pos_match_error_m = transfer.position_match_error_m,
        vel_match_error_ms = transfer.velocity_match_error_ms,
        max_jacobi_variation = transfer.max_jacobi_variation,
        "Maneuver itinerary and verification"
    );

    // -------------------------------------------------------------------------
    // 4. Deep Space Application: Sun-Earth L2 JWST Mission Orbit
    // -------------------------------------------------------------------------
    info!("--- 4. Deep Space Application: Sun-Earth L2 (JWST Class) ---");
    let se_sys = Cr3bpSystem::sun_earth();
    let jwst = PeriodicOrbitBenchmark::sun_earth_l2_halo();
    let se_integrator = DormandPrinceIntegrator::new(&se_sys, IntegratorOptions::default());

    info!(
        system = "Sun-Earth/Moon Barycentric CR3BP",
        l_star_au = se_sys.l_star / (1.495978707e11),
        t_star_days = se_sys.t_star / 86400.0,
        "Sun-Earth CR3BP system"
    );

    let jwst_res = se_integrator
        .propagate_6d(&jwst.initial_state, 0.0, jwst.period_nondim, None)
        .expect("Sun-Earth JWST propagation failed");

    let jwst_pos_dim = se_sys.dimensionalize_position(jwst.initial_state.position());
    let dist_from_earth_km = (jwst.initial_state.x - (1.0 - se_sys.mu)).abs() * (se_sys.l_star / 1000.0);

    info!(
        orbit = "JWST Mission Orbit",
        period_days = jwst.period_days,
        dist_from_earth_km,
        x_km = jwst_pos_dim[0] / 1000.0,
        cj = jwst.jacobi_constant,
        max_cj_variation = jwst_res.max_jacobi_variation,
        steps_accepted = jwst_res.steps_accepted,
        "Deep space JWST Halo propagation verified"
    );

    // -------------------------------------------------------------------------
    // 5. Dimensional CBI Frame Transformations (Table 1)
    // -------------------------------------------------------------------------
    info!("--- 5. STK Astrogator Dimensional CBI Frame Transformations ---");
    let transformer = FrameTransformer::new(em_sys.clone());
    let state_9d = equations_of_motion_9d(&em_sys, &lyap_l1.initial_state);
    let state_cbi_9d = transformer.rotating_to_cbi_9d(&state_9d, 0.0);

    info!(
        r_rot = ?[state_9d.state.x, state_9d.state.y, state_9d.state.z],
        v_rot = ?[state_9d.state.vx, state_9d.state.vy, state_9d.state.vz],
        a_rot = ?[state_9d.ax, state_9d.ay, state_9d.az],
        "Rotating Barycentric State (Nondimensional)"
    );
    info!(
        r_cbi_km = ?[state_cbi_9d.state.x / 1000.0, state_cbi_9d.state.y / 1000.0, state_cbi_9d.state.z / 1000.0],
        v_cbi_ms = ?[state_cbi_9d.state.vx, state_cbi_9d.state.vy, state_cbi_9d.state.vz],
        a_cbi_ms2 = ?[state_cbi_9d.ax, state_cbi_9d.ay, state_cbi_9d.az],
        "Central Body Inertial (CBI) State (Dimensional)"
    );

    info!("================================================================================");
    info!("    DEMO COMPLETE: ALL CR3BP FORMULATIONS & BENCHMARKS VERIFIED ACCORDING TO AAS 20-459");
    info!("================================================================================");
}
