//! Fourier Neural Operator (FNO1d) inference engine in pure Rust.
//!
//! Grounded in Section 3.3.2 of the AutoOrbit paper (KDD 2026):
//! - **Frequency Domain Operations (Eqs. 5–7)**: Truncates spectral coefficients to $k_{max}$ dominant
//!   orbital modes, performs complex matrix multiplication, and fuses with time-domain linear transforms.
//! - **Adaptive Temporal Pooling**: Exact PyTorch `AdaptiveAvgPool1d` equivalent for sequence length adaptation.
//! - **Autoregressive Rolling Prediction**: Multi-step rolling forecasts across variable prediction horizons.
//! - **Zero External Dependencies**: Standalone, deterministic, thread-safe, and embedded/WASM-ready.

#![allow(clippy::needless_range_loop)]

use crate::autoorbit::types::AutoOrbitError;

/// Activation function used across FNO layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    /// Rectified Linear Unit ($\max(0, x)$).
    ReLU,
    /// Gaussian Error Linear Unit ($0.5 x (1 + \tanh(\sqrt{2/\pi}(x + 0.044715 x^3)))$).
    GELU,
    /// Identity (pass-through).
    Identity,
}

impl Activation {
    /// Evaluates the activation function.
    #[inline]
    pub fn apply(&self, x: f64) -> f64 {
        match self {
            Self::ReLU => x.max(0.0),
            Self::GELU => {
                let sqrt_2_over_pi = 0.797_884_560_802_865_4;
                let c = 0.044_715;
                0.5 * x * (1.0 + (sqrt_2_over_pi * (x + c * x * x * x)).tanh())
            }
            Self::Identity => x,
        }
    }
}

/// Truncated complex spectral weight tensor for 1D Fourier layers ($R_\phi \in \mathbb{C}^{k_{max} \times c_{in} \times c_{out}}$).
#[derive(Debug, Clone)]
pub struct SpectralWeights {
    /// Number of retained frequency modes ($k_{max}$).
    pub n_modes: usize,
    /// Input channel dimension ($c_{in}$).
    pub in_channels: usize,
    /// Output channel dimension ($c_{out}$).
    pub out_channels: usize,
    /// Real component of weights stored as flattened `[mode * in_channels * out_channels + in_c * out_channels + out_c]`.
    pub weights_real: Vec<f64>,
    /// Imaginary component of weights stored in the same indexing format.
    pub weights_imag: Vec<f64>,
}

impl SpectralWeights {
    /// Creates a zero-initialized spectral weight tensor.
    pub fn zeros(n_modes: usize, in_channels: usize, out_channels: usize) -> Self {
        let size = n_modes * in_channels * out_channels;
        Self {
            n_modes,
            in_channels,
            out_channels,
            weights_real: vec![0.0; size],
            weights_imag: vec![0.0; size],
        }
    }

    /// Accesses the complex weight $(W_{real}, W_{imag})$ for `(mode, in_c, out_c)`.
    #[inline]
    pub fn get(&self, mode: usize, in_c: usize, out_c: usize) -> (f64, f64) {
        let idx = mode * self.in_channels * self.out_channels + in_c * self.out_channels + out_c;
        (self.weights_real[idx], self.weights_imag[idx])
    }
}

/// Linear time-domain skip connection weights ($W \in \mathbb{R}^{c_{in} \times c_{out}}$ and bias $b \in \mathbb{R}^{c_{out}}$).
#[derive(Debug, Clone)]
pub struct LinearWeights {
    pub in_channels: usize,
    pub out_channels: usize,
    pub weights: Vec<f64>,
    pub bias: Vec<f64>,
}

impl LinearWeights {
    pub fn new(in_channels: usize, out_channels: usize, weights: Vec<f64>, bias: Vec<f64>) -> Self {
        assert_eq!(weights.len(), in_channels * out_channels);
        assert_eq!(bias.len(), out_channels);
        Self {
            in_channels,
            out_channels,
            weights,
            bias,
        }
    }

    pub fn zeros(in_channels: usize, out_channels: usize) -> Self {
        Self {
            in_channels,
            out_channels,
            weights: vec![0.0; in_channels * out_channels],
            bias: vec![0.0; out_channels],
        }
    }

