//! Astrodynamics Gym simulation environment for satellite collision avoidance.
//!
//! Grounded in Section IV-A & IV-B of Luna et al. (2026):
//! - Full 3-body dynamics integration with Earth, Moon, and Sun
//! - Continuous 3D action space $\mathbf{a} \in [-1, 1]^3$
//! - High-dimensional state observation space $\mathbb{R}^{307}$
//! - Conjunction scenario generation matching training curriculum and evaluation benchmarks

use crate::conjunction::assess_conjunction;
use crate::dynamics::step_dynamics;
use crate::reward::calculate_step_reward;
use crate::types::{
    Action3D, AvoidanceConfig, DebrisObject, RewardBreakdown, SatelliteState, TerminationReason,
    Vector3D, EARTH_RADIUS_M, MAX_DEBRIS_COUNT, MU_EARTH, OBSERVATION_DIM,
};

/// Step result information container.
#[derive(Debug, Clone, PartialEq)]
pub struct StepInfo {
    /// Instantaneous minimum distance to nearest debris in meters.
    pub min_distance_m: f64,
    /// Velocity increment $\Delta v$ applied in the current step in m/s.
    pub step_delta_v: f64,
    /// Cumulative $\Delta v$ expended so far in m/s.
    pub cumulative_delta_v: f64,
    /// Remaining propellant mass in kg.
    pub remaining_fuel_kg: f64,
    /// True if collision occurred in this step.
    pub collision_occurred: bool,
    /// Episode termination reason (if episode concluded).
    pub termination_reason: Option<TerminationReason>,
    /// Granular breakdown of individual reward terms.
    pub reward_breakdown: RewardBreakdown,
}

/// Simple fast PRNG (SplitMix64) for deterministic scenario generation without heavy dependencies.
#[derive(Debug, Clone)]
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed },
        }
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    #[inline]
    pub fn gen_range_f64(&mut self, low: f64, high: f64) -> f64 {
        low + self.next_f64() * (high - low)
    }

    #[inline]
    pub fn gen_range_usize(&mut self, low: usize, high: usize) -> usize {
        if low >= high {
            return low;
        }
        low + (self.next_u64() as usize % (high - low))
    }
}

impl rand::RngCore for SimpleRng {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        SimpleRng::next_u64(self)
    }

    #[inline]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        let mut idx = 0;
        while idx < dest.len() {
            let bytes = self.next_u64().to_le_bytes();
            let n = (dest.len() - idx).min(8);
            dest[idx..idx + n].copy_from_slice(&bytes[..n]);
            idx += n;
        }
    }

    #[inline]
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

/// High-fidelity satellite collision avoidance simulation environment.
#[derive(Debug, Clone)]
pub struct SatelliteAvoidanceEnv {
    /// Environment physical and reward configuration.
    pub config: AvoidanceConfig,
    /// Current satellite state.
    pub satellite: SatelliteState,
    /// Active debris field.
    pub debris_field: Vec<DebrisObject>,
    /// Index of designated collision target debris (if in collision scenario).
    pub target_debris_idx: Option<usize>,
    /// Whether current episode is set to an active collision course.
    pub is_collision_course: bool,
    /// Current episode time step counter.
    pub current_step: usize,
    /// Elapsed physical simulation time in seconds.
    pub elapsed_time_s: f64,
    /// Previous commanded action (for action smoothness penalty).
    pub prev_action: Action3D,
    /// Velocity increment applied in the most recent step ($\text{m/s}$).
    pub last_delta_v: f64,
    /// Cumulative velocity expenditure across episode ($\text{m/s}$).
    pub cumulative_delta_v: f64,
    /// Relative closing speed in previous step ($\text{m/s}$).
    pub prev_closing_speed_mps: Option<f64>,
    /// Minimum separation distance observed across the entire episode ($\text{m}$).
    pub min_observed_distance_m: f64,
    /// Accumulated reward components across the entire episode.
    pub cumulative_reward_breakdown: RewardBreakdown,
    /// Orbital period of reference orbit in seconds.
    pub orbital_period_s: f64,
}

