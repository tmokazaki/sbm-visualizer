//! Conjunction assessment, closest-approach prediction, and kinematic risk scoring.
//!
//! Grounded in Section IV-C1 and Equation (9) of Luna et al. (2026):
//! - **Time-to-Closest-Approach (TCA)**: Linear projection along relative velocity
//! - **Projected Miss Distance ($d_{pm}$)**: Separation at predicted TCA
//! - **Relative Closing Speed ($v_{\text{close}}$)**: Line-of-sight approach rate
//! - **Multi-Factor Risk Score ($\rho_i$)**: Logistic blend of spatial and temporal urgency

use crate::types::{ConjunctionMetrics, DebrisObject, SatelliteState, Vector3D};

/// Critical time threshold $\tau_{\text{crit}}$ for urgent maneuvers in seconds ($240\text{ s}$).
pub const TAU_CRITICAL_SECONDS: f64 = 240.0;

/// Spatial scaling parameter $\alpha_d$ in risk scoring formula (3.0).
pub const ALPHA_D_SPATIAL: f64 = 3.0;

/// Temporal scaling parameter $\alpha_\tau$ in risk scoring formula (2.5).
pub const ALPHA_TAU_TEMPORAL: f64 = 2.5;

/// Standard sigmoid / logistic activation function $\sigma(z) = \frac{1}{1 + e^{-z}}$.
#[inline]
pub fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

/// Evaluates relative kinematics and conjunction metrics between satellite and a debris object.
#[inline]
pub fn assess_conjunction(
    sat_pos: Vector3D,
    sat_vel: Vector3D,
    deb_pos: Vector3D,
    deb_vel: Vector3D,
    debris_radius_m: f64,
    collision_threshold_m: f64,
) -> ConjunctionMetrics {
    let rel_pos = deb_pos - sat_pos;
    let rel_vel = deb_vel - sat_vel;
    let distance = rel_pos.norm();

    if distance < 1e-6 {
        return ConjunctionMetrics {
            distance_m: 0.0,
            tca_seconds: 0.0,
            projected_miss_m: 0.0,
            closing_speed_mps: 0.0,
            hazard_score: 1.0,
        };
    }

    // Line-of-sight closing speed (>0 means closing)
    let closing_speed = -rel_pos.dot(&rel_vel) / distance;

    // Linear projection time-to-closest-approach (TCA)
    let v_rel_sq = rel_vel.norm_squared();
    let (tca, proj_miss) = if closing_speed > 1e-3 && v_rel_sq > 1e-12 {
        let t_star = (-rel_pos.dot(&rel_vel) / v_rel_sq).clamp(0.0, 1200.0);
        let proj_pos = rel_pos + rel_vel * t_star;
        (t_star, proj_pos.norm())
    } else {
        (f64::INFINITY, distance)
    };

    // Risk hazard score rho_i (Equation 9)
    let r_eff = collision_threshold_m.max(debris_radius_m);
    let tau_term = if tca.is_finite() {
        (TAU_CRITICAL_SECONDS - tca) / TAU_CRITICAL_SECONDS
    } else {
        -1.0
    };
    let z = ALPHA_D_SPATIAL * (r_eff / distance) + ALPHA_TAU_TEMPORAL * tau_term;
    let hazard_score = sigmoid(z);

    ConjunctionMetrics {
        distance_m: distance,
        tca_seconds: tca,
        projected_miss_m: proj_miss,
        closing_speed_mps: closing_speed,
        hazard_score,
    }
}

/// Identifies the most critical debris object (highest risk hazard score $\rho_{i^*}$) among active debris field.
///
/// Implements Equation (10) index selection:
/// $$i^* = \arg\max_i \rho_i$$
pub fn find_most_critical_debris(
    satellite: &SatelliteState,
    debris_field: &[DebrisObject],
    collision_threshold_m: f64,
) -> Option<(usize, ConjunctionMetrics)> {
    if debris_field.is_empty() {
        return None;
    }

    let mut best_idx = 0;
    let mut best_metrics = assess_conjunction(
        satellite.position,
        satellite.velocity,
        debris_field[0].position,
        debris_field[0].velocity,
        debris_field[0].radius_m,
        collision_threshold_m,
    );

    for (idx, debris) in debris_field.iter().enumerate().skip(1) {
        let metrics = assess_conjunction(
            satellite.position,
            satellite.velocity,
            debris.position,
            debris.velocity,
            debris.radius_m,
            collision_threshold_m,
        );

        if metrics.hazard_score > best_metrics.hazard_score {
            best_idx = idx;
            best_metrics = metrics;
        }
    }

    Some((best_idx, best_metrics))
}
