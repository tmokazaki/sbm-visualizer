//! Core data types and configuration structures for autonomous satellite avoidance.
//!
//! Grounded in the mathematical formulation of:
//! > **Luna, L., Couder, J. O., & Vargas-Acosta, R. A. (2026).**  
//! > *Satellite Trajectory Optimization via Proximal Policy Optimization for Space Debris Avoidance.*  
//! > IEEE Access, DOI: 10.1109/ACCESS.2026.3655237.

use serde::{Deserialize, Serialize};

/// Standard gravitational parameter of Earth $\mu_\oplus$ in $\text{m}^3/\text{s}^2$.
pub const MU_EARTH: f64 = 3.986004418e14;

/// Standard gravitational parameter of the Moon $\mu_{\text{Moon}}$ in $\text{m}^3/\text{s}^2$.
pub const MU_MOON: f64 = 4.902800066e12;

/// Standard gravitational parameter of the Sun $\mu_\odot$ in $\text{m}^3/\text{s}^2$.
pub const MU_SUN: f64 = 1.32712440018e20;

/// Standard mean Earth volumetric radius in meters.
pub const EARTH_RADIUS_M: f64 = 6378137.0;

/// Standard acceleration of gravity $g_0$ in $\text{m/s}^2$.
pub const G0_STANDARD: f64 = 9.80665;

/// Specific impulse of the chemical/electric propulsion system in seconds ($I_{sp} = 300\text{ s}$).
pub const SPECIFIC_IMPULSE_S: f64 = 300.0;

/// Maximum thrust acceleration authority ($T_{\max} = 0.15\text{ m/s}^2$).
pub const MAX_THRUST_ACCELERATION: f64 = 0.15;

/// Default collision sphere threshold $d_{\text{coll}}$ in meters ($300\text{ m}$, matching evaluation test benchmark).
pub const DEFAULT_COLLISION_DISTANCE_M: f64 = 300.0;

/// Default safe buffer radius $d_{\text{safe}} = d_{\text{coll}} + B$ in meters ($3,300\text{ m}$).
pub const DEFAULT_SAFE_BUFFER_M: f64 = 3000.0;

/// Maximum number of debris objects tracked in the observation vector.
pub const MAX_DEBRIS_COUNT: usize = 100;

/// Observation space dimension ($3 + 3 + 1 + 100 \times 3 = 307$).
pub const OBSERVATION_DIM: usize = 307;

/// Action space dimension (3D acceleration vector $[a_x, a_y, a_z]$).
pub const ACTION_DIM: usize = 3;

