use serde::{Deserialize, Serialize};

/// Standard CODATA 2018 Newtonian gravitational constant $G$ in $\text{m}^3 \text{kg}^{-1} \text{s}^{-2}$.
pub const G_STANDARD: f64 = 6.67430e-11;

/// Speed of light in vacuum $c$ in $\text{m/s}$ (exact standard).
pub const SPEED_OF_LIGHT: f64 = 299792458.0;

/// Standard Astronomical Unit in meters ($1\text{ AU}$).
pub const ASTRONOMICAL_UNIT_M: f64 = 149597870700.0;

/// One Julian year in seconds ($365.25 \times 86400$).
pub const JULIAN_YEAR_S: f64 = 31557600.0;

/// One Julian day in seconds.
pub const JULIAN_DAY_S: f64 = 86400.0;

/// Represents an individual celestial body in an N-body system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CelestialBody {
    /// Unique identifier for the body.
    pub id: usize,
    /// Human-readable name (e.g., "Sun", "Earth", "Io").
    pub name: String,
    /// Physical mass in kilograms ($\text{kg}$).
    pub mass_kg: f64,
    /// Volumetric or equatorial radius in kilometers ($\text{km}$).
    pub radius_km: f64,
    /// Cartesian position vector $[x, y, z]$ in meters ($\text{m}$).
    pub position_m: [f64; 3],
    /// Cartesian velocity vector $[v_x, v_y, v_z]$ in meters per second ($\text{m/s}$).
    pub velocity_mps: [f64; 3],
    /// UI rendering hex color code (e.g., "#38bdf8").
    pub color_hex: String,
    /// If true, this body experiences no external gravitational acceleration (fixed anchor).
    pub is_fixed: bool,
}

impl CelestialBody {
    /// Creates a new celestial body with physical properties.
    pub fn new(
        id: usize,
        name: impl Into<String>,
        mass_kg: f64,
        radius_km: f64,
        position_m: [f64; 3],
        velocity_mps: [f64; 3],
        color_hex: impl Into<String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            mass_kg,
            radius_km,
            position_m,
            velocity_mps,
            color_hex: color_hex.into(),
            is_fixed: false,
        }
    }

    /// Speed magnitude in meters per second.
    pub fn speed(&self) -> f64 {
        (self.velocity_mps[0].powi(2)
            + self.velocity_mps[1].powi(2)
            + self.velocity_mps[2].powi(2))
        .sqrt()
    }

    /// Distance from the coordinate origin in meters.
    pub fn distance_from_origin(&self) -> f64 {
        (self.position_m[0].powi(2)
            + self.position_m[1].powi(2)
            + self.position_m[2].powi(2))
        .sqrt()
    }
}

/// Numerical integrator algorithm selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum IntegratorType {
    /// 4th-Order Symplectic Integrator (Yoshida 1990).
    /// Preserves phase space volume and bounded energy errors without secular drift.
    #[default]
    Yoshida4th,
    /// 6th-Order Symplectic Integrator (Yoshida 1990, Solution A).
    /// Ultra-high order symplectic precision for long-duration orbital mechanics.
    Yoshida6th,
    /// Adaptive Dormand-Prince 8(5,3) embedded explicit Runge-Kutta.
    /// High-order with local error estimation, ideal for close encounters.
    DormandPrince853,
    /// 2nd-Order Velocity Verlet / Leapfrog.
    /// Fast symplectic baseline.
    Leapfrog,
    /// 4th-Order Hermite Predictor-Corrector using jerk $\dot{\mathbf{a}}$.
    /// Standard astrophysics N-body algorithm (Aarseth).
    Hermite4th,
}

/// Conservation laws and Hamiltonian invariants metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConservationMetrics {
    /// Total system kinetic energy $T = \sum \frac{1}{2} m_i v_i^2$ in Joules.
    pub kinetic_energy_j: f64,
    /// Total system gravitational potential energy $V = -\sum_{i<j} \frac{G m_i m_j}{r_{ij}}$ in Joules.
    pub potential_energy_j: f64,
    /// Total system mechanical energy $E = T + V$ in Joules.
    pub total_energy_j: f64,
    /// Relative mechanical energy drift $\Delta E / E_0 = |E(t) - E(0)| / |E(0)|$.
    pub relative_energy_error: f64,
    /// Total linear momentum vector $\mathbf{P} = \sum m_i \mathbf{v}_i$ in $\text{kg}\cdot\text{m/s}$.
    pub linear_momentum_kg_mps: [f64; 3],
    /// Magnitude of linear momentum $\|\mathbf{P}\|$.
    pub linear_momentum_magnitude: f64,
    /// Total angular momentum vector $\mathbf{L} = \sum m_i (\mathbf{r}_i \times \mathbf{v}_i)$ in $\text{kg}\cdot\text{m}^2/\text{s}$.
    pub angular_momentum_kg_m2_s: [f64; 3],
    /// Magnitude of angular momentum $\|\mathbf{L}\|$.
    pub angular_momentum_magnitude: f64,
    /// Relative angular momentum drift $\Delta L / L_0$.
    pub relative_angular_momentum_error: f64,
    /// Center of mass (Barycenter) position $\mathbf{R}_{\text{cm}}$ in meters.
    pub barycenter_position_m: [f64; 3],
    /// Center of mass velocity $\mathbf{V}_{\text{cm}}$ in meters per second.
    pub barycenter_velocity_mps: [f64; 3],
}

