//! Core types and physical parameters for the Circular Restricted Three-Body Problem (CR3BP).
//!
//! Grounded in the AAS/AIAA Astrodynamics specification:
//! > **Short, C., Haapala, A., & Bosanac, N. (2020).**  
//! > *Technical Implementation of the Circular Restricted Three-Body Model in STK Astrogator.*  
//! > AAS/AIAA Astrodynamics Specialist Conference, AAS 20-459.

use core::fmt;

/// State vector $[x, y, z, \dot{x}, \dot{y}, \dot{z}]$ in the CR3BP system.
///
/// Can represent either:
/// - Nondimensional rotating barycentric state (coordinates in units of $l^*$, velocities in $v^*$).
/// - Dimensional Central Body Inertial (CBI) state (meters and meters/second).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cr3bpState {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
}

impl Cr3bpState {
    /// Creates a new state vector.
    pub const fn new(x: f64, y: f64, z: f64, vx: f64, vy: f64, vz: f64) -> Self {
        Self { x, y, z, vx, vy, vz }
    }

    /// Zero state vector.
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

    /// Converts to a 6-element array `[x, y, z, vx, vy, vz]`.
    pub const fn to_array(&self) -> [f64; 6] {
        [self.x, self.y, self.z, self.vx, self.vy, self.vz]
    }

    /// Position 3-vector $[x, y, z]$.
    pub const fn position(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }

    /// Velocity 3-vector $[v_x, v_y, v_z]$.
    pub const fn velocity(&self) -> [f64; 3] {
        [self.vx, self.vy, self.vz]
    }

