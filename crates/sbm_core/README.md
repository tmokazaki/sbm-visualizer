# `sbm_core`

A high-performance, zero-dependency Rust library implementing four core pillars of modern astrodynamics and space situational awareness:

1. **NASA EVOLVE 4.0 Standard Breakup Model (SBM)**: Hypervelocity collision disruption, cratering, and explosive ruptures.
2. **AutoOrbit (KDD 2026)**: Physics-informed satellite orbit prediction with 1D Fourier Neural Operators and Gaussian Variational Equations.
3. **Circular Restricted Three-Body Problem (CR3BP, AAS 20-459)**: High-precision cislunar/deep-space propagation, periodic Lyapunov/Halo/NRHO orbits, and low-energy invariant manifold transfers.
4. **Successive Convexification (SCvx) Trajectory Optimizer (Mao 2016, Malyuta 2021)**: Pure-Rust convex subproblem engine with in-place $LU$ solver and Projected ADMM for fuel-optimal deep space trajectory planning.

---

## 1. NASA EVOLVE 4.0 Standard Breakup Model

Based on the peer-reviewed specification:
> **Johnson, N. L., Krisko, P. H., Liou, J.-C., & Anz-Meador, P. D. (2001).**  
> *NASA's new breakup model of EVOLVE 4.0.*  
> Advances in Space Research, 28(9), 1377–1387.

### Features
- **Pure Rust, Zero Dependencies**: Uses only standard library primitives (`std`). Deterministic, thread-safe, and compatible with WebAssembly (`wasm32`) and embedded systems.
- **Both Event Regimes**:
  - **Hypervelocity Collisions**: Automatic classification of Catastrophic Disruption ($E_p \ge 40\text{ kJ/kg}$) vs Non-Catastrophic Cratering ($M = M_{\text{smaller}} \cdot v_{\text{imp}}$).
  - **Explosive Ruptures**: Pressure and propellant ruptures with scaling factor $S$.
- **Empirical $A/M$ Bimodal Distributions**:
  - Small shards ($L_c < 0.08\text{ m}$): SOCIT experimental impact distribution.
  - Large Spacecraft ($L_c \ge 0.11\text{ m}$): Dual-peak structural mixture.
  - Large Rocket Bodies ($L_c \ge 0.11\text{ m}$): Tankage and casing mixture.
  - Linear bridging across transition boundaries.
- **Radar Cross-Sectional Area ($A_x$)**: Continuous piecewise power-law scaling across transition threshold $L_c = 1.67\text{ mm}$.
- **Physically Conditioned Ejection Velocity ($\Delta v$)**: Log-normal distribution conditioned directly on $\chi = \log_{10}(A/M)$ ($\sigma = 0.40$).
- **Strict Mass Conservation**: Individual fragment masses derived via $M = A_x / (A/M)$ and normalized to conserve destroyed mass.
- **Dual Population Telemetry**: Physical debris yield ($L_c \ge 1\text{ cm}$), SSN trackable yield ($L_c \ge 10\text{ cm}$), and surviving remnant mass.

### Breakup Simulation Example

```rust
use sbm_core::prelude::*;
use tracing::info;

fn main() -> Result<(), BreakupError> {
    // 1. Configure scenario via fluent builder
    let engine = BreakupEngine::builder()
        .target_mass(1000.0)             // 1000 kg target satellite
        .projectile_mass(100.0)          // 100 kg projectile
        .impact_speed(10_000.0)          // 10 km/s (10,000 m/s)
        .target_type(ObjectType::Spacecraft)
        .projectile_type(ObjectType::Spacecraft)
        .breakup_type(BreakupType::Collision)
        .num_fragments(1500)
        .build()?;

    // 2. Run deterministic Monte Carlo simulation
    let result = engine.simulate(42);

    info!(
        catastrophic = result.is_catastrophic,
        destroyed_mass_kg = result.destroyed_mass_kg,
        yield_1cm = result.physical_yield_1cm,
        yield_10cm_ssn = result.ssn_trackable_yield_10cm,
        "Breakup simulation completed"
    );

    // 3. Inspect individual fragments
    for shard in result.top_heaviest(3) {
        info!(id = shard.id, mass_kg = shard.mass_kg, size_m = shard.size_m, speed_mps = shard.speed_mps, "Top shard");
    }

    Ok(())
}
```

