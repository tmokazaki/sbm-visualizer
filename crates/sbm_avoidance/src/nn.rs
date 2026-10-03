//! Pure-Rust autograd neural network primitives and Actor-Critic architecture for PPO.
//!
//! Grounded in Section IV-B1 and Figure 8 of Luna et al. (2026):
//! - **Actor MLP**: `Linear(307, 256) -> Tanh -> Linear(256, 256) -> Tanh -> Linear(256, 128) -> Tanh -> Linear(128, 3)`
//! - **Critic MLP**: `Linear(307, 256) -> Tanh -> Linear(256, 256) -> Tanh -> Linear(256, 128) -> Tanh -> Linear(128, 1)`
//! - **Continuous Gaussian Action Head**: Parameterized by mean vector $\boldsymbol{\mu} \in \mathbb{R}^3$ and $\log \boldsymbol{\sigma} \in \mathbb{R}^3$
//! - **Adam Optimizer**: Momentum buffers and $L_2$ global gradient clipping

#![allow(clippy::needless_range_loop)]

use crate::types::{ACTION_DIM, OBSERVATION_DIM};

/// Configuration and advantage inputs for a single-sample PPO backward step.
#[derive(Debug, Clone, Copy)]
pub struct PpoSampleGradientConfig {
    pub advantage: f32,
    pub return_target: f32,
    pub ratio: f32,
    pub clip_range: f32,
    pub vf_coef: f32,
    pub ent_coef: f32,
}

/// Single fully-connected linear layer with autograd support and Adam momentum tracking.
#[derive(Debug, Clone)]
pub struct LinearLayer {
    pub in_features: usize,
    pub out_features: usize,
    /// Weights stored in row-major order: `weights[out_idx * in_features + in_idx]`.
    pub weights: Vec<f32>,
    /// Biases: `bias[out_idx]`.
    pub bias: Vec<f32>,
    /// Accumulated weight gradients.
    pub grad_weights: Vec<f32>,
    /// Accumulated bias gradients.
    pub grad_bias: Vec<f32>,
    // Adam optimizer first moment buffers
    m_w: Vec<f32>,
    m_b: Vec<f32>,
    // Adam optimizer second moment buffers
    v_w: Vec<f32>,
    v_b: Vec<f32>,
}

impl LinearLayer {
    /// Creates a new linear layer initialized using Xavier/Glorot uniform initialization.
    pub fn new(in_features: usize, out_features: usize, seed: u64) -> Self {
        let weight_count = out_features * in_features;
        let mut weights = Vec::with_capacity(weight_count);
        let bias = vec![0.0f32; out_features];

        // Xavier uniform limit: sqrt(6 / (fan_in + fan_out))
        let limit = (6.0f32 / (in_features + out_features) as f32).sqrt();
        let mut state = if seed == 0 { 0x517cc1b727220a95 } else { seed };

        for _ in 0..weight_count {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let unit_float = (state >> 32) as f32 / (u32::MAX as f32);
            weights.push(-limit + 2.0 * limit * unit_float);
        }

        Self {
            in_features,
            out_features,
            weights,
            bias,
            grad_weights: vec![0.0f32; weight_count],
            grad_bias: vec![0.0f32; out_features],
            m_w: vec![0.0f32; weight_count],
            m_b: vec![0.0f32; out_features],
            v_w: vec![0.0f32; weight_count],
            v_b: vec![0.0f32; out_features],
        }
    }

    /// Forward pass: computes $\mathbf{y} = W \mathbf{x} + \mathbf{b}$.
    #[inline]
    pub fn forward(&self, input: &[f32], output: &mut [f32]) {
        for i in 0..self.out_features {
            let row_offset = i * self.in_features;
            let mut sum = self.bias[i];
            for j in 0..self.in_features {
                sum += self.weights[row_offset + j] * input[j];
            }
            output[i] = sum;
        }
    }

    /// Backward pass: accumulates parameter gradients and propagates gradient to input:
    /// $$\frac{\partial L}{\partial W_{ij}} = \delta_i x_j, \quad \frac{\partial L}{\partial b_i} = \delta_i, \quad \delta_{\text{in}, j} = \sum_i W_{ij} \delta_i$$
    #[inline]
    pub fn backward(&mut self, input: &[f32], grad_output: &[f32], grad_input: &mut [f32]) {
        for i in 0..self.out_features {
            let delta = grad_output[i];
            let row_offset = i * self.in_features;
            self.grad_bias[i] += delta;

            for j in 0..self.in_features {
                self.grad_weights[row_offset + j] += delta * input[j];
                grad_input[j] += self.weights[row_offset + j] * delta;
            }
        }
    }

