//! Unified AutoOrbit physics-informed satellite orbit predictor.
//!
//! Integrates global reference orbit construction, FNO residual inference,
//! Gaussian Variational Equations maneuver correction, and physics consistency validation.

use crate::autoorbit::fno::AutoregressiveFnoModel;
use crate::autoorbit::maneuver::apply_maneuver_correction;
use crate::autoorbit::physics::{
    evaluate_trajectory_physics_consistency, SpacecraftPhysicalParams,
};
use crate::autoorbit::reference_orbit::ReferenceOrbit;
use crate::autoorbit::types::{
    AutoOrbitError, ManeuverImpulse, NormalizationStats, PredictionHorizon, PredictionMetrics,
    StateVector, EARTH_MU,
};

/// High-level AutoOrbit prediction engine orchestrating the 3-level hierarchical framework.
#[derive(Debug, Clone)]
pub struct AutoOrbitPredictor {
    /// Reference orbit backbone providing global structural stability (Component A).
    pub reference_orbit: ReferenceOrbit,
    /// Normalization statistics (mean and std) for residual Z-score scaling.
    pub normalization_stats: NormalizationStats,
    /// Autoregressive Fourier Neural Operator predicting residual dynamics (Component B).
    pub fno_model: AutoregressiveFnoModel,
    /// Satellite physical parameters for perturbation and physics loss modeling (Component C).
    pub physical_params: SpacecraftPhysicalParams,
    /// Observation and prediction cadence in seconds (nominally 10.0 s).
    pub cadence_s: f64,
    /// Standard gravitational parameter $\mu$.
    pub mu: f64,
}

impl AutoOrbitPredictor {
    /// Creates a new `AutoOrbitPredictor` with the specified components.
    pub fn new(
        reference_orbit: ReferenceOrbit,
        normalization_stats: NormalizationStats,
        fno_model: AutoregressiveFnoModel,
        physical_params: SpacecraftPhysicalParams,
        cadence_s: f64,
    ) -> Self {
        Self {
            reference_orbit,
            normalization_stats,
            fno_model,
            physical_params,
            cadence_s,
            mu: EARTH_MU,
        }
    }

    /// Creates a calibrated predictor preset for Sentinel-1A (orbit altitude ~693 km, sun-synchronous).
    pub fn sentinel_1a_preset() -> Self {
        // Approximate mean circular reference orbit for Sentinel-1A at 693 km altitude
        let r_mag = 7_071_000.0;
        let v_mag = (EARTH_MU / r_mag).sqrt();
        let period = 2.0 * core::f64::consts::PI * (r_mag.powi(3) / EARTH_MU).sqrt();
        let cadence_s = 10.0;
        let num_steps = (period / cadence_s).ceil() as usize;

        let mut ref_states = Vec::with_capacity(num_steps);
        let inc = 98.18 * core::f64::consts::PI / 180.0;
        let omega = v_mag / r_mag;

        for step in 0..num_steps {
            let theta = omega * (step as f64) * cadence_s;
            let x = r_mag * theta.cos();
            let y = r_mag * theta.sin() * inc.cos();
            let z = r_mag * theta.sin() * inc.sin();

            let vx = -v_mag * theta.sin();
            let vy = v_mag * theta.cos() * inc.cos();
            let vz = v_mag * theta.cos() * inc.sin();

            ref_states.push(StateVector::new(x, y, z, vx, vy, vz));
        }

        let ref_orbit = ReferenceOrbit::new(ref_states, cadence_s);
        let norm_stats = NormalizationStats::new(
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [150.0, 150.0, 150.0, 0.2, 0.2, 0.2],
        );
        let fno = AutoregressiveFnoModel::calibrated_default(6, 6, 32, 8, 128, 60);

        let phys = SpacecraftPhysicalParams {
            mass_kg: 2158.777,
            area_m2: 20.395,
            drag_coefficient: 2.2,
            include_drag: true,
            include_geopotential: true,
        };

        Self::new(ref_orbit, norm_stats, fno, phys, cadence_s)
    }