/// 3D Cartesian vector with double precision.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vector3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3D {
    /// Constructs a new 3D vector.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Zero vector $[0, 0, 0]$.
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }

    /// Vector Euclidean $L_2$ norm.
    #[inline]
    pub fn norm(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Squared Euclidean norm.
    #[inline]
    pub fn norm_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Returns unit direction vector, or zero if magnitude is negligible.
    #[inline]
    pub fn normalize_or_zero(&self) -> Self {
        let n = self.norm();
        if n > 1e-12 {
            Self {
                x: self.x / n,
                y: self.y / n,
                z: self.z / n,
            }
        } else {
            Self::zero()
        }
    }

    /// Dot product between two vectors.
    #[inline]
    pub fn dot(&self, other: &Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product $\mathbf{a} \times \mathbf{b}$.
    #[inline]
    pub fn cross(&self, other: &Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Converts to an array `[x, y, z]`.
    pub const fn to_array(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }

    /// Constructs from an array `[x, y, z]`.
    pub const fn from_array(arr: [f64; 3]) -> Self {
        Self {
            x: arr[0],
            y: arr[1],
            z: arr[2],
        }
    }
}

impl core::ops::Add for Vector3D {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl core::ops::Sub for Vector3D {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl core::ops::Mul<f64> for Vector3D {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl core::ops::Neg for Vector3D {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

/// Commanded 3D continuous control action $\mathbf{a} \in [-1, 1]^3$.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Action3D {
    pub ax: f64,
    pub ay: f64,
    pub az: f64,
}

impl Action3D {
    /// Creates a new action vector, clamped to $[-1.0, 1.0]$ bounds.
    pub fn new(ax: f64, ay: f64, az: f64) -> Self {
        Self {
            ax: ax.clamp(-1.0, 1.0),
            ay: ay.clamp(-1.0, 1.0),
            az: az.clamp(-1.0, 1.0),
        }
    }

    /// Passive coasting action (no thrust).
    pub const fn zero() -> Self {
        Self {
            ax: 0.0,
            ay: 0.0,
            az: 0.0,
        }
    }

    /// Converts to `Vector3D`.
    pub fn to_vector(&self) -> Vector3D {
        Vector3D::new(self.ax, self.ay, self.az)
    }

    /// Euclidean magnitude of the commanded action $\|\mathbf{a}\|$.
    pub fn magnitude(&self) -> f64 {
        (self.ax * self.ax + self.ay * self.ay + self.az * self.az).sqrt()
    }
}

/// Satellite dynamic physical state in Earth-Centered Inertial (ECI) frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SatelliteState {
    /// ECI Position vector $\mathbf{r}_{\text{sat}}$ in meters.
    pub position: Vector3D,
    /// ECI Velocity vector $\mathbf{v}_{\text{sat}}$ in meters per second.
    pub velocity: Vector3D,
    /// Total instantaneous satellite mass $m_t$ (dry + fuel) in kg.
    pub total_mass_kg: f64,
    /// Remaining propellant mass $m_{\text{fuel}}$ in kg.
    pub fuel_mass_kg: f64,
}

impl SatelliteState {
    /// Constructs default satellite at LEO ($700\text{ km}$ equatorial orbit, $1000\text{ kg}$ total mass, $500\text{ kg}$ fuel).
    pub fn new_leo_equatorial(alt_m: f64) -> Self {
        let r = EARTH_RADIUS_M + alt_m;
        let v = (MU_EARTH / r).sqrt();
        Self {
            position: Vector3D::new(r, 0.0, 0.0),
            velocity: Vector3D::new(0.0, v, 0.0),
            total_mass_kg: 1000.0,
            fuel_mass_kg: 500.0,
        }
    }
}

/// Debris fragment state in ECI frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebrisObject {
    /// Debris ECI position in meters.
    pub position: Vector3D,
    /// Debris ECI velocity in meters per second.
    pub velocity: Vector3D,
    /// Physical spherical radius in meters ($r_{\text{deb}}$).
    pub radius_m: f64,
}

impl DebrisObject {
    /// Creates a new debris fragment.
    pub fn new(position: Vector3D, velocity: Vector3D, radius_m: f64) -> Self {
        Self {
            position,
            velocity,
            radius_m,
        }
    }
}

/// Conjunction and relative geometry assessment metrics for a debris encounter.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConjunctionMetrics {
    /// Instantaneous separation distance $d = \|\mathbf{r}_{\text{rel}}\|$ in meters.
    pub distance_m: f64,
    /// Linear predicted Time-to-Closest-Approach (TCA) $\tau$ in seconds.
    pub tca_seconds: f64,
    /// Projected miss distance at closest approach $d_{pm}$ in meters.
    pub projected_miss_m: f64,
    /// Instantaneous relative closing speed $v_{\text{close}} = -\frac{\mathbf{r}_{\text{rel}}\cdot\mathbf{v}_{\text{rel}}}{\|\mathbf{r}_{\text{rel}}\|}$ in m/s.
    pub closing_speed_mps: f64,
    /// Risk hazard score $\rho \in [0, 1]$ computed via Eq. (9).
    pub hazard_score: f64,
}

/// Breakdown of individual reward terms accrued during a step or episode (Algorithm 3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct RewardBreakdown {
    /// Constant survival tick ($\lambda_{\text{surv}}$).
    pub survival: f64,
    /// Distance shaping reward/penalty.
    pub distance: f64,
    /// Closing speed reduction bonus.
    pub closing: f64,
    /// Milestone survival bonus ($R_{\text{mile}}$).
    pub milestone: f64,
    /// Linear, quadratic, and large burn $\Delta v$ penalties.
    pub dv_penalty: f64,
    /// Action change jitter penalty.
    pub smooth_penalty: f64,
    /// Cumulative $\Delta v$ soft-cap penalty.
    pub cumulative_penalty: f64,
    /// Catastrophic collision termination penalty ($-P_{\text{coll}}$).
    pub collision_penalty: f64,
    /// Total combined reward for the step.
    pub total: f64,
}

impl RewardBreakdown {
    /// Accumulates another reward breakdown into this record.
    pub fn add(&mut self, other: &Self) {
        self.survival += other.survival;
        self.distance += other.distance;
        self.closing += other.closing;
        self.milestone += other.milestone;
        self.dv_penalty += other.dv_penalty;
        self.smooth_penalty += other.smooth_penalty;
        self.cumulative_penalty += other.cumulative_penalty;
        self.collision_penalty += other.collision_penalty;
        self.total += other.total;
    }
}

/// Episode termination reason matching Table 3 taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerminationReason {
    /// Satellites successfully completed episode horizon without collision.
    Success,
    /// Separation distance violated collision threshold ($d_t \le d_{\text{coll}}$).
    Collision,
    /// Propellant depleted ($m_{\text{fuel}} \le 0$).
    FuelDepleted,
    /// Full orbit completed ($t \ge T_{\text{orbit}}$).
    OrbitComplete,
    /// Hit max episode steps without terminal collision or depletion.
    MaxSteps,
}

