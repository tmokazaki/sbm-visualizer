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
* **Follow Camera Tracking**:
  * Smooth camera lerp tracking locked onto any selected fragment as it disperses along its orbital trajectory.
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

### Rust Library & Standalone CLI Engine

The repository provides a modular, multi-crate Rust architecture:

* **`crates/sbm_core`**: A standalone, zero-dependency Rust library implementing the complete NASA EVOLVE 4.0 Standard Breakup Model physics engine, mathematical formulations, continuous cross-sectional area calculations, stochastic A/M sampling, and Clohessy-Wiltshire state transitions.
* **`engine_cli`**: Standalone CLI application consuming `sbm_core` to run high-speed Monte Carlo breakup simulations, print telemetry metrics, and export debris clouds as JSON.

```bash
# Run the Rust CLI engine:
cargo run -p sbm_simple_engine --release

# Run the test suite:
cargo test --workspace

# Run clippy with zero warnings:
cargo clippy --workspace --all-targets -- -D warnings
```

---

## License
MIT License