    /// Resets accumulated gradients to zero.
    pub fn zero_grad(&mut self) {
        self.grad_weights.fill(0.0);
        self.grad_bias.fill(0.0);
    }

    /// Returns the sum of squared gradients for $L_2$ global gradient clipping.
    pub fn grad_norm_squared(&self) -> f32 {
        let mut sum = 0.0f32;
        for &g in &self.grad_weights {
            sum += g * g;
        }
        for &g in &self.grad_bias {
            sum += g * g;
        }
        sum
    }

    /// Scales all gradients by a factor (used during global gradient norm clipping).
    pub fn scale_grads(&mut self, factor: f32) {
        for g in &mut self.grad_weights {
            *g *= factor;
        }
        for g in &mut self.grad_bias {
            *g *= factor;
        }
    }

    /// Steps the Adam optimizer for weights and biases.
    pub fn adam_step(&mut self, lr: f32, beta1: f32, beta2: f32, eps: f32, step: usize) {
        let correction1 = 1.0 - beta1.powi(step as i32);
        let correction2 = 1.0 - beta2.powi(step as i32);

        // Update weights
        for idx in 0..self.weights.len() {
            let g = self.grad_weights[idx];
            self.m_w[idx] = beta1 * self.m_w[idx] + (1.0 - beta1) * g;
            self.v_w[idx] = beta2 * self.v_w[idx] + (1.0 - beta2) * (g * g);

            let m_hat = self.m_w[idx] / correction1;
            let v_hat = self.v_w[idx] / correction2;

            self.weights[idx] -= lr * m_hat / (v_hat.sqrt() + eps);
        }

        // Update biases
        for idx in 0..self.bias.len() {
            let g = self.grad_bias[idx];
            self.m_b[idx] = beta1 * self.m_b[idx] + (1.0 - beta1) * g;
            self.v_b[idx] = beta2 * self.v_b[idx] + (1.0 - beta2) * (g * g);

            let m_hat = self.m_b[idx] / correction1;
            let v_hat = self.v_b[idx] / correction2;

            self.bias[idx] -= lr * m_hat / (v_hat.sqrt() + eps);
        }
    }
}

/// Actor-Critic Neural Network implementing the 3-layer MLP architecture of Luna et al. (2026).
#[derive(Debug, Clone)]
pub struct ActorCritic {
    // Policy Network (Actor)
    pub policy_fc1: LinearLayer,
    pub policy_fc2: LinearLayer,
    pub policy_fc3: LinearLayer,
    pub action_head: LinearLayer,
    pub log_std: [f32; ACTION_DIM],
    pub grad_log_std: [f32; ACTION_DIM],
    m_log_std: [f32; ACTION_DIM],
    v_log_std: [f32; ACTION_DIM],

    // Value Network (Critic)
    pub value_fc1: LinearLayer,
    pub value_fc2: LinearLayer,
    pub value_fc3: LinearLayer,
    pub value_head: LinearLayer,
}

impl ActorCritic {
    /// Creates a newly initialized Actor-Critic network with default Xavier weights.
    pub fn new(seed: u64) -> Self {
        Self {
            // Actor: 307 -> 256 -> 256 -> 128 -> 3
            policy_fc1: LinearLayer::new(OBSERVATION_DIM, 256, seed.wrapping_add(1)),
            policy_fc2: LinearLayer::new(256, 256, seed.wrapping_add(2)),
            policy_fc3: LinearLayer::new(256, 128, seed.wrapping_add(3)),
            action_head: LinearLayer::new(128, ACTION_DIM, seed.wrapping_add(4)),
            log_std: [0.0f32; ACTION_DIM],
            grad_log_std: [0.0f32; ACTION_DIM],
            m_log_std: [0.0f32; ACTION_DIM],
            v_log_std: [0.0f32; ACTION_DIM],

            // Critic: 307 -> 256 -> 256 -> 128 -> 1
            value_fc1: LinearLayer::new(OBSERVATION_DIM, 256, seed.wrapping_add(5)),
            value_fc2: LinearLayer::new(256, 256, seed.wrapping_add(6)),
            value_fc3: LinearLayer::new(256, 128, seed.wrapping_add(7)),
            value_head: LinearLayer::new(128, 1, seed.wrapping_add(8)),
        }
    }

