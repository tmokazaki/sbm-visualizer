//! Structured error definitions for the NASA EVOLVE 4.0 breakup engine.

use std::fmt;

/// Errors that can occur during breakup engine configuration or simulation.
#[derive(Debug, Clone, PartialEq)]
pub enum BreakupError {
    /// A mass parameter was non-positive, zero, or non-finite.
    InvalidMass {
        parameter: &'static str,
        value: f64,
    },
    /// Impact velocity was negative or non-finite.
    InvalidVelocity {
        value: f64,
    },
    /// The requested number of fragments was zero.
    InvalidFragmentCount {
        value: usize,
    },
    /// The explosion scaling factor was non-positive or non-finite.
    InvalidScaling {
        value: f64,
    },
    /// The size cutoff range [min_size, max_size] was invalid.
    InvalidSizeRange {
        min_size: f64,
        max_size: f64,
    },
    /// The power-law exponent was invalid (must be greater than 1.0).
    InvalidPowerLawExponent {
        value: f64,
    },
}

impl fmt::Display for BreakupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BreakupError::InvalidMass { parameter, value } => {
                write!(f, "Invalid mass for parameter '{}': {} kg (must be positive and finite)", parameter, value)
            }
            BreakupError::InvalidVelocity { value } => {
                write!(f, "Invalid impact velocity: {} m/s (must be non-negative and finite)", value)
            }
            BreakupError::InvalidFragmentCount { value } => {
                write!(f, "Invalid fragment count: {} (must be greater than zero)", value)
            }
            BreakupError::InvalidScaling { value } => {
                write!(f, "Invalid explosion scaling factor: {} (must be positive and finite)", value)
            }
            BreakupError::InvalidSizeRange { min_size, max_size } => {
                write!(f, "Invalid size cutoff range: min_size={} m, max_size={} m (must satisfy 0 < min_size < max_size)", min_size, max_size)
            }
            BreakupError::InvalidPowerLawExponent { value } => {
                write!(f, "Invalid power-law exponent: {} (must be greater than 1.0)", value)
            }
        }
    }
}

impl std::error::Error for BreakupError {}
