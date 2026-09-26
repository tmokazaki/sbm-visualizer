# High-Precision N-Body Gravitational Dynamics & Astronomical Systems: Mathematical & Validation Specification

This document provides the foundational mathematical formulations, numerical integration proofs, and peer-reviewed literature benchmarks for the high-precision N-body gravitational dynamics engine in `sbm_core::nbody`.

---

## 1. Foundational Scientific Literature & Peer-Reviewed References

The implementation is verified against seven seminal peer-reviewed papers and astronomical standards:

| Benchmark Domain | Primary Scientific Paper / Reference | Key Phenomenon & Invariants | Automated Test Suite |
| :--- | :--- | :--- | :--- |
| **Symplectic Composition** | **Yoshida (1990)**<br>*Construction of higher order symplectic integrators*<br>Phys. Lett. A, 150(5–7), pp. 262–268 | Order 4 & 6 composition coefficients ($w_0, w_1, w_2, w_3$), zero secular energy drift | `test_yoshida4_energy_conservation_figure8`<br>`test_yoshida6_precision_higher_than_yoshida4` |
| **Astrophysical Direct N-Body** | **Aarseth (1999) & Makino (1992)**<br>*Hermite Integrators with Ahmad-Cohen Scheme*<br>PASJ 44, pp. 141–151; PASP 111, pp. 1333–1346 | Gravitational jerk $\dot{\mathbf{a}}_i$, $P(EC)^n$ 4th-order predictor-corrector | `test_leapfrog_and_hermite4_steps` |
| **Relativistic Mechanics** | **Einstein (1915) & Will (2014)**<br>*The Confrontation between General Relativity and Experiment*<br>Living Rev. Relativ. 17(4) | 1PN Post-Newtonian Schwarzschild acceleration, Mercury perihelion advance ($42.98''/\text{cy}$) | `test_relativistic_mercury_precession_computation` |
| **Orbital Resonances** | **Laplace (1784) & Murray & Dermott (1999)**<br>*Solar System Dynamics* (Cambridge University Press)<br>Chapter 8: Resonances in the Solar System | Jovian Laplace 4:2:1 mean-motion resonance, resonant angle libration $\phi_L \approx 180^\circ$ | `test_laplace_resonance_4_2_1` |
| **Periodic Choreographies** | **Chenciner & Montgomery (2000)**<br>*A remarkable periodic solution of the three-body problem*<br>Annals of Mathematics, 152(3), pp. 881–901 | Exact figure-8 zero-angular-momentum 3-body solution, $T \approx 6.3259$ | `test_yoshida4_energy_conservation_figure8` |
| **JPL Planetary Ephemerides** | **Standish (1995) & Folkner et al. (2014)**<br>*JPL Planetary and Lunar Ephemerides (DE403/DE430)*<br>JPL IOM 314.10-127; IAU 2015 Cartographic Report | J2000.0 Cartesian state vectors, Earth $1\text{ AU}$ & $365.25\text{ d}$, momentum balance | `test_jpl_solar_system_earth_orbital_period_and_radius`<br>`test_linear_and_angular_momentum_conservation` |
| **Gravitational Spheres** | **Chebotarev (1964)**<br>*Gravitational spheres of the major planets, Moon and Sun*<br>Soviet Astronomy, 7(5), pp. 618–622 | Sphere of Attraction ($r_a = a\sqrt{m/M}$), Laplace SOI ($r_s = a(m/M)^{2/5}$), Hill sphere ($r_H = a(1-e)\sqrt[3]{m/(3M)}$) | `test_chebotarev_1964_gravitational_spheres_reproduction` |
| **Satellite Stability Inside Hill Sphere** | **Domingos, Winter, & Yokoyama (2006)**<br>*Stable satellites around extrasolar giant planets*<br>MNRAS, 373(3), pp. 1227–1234 | Critical prograde satellite stability radius $r_{\text{crit}} \approx 0.4895 r_H (1 - 1.0305 e_{\text{sat}} - 0.2738 e_{\text{planet}})$ | `test_domingos_2006_hill_sphere_satellite_stability` |
| **Gravity Gradient & Tidal Tensor** | **arXiv:1608.03366**<br>*Gravity Gradient Tensor Eigendecomposition* | Trace-free $\text{Tr}(\mathbf{T}) = 0$, analytical Cardano cubic eigenvalues, radial stretching ($\lambda_1 > 0$), orthogonal compression ($\lambda_2, \lambda_3 < 0$) | `test_tidal_tensor_trace_free_and_eigenvalues_arxiv_1608_03366` |

---

## 2. Governing Equations of Motion & Conservation Laws