/// Classical Keplerian orbital elements extracted from Cartesian state vectors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OsculatingElements {
    /// Semi-major axis $a$ in meters.
    pub semi_major_axis_m: f64,
    /// Orbital eccentricity $e$.
    pub eccentricity: f64,
    /// Orbital inclination $i$ in radians.
    pub inclination_rad: f64,
    /// Longitude of ascending node $\Omega$ (RAAN) in radians.
    pub raan_rad: f64,
    /// Argument of periapsis $\omega$ in radians.
    pub arg_periapsis_rad: f64,
    /// True anomaly $\nu$ in radians.
    pub true_anomaly_rad: f64,
    /// Mean anomaly $M$ in radians.
    pub mean_anomaly_rad: f64,
    /// Orbital period $P = 2\pi \sqrt{a^3 / \mu}$ in seconds.
    pub period_s: f64,
    /// Periapsis distance $r_p = a(1 - e)$ in meters.
    pub periapsis_m: f64,
    /// Apoapsis distance $r_a = a(1 + e)$ in meters.
    pub apoapsis_m: f64,
}

/// Resonance tracking metrics for multi-body orbital resonances.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResonanceMetrics {
    /// Identifier or name of the resonance (e.g., "Laplace 4:2:1", "Jupiter-Trojan L4").
    pub name: String,
    /// Canonical ratio descriptor (e.g., "4:2:1", "1:1", "3:2").
    pub ratio_desc: String,
    /// Resonant angle in degrees (e.g. $\phi_L = \lambda_1 - 3\lambda_2 + 2\lambda_3$ librating around $180^\circ$).
    pub resonant_angle_deg: f64,
    /// Ratio of orbital frequencies between inner and outer components.
    pub frequency_ratio: f64,
}

/// High-precision N-body gravitational system simulation container.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NBodySystem {
    /// List of celestial bodies in the system.
    pub bodies: Vec<CelestialBody>,
    /// Accumulated simulation time in seconds.
    pub time_s: f64,
    /// Number of integration steps executed.
    pub step_count: u64,
    /// Gravitational constant $G$ in $\text{m}^3 \text{kg}^{-1} \text{s}^{-2}$.
    pub gravitational_constant: f64,
    /// Gravitational Plummer softening length $\epsilon$ in meters (0 for unsoftened).
    pub softening_m: f64,
    /// Enable Post-Newtonian (1PN) General Relativistic correction.
    pub enable_general_relativity: bool,
    /// Baseline mechanical energy $E_0$ recorded at $t=0$.
    pub initial_energy_j: Option<f64>,
    /// Baseline angular momentum magnitude $L_0$ recorded at $t=0$.
    pub initial_angular_momentum_mag: Option<f64>,
    /// Selected numerical integrator.
    pub integrator: IntegratorType,
}

impl NBodySystem {
    /// Creates a new empty N-body system with standard physics defaults.
    pub fn new() -> Self {
        Self {
            bodies: Vec::new(),
            time_s: 0.0,
            step_count: 0,
            gravitational_constant: G_STANDARD,
            softening_m: 0.0,
            enable_general_relativity: false,
            initial_energy_j: None,
            initial_angular_momentum_mag: None,
            integrator: IntegratorType::Yoshida4th,
        }
    }

    /// Adds a body to the system.
    pub fn add_body(&mut self, body: CelestialBody) {
        self.bodies.push(body);
    }

    /// Total system mass in kilograms.
    pub fn total_mass(&self) -> f64 {
        self.bodies.iter().map(|b| b.mass_kg).sum()
    }
}

