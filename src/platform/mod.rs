//! Platform abstraction traits for no_std compatibility.
//!
//! This module defines traits that abstract over platform-specific functionality,
//! allowing the interpreter to run in both std and no_std environments.

use crate::prelude::{Rc, String, Vec};
use core::fmt::Debug;

#[cfg(feature = "std")]
mod std_impl;

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
mod wasm_impl;

#[cfg(feature = "std")]
pub use std_impl::{StdConsoleProvider, StdRandomProvider, StdTimeProvider};

#[cfg(feature = "regex")]
pub use std_impl::FancyRegexProvider;

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub use wasm_impl::{WasmConsoleProvider, WasmRandomProvider, WasmRegExpProvider, WasmTimeProvider};

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

// ═══════════════════════════════════════════════════════════════════════════════
// RegExp Provider
// ═══════════════════════════════════════════════════════════════════════════════

/// Result from a single regex match.
///
/// Contains byte offsets (not character offsets) for the match and capture groups.
#[derive(Debug, Clone)]
pub struct RegexMatch {
    /// Start byte offset of the full match in the input string.
    pub start: usize,
    /// End byte offset (exclusive) of the full match.
    pub end: usize,
    /// Capture groups. Index 0 is the full match, 1+ are numbered groups.
    /// `None` entries represent non-participating optional groups.
    pub captures: Vec<Option<(usize, usize)>>,
}

/// A compiled regular expression.
///
/// This trait abstracts over different regex implementations, allowing
/// the interpreter to use browser's native RegExp in WASM, custom C
/// implementations via FFI, or the default `fancy-regex` in std builds.
pub trait CompiledRegex: Debug {
    /// Test if the regex matches anywhere in the input string.
    fn is_match(&self, input: &str) -> Result<bool, String>;

    /// Find the first match starting at or after `start_pos` (byte offset).
    ///
    /// Returns `None` if no match is found.
    fn find(&self, input: &str, start_pos: usize) -> Result<Option<RegexMatch>, String>;

    /// Find all non-overlapping matches in the input string.
    fn find_iter(&self, input: &str) -> Result<Vec<RegexMatch>, String>;

    /// Split the input string by matches of this regex.
    ///
    /// Returns the substrings between matches.
    fn split(&self, input: &str) -> Result<Vec<String>, String>;

    /// Replace the first match with the replacement string.
    ///
    /// The replacement string can contain:
    /// - `$1`, `$2`, ... for capture group references
    /// - `$&` for the full match
    /// - `$$` for a literal `$`
    fn replace(&self, input: &str, replacement: &str) -> Result<String, String>;

    /// Replace all matches with the replacement string.
    fn replace_all(&self, input: &str, replacement: &str) -> Result<String, String>;
}

/// Trait for providing regex compilation functionality.
///
/// Implementations can wrap different regex engines:
/// - `FancyRegexProvider`: Uses the `fancy-regex` crate (default for std builds)
/// - `WasmRegExpProvider`: Delegates to browser's native RegExp via wasm-bindgen
/// - Custom C implementations via FFI callbacks
pub trait RegExpProvider {
    /// Compile a regex pattern with the given flags.
    ///
    /// # Flags
    /// - `g`: global (affects JS-level iteration, not compilation)
    /// - `i`: case-insensitive matching
    /// - `m`: multiline (^ and $ match line boundaries)
    /// - `s`: dotAll (. matches newlines)
    /// - `u`: unicode
    /// - `y`: sticky (match only at lastIndex)
    ///
    /// Returns a compiled regex that can be used for matching operations.
    fn compile(&self, pattern: &str, flags: &str) -> Result<Rc<dyn CompiledRegex>, String>;
}

/// A no-op regex provider that always returns errors.
///
/// Used as a fallback when no regex implementation is available
/// (e.g., in no_std environments without a custom provider).
pub struct NoOpRegExpProvider;

impl RegExpProvider for NoOpRegExpProvider {
    fn compile(&self, _pattern: &str, _flags: &str) -> Result<Rc<dyn CompiledRegex>, String> {
        Err("RegExp not available: enable 'regex' feature or provide a custom RegExpProvider".into())
    }
}
