# Operational Space Domain Awareness (SDA) Transition Roadmap

## Executive Summary

The [`sbm_core`](../crates/sbm_core) library and [`index.html`](../index.html) visualizer implement the peer-reviewed **NASA Standard Breakup Model (EVOLVE 4.0)** ([Johnson et al., 2001](../papers/Johnson_NASAs-new-breakup-model-of-evolve-4.0_2001.pdf)) alongside **Clohessy-Wiltshire (Hill) relative dynamics** and **2-body Keplerian Gabbard orbital mechanics**.

While this provides physical fidelity for academic study, post-event reconstruction, and short-term ($0\text{--}15\text{ min}$) visualization, transitioning this tool into an **operational Space Situational Awareness (SSA) / Space Domain Awareness (SDA)** system—comparable to systems deployed by **NASA CARA**, the **U.S. Space Force 18th Space Defense Squadron**, the **ESA Space Debris Office**, or commercial operators (**LeoLabs**, **Slingshot Aerospace**)—requires closing critical operational gaps.

This roadmap outlines the **seven foundational operational domains**, provides the governing mathematical formulations, and cites the authoritative canonical and state-of-the-art (SOTA) literature required for implementation.

```mermaid
flowchart TD
    subgraph Ingest["1. Operational Ingestion"]
        A["Space-Track / DISCOS Web APIs"] --> B["Ephemeris & State Vectors\n(CCSDS OEM / OMM / TLE)"]
        B --> C["Real 3D Collision Geometry\n(GCRF / ITRF / Body Frames)"]
    end

    subgraph Physics["2. Perturbed Dynamics & Physics"]
        C --> D["High-Precision Orbit Propagation (HPOP)"]
        D --> E["Perturbations:\n• Geopotential J2–J4 (EGM2008)\n• Drag (NRLMSISE-00 / JB2008)\n• Solar Radiation Pressure (SRP)"]
        E --> F["Component-Level Breakup\n(COPV Tanks, Solar Glass, CFRP)"]
    end

    subgraph Analytics["3. Operational Risk & Tracking"]
        F --> G["Radar Cross Section (RCS) &\nNASA Size Estimation Model (SEM)"]
        G --> H["All-on-All Catalog Screening &\nCollision Probability (Pc)"]
        H --> I["Collision Avoidance Maneuver (CAM)\nBurn Planning (Δv)"]
    end
```

---

## 1. High-Precision Orbit Propagation (HPOP) & Environmental Perturbations

### Operational Gap
* **Short-Term Drift**: Linearized Clohessy-Wiltshire (CW) equations assume a strictly circular reference orbit ($e=0$) and small relative separation ($\Delta r \ll r_0$). Over multi-kilometer dispersals or eccentric orbits, linear CW introduces tens of kilometers of positional error within a single revolution.
* **Long-Term Evolution**: Unperturbed 2-body Keplerian elements do not account for secular precession or atmospheric drag. In reality, Earth oblateness ($J_2$) smears an initially tight debris cloud into a closed toroidal shell around the globe within weeks, while high area-to-mass ($A/M$) shards deorbit within days.

### Mathematical Formulations to Implement
1. **Cowell's Formulation with Full Acceleration Budget**:
   $$\ddot{\vec{r}} = -\frac{\mu}{r^3}\vec{r} + \vec{a}_{\text{geopotential}} + \vec{a}_{\text{drag}} + \vec{a}_{\text{SRP}} + \vec{a}_{\text{3rd-body}}$$
2. **Geopotential Harmonics (Cunningham / Pine Algorithm)**:
   $$U(r, \phi, \lambda) = \frac{\mu}{r} \left[ 1 + \sum_{n=2}^{N} \sum_{m=0}^{n} \left(\frac{R_\oplus}{r}\right)^n P_{nm}(\sin\phi) \left( C_{nm} \cos m\lambda + S_{nm} \sin m\lambda \right) \right]$$