    /// Evaluates deterministic mean action $\boldsymbol{\mu}(s)$ without exploration noise.
    pub fn forward_actor_deterministic(&self, obs: &[f32; OBSERVATION_DIM]) -> [f32; ACTION_DIM] {
        let mut h1 = [0.0f32; 256];
        self.policy_fc1.forward(obs, &mut h1);
        for x in &mut h1 { *x = x.tanh(); }

        let mut h2 = [0.0f32; 256];
        self.policy_fc2.forward(&h1, &mut h2);
        for x in &mut h2 { *x = x.tanh(); }

        let mut h3 = [0.0f32; 128];
        self.policy_fc3.forward(&h2, &mut h3);
        for x in &mut h3 { *x = x.tanh(); }

        let mut act = [0.0f32; ACTION_DIM];
        self.action_head.forward(&h3, &mut act);
        act
    }

    /// Evaluates scalar baseline value $V(s)$.
    pub fn forward_critic(&self, obs: &[f32; OBSERVATION_DIM]) -> f32 {
        let mut v1 = [0.0f32; 256];
        self.value_fc1.forward(obs, &mut v1);
        for x in &mut v1 { *x = x.tanh(); }

        let mut v2 = [0.0f32; 256];
        self.value_fc2.forward(&v1, &mut v2);
        for x in &mut v2 { *x = x.tanh(); }

        let mut v3 = [0.0f32; 128];
        self.value_fc3.forward(&v2, &mut v3);
        for x in &mut v3 { *x = x.tanh(); }

        let mut val = [0.0f32; 1];
        self.value_head.forward(&v3, &mut val);
        val[0]
    }

    /// Samples an action with Gaussian exploration noise during training rollout.
    /// Returns `(sampled_action, log_prob, value)`.
    pub fn sample_action_and_value(
        &self,
        obs: &[f32; OBSERVATION_DIM],
        rng: &mut impl rand::Rng,
    ) -> ([f32; ACTION_DIM], f32, f32) {
        let mean = self.forward_actor_deterministic(obs);
        let value = self.forward_critic(obs);

        let mut action = [0.0f32; ACTION_DIM];
        let mut log_prob = 0.0f32;
        const LN_2PI: f32 = 1.837877; // ln(2 * pi)

        for i in 0..ACTION_DIM {
            let std = self.log_std[i].exp();
            // Box-Muller standard normal sample
            let u1: f32 = rng.gen::<f32>().max(1e-7);
            let u2: f32 = rng.gen::<f32>();
            let eps = (-2.0 * u1.ln()).sqrt() * (2.0 * core::f32::consts::PI * u2).cos();

            let a = mean[i] + std * eps;
            action[i] = a;

            let diff = (a - mean[i]) / std;
            log_prob -= 0.5 * (diff * diff + 2.0 * self.log_std[i] + LN_2PI);
        }

        (action, log_prob, value)
    }

    /// Evaluates log probability, entropy, and baseline value for a state-action pair during PPO update.
    pub fn evaluate_action(
        &self,
        obs: &[f32; OBSERVATION_DIM],
        action: &[f32; ACTION_DIM],
    ) -> (f32, f32, f32) {
        let mean = self.forward_actor_deterministic(obs);
        let value = self.forward_critic(obs);

        let mut log_prob = 0.0f32;
        let mut entropy = 0.0f32;
        const LN_2PI: f32 = 1.837877;

        for i in 0..ACTION_DIM {
            let std = self.log_std[i].exp();
            let diff = (action[i] - mean[i]) / std;
            log_prob -= 0.5 * (diff * diff + 2.0 * self.log_std[i] + LN_2PI);
            entropy += 0.5 * (1.0 + LN_2PI + 2.0 * self.log_std[i]);
        }

        (log_prob, entropy, value)
    }