    /// Position magnitude $\|\vec{r}\|$.
    pub fn r_norm(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Velocity magnitude $\|\vec{v}\|$.
    pub fn v_norm(&self) -> f64 {
        (self.vx * self.vx + self.vy * self.vy + self.vz * self.vz).sqrt()
    }

    /// Velocity squared $v^2 = \dot{x}^2 + \dot{y}^2 + \dot{z}^2$.
    pub fn v_squared(&self) -> f64 {
        self.vx * self.vx + self.vy * self.vy + self.vz * self.vz
    }

    /// Geostationary Earth Orbit (GEO) in rotating barycentric coordinates.
    ///
    /// Altitude $h = 35,786\text{ km}$, radius $r_{GEO} \approx 42,164\text{ km}$.
    /// Located at sub-lunar point ($+x$ axis toward Moon) with prograde orbital velocity.
    pub fn earth_geostationary(system: &Cr3bpSystem) -> Self {
        let r_geo_m = 42_164_137.0;
        let r_nd = r_geo_m / system.l_star;
        let v_c_ms = (system.gm1 / r_geo_m).sqrt();
        let v_c_nd = v_c_ms / system.v_star;
        let vy_rot = v_c_nd - r_nd;
        let x_bary = -system.mu + r_nd;
        Self::new(x_bary, 0.0, 0.0, 0.0, vy_rot, 0.0)
    }

    /// Geostationary Transfer Orbit (GTO) at apogee in rotating barycentric coordinates.
    ///
    /// Perigee altitude $250\text{ km}$, apogee altitude $35,786\text{ km}$.
    pub fn earth_gto_apogee(system: &Cr3bpSystem) -> Self {
        let r_p_m = 6_628_137.0;
        let r_a_m = 42_164_137.0;
        let a_m = 0.5 * (r_p_m + r_a_m);
        let v_a_ms = (system.gm1 * (2.0 / r_a_m - 1.0 / a_m)).sqrt();
        let r_nd = r_a_m / system.l_star;
        let v_a_nd = v_a_ms / system.v_star;
        let vy_rot = v_a_nd - r_nd;
        let x_bary = -system.mu + r_nd;
        Self::new(x_bary, 0.0, 0.0, 0.0, vy_rot, 0.0)
    }

    /// Trans-Lunar Injection (TLI) high-apogee staging state in rotating barycentric coordinates.
    ///
    /// Represents a spacecraft on a high-eccentricity cislunar transfer ellipse arriving
    /// near the weak stability boundary / cislunar entry point ($r \approx 320,000\text{ km}$).
    pub fn trans_lunar_injection_apogee(system: &Cr3bpSystem) -> Self {
        let r_m = 320_000_000.0;
        let r_p_m = 6_678_137.0; // 300 km LEO perigee
        let r_a_m = system.l_star;
        let a_m = 0.5 * (r_p_m + r_a_m);
        let v_ms = (system.gm1 * (2.0 / r_m - 1.0 / a_m)).sqrt();
        let r_nd = r_m / system.l_star;
        let v_nd = v_ms / system.v_star;
        let vy_rot = v_nd - r_nd;
        let x_bary = -system.mu + r_nd;
        Self::new(x_bary, 0.0, 0.0, 0.0, vy_rot, 0.0)
    }
}

/// Calculates the impulsive Trans-Lunar Injection (TLI) $\Delta v$ (in m/s) from a circular LEO orbit.
///
/// Given circular LEO altitude $h_{LEO}$ (km) and optional target apogee distance $r_a$ (km, default primary-secondary distance):
///
/// $$\Delta v_{TLI} = \sqrt{\mu_E \left(\frac{2}{r_p} - \frac{1}{a}\right)} - \sqrt{\frac{\mu_E}{r_p}}$$
pub fn calculate_tli_impulsive_dv(system: &Cr3bpSystem, leo_altitude_km: f64, target_apogee_km: Option<f64>) -> f64 {
    let r_earth_km = 6378.137;
    let rp_m = (r_earth_km + leo_altitude_km) * 1000.0;
    let ra_m = target_apogee_km.unwrap_or(system.l_star / 1000.0) * 1000.0;
    let a_m = 0.5 * (rp_m + ra_m);

    let v_circ_leo = (system.gm1 / rp_m).sqrt();
    let v_peri_tli = (system.gm1 * (2.0 / rp_m - 1.0 / a_m)).sqrt();

    (v_peri_tli - v_circ_leo).max(0.0)
}

impl core::ops::Add for Cr3bpState {
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

impl core::ops::Sub for Cr3bpState {
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

impl core::ops::Mul<f64> for Cr3bpState {
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

/// 9-dimensional state vector $[x, y, z, v_x, v_y, v_z, a_x, a_y, a_z]^T$ used in STK Astrogator frame transformations (Table 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cr3bpState9D {
    pub state: Cr3bpState,
    pub ax: f64,
    pub ay: f64,
    pub az: f64,
}

impl Cr3bpState9D {
    pub const fn new(state: Cr3bpState, ax: f64, ay: f64, az: f64) -> Self {
        Self { state, ax, ay, az }
    }

    pub const fn acceleration(&self) -> [f64; 3] {
        [self.ax, self.ay, self.az]
    }
}

/// Circular Restricted Three-Body Problem (CR3BP) System definition.
///
/// Grounded in Equation (12):
///
/// $$\mu = \frac{G M_2}{G M_1 + G M_2}$$
#[derive(Debug, Clone, PartialEq)]
pub struct Cr3bpSystem {
    /// System name (e.g. "Earth-Moon", "Sun-Earth", "Sun-Jupiter").
    pub name: String,
    /// System mass parameter $\mu = m_2 / (m_1 + m_2)$.
    pub mu: f64,
    /// Characteristic length $l^*$ (primary-secondary separation) in meters.
    pub l_star: f64,
    /// Characteristic time $t^* = \sqrt{l^{*3} / (G(m_1 + m_2))}$ in seconds.
    pub t_star: f64,
    /// Characteristic velocity $v^* = l^* / t^*$ in m/s.
    pub v_star: f64,
    /// Characteristic acceleration $a^* = l^* / t^{*2}$ in $\text{m/s}^2$.
    pub a_star: f64,
    /// Primary gravitational parameter $G M_1$ in $\text{m}^3/\text{s}^2$.
    pub gm1: f64,
    /// Secondary gravitational parameter $G M_2$ in $\text{m}^3/\text{s}^2$.
    pub gm2: f64,
}

impl Cr3bpSystem {
    /// Constructs a custom CR3BP system from gravitational parameters and primary-secondary separation.
    pub fn new(name: impl Into<String>, gm1: f64, gm2: f64, l_star: f64) -> Self {
        let mu = gm2 / (gm1 + gm2);
        let gm_total = gm1 + gm2;
        let t_star = (l_star.powi(3) / gm_total).sqrt();
        let v_star = l_star / t_star;
        let a_star = l_star / (t_star * t_star);

        Self {
            name: name.into(),
            mu,
            l_star,
            t_star,
            v_star,
            a_star,
            gm1,
            gm2,
        }
    }