### 2.1 Pairwise Newtonian Gravitation
For $N$ celestial bodies with masses $m_i$ at positions $\mathbf{r}_i \in \mathbb{R}^3$, the Newtonian acceleration is:
$$\mathbf{a}_{i,\text{Newton}} = \sum_{j \ne i}^N \frac{G m_j (\mathbf{r}_j - \mathbf{r}_i)}{\left(\|\mathbf{r}_j - \mathbf{r}_i\|^2 + \epsilon^2\right)^{3/2}}$$
where $G = 6.67430 \times 10^{-11} \text{ m}^3\text{kg}^{-1}\text{s}^{-2}$ (CODATA 2018) and $\epsilon \ge 0$ is the Plummer softening parameter.

### 2.2 First-Order Post-Newtonian (1PN) General Relativistic Correction
To capture general relativistic precession without requiring full numerical relativity grids, the Einstein-Infeld-Hoffmann (EIH) / Schwarzschild 1PN acceleration term is evaluated:
$$\mathbf{a}_{ij}^{\text{1PN}} = \frac{G m_j}{c^2 r_{ij}^3} \left[ \left( 4 \frac{G m_j}{r_{ij}} - \|\mathbf{v}_i\|^2 \right) \mathbf{r}_{ij} + 4 (\mathbf{r}_{ij} \cdot \mathbf{v}_i) \mathbf{v}_i \right]$$
where $c = 299,792,458 \text{ m/s}$ and $\mathbf{r}_{ij} = \mathbf{r}_j - \mathbf{r}_i$.

For Mercury on an orbit with semi-major axis $a = 0.3871\text{ AU}$ and eccentricity $e = 0.2056$, this produces the exact secular perihelion advance:
$$\dot{\varpi}_{\text{GR}} = \frac{6 \pi G M_\odot}{c^2 a (1 - e^2) P} = 42.98'' / \text{century}$$

### 2.3 Hamiltonian Invariants & Conservation Laws
The system Hamiltonian is:
$$H(\mathbf{q}, \mathbf{p}) = T(\mathbf{p}) + V(\mathbf{q}) = \sum_{i=1}^N \frac{\|\mathbf{p}_i\|^2}{2 m_i} - \sum_{i < j} \frac{G m_i m_j}{\sqrt{\|\mathbf{r}_i - \mathbf{r}_j\|^2 + \epsilon^2}}$$

In any closed gravitational system, three exact invariants must be conserved:
1. **Total Mechanical Energy**:
   $$E = T + V = \text{const}, \quad \Delta E / E_0 = \frac{|E(t) - E(0)|}{|E(0)|}$$
2. **Total Linear Momentum & Barycenter**:
   $$\mathbf{P} = \sum_{i=1}^N m_i \mathbf{v}_i = \text{const}, \quad \mathbf{R}_{\text{cm}} = \frac{\sum m_i \mathbf{r}_i}{\sum m_i}$$
3. **Total Angular Momentum**:
   $$\mathbf{L} = \sum_{i=1}^N m_i (\mathbf{r}_i \times \mathbf{v}_i) = \text{const}$$

---

## 3. Gravitational Spheres & Satellite Orbital Stability

### 3.1 Chebotarev (1964) Gravitational Sphere Formulations
Chebotarev (*Soviet Astronomy*, 1964) derived and tabulated three concentric boundaries defining a secondary body's gravitational dominance relative to a primary body:

1. **Sphere of Attraction ($r_a$)**: The boundary where the secondary body's gravitational force equals the primary's gravitational force:
   $$r_a = a \sqrt{\frac{m}{M}}$$
   For Earth: $r_a \approx 259,000 \text{ km}$. For Jupiter: $r_a \approx 24.1 \times 10^6 \text{ km}$.
2. **Laplace Sphere of Influence ($r_s$ / SOI)**: The locus where the primary's perturbation on a secondary-centric orbit equals the secondary's perturbation on a primary-centric orbit:
   $$r_s = a \left(\frac{m}{M}\right)^{2/5}$$
   For Earth: $r_s \approx 924,000 \text{ km}$. For Jupiter: $r_s \approx 48.2 \times 10^6 \text{ km}$.
3. **Hill Sphere ($r_H$)**: The boundary of the zero-velocity surface enclosing the inner collinear Lagrange points $L_1$ and $L_2$:
   $$r_H = a (1 - e) \sqrt[3]{\frac{m}{3 M}}$$
   For Earth: $r_H \approx 1.50 \times 10^6 \text{ km} \approx 0.010\text{ AU}$. For Jupiter: $r_H \approx 53.1 \times 10^6 \text{ km} \approx 0.355\text{ AU}$.

