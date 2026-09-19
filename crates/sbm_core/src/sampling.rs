//! Stochastic sampling distributions according to NASA EVOLVE 4.0 specifications.

use std::f64::consts::PI;
use crate::rng::RngSource;
use crate::types::{BreakupType, ObjectType};

/// Samples a two-component Gaussian mixture distribution:
/// $$p(x) = \alpha \cdot \mathcal{N}(\mu_1, \sigma_1^2) + (1 - \alpha) \cdot \mathcal{N}(\mu_2, \sigma_2^2)$$
pub fn sample_gaussian_mixture<R: RngSource + ?Sized>(
    rng: &mut R,
    alpha: f64,
    mu1: f64,
    sigma1: f64,
    mu2: f64,
    sigma2: f64,
) -> f64 {
    if rng.next_f64() < alpha {
        mu1 + sigma1 * rng.rand_normal()
    } else {
        mu2 + sigma2 * rng.rand_normal()
    }
}

/// Samples the Area-to-Mass ratio logarithmic value $\chi = \log_{10}(A/M)$ and $A/M$ in $\text{m}^2/\text{kg}$.
///
/// Implements Johnson et al. (2001), Section 3.2:
/// - Small particles ($L_c < 0.08\text{ m}$): SOCIT distribution (Eq. 7)
/// - Large Rocket Bodies ($L_c \ge 0.11\text{ m}$): Eq. (5)
/// - Large Spacecraft ($L_c \ge 0.11\text{ m}$): Eq. (6)
/// - Transition region ($0.08\text{ m} \le L_c < 0.11\text{ m}$): Linear interpolation
// Note: 0.318 in Eq. (6) is an empirical regression coefficient from Johnson et al. (2001), not 1/pi.
#[allow(clippy::approx_constant)]
pub fn sample_am_ratio<R: RngSource + ?Sized>(
    rng: &mut R,
    lc: f64,
    obj_type: ObjectType,
) -> (f64, f64) {
    let lambda_c = lc.log10();
    let chi = if lc < 0.08 {
        // Eq. (7): Small fragments (Lc < 8 cm) from SOCIT hypervelocity impact tests
        let mu_soc = if lambda_c <= -1.75 {
            -0.30
        } else if lambda_c < -1.25 {
            -0.30 - 1.4 * (lambda_c + 1.75)
        } else {
            -1.00
        };
        let sigma_soc = if lambda_c <= -3.5 {
            0.20
        } else {
            0.20 + 0.1333 * (lambda_c + 3.5)
        };
        mu_soc + sigma_soc * rng.rand_normal()
    } else if lc >= 0.11 {
        // Large fragments (Lc >= 11 cm)
        match obj_type {
            ObjectType::RocketBody => {
                // Eq. (5): Upper stages / Rocket Bodies
                let alpha = if lambda_c <= -1.4 {
                    1.0
                } else if lambda_c < 0.0 {
                    1.0 - 0.3571 * (lambda_c + 1.4)
                } else {
                    0.5
                };
                let mu1 = if lambda_c <= -0.5 {
                    -0.45
                } else if lambda_c < 0.0 {
                    -0.45 - 0.9 * (lambda_c + 0.5)
                } else {
                    -0.90
                };
                let sigma1 = 0.55;
                let mu2 = -0.90;
                let sigma2 = if lambda_c <= -1.0 {
                    0.28
                } else if lambda_c < 0.1 {
                    0.28 - 0.1636 * (lambda_c + 1.0)
                } else {
                    0.10
                };
                sample_gaussian_mixture(rng, alpha, mu1, sigma1, mu2, sigma2)
            }
            ObjectType::Spacecraft => {
                // Eq. (6): Spacecraft / Payloads
                let alpha = if lambda_c <= -1.95 {
                    0.0
                } else if lambda_c < 0.55 {
                    0.3 + 0.4 * (lambda_c + 1.2)
                } else {
                    1.0
                };
                let mu1 = if lambda_c <= -1.1 {
                    -0.60
                } else if lambda_c < 0.0 {
                    -0.60 - 0.318 * (lambda_c + 1.1)
                } else {
                    -0.95
                };
                let sigma1 = if lambda_c <= -1.3 {
                    0.10
                } else if lambda_c < -0.3 {
                    0.10 + 0.2 * (lambda_c + 1.3)
                } else {
                    0.30
                };
                let mu2 = if lambda_c <= -0.7 {
                    -1.20
                } else if lambda_c < -0.1 {
                    -1.20 - 1.333 * (lambda_c + 0.7)
                } else {
                    -2.00
                };
                let sigma2 = if lambda_c <= -0.5 {
                    0.50
                } else if lambda_c < -0.3 {
                    0.50 - (lambda_c + 0.5)
                } else {
                    0.30
                };
                sample_gaussian_mixture(rng, alpha, mu1, sigma1, mu2, sigma2)
            }
        }
    } else {
        // Bridging region: 8 cm <= Lc < 11 cm (linear interpolation)
        let t = (lc - 0.08) / (0.11 - 0.08);
        let (chi_soc, _) = sample_am_ratio(rng, 0.0799, obj_type);
        let (chi_large, _) = sample_am_ratio(rng, 0.1101, obj_type);
        (1.0 - t) * chi_soc + t * chi_large
    };

    (chi, 10.0f64.powf(chi))
}

