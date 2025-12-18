//! Integration tests for the interpreter, organized by feature
//!
//! These tests exercise the interpreter through the public API.
//!
//! ## GC Stress Testing
//!
//! Tests default to `GC_THRESHOLD=1` (GC on every allocation) to catch GC bugs early.
//! Set the `GC_THRESHOLD` environment variable to override:
//!
//! ```bash
//! cargo test                           # Default: GC_THRESHOLD=1 (most aggressive)
//! GC_THRESHOLD=100 cargo test          # Less aggressive for faster runs
//! GC_THRESHOLD=0 cargo test            # Disable automatic GC
//! ```

mod array;
mod async_await;
mod basics;
mod class;
mod console;
mod control_flow;
mod cycle_leak;
mod date;
mod decorator;
mod enum_test;
mod error;
mod function;
mod gc;
mod generator;
mod global;
mod json;
mod map;
mod math;
mod modules;
mod number;
mod object;
mod promise;
mod regexp;
mod set;
mod stack;
mod string;
mod symbol;

use typescript_eval::{JsError, Runtime, RuntimeResult, RuntimeValue};

/// Create a new runtime with GC threshold from environment or default (1)
fn create_test_runtime() -> Runtime {
    let runtime = Runtime::new();

    // Default to GC_THRESHOLD=1 (most aggressive) to catch GC bugs early
    // Override via environment variable if needed:
    // GC_THRESHOLD=100 cargo test  # Faster runs
    // GC_THRESHOLD=0 cargo test    # Disable automatic GC
    let threshold = std::env::var("GC_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1); // Default to 1 for tests
    runtime.set_gc_threshold(threshold);

    runtime
}

/// Helper function to evaluate TypeScript source code
/// Uses the full eval() API which properly handles async/await
/// Returns RuntimeValue which keeps the result guarded from GC
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
    let result = runtime.eval(source)?;

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