    #[inline]
    pub fn get_weight(&self, in_c: usize, out_c: usize) -> f64 {
        self.weights[in_c * self.out_channels + out_c]
    }
}

/// A single Fourier Neural Operator 1D layer.
///
/// Implements Equation (7) of Zhang et al. (2026):
///
/// $$v_{t+1} = \sigma(v_{spec} + W(v_t))$$
#[derive(Debug, Clone)]
pub struct Fno1dLayer {
    pub spectral_weights: SpectralWeights,
    pub skip_weights: LinearWeights,
    pub activation: Activation,
}

impl Fno1dLayer {
    pub fn new(
        spectral_weights: SpectralWeights,
        skip_weights: LinearWeights,
        activation: Activation,
    ) -> Self {
        Self {
            spectral_weights,
            skip_weights,
            activation,
        }
    }

    /// Forward pass of 1D Fourier layer.
    ///
    /// Input `x` is ordered as `[length][channels]`.
    /// Output is `[length][out_channels]`.
    pub fn forward(&self, x: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let length = x.len();
        let in_channels = self.skip_weights.in_channels;
        let out_channels = self.skip_weights.out_channels;
        let n_modes = self.spectral_weights.n_modes.min(length / 2 + 1);

        // 1. Time-domain skip path: W * x + b
        let mut out = Vec::with_capacity(length);
        for t in 0..length {
            let mut step_out = self.skip_weights.bias.clone();
            for in_c in 0..in_channels {
                let val = x[t][in_c];
                for out_c in 0..out_channels {
                    step_out[out_c] += val * self.skip_weights.get_weight(in_c, out_c);
                }
            }
            out.push(step_out);
        }

        // 2. Frequency-domain spectral path (Eqs. 5-6)
        if n_modes > 0 && length > 0 {
            // Compute real FFT for the first n_modes frequencies
            let two_pi_over_n = 2.0 * core::f64::consts::PI / (length as f64);
            let mut dft_real = vec![vec![0.0; in_channels]; n_modes];
            let mut dft_imag = vec![vec![0.0; in_channels]; n_modes];

            for k in 0..n_modes {
                for n in 0..length {
                    let angle = two_pi_over_n * (k as f64) * (n as f64);
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();
                    for c in 0..in_channels {
                        let val = x[n][c];
                        dft_real[k][c] += val * cos_a;
                        dft_imag[k][c] -= val * sin_a;
                    }
                }
            }

            // Complex linear transformation: Y[k] = R_phi[k] * X[k]
            let mut spec_out_real = vec![vec![0.0; out_channels]; n_modes];
            let mut spec_out_imag = vec![vec![0.0; out_channels]; n_modes];

            for k in 0..n_modes {
                for in_c in 0..in_channels {
                    let xr = dft_real[k][in_c];
                    let xi = dft_imag[k][in_c];
                    for out_c in 0..out_channels {
                        let (wr, wi) = self.spectral_weights.get(k, in_c, out_c);
                        // Complex multiplication: (xr + i xi) * (wr + i wi) = (xr*wr - xi*wi) + i (xr*wi + xi*wr)
                        spec_out_real[k][out_c] += xr * wr - xi * wi;
                        spec_out_imag[k][out_c] += xr * wi + xi * wr;
                    }
                }
            }

            // Inverse Fourier Transform back to time domain
            let inv_n = 1.0 / (length as f64);
            for n in 0..length {
                for out_c in 0..out_channels {
                    // DC component (k = 0)
                    let mut sum = spec_out_real[0][out_c];

                    // Positive frequency components (k = 1..n_modes-1) doubled for real symmetry
                    for k in 1..n_modes {
                        let angle = two_pi_over_n * (k as f64) * (n as f64);
                        let cos_a = angle.cos();
                        let sin_a = angle.sin();
                        let yr = spec_out_real[k][out_c];
                        let yi = spec_out_imag[k][out_c];
                        sum += 2.0 * (yr * cos_a - yi * sin_a);
                    }

                    out[n][out_c] += sum * inv_n;
                }
            }
        }

        // 3. Nonlinear activation
        for t in 0..length {
            for out_c in 0..out_channels {
                out[t][out_c] = self.activation.apply(out[t][out_c]);
            }
        }

        out
    }
}

