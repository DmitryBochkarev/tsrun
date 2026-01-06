//! Platform abstraction traits for no_std compatibility.
//!
//! This module defines traits that abstract over platform-specific functionality,
//! allowing the interpreter to run in both std and no_std environments.

#[cfg(feature = "std")]
mod std_impl;

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
mod wasm_impl;

#[cfg(feature = "std")]
pub use std_impl::{StdConsoleProvider, StdRandomProvider, StdTimeProvider};

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub use wasm_impl::{WasmConsoleProvider, WasmRandomProvider, WasmTimeProvider};

/// Trait for providing time-related functionality.
///
/// In std environments, this is implemented using `std::time::Instant` and `SystemTime`.
/// In no_std environments, the host must provide an implementation.
pub trait TimeProvider {
    /// Get the current time as milliseconds since Unix epoch.
    /// Used for `Date.now()`.
    fn now_millis(&self) -> i64;

    /// Get elapsed milliseconds since a timer was started.
    /// Used for `console.time()` / `console.timeEnd()`.
    fn elapsed_millis(&self, start: u64) -> u64;

    /// Start a timer and return an opaque handle.
    /// The handle can be passed to `elapsed_millis` to get the elapsed time.
    fn start_timer(&self) -> u64;
}

/// Trait for providing random number generation.
///
/// In std environments, this uses thread-local RNG.
/// In no_std environments, the host must provide an implementation.
pub trait RandomProvider {
    /// Generate a random f64 in the range [0, 1).
    /// Used for `Math.random()`.
    fn random(&mut self) -> f64;
}

/// A no-op time provider that returns constant values.
/// Used as a fallback in no_std environments when the host doesn't provide time.
pub struct NoOpTimeProvider;

impl TimeProvider for NoOpTimeProvider {
    fn now_millis(&self) -> i64 {
        0
    }

    fn elapsed_millis(&self, _start: u64) -> u64 {
        0
    }

    fn start_timer(&self) -> u64 {
        0
    }
}

/// A no-op random provider that returns a constant value.
/// Used as a fallback in no_std environments when the host doesn't provide randomness.
pub struct NoOpRandomProvider;

impl RandomProvider for NoOpRandomProvider {
    fn random(&mut self) -> f64 {
        // Return 0.5 as a predictable fallback
        0.5
    }
}

/// Log level for console output.
///
/// Maps to the different console methods: console.log(), console.warn(), etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleLevel {
    /// console.log() - general output
    Log,
    /// console.info() - informational messages
    Info,
    /// console.debug() - debug messages
    Debug,
    /// console.warn() - warnings
    Warn,
    /// console.error() - errors
    Error,
}

/// Trait for handling console output.
///
/// In std environments, this writes to stdout/stderr.
/// In WASM environments, this calls the browser's console object.
/// In no_std environments, this is a no-op.
pub trait ConsoleProvider {
    /// Write a message at the specified log level.
    fn write(&self, level: ConsoleLevel, message: &str);

    /// Clear the console (optional operation, may be no-op).
    fn clear(&self) {}
}

/// A no-op console provider that discards all output.
/// Used as a fallback in no_std environments.
pub struct NoOpConsoleProvider;

impl ConsoleProvider for NoOpConsoleProvider {
    fn write(&self, _level: ConsoleLevel, _message: &str) {
        // Discard output
    }
}