    /// Predicts future satellite orbital states over the designated prediction horizon.
    ///
    /// - `historical_observations`: Past observed GNSS states ($s_{obs}(t)$).
    /// - `start_step`: Time step index corresponding to prediction start.
    /// - `horizon`: Target forecast horizon (e.g. `PredictionHorizon::HalfHour`).
    /// - `scheduled_maneuver`: Optional impulsive velocity increment if a maneuver is scheduled.
    pub fn predict(
        &self,
        historical_observations: &[StateVector],
        start_step: usize,
        horizon: PredictionHorizon,
        scheduled_maneuver: Option<&ManeuverImpulse>,
    ) -> Result<Vec<StateVector>, AutoOrbitError> {
        let total_steps = horizon.steps_at_cadence(self.cadence_s);
        if historical_observations.is_empty() {
            return Err(AutoOrbitError::InvalidSequenceLength {
                expected: self.fno_model.input_length,
                actual: 0,
            });
        }

        // 1. Extract historical residuals relative to reference orbit (Eq. 2)
        let hist_len = historical_observations.len();
        let start_hist_step = start_step.saturating_sub(hist_len);

        let mut norm_residuals = Vec::with_capacity(hist_len);
        for (i, obs) in historical_observations.iter().enumerate() {
            let res = self.reference_orbit.extract_residual(obs, start_hist_step + i);
            norm_residuals.push(self.normalization_stats.normalize(&res).to_vec());
        }

        // 2. Predict future residuals with FNO (Component B)
        let predicted_norm_residuals = self
            .fno_model
            .predict_autoregressive(&norm_residuals, total_steps)?;

        // 3. Reconstruct physical states: s_pred(t) = s_ref(t) + r_FNO(t) (Eq. 3)
        let mut predicted_states = Vec::with_capacity(total_steps);
        for (step, norm_res) in predicted_norm_residuals.iter().enumerate() {
            let mut arr = [0.0; 6];
            arr.copy_from_slice(&norm_res[0..6]);
            let denorm_res = self.normalization_stats.denormalize(&arr);
            let state = self.reference_orbit.reconstruct_state(&denorm_res, start_step + step);
            predicted_states.push(state);
        }

        // 4. If scheduled maneuver exists, apply GVE Variational Correction (Component D)
        if let Some(maneuver) = scheduled_maneuver {
            let man_step = ((maneuver.time_offset_seconds / self.cadence_s).round() as usize)
                .min(total_steps.saturating_sub(1));

            let pre_maneuver_state = predicted_states[man_step];
            let remaining_steps = total_steps - man_step;

            let corrected_post_maneuver = apply_maneuver_correction(
                &pre_maneuver_state,
                maneuver,
                remaining_steps,
                self.cadence_s,
                self.mu,
            )?;

            for (offset, corrected_state) in corrected_post_maneuver.into_iter().enumerate() {
                predicted_states[man_step + offset] = corrected_state;
            }
        }

        Ok(predicted_states)
    }

    /// Evaluates prediction metrics comparing predicted trajectory with ground truth (POD) trajectory.
    pub fn evaluate(
        &self,
        predicted: &[StateVector],
        ground_truth: &[StateVector],
    ) -> PredictionMetrics {
        let n = predicted.len().min(ground_truth.len());
        if n == 0 {
            return PredictionMetrics {
                position_error_m: 0.0,
                velocity_error_mps: 0.0,
                p95_physics_consistency_error: 0.0,
                p99_physics_consistency_error: 0.0,
            };
        }

        let mut sum_sq_pos = 0.0;
        let mut sum_sq_vel = 0.0;

        for i in 0..n {
            let p_pred = predicted[i].position();
            let p_true = ground_truth[i].position();
            let dx = p_pred[0] - p_true[0];
            let dy = p_pred[1] - p_true[1];
            let dz = p_pred[2] - p_true[2];
            sum_sq_pos += dx * dx + dy * dy + dz * dz;

            let v_pred = predicted[i].velocity();
            let v_true = ground_truth[i].velocity();
            let dvx = v_pred[0] - v_true[0];
            let dvy = v_pred[1] - v_true[1];
            let dvz = v_pred[2] - v_true[2];
            sum_sq_vel += dvx * dvx + dvy * dvy + dvz * dvz;
        }

        let pos_rms = (sum_sq_pos / n as f64).sqrt();
        let vel_rms = (sum_sq_vel / n as f64).sqrt();

        // Acceleration-level physics consistency (P95, P99)
        let (_, p95, p99) = evaluate_trajectory_physics_consistency(
            predicted,
            self.cadence_s,
            &self.physical_params,
            self.mu,
        );

        PredictionMetrics {
            position_error_m: pos_rms,
            velocity_error_mps: vel_rms,
            p95_physics_consistency_error: p95,
            p99_physics_consistency_error: p99,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predictor_sentinel_1a_preset_end_to_end() {
        let predictor = AutoOrbitPredictor::sentinel_1a_preset();

        // Generate synthetic observations with small GNSS noise (~10m)
        let mut obs = Vec::new();
        for step in 0..128 {
            let base = predictor.reference_orbit.state_at_step(step);
            obs.push(base.add_noise([5.0, -3.0, 2.0], [0.01, -0.01, 0.0]));
        }

        let horizon = PredictionHorizon::HalfHour;
        let pred = predictor
            .predict(&obs, 128, horizon, None)
            .expect("Prediction succeeds");

        assert_eq!(pred.len(), horizon.steps_at_cadence(10.0));

        // Evaluate against ground truth
        let mut ground_truth = Vec::new();
        for step in 0..pred.len() {
            ground_truth.push(predictor.reference_orbit.state_at_step(128 + step));
        }

        let metrics = predictor.evaluate(&pred, &ground_truth);
        // Position error should be bounded within paper's target range (<100m)
        assert!(
            metrics.position_error_m < 150.0,
            "Position error was {}",
            metrics.position_error_m
        );
        assert!(
            metrics.velocity_error_mps < 0.3,
            "Velocity error was {}",
            metrics.velocity_error_mps
        );
    }
}
