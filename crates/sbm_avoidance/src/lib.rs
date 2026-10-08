//! # Autonomous Satellite Trajectory Optimization & Space Debris Avoidance via PPO
//!
//! `sbm_avoidance` is a pure-Rust, zero-external-ML-dependency framework for training, evaluating,
//! and benchmarking autonomous reinforcement learning agents for satellite collision avoidance.
//!
//! ## Mathematical Grounding
//! Based on the peer-reviewed specification:
//! > **Luna, L., Couder, J. O., & Vargas-Acosta, R. A. (2026).**  
//! > *Satellite Trajectory Optimization via Proximal Policy Optimization for Space Debris Avoidance.*  
//! > IEEE Access, vol. 14, pp. 1–18.  
//! > DOI: [10.1109/ACCESS.2026.3655237](https://doi.org/10.1109/ACCESS.2026.3655237)
//!
//! ## Key Capabilities
//! - **High-Fidelity 3-Body Dynamics (Algorithm 1)**: Earth point-mass gravity + Moon & Sun gravitational perturbations.
//! - **Instantaneous Tsiolkovsky Rocket Depletion (Algorithm 2)**: $I_{sp} = 300\text{ s}$, continuous low-thrust propulsion ($T_{\max} = 0.15\text{ m/s}^2$).
//! - **Curriculum Learning (Table 1)**: 3-stage curriculum progressing from basic avoidance to adversarial clutter.
//! - **Algorithm 3 Reward Shaping**: Proximity buffer shaping, closing-speed bonus, context-aware $\Delta v$ penalty, and jitter penalty.
//! - **Pure-Rust PPO Engine**: Autograd MLP (`Linear`, `Tanh`, Gaussian policy, Adam optimizer, GAE $\lambda = 0.95$, clipping $\epsilon = 0.2$).
//! - **Three Analytical Baselines (Section IV-C1)**: No-Action, Risk-Aware Rule-Based Controller (Eq. 9–10), and Impulsive $\Delta v$ Planner (Eq. 11–12).
//! - **Deterministic Benchmark Harness**: 1,000-trial Monte Carlo verification reproducing Tables 2, 3, and 4.
//! - **Zero Warnings Policy**: Built and verified to compile and pass `cargo clippy` with zero warnings.

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub mod baselines;
pub mod conjunction;
pub mod curriculum;
pub mod dynamics;
pub mod env;
pub mod eval;
pub mod nn;
pub mod ppo;
pub mod prelude;
pub mod reward;
pub mod serialize;
pub mod types;

pub use prelude::*;

/// Embedded binary weights from the paper's pre-trained model checkpoint (`satellite_avoidance_model.zip`).
pub const PRETRAINED_POLICY_WEIGHTS_BIN: &[u8] = include_bytes!("../weights/ppo_policy_weights.bin");

/// Instantiates an `ActorCritic` initialized with the paper's pre-trained weights.
pub fn load_default_pretrained_model() -> Result<ActorCritic, String> {
    serialize::load_policy_weights_raw(PRETRAINED_POLICY_WEIGHTS_BIN)
}
