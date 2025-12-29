//! Integration tests for the interpreter, organized by feature
//!
//! These tests exercise the interpreter through the public API.
//!
//! ## Aggressive Test Defaults
//!
//! Tests use aggressive defaults to catch bugs early:
//! - `GC_THRESHOLD=1` - GC on every allocation to catch GC bugs
//! - `MAX_CALL_DEPTH=50` - Low recursion limit to catch infinite loops before Rust stack overflow
//!
//! Override via environment variables:
//!
//! ```bash
//! cargo test                           # Default: aggressive settings
//! GC_THRESHOLD=100 cargo test          # Less aggressive GC for faster runs
//! MAX_CALL_DEPTH=256 cargo test        # Higher recursion limit
//! ```

mod api;
mod array;
mod async_await;
mod async_iter;
mod basics;
mod boolean;
mod bytecode;
mod class;
mod console;
mod control_flow;
mod cycle_leak;
mod date;
mod decorator;
mod enum_test;
mod error;
mod eval;
mod function;
mod gc;
mod generator;
mod global;
mod json;
mod map;
mod math;
mod modules;
mod namespace;
mod number;
mod object;
mod orders;
mod promise;
mod proxy;
mod regexp;
mod set;
mod strict;
mod string;
mod symbol;
mod typescript;

use tsrun::{JsError, Runtime, RuntimeResult, RuntimeValue};

/// Create a new runtime with aggressive defaults for testing:
/// - GC_THRESHOLD=1 (GC on every allocation) to catch GC bugs
/// - MAX_CALL_DEPTH=50 to catch infinite recursion before Rust stack overflow
fn create_test_runtime() -> Runtime {
    let mut runtime = Runtime::new();

    // Default to GC_THRESHOLD=1 (most aggressive) to catch GC bugs early
    // Override via environment variable if needed:
    // GC_THRESHOLD=100 cargo test  # Faster runs
    // GC_THRESHOLD=0 cargo test    # Disable automatic GC
    let gc_threshold = std::env::var("GC_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    runtime.set_gc_threshold(gc_threshold);

    // Default to MAX_CALL_DEPTH=50 to catch infinite recursion early
    // Override via environment variable if needed:
    // MAX_CALL_DEPTH=256 cargo test  # Default production limit
    // MAX_CALL_DEPTH=0 cargo test    # Disable limit (not recommended)
    let max_call_depth = std::env::var("MAX_CALL_DEPTH")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(50);
    runtime.set_max_call_depth(max_call_depth);

    runtime
}

/// Helper function to evaluate TypeScript source code
/// Uses the full eval() API which properly handles async/await
/// Returns RuntimeValue which keeps the result guarded from GC
#[allow(clippy::expect_used)]
pub fn eval(source: &str) -> RuntimeValue {
    eval_result(source).expect("eval failed")
}

/// Helper function to evaluate and return Result for error testing
/// Uses the full eval() API which properly handles async/await
/// Returns RuntimeValue which keeps the result guarded from GC
pub fn eval_result(source: &str) -> Result<RuntimeValue, JsError> {
    let mut runtime = create_test_runtime();

    // Use the full eval() API instead of eval_simple()
    // This properly handles promise resolution via run_to_completion_or_suspend()
    let result = runtime.eval(source, None)?;

    // Handle the RuntimeResult
    match result {
        RuntimeResult::Complete(rv) => Ok(rv),
        RuntimeResult::NeedImports(specifiers) => Err(JsError::type_error(format!(
            "Missing imports in test: {:?}",
            specifiers
        ))),
        RuntimeResult::Suspended { pending, .. } => {
            // For tests without external dependencies, this shouldn't happen
            // If it does, treat as error
            Err(JsError::type_error(format!(
                "Test suspended waiting for {} orders",
                pending.len()
            )))
        }
    }
}

/// Helper to check if evaluation throws an error containing a specific message
pub fn throws_error(source: &str, error_contains: &str) -> bool {
    match eval_result(source) {
        Err(e) => format!("{:?}", e).contains(error_contains),
        Ok(_) => false,
    }
}
