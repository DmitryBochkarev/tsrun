//! Integration tests for the interpreter, organized by feature
//!
//! These tests exercise the interpreter through the public API.
//!
//! ## Aggressive Test Defaults
//!
//! Tests use aggressive defaults to catch bugs early:
//! - `GC_THRESHOLD=1` - GC on every allocation to catch GC bugs
//!
//! Override via environment variables:
//!
//! ```bash
//! cargo test                           # Default: aggressive settings
//! GC_THRESHOLD=100 cargo test          # Less aggressive GC for faster runs
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
mod step;
mod strict;
mod string;
mod symbol;
mod typescript;

use tsrun::{Interpreter, JsError, JsValue, RuntimeValue, StepResult};

/// Create a new interpreter with aggressive defaults for testing:
/// - GC_THRESHOLD=1 (GC on every allocation) to catch GC bugs
pub fn create_test_runtime() -> Interpreter {
    let interp = Interpreter::new();

    // Default to GC_THRESHOLD=1 (most aggressive) to catch GC bugs early
    // Override via environment variable if needed:
    // GC_THRESHOLD=100 cargo test  # Faster runs
    // GC_THRESHOLD=0 cargo test    # Disable automatic GC
    let gc_threshold = std::env::var("GC_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    interp.set_gc_threshold(gc_threshold);

    interp
}

/// Run an interpreter to completion using the step-based API.
/// Returns the final StepResult (Complete, NeedImports, or Suspended).
pub fn run_to_completion(interp: &mut Interpreter) -> Result<StepResult, JsError> {
    loop {
        match interp.step()? {
            StepResult::Continue => continue,
            result => return Ok(result),
        }
    }
}

/// Run code in an interpreter using prepare() + step loop.
/// This is a test helper equivalent to the old runtime.run() method.
pub fn run(
    interp: &mut Interpreter,
    source: &str,
    path: Option<&str>,
) -> Result<StepResult, JsError> {
    interp.prepare(source, path.map(tsrun::ModulePath::new))?;
    run_to_completion(interp)
}

/// Helper function to evaluate TypeScript source code.
/// Uses the step-based API which properly handles async/await.
/// Returns RuntimeValue which keeps the result guarded from GC.
#[allow(clippy::expect_used)]
pub fn eval(source: &str) -> RuntimeValue {
    eval_result(source).expect("eval failed")
}

/// Helper function to evaluate and return Result for error testing.
/// Uses the step-based API which properly handles async/await.
/// Returns RuntimeValue which keeps the result guarded from GC.
pub fn eval_result(source: &str) -> Result<RuntimeValue, JsError> {
    let mut interp = create_test_runtime();

    // Prepare the source
    interp.prepare(source, None)?;

    // Run to completion using step-based API
    match run_to_completion(&mut interp)? {
        StepResult::Complete(rv) => Ok(rv),
        StepResult::NeedImports(specifiers) => Err(JsError::type_error(format!(
            "Missing imports in test: {:?}",
            specifiers
        ))),
        StepResult::Suspended { pending, .. } => {
            // For tests without external dependencies, this shouldn't happen
            // If it does, treat as error
            Err(JsError::type_error(format!(
                "Test suspended waiting for {} orders",
                pending.len()
            )))
        }
        StepResult::Continue => Err(JsError::internal_error(
            "Unexpected Continue from run_to_completion",
        )),
        StepResult::Done => Ok(RuntimeValue::unguarded(JsValue::Undefined)),
    }
}

/// Helper to check if evaluation throws an error containing a specific message
pub fn throws_error(source: &str, error_contains: &str) -> bool {
    match eval_result(source) {
        Err(e) => format!("{:?}", e).contains(error_contains),
        Ok(_) => false,
    }
}
