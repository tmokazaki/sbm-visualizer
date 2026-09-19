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

## Testing & Quality Assurance

```bash
# Run all unit tests, integration tests, and doc-tests
cargo test

# Enforce zero-warning linting across all targets
cargo clippy --all-targets -- -D warnings

# Build documentation
cargo doc --no-deps
```