    /// Earth–Moon CR3BP system preset matching the paper (AAS 20-459, Section 4).
    ///
    /// Primary $P_1$: Earth ($G M_1 = 398600.4418\text{ km}^3/\text{s}^2$).  
    /// Secondary $P_2$: Moon / Luna ($G M_2 = 4902.800066\text{ km}^3/\text{s}^2$).  
    /// Distance $l^* = 384,400\text{ km}$.  
    /// Mass ratio $\mu \approx 0.0121505856$.  
    /// Characteristic time $t^* \approx 375,190.26\text{ s} \approx 4.3425\text{ days}$.
    pub fn earth_moon() -> Self {
        let gm_earth = 3.986_004_418e14;
        let gm_moon = 4.902_800_066e12;
        let l_star = 384_400_000.0;
        Self::new("Earth-Moon", gm_earth, gm_moon, l_star)
    }

    /// Sun–Earth CR3BP system preset (useful for deep space libration point missions like JWST, SOHO, Gaia).
    ///
    /// Primary $P_1$: Sun ($G M_\odot = 1.32712440018\times 10^{20}\text{ m}^3/\text{s}^2$).  
    /// Secondary $P_2$: Earth+Moon ($G M_{\oplus+M} = 4.0350324\times 10^{14}\text{ m}^3/\text{s}^2$).  
    /// Distance $l^* = 1\text{ AU} = 1.495978707\times 10^{11}\text{ m}$.  
    /// Mass ratio $\mu \approx 3.04042 \times 10^{-6}$.  
    /// Characteristic time $t^* \approx 5.0226 \times 10^6\text{ s} \approx 58.13\text{ days}$.
    pub fn sun_earth() -> Self {
        let gm_sun = 1.327_124_400_18e20;
        let gm_earth_moon = 4.035_032_4e14;
        let l_star = 1.495_978_707e11;
        Self::new("Sun-Earth", gm_sun, gm_earth_moon, l_star)
    }

    /// Sun–Jupiter CR3BP system preset (deep space Trojan asteroids, Jupiter flybys).
    pub fn sun_jupiter() -> Self {
        let gm_sun = 1.327_124_400_18e20;
        let gm_jupiter = 1.266_865_34e17;
        let l_star = 7.785_7e11;
        Self::new("Sun-Jupiter", gm_sun, gm_jupiter, l_star)
    }

    /// Nondimensionalizes a dimensional position vector (meters $\rightarrow$ nondimensional).
    #[inline]
    pub fn nondimensionalize_position(&self, r_dim: [f64; 3]) -> [f64; 3] {
        [r_dim[0] / self.l_star, r_dim[1] / self.l_star, r_dim[2] / self.l_star]
    }

    /// Dimensionalizes a nondimensional position vector (nondimensional $\rightarrow$ meters).
    #[inline]
    pub fn dimensionalize_position(&self, r_nd: [f64; 3]) -> [f64; 3] {
        [r_nd[0] * self.l_star, r_nd[1] * self.l_star, r_nd[2] * self.l_star]
    }