impl Default for NBodySystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Breakdown of pairwise gravitational force from a source body acting on a target body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairwiseForce {
    /// Source body identifier.
    pub source_id: usize,
    /// Source body name (e.g., "Sun", "Jupiter").
    pub source_name: String,
    /// Cartesian force vector $[F_x, F_y, F_z]$ in Newtons ($\text{N}$).
    pub force_vector_n: [f64; 3],
    /// Scalar force magnitude $\|\mathbf{F}\|$ in Newtons ($\text{N}$).
    pub magnitude_n: f64,
    /// Fraction of total gravitational pull on the target body ($0.0 \dots 1.0$).
    pub fraction_of_total: f64,
}

/// Gravitational tidal tensor (spatial gravity gradient matrix $\mathbf{T}_{ab} = \frac{\partial g_a}{\partial x_b}$).
///
/// In vacuum, $\nabla \cdot \mathbf{g} = 0$, so $\text{Tr}(\mathbf{T}) = 0$.
/// The eigenvalues satisfy $\lambda_1 > 0$ (stretching along radial line)
/// and $\lambda_2, \lambda_3 < 0$ (orthogonal compression).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TidalTensor {
    /// $3 \times 3$ symmetric tidal gradient matrix in $\text{s}^{-2}$.
    pub matrix: [[f64; 3]; 3],
    /// Trace of the matrix (analytically 0.0 in vacuum).
    pub trace: f64,
    /// Sorted eigenvalues $[\lambda_1, \lambda_2, \lambda_3]$ in descending order.
    pub eigenvalues: [f64; 3],
    /// Maximum tidal stretching strain in Eötvös units ($1\text{ E} = 10^{-9}\text{ s}^{-2}$).
    pub max_strain_eotvos: f64,
}

/// Characteristic gravitational domains of planetary dominance.
///
/// References:
/// - Chebotarev, G. A. (1964), *Soviet Astronomy*, 7(5), pp. 618–622.
/// - Domingos, Winter, & Yokoyama (2006), *MNRAS*, 373(3), pp. 1227–1234.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GravitationalSphereRadii {
    /// Sphere of attraction radius $r_a = a \sqrt{\frac{m}{M_\odot}}$ in meters.
    pub sphere_of_attraction_m: f64,
    /// Laplace Sphere of Influence (SOI) $r_s = a \left(\frac{m}{M_\odot}\right)^{2/5}$ in meters.
    pub laplace_soi_m: f64,
    /// Hill sphere radius $r_H = a(1 - e) \sqrt[3]{\frac{m}{3 M_\odot}}$ in meters.
    pub hill_sphere_m: f64,
    /// Critical empirical stability radius for prograde satellites (Domingos et al. 2006):
    /// $r_{\text{crit}} \approx 0.4895 r_H (1 - 1.0305 e_{\text{sat}} - 0.2738 e_{\text{planet}})$.
    pub critical_stability_radius_m: f64,
}

/// Individual celestial body's gravitational field contribution at an arbitrary spatial point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyFieldContribution {
    /// Identifier of the source celestial body.
    pub body_id: usize,
    /// Name of the source celestial body.
    pub body_name: String,
    /// Hex color code of the source celestial body.
    pub body_color: String,
    /// Gravitational acceleration vector $\mathbf{g}_j(\mathbf{r}) = \frac{G m_j (\mathbf{r}_j - \mathbf{r})}{\|\mathbf{r}_j - \mathbf{r}\|^3}$ in $\text{m/s}^2$.
    pub acceleration_vector_mps2: [f64; 3],
    /// Acceleration scalar magnitude $\|\mathbf{g}_j(\mathbf{r})\|$ in $\text{m/s}^2$.
    pub acceleration_magnitude: f64,
    /// Newtonian gravitational potential contribution $\Phi_j(\mathbf{r}) = -\frac{G m_j}{\|\mathbf{r}_j - \mathbf{r}\|}$ in $\text{J/kg}$.
    pub gravitational_potential_j_kg: f64,
    /// Fraction of total scalar gravitational pull at this point ($0.0 \dots 1.0$).
    pub fraction_of_total: f64,
    /// Distance from the source body center to the spatial point in meters.
    pub distance_m: f64,
}

