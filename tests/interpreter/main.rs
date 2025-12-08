//! Integration tests for the interpreter, organized by feature
//!
//! These tests exercise the interpreter through the public API.

mod array;
mod basics;
mod class;
mod control_flow;
mod console;
mod date;
mod error;
mod exports;
mod function;
mod generator;
mod global;
mod map;
mod math;
mod number;
mod object;
mod regexp;
mod set;
mod string;
mod symbol;

use typescript_eval::parser::Parser;
use typescript_eval::{Interpreter, JsError, JsValue};

/// Helper function to evaluate TypeScript source code
pub fn eval(source: &str) -> JsValue {
    let mut parser = Parser::new(source);
    let program = parser.parse_program().unwrap();
    let mut interp = Interpreter::new();
    interp.execute(&program).unwrap()
}

/// Helper function to evaluate and return Result for error testing
pub fn eval_result(source: &str) -> Result<JsValue, JsError> {
    let mut parser = Parser::new(source);
    let program = parser.parse_program().unwrap();
    let mut interp = Interpreter::new();
    interp.execute(&program)
}

/// Helper to check if evaluation throws an error containing a specific message
pub fn throws_error(source: &str, error_contains: &str) -> bool {
    match eval_result(source) {
        Err(e) => format!("{:?}", e).contains(error_contains),
        Ok(_) => false,
    }
}