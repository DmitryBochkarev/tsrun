//! TypeScript interpreter for config/manifest generation
//!
//! # Example
//!
//! ```ignore
//! use typescript_eval::Runtime;
//! use serde_json::json;
//!
//! let mut runtime = Runtime::new();
//! runtime.load_module("config.ts")?;
//! let result: serde_json::Value = runtime.call_function("generateConfig", &json!({"env": "prod"}))?;
//! ```

pub mod ast;
pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod value;

pub use error::JsError;
pub use interpreter::Interpreter;
pub use value::JsValue;

/// The main runtime for executing TypeScript code
pub struct Runtime {
    interpreter: Interpreter,
}

impl Runtime {
    /// Create a new runtime instance
    pub fn new() -> Self {
        Self {
            interpreter: Interpreter::new(),
        }
    }

    /// Evaluate TypeScript source code and return the result
    pub fn eval(&mut self, source: &str) -> Result<JsValue, JsError> {
        let mut parser = parser::Parser::new(source);
        let program = parser.parse_program()?;
        self.interpreter.execute(&program)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        let mut runtime = Runtime::new();
        let result = runtime.eval("1 + 2 * 3").unwrap();
        assert_eq!(result, JsValue::Number(7.0));
    }
}