/// Comprehensive physical state of the gravitational field at an arbitrary spatial coordinate $\mathbf{r} = (x, y, z)$.
///
/// Enables spatial probe telemetry and domain fragmentation into Gravitational Dominance Basins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialFieldPoint {
    /// Cartesian position of the probe point $[x, y, z]$ in meters.
    pub position_m: [f64; 3],
    /// Net gravitational acceleration vector $\mathbf{g}(\mathbf{r}) = -\nabla \Phi(\mathbf{r})$ in $\text{m/s}^2$.
    pub acceleration_vector_mps2: [f64; 3],
    /// Net gravitational acceleration magnitude $\|\mathbf{g}(\mathbf{r})\|$ in $\text{m/s}^2$.
    pub acceleration_magnitude: f64,
    /// Total gravitational potential $\Phi(\mathbf{r}) = -\sum_{j=1}^N \frac{G m_j}{\|\mathbf{r}_j - \mathbf{r}\|}$ in $\text{J/kg}$ ($\text{m}^2/\text{s}^2$).
    pub gravitational_potential_j_kg: f64,
    /// ID of the dominant gravitational body at this point ($\arg\max_j \|\mathbf{g}_j(\mathbf{r})\|$).
    pub dominant_body_id: usize,
    /// Name of the dominant gravitational body (defines the spatial basin).
    pub dominant_body_name: String,
    /// Dominant body pull fraction ($0.0 \dots 1.0$).
    pub dominant_body_fraction: f64,
    /// Individual gravitational field contributions from all bodies in the system (Tug-of-War breakdown).
    pub contributions: Vec<BodyFieldContribution>,
    /// Gravitational tidal tensor (gravity gradient matrix $\mathbf{T}_{ab} = \partial g_a / \partial x_b$) at this point.
    pub tidal_tensor: TidalTensor,
}

/// Centric frame of reference defined by the major gravitational body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GravitationalCentricFrame {
    /// Heliocentric: centered on the Sun.
    Heliocentric,
    /// Geocentric: centered on the Earth.
    Geocentric,
    /// Selenocentric: centered on the Moon.
    Selenocentric,
    /// Jovicentric: centered on Jupiter.
    Jovicentric,
    /// Areocentric: centered on Mars.
    Areocentric,
    /// Cytherocentric: centered on Venus.
    Cytherocentric,
    /// Automatic: switches frame based on dominant gravitational attractor.
    Auto,
}

/// A massless test particle (orbital tracer / probe) moving in a centric reference frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CentricTestParticle {
    /// Unique identifier
    pub id: u64,
    /// Name of centric anchor body (e.g. "Earth", "Moon", "Jupiter")
    pub anchor_body_name: String,
    /// Position relative to centric anchor body [m]
    pub rel_position_m: [f64; 3],
    /// Velocity relative to centric anchor body [m/s]
    pub rel_velocity_m_s: [f64; 3],
    /// Semi-major axis or characteristic orbital distance [m]
    pub orbital_radius_m: f64,
}

/// Illumination and eclipse condition of a spacecraft relative to an occulting central body and the Sun.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EclipseState {
    /// Spacecraft is in direct line-of-sight of the entire solar disk (full illumination).
    #[default]
    Sunlit,
    /// Spacecraft is in the penumbral shadow (partial solar eclipse).
    Penumbra,
    /// Spacecraft is completely inside the umbral shadow cone (total solar eclipse).
    Umbra,
}

/// Geometric telemetry describing the shadow cone and eclipse boundaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShadowConeGeometry {
    /// Apex distance of the umbra cone from the center of the occulting body [m].
    pub umbra_length_m: f64,
    /// Half-angle of the umbral convergence cone [rad].
    pub umbra_half_angle_rad: f64,
    /// Half-angle of the penumbral divergence cone [rad].
    pub penumbra_half_angle_rad: f64,
    /// Unit vector pointing along the shadow axis away from the Sun.
    pub shadow_axis_unit: [f64; 3],
}

/// Comprehensive orbital and eclipse telemetry for a tracked satellite or test particle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SatelliteOrbitalTelemetry {
    /// Unique identifier
    pub id: u64,
    /// Human-readable label (e.g. "SAT-042", "Orbiter 1")
    pub name: String,
    /// Centric anchor body name (e.g. "Earth")
    pub anchor_body: String,
    /// Semi-major axis $a$ [m]
    pub semi_major_axis_m: f64,
    /// Orbital eccentricity $e$
    pub eccentricity: f64,
    /// Orbital inclination $i$ in degrees
    pub inclination_deg: f64,
    /// Periapsis radius from central body center [m]
    pub periapsis_radius_m: f64,
    /// Apoapsis radius from central body center [m]
    pub apoapsis_radius_m: f64,
    /// Periapsis altitude above central body surface [m]
    pub periapsis_altitude_m: f64,
    /// Apoapsis altitude above central body surface [m]
    pub apoapsis_altitude_m: f64,
    /// Current radial distance from central body center [m]
    pub current_radius_m: f64,
    /// Current altitude above central body surface [m]
    pub current_altitude_m: f64,
    /// Current orbital speed relative to central body [m/s]
    pub current_speed_m_s: f64,
    /// Orbital period [s]
    pub orbital_period_s: f64,
    /// Realtime illumination condition
    pub eclipse_state: EclipseState,
}
