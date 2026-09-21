//! Core types and data structures for the AutoOrbit physics-informed orbit prediction framework.
//!
//! Grounded in the KDD 2026 paper:
//! > **Zhang, D., Liang, S., Yang, B., Qi, T., Wang, S., & Li, Q. (2026).**  
//! > *AutoOrbit: Physics-Informed Satellite Orbit Prediction.*  
//! > In Proceedings of the 32nd ACM SIGKDD Conference on Knowledge Discovery and Data Mining (KDD '26).
//! > DOI: 10.1145/3770855.3818960

use core::fmt;

/// Gravitational parameter of Earth ($\mu = G M_\oplus$) in $\text{m}^3/\text{s}^2$.
pub const EARTH_MU: f64 = 3.986_004_418e14;

/// Earth equatorial radius ($R_\oplus$) in meters (WGS-84 / EGM2008).
pub const EARTH_RADIUS_M: f64 = 6_378_137.0;

/// Earth oblateness zonal harmonic ($J_2$).
pub const J2: f64 = 1.082_626_68e-3;

/// Earth zonal harmonic ($J_3$).
pub const J3: f64 = -2.532_656_485e-6;

/// Earth zonal harmonic ($J_4$).
pub const J4: f64 = -1.619_621_59e-6;

/// Six-dimensional state vector in Cartesian coordinates $[x, y, z, v_x, v_y, v_z]^T$.
///
/// Distances are in meters ($\text{m}$), and velocities are in meters per second ($\text{m/s}$).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateVector {
    /// X position (m) in ECI or ECEF frame.
    pub x: f64,
    /// Y position (m) in ECI or ECEF frame.
    pub y: f64,
    /// Z position (m) in ECI or ECEF frame.
    pub z: f64,
    /// X velocity component (m/s).
    pub vx: f64,
    /// Y velocity component (m/s).
    pub vy: f64,
    /// Z velocity component (m/s).
    pub vz: f64,
}

impl StateVector {
    /// Creates a new state vector.
    pub const fn new(x: f64, y: f64, z: f64, vx: f64, vy: f64, vz: f64) -> Self {
        Self { x, y, z, vx, vy, vz }
    }

    /// Returns the zero state vector.
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

    /// Creates a state vector from a 6-element array `[x, y, z, vx, vy, vz]`.
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

    /// Converts the state vector to a 6-element array `[x, y, z, vx, vy, vz]`.
    pub const fn to_array(&self) -> [f64; 6] {
        [self.x, self.y, self.z, self.vx, self.vy, self.vz]
    }

    /// Returns the 3D position vector $[x, y, z]$ in meters.
    pub const fn position(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }

    /// Returns the 3D velocity vector $[v_x, v_y, v_z]$ in m/s.
    pub const fn velocity(&self) -> [f64; 3] {
        [self.vx, self.vy, self.vz]
    }

    /// Position magnitude $\|\vec{r}\|$ in meters.
    pub fn r_norm(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Velocity magnitude $\|\vec{v}\|$ in m/s.
    pub fn v_norm(&self) -> f64 {
        (self.vx * self.vx + self.vy * self.vy + self.vz * self.vz).sqrt()
    }

    /// Returns a new state with injected Gaussian noise on position and velocity.
    pub fn add_noise(&self, pos_noise_m: [f64; 3], vel_noise_mps: [f64; 3]) -> Self {
        Self {
            x: self.x + pos_noise_m[0],
            y: self.y + pos_noise_m[1],
            z: self.z + pos_noise_m[2],
            vx: self.vx + vel_noise_mps[0],
            vy: self.vy + vel_noise_mps[1],
            vz: self.vz + vel_noise_mps[2],
        }
    }
}

impl core::ops::Add for StateVector {
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

impl core::ops::Sub for StateVector {
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

impl core::ops::Mul<f64> for StateVector {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
            vx: self.vx * rhs,
            vy: self.vy * rhs,
            vz: self.vz * rhs,
        }
    }
}

/// Classical Keplerian orbital elements $(a, e, i, \Omega, \omega, \nu)$.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeplerianElements {
    /// Semi-major axis $a$ in meters.
    pub semi_major_axis_m: f64,
    /// Orbital eccentricity $e$ (dimensionless, $0 \le e < 1$ for elliptic orbits).
    pub eccentricity: f64,
    /// Inclination $i$ in radians ($0 \le i \le \pi$).
    pub inclination_rad: f64,
    /// Right Ascension of Ascending Node $\Omega$ in radians ($0 \le \Omega < 2\pi$).
    pub raan_rad: f64,
    /// Argument of periapsis $\omega$ in radians ($0 \le \omega < 2\pi$).
    pub arg_periapsis_rad: f64,
    /// True anomaly $\nu$ in radians ($0 \le \nu < 2\pi$).
    pub true_anomaly_rad: f64,
}

