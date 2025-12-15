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

// TODO: Re-enable these test modules once the new interpreter supports more features
mod array;
// mod async_await;
mod basics;
mod class;
// mod console;
mod control_flow;
// mod date;
// mod dynamic_import;
// mod error;
// mod exports;
mod function;
// mod gc;
// mod generator;
// mod global;
// mod json;
// mod map;
// mod math;
// mod namespace;
// mod number;
mod object;
// mod promise;
mod regexp;
// mod set;
// mod state_machine;
mod string;
// mod symbol;
// mod timeout;

use typescript_eval::{JsError, JsValue, Runtime};

/// Create a new runtime
fn create_test_runtime() -> Runtime {
    Runtime::new()
}

/// Helper function to evaluate TypeScript source code
pub fn eval(source: &str) -> JsValue {
    let mut runtime = create_test_runtime();
    runtime.eval_simple(source).expect("eval failed")
}

/// Helper function to evaluate and return Result for error testing
pub fn eval_result(source: &str) -> Result<JsValue, JsError> {
    let mut runtime = create_test_runtime();
    runtime.eval_simple(source)
}

/// Helper to check if evaluation throws an error containing a specific message
pub fn throws_error(source: &str, error_contains: &str) -> bool {
    match eval_result(source) {
        Err(e) => format!("{:?}", e).contains(error_contains),
        Ok(_) => false,
    }
}
