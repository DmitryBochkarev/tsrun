//! WebAssembly implementations of platform traits.
//!
//! These implementations use browser APIs via wasm-bindgen for:
//! - Console output (console.log, etc.)
//! - Time (Date.now(), performance.now())
//! - Random numbers (Math.random())

use super::{ConsoleLevel, ConsoleProvider, RandomProvider, TimeProvider};
use wasm_bindgen::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════════
// JavaScript bindings
// ═══════════════════════════════════════════════════════════════════════════════

#[wasm_bindgen]
extern "C" {
    // Console bindings
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn info(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn debug(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn warn(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = clear)]
    fn console_clear();

    // Date.now() for wall-clock time
    #[wasm_bindgen(js_namespace = Date)]
    fn now() -> f64;

    // performance.now() for high-resolution timing
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn performance_now() -> f64;

    // Math.random() for random numbers
    #[wasm_bindgen(js_namespace = Math)]
    fn random() -> f64;
}

// ═══════════════════════════════════════════════════════════════════════════════
// Console Provider
// ═══════════════════════════════════════════════════════════════════════════════

/// Console provider using browser's console object.
///
/// Routes console.log(), console.error(), etc. to the browser's console.
pub struct WasmConsoleProvider;

impl WasmConsoleProvider {
    /// Create a new WasmConsoleProvider.
    pub fn new() -> Self {
        Self
    }
}

impl Default for WasmConsoleProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleProvider for WasmConsoleProvider {
    fn write(&self, level: ConsoleLevel, message: &str) {
        match level {
            ConsoleLevel::Log => log(message),
            ConsoleLevel::Info => info(message),
            ConsoleLevel::Debug => debug(message),
            ConsoleLevel::Warn => warn(message),
            ConsoleLevel::Error => error(message),
        }
    }

    fn clear(&self) {
        console_clear();
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Time Provider
// ═══════════════════════════════════════════════════════════════════════════════

/// Time provider using browser's Date and performance APIs.
///
/// Uses Date.now() for wall-clock time and performance.now() for timing.
pub struct WasmTimeProvider;

impl WasmTimeProvider {
    /// Create a new WasmTimeProvider.
    pub fn new() -> Self {
        Self
    }
}

impl Default for WasmTimeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeProvider for WasmTimeProvider {
    fn now_millis(&self) -> i64 {
        now() as i64
    }

    fn elapsed_millis(&self, start: u64) -> u64 {
        let current = performance_now() as u64;
        current.saturating_sub(start)
    }

    fn start_timer(&self) -> u64 {
        performance_now() as u64
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Random Provider
// ═══════════════════════════════════════════════════════════════════════════════

/// Random provider using browser's Math.random().
pub struct WasmRandomProvider;

impl WasmRandomProvider {
    /// Create a new WasmRandomProvider.
    pub fn new() -> Self {
        Self
    }
}

impl Default for WasmRandomProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomProvider for WasmRandomProvider {
    fn random(&mut self) -> f64 {
        random()
    }
}