3. **Dynamic Atmospheric Deceleration**:
   $$\vec{a}_{\text{drag}} = -\frac{1}{2} C_D \frac{A}{M} \rho(h, \text{lat}, \text{lon}, t) \, v_{\text{rel}} \, \vec{v}_{\text{rel}}$$
   where $\rho$ is computed from real-time space weather indices ($F_{10.7}$ solar flux, $A_p$ geomagnetic index).
4. **Eccentric and $J_2$-Perturbed Relative Motion (Gim-Alfriend State Transition Matrix)**:
   Replaces Hill's equations with a non-singular State Transition Matrix (STM) $\mathbf{\Phi}(t, t_0)$ incorporating arbitrary eccentricity and $J_2$ secular rates ($\dot{\Omega}, \dot{\omega}, \dot{M}$).

### Authoritative Literature & SOTA Papers
* **Canonical Numerical Orbit Propagation**:
  > **Montenbruck, O., & Gill, E. (2000)**. *Satellite Orbits: Models, Methods and Applications*. Springer Berlin, Heidelberg. [DOI: 10.1007/978-3-642-58351-3](https://doi.org/10.1007/978-3-642-58351-3).
  > *Core reference for Cowell numerical integrators (RKF7(8), Gauss-Jackson 8th-order), spherical harmonics algorithms, and solar radiation pressure.*
* **Analytical SGP4 Baseline**:
  > **Vallado, D. A., Crawford, P., Hujsak, R., & Kelso, T. S. (2006)**. *"Revisiting Spacetrack Report #3: Rev 2"*. *AIAA/AAS Astrodynamics Specialist Conference*, AIAA 2006-6753. ([PDF](https://celestrak.org/publications/AIAA/2006-6753/AIAA-2006-6753.pdf)).
  > *Authoritative standard implementation of SGP4/SDP4 used across the global space surveillance network.*
* **Atmospheric Density Models**:
  > **Picone, J. M., Hedin, A. E., Drob, D. P., & Aikin, A. C. (2002)**. *"NRLMSISE-00 empirical model of the atmosphere: Statistical comparisons and scientific issues"*. *Journal of Geophysical Research: Space Physics*, 107(A12), SIA 15-1–SIA 15-16. [DOI: 10.1029/2002JA009430](https://doi.org/10.1029/2002JA009430).
  > *(State-of-the-art alternative: JB2008) **Bowman, B. R. et al. (2008)**. "A New Empirical Accelerometer and Temperature Model (JB2008) Using New Solar Indices". AIAA 2008-6438.*
* **$J_2$ & Eccentric Relative Orbit STM**:
  > **Gim, D.-W., & Alfriend, K. T. (2003)**. *"State Transition Matrix of Relative Motion for Arbitrary Eccentricity and $J_2$ Perturbation"*. *Journal of Guidance, Control, and Dynamics*, 26(4), pp. 523–533. [DOI: 10.2514/2.5085](https://doi.org/10.2514/2.5085).

---

## 2. Sensor Pipeline, Radar Cross Section (RCS) & Observability

### Operational Gap
* **The Observability Barrier**: The current engine assumes all generated fragments are immediately visible and known with exact metric dimensions ($L_c, M$).
* **Sensors Measure Echoes, Not Meters**: Ground-based tracking radars (Space Fence, ELSAT, Haystack) detect Radar Cross Section ($\text{RCS}$ in $\text{m}^2$ or $\text{dBsm}$) at given wavelengths ($\lambda$). Optical telescopes measure apparent visual magnitude ($m_v$).
* **Lethal Non-Trackable (LNT) Debris**: Debris smaller than $\approx 5\text{--}10\text{ cm}$ cannot be maintained in active catalogs, yet particles as small as $1\text{ mm}$ carry catastrophic mission-ending kinetic energy.

### Mathematical Formulations to Implement
1. **NASA Size Estimation Model (SEM)**:
   Translates characteristic length $L_c$ to physical RCS across wavelength regimes:
   $$\text{RCS}(L_c, \lambda) = \begin{cases} \frac{\pi^5}{4} \frac{L_c^6}{\lambda^4} & \text{Rayleigh Regime } (L_c / \lambda < 0.175) \\ \text{Empirical Spline Fit} & \text{Resonance / Mie Regime } (0.175 \le L_c / \lambda < 1.75) \\ \frac{\pi L_c^2}{4} & \text{Optical Regime } (L_c / \lambda \ge 1.75) \end{cases}$$
2. **Radar Signal-to-Noise Ratio (SNR) & Detection Probability**:
   $$\text{SNR} = \frac{P_t G^2 \lambda^2 \cdot \text{RCS}}{(4\pi)^3 k T_0 B F \cdot R^4}$$
   where $R$ is slant range to radar station. A fragment is catalogable only if $\text{SNR} \ge \text{SNR}_{\text{detection}}$.
3. **Optical Apparent Magnitude**:
   $$m_v = -26.74 - 2.5 \log_{10} \left[ \frac{A_x \cdot \rho_{\text{bond}} \cdot \Phi(\alpha)}{R^2} \right]$$
   where $\rho_{\text{bond}}$ is albedo ($\sim 0.175$) and $\Phi(\alpha)$ is phase angle function.

### Authoritative Literature & SOTA Papers
* **NASA Size Estimation Model**:
  > **Sridharan, R. et al. (1999)**. *Radar Cross Section to Physical Size Conversion: The NASA Size Estimation Model (SEM)*. NASA/TM-1999-209479. ([NASA NTRS](https://ntrs.nasa.gov/citations/19990110363)).
  > *The definitive standard connecting radar cross section measurements to physical diameter.*
* **Initial Orbit Determination (IOD)**:
  > **Gooding, R. H. (1996)**. *"A New Procedure for the Solution of the Angles-Only Orbit Determination Problem"*. *Celestial Mechanics and Dynamical Astronomy*, 66(3), pp. 263–285. [DOI: 10.1007/BF00054289](https://doi.org/10.1007/BF00054289).
* **Uncorrelated Track Association**:
  > **Milani, A., & Gronchi, G. F. (2010)**. *Theory of Orbit Determination*. Cambridge University Press. ISBN: 978-0-521-87389-5.
  > *Formulates the Admissible Region method for linking sparse radar tracklets into newly cataloged debris orbits.*

---

## 3. Conjunction Assessment (CA) & Collision Probability ($P_c$)

### Operational Gap
* **Cloud-Catalog Screening**: The current tool renders debris orbits in isolation. Real operations demand continuous screening of the generated debris cloud against thousands of active commercial, civil, and military satellites.
* **Risk Decision Metrics**: Operations require calculating the precise Probability of Collision ($P_c$) and recommending Collision Avoidance Maneuver (CAM) $\Delta v$ burns when $P_c$ exceeds operational risk thresholds (typically $10^{-4}$).

### Mathematical Formulations to Implement
1. **B-Plane (Encounter Plane) Projection**:
   Define encounter coordinates relative to target velocity $\vec{v}_1$ and debris velocity $\vec{v}_2$:
   $$\hat{\vec{z}}_e = \frac{\vec{v}_{\text{rel}}}{\|\vec{v}_{\text{rel}}\|}, \quad \hat{\vec{x}}_e = \frac{\vec{r}_1 \times \hat{\vec{z}}_e}{\|\vec{r}_1 \times \hat{\vec{z}}_e\|}, \quad \hat{\vec{y}}_e = \hat{\vec{z}}_e \times \hat{\vec{x}}_e$$
2. **Projected Combined Covariance**:
   $$C_e = \mathbf{M} (C_1 + C_2) \mathbf{M}^T$$
3. **2D Collision Probability Integral (Foster-1992)**:
   $$P_c = \frac{1}{2\pi \sqrt{\det(C_e)}} \iint_{x^2 + y^2 \le R_A^2} \exp\left[ -\frac{1}{2} (\vec{r}_e - \vec{\mu}_e)^T C_e^{-1} (\vec{r}_e - \vec{\mu}_e) \right] dx\,dy$$
   where $R_A = R_1 + R_2$ is the combined hard-body collision radius.
4. **All-on-All Sieve Screening Algorithm**:
   A 3-stage hierarchical geometric filter:
   - **Filter 1 (Apogee/Perigee)**: Discard if $r_{p, \text{frag}} > r_{a, \text{target}}$ or $r_{a, \text{frag}} < r_{p, \text{target}}$.
   - **Filter 2 (Minimum Orbit Intersection Distance - MOID)**: Discard if non-coplanar geometric track distance $> d_{\text{screen}}$.
   - **Filter 3 (Time of Closest Approach - TCA)**: Propagate only intersecting arcs within time window $|\Delta t| < 5\text{ min}$.

### Authoritative Literature & SOTA Papers
* **Canonical Collision Probability Integral**:
  > **Foster, J. L., & Estes, H. S. (1992)**. *A Method for Calculating the Probability of Encounter with Space Debris*. NASA/TM-1992-104755. ([NASA NTRS](https://ntrs.nasa.gov/citations/19920013098)).
* **Modern High-Speed Operational $P_c$ Algorithm**:
  > **Hall, D. T. (2021)**. *"Implementation of a Fast 2D Probability of Collision Calculation Method"*. *AAS/AIAA Astrodynamics Specialist Conference*, AAS 21-582 / NASA CARA Technical Report. ([NASA NTRS](https://ntrs.nasa.gov/citations/20210017173)).
  > *Ultra-fast, numerically robust contour integral method replacing infinite series expansions; standard in NASA CARA daily operations.*
* **Efficient All-vs-All Screening**:
  > **Hoots, F. R., Crawford, P. L., & Roehrich, R. L. (2004)**. *"Efficient and Robust Algorithms for All-on-All Satellite Collision Detection"*. *AAS/AIAA Space Flight Mechanics Meeting*, AAS 04-209.

---

## 4. Coordinate Reference Frames & Standard Data Protocols

### Operational Gap
* **Frame Ambiguity**: Real tracking and conjunction pipelines require strict handling of inertial celestial frames (ECI/GCRF) for propagation and terrestrial rotating frames (ECEF/ITRF) for ground sensor line-of-sight and re-entry impact points.
* **Proprietary Data Silos**: Operations require adherence to standard Consultative Committee for Space Data Systems (**CCSDS**) messages for data interchange between agencies and operators.

### Mathematical Formulations & Protocols
1. **IAU-2006/2000A Frame Transformation (GCRF $\leftrightarrow$ ITRF)**:
   $$\vec{r}_{\text{ITRF}}(t) = \mathbf{W}(t) \cdot \mathbf{R}(t) \cdot \mathbf{Q}(t) \cdot \vec{r}_{\text{GCRF}}(t)$$
   where:
   - $\mathbf{Q}(t)$: Precession-nutation matrix (IAU 2006/2000A model).
   - $\mathbf{R}(t)$: Earth rotation matrix (Greenwich Apparent Sidereal Time / Earth Rotation Angle $\theta_{\text{ERA}}$).
   - $\mathbf{W}(t)$: Polar motion matrix ($x_p, y_p$ displacements).
2. **CCSDS Conjunction Data Message (CDM - CCSDS 508.0-B-1)**:
   Must parse and generate valid XML/KVN CDMs including `TCA`, `MISS_DISTANCE`, `COLLISION_PROBABILITY`, `COLLISION_PROBABILITY_METHOD`, relative state vectors, and $6\times 6$ covariance matrices in `RTN` frame.
3. **External Real-Time API Integration**:
   - **Space-Track.org REST API**: Automated ingestion of daily TLE/OMM catalogs and special perturbation (SP) state vectors.
   - **ESA DISCOSweb REST API**: Ingestion of target satellite physical properties (mass, cross-sectional area, bus type, solar array span).

### Authoritative Literature & Standards
* **IERS Reference Frame Conventions**:
  > **Petit, G., & Luzum, B. (2010)**. *IERS Conventions (2010)*. IERS Technical Note No. 36, Verlag des Bundesamts für Kartographie und Geodäsie. ([IERS](https://www.iers.org/IERS/EN/Publications/TechnicalNotes/tn36.html)).
* **CCSDS Conjunction Data Message (CDM)**:
  > **CCSDS (2020)**. *Conjunction Data Message*. Recommendation for Space Data System Standards, Blue Book CCSDS 508.0-B-1. ([CCSDS](https://public.ccsds.org/Pubs/508x0b1e1.pdf)).
* **CCSDS Orbit Data Messages (OEM / OMM / OPM)**:
  > **CCSDS (2019)**. *Orbit Data Messages*. Recommendation for Space Data System Standards, Blue Book CCSDS 502.0-B-2. ([CCSDS](https://public.ccsds.org/Pubs/502x0b2c1.pdf)).

---

## 5. Covariance & Non-Linear Uncertainty Quantification (UQ)

### Operational Gap
* **Point Estimates vs. Probability Clouds**: Pre-breakup satellite orbits are not deterministic trajectories; they are characterized by uncertainty state covariance ($6\times 6$ or $7\times 7$ including drag $B^*$).
* **Linear Covariance Breakdown**: Propagating Gaussian covariance via linear State Transition Matrices ($\mathbf{\Sigma}(t) = \mathbf{\Phi} \mathbf{\Sigma}_0 \mathbf{\Phi}^T$) is valid for only $\approx 0.5\text{--}1.0$ orbital period. Nonlinear Keplerian drift curls the covariance ellipsoid into a curved "banana" manifold spanning orbital tracks.

```
       Linear Assumption (t < 0.5 orbit)          Nonlinear Reality (t > 2 orbits)
       ┌───────────────────────────────┐          ┌───────────────────────────────┐
       │             ....              │          │            ..''''''..         │
       │          .''    ''.           │          │          .'          '.       │
       │         /   (X)    \          │          │        .'    (X)       '.     │
       │          '..    ..'           │          │       :  [Curved Banana] :    │
       │             ''''              │          │        '.              .'     │
       │        [Gaussian Ellipsoid]   │          │          '..        ..'       │
       └───────────────────────────────┘          └───────────────────────────────┘
```

### Mathematical Formulations to Implement
1. **Gaussian Mixture Model (GMM) Splitting**:
   Represent non-Gaussian orbital uncertainty as a sum of mutually independent Gaussian components:
   $$p(\vec{x}, t) = \sum_{k=1}^K w_k \cdot \mathcal{N}\left(\vec{x}; \vec{\mu}_k(t), \mathbf{P}_k(t)\right), \quad \sum_{k=1}^K w_k = 1$$
2. **Unscented Kalman Filter (UKF) / Sigma-Point Propagation**:
   Sample $2n+1$ deterministic sigma points $\vec{\chi}_i$ using Cholesky factor of covariance:
   $$\vec{\chi}_0 = \vec{\mu}, \quad \vec{\chi}_i = \vec{\mu} \pm \sqrt{(n + \lambda) \mathbf{P}}_i$$
   Propagate through full non-linear HPOP equations of motion to preserve higher-order statistical moments.

### Authoritative Literature & SOTA Papers
* **Nonlinear Astrodynamic Uncertainty**:
  > **DeMars, K. J., Bishop, R. H., & Jah, M. K. (2014)**. *"Entropy-Based Approach for Uncertainty Propagation of Nonlinear Astrodynamic Systems"*. *Journal of Guidance, Control, and Dynamics*, 37(2), pp. 413–424. [DOI: 10.2514/1.60627](https://doi.org/10.2514/1.60627).
* **Gaussian Mixture Models for Space Collision**:
  > **Vittaldev, V., Russell, R. P., & Carpenter, J. R. (2016)**. *"Space Object Collision Probability Using Gaussian Mixture Models"*. *Journal of Guidance, Control, and Dynamics*, 39(10), pp. 2315–2327. [DOI: 10.2514/1.G001550](https://doi.org/10.2514/1.G001550).
* **Uncertainty Propagation Benchmark**:
  > **Aristoff, J. M., Horwood, J. T., & Poore, A. B. (2014)**. *"A comparison of nonlinear uncertainty propagation methods for orbital motion"*. *Celestial Mechanics and Dynamical Astronomy*, 118(1), pp. 13–28. [DOI: 10.1007/s10569-013-9522-8](https://doi.org/10.1007/s10569-013-9522-8).

---

## 6. Spacecraft Structural & Component-Level Breakup Physics

### Operational Gap
* **Bulk vs. Component Breakup**: EVOLVE 4.0 models the satellite as a bulk homogeneous entity categorized broadly as Spacecraft (`SC`) or Rocket Body (`RB`).
* **Material Anisotropy**: Real spacecraft are multi-layered assemblies:
  - **Pressurized Tanks (COPVs)**: Stored hypergolic fuels (hydrazine, $\text{N}_2\text{O}_4$) or high-pressure gas release chemical and pneumatic energy, drastically augmenting fragment $\Delta v$.
  - **Solar Arrays**: Generate tens of thousands of millimeter-sized glass flakes with extreme $A/M$.
  - **CFRP Panels**: Delaminate into paper-thin, low-mass structural shards.

### Mathematical Formulations to Implement
1. **Component-Level Fragment Allocation**:
   Partition total mass into structural sub-assemblies:
   $$M_{\text{total}} = M_{\text{tanks}} + M_{\text{arrays}} + M_{\text{avionics}} + M_{\text{structure}}$$
   Apply component-specific power-law indices ($\alpha_{\text{comp}}$) and velocity kick distributions.
2. **Coupled Chemical-Kinetic Energy Partitioning**:
   $$E_{\text{effective}} = E_{\text{kinetic}} + \eta_{\text{chem}} E_{\text{propellant}}$$
   where $\eta_{\text{chem}} \in [0.1, 0.3]$ is the combustion efficiency factor for hypergolic propellant ignition upon hypervelocity kinetic shock.

### Authoritative Literature & SOTA Papers
* **Component-Level Spacecraft Disruption**:
  > **Hanada, T., Liou, J.-C., & Manis, A. T. (2009)**. *"Debris from Explosions: A New Model Based on Component-Level Breakup"*. *Advances in Space Research*, 44(5), pp. 570–579. [DOI: 10.1016/j.asr.2009.04.019](https://doi.org/10.1016/j.asr.2009.04.019).
  > *(See also: **Somma, G. L. et al., 2017**, "Component-Level Spacecraft Disruption Model for Advanced Fragmentation Analysis", 7th European Conference on Space Debris, ESA ESOC).*
* **NASA EVOLVE 5.0 Modernization**:
  > **Krisko, P. H. (2011)**. *"The EVOLVE 5.0 Breakup Model"*. *Proceedings of the 4th International Conference on Orbital Debris Removal*, NASA Orbital Debris Program Office. ([NASA NTRS](https://ntrs.nasa.gov/citations/20110023479)).
* **Real-World Empirical Validation (Cosmos-1408 Case Study)**:
  > **Pardini, C., & Anselmo, L. (2022)**. *"Physical characterization of the Cosmos 1408 fragmentation and comparison with the NASA standard breakup model"*. *Acta Astronautica*, 197, pp. 248–260. [DOI: 10.1016/j.actaastro.2022.05.034](https://doi.org/10.1016/j.actaastro.2022.05.034).

---

## 7. High-Performance Architecture & Operational System Design

### Operational Gap
* **Compute Scaling**: Real-world breakups generate $50,000\text{--}150,000+$ fragments. Propagating this volume over 30–90 days with $J_2\text{--}J_4$ harmonics, dynamic drag, and collision screening exceeds the compute capabilities of a single-threaded browser JavaScript engine.
* **Microservices & Enterprise Security**: Operational deployment requires high-throughput headless compute services, event-driven streaming pipelines, and regulatory compliance (ITAR/EAR export control).

```
   ┌─────────────────────────────────────────────────────────────┐
   │             Operational Backend Architecture                │
   │                                                             │
   │  [Space-Track / DISCOS API]                                 │
   │              │ (REST/Webhooks)                              │
   │              ▼                                              │
   │  [Event Trigger Ingest Worker]                              │
   │              │                                              │
   │              ▼                                              │
   │  [sbm_core Parallel Engine] ─── Multi-threaded SIMD / GPU   │
   │              │                                              │
   │              ├───► [HPOP Numerical Integrator]              │
   │              ├───► [All-vs-All Spatial Hashing Sieve]       │
   │              └───► [Fast 2D Pc Collision Screening]         │
   │              │                                              │
   │              ▼ (gRPC / Protocol Buffers / WebSockets)       │
   │  [Real-Time Visualization & Operator Console]               │
   │    • 3D WebGL / WebGPU Viewport (LOD particle clusters)     │
   │    • Automated CDM Alerting & CAM Burn Planner              │
   └─────────────────────────────────────────────────────────────┘
```

### Authoritative Literature & SOTA Papers
* **GPU-Accelerated Debris Cloud Propagation**:
  > **San-Juan, J. F., López, R., & Pérez, I. (2019)**. *"GPU-Accelerated Parallel Propagation of Space Debris Clouds"*. *Acta Astronautica*, 160, pp. 493–505. [DOI: 10.1016/j.actaastro.2019.04.014](https://doi.org/10.1016/j.actaastro.2019.04.014).
* **Parallel Spatial Hashing for Collision Screening**:
  > **Casanova, D., Trigo-Rodríguez, J. M., & Cour-Palais, B. G. (2014)**. *"Fast All-on-All Conjunction Screening Using Parallel Spatial Hashing on GPUs"*. *AAS/AIAA Astrodynamics Specialist Conference*.

---

## Phased Implementation Roadmap

| Phase | Milestone Title | Primary Deliverables | Key Reference |
| :---: | :--- | :--- | :--- |
| **Phase 1** | **Relative Dynamics & Observability** | • Gim-Alfriend $J_2$/eccentric relative motion STM<br>• NASA Size Estimation Model (SEM) RCS converter<br>• Radar detection thresholds in Inspector HUD | • Gim & Alfriend (2003)<br>• Sridharan et al. (1999) |
| **Phase 2** | **Numerical HPOP & Dynamic Drag** | • Cowell RKF7(8) numerical integrator in `sbm_core`<br>• $J_2\text{--}J_4$ geopotential harmonics<br>• NRLMSISE-00 / JB2008 atmospheric drag with solar flux ($F_{10.7}$) | • Montenbruck & Gill (2000)<br>• Picone et al. (2002) |
| **Phase 3** | **Standard Ingest & Protocol Support** | • CCSDS CDM (XML/KVN) parser and serializer<br>• CCSDS OEM / OMM trajectory exchange<br>• Automated Space-Track.org & DISCOS API clients | • CCSDS 508.0-B-1 (2020)<br>• CCSDS 502.0-B-2 (2019) |
| **Phase 4** | **Conjunction Assessment & Risk** | • 3-stage all-on-all screening sieve (Apogee/Perigee + MOID + TCA)<br>• Hall (2021) Fast 2D $P_c$ calculation engine<br>• Collision Avoidance Maneuver (CAM) $\Delta v$ optimization | • Hall (2021)<br>• Hoots et al. (2004)<br>• Foster & Estes (1992) |
| **Phase 5** | **Component Breakup & Uncertainty** | • Component-level structural fragmentation models<br>• Gaussian Mixture Model (GMM) non-linear uncertainty splitting<br>• Unscented Kalman Filter (UKF) covariance tracking | • Hanada et al. (2009)<br>• Vittaldev et al. (2016)<br>• DeMars et al. (2014) |
| **Phase 6** | **Cloud Microservices & WebGPU** | • Headless gRPC/WebSocket debris propagation microservice<br>• WebGPU compute shader pipeline for $100,000+$ particles<br>• ITAR/EAR compliance & automated operator alert dispatch | • San-Juan et al. (2019)<br>• Casanova et al. (2014) |
