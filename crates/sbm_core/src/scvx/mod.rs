//! Successive Convexification (SCvx) Trajectory Optimization Engine.
//!
//! Grounded in:
//! - **Mao, Y., Szmuk, M., & Açıkmeşe, B. (2016)**.  
//!   *Successive Convexification of Non-Convex Optimal Control Problems and Its Convergence Properties.*  
//!   arXiv:1608.05133 (Section II, Algorithm 1, & Section IV Table I).
//! - **Malyuta, D., Reynolds, T. P., Szmuk, M., et al. (2022)**.  
//!   *Convex Optimization for Trajectory Generation: A Tutorial on Generating Dynamically Feasible Trajectories Reliably and Efficiently.*  
//!   IEEE Control Systems Magazine, 42(5), 40–113 (arXiv:2106.09125).
//! - **Short, Haapala, & Bosanac (2020)**.  
//!   *Implementation of CR3BP & Low-Energy Transfers in Astrogator.*  
//!   AAS 20-459.

pub mod admm;
pub mod cr3bp_transfer;
pub mod drag_benchmark;
pub mod types;

pub use admm::{solve_linear_system, KktConvexSubproblem, LuSolver, SubproblemSolution};
pub use cr3bp_transfer::{
    Cr3bpBurnSegment, Cr3bpTransferMissionConfig, Cr3bpTransferNode, Cr3bpTransferOptimizer,
    Cr3bpTransferPlan,
};
pub use drag_benchmark::DragBenchmarkProblem;
pub use types::{ScvxIterationReport, ScvxOptions, ScvxSolution, TrajectoryNode};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cr3bp::types::{Cr3bpState, Cr3bpSystem};

    #[test]
    fn test_mao_2016_scvx_reproduction_benchmark() {
        let problem = DragBenchmarkProblem {
            v_final: [2.5, 0.0],
            ..Default::default()
        };
        let opts = ScvxOptions {
            n_nodes: 25,
            initial_trust_region: 3.0,
            min_trust_region: 1e-4,
            alpha: 2.0,
            virtual_control_weight: 500.0,
            rho_0: 0.0,
            rho_1: 0.25,
            rho_2: 0.9,
            max_iterations: 15,
            tol_virtual_control: 1e-3,
            tol_state_increment: 1e-3,
        };

        let solution = problem.solve(Some(opts));

        tracing::info!(
            total_iterations = solution.total_iterations,
            accepted = solution.accepted_successions,
            final_cost = solution.final_cost,
            final_nu = solution.final_virtual_control_residual,
            "SCvx benchmark solve completed"
        );
        for rep in &solution.iteration_history {
            tracing::info!(
                iteration = rep.iteration,
                cost = rep.cost_actual,
                ratio_r = rep.ratio_r,
                trust = rep.trust_region_radius,
                nu = rep.virtual_control_norm,
                inc = rep.state_increment_norm,
                accepted = rep.step_accepted,
                "SCvx iteration summary"
            );
        }

        // 1. Must achieve convergence
        assert!(
            solution.converged,
            "SCvx must converge on Mao et al. 2016 benchmark"
        );

        // 2. Must achieve accepted successions
        assert!(
            solution.accepted_successions >= 3,
            "Must accept multiple successive trust region updates: got {}",
            solution.accepted_successions
        );

        // 3. Virtual control defect must be negligible
        assert!(
            solution.final_virtual_control_residual < 5e-3,
            "Virtual control must be absorbed: got {}",
            solution.final_virtual_control_residual
        );

        // 4. Initial and final boundary conditions must be strictly satisfied
        let first_node = solution.nodes.first().unwrap();
        let last_node = solution.nodes.last().unwrap();

        assert!((first_node.state[0] - 0.0).abs() < 1e-3);
        assert!((first_node.state[1] - 0.0).abs() < 1e-3);
        assert!((first_node.state[2] - 5.0).abs() < 1e-3);
        assert!((first_node.state[3] - 0.0).abs() < 1e-3);

        assert!((last_node.state[0] - 10.0).abs() < 1e-3);
        assert!((last_node.state[1] - 10.0).abs() < 1e-3);
        assert!((last_node.state[2] - 2.5).abs() < 1e-3);
        assert!((last_node.state[3] - 0.0).abs() < 1e-3);

        // 5. Fuel cost must be non-zero and bounded by T_max * t_f = 2.0 * 10 = 20.0
        assert!(
            solution.final_cost > 0.5 && solution.final_cost < 20.0,
            "Fuel cost must be physically valid: got {}",
            solution.final_cost
        );
    }

    #[test]
    fn test_cr3bp_low_thrust_transfer_optimization() {
        let system = Cr3bpSystem::earth_moon();

        // Origin: Near Earth-Moon L1 (x ~ 0.8369)
        let origin = Cr3bpState::new(0.8369, 0.0, 0.0, 0.0, 0.12, 0.0);
        // Destination: In the vicinity of L2 (x ~ 1.155)
        let target = Cr3bpState::new(1.155, 0.0, 0.0, 0.0, -0.15, 0.0);

        let config = Cr3bpTransferMissionConfig {
            wet_mass_kg: 450.0,
            max_thrust_n: 0.35, // 350 mN ion engine
            isp_s: 2800.0,      // NEXT-C ion engine
            flight_days: 14.0,   // 14 day flight
            n_nodes: 30,
        };

        let optimizer = Cr3bpTransferOptimizer::new(system, origin, target, config);
        let plan = optimizer.optimize().expect("Transfer optimization must succeed");

        assert!(plan.converged, "SCvx transfer optimizer must converge");
        assert!(plan.nodes.len() == 30, "Must produce 30 trajectory nodes");
        assert!(
            plan.total_delta_v_m_s > 10.0,
            "Must spend non-zero Delta-V for deep space transfer: got {} m/s",
            plan.total_delta_v_m_s
        );
        assert!(
            plan.total_fuel_consumed_kg < 50.0,
            "Fuel consumption must be reasonable for ion engine: got {} kg",
            plan.total_fuel_consumed_kg
        );
        assert!(
            plan.final_mass_kg < 450.0 && plan.final_mass_kg > 400.0,
            "Final mass must reflect fuel expended: got {} kg",
            plan.final_mass_kg
        );

        // Boundary checks
        let start_pos = plan.nodes.first().unwrap().position_km;
        let end_pos = plan.nodes.last().unwrap().position_km;

        let expected_start_x = 0.8369 * 384400.0;
        let expected_end_x = 1.155 * 384400.0;

        assert!((start_pos[0] - expected_start_x).abs() < 10.0);
        assert!((end_pos[0] - expected_end_x).abs() < 10.0);
    }
}
