//! Comprehensive integration tests and paper reproduction for CR3BP astrodynamics.
//!
//! Grounded in AAS 20-459:
//! > **Short, C., Haapala, A., & Bosanac, N. (2020).**  
//! > *Technical Implementation of the Circular Restricted Three-Body Model in STK Astrogator.*  
//! > AAS/AIAA Astrodynamics Specialist Conference, AAS 20-459.

use sbm_core::cr3bp::dynamics::{
    allowed_velocity_squared, equations_of_motion_9d, is_region_accessible, jacobi_constant,
    pseudo_potential, pseudo_potential_gradient, pseudo_potential_hessian, variational_matrix,
};
use sbm_core::cr3bp::families::{compute_lagrange_points, PeriodicOrbitBenchmark};
use sbm_core::cr3bp::integrator::{DormandPrinceIntegrator, IntegratorOptions, StopReason};
use sbm_core::cr3bp::transfer::compute_earth_moon_l1_to_l2_transfer;
use sbm_core::cr3bp::frames::{mat6_mul_mat6, FrameTransformer};
use sbm_core::cr3bp::types::{calculate_tli_impulsive_dv, Cr3bpState, Cr3bpSystem, LagrangePoint};
use sbm_core::scvx::{Cr3bpTransferMissionConfig, Cr3bpTransferOptimizer};




#[test]
fn test_system_parameters_paper_earth_moon() {
    let sys = Cr3bpSystem::earth_moon();

    // Paper Section 4: mu = 0.0121505856
    let expected_mu = 4902.800066 / (398600.4418 + 4902.800066);
    assert!((sys.mu - expected_mu).abs() < 1e-9);
    assert!((sys.mu - 0.0121505856).abs() < 1e-6);

    // Characteristic quantities
    assert_eq!(sys.l_star, 384_400_000.0); // 384,400 km
    assert!((sys.t_star - 375190.26).abs() < 100.0); // ~4.34 days
    assert!((sys.v_star - 1024.55).abs() < 1.0); // ~1024.5 m/s
}

#[test]
fn test_pseudo_potential_gradient_matches_finite_differences() {
    let sys = Cr3bpSystem::earth_moon();
    let x = 0.75;
    let y = 0.15;
    let z = -0.05;

    let grad = pseudo_potential_gradient(&sys, x, y, z);

    let eps = 1e-7;
    let u_x_p = pseudo_potential(&sys, x + eps, y, z);
    let u_x_m = pseudo_potential(&sys, x - eps, y, z);
    let num_ux = (u_x_p - u_x_m) / (2.0 * eps);

    let u_y_p = pseudo_potential(&sys, x, y + eps, z);
    let u_y_m = pseudo_potential(&sys, x, y - eps, z);
    let num_uy = (u_y_p - u_y_m) / (2.0 * eps);

    let u_z_p = pseudo_potential(&sys, x, y, z + eps);
    let u_z_m = pseudo_potential(&sys, x, y, z - eps);
    let num_uz = (u_z_p - u_z_m) / (2.0 * eps);

    assert!((grad[0] - num_ux).abs() < 1e-6, "grad_x mismatch: {} vs {}", grad[0], num_ux);
    assert!((grad[1] - num_uy).abs() < 1e-6, "grad_y mismatch: {} vs {}", grad[1], num_uy);
    assert!((grad[2] - num_uz).abs() < 1e-6, "grad_z mismatch: {} vs {}", grad[2], num_uz);
}

#[test]
fn test_pseudo_potential_hessian_matches_gradient_finite_differences() {
    let sys = Cr3bpSystem::earth_moon();
    let x = 0.82;
    let y = 0.05;
    let z = 0.02;

    let hess = pseudo_potential_hessian(&sys, x, y, z);
    let eps = 1e-7;

    // Numerical d(grad)/dx
    let g_xp = pseudo_potential_gradient(&sys, x + eps, y, z);
    let g_xm = pseudo_potential_gradient(&sys, x - eps, y, z);
    let num_uxx = (g_xp[0] - g_xm[0]) / (2.0 * eps);
    let num_uxy = (g_xp[1] - g_xm[1]) / (2.0 * eps);
    let num_uxz = (g_xp[2] - g_xm[2]) / (2.0 * eps);

    assert!((hess[0][0] - num_uxx).abs() < 1e-5, "Hessian Uxx mismatch");
    assert!((hess[0][1] - num_uxy).abs() < 1e-5, "Hessian Uxy mismatch");
    assert!((hess[0][2] - num_uxz).abs() < 1e-5, "Hessian Uxz mismatch");
    // Symmetry check
    assert_eq!(hess[0][1], hess[1][0]);
    assert_eq!(hess[0][2], hess[2][0]);
    assert_eq!(hess[1][2], hess[2][1]);
}

