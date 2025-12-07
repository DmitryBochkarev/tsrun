//! Integration tests for the interpreter, organized by feature
//!
//! These tests exercise the interpreter through the public API.

mod array;
mod basics;
mod class;
mod console;
mod date;
mod error;
mod function;
mod global;
mod map;
mod math;
mod number;
mod object;
mod regexp;
mod set;
mod string;

use typescript_eval::parser::Parser;
use typescript_eval::{Interpreter, JsValue};

/// Helper function to evaluate TypeScript source code
pub fn eval(source: &str) -> JsValue {
    let mut parser = Parser::new(source);
    let program = parser.parse_program().unwrap();
    let mut interp = Interpreter::new();
    interp.execute(&program).unwrap()
}