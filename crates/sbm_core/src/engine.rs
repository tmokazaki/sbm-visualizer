//! Core NASA EVOLVE 4.0 breakup simulation engine and builder.

use crate::error::BreakupError;
use crate::math::{
    collision_destroyed_mass, cross_sectional_area, cumulative_fragment_count,
    is_catastrophic_collision, specific_impact_energy,
};
use crate::rng::{RngSource, SimpleRng};
use crate::sampling::{sample_am_ratio, sample_delta_v, sample_direction_isotropic};
use crate::types::{BreakupResult, BreakupType, Fragment, ObjectType};

/// Builder for constructing a configured [`BreakupEngine`].
#[derive(Debug, Clone)]
pub struct BreakupEngineBuilder {
    target_mass: f64,
    projectile_mass: f64,
    impact_speed: f64,
    target_type: ObjectType,
    projectile_type: ObjectType,
    breakup_type: BreakupType,
    explosion_scaling: f64,
    num_fragments: usize,
    size_skew: f64,
    min_size: f64,
    max_size: f64,
}

impl Default for BreakupEngineBuilder {
    fn default() -> Self {
        Self {
            target_mass: 1000.0,
            projectile_mass: 100.0,
            impact_speed: 10_000.0,
            target_type: ObjectType::Spacecraft,
            projectile_type: ObjectType::Spacecraft,
            breakup_type: BreakupType::Collision,
            explosion_scaling: 1.0,
            num_fragments: 1500,
            size_skew: 1.71,
            min_size: 0.01,
            max_size: 2.50,
        }
    }
}

impl BreakupEngineBuilder {
    /// Creates a new builder with default parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the target satellite mass in kilograms.
    pub fn target_mass(mut self, mass: f64) -> Self {
        self.target_mass = mass;
        self
    }

    /// Sets the projectile/impactor mass in kilograms.
    pub fn projectile_mass(mut self, mass: f64) -> Self {
        self.projectile_mass = mass;
        self
    }

    /// Sets the relative collision impact speed in meters per second.
    pub fn impact_speed(mut self, speed_mps: f64) -> Self {
        self.impact_speed = speed_mps;
        self
    }

    /// Sets the target object classification (Spacecraft or RocketBody).
    pub fn target_type(mut self, obj_type: ObjectType) -> Self {
        self.target_type = obj_type;
        self
    }

    /// Sets the projectile object classification (Spacecraft or RocketBody).
    pub fn projectile_type(mut self, obj_type: ObjectType) -> Self {
        self.projectile_type = obj_type;
        self
    }

    /// Sets the breakup event regime (Collision or Explosion).
    pub fn breakup_type(mut self, breakup_type: BreakupType) -> Self {
        self.breakup_type = breakup_type;
        self
    }

    /// Sets the explosion scaling factor $S$ (Johnson et al. 2001, Eq. 3).
    pub fn explosion_scaling(mut self, s: f64) -> Self {
        self.explosion_scaling = s;
        self
    }

    /// Sets the number of discrete sample fragments to generate.
    pub fn num_fragments(mut self, count: usize) -> Self {
        self.num_fragments = count;
        self
    }

    /// Sets the power-law size distribution exponent $\alpha$ (nominal 1.71 for collisions, 1.60 for explosions).
    pub fn size_skew(mut self, alpha: f64) -> Self {
        self.size_skew = alpha;
        self
    }

    /// Sets the minimum fragment characteristic length cutoff $L_{\min}$ in meters.
    pub fn min_size(mut self, min_size: f64) -> Self {
        self.min_size = min_size;
        self
    }

    /// Sets the maximum fragment characteristic length cutoff $L_{\max}$ in meters.
    pub fn max_size(mut self, max_size: f64) -> Self {
        self.max_size = max_size;
        self
    }

    /// Validates all parameters and builds a [`BreakupEngine`].
    pub fn build(self) -> Result<BreakupEngine, BreakupError> {
        if !self.target_mass.is_finite() || self.target_mass <= 0.0 {
            return Err(BreakupError::InvalidMass {
                parameter: "target_mass",
                value: self.target_mass,
            });
        }
        if !self.projectile_mass.is_finite() || self.projectile_mass <= 0.0 {
            return Err(BreakupError::InvalidMass {
                parameter: "projectile_mass",
                value: self.projectile_mass,
            });
        }
        if !self.impact_speed.is_finite() || self.impact_speed < 0.0 {
            return Err(BreakupError::InvalidVelocity {
                value: self.impact_speed,
            });
        }
        if self.num_fragments == 0 {
            return Err(BreakupError::InvalidFragmentCount {
                value: self.num_fragments,
            });
        }
        if !self.explosion_scaling.is_finite() || self.explosion_scaling <= 0.0 {
            return Err(BreakupError::InvalidScaling {
                value: self.explosion_scaling,
            });
        }
        if !self.min_size.is_finite() || !self.max_size.is_finite() || self.min_size <= 0.0 || self.max_size <= self.min_size {
            return Err(BreakupError::InvalidSizeRange {
                min_size: self.min_size,
                max_size: self.max_size,
            });
        }
        if !self.size_skew.is_finite() || self.size_skew <= 1.0 {
            return Err(BreakupError::InvalidPowerLawExponent {
                value: self.size_skew,
            });
        }

        Ok(BreakupEngine {
            target_mass: self.target_mass,
            projectile_mass: self.projectile_mass,
            impact_speed: self.impact_speed,
            target_type: self.target_type,
            projectile_type: self.projectile_type,
            breakup_type: self.breakup_type,
            explosion_scaling: self.explosion_scaling,
            num_fragments: self.num_fragments,
            size_skew: self.size_skew,
            min_size: self.min_size,
            max_size: self.max_size,
        })
    }
}