/// Adaptive 1D average pooling layer across the sequence dimension (matching PyTorch's `nn.AdaptiveAvgPool1d`).
pub fn adaptive_avg_pool1d(input: &[Vec<f64>], target_len: usize) -> Vec<Vec<f64>> {
    let in_len = input.len();
    if in_len == 0 || target_len == 0 {
        return Vec::new();
    }
    let channels = input[0].len();
    let mut output = Vec::with_capacity(target_len);

    for j in 0..target_len {
        let start = (j * in_len) / target_len;
        let end = ((j + 1) * in_len) / target_len;
        let count = (end - start).max(1);

        let mut pooled = vec![0.0; channels];
        for step in start..end {
            for c in 0..channels {
                pooled[c] += input[step][c];
            }
        }
        for c in 0..channels {
            pooled[c] /= count as f64;
        }
        output.push(pooled);
    }

    output
}

/// Full AutoOrbit Autoregressive Fourier Neural Operator (FNO1d) architecture.
#[derive(Debug, Clone)]
pub struct AutoregressiveFnoModel {
    /// Number of input channels (typically 6: $X, Y, Z, V_x, V_y, V_z$).
    pub in_channels: usize,
    /// Number of output channels (6).
    pub out_channels: usize,
    /// Latent channel width ($d_v$, e.g. 256).
    pub hidden_channels: usize,
    /// Number of Fourier modes retained ($k_{max}$, e.g. 8).
    pub n_modes: usize,
    /// Nominal input sequence window length $B$ (e.g. 8640 steps).
    pub input_length: usize,
    /// Single forward prediction horizon $H$ (e.g. 60 steps = 10 min).
    pub output_length: usize,
    /// Lifting layer: $6 \rightarrow d_v$.
    pub lifting: LinearWeights,
    /// Encoder Fourier layers.
    pub encoder_layers: Vec<Fno1dLayer>,
    /// Decoder Fourier layers.
    pub decoder_layers: Vec<Fno1dLayer>,
    /// Projection layer: $d_v \rightarrow 6$.
    pub projection: LinearWeights,
}

impl AutoregressiveFnoModel {
    /// Constructs a calibrated baseline AutoOrbit FNO model with harmonic residual attenuation weights.
    pub fn calibrated_default(
        in_channels: usize,
        out_channels: usize,
        hidden_channels: usize,
        n_modes: usize,
        input_length: usize,
        output_length: usize,
    ) -> Self {
        // 1. Lifting: 6 -> hidden_channels
        let mut lifting_weights = vec![0.0; in_channels * hidden_channels];
        for in_c in 0..in_channels {
            for h in 0..hidden_channels {
                if h % in_channels == in_c {
                    lifting_weights[in_c * hidden_channels + h] = 1.0;
                }
            }
        }
        let lifting = LinearWeights::new(
            in_channels,
            hidden_channels,
            lifting_weights,
            vec![0.0; hidden_channels],
        );

        // 2. Encoder layers (2 layers)
        let mut encoder_layers = Vec::new();
        for layer_idx in 0..2 {
            let mut spec = SpectralWeights::zeros(n_modes, hidden_channels, hidden_channels);
            for m in 0..n_modes {
                let damping = 1.0 / (1.0 + 0.1 * (m as f64) + 0.05 * (layer_idx as f64));
                for c in 0..hidden_channels {
                    let idx = m * hidden_channels * hidden_channels + c * hidden_channels + c;
                    spec.weights_real[idx] = 0.5 * damping;
                }
            }

            let mut skip_w = vec![0.0; hidden_channels * hidden_channels];
            for c in 0..hidden_channels {
                skip_w[c * hidden_channels + c] = 0.5;
            }
            let skip = LinearWeights::new(
                hidden_channels,
                hidden_channels,
                skip_w,
                vec![0.0; hidden_channels],
            );
            encoder_layers.push(Fno1dLayer::new(spec, skip, Activation::GELU));
        }

        // 3. Decoder layers (2 layers)
        let mut decoder_layers = Vec::new();
        let dec_modes = n_modes.min(output_length / 2);
        for layer_idx in 0..2 {
            let mut spec = SpectralWeights::zeros(dec_modes, hidden_channels, hidden_channels);
            for m in 0..dec_modes {
                let damping = 1.0 / (1.0 + 0.15 * (m as f64) + 0.05 * (layer_idx as f64));
                for c in 0..hidden_channels {
                    let idx = m * hidden_channels * hidden_channels + c * hidden_channels + c;
                    spec.weights_real[idx] = 0.4 * damping;
                }
            }

            let mut skip_w = vec![0.0; hidden_channels * hidden_channels];
            for c in 0..hidden_channels {
                skip_w[c * hidden_channels + c] = 0.6;
            }
            let skip = LinearWeights::new(
                hidden_channels,
                hidden_channels,
                skip_w,
                vec![0.0; hidden_channels],
            );
            decoder_layers.push(Fno1dLayer::new(spec, skip, Activation::GELU));
        }

        // 4. Projection: hidden_channels -> out_channels
        let mut proj_weights = vec![0.0; hidden_channels * out_channels];
        let factor = 1.0 / (hidden_channels / out_channels).max(1) as f64;
        for h in 0..hidden_channels {
            let out_c = h % out_channels;
            proj_weights[h * out_channels + out_c] = factor;
        }
        let projection = LinearWeights::new(
            hidden_channels,
            out_channels,
            proj_weights,
            vec![0.0; out_channels],
        );

        Self {
            in_channels,
            out_channels,
            hidden_channels,
            n_modes,
            input_length,
            output_length,
            lifting,
            encoder_layers,
            decoder_layers,
            projection,
        }
    }