impl KeplerianElements {
    /// Creates a new set of Keplerian orbital elements.
    pub const fn new(
        semi_major_axis_m: f64,
        eccentricity: f64,
        inclination_rad: f64,
        raan_rad: f64,
        arg_periapsis_rad: f64,
        true_anomaly_rad: f64,
    ) -> Self {
        Self {
            semi_major_axis_m,
            eccentricity,
            inclination_rad,
            raan_rad,
            arg_periapsis_rad,
            true_anomaly_rad,
        }
    }

    /// Semi-latus rectum $p = a(1 - e^2)$ in meters.
    pub fn semi_latus_rectum_m(&self) -> f64 {
        self.semi_major_axis_m * (1.0 - self.eccentricity * self.eccentricity)
    }

    /// Orbital period $T = 2\pi \sqrt{a^3 / \mu}$ in seconds.
    pub fn orbital_period_seconds(&self, mu: f64) -> f64 {
        2.0 * core::f64::consts::PI * (self.semi_major_axis_m.powi(3) / mu).sqrt()
    }

    /// Mean motion $n = \sqrt{\mu / a^3}$ in radians per second.
    pub fn mean_motion_rad_per_s(&self, mu: f64) -> f64 {
        (mu / self.semi_major_axis_m.powi(3)).sqrt()
    }

    /// Instantaneous radial distance $r = \frac{p}{1 + e \cos\nu}$ in meters.
    pub fn radius_m(&self) -> f64 {
        let p = self.semi_latus_rectum_m();
        p / (1.0 + self.eccentricity * self.true_anomaly_rad.cos())
    }

    /// Instantaneous speed $v = \sqrt{\mu (2/r - 1/a)}$ in m/s.
    pub fn speed_mps(&self, mu: f64) -> f64 {
        let r = self.radius_m();
        (mu * (2.0 / r - 1.0 / self.semi_major_axis_m)).max(0.0).sqrt()
    }
}

/// Impulsive velocity increment in the satellite's Radial-Along-Track-Cross-Track (RAC/RIC) body frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ManeuverImpulse {
    /// Radial velocity increment $\Delta v_r$ (m/s), pointing away from Earth center.
    pub delta_vr: f64,
    /// Along-track velocity increment $\Delta v_a$ (m/s), in orbital velocity direction.
    pub delta_va: f64,
    /// Cross-track velocity increment $\Delta v_c$ (m/s), normal to the orbital plane.
    pub delta_vc: f64,
    /// Maneuver execution time offset from prediction start (seconds).
    pub time_offset_seconds: f64,
}

impl ManeuverImpulse {
    /// Creates a new maneuver impulse.
    pub const fn new(delta_vr: f64, delta_va: f64, delta_vc: f64, time_offset_seconds: f64) -> Self {
        Self {
            delta_vr,
            delta_va,
            delta_vc,
            time_offset_seconds,
        }
    }

    /// Returns total velocity increment magnitude $\|\Delta \vec{v}\|$ in m/s.
    pub fn magnitude(&self) -> f64 {
        (self.delta_vr * self.delta_vr + self.delta_va * self.delta_va + self.delta_vc * self.delta_vc).sqrt()
    }
}

/// Normalization parameters (Z-score standardization: $\hat{x} = (x - \mu) / \sigma$).
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizationStats {
    /// Mean values for the 6 state channels $[\mu_x, \mu_y, \mu_z, \mu_{vx}, \mu_{vy}, \mu_{vz}]$.
    pub mean: [f64; 6],
    /// Standard deviations $[\sigma_x, \sigma_y, \sigma_z, \sigma_{vx}, \sigma_{vy}, \sigma_{vz}]$.
    pub std: [f64; 6],
}

impl Default for NormalizationStats {
    fn default() -> Self {
        Self {
            mean: [0.0; 6],
            std: [1.0; 6],
        }
    }
}

impl NormalizationStats {
    /// Creates normalization statistics from mean and standard deviation arrays.
    pub fn new(mean: [f64; 6], std: [f64; 6]) -> Self {
        let mut clean_std = std;
        for s in &mut clean_std {
            if *s <= 1e-12 {
                *s = 1.0;
            }
        }
        Self { mean, std: clean_std }
    }

