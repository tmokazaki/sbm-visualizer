# `sbm_simple_engine`

High-performance, zero-dependency Rust crate implementing the **NASA EVOLVE 4.0 Standard Breakup Model (SBM)** for orbital collisions and explosions.

Based on the peer-reviewed specification:
> **Johnson, N. L., Krisko, P. H., Liou, J.-C., & Anz-Meador, P. D. (2001).**  
> *NASA's new breakup model of EVOLVE 4.0.*  
> Advances in Space Research, 28(9), 1377–1387.

---

## Features

- **Pure Rust, Zero Dependencies**: Uses only standard library primitives (`std`). Deterministic, thread-safe, and ready for WebAssembly (`wasm32`) and embedded systems.
- **Both Event Regimes**:
  - **Hypervelocity Collisions**: Automatic classification of Catastrophic Disruption ($E_p \ge 40\text{ kJ/kg}$) vs Non-Catastrophic Cratering ($M = M_{\text{smaller}} \cdot v_{\text{imp}}$).
  - **Explosive Ruptures**: Pressure/propellant ruptures with scaling factor $S$.
- **Empirical $A/M$ Bimodal Distributions**:
  - Small shards ($L_c < 0.08\text{ m}$): SOCIT experimental impact distribution.
  - Large Spacecraft ($L_c \ge 0.11\text{ m}$): Dual-peak structural mixture.
  - Large Rocket Bodies ($L_c \ge 0.11\text{ m}$): Tankage and casing mixture.
  - Linear bridging across transition boundaries.
- **Radar Cross-Sectional Area ($A_x$)**: Continuous piecewise power-law scaling across transition threshold $L_c = 1.67\text{ mm}$.
- **Physically Conditioned Ejection Velocity ($\Delta v$)**: Log-normal distribution conditioned directly on $\chi = \log_{10}(A/M)$ ($\sigma = 0.40$).
- **Strict Mass Conservation**: Individual fragment masses derived via $M = A_x / (A/M)$ and normalized to conserve destroyed mass.
- **Dual Population Telemetry**: Physical debris yield ($L_c \ge 1\text{ cm}$), SSN trackable yield ($L_c \ge 10\text{ cm}$), and surviving remnant mass.

---

## Adding as a Dependency

In your `Cargo.toml`:

```toml
[dependencies]
sbm_simple_engine = { path = "../engine_cli" }
```

---

## Library Usage

### 1. Basic Collision Simulation

```rust
use sbm_simple_engine::prelude::*;

fn main() -> Result<(), BreakupError> {
    // Configure scenario via fluent builder
    let engine = BreakupEngine::builder()
        .target_mass(1000.0)             // 1000 kg target satellite
        .projectile_mass(100.0)          // 100 kg projectile
        .impact_speed(10_000.0)          // 10 km/s (10,000 m/s)
        .target_type(ObjectType::Spacecraft)
        .projectile_type(ObjectType::Spacecraft)
        .breakup_type(BreakupType::Collision)
        .num_fragments(1500)
        .build()?;

    // Run deterministic Monte Carlo simulation
    let result = engine.simulate(42);

    println!("Catastrophic: {}", result.is_catastrophic);
    println!("Destroyed Mass: {:.1} kg", result.destroyed_mass_kg);
    println!("Physical Yield (>=1cm): {:.0}", result.physical_yield_1cm);
    println!("SSN Trackable (>=10cm): {:.0}", result.ssn_trackable_yield_10cm);

    // Inspect individual fragments
    for shard in result.top_heaviest(3) {
        println!("Shard #{} -> Mass: {:.2} kg, Size: {:.2} m, Speed: {:.1} m/s",
            shard.id, shard.mass_kg, shard.size_m, shard.speed_mps);
    }

    Ok(())
}
```

### 2. Cratering Collision with Surviving Remnant