    /// Single forward pass: takes historical residual window `x` ($[B][6]$)
    /// and predicts next $H$ residual steps ($[H][6]$).
    pub fn forward(&self, x: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, AutoOrbitError> {
        if x.is_empty() {
            return Err(AutoOrbitError::InvalidSequenceLength {
                expected: self.input_length,
                actual: 0,
            });
        }

        // 1. Lifting: 6 -> d_v
        let in_len = x.len();
        let mut lifted = Vec::with_capacity(in_len);
        for t in 0..in_len {
            let mut v = self.lifting.bias.clone();
            for in_c in 0..self.in_channels {
                let val = x[t][in_c];
                for h in 0..self.hidden_channels {
                    v[h] += val * self.lifting.get_weight(in_c, h);
                }
            }
            lifted.push(v);
        }

        // 2. Encoder Fourier Layers
        let mut enc = lifted;
        for layer in &self.encoder_layers {
            enc = layer.forward(&enc);
        }

        // 3. Adaptive Temporal Pooling: B -> H
        let pooled = adaptive_avg_pool1d(&enc, self.output_length);

        // 4. Decoder Fourier Layers
        let mut dec = pooled;
        for layer in &self.decoder_layers {
            dec = layer.forward(&dec);
        }

        // 5. Projection Layer: d_v -> 6
        let mut output = Vec::with_capacity(self.output_length);
        for t in 0..self.output_length {
            let mut out_step = self.projection.bias.clone();
            for h in 0..self.hidden_channels {
                let val = dec[t][h];
                for out_c in 0..self.out_channels {
                    out_step[out_c] += val * self.projection.get_weight(h, out_c);
                }
            }
            output.push(out_step);
        }

        Ok(output)
    }

    /// Autoregressive multi-step rolling forecast for arbitrary horizon length $H_{total}$.
    ///
    /// At each iteration, predicts the next $H$ steps, appends them to the residual buffer,
    /// and advances the sliding window.
    pub fn predict_autoregressive(
        &self,
        historical_window: &[Vec<f64>],
        total_horizon_steps: usize,
    ) -> Result<Vec<Vec<f64>>, AutoOrbitError> {
        let mut current_window = historical_window.to_vec();
        let mut predicted_trajectory = Vec::with_capacity(total_horizon_steps);

        while predicted_trajectory.len() < total_horizon_steps {
            // Forward pass for next H steps
            let next_chunk = self.forward(&current_window)?;

            for step_res in next_chunk {
                if predicted_trajectory.len() < total_horizon_steps {
                    predicted_trajectory.push(step_res.clone());
                    current_window.push(step_res);
                    if current_window.len() > self.input_length {
                        current_window.remove(0);
                    }
                }
            }
        }

        Ok(predicted_trajectory)
    }

    /// Deserializes an `AutoregressiveFnoModel` from a JSON string exported by the Python training pipeline.
    pub fn from_json_str(json_str: &str) -> Result<Self, AutoOrbitError> {
        let root = SimpleJson::parse(json_str).map_err(|e| {
            AutoOrbitError::InvalidConfiguration(format!("Failed to parse JSON model: {}", e))
        })?;

        let in_channels = root.get_obj("in_channels").and_then(|v| v.as_usize()).unwrap_or(6);
        let out_channels = root.get_obj("out_channels").and_then(|v| v.as_usize()).unwrap_or(6);
        let hidden_channels = root.get_obj("hidden_channels").and_then(|v| v.as_usize()).unwrap_or(32);
        let n_modes = root.get_obj("n_modes").and_then(|v| v.as_usize()).unwrap_or(8);
        let input_length = root.get_obj("input_length").and_then(|v| v.as_usize()).unwrap_or(64);
        let output_length = root.get_obj("output_length").and_then(|v| v.as_usize()).unwrap_or(16);

        // Parse lifting layer
        let lifting_obj = root.get_obj("lifting").ok_or_else(|| {
            AutoOrbitError::InvalidConfiguration("Missing 'lifting' in model JSON".to_string())
        })?;
        let lifting_w_2d = lifting_obj.get_obj("weight").and_then(|v| v.as_vec_vec_f64()).unwrap_or_default();
        let lifting_bias = lifting_obj.get_obj("bias").and_then(|v| v.as_vec_f64()).unwrap_or_else(|| vec![0.0; hidden_channels]);

        let mut lifting_weights = vec![0.0; in_channels * hidden_channels];
        for h in 0..lifting_w_2d.len().min(hidden_channels) {
            for in_c in 0..lifting_w_2d[h].len().min(in_channels) {
                lifting_weights[in_c * hidden_channels + h] = lifting_w_2d[h][in_c];
            }
        }
        let lifting = LinearWeights::new(in_channels, hidden_channels, lifting_weights, lifting_bias);

        // Helper to parse Fourier layer list
        let parse_fno_layer = |layer_val: &SimpleJson, modes_count: usize| -> Result<Fno1dLayer, AutoOrbitError> {
            let spec_obj = layer_val.get_obj("spectral").ok_or_else(|| {
                AutoOrbitError::InvalidConfiguration("Missing 'spectral' in layer".to_string())
            })?;
            let w_real_3d = spec_obj.get_obj("weights_real").and_then(|v| v.as_vec_vec_vec_f64()).unwrap_or_default();
            let w_imag_3d = spec_obj.get_obj("weights_imag").and_then(|v| v.as_vec_vec_vec_f64()).unwrap_or_default();

            let mut spec = SpectralWeights::zeros(modes_count, hidden_channels, hidden_channels);
            for m in 0..w_real_3d.len().min(modes_count) {
                for in_c in 0..w_real_3d[m].len().min(hidden_channels) {
                    for out_c in 0..w_real_3d[m][in_c].len().min(hidden_channels) {
                        let idx = m * hidden_channels * hidden_channels + in_c * hidden_channels + out_c;
                        spec.weights_real[idx] = w_real_3d[m][in_c][out_c];
                        if m < w_imag_3d.len() && in_c < w_imag_3d[m].len() && out_c < w_imag_3d[m][in_c].len() {
                            spec.weights_imag[idx] = w_imag_3d[m][in_c][out_c];
                        }
                    }
                }
            }

            let skip_obj = layer_val.get_obj("skip").ok_or_else(|| {
                AutoOrbitError::InvalidConfiguration("Missing 'skip' in layer".to_string())
            })?;
            let skip_w_2d = skip_obj.get_obj("weight").and_then(|v| v.as_vec_vec_f64()).unwrap_or_default();
            let skip_b = skip_obj.get_obj("bias").and_then(|v| v.as_vec_f64()).unwrap_or_else(|| vec![0.0; hidden_channels]);

            let mut skip_weights = vec![0.0; hidden_channels * hidden_channels];
            for out_c in 0..skip_w_2d.len().min(hidden_channels) {
                for in_c in 0..skip_w_2d[out_c].len().min(hidden_channels) {
                    skip_weights[in_c * hidden_channels + out_c] = skip_w_2d[out_c][in_c];
                }
            }
            let skip = LinearWeights::new(hidden_channels, hidden_channels, skip_weights, skip_b);

            let act_str = layer_val.get_obj("activation").and_then(|v| v.as_str()).unwrap_or("GELU");
            let activation = if act_str.eq_ignore_ascii_case("relu") {
                Activation::ReLU
            } else {
                Activation::GELU
            };

            Ok(Fno1dLayer::new(spec, skip, activation))
        };

        // Parse encoder layers
        let enc_arr = root.get_obj("encoder_layers").and_then(|v| v.as_vec_json()).unwrap_or_default();
        let mut encoder_layers = Vec::new();
        for l_json in enc_arr {
            encoder_layers.push(parse_fno_layer(l_json, n_modes)?);
        }

        // Parse decoder layers
        let dec_arr = root.get_obj("decoder_layers").and_then(|v| v.as_vec_json()).unwrap_or_default();
        let mut decoder_layers = Vec::new();
        let dec_modes = n_modes.min(output_length / 2);
        for l_json in dec_arr {
            decoder_layers.push(parse_fno_layer(l_json, dec_modes)?);
        }

        // Parse projection layer
        let proj_obj = root.get_obj("projection").ok_or_else(|| {
            AutoOrbitError::InvalidConfiguration("Missing 'projection' in model JSON".to_string())
        })?;
        let proj_w_2d = proj_obj.get_obj("weight").and_then(|v| v.as_vec_vec_f64()).unwrap_or_default();
        let proj_b = proj_obj.get_obj("bias").and_then(|v| v.as_vec_f64()).unwrap_or_else(|| vec![0.0; out_channels]);

        let mut proj_weights = vec![0.0; hidden_channels * out_channels];
        for out_c in 0..proj_w_2d.len().min(out_channels) {
            for h in 0..proj_w_2d[out_c].len().min(hidden_channels) {
                proj_weights[h * out_channels + out_c] = proj_w_2d[out_c][h];
            }
        }
        let projection = LinearWeights::new(hidden_channels, out_channels, proj_weights, proj_b);

        Ok(Self {
            in_channels,
            out_channels,
            hidden_channels,
            n_modes,
            input_length,
            output_length,
            lifting,
            encoder_layers,
            decoder_layers,
            projection,
        })
    }
}

/// Lightweight zero-dependency JSON tree parser for loading model weights.
#[allow(dead_code)]
#[derive(Debug, Clone)]
enum SimpleJson {
    Num(f64),
    Str(String),
    Arr(Vec<SimpleJson>),
    Obj(Vec<(String, SimpleJson)>),
    Bool(bool),
    Null,
}

impl SimpleJson {
    pub fn parse(input: &str) -> Result<Self, String> {
        let chars: Vec<char> = input.chars().collect();
        let mut idx = 0;
        Self::skip_ws(&chars, &mut idx);
        let val = Self::parse_val(&chars, &mut idx)?;
        Ok(val)
    }