    /// Normalizes a state vector: $\hat{x}_i = (x_i - \mu_i) / \sigma_i$.
    pub fn normalize(&self, state: &StateVector) -> [f64; 6] {
        let arr = state.to_array();
        let mut out = [0.0; 6];
        for i in 0..6 {
            out[i] = (arr[i] - self.mean[i]) / self.std[i];
        }
        out
    }

    /// Denormalizes a 6-element array: $x_i = \hat{x}_i \cdot \sigma_i + \mu_i$.
    pub fn denormalize(&self, norm: &[f64; 6]) -> StateVector {
        let mut arr = [0.0; 6];
        for i in 0..6 {
            arr[i] = norm[i] * self.std[i] + self.mean[i];
        }
        StateVector::from_array(arr)
    }
}

/// Target prediction horizon evaluated in AutoOrbit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredictionHorizon {
    /// 30-minute prediction (1800 seconds).
    HalfHour,
    /// 1-hour prediction (3600 seconds).
    OneHour,
    /// 4-hour prediction (14400 seconds).
    FourHours,
    /// 8-hour prediction (28800 seconds).
    EightHours,
    /// 12-hour prediction (43200 seconds).
    TwelveHours,
    /// 24-hour prediction (86400 seconds).
    TwentyFourHours,
}

impl PredictionHorizon {
    /// Returns duration in seconds.
    pub const fn seconds(&self) -> f64 {
        match self {
            Self::HalfHour => 1800.0,
            Self::OneHour => 3600.0,
            Self::FourHours => 14400.0,
            Self::EightHours => 28800.0,
            Self::TwelveHours => 43200.0,
            Self::TwentyFourHours => 86400.0,
        }
    }

    /// Number of prediction steps at a 10-second cadence.
    pub const fn steps_at_cadence(&self, cadence_seconds: f64) -> usize {
        (self.seconds() / cadence_seconds) as usize
    }
}

/// Evaluation metrics comparing predicted orbit against ground truth (POD).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PredictionMetrics {
    /// 3D Root-Mean-Square position error in meters ($\text{m}$).
    pub position_error_m: f64,
    /// 3D Root-Mean-Square velocity error in meters per second ($\text{m/s}$).
    pub velocity_error_mps: f64,
    /// 95th-percentile physical consistency error ($\times 10^{-6}\text{ m/s}^2$).
    pub p95_physics_consistency_error: f64,
    /// 99th-percentile physical consistency error ($\times 10^{-6}\text{ m/s}^2$).
    pub p99_physics_consistency_error: f64,
}

impl fmt::Display for PredictionMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Position Error: {:.3} m | Velocity Error: {:.4} m/s | P95 Physics: {:.2}e-6 | P99 Physics: {:.2}e-6",
            self.position_error_m,
            self.velocity_error_mps,
            self.p95_physics_consistency_error * 1e6,
            self.p99_physics_consistency_error * 1e6,
        )
    }
}

/// Errors that can occur during AutoOrbit operations.
#[derive(Debug, Clone, PartialEq)]
pub enum AutoOrbitError {
    /// Sequence length does not match expected dimension.
    InvalidSequenceLength { expected: usize, actual: usize },
    /// Non-positive or hyperbolic semi-major axis.
    NonEllipticOrbit { semi_major_axis: f64, eccentricity: f64 },
    /// Failed to solve Kepler's transcendental equation.
    KeplerConvergenceFailed { mean_anomaly: f64, iterations: usize },
    /// Invalid configuration parameter.
    InvalidConfiguration(String),
}

impl fmt::Display for AutoOrbitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSequenceLength { expected, actual } => {
                write!(f, "Invalid sequence length: expected {}, got {}", expected, actual)
            }
            Self::NonEllipticOrbit { semi_major_axis, eccentricity } => {
                write!(
                    f,
                    "Non-elliptic orbit: semi-major axis = {:.1} m, eccentricity = {:.4}",
                    semi_major_axis, eccentricity
                )
            }
            Self::KeplerConvergenceFailed { mean_anomaly, iterations } => {
                write!(
                    f,
                    "Kepler equation solver failed to converge for M = {:.4} after {} iterations",
                    mean_anomaly, iterations
                )
            }
            Self::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
        }
    }
}

impl std::error::Error for AutoOrbitError {}