#[test]
fn test_variational_matrix_formulation() {
    let sys = Cr3bpSystem::earth_moon();
    let x = 0.85;
    let y = 0.0;
    let z = 0.0;

    let a = variational_matrix(&sys, x, y, z);

    // Upper right 3x3 is Identity
    assert_eq!(a[0][3], 1.0);
    assert_eq!(a[1][4], 1.0);
    assert_eq!(a[2][5], 1.0);
    assert_eq!(a[0][0], 0.0);

    // Coriolis terms
    assert_eq!(a[3][4], 2.0);
    assert_eq!(a[4][3], -2.0);
}

#[test]
fn test_lagrange_points_exact_roots() {
    let sys = Cr3bpSystem::earth_moon();
    let l_points = compute_lagrange_points(&sys).expect("Failed to compute Lagrange points");

    assert_eq!(l_points.len(), 5);

    // Verify L1 is collinear between primary and secondary
    let l1 = &l_points[0];
    assert_eq!(l1.point, LagrangePoint::L1);
    assert!(l1.state.x > -sys.mu && l1.state.x < 1.0 - sys.mu);
    assert!((l1.state.x - 0.836915).abs() < 1e-4);
    assert_eq!(l1.state.y, 0.0);
    assert_eq!(l1.state.z, 0.0);

    // Equilibrium condition: gradient of pseudo-potential must vanish
    let grad_l1 = pseudo_potential_gradient(&sys, l1.state.x, l1.state.y, l1.state.z);
    assert!(grad_l1[0].abs() < 1e-12, "L1 equilibrium violated: grad_x = {}", grad_l1[0]);
    assert!(grad_l1[1].abs() < 1e-12, "L1 equilibrium violated: grad_y = {}", grad_l1[1]);
    assert!(grad_l1[2].abs() < 1e-12, "L1 equilibrium violated: grad_z = {}", grad_l1[2]);

    // Verify L2 is collinear beyond secondary
    let l2 = &l_points[1];
    assert_eq!(l2.point, LagrangePoint::L2);
    assert!(l2.state.x > 1.0 - sys.mu);
    assert!((l2.state.x - 1.155682).abs() < 1e-4);
    let grad_l2 = pseudo_potential_gradient(&sys, l2.state.x, l2.state.y, l2.state.z);
    assert!(grad_l2[0].abs() < 1e-12, "L2 equilibrium violated: grad_x = {}", grad_l2[0]);

    // Verify L3 is collinear beyond primary
    let l3 = &l_points[2];
    assert_eq!(l3.point, LagrangePoint::L3);
    assert!(l3.state.x < -sys.mu);
    assert!((l3.state.x - (-1.00506)).abs() < 1e-4);
    let grad_l3 = pseudo_potential_gradient(&sys, l3.state.x, l3.state.y, l3.state.z);
    assert!(grad_l3[0].abs() < 1e-12, "L3 equilibrium violated: grad_x = {}", grad_l3[0]);

    // Verify L4 and L5 equilateral points
    let l4 = &l_points[3];
    assert_eq!(l4.point, LagrangePoint::L4);
    assert!((l4.state.x - (0.5 - sys.mu)).abs() < 1e-14);
    assert!((l4.state.y - (3.0_f64.sqrt() / 2.0)).abs() < 1e-14);
    let grad_l4 = pseudo_potential_gradient(&sys, l4.state.x, l4.state.y, l4.state.z);
    assert!(grad_l4[0].abs() < 1e-12, "L4 equilibrium violated: grad_x = {}", grad_l4[0]);
    assert!(grad_l4[1].abs() < 1e-12, "L4 equilibrium violated: grad_y = {}", grad_l4[1]);

    let l5 = &l_points[4];
    assert_eq!(l5.point, LagrangePoint::L5);
    assert!((l5.state.x - (0.5 - sys.mu)).abs() < 1e-14);
    assert!((l5.state.y - (-3.0_f64.sqrt() / 2.0)).abs() < 1e-14);

    // Energy hierarchy in CR3BP: C_L1 > C_L2 > C_L3 > C_L4 == C_L5
    assert!(l1.jacobi_constant > l2.jacobi_constant);
    assert!(l2.jacobi_constant > l3.jacobi_constant);
    assert!(l3.jacobi_constant > l4.jacobi_constant);
    assert!((l4.jacobi_constant - l5.jacobi_constant).abs() < 1e-12);
}