---

## 2. AutoOrbit: Physics-Informed Satellite Orbit Prediction (KDD 2026)

Based on the research paper:
> **Yuan, T., Gao, D., Long, R., Zhang, J., Zhao, X., Xu, M., & Li, Y. (2026).**  
> *AutoOrbit: Towards Long-term Satellite Orbit Prediction with Global-Local Physics.*  
> In Proceedings of the 32nd ACM SIGKDD Conference on Knowledge Discovery and Data Mining (KDD '26).  
> DOI: [10.1145/3770855.3818960](https://doi.org/10.1145/3770855.3818960)

### 3-Level Hierarchical Architecture
1. **Global Orbital Structure**: Mean reference orbit $s_{ref}(t)$ constructed via ground-track recurrence phase-averaging. Residual deviations $r_{res}(t) = s_{obs}(t) - s_{ref}(t)$ are predicted to avoid numerical dynamic range issues and long-horizon drift.
2. **Local Orbital Dynamics**: 1D Fourier Neural Operator (FNO1d) with low-frequency mode truncation ($k_{max}$) and acceleration-level physics loss regularizing kinematic motion against Earth gravity, $J_2\text{--}J_4$ harmonics, and atmospheric drag.
3. **Discrete Maneuver Correction**: Gaussian Variational Equations (GVEs) mapping impulsive RAC velocity increments $[\Delta v_r, \Delta v_a, \Delta v_c]^T$ to instantaneous orbital element jumps $(\Delta a, \Delta e, \Delta \omega, \Delta i, \Delta \Omega)$, analytically propagated with $O(H)$ complexity via Kepler's equation.

### AutoOrbit Usage Example

```rust
use sbm_core::prelude::*;
use tracing::info;

fn main() -> Result<(), AutoOrbitError> {
    let predictor = AutoOrbitPredictor::sentinel_1a_preset();

    let observations: Vec<StateVector> = (0..128)
        .map(|step| predictor.reference_orbit.state_at_step(step))
        .collect();

    let maneuver = ManeuverImpulse::new(0.0, 0.5, 0.0, 0.0); // +0.5 m/s along-track burn

    let predicted_trajectory = predictor.predict(
        &observations,
        128,
        PredictionHorizon::HalfHour,
        Some(&maneuver),
    )?;

    info!(states_count = predicted_trajectory.len(), "Predicted future states");
    Ok(())
}
```

---

## 3. Circular Restricted Three-Body Problem (CR3BP, AAS 20-459)

Based on the peer-reviewed specification:
> **Short, C., Haapala, A., & Bosanac, N. (2020).**  
> *Technical Implementation of the Circular Restricted Three-Body Model in STK Astrogator.*  
> AAS/AIAA Astrodynamics Specialist Conference, AAS 20-459.

### Core Capabilities
- **Equations of Motion & Pseudo-Potential**: $U^*$, $\nabla U^*$, Hessian $U^*_{ij}$, and variational equations for State Transition Matrix (STM) propagation.
- **Astrogator Frame Transformations**: 6D and 9D transformations between STK Central Body Inertial (CBI) and Rotating Barycentric frames.
- **High-Order Adaptive Integration**: Dormand-Prince 5(4) with adaptive step size control, Jacobi conservation $\Delta C_J < 10^{-12}$, and root-finding event detection for Poincaré sections and hyperplanes.
- **Equilibrium Points & Periodic Orbit Families**: Euler quintic solver for $L_1\text{--}L_5$, Planar Lyapunov, 3D Halo, NRHO ($86\text{ km}$ perilune), and JWST deep space mission orbits.
- **Multi-Body Low-Energy Transfers**: Reproduction of the AAS 20-459 Section 5 3-maneuver itinerary ($\Delta v_1 \approx 0.19\text{ mm/s}$, $\Delta v_2 \approx 23.2\text{ m/s}$ at $\Sigma: x = 1-\mu$, $\Delta v_3 \approx 9\text{ mm/s}$).

### CR3BP Usage Example

```rust
use sbm_core::prelude::*;
use tracing::info;

fn main() -> Result<(), Cr3bpError> {
    let system = Cr3bpSystem::earth_moon();

    let l_points = compute_lagrange_points(&system)?;
    info!(l1_x = l_points[0].state.x, l1_cj = l_points[0].jacobi_constant, "Lagrange point L1");

    let lyap = PeriodicOrbitBenchmark::earth_moon_l1_lyapunov();
    let integrator = DormandPrinceIntegrator::new(&system, IntegratorOptions::default());
    let result = integrator.propagate_6d(&lyap.initial_state, 0.0, lyap.period_nondim, None)?;

    info!(period_days = lyap.period_days, max_cj_var = result.max_jacobi_variation, "Lyapunov orbit");

    let transfer = compute_earth_moon_l1_to_l2_transfer(&system, None)?;
    info!(total_dv_ms = transfer.total_dv_ms, duration_days = transfer.transfer_duration_days, "L1->L2 Transfer");

    Ok(())
}
```

---

## 4. Successive Convexification (SCvx) Trajectory Optimizer

Based on:
> **Mao, Y., Szmuk, M., & Açıkmeşe, B. (2016).**  
> *Successive Convexification of Non-Convex Optimal Control Problems with State Constraints.*  
> arXiv:1608.05133.
>
> **Malyuta, D., et al. (2021).**  
> *Advances in Trajectory Optimization for Aerospace Systems: A Tutorial on Successive Convexification.*  
> IEEE Control Systems Magazine.

### Core Capabilities
- **Pure-Rust ADMM & LU Factorization**: Solves the convexified subproblem without external C/C++ solvers (OSQP, ECOS, IPOPT). An in-place $LU$ solver factorizes the block KKT dynamics equality matrix once per succession, while Projected ADMM handles $L_2$ thrust saturation $\|\mathbf{u}\|_2 \le T_{\max}$ with decoupled proximal shrinkage.
- **Line-Search Trust Regions & Virtual Control**: Dynanically adjusts trust radius $r_k$ based on step ratio $\rho_k = \Delta J / \Delta L$. Absorbs infeasible initializations through virtual control penalty $\lambda_{\nu} \|\boldsymbol{\nu}\|_1 \to 0$.
- **Cislunar Transfer Optimization**: Couples 6-DoF rotating equations of motion with thruster models (NASA NEXT-C, Busek BHT-600, Chemical Bipropellant) to generate operational burn schedules and 3D trajectory waypoints.

### SCvx Transfer Optimization Example

```rust
use sbm_core::cr3bp::{Cr3bpState, Cr3bpSystem};
use sbm_core::scvx::{Cr3bpTransferMissionConfig, Cr3bpTransferOptimizer};
use tracing::info;

fn main() -> Result<(), String> {
    let system = Cr3bpSystem::earth_moon();
    let origin = Cr3bpState::new(0.8369, 0.0, 0.0, 0.0, 0.12, 0.0);
    let target = Cr3bpState::new(1.155, 0.0, 0.05, 0.0, -0.15, 0.0);

    let config = Cr3bpTransferMissionConfig {
        wet_mass_kg: 450.0,
        max_thrust_n: 0.35,
        isp_s: 2800.0,
        flight_days: 14.0,
        n_nodes: 30,
    };

    let optimizer = Cr3bpTransferOptimizer::new(system, origin, target, config);
    let plan = optimizer.optimize()?;

    info!(
        converged = plan.converged,
        iterations = plan.iterations,
        total_delta_v_ms = plan.total_delta_v_m_s,
        fuel_kg = plan.total_fuel_consumed_kg,
        burn_segments = plan.burn_schedule.len(),
        "SCvx Transfer Plan Complete"
    );

    Ok(())
}
```

---

## Testing & Verification

```bash
# Run all 56 tests across workspace
cargo test --workspace

# Enforce zero warnings and zero print calls
cargo clippy --workspace --all-targets -- -D warnings -D clippy::print_stdout -D clippy::print_stderr

# Run Python paper reproduction test suite
python3 -m unittest tests/test_autoorbit_reproduction.py
```
