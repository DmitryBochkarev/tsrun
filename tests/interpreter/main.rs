//! Integration tests for the interpreter, organized by feature
//!
//! These tests exercise the interpreter through the public API.

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

use typescript_eval::{JsError, JsValue, Runtime, RuntimeResult};

/// Helper function to evaluate TypeScript source code
pub fn eval(source: &str) -> JsValue {
    let mut runtime = Runtime::new();
    match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    }
}

/// Helper function to evaluate and return Result for error testing
pub fn eval_result(source: &str) -> Result<JsValue, JsError> {
    let mut runtime = Runtime::new();
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
