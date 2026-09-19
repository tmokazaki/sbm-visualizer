# ADR-001: Alignment of NASA Standard Breakup Model with EVOLVE 4.0 Specification

## Status
**Proposed**

## Context
The repository [`sbm_visualizer`](file:///Users/tomohiko/work/sbm_visualizer) provides a 3D orbital debris visualization suite and a Rust CLI engine (`engine_cli`) simulating hypervelocity satellite collisions in Low Earth Orbit (LEO).

The system currently relies on a simplified empirical prototype in [`engine_cli/src/main.rs`](file:///Users/tomohiko/work/sbm_visualizer/engine_cli/src/main.rs) and [`index.html`](file:///Users/tomohiko/work/sbm_visualizer/index.html#L1119-L1393). While downstream orbital mechanics (Clohessy-Wiltshire state transition, Gabbard diagram geometry, and 3D spatial covariance ellipsoids) are mathematically established, a quantitative evaluation against the definitive paper:
> **Johnson, N. L., Krisko, P. H., Liou, J.-C., & Anz-Meador, P. D. (2001)**. *"NASA's new breakup model of EVOLVE 4.0"*, *Advances in Space Research*, 28(9), pp. 1377–1384 ([DOI: 10.1016/S0273-1177(01)00423-5](https://doi.org/10.1016/S0273-1177(01)00423-5))

revealed severe divergence in the upstream fragment generation pipeline:
1. **Absence of Area-to-Mass ($A/M$) Distributions (Eqs. 5, 6, 7)**: Area-to-mass ratio is the central variable of EVOLVE 4.0, governing both fragment aerodynamic decay and ejection velocities. It is completely absent from the current codebase.
2. **Incorrect Conditioning and Variance for Ejection Velocity $\Delta v$ (Eqs. 11, 12)**: EVOLVE 4.0 specifies that ejection velocity follows a log-normal distribution conditioned on $\chi = \log_{10}(A/M)$ with $\sigma = 0.40$. The current code conditions on $\log_{10}(L_c)$ with $\sigma = 0.16$, compressing velocity variance and distorting the Gabbard diagram wings.
3. **Absence of Cross-Sectional Area $A_x$ and Inverted Mass Derivation (Eqs. 8, 9, 10)**: In EVOLVE 4.0, individual fragment mass is derived via $M = A_x / (A/M)$. The current code uses an unphysical heuristic $m \propto L_c^{2.3}$ normalized to an aggregate destroyed mass.
4. **Incorrect Effective Mass for Non-Catastrophic Collisions**: The paper specifies $M = M_{\text{proj}} \cdot v_{\text{imp, km/s}}$ for Eq. (4), whereas the code applies an ad-hoc heuristic $M_2 (1 + (v_{\text{imp}} / 2)^{1.5})$, producing up to $+43.6\%$ error.
5. **Decoupled Fragment Count & Truncation**: Fragment count is fixed to a slider value ($N = 1500$) rather than calculated from $N(L_c \ge L_{\min})$, under-representing small fragments by $97\%$ at $1\text{ cm}$ and truncating fragments $> 22\text{ cm}$.
6. **Missing Explosion Mode (Eqs. 2, 3, 11)**: Only collision mode is supported.

## Decision Drivers
- **Physical Fidelity**: The visualizer and CLI must accurately reproduce peer-reviewed experimental and observational debris characteristics from SSN radar tracking, Haystack observations, and SOCIT ground tests.
- **Astrodynamics Integrity**: Accurate Gabbard diagram branches, re-entry predictions, and ballistic coefficients ($B^* \propto A/M$) require authentic $A/M$ and $\Delta v$ distributions.
- **Performance & Usability**: Must preserve real-time 60 FPS WebGL rendering in the browser while accommodating physically realistic sample counts.
- **Reproducibility**: Source implementation must match the equations and empirical curves published in Johnson et al. (2001) within statistical sampling tolerances.

## Considered Options

### Option 1: Retain Current Heuristic Model
- **Pros**: Low complexity, fixed fragment counts, simple UI sliders.
- **Cons**: Scientifically indefensible, fails peer-reviewed benchmarks, produces erroneous velocity and density fields.

### Option 2: Full EVOLVE 4.0 Standard Breakup Model (Recommended)
Implement the exact multi-stage stochastic pipeline formulated in Johnson et al. (2001):
1. **Disruption Classification**: Specific kinetic energy $E_p \ge 40\text{ kJ/kg}$ (catastrophic) vs $E_p < 40\text{ kJ/kg}$ (cratering).
2. **Effective Mass**: $M = M_1 + M_2$ (catastrophic) or $M = M_{\text{smaller}} \cdot v_{\text{imp, km/s}}$ (non-catastrophic).
3. **Cumulative Power-Law Count**: $N(L_c \ge L_{\min}) = 0.1 M^{0.75} L_{\min}^{-1.71}$ (collision) or $S \cdot 6 L_{\min}^{-1.6}$ (explosion).
4. **Characteristic Length ($L_c$)**: Sampled via inverse-transform Pareto over $[L_{\min}, L_{\max}]$.
5. **Area-to-Mass Ratio ($A/M$)**: Bimodal Gaussian mixture for $L_c \ge 11\text{ cm}$ (Eq. 5 for Rocket Bodies, Eq. 6 for Spacecraft), SOCIT Gaussian for $L_c < 8\text{ cm}$ (Eq. 7), and linear bridging between $8\text{ cm}$ and $11\text{ cm}$.
6. **Cross-Sectional Area ($A_x$)**: Piecewise power-law (Eq. 8 for $L_c < 1.67\text{ mm}$, Eq. 9 for $L_c \ge 1.67\text{ mm}$).
7. **Mass Derivation**: $M_i = A_x / (A/M)_i$ (Eq. 10).
8. **Ejection Velocity ($\Delta v$)**: Log-normal conditioned on $\chi = \log_{10}(A/M)$ (Eq. 12 for collision: $\mu = 0.9\chi + 2.9, \sigma = 0.40$; Eq. 11 for explosion: $\mu = 0.2\chi + 1.85, \sigma = 0.40$).

### Option 3: Modern Revision (NASA SBM 2011 / Krisko 2011)
- Incorporates 2011 updates to velocity dispersion and SOCIT fits.
- **Cons**: Beyond the scope of the EVOLVE 4.0 paper provided under `papers/`. Can be incorporated as a future configuration mode.

## Decision
Adopt **Option 2 (Full EVOLVE 4.0 Standard Breakup Model)** as the authoritative standard for both [`engine_cli/src/main.rs`](file:///Users/tomohiko/work/sbm_visualizer/engine_cli/src/main.rs) and [`index.html`](file:///Users/tomohiko/work/sbm_visualizer/index.html).

To accommodate browser GPU limits while maintaining physical realism:
- Allow the user to specify minimum cutoff size $L_{\min}$ (e.g. $5\text{ cm}$ or $10\text{ cm}$) or a visual sample budget, while displaying the true theoretical total fragment count $N_{\text{total}} = N(L_c \ge 1\text{ mm})$ in the telemetry panel.
- Implement discrete mass conservation handling: sample fragments sequentially or apply a physical mass balancing pass that preserves the sampled $A/M$ distribution.

## Consequences

### Positive
- **Valid Gabbard Diagrams**: Fragments correctly reproduce the characteristic "X-wing" dispersion with authentic apogee/perigee spread driven by $\sigma = 0.40$.
- **Ballistic Coefficient Support**: Enables future atmospheric drag perturbation and realistic orbital decay lifetime modeling using authentic $A/M$.
- **Scientific Defensibility**: Every mathematical expression and parameter directly cites Johnson et al. (2001) or NASA TM revisions.
- **Unified Engine**: Rust CLI and WebGL visualizer share identical mathematical formulations.

### Negative / Risks
- Generating large fragment populations ($N > 10,000$ for small cutoffs) requires sampling management or representative sub-sampling on lower-end client devices.
- Sampling the Gaussian mixture distribution requires slightly more floating-point operations per fragment (negligible on modern CPUs/GPUs).

---

## References
1. Johnson, N. L., Krisko, P. H., Liou, J.-C., & Anz-Meador, P. D. (2001). *"NASA's new breakup model of EVOLVE 4.0"*, *Advances in Space Research*, 28(9), pp. 1377–1384.
2. Reynolds, R. C. et al. (1998). *"NASA Standard Breakup Model 1998 Revision"*, LMSMSS-32532, Lockheed Martin.
3. Krisko, P. H. (2011). *"The Revised NASA Standard Breakup Model"*, *Astrodynamics 2011*, Vol. 142, pp. 2487–2498.
