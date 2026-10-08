//! PPO rollout buffer, Generalized Advantage Estimation (GAE), and training loop.
//!
//! Grounded in Section IV-B1 and Equations (6)–(8) of Luna et al. (2026):
//! - **Equation (6)**: Clipped surrogate policy objective $L^{\text{CLIP}}(\theta)$
//! - **Equation (7)**: Combined policy, value, and entropy loss $L^{\text{PPO}}(\theta)$
//! - **Equation (8)**: Generalized Advantage Estimation (GAE) with $\gamma = 0.995, \lambda = 0.95$

use crate::curriculum::CurriculumScheduler;
use crate::env::SatelliteAvoidanceEnv;
use crate::nn::ActorCritic;
use crate::types::{Action3D, ACTION_DIM, OBSERVATION_DIM};

/// Storage buffer for experience trajectories collected during rollout.
#[derive(Debug, Clone, Default)]
pub struct RolloutBuffer {
    pub observations: Vec<[f32; OBSERVATION_DIM]>,
    pub actions: Vec<[f32; ACTION_DIM]>,
    pub rewards: Vec<f32>,
    pub values: Vec<f32>,
    pub log_probs: Vec<f32>,
    pub dones: Vec<bool>,
    pub advantages: Vec<f32>,
    pub returns: Vec<f32>,
}

impl RolloutBuffer {
    /// Clears all trajectory buffers.
    pub fn clear(&mut self) {
        self.observations.clear();
        self.actions.clear();
        self.rewards.clear();
        self.values.clear();
        self.log_probs.clear();
        self.dones.clear();
        self.advantages.clear();
        self.returns.clear();
    }

    /// Appends a single transition into the buffer.
    pub fn push(
        &mut self,
        obs: [f32; OBSERVATION_DIM],
        action: [f32; ACTION_DIM],
        reward: f32,
        value: f32,
        log_prob: f32,
        done: bool,
    ) {
        self.observations.push(obs);
        self.actions.push(action);
        self.rewards.push(reward);
        self.values.push(value);
        self.log_probs.push(log_prob);
        self.dones.push(done);
    }

    /// Computes Generalized Advantage Estimation (GAE) and returns backwards across the rollout.
    ///
    /// Implements Equation (8):
    /// $$\delta_t = r_t + \gamma V(s_{t+1})(1 - d_t) - V(s_t)$$
    /// $$\hat{A}_t = \delta_t + (\gamma \lambda)(1 - d_t) \hat{A}_{t+1}$$
    pub fn compute_gae(&mut self, last_val: f32, gamma: f32, gae_lambda: f32) {
        let n = self.rewards.len();
        self.advantages = vec![0.0f32; n];
        self.returns = vec![0.0f32; n];

        let mut gae = 0.0f32;
        for t in (0..n).rev() {
            let next_value = if t + 1 < n {
                self.values[t + 1]
            } else {
                last_val
            };
            let non_terminal = if self.dones[t] { 0.0f32 } else { 1.0f32 };

            let delta = self.rewards[t] + gamma * next_value * non_terminal - self.values[t];
            gae = delta + gamma * gae_lambda * non_terminal * gae;

            self.advantages[t] = gae;
            self.returns[t] = gae + self.values[t];
        }

        // Advantage normalization across batch: (A - mean) / (std + 1e-8)
        let mean = self.advantages.iter().sum::<f32>() / n.max(1) as f32;
        let var = self.advantages.iter().map(|&a| (a - mean) * (a - mean)).sum::<f32>() / n.max(1) as f32;
        let std = var.sqrt().max(1e-8);

        for a in &mut self.advantages {
            *a = (*a - mean) / std;
        }
    }
}

/// Hyperparameters for PPO training matching Table 10 of Luna et al. (2026).
#[derive(Debug, Clone)]
pub struct PpoHyperparameters {
    /// Rollout length per update ($n_{\text{steps}} = 2048$).
    pub n_steps: usize,
    /// Batch size for Adam updates ($256$).
    pub batch_size: usize,
    /// Number of optimization epochs per rollout buffer ($10$).
    pub n_epochs: usize,
    /// Learning rate for Adam optimizer ($3 \times 10^{-4}$).
    pub learning_rate: f32,
    /// Discount factor ($\gamma = 0.995$).
    pub gamma: f32,
    /// GAE lambda parameter ($\lambda = 0.95$).
    pub gae_lambda: f32,
    /// PPO clipping parameter ($\epsilon = 0.2$).
    pub clip_range: f32,
    /// Value function loss coefficient ($c_1 = 0.5$).
    pub vf_coef: f32,
    /// Entropy regularization coefficient ($c_2 = 0.01$).
    pub ent_coef: f32,
    /// Maximum global gradient norm for clipping ($0.5$).
    pub max_grad_norm: f32,
}