/// Core simulation engine for NASA EVOLVE 4.0 Standard Breakup Model.
#[derive(Debug, Clone)]
pub struct BreakupEngine {
    pub target_mass: f64,
    pub projectile_mass: f64,
    pub impact_speed: f64,
    pub target_type: ObjectType,
    pub projectile_type: ObjectType,
    pub breakup_type: BreakupType,
    pub explosion_scaling: f64,
    pub num_fragments: usize,
    pub size_skew: f64,
    pub min_size: f64,
    pub max_size: f64,
}

impl BreakupEngine {
    /// Returns a new fluent builder for configuring a [`BreakupEngine`].
    pub fn builder() -> BreakupEngineBuilder {
        BreakupEngineBuilder::new()
    }

    /// Convenience constructor with nominal collision parameters.
    pub fn new(target_mass: f64, projectile_mass: f64, impact_speed: f64) -> Result<Self, BreakupError> {
        Self::builder()
            .target_mass(target_mass)
            .projectile_mass(projectile_mass)
            .impact_speed(impact_speed)
            .build()
    }

    /// Executes the breakup simulation using a deterministic seed with the internal [`SimpleRng`].
    pub fn simulate(&self, seed: u64) -> BreakupResult {
        let mut rng = SimpleRng::new(seed);
        self.simulate_with_rng(&mut rng)
    }

    /// Executes the breakup simulation using any caller-provided [`RngSource`].
    pub fn simulate_with_rng<R: RngSource + ?Sized>(&self, rng: &mut R) -> BreakupResult {
        // 1. Catastrophic Disruption Criterion (Johnson et al. 2001, page 1379)
        let (is_catastrophic, ep_kj_per_kg, destroyed_mass) = match self.breakup_type {
            BreakupType::Collision => {
                let ep = specific_impact_energy(self.target_mass, self.projectile_mass, self.impact_speed);
                let is_cat = is_catastrophic_collision(ep);
                let m_dest = collision_destroyed_mass(self.target_mass, self.projectile_mass, self.impact_speed, is_cat);
                (is_cat, ep, m_dest)
            }
            BreakupType::Explosion => {
                let m_dest = (self.explosion_scaling * self.target_mass).min(self.target_mass);
                (true, 0.0, m_dest)
            }
        };

        let remnant_mass = if is_catastrophic {
            0.0
        } else {
            (self.target_mass.max(self.projectile_mass) - destroyed_mass).max(0.0)
        };

        // Theoretical yields
        let physical_yield_1cm = cumulative_fragment_count(
            destroyed_mass,
            0.01,
            self.breakup_type,
            self.explosion_scaling,
        );
        let ssn_trackable_yield_10cm = cumulative_fragment_count(
            destroyed_mass,
            0.10,
            self.breakup_type,
            self.explosion_scaling,
        );

        // 2. Sample sizes, areas, and unnormalized masses
        let mut sizes = Vec::with_capacity(self.num_fragments);
        let mut areas = Vec::with_capacity(self.num_fragments);
        let mut raw_masses = Vec::with_capacity(self.num_fragments);
        let mut origins = Vec::with_capacity(self.num_fragments);
        let mut obj_types = Vec::with_capacity(self.num_fragments);
        let mut total_raw_mass = 0.0;

        let total_input_mass = self.target_mass + self.projectile_mass;
        let target_ratio = self.target_mass / total_input_mass;

        for _ in 0..self.num_fragments {
            let u = rng.next_f64();
            let alpha = self.size_skew;
            let lc_raw = self.min_size * (1.0 - u).powf(-1.0 / (alpha - 1.0));
            let lc = lc_raw.min(self.max_size);

            let origin = if rng.next_f64() < target_ratio { 0 } else { 1 };
            let obj_type = if origin == 0 {
                self.target_type
            } else {
                self.projectile_type
            };

            // Sample chi = log10(A/M)
            let (_chi, am) = sample_am_ratio(rng, lc, obj_type);

            // Compute cross-sectional area Ax from Eqs. (8) & (9)
            let ax = cross_sectional_area(lc);

            // Individual raw mass from Eq. (10): M = Ax / (A/M)
            let raw_m = (ax / am).max(1e-9);

            sizes.push(lc);
            areas.push(ax);
            raw_masses.push(raw_m);
            origins.push(origin);
            obj_types.push(obj_type);
            total_raw_mass += raw_m;
        }

        // 3. Normalize masses to conserve destroyed mass, and sample EVOLVE 4.0 ejection velocities
        let mut fragments = Vec::with_capacity(self.num_fragments);

        for i in 0..self.num_fragments {
            let mass = (raw_masses[i] / total_raw_mass) * destroyed_mass;
            let lc = sizes[i];
            let ax = areas[i];
            let obj_type = obj_types[i];
            let origin = origins[i];

            // Recompute consistent A/M and chi based on normalized physical mass
            let actual_am = (ax / mass).max(1e-6);
            let chi = actual_am.log10();

            // Sample Delta-v conditioned on chi = log10(A/M) from EVOLVE 4.0
            let speed = sample_delta_v(rng, chi, self.breakup_type).clamp(5.0, 25_000.0);

            let contour_band = if speed < 250.0 {
                "<250 m/s (Deep Blue)"
            } else if speed < 500.0 {
                "250-500 m/s (Cyan)"
            } else if speed < 1000.0 {
                "500-1000 m/s (Green)"
            } else if speed < 1500.0 {
                "1000-1500 m/s (Yellow)"
            } else if speed < 2000.0 {
                "1500-2000 m/s (Orange)"
            } else {
                ">2000 m/s (Crimson Red)"
            };

            // Direction vector on unit sphere
            let [dx, dy, dz] = sample_direction_isotropic(rng);
            let vel_vec = [speed * dx, speed * dy, speed * dz];

            fragments.push(Fragment {
                id: i,
                size_m: lc,
                cross_section_m2: ax,
                am_ratio: actual_am,
                chi,
                mass_kg: mass,
                speed_mps: speed,
                vel_vec,
                origin,
                object_type: obj_type,
                contour_band,
            });
        }

        BreakupResult {
            fragments,
            is_catastrophic,
            specific_energy_kj_per_kg: ep_kj_per_kg,
            destroyed_mass_kg: destroyed_mass,
            remnant_mass_kg: remnant_mass,
            physical_yield_1cm,
            ssn_trackable_yield_10cm,
        }
    }
}