```rust
use sbm_simple_engine::prelude::*;

let engine = BreakupEngine::builder()
    .target_mass(2000.0)
    .projectile_mass(0.5) // 500 g debris pellet
    .impact_speed(7000.0) // 7 km/s -> Ep = 6.1 kJ/kg < 40 kJ/kg
    .build()?;

let result = engine.simulate(101);
assert!(!result.is_catastrophic);
println!("Destroyed Mass: {:.2} kg", result.destroyed_mass_kg); // 0.5 * 7 = 3.5 kg
println!("Surviving Remnant Mass: {:.2} kg", result.remnant_mass_kg); // 1996.5 kg
```

### 3. Explosive Breakup

```rust
use sbm_simple_engine::prelude::*;

let engine = BreakupEngine::builder()
    .target_mass(1200.0)
    .target_type(ObjectType::RocketBody)
    .breakup_type(BreakupType::Explosion)
    .explosion_scaling(1.5)
    .build()?;

let result = engine.simulate(999);
```

### 4. Custom RNG Integration

Implement the [`RngSource`](crate::rng::RngSource) trait to use custom random generators (such as `rand::rngs::StdRng`):

```rust
use sbm_simple_engine::prelude::*;

struct MyRng;
impl RngSource for MyRng {
    fn next_f64(&mut self) -> f64 {
        0.5 // or custom generator
    }
}

let engine = BreakupEngine::new(1000.0, 100.0, 10_000.0)?;
let result = engine.simulate_with_rng(&mut MyRng);
```

---

## CLI Usage

Run the built-in command-line tool:

```bash
cargo run --release
```

This simulates the nominal scenario, displays formatted console summaries, and exports `fragments_output.json`.

---

## 2. AutoOrbit: Physics-Informed Satellite Orbit Prediction (KDD 2026)

`sbm_core::autoorbit` provides a pure-Rust, zero-dependency implementation of the **AutoOrbit** framework for long-term physics-informed satellite orbit prediction and discrete maneuver correction.