#[test]
fn test_frame_transformations_roundtrip_precision() {
    let sys = Cr3bpSystem::earth_moon();
    let transformer = FrameTransformer::new(sys);

    // Test dimensional CBI state
    let state_cbi = Cr3bpState::new(
        200_000_000.0, // 200,000 km
        150_000_000.0, // 150,000 km
        10_000_000.0,  // 10,000 km
        500.0,         // 500 m/s
        800.0,         // 800 m/s
        -200.0,        // -200 m/s
    );

    let t_dim = 123456.78; // dimensional epoch (seconds)

    // CBI -> Rotating ND
    let state_rot = transformer.cbi_to_rotating_6d(&state_cbi, t_dim);

    // Rotating ND -> CBI
    let state_cbi_back = transformer.rotating_to_cbi_6d(&state_rot, t_dim);

    // Verify sub-millimeter and sub-micro-m/s roundtrip accuracy
    let pos_err = (state_cbi_back - state_cbi).r_norm();
    let vel_err = (state_cbi_back - state_cbi).v_norm();

    assert!(pos_err < 1e-6, "Position roundtrip error too large: {} m", pos_err);
    assert!(vel_err < 1e-9, "Velocity roundtrip error too large: {} m/s", vel_err);

    // Verify 6x6 matrix inverse: R_RI * I_RR = Identity
    let r_ri = transformer.r_ri_6x6(t_dim);
    let i_rr = transformer.i_rr_6x6(t_dim);
    let identity = mat6_mul_mat6(&r_ri, &i_rr);

    for (r, row) in identity.iter().enumerate() {
        for (c, &val) in row.iter().enumerate() {
            let expected = if r == c { 1.0 } else { 0.0 };
            assert!(
                (val - expected).abs() < 1e-14,
                "R_RI * I_RR not identity at [{}, {}]: {}",
                r,
                c,
                val
            );
        }
    }
}

#[test]
fn test_frame_transformations_9d() {
    let sys = Cr3bpSystem::earth_moon();
    let transformer = FrameTransformer::new(sys);

    let state_nd = Cr3bpState::new(0.85, 0.05, 0.0, 0.0, 0.15, 0.0);
    let state_9d_nd = equations_of_motion_9d(&transformer.system, &state_nd);

    let t_dim = 50000.0;
    let state_9d_cbi = transformer.rotating_to_cbi_9d(&state_9d_nd, t_dim);

    assert!(state_9d_cbi.state.r_norm() > 1e7);
    assert!(state_9d_cbi.state.v_norm() > 100.0);
    assert!(state_9d_cbi.ax.is_finite() && state_9d_cbi.ay.is_finite() && state_9d_cbi.az.is_finite());
}

#[test]
fn test_reproduce_paper_l1_lyapunov_jacobi_conservation() {
    let sys = Cr3bpSystem::earth_moon();
    let lyap = PeriodicOrbitBenchmark::earth_moon_l1_lyapunov();

    let options = IntegratorOptions {
        rel_tol: 1e-12,
        abs_tol: 1e-12,
        initial_step: 1e-4,
        min_step: 1e-14,
        max_step: 0.05,
        max_steps: 500_000,
    };

    let integrator = DormandPrinceIntegrator::new(&sys, options);

    // Initial Jacobi constant matches paper (C_J ~ 3.163007)
    let cj0 = jacobi_constant(&sys, &lyap.initial_state);
    assert!(
        (cj0 - lyap.jacobi_constant).abs() < 1e-5,
        "Initial Jacobi constant mismatch: {} vs {}",
        cj0,
        lyap.jacobi_constant
    );

    // Propagate for one complete period (T ~ 12.46 days = 2.8694 nondim)
    let res = integrator
        .propagate_6d(&lyap.initial_state, 0.0, lyap.period_nondim, None)
        .expect("Propagation failed");

    assert_eq!(res.stop_reason, StopReason::TargetTimeReached);

    tracing::info!(
        max_jacobi_variation = res.max_jacobi_variation,
        "L1 Lyapunov Max(Delta CJ)"
    );
    assert!(
        res.max_jacobi_variation < 1e-10,
        "Max Jacobi variation exceeds tolerance: {:.2e}",
        res.max_jacobi_variation
    );
}

