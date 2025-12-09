//! TypeScript interpreter for config/manifest generation
//!
//! # Example
//!
//! ```
//! use typescript_eval::{Runtime, JsValue};
//!
//! let mut runtime = Runtime::new();
//! let result = runtime.eval_simple("1 + 2 * 3").unwrap();
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
pub use interpreter::SavedExecutionState;
pub use value::CheapClone;
pub use value::EnvId;
pub use value::EnvironmentArena;
pub use value::JsString;
pub use value::JsValue;

use std::cell::RefCell;
use std::rc::Rc;

/// Result of evaluating TypeScript code with suspension support
#[derive(Debug)]
pub enum RuntimeResult {
    /// Execution completed with a final value
    Complete(JsValue),

    /// Execution suspended waiting for a module to be loaded
    ImportAwaited {
        /// Slot to fill with the loaded module
        slot: PendingSlot,
        /// Module specifier (e.g., "./utils" or "lodash")
        specifier: String,
    },

    /// Execution suspended waiting for a promise to resolve
    AsyncAwaited {
        /// Slot to fill with the resolved value
        slot: PendingSlot,
        /// The promise being awaited (for debugging/inspection)
        promise: JsValue,
    },
}

/// A slot that can be filled with a value or error
///
/// IMPORTANT: Values assigned to slots MUST be created via Runtime methods
/// (create_module_from_source, create_value_from_json, etc.) to ensure
/// proper prototype chains and internal state.
#[derive(Debug, Clone)]
pub struct PendingSlot {
    id: u64,
    value: Rc<RefCell<Option<Result<JsValue, JsError>>>>,
}

// PendingSlot is cheap to clone - just u64 + Rc increment
impl CheapClone for PendingSlot {}

impl PendingSlot {
    /// Create a new pending slot
    pub fn new(id: u64) -> Self {
        PendingSlot {
            id,
            value: Rc::new(RefCell::new(None)),
        }
    }

    /// Fill the slot with a successful value
    pub fn set_success(&self, value: JsValue) {
        *self.value.borrow_mut() = Some(Ok(value));
    }

    /// Fill the slot with an error (will be thrown at resume point)
    pub fn set_error(&self, error: JsError) {
        *self.value.borrow_mut() = Some(Err(error));
    }

    /// Check if the slot has been filled
    pub fn is_filled(&self) -> bool {
        self.value.borrow().is_some()
    }

    /// Take the value out of the slot (used internally)
    pub(crate) fn take(&self) -> Option<Result<JsValue, JsError>> {
        self.value.borrow_mut().take()
    }

