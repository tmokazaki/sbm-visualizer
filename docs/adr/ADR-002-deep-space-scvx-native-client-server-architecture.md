# ADR-002: Native Client-Server Architecture & Pure-Rust Successive Convexification Engine for Deep-Space Mission Planning

## Status
**Accepted** (2026-09-22)

---

## Context & Problem Statement
The addition of optimal cislunar trajectory planning and orbital candidate generation requires solving non-convex optimal control problems in the Circular Restricted Three-Body Problem (CR3BP). This involves:
1. Variational state transition matrix (STM) integration coupling 6-DoF dynamics with the three-body gravitational Hessian.
2. Iterative convex subproblem solving with line-search trust regions and virtual control absorption.
3. Solving large block KKT systems ($30\text{--}100$ nodes $\times$ $6$ state variables + $3$ control variables).

We needed to determine:
1. **System Topology**: Should the optimizer execute entirely client-side inside the browser via WebAssembly (Option A), or run as a native local daemon communicating with the Web cockpit via REST/WebSocket (Option B)?
2. **Solver Implementation**: Should we link external C/C++ convex optimization libraries (e.g. OSQP, ECOS, IPOPT), or implement a pure-Rust convex subproblem engine?

---

## Decision Drivers
* **UI Responsiveness**: The Three.js 3D viewport requires a continuous 60 FPS rendering loop without stutter or UI freezing during intensive multi-second optimization runs.
* **Portability & Build Simplicity**: The suite must compile cleanly with `cargo build` across macOS, Linux, and Windows without requiring external C compilers, CMake, or system package dependencies.
* **Deterministic Verification**: Every mathematical formulation must be testable headlessly in standard CI environments (`cargo test`).
* **Operator Usability**: The interface must be operable by personnel without background in optimal control theory or non-Euclidean Hamiltonian mechanics.
* **Standard Compatibility**: Flight deliverables must conform to international space flight standards (CCSDS OEM v2.0).

---

## Considered Options

* **Option A: In-Browser WASM Execution**  
  * *Pros*: Single-file deployment; no local server daemon required.
  * *Cons*: Browser memory constraints; single-threaded WebAssembly blocks UI rendering unless complex Web Worker threads are managed; difficult to scale to high-node cluster optimization.

* **Option B: Native Client-Server Architecture (Local-First)**  
  * *Pros*: Full multithreaded native performance; clean separation of concerns; instant 60 FPS UI responsiveness; headless testability; future cloud/cluster scale-out capability.
  * *Cons*: Requires starting a local server process (`cargo run -p sbm_server`).

---

## Decision Outcome

We selected **Option B (Native Client-Server Architecture)** combined with a **Pure-Rust ADMM & LU Solver**:

1. **Native Local Daemon (`crates/sbm_server`)**:
   - Implemented with [Axum 0.7](https://github.com/tokio-rs/axum) and [Tokio](https://tokio.rs/) running locally on `http://127.0.0.1:8080`.
   - Exposes `/api/v1/transfer/optimize`, `/api/v1/export/oem`, and orbit correction endpoints.
   - Serves the static 3D visualizers directly for a seamless zero-configuration experience.

2. **Pure-Rust SCvx Solver (`crates/sbm_core::scvx`)**:
   - Zero external C/C++ dependencies.
   - Combines an in-place $LU$ solver with partial pivoting for affine dynamics equality constraints with Projected Alternating Direction Method of Multipliers (ADMM) for $L_2$ thrust saturation.

3. **Mission Planning Wizard UI**:
   - Embedded interactive modal inside `cr3bp_deep_space_visualizer.html`.
   - Automatically computes and ranks **Top 3 Flight Candidates** (Minimum Fuel, Balanced Transit, Rapid Response) and renders discrete operational burn schedules.

---

## Consequences

### Positive
* **High Performance**: Native SIMD-optimized Rust execution without WebAssembly sandboxing overhead.
* **Zero UI Lag**: Optimization runs asynchronously in background Tokio worker threads while the Three.js viewport renders smoothly.
* **Zero Dependency Hell**: No C/C++ build dependencies; builds out of the box with standard Rust toolchain.
* **Standards Compliance**: Native generation of official CCSDS OEM v2.0 ephemerides.
* **Zero Warnings & Zero Prints**: Strict lint compliance enforced via `#![deny(clippy::print_stdout, clippy::print_stderr)]` and `tracing`.

### Negative / Trade-offs
* Users must run `cargo run -p sbm_server` to enable real-time SCvx optimization (the web visualizer includes offline fallbacks and clear status indicators when the daemon is not running).