### 3.2 Domingos, Winter, & Yokoyama (2006) Critical Satellite Stability
While the Hill sphere defines the theoretical maximum extent of gravitational binding, Domingos et al. (2006) showed through numerical integration that quasi-satellite orbits near $r_H$ are chaotic and prone to solar stripping. For prograde circular-to-eccentric orbits, long-term orbital stability requires:
$$r_{\text{crit}} \approx 0.4895\, r_H\, (1 - 1.0305\, e_{\text{sat}} - 0.2738\, e_{\text{planet}})$$

For the Earth-Moon system:
- $e_{\text{sat}} = 0.0549$, $e_{\text{planet}} = 0.0167$
- $r_{\text{crit}} \approx 0.4895 \times 1.50\times 10^6 \times 0.9389 \approx 689,000 \text{ km}$
- Moon's actual semi-major axis: $a_{\leftmoon} \approx 384,400 \text{ km}$
- Safety ratio: $a_{\leftmoon} / r_{\text{crit}} \approx 0.558$ (deeply within the stable zone).

---

## 4. Gravitational Tidal Tensor & Eigendecomposition (arXiv:1608.03366)

The spatial gravity gradient (tidal) tensor $\mathbf{T} \in \mathbb{R}^{3 \times 3}$ is the Jacobian of the gravitational field:
$$T_{ab} = \frac{\partial g_a}{\partial x_b} = \sum_{j \ne i} \frac{G m_j}{\|\mathbf{r}_j - \mathbf{r}_i\|^5} \left[ 3 (\mathbf{r}_j - \mathbf{r}_i)_a (\mathbf{r}_j - \mathbf{r}_i)_b - \|\mathbf{r}_j - \mathbf{r}_i\|^2 \delta_{ab} \right]$$

### 4.1 Invariants & Spectral Properties
1. **Trace-Free Invariant**: Since Laplace's equation holds in vacuum ($\nabla \cdot \mathbf{g} = 0$):
   $$\text{Tr}(\mathbf{T}) = T_{11} + T_{22} + T_{33} = 0$$
2. **Eigenvalue Solution**: The characteristic equation $\det(\mathbf{T} - \lambda \mathbf{I}) = 0$ simplifies to $\lambda^3 - q \lambda - r = 0$, solved analytically via Cardano's trigonometric formulation:
   $$q = \frac{1}{6} \sum_{a,b} T_{ab}^2, \quad r = \frac{1}{2} \det(\mathbf{T}), \quad \theta = \frac{1}{3} \arccos\left(\frac{r}{q^{3/2}}\right)$$
   $$\lambda_1 = 2 \sqrt{q} \cos(\theta) > 0 \quad \text{(radial tidal stretching)}$$
   $$\lambda_2 = 2 \sqrt{q} \cos\left(\theta - \frac{2\pi}{3}\right) < 0, \quad \lambda_3 = 2 \sqrt{q} \cos\left(\theta + \frac{2\pi}{3}\right) < 0 \quad \text{(orthogonal lateral compression)}$$
   $$\lambda_1 + \lambda_2 + \lambda_3 = 0$$

---

## 5. The Gravitational Tug-of-War: Earth-Moon-Sun Dynamics

A fundamental paradox in astrodynamics is that the Sun exerts more gravitational force on the Moon than the Earth does:
- Solar force on Moon: $F_{\odot \to \leftmoon} = \frac{G M_\odot m_{\leftmoon}}{r_{\odot\leftmoon}^2} \approx 4.36 \times 10^{20} \text{ N}$
- Terrestrial force on Moon: $F_{\oplus \to \leftmoon} = \frac{G M_\oplus m_{\leftmoon}}{r_{\oplus\leftmoon}^2} \approx 1.98 \times 10^{20} \text{ N}$
- Force ratio:
  $$\frac{F_{\odot \to \leftmoon}}{F_{\oplus \to \leftmoon}} \approx 2.20$$

### Why does the Moon orbit Earth?
1. **Equivalence Principle & Free-Fall**: The Moon and Earth both accelerate in the Sun's gravitational potential at nearly identical rates ($\mathbf{a}_{\odot} \approx 5.93 \times 10^{-3} \text{ m/s}^2$). The differential solar acceleration across the Earth-Moon distance is merely the solar tidal force:
   $$\Delta a_\odot \approx \frac{2 G M_\odot}{r^3} d_{\oplus\leftmoon} \approx 3.05 \times 10^{-5} \text{ m/s}^2$$
   which is orders of magnitude smaller than the Moon's acceleration towards Earth:
   $$a_{\oplus \to \leftmoon} = \frac{G M_\oplus}{d_{\oplus\leftmoon}^2} \approx 2.70 \times 10^{-3} \text{ m/s}^2 \gg \Delta a_\odot$$
