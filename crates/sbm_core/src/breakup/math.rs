//! Analytical equations and mathematical relationships for NASA EVOLVE 4.0 breakup physics.

use super::types::BreakupType;

/// Catastrophic disruption specific energy threshold in kJ/kg ($40\text{ J/g} = 40.0\text{ kJ/kg}$).
pub const CATASTROPHIC_THRESHOLD_KJ_PER_KG: f64 = 40.0;

/// Default hypersonic drag coefficient for space debris shards.
pub const DEFAULT_DRAG_COEFFICIENT: f64 = 2.2;

/// Calculates the average cross-sectional area $A_x$ ($\text{m}^2$) from characteristic length $L_c$ (m).
///
/// Implements Johnson et al. (2001), Section 3.3, Equations (8) and (9):
/// - $A_x = 0.540424 \cdot L_c^2$ for $L_c < 0.00167\text{ m}$
/// - $A_x = 0.556945 \cdot L_c^{2.0047077}$ for $L_c \ge 0.00167\text{ m}$
pub fn cross_sectional_area(lc: f64) -> f64 {
    if lc < 0.00167 {
        0.540424 * lc.powi(2)
    } else {
        0.556945 * lc.powf(2.0047077)
    }
}

/// Calculates the theoretical cumulative number of fragments $N(L_c \ge d)$ with $L_c \ge d$ (meters).
///
/// Implements Johnson et al. (2001):
/// - Collisions (Eq. 4): $N(L_c \ge d) = 0.10 \cdot M_{\text{eff}}^{0.75} \cdot d^{-1.71}$
/// - Explosions (Eq. 3): $N(L_c \ge d) = 6 \cdot S \cdot d^{-1.60}$
pub fn cumulative_fragment_count(
    effective_m: f64,
    lc_m: f64,
    breakup_type: BreakupType,
    s_scaling: f64,
) -> f64 {
    match breakup_type {
        BreakupType::Collision => 0.10 * effective_m.powf(0.75) * lc_m.powf(-1.71),
        BreakupType::Explosion => s_scaling * 6.0 * lc_m.powf(-1.60),
    }
}

/// Calculates the specific impact energy $E_p$ in $\text{kJ/kg}$ relative to the larger body.
///
/// $$E_p = \frac{\frac{1}{2} M_{\text{smaller}} v_{\text{imp}}^2}{M_{\text{larger}}} \quad [\text{J/kg}] = \frac{1000 \cdot M_{\text{smaller}} v_{\text{imp, km/s}}^2}{2 M_{\text{larger}}} \quad [\text{kJ/kg}]$$
pub fn specific_impact_energy(target_mass: f64, projectile_mass: f64, impact_speed_mps: f64) -> f64 {
    let m_smaller = target_mass.min(projectile_mass);
    let m_larger = target_mass.max(projectile_mass);
    if m_larger <= 0.0 {
        return 0.0;
    }
    let ep_j_per_kg = (0.5 * m_smaller * impact_speed_mps.powi(2)) / m_larger;
    ep_j_per_kg / 1000.0
}

/// Checks whether a collision is catastrophic ($E_p \ge 40.0\text{ kJ/kg}$).
pub fn is_catastrophic_collision(ep_kj_per_kg: f64) -> bool {
    ep_kj_per_kg >= CATASTROPHIC_THRESHOLD_KJ_PER_KG
}

/// Computes the destroyed mass in kilograms for a collision according to EVOLVE 4.0 (page 1379, col 2):
/// - Catastrophic: $M = M_1 + M_2$
/// - Non-catastrophic (cratering): $M = M_{\text{smaller}} \cdot v_{\text{imp, km/s}}$
pub fn collision_destroyed_mass(
    target_mass: f64,
    projectile_mass: f64,
    impact_speed_mps: f64,
    is_catastrophic: bool,
) -> f64 {
    let m_total = target_mass + projectile_mass;
    if is_catastrophic {
        m_total
    } else {
        let m_smaller = target_mass.min(projectile_mass);
        let v_imp_kms = impact_speed_mps / 1000.0;
        (m_smaller * v_imp_kms).min(m_total)
    }
}

/// Calculates the ballistic coefficient $B^* = \frac{1}{2} C_D (A/M)$ in $\text{m}^2/\text{kg}$.
pub fn ballistic_coefficient(am_ratio: f64, drag_coeff: Option<f64>) -> f64 {
    let cd = drag_coeff.unwrap_or(DEFAULT_DRAG_COEFFICIENT);
    0.5 * cd * am_ratio
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_sectional_area_continuity() {
        let lc_trans = 0.00167;
        let left = cross_sectional_area(lc_trans - 1e-7);
        let right = cross_sectional_area(lc_trans + 1e-7);
        assert!((left - right).abs() < 1e-6);
    }

    #[test]
    fn test_cumulative_fragment_count_collision() {
        let n = cumulative_fragment_count(1100.0, 0.10, BreakupType::Collision, 1.0);
        let expected = 0.10 * 1100.0f64.powf(0.75) * 0.10f64.powf(-1.71);
        assert!((n - expected).abs() < 1e-6);
    }

    #[test]
    fn test_specific_energy_and_regimes() {
        let ep = specific_impact_energy(1000.0, 100.0, 10_000.0);
        assert_eq!(ep, 5000.0);
        assert!(is_catastrophic_collision(ep));

        let m_cat = collision_destroyed_mass(1000.0, 100.0, 10_000.0, true);
        assert_eq!(m_cat, 1100.0);

        let ep_small = specific_impact_energy(1000.0, 1.0, 5000.0);
        assert_eq!(ep_small, 12.5);
        assert!(!is_catastrophic_collision(ep_small));

        let m_crater = collision_destroyed_mass(1000.0, 1.0, 5000.0, false);
        assert_eq!(m_crater, 5.0); // 1 kg * 5 km/s = 5 kg
    }
}