    /// Full forward and backward pass for PPO actor and critic losses.
    pub fn backward_step(
        &mut self,
        obs: &[f32; OBSERVATION_DIM],
        action: &[f32; ACTION_DIM],
        cfg: &PpoSampleGradientConfig,
    ) {
        let advantage = cfg.advantage;
        let return_target = cfg.return_target;
        let ratio = cfg.ratio;
        let clip_range = cfg.clip_range;
        let vf_coef = cfg.vf_coef;
        let ent_coef = cfg.ent_coef;
        // --- Forward Policy ---
        let mut h1 = [0.0f32; 256];
        self.policy_fc1.forward(obs, &mut h1);
        let h1_act: Vec<f32> = h1.iter().map(|&x| x.tanh()).collect();

        let mut h2 = [0.0f32; 256];
        self.policy_fc2.forward(&h1_act, &mut h2);
        let h2_act: Vec<f32> = h2.iter().map(|&x| x.tanh()).collect();

        let mut h3 = [0.0f32; 128];
        self.policy_fc3.forward(&h2_act, &mut h3);
        let h3_act: Vec<f32> = h3.iter().map(|&x| x.tanh()).collect();

        let mut mean = [0.0f32; ACTION_DIM];
        self.action_head.forward(&h3_act, &mut mean);

        // --- Clipped Policy Gradient ---
        // If clipped and moving in direction of clipping, policy gradient is 0
        let clipped = (ratio > 1.0 + clip_range && advantage > 0.0)
            || (ratio < 1.0 - clip_range && advantage < 0.0);

        let grad_scale_policy = if clipped { 0.0f32 } else { ratio * advantage };

        let mut grad_mean = [0.0f32; ACTION_DIM];
        for i in 0..ACTION_DIM {
            let std = self.log_std[i].exp();
            let diff = action[i] - mean[i];

            // d(log_prob)/d(mean) = (a - mean) / (std^2)
            let d_logp_d_mu = diff / (std * std);
            // We minimize negative surrogate loss: L_clip = - ratio * A
            grad_mean[i] = -grad_scale_policy * d_logp_d_mu;

            // d(log_prob)/d(log_std) = ((a - mean)^2 / std^2) - 1
            let d_logp_d_logstd = (diff * diff) / (std * std) - 1.0;
            // Entropy bonus: - ent_coef * d(entropy)/d(log_std) = - ent_coef * 1.0
            self.grad_log_std[i] += -grad_scale_policy * d_logp_d_logstd - ent_coef * 1.0;
        }

        // --- Backprop Policy Head & Layers ---
        let mut grad_h3_act = vec![0.0f32; 128];
        self.action_head.backward(&h3_act, &grad_mean, &mut grad_h3_act);

        let mut grad_h3_pre = vec![0.0f32; 128];
        for i in 0..128 { grad_h3_pre[i] = grad_h3_act[i] * (1.0 - h3_act[i] * h3_act[i]); }

        let mut grad_h2_act = vec![0.0f32; 256];
        self.policy_fc3.backward(&h2_act, &grad_h3_pre, &mut grad_h2_act);

        let mut grad_h2_pre = vec![0.0f32; 256];
        for i in 0..256 { grad_h2_pre[i] = grad_h2_act[i] * (1.0 - h2_act[i] * h2_act[i]); }

        let mut grad_h1_act = vec![0.0f32; 256];
        self.policy_fc2.backward(&h1_act, &grad_h2_pre, &mut grad_h1_act);

        let mut grad_h1_pre = vec![0.0f32; 256];
        for i in 0..256 { grad_h1_pre[i] = grad_h1_act[i] * (1.0 - h1_act[i] * h1_act[i]); }

        let mut dummy_in = vec![0.0f32; OBSERVATION_DIM];
        self.policy_fc1.backward(obs, &grad_h1_pre, &mut dummy_in);

        // --- Forward & Backward Critic (Value Head) ---
        let mut v1 = [0.0f32; 256];
        self.value_fc1.forward(obs, &mut v1);
        let v1_act: Vec<f32> = v1.iter().map(|&x| x.tanh()).collect();

        let mut v2 = [0.0f32; 256];
        self.value_fc2.forward(&v1_act, &mut v2);
        let v2_act: Vec<f32> = v2.iter().map(|&x| x.tanh()).collect();

        let mut v3 = [0.0f32; 128];
        self.value_fc3.forward(&v2_act, &mut v3);
        let v3_act: Vec<f32> = v3.iter().map(|&x| x.tanh()).collect();

        let mut val = [0.0f32; 1];
        self.value_head.forward(&v3_act, &mut val);

        // d(0.5 * (V - target)^2)/d(V) = (V - target)
        let grad_v = [vf_coef * (val[0] - return_target)];

        let mut grad_v3_act = vec![0.0f32; 128];
        self.value_head.backward(&v3_act, &grad_v, &mut grad_v3_act);

        let mut grad_v3_pre = vec![0.0f32; 128];
        for i in 0..128 { grad_v3_pre[i] = grad_v3_act[i] * (1.0 - v3_act[i] * v3_act[i]); }

        let mut grad_v2_act = vec![0.0f32; 256];
        self.value_fc3.backward(&v2_act, &grad_v3_pre, &mut grad_v2_act);

        let mut grad_v2_pre = vec![0.0f32; 256];
        for i in 0..256 { grad_v2_pre[i] = grad_v2_act[i] * (1.0 - v2_act[i] * v2_act[i]); }

        let mut grad_v1_act = vec![0.0f32; 256];
        self.value_fc2.backward(&v1_act, &grad_v2_pre, &mut grad_v1_act);

        let mut grad_v1_pre = vec![0.0f32; 256];
        for i in 0..256 { grad_v1_pre[i] = grad_v1_act[i] * (1.0 - v1_act[i] * v1_act[i]); }

        let mut dummy_in_v = vec![0.0f32; OBSERVATION_DIM];
        self.value_fc1.backward(obs, &grad_v1_pre, &mut dummy_in_v);
    }