/// Backward compatibility alias for [`BreakupEngine`].
pub type SimpleBreakupEngine = BreakupEngine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_builder_validation() {
        // Negative mass should error
        assert!(BreakupEngine::builder().target_mass(-10.0).build().is_err());
        // Zero fragments should error
        assert!(BreakupEngine::builder().num_fragments(0).build().is_err());
        // Inverted size range should error
        assert!(BreakupEngine::builder().min_size(1.0).max_size(0.5).build().is_err());
        // Valid configuration should succeed
        assert!(BreakupEngine::builder().build().is_ok());
    }

    #[test]
    fn test_simulation_mass_conservation() {
        let engine = BreakupEngine::builder()
            .target_mass(1000.0)
            .projectile_mass(100.0)
            .impact_speed(10_000.0)
            .num_fragments(500)
            .build()
            .unwrap();

        let result = engine.simulate(42);
        assert!(result.is_catastrophic);
        assert_eq!(result.destroyed_mass_kg, 1100.0);
        assert_eq!(result.remnant_mass_kg, 0.0);
        assert_eq!(result.fragments.len(), 500);

        let total_mass = result.total_fragment_mass();
        assert!((total_mass - 1100.0).abs() < 1e-6);
    }

    #[test]
    fn test_simulation_cratering_remnant() {
        let engine = BreakupEngine::builder()
            .target_mass(1000.0)
            .projectile_mass(1.0)
            .impact_speed(5000.0)
            .num_fragments(200)
            .build()
            .unwrap();

        let result = engine.simulate(123);
        assert!(!result.is_catastrophic);
        assert_eq!(result.destroyed_mass_kg, 5.0);
        assert_eq!(result.remnant_mass_kg, 995.0);

        let total_mass = result.total_fragment_mass();
        assert!((total_mass - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_top_heaviest_and_fastest() {
        let engine = BreakupEngine::new(1000.0, 100.0, 10_000.0).unwrap();
        let result = engine.simulate(777);

        let heaviest = result.top_heaviest(3);
        assert_eq!(heaviest.len(), 3);
        assert!(heaviest[0].mass_kg >= heaviest[1].mass_kg);
        assert!(heaviest[1].mass_kg >= heaviest[2].mass_kg);

        let fastest = result.top_fastest(3);
        assert_eq!(fastest.len(), 3);
        assert!(fastest[0].speed_mps >= fastest[1].speed_mps);
        assert!(fastest[1].speed_mps >= fastest[2].speed_mps);
    }
}