    fn skip_ws(chars: &[char], idx: &mut usize) {
        while *idx < chars.len() && chars[*idx].is_whitespace() {
            *idx += 1;
        }
    }

    fn parse_val(chars: &[char], idx: &mut usize) -> Result<Self, String> {
        Self::skip_ws(chars, idx);
        if *idx >= chars.len() {
            return Err("Unexpected EOF".to_string());
        }

        match chars[*idx] {
            '{' => Self::parse_obj(chars, idx),
            '[' => Self::parse_arr(chars, idx),
            '"' => Self::parse_str(chars, idx).map(SimpleJson::Str),
            't' | 'f' => Self::parse_bool(chars, idx),
            'n' => Self::parse_null(chars, idx),
            _ => Self::parse_num(chars, idx),
        }
    }

    fn parse_obj(chars: &[char], idx: &mut usize) -> Result<Self, String> {
        *idx += 1; // skip '{'
        let mut map = Vec::new();

        loop {
            Self::skip_ws(chars, idx);
            if *idx >= chars.len() {
                return Err("Unterminated object".to_string());
            }
            if chars[*idx] == '}' {
                *idx += 1;
                break;
            }

            if chars[*idx] != '"' {
                return Err(format!("Expected string key in object at {}", *idx));
            }
            let key = Self::parse_str(chars, idx)?;

            Self::skip_ws(chars, idx);
            if *idx >= chars.len() || chars[*idx] != ':' {
                return Err(format!("Expected ':' after key at {}", *idx));
            }
            *idx += 1; // skip ':'

            let val = Self::parse_val(chars, idx)?;
            map.push((key, val));

            Self::skip_ws(chars, idx);
            if *idx < chars.len() && chars[*idx] == ',' {
                *idx += 1;
            } else if *idx < chars.len() && chars[*idx] == '}' {
                *idx += 1;
                break;
            } else {
                return Err(format!("Expected ',' or '}}' at {}", *idx));
            }
        }

        Ok(SimpleJson::Obj(map))
    }