2. **Earth-Moon Barycenter (EMB)**: Both bodies orbit their mutual center of mass:
   $$\Delta r_{\text{EMB}} = \frac{m_{\leftmoon}}{M_\oplus + m_{\leftmoon}} d_{\oplus\leftmoon} \approx 4,671 \text{ km}$$
   Because $4,671 \text{ km} < R_\oplus \approx 6,371 \text{ km}$, the Earth-Moon Barycenter lies inside Earth's mantle (~$1,700 \text{ km}$ below the crust).

---

## 6. High-Order Symplectic Integration Architecture (Yoshida 1990)

Standard non-symplectic Runge-Kutta schemes introduce artificial dissipation or numerical secular energy growth ($E(t) \propto t$). Symplectic integrators preserve the differential 2-form $\omega = \sum dq_i \wedge dp_i$, guaranteeing that the numerical solution is the exact trajectory of a nearby shadow Hamiltonian $\tilde{H} = H + h^p H_p$.

### 6.1 Yoshida 4th-Order Symplectic Scheme
Yoshida (1990) established that a symmetric composition of 2nd-order leapfrog steps yields a 4th-order integrator:
$$w_1 = \frac{1}{2 - 2^{1/3}} \approx 1.3512071919596578, \quad w_0 = -\frac{2^{1/3}}{2 - 2^{1/3}} = 1 - 2w_1 \approx -1.7024143839193153$$

The stage sub-step coefficients are:
$$c_1 = c_4 = \frac{w_1}{2}, \quad c_2 = c_3 = \frac{w_0 + w_1}{2}$$
$$d_1 = d_3 = w_1, \quad d_2 = w_0, \quad d_4 = 0$$

For each sub-step $k \in \{1, 2, 3, 4\}$:
1. Position drift: $\mathbf{r}_i \leftarrow \mathbf{r}_i + c_k h \mathbf{v}_i$
2. Acceleration evaluation: $\mathbf{a}_i \leftarrow \mathbf{a}_i(\{\mathbf{r}_j\})$
3. Velocity kick: $\mathbf{v}_i \leftarrow \mathbf{v}_i + d_k h \mathbf{a}_i$

### 6.2 Yoshida 6th-Order Symplectic Scheme (Solution A)
For ultra-high stability over millennia, Yoshida's 7-stage 6th-order composition (Solution A) applies weights:
$$w_1 = -1.17767998417887, \quad w_2 = 0.235573213359357, \quad w_3 = 0.784513610477560$$
$$w_0 = 1 - 2(w_1 + w_2 + w_3) \approx 1.315186320683906$$
Sequence: $\{w_3, w_2, w_1, w_0, w_1, w_2, w_3\}$, achieving global truncation error $\mathcal{O}(h^6)$.

---

## 7. Multi-Body Resonances: Jovian Laplace Resonance (4:2:1)

The Jovian system exhibits the classic Laplace 3-body mean-motion resonance between Io (1), Europa (2), and Ganymede (3):
$$n_1 - 3 n_2 + 2 n_3 = 0$$
where $n_i = 2\pi / P_i$ are the mean daily orbital motions ($n_1 \approx 203.49^\circ/\text{d}$, $n_2 \approx 101.37^\circ/\text{d}$, $n_3 \approx 50.32^\circ/\text{d}$).

The Laplace resonant argument is:
$$\phi_L = \lambda_1 - 3 \lambda_2 + 2 \lambda_3$$
where $\lambda_i = \Omega_i + \omega_i + M_i$ is the true mean longitude.

In gravitational equilibrium, mutual perturbations cause $\phi_L$ to strictly librate about $180^\circ$ with small amplitude ($< 0.1^\circ$):
$$\langle \phi_L \rangle = 180^\circ$$

---

## 8. Verification Protocol & Test Results

The test suite in [`crates/sbm_core/tests/nbody_test.rs`](file:///Users/tomohiko/work/sbm_visualizer/crates/sbm_core/tests/nbody_test.rs) validates all formulations against published benchmarks:

```
running 14 tests
test test_chebotarev_1964_gravitational_spheres_reproduction ... ok
test test_domingos_2006_hill_sphere_satellite_stability ... ok
test test_earth_moon_sun_gravitational_tug_of_war ... ok
test test_jpl_solar_system_earth_orbital_period_and_radius ... ok
test test_laplace_resonance_4_2_1 ... ok
test test_leapfrog_and_hermite4_steps ... ok
test test_linear_and_angular_momentum_conservation ... ok
test test_osculating_elements_circular_and_inclined ... ok
test test_relativistic_mercury_precession_computation ... ok
test test_sun_jupiter_trojan_lagrange_angles ... ok
test test_tidal_tensor_trace_free_and_eigenvalues_arxiv_1608_03366 ... ok
test test_trajectory_propagation_output_snapshots ... ok
test test_yoshida4_energy_conservation_figure8 ... ok
test test_yoshida6_precision_higher_than_yoshida4 ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