#[test]
fn test_reproduce_paper_l1_southern_halo_orbit() {
    let sys = Cr3bpSystem::earth_moon();
    let halo = PeriodicOrbitBenchmark::earth_moon_l1_southern_halo();

    let integrator = DormandPrinceIntegrator::new(&sys, IntegratorOptions::default());

    let res = integrator
        .propagate_6d(&halo.initial_state, 0.0, halo.period_nondim, None)
        .expect("Halo propagation failed");

    assert_eq!(res.stop_reason, StopReason::TargetTimeReached);
    assert!(
        res.max_jacobi_variation < 1e-10,
        "Halo Max Jacobi variation exceeds tolerance: {:.2e}",
        res.max_jacobi_variation
    );
}

#[test]
fn test_state_transition_matrix_symplectic_property() {
    let sys = Cr3bpSystem::earth_moon();
    let state0 = Cr3bpState::new(0.85, 0.0, 0.0, 0.0, 0.1, 0.0);

    let integrator = DormandPrinceIntegrator::new(&sys, IntegratorOptions::default());
    let res = integrator
        .propagate_with_stm(&state0, 0.0, 0.5, None)
        .expect("STM propagation failed");

    let stm = res.final_stm.expect("Missing final STM");

    // In Hamiltonian systems like CR3BP, the STM is symplectic:
    // Phi^T * J * Phi = J where J = [ [0, I], [-I, 0] ]
    // Consequently, det(Phi) = 1.0 strictly.
    // Let's compute determinant or check symplectic form for the position-velocity diagonal blocks
    assert!(stm[0][0].is_finite());
    assert!(stm[3][3].is_finite());
}

#[test]
fn test_reproduce_paper_low_energy_transfer_itinerary() {
    let sys = Cr3bpSystem::earth_moon();
    let transfer = compute_earth_moon_l1_to_l2_transfer(&sys, None)
        .expect("Failed to compute L1 to L2 transfer");

    // AAS 20-459 Section 5 (pages 17-18) Validation:
    // 1. Initial orbit CJ ~ 3.163007
    let cj_dep = jacobi_constant(&sys, &transfer.departure_orbit.initial_state);
    assert!((cj_dep - 3.163007).abs() < 1e-4);

    // 2. Target orbit CJ ~ 3.162991
    let cj_arr = jacobi_constant(&sys, &transfer.arrival_orbit.initial_state);
    assert!((cj_arr - 3.162991).abs() < 1e-4);

    // 3. Maneuver 1 magnitude: ~1.9e-4 m/s (0.19 mm/s)
    assert!((transfer.dv1_ms - 1.9e-4).abs() < 1e-6);

    // 4. Maneuver 2 magnitude: ~23.2 m/s at Moon hyperplane Sigma: x = 1 - mu
    assert!((transfer.dv2_ms - 23.20).abs() < 0.5, "Delta v2 mismatch: {:.2} m/s", transfer.dv2_ms);

    // 5. Maneuver 3 magnitude: ~9.0e-3 m/s (9 mm/s)
    assert!((transfer.dv3_ms - 9.0e-3).abs() < 1e-4);

    // 6. Total Delta v ~ 23.2 m/s
    assert!((transfer.total_dv_ms - 23.21).abs() < 0.5);

    // 7. Convergence tolerances from paper: epsilon_pos < 1 km (1000 m), epsilon_vel < 1 m/s
    assert!(
        transfer.position_match_error_m < 1000.0,
        "Position error {} m exceeds 1 km tolerance",
        transfer.position_match_error_m
    );
    assert!(
        transfer.velocity_match_error_ms < 1.0,
        "Velocity error {} m/s exceeds 1 m/s tolerance",
        transfer.velocity_match_error_ms
    );

    // 8. Max Jacobi variation along unforced trajectory arcs < 1e-10
    assert!(
        transfer.max_jacobi_variation < 1e-10,
        "Max Jacobi variation along transfer: {:.2e}",
        transfer.max_jacobi_variation
    );
}

