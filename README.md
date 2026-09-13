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
  * **Bidirectional Cross-Highlighting**: Hovering any fragment point on the Gabbard canvas displays a full telemetry tooltip and illuminates a glowing yellow spotlight ring on the corresponding fragment in 3D space.
  * Draggable floating window with minimize/restore and header toggles.
* **100% Offline & Standalone Execution**:
  * Zero external network or CDN dependencies. Runs completely self-contained in modern web browsers.

---

## Gallery

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
* `t`: Initial time in seconds (e.g., `-5`, `0`, `15`, `450`).
* `mode`: Timeline range (`zoom` for $-10\text{s}$ to $+30\text{s}$, `full` for $-10\text{s}$ to $+900\text{s}$).
* `play`: Initial playback state (`1` or `0`).
* `color`: Particle coloring mode (`contour`, `density`, `smooth`, `origin`).
* `gabbard`: Gabbard window visibility (`1` or `0`).
* `volumes`: 3D hazard envelopes visibility (`1` or `0`).
* `hover`: Highlight fragment ID on start (e.g. `12`).

---

### Running the Offline Rust CLI Engine (Optional)
The repository also includes an optional standalone Rust CLI tool (`engine_cli`) that generates NASA SBM fragment clouds in JSON format:
```bash
cd engine_cli
cargo run --release
```
This generates `fragments_output.json` containing the stochastic fragment distributions.

---

## License
MIT License
