//! Data structures and core telemetry types for high-precision N-body gravitational dynamics.

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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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