    /// Get the slot's unique ID
    pub fn id(&self) -> u64 {
        self.id
    }
}

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

    /// Evaluate TypeScript source code with suspension support
    ///
    /// Returns `RuntimeResult::Complete` if execution finishes,
    /// or `RuntimeResult::ImportAwaited`/`AsyncAwaited` if suspended.
    ///
    /// # Example
    /// ```rust,ignore
    /// let mut runtime = Runtime::new();
    /// let mut result = runtime.eval(source)?;
    ///
    /// loop {
    ///     match result {
    ///         RuntimeResult::Complete(value) => {
    ///             println!("Result: {:?}", value);
    ///             break;
    ///         }
    ///         RuntimeResult::ImportAwaited { slot, specifier } => {
    ///             let module = runtime.create_module_from_source(&load_source(&specifier)?)?;
    ///             slot.set_success(module);
    ///         }
    ///         RuntimeResult::AsyncAwaited { slot, .. } => {
    ///             let value = resolve_async()?;
    ///             slot.set_success(value);
    ///         }
    ///     }
    ///     result = runtime.continue_eval()?;
    /// }
    /// ```
    pub fn eval(&mut self, source: &str) -> Result<RuntimeResult, JsError> {
        let mut parser = parser::Parser::new(source);
        let program = parser.parse_program()?;
        self.interpreter.execute(&program)
    }

    /// Continue execution after filling a pending slot
    ///
    /// Call this after receiving `ImportAwaited` or `AsyncAwaited` and
    /// filling the slot with `set_success()` or `set_error()`.
    pub fn continue_eval(&mut self) -> Result<RuntimeResult, JsError> {
        self.interpreter.continue_execution()
    }

    /// Evaluate TypeScript source code, expecting immediate completion.
    ///
    /// This is a convenience method for code that doesn't use imports or async.
    /// Returns an error if execution suspends (ImportAwaited/AsyncAwaited).
    pub fn eval_simple(&mut self, source: &str) -> Result<JsValue, JsError> {
        match self.eval(source)? {
            RuntimeResult::Complete(value) => Ok(value),
            RuntimeResult::ImportAwaited { specifier, .. } => Err(JsError::type_error(format!(
                "Execution suspended for import '{}' - use eval() for code with imports",
                specifier
            ))),
            RuntimeResult::AsyncAwaited { .. } => Err(JsError::type_error(
                "Execution suspended for async - use eval() for async code",
            )),
        }
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
    /// runtime.eval_simple("export function add(a: number, b: number): number { return a + b; }").unwrap();
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
            .ok_or_else(|| JsError::reference_error(format!("{} is not exported", name)))?;

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
            .call_function(func, JsValue::Undefined, &js_args)
    }

    /// Get a reference to all exported values
    pub fn get_exports(&self) -> &std::collections::HashMap<JsString, JsValue> {
        &self.interpreter.exports
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Value Creation Methods (for slot filling)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Create a module object from a list of exports
    ///
    /// The module object will have the given exports as properties.
    /// Use this when providing a module to fill an ImportAwaited slot.
    pub fn create_module_object(&mut self, exports: Vec<(String, JsValue)>) -> JsValue {
        self.interpreter.create_module_object(exports)
    }

    /// Create a JsValue from a JSON value
    pub fn create_value_from_json(&mut self, json: &serde_json::Value) -> Result<JsValue, JsError> {
        interpreter::builtins::json_to_js_value(json)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Nested Module Loading Support
    // ═══════════════════════════════════════════════════════════════════════════

    /// Save the current execution state for nested module loading
    ///
    /// Call this before executing a nested module with `eval()`, then restore
    /// after the module finishes. This allows loading modules during execution
    /// without losing the parent module's state.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // When an import is requested during execution:
    /// let state = runtime.save_execution_state();
    /// let module = load_nested_module(&mut runtime)?;
    /// runtime.restore_execution_state(state);
    /// slot.set_success(module);
    /// runtime.continue_eval()?;
    /// ```
    pub fn save_execution_state(&mut self) -> SavedExecutionState {
        self.interpreter.save_execution_state()
    }

    /// Restore a previously saved execution state
    ///
    /// Call this after a nested module has finished executing to restore
    /// the parent module's state so execution can continue.
    pub fn restore_execution_state(&mut self, state: SavedExecutionState) {
        self.interpreter.restore_execution_state(state);
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    /// Set the execution timeout in milliseconds
    ///
    /// Default is 3000ms (3 seconds). Set to 0 to disable timeout.
    ///
    /// # Example
    ///
    /// ```
    /// use typescript_eval::Runtime;
    ///
    /// let mut runtime = Runtime::new();
    /// runtime.set_timeout_ms(5000); // 5 second timeout
    /// ```
    pub fn set_timeout_ms(&mut self, timeout_ms: u64) {
        self.interpreter.set_timeout_ms(timeout_ms);
    }

    /// Get the current execution timeout in milliseconds
    pub fn timeout_ms(&self) -> u64 {
        self.interpreter.timeout_ms()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        let mut runtime = Runtime::new();
        let result = runtime.eval_simple("1 + 2 * 3").unwrap();
        assert_eq!(result, JsValue::Number(7.0));
    }

    #[test]
    fn test_eval_simple() {
        let mut runtime = Runtime::new();
        let result = runtime.eval_simple("1 + 2").unwrap();
        assert_eq!(result, JsValue::Number(3.0));
    }

    #[test]
    fn test_eval_simple_with_import_returns_error() {
        let mut runtime = Runtime::new();
        let result = runtime.eval_simple("import { x } from './mod'; x");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(format!("{:?}", err).contains("suspended for import"));
    }
}
