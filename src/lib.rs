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
pub mod gc;
pub mod interpreter;
pub mod lexer;
pub mod old_gc;
pub mod parser;
pub mod string_dict;
pub mod value;

pub use error::JsError;
pub use gc::{Gc, Guard, Heap, Reset};
pub use interpreter::GcStats;
pub use interpreter::Interpreter;
pub use string_dict::StringDict;
pub use value::CheapClone;
pub use value::EnvRef;
pub use value::Guarded;
pub use value::JsObject;
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

    /// Evaluate simple TypeScript/JavaScript code
    pub fn eval_simple(&mut self, source: &str) -> Result<JsValue, JsError> {
        self.interpreter.eval_simple(source)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