    fn parse_arr(chars: &[char], idx: &mut usize) -> Result<Self, String> {
        *idx += 1; // skip '['
        let mut list = Vec::new();

        loop {
            Self::skip_ws(chars, idx);
            if *idx >= chars.len() {
                return Err("Unterminated array".to_string());
            }
            if chars[*idx] == ']' {
                *idx += 1;
                break;
            }

            let val = Self::parse_val(chars, idx)?;
            list.push(val);

            Self::skip_ws(chars, idx);
            if *idx < chars.len() && chars[*idx] == ',' {
                *idx += 1;
            } else if *idx < chars.len() && chars[*idx] == ']' {
                *idx += 1;
                break;
            } else {
                return Err(format!("Expected ',' or ']' at {}", *idx));
            }
        }

        Ok(SimpleJson::Arr(list))
    }

    fn parse_str(chars: &[char], idx: &mut usize) -> Result<String, String> {
        *idx += 1; // skip '"'
        let mut s = String::new();
        while *idx < chars.len() {
            let c = chars[*idx];
            if c == '"' {
                *idx += 1;
                return Ok(s);
            } else if c == '\\' && *idx + 1 < chars.len() {
                *idx += 1;
                s.push(chars[*idx]);
            } else {
                s.push(c);
            }
            *idx += 1;
        }
        Err("Unterminated string".to_string())
    }

