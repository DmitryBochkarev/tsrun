//! TypeScript interpreter for config/manifest generation
//!
//! # Example
//!
//! ```
//! use typescript_eval::{Runtime, JsValue};
//!
//! let mut runtime = Runtime::new();
//! let result = runtime.eval("1 + 2 * 3").unwrap();
//! assert_eq!(result, JsValue::Number(7.0));
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

    /// Call an exported function by name with the given arguments
    ///
    /// If `args` is a JSON array, the elements are spread as individual arguments.
    /// Otherwise, `args` is passed as a single argument.
    ///
    /// # Example
    ///
    /// ```
    /// use typescript_eval::{Runtime, JsValue};
    /// use serde_json::json;
    ///
    /// let mut runtime = Runtime::new();
    /// runtime.eval("export function add(a: number, b: number): number { return a + b; }").unwrap();
    /// let result = runtime.call_function("add", &json!([1, 2])).unwrap();
    /// assert_eq!(result, JsValue::Number(3.0));
    /// ```
    pub fn call_function(
        &mut self,
        name: &str,
        args: &serde_json::Value,
    ) -> Result<JsValue, JsError> {
        // Look up the function in exports
        let func = self
            .interpreter
            .exports
            .get(name)
            .cloned()
            .ok_or_else(|| JsError::reference_error(&format!("{} is not exported", name)))?;

        // Convert JSON args to JsValue
        let js_args = if let serde_json::Value::Array(arr) = args {
            // Spread array elements as individual arguments
            arr.iter()
                .map(interpreter::builtins::json_to_js_value)
                .collect::<Result<Vec<_>, _>>()?
        } else {
            // Single argument
            vec![interpreter::builtins::json_to_js_value(args)?]
        };

        // Call the function
        self.interpreter
            .call_function(func, JsValue::Undefined, js_args)
    }

    /// Get a reference to all exported values
    pub fn get_exports(&self) -> &std::collections::HashMap<String, JsValue> {
        &self.interpreter.exports
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