    /// Resets all accumulated gradients across all layers.
    pub fn zero_grad(&mut self) {
        self.policy_fc1.zero_grad();
        self.policy_fc2.zero_grad();
        self.policy_fc3.zero_grad();
        self.action_head.zero_grad();
        self.grad_log_std.fill(0.0);

        self.value_fc1.zero_grad();
        self.value_fc2.zero_grad();
        self.value_fc3.zero_grad();
        self.value_head.zero_grad();
    }

    /// Performs one Adam optimization step across all parameters with $L_2$ global gradient norm clipping.
    pub fn step_adam(&mut self, lr: f32, max_grad_norm: f32, step: usize) {
        // 1. Calculate global L2 gradient norm
        let mut total_sq = 0.0f32;
        total_sq += self.policy_fc1.grad_norm_squared();
        total_sq += self.policy_fc2.grad_norm_squared();
        total_sq += self.policy_fc3.grad_norm_squared();
        total_sq += self.action_head.grad_norm_squared();
        for &g in &self.grad_log_std { total_sq += g * g; }

        total_sq += self.value_fc1.grad_norm_squared();
        total_sq += self.value_fc2.grad_norm_squared();
        total_sq += self.value_fc3.grad_norm_squared();
        total_sq += self.value_head.grad_norm_squared();

        let global_norm = total_sq.sqrt();
        let scale = if global_norm > max_grad_norm && global_norm > 1e-6 {
            max_grad_norm / global_norm
        } else {
            1.0
        };

        if scale < 1.0 {
            self.policy_fc1.scale_grads(scale);
            self.policy_fc2.scale_grads(scale);
            self.policy_fc3.scale_grads(scale);
            self.action_head.scale_grads(scale);
            for g in &mut self.grad_log_std { *g *= scale; }

            self.value_fc1.scale_grads(scale);
            self.value_fc2.scale_grads(scale);
            self.value_fc3.scale_grads(scale);
            self.value_head.scale_grads(scale);
        }

        // 2. Step Adam optimizer
        const BETA1: f32 = 0.9;
        const BETA2: f32 = 0.999;
        const EPS: f32 = 1e-8;

        self.policy_fc1.adam_step(lr, BETA1, BETA2, EPS, step);
        self.policy_fc2.adam_step(lr, BETA1, BETA2, EPS, step);
        self.policy_fc3.adam_step(lr, BETA1, BETA2, EPS, step);
        self.action_head.adam_step(lr, BETA1, BETA2, EPS, step);

        let correction1 = 1.0 - BETA1.powi(step as i32);
        let correction2 = 1.0 - BETA2.powi(step as i32);
        for i in 0..ACTION_DIM {
            let g = self.grad_log_std[i];
            self.m_log_std[i] = BETA1 * self.m_log_std[i] + (1.0 - BETA1) * g;
            self.v_log_std[i] = BETA2 * self.v_log_std[i] + (1.0 - BETA2) * (g * g);

            let m_hat = self.m_log_std[i] / correction1;
            let v_hat = self.v_log_std[i] / correction2;
            self.log_std[i] -= lr * m_hat / (v_hat.sqrt() + EPS);
        }

        self.value_fc1.adam_step(lr, BETA1, BETA2, EPS, step);
        self.value_fc2.adam_step(lr, BETA1, BETA2, EPS, step);
        self.value_fc3.adam_step(lr, BETA1, BETA2, EPS, step);
        self.value_head.adam_step(lr, BETA1, BETA2, EPS, step);
    }
}
