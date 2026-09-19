//! Random number generator abstractions and zero-dependency PRNG implementation.

use std::f64::consts::PI;

/// Trait defining minimum random sampling capabilities required by the simulation engine.
pub trait RngSource {
    /// Returns a uniform random float in the half-open interval $[0.0, 1.0)$.
    fn next_f64(&mut self) -> f64;

    /// Samples a standard normal variable $Z \sim \mathcal{N}(0, 1)$ via Box-Muller transform.
    fn rand_normal(&mut self) -> f64 {
        let mut u = 0.0;
        let mut v = 0.0;
        while u == 0.0 {
            u = self.next_f64();
        }
        while v == 0.0 {
            v = self.next_f64();
        }
        (-2.0 * u.ln()).sqrt() * (2.0 * PI * v).cos()
    }
}

/// Simple, zero-dependency, high-throughput pseudo-random number generator (Xorshift64).
#[derive(Debug, Clone)]
pub struct SimpleRng {
    /// Internal 64-bit state.
    pub state: u64,
}

impl SimpleRng {
    /// Creates a new `SimpleRng` initialized with the given seed.
    /// If `seed == 0`, a default non-zero state (`0xdeadbeef_cafebabe`) is used.
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0xdeadbeef_cafebabe
            } else {
                seed
            },
        }
    }

    /// Generates the next pseudo-random unsigned 64-bit integer using Xorshift64.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

impl RngSource for SimpleRng {
    fn next_f64(&mut self) -> f64 {
        // Shift by 11 bits to obtain 53 bits of precision for IEEE 754 f64 mantissa
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}