/// Samples the ejection velocity magnitude $\Delta v$ (in m/s) conditioned on $\chi = \log_{10}(A/M)$.
///
/// Implements Johnson et al. (2001), Section 3.4:
/// - Collisions (Eq. 12): $\mu = 0.90 \cdot \chi + 2.90, \; \sigma = 0.40$
/// - Explosions (Eq. 11): $\mu = 0.20 \cdot \chi + 1.85, \; \sigma = 0.40$
pub fn sample_delta_v<R: RngSource + ?Sized>(
    rng: &mut R,
    chi: f64,
    breakup_type: BreakupType,
) -> f64 {
    let (mu, sigma) = match breakup_type {
        BreakupType::Collision => (0.90 * chi + 2.90, 0.40),
        BreakupType::Explosion => (0.20 * chi + 1.85, 0.40),
    };
    let log_dv = mu + sigma * rng.rand_normal();
    10.0f64.powf(log_dv)
}

/// Samples an isotropic unit direction vector on the unit sphere $S^2$.
pub fn sample_direction_isotropic<R: RngSource + ?Sized>(rng: &mut R) -> [f64; 3] {
    let phi = rng.next_f64() * 2.0 * PI;
    let cos_t = rng.next_f64() * 2.0 - 1.0;
    let sin_t = (1.0 - cos_t * cos_t).max(0.0).sqrt();
    [sin_t * phi.cos(), sin_t * phi.sin(), cos_t]
}

/// Samples a direction vector concentrated along `target_dir` using von Mises-Fisher perturbation.
pub fn sample_direction_cone<R: RngSource + ?Sized>(
    rng: &mut R,
    target_dir: [f64; 3],
    kappa: f64,
) -> [f64; 3] {
    let inv_k = if kappa > 0.0 { 1.0 / kappa } else { 0.0 };
    let rx = rng.rand_normal();
    let ry = rng.rand_normal();
    let rz = rng.rand_normal();

    let vx = target_dir[0] + inv_k * rx;
    let vy = target_dir[1] + inv_k * ry;
    let vz = target_dir[2] + inv_k * rz;
    let norm = (vx * vx + vy * vy + vz * vz).sqrt();
    if norm > 1e-12 {
        [vx / norm, vy / norm, vz / norm]
    } else {
        target_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SimpleRng;

    #[test]
    fn test_sample_am_ratio_regimes() {
        let mut rng = SimpleRng::new(12345);

        // SOCIT regime
        let (chi_soc, am_soc) = sample_am_ratio(&mut rng, 0.02, ObjectType::Spacecraft);
        assert!(chi_soc > -4.0 && chi_soc < 2.0);
        assert!((am_soc - 10.0f64.powf(chi_soc)).abs() < 1e-9);

        // Rocket Body regime
        let (chi_rb, am_rb) = sample_am_ratio(&mut rng, 0.50, ObjectType::RocketBody);
        assert!(chi_rb > -4.0 && chi_rb < 2.0);
        assert!((am_rb - 10.0f64.powf(chi_rb)).abs() < 1e-9);

        // Spacecraft regime
        let (chi_sc, am_sc) = sample_am_ratio(&mut rng, 0.50, ObjectType::Spacecraft);
        assert!(chi_sc > -4.0 && chi_sc < 2.0);
        assert!((am_sc - 10.0f64.powf(chi_sc)).abs() < 1e-9);

        // Bridging regime
        let (chi_bridge, am_bridge) = sample_am_ratio(&mut rng, 0.095, ObjectType::Spacecraft);
        assert!(chi_bridge > -4.0 && chi_bridge < 2.0);
        assert!((am_bridge - 10.0f64.powf(chi_bridge)).abs() < 1e-9);
    }

    #[test]
    fn test_sample_delta_v_moments() {
        let mut rng = SimpleRng::new(54321);

        // Collision: chi = -1.0 -> mu = 2.0 -> 10^2 = 100 m/s
        let mut coll_speeds = Vec::with_capacity(1000);
        for _ in 0..1000 {
            coll_speeds.push(sample_delta_v(&mut rng, -1.0, BreakupType::Collision));
        }
        let mean_log_coll = coll_speeds.iter().map(|s| s.log10()).sum::<f64>() / 1000.0;
        assert!((mean_log_coll - 2.0).abs() < 0.10);

        // Explosion: chi = -1.0 -> mu = 1.65 -> 10^1.65 = 44.7 m/s
        let mut exp_speeds = Vec::with_capacity(1000);
        for _ in 0..1000 {
            exp_speeds.push(sample_delta_v(&mut rng, -1.0, BreakupType::Explosion));
        }
        let mean_log_exp = exp_speeds.iter().map(|s| s.log10()).sum::<f64>() / 1000.0;
        assert!((mean_log_exp - 1.65).abs() < 0.10);
    }

    #[test]
    fn test_isotropic_direction_normalization() {
        let mut rng = SimpleRng::new(999);
        for _ in 0..100 {
            let [x, y, z] = sample_direction_isotropic(&mut rng);
            let norm = (x * x + y * y + z * z).sqrt();
            assert!((norm - 1.0).abs() < 1e-12);
        }
    }
}
