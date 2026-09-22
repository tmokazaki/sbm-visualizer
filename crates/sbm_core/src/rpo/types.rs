//! Core data structures and telemetry types for Rendezvous and Proximity Operations (RPO).
//!
//! Grounded in relative orbital mechanics and Hill-Clohessy-Wiltshire dynamics:
//! > **Clohessy, W. H., & Wiltshire, R. S. (1960).**  
//! > *Terminal Guidance System for Satellite Rendezvous.*  
//! > Journal of the Aerospace Sciences, 27(9), pp. 653–658.

use core::fmt;

/// Relative state vector $[x, y, z, v_x, v_y, v_z]^T$ of a chaser spacecraft relative to a target in LVLH / Hill frame.
///
/// Coordinate conventions (Local-Vertical Local-Horizontal):
/// - $\hat{\mathbf{x}}$ (Radial / $R$-bar): Points from Earth center along target position vector (upward).
/// - $\hat{\mathbf{y}}$ (In-Track / $V$-bar): Points along target orbital velocity direction (forward).
/// - $\hat{\mathbf{z}}$ (Cross-Track / $H$-bar): Points normal to target orbital plane along angular momentum $\mathbf{h} = \mathbf{r} \times \mathbf{v}$.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelativeState {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
}

impl RelativeState {
    /// Creates a new relative state vector with dimensional units (meters and m/s).
    pub const fn new(x: f64, y: f64, z: f64, vx: f64, vy: f64, vz: f64) -> Self {
        Self { x, y, z, vx, vy, vz }
    }

    /// Zero relative state (coincident with target center of mass).
    pub const fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
        }
    }

    /// Creates a relative state from a 6-element array `[x, y, z, vx, vy, vz]`.
    pub const fn from_array(arr: [f64; 6]) -> Self {
        Self {
            x: arr[0],
            y: arr[1],
            z: arr[2],
            vx: arr[3],
            vy: arr[4],
            vz: arr[5],
        }
    }

    /// Converts to a 6-element array `[x, y, z, vx, vy, vz]`.
    pub const fn to_array(&self) -> [f64; 6] {
        [self.x, self.y, self.z, self.vx, self.vy, self.vz]
    }

    /// Position 3-vector $[x, y, z]$ in meters.
    pub const fn position(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }

    /// Velocity 3-vector $[v_x, v_y, v_z]$ in m/s.
    pub const fn velocity(&self) -> [f64; 3] {
        [self.vx, self.vy, self.vz]
    }

    /// Relative distance $\|\mathbf{r}\|$ to target in meters.
    pub fn r_norm(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Relative speed $\|\mathbf{v}\|$ in m/s.
    pub fn v_norm(&self) -> f64 {
        (self.vx * self.vx + self.vy * self.vy + self.vz * self.vz).sqrt()
    }
}

impl core::ops::Add for RelativeState {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            vx: self.vx + rhs.vx,
            vy: self.vy + rhs.vy,
            vz: self.vz + rhs.vz,
        }
    }
}

impl core::ops::Sub for RelativeState {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
            vx: self.vx - rhs.vx,
            vy: self.vy - rhs.vy,
            vz: self.vz - rhs.vz,
        }
    }
}

/// Parameters of the target satellite's reference orbit.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetOrbit {
    /// Orbit altitude in kilometers.
    pub altitude_km: f64,
    /// Semi-major axis in meters ($a = R_E + h$).
    pub semi_major_axis_m: f64,
    /// Earth gravitational parameter $\mu_E$ in $\text{m}^3/\text{s}^2$.
    pub mu: f64,
    /// Orbital mean motion $\omega = \sqrt{\mu_E / a^3}$ in rad/s.
    pub mean_motion_rad_s: f64,
    /// Orbital period in seconds ($T = 2\pi / \omega$).
    pub period_s: f64,
    /// Circular orbital speed in m/s ($v_c = \sqrt{\mu_E / a}$).
    pub circular_velocity_mps: f64,
}