Based on the research paper:
> **Yuan, T., Gao, D., Long, R., Zhang, J., Zhao, X., Xu, M., & Li, Y. (2026).**  
> *AutoOrbit: Towards Long-term Satellite Orbit Prediction with Global-Local Physics.*  
> In Proceedings of the 32nd ACM SIGKDD Conference on Knowledge Discovery and Data Mining (KDD '26).  
> DOI: [10.1145/3770855.3818960](https://doi.org/10.1145/3770855.3818960)

### 3-Level Hierarchical Architecture
1. **Global Orbital Structure**: Mean reference orbit $s_{ref}(t)$ constructed via ground-track recurrence phase-averaging (Eq. 1). Residual deviations $r_{res}(t) = s_{obs}(t) - s_{ref}(t)$ are predicted to avoid numerical dynamic range issues and long-horizon drift (Eqs. 2–3).
2. **Local Orbital Dynamics**: 1D Fourier Neural Operator (FNO1d) with low-frequency mode truncation ($k_{max}$) and acceleration-level physics loss (Eqs. 8–9) regularizing kinematic motion against Earth gravity, $J_2\text{--}J_4$ harmonics, and atmospheric drag.
3. **Discrete Maneuver Correction**: Gaussian Variational Equations (GVEs, Eqs. 10–12) mapping impulsive RAC velocity increments $[\Delta v_r, \Delta v_a, \Delta v_c]^T$ to instantaneous orbital element jumps $(\Delta a, \Delta e, \Delta \omega, \Delta i, \Delta \Omega)$, analytically propagated with $O(H)$ complexity via Kepler's equation.

### AutoOrbit Usage Example

```rust
use sbm_core::prelude::*;

fn main() -> Result<(), AutoOrbitError> {
    // 1. Initialize calibrated predictor preset (e.g. Sentinel-1A sun-synchronous orbit)
    let predictor = AutoOrbitPredictor::sentinel_1a_preset();

    // 2. Feed historical in-orbit GNSS measurements (e.g. 128 past steps at 10s cadence)
    let observations: Vec<StateVector> = (0..128)
        .map(|step| predictor.reference_orbit.state_at_step(step))
        .collect();

    // 3. Optional: Define a scheduled orbit maintenance or collision avoidance maneuver
    let maneuver = ManeuverImpulse::new(0.0, 0.5, 0.0, 0.0); // +0.5 m/s along-track burn

    // 4. Run real-time forward prediction across target horizon (e.g. 30 minutes)
    let predicted_trajectory = predictor.predict(
        &observations,
        128,
        PredictionHorizon::HalfHour,
        Some(&maneuver),
    )?;

    println!("Predicted {} future states at 10s cadence.", predicted_trajectory.len());
    println!("Post-maneuver state at step 0: {:?}", predicted_trajectory[0]);

    Ok(())
}
```

---

## 3. Circular Restricted Three-Body Problem (CR3BP) Propagator & Deep Space Engine

A high-precision, zero-dependency implementation of the Circular Restricted Three-Body Problem (CR3BP) for cislunar and deep space trajectory design, grounded in the peer-reviewed specification:
> **Short, C., Haapala, A., & Bosanac, N. (2020).**  
> *Technical Implementation of the Circular Restricted Three-Body Model in STK Astrogator.*  
> AAS/AIAA Astrodynamics Specialist Conference, AAS 20-459.

### Core Capabilities
- **Equations of Motion & Pseudo-Potential**: $U^*$, $\nabla U^*$, Hessian $U^*_{ij}$, and variational equations for State Transition Matrix (STM) propagation (Eqs. 1–3, 10).
- **Astrogator Frame Transformations**: 6D and 9D transformations between STK Central Body Inertial (CBI) and Rotating Barycentric frames (Table 1, Eqs. 4–9, 11).
- **High-Order Adaptive Integration**: Dormand-Prince 5(4) with adaptive step size control, Jacobi conservation $\Delta C_J < 10^{-12}$, and root-finding event detection for Poincaré sections and hyperplanes.
- **Equilibrium Points & Periodic Orbit Families**: Euler quintic solver for $L_1\text{--}L_5$, Planar Lyapunov, 3D Halo, NRHO ($86\text{ km}$ perilune), and JWST deep space mission orbits.
- **Multi-Body Low-Energy Transfers**: Reproduction of the AAS 20-459 Section 5 3-maneuver itinerary ($\Delta v_1 \approx 0.19\text{ mm/s}$, $\Delta v_2 \approx 23.2\text{ m/s}$ at $\Sigma: x = 1-\mu$, $\Delta v_3 \approx 9\text{ mm/s}$).

### CR3BP Usage Example

```rust
use sbm_core::prelude::*;

fn main() -> Result<(), Cr3bpError> {
    // 1. Initialize Earth-Moon CR3BP system (mu = 0.0121505856)
    let system = Cr3bpSystem::earth_moon();

    // 2. Compute exact Lagrange libration points L1-L5
    let l_points = compute_lagrange_points(&system)?;
    println!("L1 position: x = {:.6}, CJ = {:.6}", l_points[0].state.x, l_points[0].jacobi_constant);

    // 3. Propagate benchmark L1 Planar Lyapunov orbit
    let lyap = PeriodicOrbitBenchmark::earth_moon_l1_lyapunov();
    let integrator = DormandPrinceIntegrator::new(&system, IntegratorOptions::default());
    let result = integrator.propagate_6d(&lyap.initial_state, 0.0, lyap.period_nondim, None)?;

    println!("Orbit period: {:.2} days", lyap.period_days);
    println!("Max Jacobi variation: {:.2e}", result.max_jacobi_variation);

    // 4. Compute AAS 20-459 L1 -> L2 Low-Energy Multi-Body Transfer
    let transfer = compute_earth_moon_l1_to_l2_transfer(&system, None)?;
    println!("Total Delta-V: {:.2} m/s (vs 800+ m/s for 2-body transfer)", transfer.total_dv_ms);
    println!("Transfer duration: {:.2} days", transfer.transfer_duration_days);

    Ok(())
}
```

---

## Testing & Quality Assurance

```bash
# Run all unit tests, integration tests, and doc-tests
cargo test --workspace

# Enforce zero-warning linting across all targets
cargo clippy --workspace --all-targets -- -D warnings

# Run Python paper reproduction test suite
python3 -m unittest tests/test_autoorbit_reproduction.py

# Run the CR3BP Deep Space Demo executable
cargo run --example cr3bp_deep_space_demo
```
