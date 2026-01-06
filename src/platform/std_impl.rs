//! Standard library implementations of platform traits.
//!
//! These implementations are only available when the `std` feature is enabled.

use super::{ConsoleLevel, ConsoleProvider, RandomProvider, TimeProvider};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Time provider using std::time.
pub struct StdTimeProvider {
    /// Reference instant for timer calculations
    epoch: Instant,
}

impl StdTimeProvider {
    /// Create a new StdTimeProvider.
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
        }
    }
}

impl Default for StdTimeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeProvider for StdTimeProvider {
    fn now_millis(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    fn elapsed_millis(&self, start: u64) -> u64 {
        let now = self.epoch.elapsed().as_millis() as u64;
        now.saturating_sub(start)
    }

    fn start_timer(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }
}

/// Random provider using a simple xorshift64 PRNG.
///
/// This is a fast, decent-quality PRNG suitable for Math.random().
/// It's seeded from the current time on creation.
pub struct StdRandomProvider {
    state: u64,
}

impl StdRandomProvider {
    /// Create a new StdRandomProvider with time-based seed.
    pub fn new() -> Self {
        // Seed from current time
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x12345678_9abcdef0);

        // Ensure non-zero seed
        let seed = if seed == 0 {
            0x12345678_9abcdef0
        } else {
            seed
        };

        Self { state: seed }
    }

    /// Create with a specific seed (for testing).
    #[allow(dead_code)]
    pub fn with_seed(seed: u64) -> Self {
        let seed = if seed == 0 { 1 } else { seed };
        Self { state: seed }
    }
}

impl Default for StdRandomProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomProvider for StdRandomProvider {
    fn random(&mut self) -> f64 {
        // xorshift64 algorithm
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;

        // Convert to f64 in [0, 1)
        // Use the upper 53 bits for better distribution
        let mantissa = x >> 11; // 53 bits
        (mantissa as f64) / ((1u64 << 53) as f64)
    }
}

/// Console provider using std print macros.
///
/// Writes to stdout for Log/Info/Debug and stderr for Warn/Error.
pub struct StdConsoleProvider;

impl StdConsoleProvider {
    /// Create a new StdConsoleProvider.
    pub fn new() -> Self {
        Self
    }
}

impl Default for StdConsoleProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleProvider for StdConsoleProvider {
    fn write(&self, level: ConsoleLevel, message: &str) {
        match level {
            ConsoleLevel::Log | ConsoleLevel::Info | ConsoleLevel::Debug => {
                println!("{message}");
            }
            ConsoleLevel::Warn | ConsoleLevel::Error => {
                eprintln!("{message}");
            }
        }
    }

    fn clear(&self) {
        // Print some newlines as a visual separator
        println!("\n--- Console cleared ---\n");
    }
}