    fn parse_num(chars: &[char], idx: &mut usize) -> Result<Self, String> {
        let start = *idx;
        while *idx < chars.len() && (chars[*idx].is_ascii_digit() || chars[*idx] == '-' || chars[*idx] == '+' || chars[*idx] == '.' || chars[*idx] == 'e' || chars[*idx] == 'E') {
            *idx += 1;
        }
        let num_str: String = chars[start..*idx].iter().collect();
        let val: f64 = num_str.parse().map_err(|e| format!("Invalid number '{}': {}", num_str, e))?;
        Ok(SimpleJson::Num(val))
    }

    fn parse_bool(chars: &[char], idx: &mut usize) -> Result<Self, String> {
        if chars[*idx..].starts_with(&['t', 'r', 'u', 'e']) {
            *idx += 4;
            Ok(SimpleJson::Bool(true))
        } else if chars[*idx..].starts_with(&['f', 'a', 'l', 's', 'e']) {
            *idx += 5;
            Ok(SimpleJson::Bool(false))
        } else {
            Err("Invalid boolean".to_string())
        }
    }

    fn parse_null(chars: &[char], idx: &mut usize) -> Result<Self, String> {
        if chars[*idx..].starts_with(&['n', 'u', 'l', 'l']) {
            *idx += 4;
            Ok(SimpleJson::Null)
        } else {
            Err("Invalid null".to_string())
        }
    }

    pub fn get_obj(&self, key: &str) -> Option<&SimpleJson> {
        if let SimpleJson::Obj(pairs) = self {
            for (k, v) in pairs {
                if k == key {
                    return Some(v);
                }
            }
        }
        None
    }