/// Configurable reward shaping hyperparameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RewardCoefficients {
    pub collision_penalty: f64,
    pub survival_reward: f64,
    pub distance_shaping_coeff: f64,
    pub coast_bonus: f64,
    pub delta_v_linear_cost: f64,
    pub delta_v_quadratic_cost: f64,
    pub large_burn_penalty: f64,
    pub smoothness_penalty: f64,
    pub cumulative_dv_penalty: f64,
    pub projected_miss_reward: f64,
    pub milestone_reward: f64,
}

impl RewardCoefficients {
    /// Training reward shaping coefficients matching Table 6.
    pub fn training() -> Self {
        Self {
            collision_penalty: 1000.0,
            survival_reward: 0.02,
            distance_shaping_coeff: 0.0002,
            coast_bonus: 0.05,
            delta_v_linear_cost: 8.0,
            delta_v_quadratic_cost: 0.8,
            large_burn_penalty: 4.0,
            smoothness_penalty: 0.008,
            cumulative_dv_penalty: 0.3,
            projected_miss_reward: 0.0002,
            milestone_reward: 0.4,
        }
    }

    /// Evaluation reward shaping coefficients matching Table 7.
    pub fn evaluation() -> Self {
        Self {
            collision_penalty: 50.0,
            survival_reward: 2.0,
            distance_shaping_coeff: 1.0,
            coast_bonus: 3.0,
            delta_v_linear_cost: 0.5,
            delta_v_quadratic_cost: 0.05,
            large_burn_penalty: 0.5,
            smoothness_penalty: 0.0005,
            cumulative_dv_penalty: 0.02,
            projected_miss_reward: 2.0,
            milestone_reward: 10.0,
        }
    }
}

/// Simulation scenario configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AvoidanceConfig {
    /// Integration time step $\Delta t$ in seconds (default $1.0\text{ s}$).
    pub dt_seconds: f64,
    /// Maximum thrust acceleration authority in $\text{m/s}^2$ ($0.15$).
    pub max_thrust: f64,
    /// Specific impulse in seconds ($300.0$).
    pub specific_impulse: f64,
    /// Standard gravitational acceleration $g_0$ in $\text{m/s}^2$.
    pub g0: f64,
    /// Collision radius threshold in meters ($10,000.0\text{ m}$).
    pub collision_distance_m: f64,
    /// Safe buffer zone radius beyond collision distance in meters ($3,000.0\text{ m}$).
    pub safe_zone_buffer_m: f64,
    /// Probability that scenario initial condition is forced onto a collision course.
    pub collision_course_probability: f64,
    /// Debris physical radius in meters (from Table 1 curriculum or Table 9 eval).
    pub debris_radius_m: f64,
    /// Lookahead encounter time-to-collision steps (default 250 from configs/eval.yaml).
    pub collision_ttc_steps: Option<usize>,
    /// Reward shaping hyperparameters.
    pub reward: RewardCoefficients,
}

impl Default for AvoidanceConfig {
    fn default() -> Self {
        Self {
            dt_seconds: 1.0,
            max_thrust: 5.0,
            specific_impulse: SPECIFIC_IMPULSE_S,
            g0: G0_STANDARD,
            collision_distance_m: DEFAULT_COLLISION_DISTANCE_M,
            safe_zone_buffer_m: DEFAULT_SAFE_BUFFER_M,
            collision_course_probability: 1.0,
            debris_radius_m: 50.0,
            collision_ttc_steps: Some(250),
            reward: RewardCoefficients::evaluation(),
        }
    }
}
