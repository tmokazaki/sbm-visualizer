# NASA Breakup Model Visualizer & Orbital Hazard Suite

A high-performance, offline-first 3D visualization and astrodynamics analysis suite for hypervelocity satellite collisions in Low Earth Orbit (LEO), based on the **NASA Standard Breakup Model (NASA SBM)**, **Clohessy-Wiltshire (Hill) relative orbital dynamics**, **volumetric spatial density hazard modeling**, and an **interactive Gabbard diagram**.

![Gabbard Diagram & 3D Hazard Volumes](docs/images/screenshot_gabbard.png)

---

## Key Capabilities

* **Mathematical $T = 0.000\text{s}$ Impact Synchronization**:
  * Incoming approach trajectories with visual flight guide lines.
  * Surfaces touch at precisely $(0, 0, 0)$ at $T = 0.0\text{s}$, followed by a dynamic shockwave ring and collision flash.
* **Physical Mass-Proportional Scaling ($R \propto \sqrt[3]{m}$)**:
  * Fragment visual radii scale strictly to the cube root of physical fragment mass ($R_i \propto m_i^{1/3}$).
  * Custom WebGL GLSL `ShaderMaterial` with view-space point attenuation, anti-aliased Gaussian glow discs, and additive blending.
* **Clohessy-Wiltshire (Hill) Orbital Propagation**:
  * Closed-form analytical propagation in the rotating Hill frame ($\hat{\mathbf{x}}$ radial, $\hat{\mathbf{y}}$ in-track, $\hat{\mathbf{z}}$ cross-track).
  * Dual-mode scrubbing: **Impact Zoom** ($-10\text{s}$ to $+30\text{s}$) for collision micro-dynamics and **Full 15-Minute Orbit** ($-10\text{s}$ to $+900\text{s}$) capturing multi-thousand-kilometer secular along-track Keplerian shear ($y_{\text{secular}} \approx -3 \Delta v_y t$).
* **Volumetric Spatial Debris Density Heatmap ($\rho(x, y, z)$)**:
  * Dynamic spatial covariance tensor $\mathbf{\Sigma}(t) = \text{diag}(\sigma_x^2, \sigma_y^2, \sigma_z^2)$.
  * Real-time calculation of peak spatial density:
    $$\rho_{\max}(t) = \frac{N}{(2\pi)^{3/2} \sigma_x(t) \sigma_y(t) \sigma_z(t)} \quad [\text{fragments / km}^3]$$
  * Volumetric 3D Hazard Envelopes: **$1\sigma$ Core Hazard Shell** (crimson red wireframe) and **$2\sigma$ Dispersion Boundary** (cyan wireframe).
  * Vertex coloring mapping local concentration relative to peak density.
* **Interactive Gabbard Diagram ($P \text{ vs. } h_a / h_p$)**:
  * Classical astrodynamics plot displaying orbital period ($P$) vs. apogee ($h_a$, cyan) and perigee ($h_p$, orange/red).
  * Calibrated LEO domain ($86\text{--}114\text{ min}$, $0\text{--}2,500\text{ km}$) highlighting the classic "X-wing" crossing at reference breakup altitude ($h_0 = 500\text{ km}$, $T_0 = 94.62\text{ min}$).
  * **Atmospheric Re-entry Danger Zone ($< 120\text{ km}$)** and direct ballistic surface impact ($0\text{ km}$) detection with live percentage tracking ($\sim 55\%$).
  * **Bidirectional Cross-Highlighting**: Hovering or clicking any fragment point on the Gabbard canvas displays a full telemetry tooltip and illuminates a glowing yellow spotlight ring on the corresponding fragment in 3D space.
  * Draggable floating window with minimize/restore and header toggles.
* **Interactive 3D Fragment Selection & Floating Inspector HUD**:
  * Precision 3D raycasting with drag-filtering selects individual fragments directly in the 3D scene.
  * Floating, draggable Fragment Inspector HUD displaying detailed physical and orbital parameters: mass, characteristic length ($L_c$), cross-sectional area ($A_x$), area-to-mass ratio ($A/M$), $B^*$ ballistic drag coefficient, ejection $\Delta v$, apogee, perigee, period, parent satellite origin, and orbit stability classification.
  * Bidirectional synchronization across 3D viewport, Gabbard diagram, and data preview table.
