//! Integration tests for the interpreter, organized by feature
//!
//! These tests exercise the interpreter through the public API.
//!
//! ## GC Stress Testing
//!
//! Set the `GC_THRESHOLD` environment variable to override the default GC threshold
//! for all tests. This is useful for finding GC-related bugs:
//!
//! ```bash
//! GC_THRESHOLD=1 cargo test           # Most aggressive - GC on every allocation
//! GC_THRESHOLD=10 cargo test          # Frequent GC
//! GC_THRESHOLD=100 cargo test         # Moderate GC frequency
//! ```

mod array;
mod async_await;
mod basics;
mod class;
mod console;
mod control_flow;
mod date;
mod dynamic_import;
mod error;
mod exports;
mod function;
mod gc;
mod generator;
mod global;
mod json;
mod map;
mod math;
mod namespace;
mod number;
mod object;
mod promise;
mod regexp;
mod set;
mod state_machine;
mod string;
mod symbol;
mod timeout;

use std::sync::OnceLock;
use typescript_eval::{JsError, JsValue, Runtime, RuntimeResult};

/// Get GC threshold from environment variable, or None for default
fn get_gc_threshold_from_env() -> Option<usize> {
    static GC_THRESHOLD: OnceLock<Option<usize>> = OnceLock::new();
    *GC_THRESHOLD.get_or_init(|| {
        std::env::var("GC_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
    })
}

/// Create a new runtime with optional GC threshold from environment
fn create_test_runtime() -> Runtime {
    let mut runtime = Runtime::new();
    if let Some(threshold) = get_gc_threshold_from_env() {
        runtime.set_gc_threshold(threshold);
        // Disable timeout for GC stress tests (they run slower)
        if threshold <= 10 {
            runtime.set_timeout_ms(0);
        }
    }
    runtime
}

/// Helper function to evaluate TypeScript source code
pub fn eval(source: &str) -> JsValue {
    let mut runtime = create_test_runtime();
    match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    }
}

/// Helper function to evaluate and return Result for error testing
pub fn eval_result(source: &str) -> Result<JsValue, JsError> {
    let mut runtime = create_test_runtime();
    match runtime.eval(source)? {
        RuntimeResult::Complete(value) => Ok(value),
        other => panic!("Expected Complete, got {:?}", other),
    }
}

/// Helper to check if evaluation throws an error containing a specific message
pub fn throws_error(source: &str, error_contains: &str) -> bool {
    match eval_result(source) {
        Err(e) => format!("{:?}", e).contains(error_contains),
        Ok(_) => false,
    }
}