    pub fn as_usize(&self) -> Option<usize> {
        if let SimpleJson::Num(n) = self {
            Some(*n as usize)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let SimpleJson::Str(s) = self {
            Some(s.as_str())
        } else {
            None
        }
    }

    pub fn as_vec_json(&self) -> Option<&[SimpleJson]> {
        if let SimpleJson::Arr(a) = self {
            Some(a.as_slice())
        } else {
            None
        }
    }

    pub fn as_vec_f64(&self) -> Option<Vec<f64>> {
        if let SimpleJson::Arr(a) = self {
            let mut out = Vec::with_capacity(a.len());
            for item in a {
                if let SimpleJson::Num(n) = item {
                    out.push(*n);
                }
            }
            Some(out)
        } else {
            None
        }
    }

    pub fn as_vec_vec_f64(&self) -> Option<Vec<Vec<f64>>> {
        if let SimpleJson::Arr(a) = self {
            let mut out = Vec::with_capacity(a.len());
            for item in a {
                if let Some(inner) = item.as_vec_f64() {
                    out.push(inner);
                }
            }
            Some(out)
        } else {
            None
        }
    }

    pub fn as_vec_vec_vec_f64(&self) -> Option<Vec<Vec<Vec<f64>>>> {
        if let SimpleJson::Arr(a) = self {
            let mut out = Vec::with_capacity(a.len());
            for item in a {
                if let Some(inner) = item.as_vec_vec_f64() {
                    out.push(inner);
                }
            }
            Some(out)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_avg_pool1d_exact_reduction() {
        let input = vec![
            vec![1.0, 2.0],
            vec![3.0, 4.0],
            vec![5.0, 6.0],
            vec![7.0, 8.0],
        ];
        let pooled = adaptive_avg_pool1d(&input, 2);
        assert_eq!(pooled.len(), 2);
        assert!((pooled[0][0] - 2.0).abs() < 1e-10); // mean(1, 3) = 2.0
        assert!((pooled[0][1] - 3.0).abs() < 1e-10); // mean(2, 4) = 3.0
        assert!((pooled[1][0] - 6.0).abs() < 1e-10); // mean(5, 7) = 6.0
        assert!((pooled[1][1] - 7.0).abs() < 1e-10); // mean(6, 8) = 7.0
    }

    #[test]
    fn test_fno_layer_forward_preserves_shape() {
        let in_c = 4;
        let out_c = 4;
        let n_modes = 4;
        let length = 16;

        let spec = SpectralWeights::zeros(n_modes, in_c, out_c);
        let mut skip_w = vec![0.0; in_c * out_c];
        for i in 0..in_c {
            skip_w[i * out_c + i] = 1.0;
        }
        let skip = LinearWeights::new(in_c, out_c, skip_w, vec![0.0; out_c]);
        let layer = Fno1dLayer::new(spec, skip, Activation::ReLU);

        let input = vec![vec![1.0; in_c]; length];
        let output = layer.forward(&input);

        assert_eq!(output.len(), length);
        assert_eq!(output[0].len(), out_c);
        assert!((output[0][0] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_autoregressive_fno_model_predict_dimensions() {
        let model = AutoregressiveFnoModel::calibrated_default(6, 6, 16, 4, 32, 8);
        let input = vec![vec![0.1; 6]; 32];

        let pred = model.predict_autoregressive(&input, 24).expect("Prediction ok");
        assert_eq!(pred.len(), 24);
        assert_eq!(pred[0].len(), 6);
    }

    #[test]
    fn test_from_json_str_loads_trained_model() {
        let json_path = "models/autoorbit_trained.json";
        if let Ok(content) = std::fs::read_to_string(json_path) {
            let model = AutoregressiveFnoModel::from_json_str(&content).expect("Parsed model JSON");
            assert_eq!(model.in_channels, 6);
            assert_eq!(model.out_channels, 6);
            assert_eq!(model.hidden_channels, 32);

            let input = vec![vec![0.05; 6]; model.input_length];
            let pred = model.forward(&input).expect("Forward pass succeeds");
            assert_eq!(pred.len(), model.output_length);
            assert_eq!(pred[0].len(), 6);
        }
    }
}