* **Dynamic Clohessy-Wiltshire (CW) 3D Orbit Trails**:
  * Real-time closed-form evaluation of Hill relative orbit curves for past trajectories and future orbit predictions.
  * Multiple display modes: **Selected Fragment**, **Top 5 Heaviest Fragments**, **Top 5 Fastest Fragments**, and **4 Cardinal Axes** (In-track, Radial, Cross-track).
* **AutoOrbit: Physics-Informed Satellite Orbit Prediction (KDD 2026)**:
  * Pure-Rust, zero-dependency implementation of the hierarchical orbit prediction framework:
    * **Global Structure**: Ground-track recurrence phase-averaged reference orbit ($s_{ref}$) and residual learning.
    * **Local Dynamics**: 1D Fourier Neural Operator (FNO1d) with low-frequency mode truncation ($k_{max}$) and acceleration-level physics loss ($\vec{a}_{pred} = \frac{-\vec{v}_{+2} + 8\vec{v}_{+1} - 8\vec{v}_{-1} + \vec{v}_{-2}}{12\tau}$).
    * **Maneuver Correction**: Gaussian Variational Equations (GVEs) mapping impulsive RAC thrust $\Delta \vec{v}$ to instantaneous element jumps $(\Delta a, \Delta e, \Delta \omega, \Delta i, \Delta \Omega)$ with $O(H)$ Keplerian propagation.
  * Python training & weight export pipeline (`scripts/autoorbit/`) and reproduction test suite (`tests/test_autoorbit_reproduction.py`).
* **100% Offline & Standalone Execution**:
  * Zero external network or CDN dependencies. Runs completely self-contained in modern web browsers.

---

## Gallery

| Interactive Fragment Inspector & Orbit Trails | 3D Debris Cloud with Minimized Gabbard |
| :---: | :---: |
| ![Inspector & Trails](docs/images/screenshot_trails_verified.png) | ![3D Cloud Minimized Gabbard](docs/images/screenshot_gabbard_minimized.png) |

| Volumetric Debris Density Heatmap ($\rho$) | 15-Minute Orbital Shear Dispersion |
| :---: | :---: |
| ![Density Heatmap](docs/images/screenshot_density.png) | ![15-Minute Shear](docs/images/screenshot_full.png) |

---

## Mathematical & Architectural Documentation

Detailed engineering and mathematical specifications are provided in the `docs/` directory:

1. [**Mathematical Specification**](docs/breakup_model_mathematical_specification.md):
   * Coordinate frame transformations ($\mathcal{F}_{\text{ECI}} \leftrightarrow \mathcal{F}_{\text{LVLH}}$).
   * Specific impact energy ($E_p$) and catastrophic disruption criterion ($E_p \ge 40\text{ kJ/kg}$).
   * NASA SBM power-law sampling and mass conservation.
   * Log-normal velocity dispersion and directional distribution functions.
   * Analytical Clohessy-Wiltshire state transition equations.
   * 3D spatial covariance tensor and Mahalanobis distance metric.
   * Vis-viva reconstruction and analytical proof of the Gabbard "X-wing" asymptotes.
2. [**Application Architecture Specification**](docs/application_architecture_and_functional_specification.md):
   * Component architecture and module-by-module breakdown.
   * UI/UX glassmorphism layout and window management.
   * Headless Chrome verification protocol and URL parameter automation.
3. [**Operational Space Domain Awareness (SDA) Roadmap**](docs/operational_space_situational_awareness_roadmap.md):
   * Detailed gap analysis between academic breakup models and real-world operational environments (NASA CARA, USSF 18th SDS, ESA).
   * Canonical and SOTA reference papers across 7 operational domains (HPOP, Sensor/RCS, Conjunction Assessment, Frame Standards, UQ, Component Breakup, Parallel Compute).
   * Governing equations for Gim-Alfriend $J_2$ STM, NASA SEM radar cross-section conversion, Hall fast 2D $P_c$ collision probability, and CCSDS CDM/OEM standards.
   * 6-phase engineering transition roadmap.