impl SatelliteAvoidanceEnv {
    /// Creates a new environment with the specified configuration.
    pub fn new(config: AvoidanceConfig) -> Self {
        let default_sat = SatelliteState::new_leo_equatorial(700_000.0);
        let r = default_sat.position.norm();
        let orbital_period = 2.0 * core::f64::consts::PI * (r.powi(3) / MU_EARTH).sqrt();

        Self {
            config,
            satellite: default_sat,
            debris_field: Vec::new(),
            target_debris_idx: None,
            is_collision_course: false,
            current_step: 0,
            elapsed_time_s: 0.0,
            prev_action: Action3D::zero(),
            last_delta_v: 0.0,
            cumulative_delta_v: 0.0,
            prev_closing_speed_mps: None,
            min_observed_distance_m: f64::INFINITY,
            cumulative_reward_breakdown: RewardBreakdown::default(),
            orbital_period_s: orbital_period,
        }
    }

    /// Resets the environment deterministically with a random seed.
    ///
    /// Generates LEO debris field, sets up forced near head-on encounter if triggered,
    /// and returns the 307-dimensional initial observation vector.
    pub fn reset(&mut self, seed: u64) -> [f32; OBSERVATION_DIM] {
        let mut rng = SimpleRng::new(seed);

        self.current_step = 0;
        self.elapsed_time_s = 0.0;
        self.prev_action = Action3D::zero();
        self.last_delta_v = 0.0;
        self.cumulative_delta_v = 0.0;
        self.prev_closing_speed_mps = None;
        self.min_observed_distance_m = f64::INFINITY;
        self.cumulative_reward_breakdown = RewardBreakdown::default();

        // 1. Generate random debris field in LEO (altitude 160 km to 2000 km)
        let num_debris = rng.gen_range_usize(1, MAX_DEBRIS_COUNT + 1);
        self.debris_field.clear();

        for _ in 0..num_debris {
            let alt = rng.gen_range_f64(160_000.0, 2_000_000.0);
            let theta = rng.gen_range_f64(0.0, 2.0 * core::f64::consts::PI);
            let phi = rng.gen_range_f64(0.0, core::f64::consts::PI);

            let r = EARTH_RADIUS_M + alt;
            let x = r * phi.sin() * theta.cos();
            let y = r * phi.sin() * theta.sin();
            let z = r * phi.cos();

            self.debris_field.push(DebrisObject::new(
                Vector3D::new(x, y, z),
                Vector3D::zero(),
                self.config.debris_radius_m,
            ));
        }

        // 2. Determine if collision course scenario is activated
        self.is_collision_course = rng.next_f64() < self.config.collision_course_probability;
        self.target_debris_idx = None;

        if self.is_collision_course && !self.debris_field.is_empty() {
            let tgt_idx = rng.gen_range_usize(0, self.debris_field.len());
            self.target_debris_idx = Some(tgt_idx);

            let tgt_debris = &self.debris_field[tgt_idx];
            let dir_vec = tgt_debris.position.normalize_or_zero();

            let base_offset = (self.config.debris_radius_m + 2000.0).max(20000.0);
            let start_offset = base_offset.max(20000.0);

            let steps_to_hit = self.config.collision_ttc_steps.unwrap_or(250) as f64;
            let mut approach_t = steps_to_hit * self.config.dt_seconds;

            // Feasibility check: ensure lateral displacement authority can achieve clearance
            let clearance_needed = self.config.collision_distance_m + self.config.safe_zone_buffer_m;
            let achievable_disp = 0.5 * self.config.max_thrust * (approach_t * approach_t);

            if achievable_disp < clearance_needed + 500.0 {
                let t_needed = (2.0 * (clearance_needed + 500.0) / self.config.max_thrust.max(1e-9)).sqrt();
                approach_t = t_needed.min(1200.0);
            }

            let relative_speed = start_offset / approach_t.max(1e-6);
            let v0 = dir_vec * (-relative_speed);
            let p0 = tgt_debris.position + dir_vec * start_offset;

            self.satellite = SatelliteState {
                position: p0,
                velocity: v0,
                total_mass_kg: 1000.0,
                fuel_mass_kg: 500.0,
            };
        } else {
            self.satellite = SatelliteState::new_leo_equatorial(700_000.0);
        }

        let r = self.satellite.position.norm();
        self.orbital_period_s = 2.0 * core::f64::consts::PI * (r.powi(3) / MU_EARTH).sqrt();

        let mut min_d = f64::INFINITY;
        for deb in &self.debris_field {
            let d = (deb.position - self.satellite.position).norm();
            if d < min_d {
                min_d = d;
            }
        }
        self.min_observed_distance_m = min_d;

        self.get_observation()
    }