impl Default for PpoHyperparameters {
    fn default() -> Self {
        Self {
            n_steps: 2048,
            batch_size: 256,
            n_epochs: 10,
            learning_rate: 0.0003,
            gamma: 0.995,
            gae_lambda: 0.95,
            clip_range: 0.2,
            vf_coef: 0.5,
            ent_coef: 0.01,
            max_grad_norm: 0.5,
        }
    }
}

/// Complete PPO training engine managing rollouts, curriculum scheduling, and gradient steps.
#[derive(Debug, Clone)]
pub struct PpoTrainer {
    pub model: ActorCritic,
    pub curriculum: CurriculumScheduler,
    pub buffer: RolloutBuffer,
    pub params: PpoHyperparameters,
    pub global_step: usize,
    pub update_count: usize,
}

impl PpoTrainer {
    /// Constructs a new PPO trainer.
    pub fn new(seed: u64, params: PpoHyperparameters) -> Self {
        Self {
            model: ActorCritic::new(seed),
            curriculum: CurriculumScheduler::default(),
            buffer: RolloutBuffer::default(),
            params,
            global_step: 0,
            update_count: 0,
        }
    }

    /// Collects a rollout buffer and runs a full PPO update (10 epochs across minibatches).
    pub fn train_iteration(&mut self, env: &mut SatelliteAvoidanceEnv, rng: &mut impl rand::Rng) -> f32 {
        self.buffer.clear();

        // 1. Synchronize environment with active curriculum stage
        let stage = self.curriculum.get_stage(self.global_step);
        env.config.collision_course_probability = stage.collision_probability;
        env.config.debris_radius_m = stage.debris_radius_m;

        let mut obs = env.reset(rng.gen::<u64>());
        let mut episode_reward_sum = 0.0f32;
        let mut completed_episodes = 0;

        // 2. Collect trajectory rollout of length n_steps
        for _ in 0..self.params.n_steps {
            let (action, log_prob, value) = self.model.sample_action_and_value(&obs, rng);

            let act_3d = Action3D::new(action[0] as f64, action[1] as f64, action[2] as f64);
            let (next_obs, reward, done, _info) = env.step(act_3d);

            self.buffer.push(obs, action, reward as f32, value, log_prob, done);
            self.global_step += 1;
            episode_reward_sum += reward as f32;

            if done {
                completed_episodes += 1;
                obs = env.reset(rng.gen::<u64>());
            } else {
                obs = next_obs;
            }
        }

        // 3. Compute GAE advantages
        let last_val = self.model.forward_critic(&obs);
        self.buffer.compute_gae(last_val, self.params.gamma, self.params.gae_lambda);

        // 4. Optimization epochs
        let n_samples = self.buffer.rewards.len();
        let mut indices: Vec<usize> = (0..n_samples).collect();

        for _epoch in 0..self.params.n_epochs {
            // Shuffle sample indices
            for i in (1..n_samples).rev() {
                let j = rng.gen_range(0..=i);
                indices.swap(i, j);
            }

            // Minibatch updates
            for chunk in indices.chunks(self.params.batch_size) {
                self.model.zero_grad();

                for &idx in chunk {
                    let s = &self.buffer.observations[idx];
                    let a = &self.buffer.actions[idx];
                    let old_logp = self.buffer.log_probs[idx];
                    let adv = self.buffer.advantages[idx];
                    let ret = self.buffer.returns[idx];

                    let (curr_logp, _entropy, _val) = self.model.evaluate_action(s, a);
                    let ratio = (curr_logp - old_logp).exp();

                    let step_cfg = crate::nn::PpoSampleGradientConfig {
                        advantage: adv,
                        return_target: ret,
                        ratio,
                        clip_range: self.params.clip_range,
                        vf_coef: self.params.vf_coef,
                        ent_coef: self.params.ent_coef,
                    };
                    self.model.backward_step(s, a, &step_cfg);
                }

                self.update_count += 1;
                self.model.step_adam(
                    self.params.learning_rate,
                    self.params.max_grad_norm,
                    self.update_count,
                );
            }
        }

        if completed_episodes > 0 {
            episode_reward_sum / completed_episodes as f32
        } else {
            0.0
        }
    }
}
