//! Global orbital reference structure construction and residual modeling.
//!
//! Grounded in Section 3.2 of the AutoOrbit paper (KDD 2026):
//! - **Reference Orbit Construction (Eq. 1)**: Computes mean recurring trajectory by phase-averaging
//!   historical in-orbit observations over the orbital recurrence period $T_{rec}$.
//! - **Residual Learning Strategy (Eqs. 2–3)**: Models deviation residuals $r_{res}(t) = s_{obs}(t) - s_{ref}(t)$
//!   to eliminate dynamic range issues, accelerate convergence, and prevent long-horizon drift.

use crate::autoorbit::types::{NormalizationStats, StateVector};

/// Represents a precomputed or analytically generated global reference orbit backbone.
#[derive(Debug, Clone)]
pub struct ReferenceOrbit {
    /// Recurrence period $T_{rec}$ in seconds (e.g., repeating ground-track cycle).
    pub recurrence_period_s: f64,
    /// Sampling cadence in seconds between consecutive orbital epochs (e.g., 10.0 s).
    pub cadence_s: f64,
    /// Sequence of reference states spanning at least one recurrence cycle.
    pub states: Vec<StateVector>,
}

impl ReferenceOrbit {
    /// Constructs a new reference orbit from a discrete sequence of reference states.
    pub fn new(states: Vec<StateVector>, cadence_s: f64) -> Self {
        let recurrence_period_s = states.len() as f64 * cadence_s;
        Self {
            recurrence_period_s,
            cadence_s,
            states,
        }
    }

    /// Constructs a reference orbit by phase-averaging $N$ historical orbital cycles (Eq. 1):
    ///
    /// $$s_{ref}(t_k) = \frac{1}{N} \sum_{i=1}^N s_{obs}(t_k + i \cdot T_{rec})$$
    pub fn from_phase_averaging(
        observations: &[StateVector],
        cycle_len: usize,
        cadence_s: f64,
    ) -> Self {
        assert!(cycle_len > 0, "cycle_len must be positive");
        let num_cycles = observations.len() / cycle_len;
        assert!(num_cycles > 0, "observations must contain at least one complete cycle");

        let mut ref_states = Vec::with_capacity(cycle_len);

        for phase in 0..cycle_len {
            let mut sum = StateVector::zero();
            for cycle in 0..num_cycles {
                let idx = cycle * cycle_len + phase;
                sum = sum + observations[idx];
            }
            ref_states.push(sum * (1.0 / num_cycles as f64));
        }

        Self {
            recurrence_period_s: cycle_len as f64 * cadence_s,
            cadence_s,
            states: ref_states,
        }
    }

    /// Returns the reference state at index `t`, wrapping periodically over the recurrence cycle.
    pub fn state_at_step(&self, step: usize) -> StateVector {
        if self.states.is_empty() {
            return StateVector::zero();
        }
        let wrapped_idx = step % self.states.len();
        self.states[wrapped_idx]
    }

    /// Extracts the residual vector $r_{res}(t) = s_{obs}(t) - s_{ref}(t)$ (Eq. 2).
    pub fn extract_residual(&self, obs: &StateVector, step: usize) -> StateVector {
        *obs - self.state_at_step(step)
    }

    /// Reconstructs the full predicted orbital state $s_{pred}(t) = s_{ref}(t) + r_{FNO}(t)$ (Eq. 3).
    pub fn reconstruct_state(&self, pred_residual: &StateVector, step: usize) -> StateVector {
        self.state_at_step(step) + *pred_residual
    }

    /// Computes empirical Z-score normalization statistics from historical residual sequences.
    pub fn compute_normalization_stats(residuals: &[StateVector]) -> NormalizationStats {
        if residuals.is_empty() {
            return NormalizationStats::default();
        }

        let n = residuals.len() as f64;
        let mut sum = [0.0; 6];
        for res in residuals {
            let arr = res.to_array();
            for i in 0..6 {
                sum[i] += arr[i];
            }
        }

        let mut mean = [0.0; 6];
        for i in 0..6 {
            mean[i] = sum[i] / n;
        }

        let mut var_sum = [0.0; 6];
        for res in residuals {
            let arr = res.to_array();
            for i in 0..6 {
                let diff = arr[i] - mean[i];
                var_sum[i] += diff * diff;
            }
        }

        let mut std = [1.0; 6];
        for i in 0..6 {
            let variance = var_sum[i] / n;
            std[i] = variance.sqrt().max(1e-8);
        }

        NormalizationStats::new(mean, std)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_averaging_and_reconstruction() {
        let cycle_len = 10;
        let mut obs = Vec::new();
        for cycle in 0..3 {
            for phase in 0..cycle_len {
                let x = (phase as f64) * 100.0 + (cycle as f64);
                obs.push(StateVector::new(x, 0.0, 0.0, 1.0, 0.0, 0.0));
            }
        }

        let ref_orbit = ReferenceOrbit::from_phase_averaging(&obs, cycle_len, 10.0);
        assert_eq!(ref_orbit.states.len(), cycle_len);

        // For phase 0: values are 0, 1, 2 -> mean = 1.0
        let ref_0 = ref_orbit.state_at_step(0);
        assert!((ref_0.x - 1.0).abs() < 1e-10);

        // Residual extraction and reconstruction
        let sample_obs = StateVector::new(105.0, 10.0, 20.0, 1.0, 2.0, 3.0);
        let res = ref_orbit.extract_residual(&sample_obs, 0);
        assert!((res.x - 104.0).abs() < 1e-10);

        let reconstructed = ref_orbit.reconstruct_state(&res, 0);
        assert!((reconstructed.x - sample_obs.x).abs() < 1e-10);
    }
}