    /// Performs one simulation step: applies thrust, integrates orbit, assesses conjunctions, and computes reward.
    pub fn step(&mut self, action: Action3D) -> ([f32; OBSERVATION_DIM], f64, bool, StepInfo) {
        self.current_step += 1;

        // 1. Action change norm for jitter penalty
        let action_diff = action.to_vector() - self.prev_action.to_vector();
        let action_change_norm = action_diff.norm();
        self.prev_action = action;

        // 2. Step orbital physical dynamics and propellant consumption
        let (step_dv, _fuel_consumed) = step_dynamics(
            &mut self.satellite,
            action,
            self.elapsed_time_s,
            self.config.max_thrust,
            self.config.dt_seconds,
        );

        self.last_delta_v = step_dv;
        self.cumulative_delta_v += step_dv;
        self.elapsed_time_s += self.config.dt_seconds;

        // 3. Conjunction assessment with all debris
        let mut min_distance = f64::INFINITY;
        for deb in &self.debris_field {
            let dist = (deb.position - self.satellite.position).norm();
            if dist < min_distance {
                min_distance = dist;
            }
        }
        if min_distance < self.min_observed_distance_m {
            self.min_observed_distance_m = min_distance;
        }

        // Relative kinematics to target debris (if any)
        let (closing_speed, proj_miss) = if let Some(idx) = self.target_debris_idx {
            if idx < self.debris_field.len() {
                let m = assess_conjunction(
                    self.satellite.position,
                    self.satellite.velocity,
                    self.debris_field[idx].position,
                    self.debris_field[idx].velocity,
                    self.debris_field[idx].radius_m,
                    self.config.collision_distance_m,
                );
                (Some(m.closing_speed_mps), Some(m.projected_miss_m))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // 4. Calculate reward via Algorithm 3
        let rwd_ctx = crate::reward::RewardContext {
            min_distance_m: min_distance,
            step_delta_v: step_dv,
            action_change_norm,
            cumulative_delta_v: self.cumulative_delta_v,
            step: self.current_step,
            closing_speed_mps: closing_speed,
            prev_closing_speed_mps: self.prev_closing_speed_mps,
            projected_miss_m: proj_miss,
            collision_threshold_m: self.config.collision_distance_m,
            safe_zone_buffer_m: self.config.safe_zone_buffer_m,
        };
        let (step_reward_breakdown, is_collision) = calculate_step_reward(&rwd_ctx, &self.config.reward);

        self.cumulative_reward_breakdown.add(&step_reward_breakdown);
        self.prev_closing_speed_mps = closing_speed;

        // 5. Termination checks
        let mut done = false;
        let mut termination_reason = None;

        if is_collision {
            done = true;
            termination_reason = Some(TerminationReason::Collision);
        } else if self.satellite.fuel_mass_kg <= 0.0 {
            done = true;
            termination_reason = Some(TerminationReason::FuelDepleted);
        } else if self.elapsed_time_s >= self.orbital_period_s {
            done = true;
            termination_reason = Some(TerminationReason::OrbitComplete);
        }

        let info = StepInfo {
            min_distance_m: min_distance,
            step_delta_v: step_dv,
            cumulative_delta_v: self.cumulative_delta_v,
            remaining_fuel_kg: self.satellite.fuel_mass_kg,
            collision_occurred: is_collision,
            termination_reason,
            reward_breakdown: step_reward_breakdown,
        };

        (self.get_observation(), step_reward_breakdown.total, done, info)
    }

    /// Constructs the normalized 307-dimensional observation vector.
    ///
    /// Layout:
    /// - `obs[0..3]`: Satellite ECI position $[x, y, z]$
    /// - `obs[3..6]`: Satellite ECI velocity $[v_x, v_y, v_z]$
    /// - `obs[6]`: Residual fuel mass $m_{\text{fuel}}$
    /// - `obs[7..307]`: Debris Cartesian positions (up to 100 objects, zero-padded)
    pub fn get_observation(&self) -> [f32; OBSERVATION_DIM] {
        let mut obs = [0.0f32; OBSERVATION_DIM];

        obs[0] = self.satellite.position.x as f32;
        obs[1] = self.satellite.position.y as f32;
        obs[2] = self.satellite.position.z as f32;

        obs[3] = self.satellite.velocity.x as f32;
        obs[4] = self.satellite.velocity.y as f32;
        obs[5] = self.satellite.velocity.z as f32;

        obs[6] = self.satellite.fuel_mass_kg as f32;

        for (i, deb) in self.debris_field.iter().take(MAX_DEBRIS_COUNT).enumerate() {
            let offset = 7 + i * 3;
            obs[offset] = deb.position.x as f32;
            obs[offset + 1] = deb.position.y as f32;
            obs[offset + 2] = deb.position.z as f32;
        }

        obs
    }
}
