//! Binary model checkpoint serialization and weight loading.
//!
//! Supports:
//! 1. Saving/loading full Actor-Critic checkpoints (`.bin`) for resume and training pipelines
//! 2. Loading policy-only inference weights (e.g. from the paper's pre-trained checkpoint)

use crate::nn::ActorCritic;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

const MAGIC_HEADER: &[u8; 8] = b"SBMPPO01";

/// Exports the full Actor-Critic model weights and biases to a compact binary file.
pub fn save_model_checkpoint(model: &ActorCritic, path: impl AsRef<Path>) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(MAGIC_HEADER)?;

    // Helper closure to write a slice of f32 as little-endian bytes
    let mut write_slice = |slice: &[f32]| -> std::io::Result<()> {
        for &val in slice {
            file.write_all(&val.to_le_bytes())?;
        }
        Ok(())
    };

    // Actor parameters
    write_slice(&model.policy_fc1.weights)?;
    write_slice(&model.policy_fc1.bias)?;
    write_slice(&model.policy_fc2.weights)?;
    write_slice(&model.policy_fc2.bias)?;
    write_slice(&model.policy_fc3.weights)?;
    write_slice(&model.policy_fc3.bias)?;
    write_slice(&model.action_head.weights)?;
    write_slice(&model.action_head.bias)?;
    write_slice(&model.log_std)?;

    // Critic parameters
    write_slice(&model.value_fc1.weights)?;
    write_slice(&model.value_fc1.bias)?;
    write_slice(&model.value_fc2.weights)?;
    write_slice(&model.value_fc2.bias)?;
    write_slice(&model.value_fc3.weights)?;
    write_slice(&model.value_fc3.bias)?;
    write_slice(&model.value_head.weights)?;
    write_slice(&model.value_head.bias)?;

    file.flush()?;
    Ok(())
}

/// Loads full Actor-Critic model weights from a `.bin` checkpoint file.
pub fn load_model_checkpoint(path: impl AsRef<Path>) -> Result<ActorCritic, String> {
    let mut file = File::open(path.as_ref()).map_err(|e| e.to_string())?;
    let mut header = [0u8; 8];
    file.read_exact(&mut header).map_err(|e| e.to_string())?;

    if &header != MAGIC_HEADER {
        return Err("Invalid checkpoint header magic".into());
    }

    let mut model = ActorCritic::new(42);

    let mut read_slice = |slice: &mut [f32]| -> Result<(), String> {
        let mut buf = [0u8; 4];
        for val in slice.iter_mut() {
            file.read_exact(&mut buf).map_err(|e| e.to_string())?;
            *val = f32::from_le_bytes(buf);
        }
        Ok(())
    };

    // Actor
    read_slice(&mut model.policy_fc1.weights)?;
    read_slice(&mut model.policy_fc1.bias)?;
    read_slice(&mut model.policy_fc2.weights)?;
    read_slice(&mut model.policy_fc2.bias)?;
    read_slice(&mut model.policy_fc3.weights)?;
    read_slice(&mut model.policy_fc3.bias)?;
    read_slice(&mut model.action_head.weights)?;
    read_slice(&mut model.action_head.bias)?;
    read_slice(&mut model.log_std)?;

    // Critic
    read_slice(&mut model.value_fc1.weights)?;
    read_slice(&mut model.value_fc1.bias)?;
    read_slice(&mut model.value_fc2.weights)?;
    read_slice(&mut model.value_fc2.bias)?;
    read_slice(&mut model.value_fc3.weights)?;
    read_slice(&mut model.value_fc3.bias)?;
    read_slice(&mut model.value_head.weights)?;
    read_slice(&mut model.value_head.bias)?;

    Ok(model)
}

/// Loads policy actor weights from raw byte buffer (such as extracted PyTorch weights or embedded bytes).
/// Expected order: `w0 (256x307), b0 (256), w1 (256x256), b1 (256), w2 (128x256), b2 (128), w_act (3x128), b_act (3)`.
pub fn load_policy_weights_raw(raw_bytes: &[u8]) -> Result<ActorCritic, String> {
    const EXPECTED_BYTES: usize = (256 * 307 + 256 + 256 * 256 + 256 + 128 * 256 + 128 + 3 * 128 + 3) * 4;
    if raw_bytes.len() < EXPECTED_BYTES {
        return Err(format!(
            "Insufficient raw weight bytes: expected {}, found {}",
            EXPECTED_BYTES,
            raw_bytes.len()
        ));
    }

    let mut model = ActorCritic::new(42);
    let mut offset = 0;

    let mut read_section = |slice: &mut [f32]| {
        for val in slice.iter_mut() {
            let chunk = [
                raw_bytes[offset],
                raw_bytes[offset + 1],
                raw_bytes[offset + 2],
                raw_bytes[offset + 3],
            ];
            *val = f32::from_le_bytes(chunk);
            offset += 4;
        }
    };

    read_section(&mut model.policy_fc1.weights);
    read_section(&mut model.policy_fc1.bias);
    read_section(&mut model.policy_fc2.weights);
    read_section(&mut model.policy_fc2.bias);
    read_section(&mut model.policy_fc3.weights);
    read_section(&mut model.policy_fc3.bias);
    read_section(&mut model.action_head.weights);
    read_section(&mut model.action_head.bias);

    Ok(model)
}