4. [**Deep Space Optimal Trajectory Planning & SCvx Specification**](docs/deep_space_scvx_flight_dynamics_specification.md):
   * Multi-body non-convex optimal control formulation in the Circular Restricted Three-Body Problem (CR3BP).
   * Variational dynamics linearization ($\mathbf{A}_k, \mathbf{B}_k, \mathbf{r}_k$), Coriolis coupling ($\boldsymbol{\Omega}$), and potential Hessian ($\mathbf{U}_{(x, y, z)}$).
   * In-place $LU$ factorization and Projected ADMM for exact $L_2$ thrust saturation ($\|\mathbf{u}\|_2 \le T_{\max}$) with zero external C dependencies.
   * Dynamic line-search trust regions ($\rho$-ratio step adaptation) and virtual control absorption ($\|\boldsymbol{\nu}\|_1 \to 0$).
   * Verified reproduction of Mao et al. (2016) drag benchmark and Short et al. (2020) AAS 20-459 low-energy transfer.
5. [**Earth-Centric Optimal Orbit Search & Trajectory Optimization Guide**](docs/earth_centric_optimal_orbit_search_literature_and_architecture.md):
   * Comprehensive astrodynamics literature survey and mathematical formulations for Earth-centric orbit space (LEO, MEO, GEO, GTO).
   * Modified Equinoctial Elements (MEE) and Gauss Variational Equations (GVE).
   * Edelbaum's analytical low-thrust $\Delta v$ velocity equation and Petropoulos' Q-law Lyapunov feedback control.
   * Successive Convex Programming (SCP / SOCP) and $J_2$ secular nodal precession drift for active debris removal (ADR) and constellation phasing.
   * Integration with the SBM codebase (`autoorbit`, `cr3bp::frames`, `scvx`) and 3-phase implementation roadmap.
6. [**Architectural Decision Records (ADRs)**](docs/adr/):
   * [ADR-001: NASA EVOLVE 4.0 Breakup Model Alignment](docs/adr/ADR-001-nasa-evolve4-breakup-model-alignment.md).
   * [ADR-002: Native Client-Server Architecture & Pure-Rust SCvx Trajectory Engine](docs/adr/ADR-002-deep-space-scvx-native-client-server-architecture.md).

---

## Quick Start

### Running the Web Visualizer
Simply open `index.html` in any modern web browser:
```bash
# macOS
open index.html

# Linux
xdg-open index.html

# Windows
start index.html
```

### URL Automation Parameters
You can launch the visualizer with pre-configured parameters:
```bash
# Open at T = +15s, paused, in density heatmap mode, hovering fragment #12
open "index.html?t=15&play=0&color=density&hover=12"

# Open in full 15-minute orbit mode at T = +7.5 min
open "index.html?t=450&play=0&mode=full"
```

Available query parameters:
* `t`: Initial time in seconds (e.g., `-5`, `0`, `8`, `450`).
* `mode`: Timeline range (`zoom` for $-10\text{s}$ to $+30\text{s}$, `full` for $-10\text{s}$ to $+900\text{s}$).
* `play`: Initial playback state (`1` or `0`).
* `color`: Particle coloring mode (`contour`, `density`, `smooth`, `origin`).
* `gabbard`: Gabbard window visibility (`1` or `0`).
* `volumes`: 3D hazard envelopes visibility (`1` or `0`).
* `hover`: Highlight fragment ID on start (e.g. `12`).
* `select`: Select fragment ID and open Inspector on start (e.g. `0`).
* `trails`: Orbit trail mode (`off`, `selected`, `heaviest`, `fastest`, `cardinal`).

---

### Deep Space Flight Dynamics & Successive Convexification (SCvx)

The suite integrates a pure-Rust, state-of-the-art optimal trajectory candidate finder for cislunar and deep-space missions:

* **Successive Convexification (SCvx) Engine (`sbm_core::scvx`)**:
  - Exact scientific reproduction of Mao, Szmuk, Açıkmeşe (2016) (*Successive Convexification of Non-Convex Optimal Control Problems with State Constraints*, arXiv:1608.05133) and Malyuta et al. (2021) (*IEEE CSM Tutorial*).
  - In-place $LU$ solver with partial pivoting factorizing the block KKT system once per succession.
  - Projected Alternating Direction Method of Multipliers (ADMM) enforcing exact $L_2$ thrust saturation $\|\mathbf{u}\|_2 \le T_{\max}$ with decoupled proximal shrinkage.
  - Dynamical line-search trust-region adaptation ($\rho$-ratio trust expansion/contraction) and virtual control absorption ($\|\boldsymbol{\nu}\|_1 \to 0$).