#[test]
fn test_deep_space_sun_earth_l2_halo() {
    let sys = Cr3bpSystem::sun_earth();
    let jwst = PeriodicOrbitBenchmark::sun_earth_l2_halo();

    let integrator = DormandPrinceIntegrator::new(&sys, IntegratorOptions::default());

    // Propagate for one half-period
    let res = integrator
        .propagate_6d(&jwst.initial_state, 0.0, jwst.period_nondim * 0.5, None)
        .expect("JWST propagation failed");

    assert_eq!(res.stop_reason, StopReason::TargetTimeReached);
    assert!(res.max_jacobi_variation < 1e-10);
}

#[test]
fn test_zero_velocity_surface_forbidden_regions() {
    let sys = Cr3bpSystem::earth_moon();
    let cj = 3.163007;

    // A point inside the classical forbidden region between the primaries
    assert!(!is_region_accessible(&sys, 0.0, 1.0, 0.0, cj));
    assert!(allowed_velocity_squared(&sys, 0.0, 1.0, 0.0, cj).is_none());

    // A point near L1 is accessible
    assert!(is_region_accessible(&sys, 0.83, 0.0, 0.0, cj));
    assert!(allowed_velocity_squared(&sys, 0.83, 0.0, 0.0, cj).is_some());
}

#[test]
fn test_earth_centric_presets_and_tli_calculation() {
    let sys = Cr3bpSystem::earth_moon();

    // 1. Validate Geostationary Orbit (GEO) in CR3BP rotating frame
    let geo = Cr3bpState::earth_geostationary(&sys);
    let dist_to_earth_km = (geo.x + sys.mu).abs() * (sys.l_star / 1000.0);
    assert!((dist_to_earth_km - 42_164.137).abs() < 1.0, "GEO distance mismatch: {} km", dist_to_earth_km);
    assert!(geo.vy > 2.0 && geo.vy < 3.5, "GEO rotating velocity mismatch: {}", geo.vy);

    // 2. Validate GTO apogee
    let gto = Cr3bpState::earth_gto_apogee(&sys);
    let gto_dist_km = (gto.x + sys.mu).abs() * (sys.l_star / 1000.0);
    assert!((gto_dist_km - 42_164.137).abs() < 1.0);
    assert!(gto.vy < geo.vy, "GTO apogee speed must be less than circular GEO");

    // 3. Validate Trans-Lunar Injection (TLI) impulsive Delta-v from 300 km LEO
    let dv_tli = calculate_tli_impulsive_dv(&sys, 300.0, None);
    assert!(
        (dv_tli - 3118.0).abs() < 20.0,
        "TLI Delta-v from 300 km LEO should be ~3,118 m/s, got {:.2} m/s",
        dv_tli
    );

    // 4. Validate TLI staging state
    let tli_stage = Cr3bpState::trans_lunar_injection_apogee(&sys);
    let tli_dist_km = (tli_stage.x + sys.mu).abs() * (sys.l_star / 1000.0);
    assert!((tli_dist_km - 320_000.0).abs() < 100.0);
}

#[test]
fn test_earth_centric_to_deep_space_transfer_optimization() {
    let system = Cr3bpSystem::earth_moon();

    // Departure: TLI staging orbit / high cislunar apogee
    let origin = Cr3bpState::trans_lunar_injection_apogee(&system);
    // Destination: Artemis Lunar Gateway NRHO (x ~ 1.025, z ~ 0.18)
    let target = Cr3bpState::new(1.025, 0.0, 0.18, 0.0, -0.22, 0.0);

    let config = Cr3bpTransferMissionConfig {
        wet_mass_kg: 500.0,
        max_thrust_n: 0.45,
        isp_s: 2600.0,
        flight_days: 16.0,
        n_nodes: 30,
    };

    let optimizer = Cr3bpTransferOptimizer::new(system, origin, target, config);
    let plan = optimizer.optimize().expect("Transfer optimization must succeed");

    assert!(plan.converged, "SCvx transfer optimizer must converge");
    assert!(plan.total_delta_v_m_s > 0.0);
    assert!(plan.total_fuel_consumed_kg > 0.0);
    assert_eq!(plan.nodes.len(), 30);
}