    /// Nondimensionalizes a dimensional velocity vector (m/s $\rightarrow$ nondimensional).
    #[inline]
    pub fn nondimensionalize_velocity(&self, v_dim: [f64; 3]) -> [f64; 3] {
        [v_dim[0] / self.v_star, v_dim[1] / self.v_star, v_dim[2] / self.v_star]
    }

    /// Dimensionalizes a nondimensional velocity vector (nondimensional $\rightarrow$ m/s).
    #[inline]
    pub fn dimensionalize_velocity(&self, v_nd: [f64; 3]) -> [f64; 3] {
        [v_nd[0] * self.v_star, v_nd[1] * self.v_star, v_nd[2] * self.v_star]
    }

    /// Nondimensionalizes a full 6D state vector.
    pub fn nondimensionalize_state(&self, state: &Cr3bpState) -> Cr3bpState {
        Cr3bpState {
            x: state.x / self.l_star,
            y: state.y / self.l_star,
            z: state.z / self.l_star,
            vx: state.vx / self.v_star,
            vy: state.vy / self.v_star,
            vz: state.vz / self.v_star,
        }
    }

    /// Dimensionalizes a full 6D state vector.
    pub fn dimensionalize_state(&self, state: &Cr3bpState) -> Cr3bpState {
        Cr3bpState {
            x: state.x * self.l_star,
            y: state.y * self.l_star,
            z: state.z * self.l_star,
            vx: state.vx * self.v_star,
            vy: state.vy * self.v_star,
            vz: state.vz * self.v_star,
        }
    }
}

/// The 5 equilibrium Lagrange (libration) points of the CR3BP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LagrangePoint {
    /// L1: Collinear point between primary P1 and secondary P2.
    L1,
    /// L2: Collinear point beyond secondary P2.
    L2,
    /// L3: Collinear point beyond primary P1.
    L3,
    /// L4: Triangular leading point ($60^\circ$ ahead of secondary).
    L4,
    /// L5: Triangular trailing point ($60^\circ$ behind secondary).
    L5,
}

impl fmt::Display for LagrangePoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::L1 => write!(f, "L1"),
            Self::L2 => write!(f, "L2"),
            Self::L3 => write!(f, "L3"),
            Self::L4 => write!(f, "L4"),
            Self::L5 => write!(f, "L5"),
        }
    }
}

/// Errors occurring during CR3BP operations.
#[derive(Debug, Clone, PartialEq)]
pub enum Cr3bpError {
    /// Numerical integrator failed to converge within step limit.
    IntegrationDiverged { step: usize, reason: String },
    /// State is within physical collision radius of a primary body.
    BodyCollision { body: String, distance_m: f64 },
    /// Solver failed to locate libration point root.
    LibrationPointConvergenceFailed { point: LagrangePoint, iterations: usize },
    /// Differential correction failed to converge to periodic orbit.
    DifferentialCorrectionFailed { iterations: usize, residual: f64 },
    /// Invalid configuration.
    InvalidConfig(String),
}

impl fmt::Display for Cr3bpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IntegrationDiverged { step, reason } => {
                write!(f, "CR3BP numerical integration diverged at step {}: {}", step, reason)
            }
            Self::BodyCollision { body, distance_m } => {
                write!(f, "Spacecraft collided with body '{}' at distance {:.1} m", body, distance_m)
            }
            Self::LibrationPointConvergenceFailed { point, iterations } => {
                write!(f, "Failed to converge for {:?} after {} iterations", point, iterations)
            }
            Self::DifferentialCorrectionFailed { iterations, residual } => {
                write!(f, "Differential correction failed after {} iterations (residual: {:e})", iterations, residual)
            }
            Self::InvalidConfig(msg) => write!(f, "Invalid CR3BP configuration: {}", msg),
        }
    }
}

impl std::error::Error for Cr3bpError {}