* **CR3BP Low-Thrust Transfer Optimization**:
  - Full 6-DoF variational state-transition coupling with the rotating three-body potential Hessian.
  - Real-world electric propulsion modeling (NASA NEXT-C Ion Thruster, Busek BHT-600 Hall Thruster, Chemical Bipropellant).
  - Generates discrete, operational burn schedule segments (burn durations, thrust levels in mN, $\Delta v$, fuel mass consumption in kg).
* **Local Native Daemon (`crates/sbm_server`)**:
  - High-performance Axum REST daemon running locally on `http://127.0.0.1:8080`.
  - Exposes `/api/v1/transfer/optimize`, `/api/v1/export/oem`, and orbit correction endpoints.
  - Direct export to standard **CCSDS OEM v2.0** ephemerides and **RFC 4180 CSV** burn schedules.
* **Non-Mathematician Operator Cockpit UI**:
  - Embedded Mission Planning Wizard inside [`cr3bp_deep_space_visualizer.html`](file:///Users/tomohiko/work/sbm_visualizer/cr3bp_deep_space_visualizer.html).
  - Evaluates and ranks **Top 3 Flight Candidates**:
    1. *Candidate A (Recommended: Min-Fuel SCvx)*: Maximum payload delivery fraction.
    2. *Candidate B (Balanced Transit)*: $-15\%$ flight duration with moderate propellant trade-off.
    3. *Candidate C (Rapid Response)*: High-thrust insertion for time-critical operations.
  - Live 3D trajectory rendering into Three.js scene with synchronized camera framing and telemetry inspector.

---

### High-Precision Symplectic Astronomy & N-Body Visualizer (`sbm_core::nbody`)

The suite features an astrophysical, double-precision N-body simulation and celestial visualization engine designed for symplectic, long-term orbital integration with Hamiltonian conservation:

* **High-Order Symplectic & Predictor-Corrector Integrators**:
  - **Yoshida 4th-Order Symplectic Composition**: Reversible 3-stage composition of Velocity Verlet ($w_1 = \frac{1}{2 - 2^{1/3}}$, $w_0 = -\frac{2^{1/3}}{2 - 2^{1/3}}$) preserving phase-space volume and keeping energy error strictly bounded ($\Delta E / E_0 < 10^{-7}$) over arbitrary orbital revolutions without artificial numerical damping.
  - **Yoshida 6th-Order Symplectic Integrator**: 7-stage symmetric composition (Solution A) achieving double-precision conservation for resonant systems.
  - **Hermite 4th-Order Predictor-Corrector**: Utilizes analytical pairwise gravitational jerk $\mathbf{\dot{a}}$ for close-encounter and high-eccentricity multi-body dynamics.
* **Relativistic Corrections & Resonance Extraction**:
  - **1PN General Relativity (Einstein-Infeld-Hoffmann)**: Direct post-Newtonian acceleration modeling reproducing Mercury's secular perihelion advance ($\Delta \varpi \approx 42.98'' / \text{century}$).
  - **Osculating Keplerian Elements**: Real-time extraction of instantaneous semimajor axis $a$, eccentricity $e$, inclination $i$, longitude of ascending node $\Omega$, argument of periapsis $\omega$, true anomaly $\nu$, and orbital period $P$.
  - **Resonance Arguments**: Live tracking of resonant angles (e.g. Jovian Laplace resonance $\phi_L = \lambda_{\text{Io}} - 3\lambda_{\text{Europa}} + 2\lambda_{\text{Ganymede}} \approx 180^\circ$).
* **High-Fidelity Gravitational Force & Perturbation Analysis**:
  - **Pairwise Force Vectors & Tethers**: Real-time rendering of inter-body gravitational vectors with dynamic **Perturbation Magnification** ($1\times \to 50,000\times$, default $5,000\times$) enabling visualization of planetary perturbations (e.g. Jupiter's secular pull on Earth, $1.46 \times 10^{18}\text{ N}$) alongside dominant solar pulls ($3.66 \times 10^{22}\text{ N}$).
  - **3D Deformable Spacetime Curvature Potential Funnels**: Dynamic logarithmic gravitational potential wells ($z = -S \ln(1 + |\Phi|/\Phi_0)$) with vertex-colored depth gradient shading reflecting local gravitational binding energy.
  - **Concentric Gravitational Spheres (Chebotarev 1964 & Domingos et al. 2006)**: Precise 3D translucent wireframes for planetary Hill spheres ($r_H = a(1-e)\sqrt[3]{m/(3M_\odot)}$), Laplace spheres of influence ($r_s = a(m/M)^{2/5}$), and prograde satellite stability limits ($r_{\text{crit}} \approx 0.4895 r_H (1 - 1.0305 e_{\text{sat}} - 0.2738 e_{\text{planet}})$).
  - **Gravitational Tidal Tensor (arXiv:1608.03366)**: Analytical evaluation of the spatial gravity gradient matrix $\mathbf{T}_{ab} = \partial g_a / \partial x_b$, trace-free property $\text{Tr}(\mathbf{T}) = 0$, and analytical Cardano cubic eigenvalues ($\lambda_1 > 0$ radial stretching strain, $\lambda_2, \lambda_3 < 0$ lateral compression).
  - **"Gravitational Tug-of-War" Telemetry HUD & Earth-Moon Barycenter (EMB)**: Solves and explains the apparent paradox that $F_{\odot \to \leftmoon} \approx 4.36 \times 10^{20}\text{ N}$ is $2.20\times$ larger than $F_{\oplus \to \leftmoon} \approx 1.98 \times 10^{20}\text{ N}$, demonstrating stable satellite binding within Earth's Hill sphere and tracking the EMB located $\approx 4,671\text{ km}$ from Earth's center (inside Earth's mantle).
* **Canonical Astronomical Presets**:
  - *Sun-Venus-Earth-Moon-Mars-Jupiter (Gravitational Force Focus)*: High-fidelity system configured for gravitational interaction, tidal tensor, and Hill sphere exploration.
  - *Solar System (JPL J2000)*: Sun + 8 planets + Pluto + Moon with zero net barycentric linear momentum.
  - *Jovian Laplace Resonance (4:2:1)*: Exact orbital frequency coupling of Io, Europa, and Ganymede.
  - *Figure-8 Three-Body Choreography*: Chenciner & Montgomery (2000) zero-angular-momentum periodic solution.
  - *Sun-Jupiter Trojans*: $L_4$ (Greek camp) and $L_5$ (Trojan camp) 1:1 resonant co-orbital asteroids.
  - *Relativistic Mercury Precession*: 1PN general relativistic precession simulation.
  - *Pythagorean Three-Body Problem*: Classic Burrau (1913) / Szebehely (1967) chaotic benchmark.
  - *TRAPPIST-1 Resonant Chain*: 7-planet compact resonant multi-exoplanetary system.

---

### Rust Library & Standalone CLI Engine

The repository provides a modular, multi-crate Rust architecture:

* **`crates/sbm_core`**: A standalone, zero-dependency Rust library implementing:
  - **NASA EVOLVE 4.0 Standard Breakup Model**: Complete collision and explosion physics engine.
  - **AutoOrbit (KDD 2026)**: Hierarchical satellite orbit prediction with FNO and Gaussian Variational Equations.
  - **CR3BP Propagator & Deep Space Engine (AAS 20-459)**: High-precision Circular Restricted Three-Body Problem propagator reproducing STK Astrogator, exact $L_1\text{--}L_5$ libration points, periodic Lyapunov/Halo/NRHO orbits, frame transformations (CBI $\leftrightarrow$ Rotating), and multi-body low-energy transfers ($\Delta v \approx 23.2\text{ m/s}$).
  - **SCvx Trajectory Optimizer (Mao 2016, Malyuta 2021)**: Zero-external-dependency convexified trajectory optimization.
  - **High-Order Symplectic N-Body Engine (Yoshida 1990, Aarseth 1999)**: 4th/6th order symplectic integrators, 1PN General Relativity, and osculating element extraction.
* **`crates/sbm_server`**: Local-first native daemon serving the Web Cockpits and providing high-speed astrodynamics and N-body APIs on `127.0.0.1:8080`.
* **`engine_cli`**: Standalone CLI application consuming `sbm_core` to run high-speed Monte Carlo breakup simulations, print telemetry metrics, and export debris clouds as JSON.

```bash
# Launch the local Astrodynamics Server & Web Cockpit:
cargo run -p sbm_server

# Run the Rust CLI engine:
cargo run --bin sbm_simple_engine --release

# Run the CR3BP Deep Space & Multi-Body Transfer Demo:
cargo run --example cr3bp_deep_space_demo

# Run the full test suite (66 tests across all crates):
cargo test --workspace

# Run clippy with zero warnings & zero print macros:
cargo clippy --workspace --all-targets -- -D warnings -D clippy::print_stdout -D clippy::print_stderr

# Run Python paper reproduction test suite:
python3 -m unittest tests/test_autoorbit_reproduction.py
```

---

## Interactive 3D Visualizers

* **NASA EVOLVE 4.0 Collision Visualizer**: Open [`index.html`](file:///Users/tomohiko/work/sbm_visualizer/index.html) in any modern web browser.
* **CR3BP Deep Space & Multi-Body Transfer Cockpit**: Open [`cr3bp_deep_space_visualizer.html`](file:///Users/tomohiko/work/sbm_visualizer/cr3bp_deep_space_visualizer.html) or navigate to `http://127.0.0.1:8080/` with the daemon running. Click **Mission Planning Wizard** to run live SCvx optimizations, inspect burn schedules, and download CCSDS OEM v2.0 files.
* **High-Precision Astronomy & Symplectic N-Body Cockpit**: Open [`astronomy_visualizer.html`](file:///Users/tomohiko/work/sbm_visualizer/astronomy_visualizer.html) or navigate to `http://127.0.0.1:8080/astronomy`. Real-time Hamiltonian energy drift gauges ($\Delta E / E_0$), resonance meters, dual scale modes (1:1 physical AU/km vs Perceptual), osculating Keplerian orbital telemetry, and velocity vectors.

---

## Scientific Reference Papers & Verification Matrix

> For the exhaustive, formula-by-formula verification matrix mapping every equation, tolerance bound, and automated test suite, see [**`docs/physics_verification_and_literature_matrix.md`**](docs/physics_verification_and_literature_matrix.md).

All foundational papers have been verified via automated reproduction test suites:

### Celestial Mechanics, Symplectic Astronomy & Gravitational Fields
1. **Yoshida, H. (1990)**: *Construction of higher order symplectic integrators*, Physics Letters A, 150(5–7), pp. 262–268 ([DOI: 10.1016/0375-9601(90)90092-3](https://doi.org/10.1016/0375-9601(90)90092-3)) — Verified in `test_yoshida4_energy_conservation_figure8` and `test_yoshida6_precision_higher_than_yoshida4`.
2. **Aarseth, S. J. (1999) & Makino, J. (1992)**: *Hermite Integrators with Ahmad-Cohen Scheme*, PASJ 44, pp. 141–151; PASP 111, pp. 1333–1346 — Verified in `test_leapfrog_and_hermite4_steps`.
3. **Einstein, A., Infeld, L., & Hoffmann, B. (1938) / Will, C. M. (2014)**: *The Confrontation between General Relativity and Experiment*, Living Rev. Relativ., 17(4) — Verified in `test_relativistic_mercury_precession_computation`.
4. **Chebotarev, G. A. (1964)**: *Gravitational Spheres of the Major Planets, Moon and Sun*, Soviet Astronomy, 7(5), pp. 618–622 — **Reproduced Table 1** in `test_chebotarev_1964_gravitational_spheres_reproduction`.
5. **Domingos, P. D., Winter, O. C., & Yokoyama, T. (2006)**: *Stable orbits for satellites of extrasolar planets*, MNRAS, 373(3), pp. 1227–1234 ([DOI: 10.1111/j.1365-2966.2006.11104.x](https://doi.org/10.1111/j.1365-2966.2006.11104.x)) — Verified in `test_domingos_2006_hill_sphere_satellite_stability`.
6. **Poisson, E., & Will, C. M. (2014) / arXiv:1608.03366**: *Gravity: Newtonian, Post-Newtonian, Relativistic*, Cambridge Univ. Press — Verified in `test_tidal_tensor_trace_free_and_eigenvalues_arxiv_1608_03366`.
7. **Curtis, H. D. (2014) & Vallado, D. A. (2013)**: *Orbital Mechanics for Engineering Students*, Elsevier; *Fundamentals of Astrodynamics* — Verified in `test_vis_viva_orbital_speed_and_normalized_kinetic` and `test_osculating_elements_circular_and_inclined`.
8. **Meeus, J. (1998) & Seidelmann, P. K. (1992)**: *Astronomical Algorithms*, Willmann-Bell; *Explanatory Supplement* — Verified in `test_shadow_cone_geometry_and_eclipse_evaluation`.
9. **Laplace, P.-S. (1799) & Peale, S. J. (1976)**: *Orbital resonances in the solar system*, Ann. Rev. Astron. Astrophys., 14, pp. 215–246 — Verified in `test_laplace_resonance_4_2_1`.
10. **Lagrange, J.-L. (1772)**: *Essai sur le problème des trois corps*, Prix de l'Acad. R. Sci. Paris — Verified in `test_sun_jupiter_trojan_lagrange_angles`.
11. **Chenciner, A., & Montgomery, R. (2000)**: *A remarkable periodic solution of the three-body problem in the case of equal masses*, Annals of Mathematics, 152(3), pp. 881–901 — Verified in Figure-8 zero-momentum choreographic conservation.
12. **Standish, E. M. (1995) / Folkner et al. (2014)**: *JPL Planetary and Lunar Ephemerides (DE403/DE430)*, NASA JPL — Verified in `test_jpl_solar_system_earth_orbital_period_and_radius`.
13. **Burrau, C. (1913) / Szebehely & Peters (1967)**: *A New Family of Periodic Orbits in the Restricted Three-Body Problem*, Astronomical Journal — Verified in Pythagorean three-body benchmark.

### Astrodynamics, Trajectory Optimization & Debris Modeling
14. **Short, Haapala, Bosanac (2020)**: *STK Astrogator CR3BP & Low-Energy Transfers*, AAS 20-459 ([`papers/2020_AAS_ShoHaaBos.pdf`](file:///Users/tomohiko/work/sbm_visualizer/papers/2020_AAS_ShoHaaBos.pdf)) — Verified in `crates/sbm_core/tests/cr3bp_test.rs`.
15. **Mao, Szmuk, Açıkmeşe (2016)**: *Successive Convexification of Non-Convex Optimal Control Problems with State Constraints*, arXiv:1608.05133 ([`papers/2016_arXiv_Mao_Szmuk_Acikmese_SCvx.pdf`](file:///Users/tomohiko/work/sbm_visualizer/papers/2016_arXiv_Mao_Szmuk_Acikmese_SCvx.pdf)) — Verified in `crates/sbm_core/src/scvx/drag_benchmark.rs`.
16. **Malyuta et al. (2021)**: *Advances in Trajectory Optimization for Aerospace Systems: A Tutorial on Successive Convexification*, IEEE CSM, arXiv:2106.09125 ([`papers/2021_arXiv_Malyuta_SCvx_Tutorial.pdf`](file:///Users/tomohiko/work/sbm_visualizer/papers/2021_arXiv_Malyuta_SCvx_Tutorial.pdf)).
17. **Clohessy, W. H., & Wiltshire, R. S. (1960)**: *Terminal Guidance System for Satellite Rendezvous*, Journal of the Aerospace Sciences, 27(9), pp. 653–658 — Verified in `crates/sbm_core/tests/rpo_test.rs`.
18. **Johnson, N. L., Krisko, P. H., Liou, J.-C., & Anz-Meador, P. D. (2001)**: *NASA's new breakup model of EVOLVE 4.0*, Adv. Space Res., 28(9), pp. 1377–1384 ([DOI: 10.1016/S0273-1177(01)00423-5](https://doi.org/10.1016/S0273-1177(01)00423-5)) — Verified in `crates/sbm_core/tests/library_api_test.rs`.
19. **Zhang et al. (2026)**: *AutoOrbit: Physics-Informed Satellite Orbit Prediction*, ACM KDD 2026 ([`papers/3770855.3818960.pdf`](file:///Users/tomohiko/work/sbm_visualizer/papers/3770855.3818960.pdf)) — Verified in `tests/autoorbit_test.rs`.

---

## License
MIT License