impl TargetOrbit {
    /// Constructs a circular Earth orbit specification from altitude in kilometers.
    pub fn circular(altitude_km: f64) -> Self {
        let r_earth_m = 6_378_137.0;
        let mu_earth = 3.986_004_418e14;
        let a_m = r_earth_m + altitude_km * 1000.0;
        let omega = (mu_earth / a_m.powi(3)).sqrt();
        let period = 2.0 * core::f64::consts::PI / omega;
        let v_c = (mu_earth / a_m).sqrt();

        Self {
            altitude_km,
            semi_major_axis_m: a_m,
            mu: mu_earth,
            mean_motion_rad_s: omega,
            period_s: period,
            circular_velocity_mps: v_c,
        }
    }

    /// Standard International Space Station (ISS) target orbit (~400 km).
    pub fn iss() -> Self {
        Self::circular(400.0)
    }

    /// Standard Geostationary (GEO) target orbit (35,786 km).
    pub fn geo() -> Self {
        Self::circular(35_786.0)
    }

    /// Standard Sun-Synchronous LEO target orbit (700 km, e.g. Earth observation / debris).
    pub fn sso_700km() -> Self {
        Self::circular(700.0)
    }
}

/// Operational maneuver impulse in an RPO schedule.
#[derive(Debug, Clone, PartialEq)]
pub struct RpoManeuverDto {
    /// Time of burn in seconds from scenario epoch.
    pub time_s: f64,
    /// Dimensional velocity increment vector $[\Delta v_x, \Delta v_y, \Delta v_z]$ in m/s (LVLH).
    pub delta_v_mps: [f64; 3],
    /// Maneuver magnitude in m/s.
    pub magnitude_mps: f64,
    /// Operational description (e.g. "Departure Burn", "Mid-course Correction", "Braking & Hold").
    pub description: String,
}

/// Output flight plan for a targeted 2-impulse relative orbit transfer.
#[derive(Debug, Clone, PartialEq)]
pub struct TwoImpulseTransferPlan {
    pub success: bool,
    pub transfer_duration_s: f64,
    pub transfer_duration_min: f64,
    pub dv1: RpoManeuverDto,
    pub dv2: RpoManeuverDto,
    pub total_delta_v_mps: f64,
    pub initial_state: RelativeState,
    pub final_state: RelativeState,
    pub trajectory_points: Vec<[f64; 4]>, // [time_s, x_m, y_m, z_m]
}

/// Output flight plan for a Natural Motion Circumnavigation (NMC) passive inspection orbit.
#[derive(Debug, Clone, PartialEq)]
pub struct NmcInspectionPlan {
    pub success: bool,
    pub radial_amplitude_m: f64,
    pub along_track_amplitude_m: f64,
    pub cross_track_amplitude_m: f64,
    pub period_s: f64,
    pub insertion_maneuver: RpoManeuverDto,
    pub initial_drift_free_state: RelativeState,
    pub trajectory_points: Vec<[f64; 4]>, // [time_s, x_m, y_m, z_m]
}

/// Output flight plan for an along-track V-bar or radial R-bar glideslope approach.
#[derive(Debug, Clone, PartialEq)]
pub struct GlideslopeApproachPlan {
    pub success: bool,
    pub approach_type: String, // "V-Bar" or "R-Bar"
    pub start_distance_m: f64,
    pub end_distance_m: f64,
    pub duration_s: f64,
    pub total_delta_v_mps: f64,
    pub burns: Vec<RpoManeuverDto>,
    pub trajectory_points: Vec<[f64; 4]>,
}

/// Errors occurring during RPO maneuver operations.
#[derive(Debug, Clone, PartialEq)]
pub enum RpoError {
    SingularTransferTime(String),
    ZeroTargetMeanMotion,
    InvalidConfiguration(String),
}

impl fmt::Display for RpoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SingularTransferTime(msg) => write!(f, "Singular transfer duration: {}", msg),
            Self::ZeroTargetMeanMotion => write!(f, "Target orbital mean motion cannot be zero"),
            Self::InvalidConfiguration(msg) => write!(f, "Invalid RPO configuration: {}", msg),
        }
    }
}

impl std::error::Error for RpoError {}
