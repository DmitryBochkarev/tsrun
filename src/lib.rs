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
pub use gc::{Gc, GcStats, Guard, Heap, Reset};
pub use interpreter::Interpreter;
pub use string_dict::StringDict;
pub use value::CheapClone;
pub use value::EnvRef;
pub use value::Guarded;
pub use value::JsObject;
pub use value::JsString;
pub use value::JsValue;

// Re-export order system types
// Note: Order, OrderId, OrderResponse, RuntimeResult are defined in this module

use std::cell::RefCell;
use std::rc::Rc;

// ═══════════════════════════════════════════════════════════════════════════════
// Order System Types
// ═══════════════════════════════════════════════════════════════════════════════

/// Unique identifier for an order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderId(pub u64);

/// An order is a request for an external effect.
/// The payload is a JsValue that the host interprets to perform side effects.
#[derive(Debug)]
pub struct Order {
    /// Unique identifier for this order
    pub id: OrderId,
    /// The JS value describing what operation to perform
    pub payload: JsValue,
}

/// Response to fulfill an order from the host
pub struct OrderResponse {
    /// The order ID this response is for
    pub id: OrderId,
    /// The result of the operation (success or error)
    pub result: Result<JsValue, JsError>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Runtime Result
// ═══════════════════════════════════════════════════════════════════════════════

/// Result of running the interpreter
#[derive(Debug)]
pub enum RuntimeResult {
    /// Execution completed with a final value
    Complete(JsValue),

    /// Need these modules before execution can start.
    /// Only includes non-internal modules (internal ones resolve automatically).
    NeedImports(Vec<String>),

    /// Execution suspended waiting for orders to be fulfilled
    Suspended {
        /// Orders waiting for fulfillment
        pending: Vec<Order>,
        /// Orders that were cancelled (e.g., Promise.race loser)
        cancelled: Vec<OrderId>,
    },
}

// ═══════════════════════════════════════════════════════════════════════════════
// Internal Module System
// ═══════════════════════════════════════════════════════════════════════════════

/// A native function that can be exported from an internal module
pub type InternalFn = fn(&mut Interpreter, JsValue, &[JsValue]) -> Result<Guarded, JsError>;

/// Definition of an export from an internal module
#[derive(Clone)]
pub enum InternalExport {
    /// A native function
    Function {
        name: String,
        func: InternalFn,
        arity: usize,
    },
    /// A constant value
    Value(JsValue),
}

/// How an internal module is defined
#[derive(Clone)]
pub enum InternalModuleKind {
    /// Native module with Rust functions
    Native(Vec<(String, InternalExport)>),
    /// Source module (TypeScript code that may import from other internal modules)
    Source(String),
}

/// Definition of an internal module
pub struct InternalModule {
    /// The import specifier (e.g., "eval:internal", "eval:fs")
    pub specifier: String,
    /// How the module is implemented
    pub kind: InternalModuleKind,
}

impl InternalModule {
    /// Create a native module builder
    pub fn native(specifier: impl Into<String>) -> NativeModuleBuilder {
        NativeModuleBuilder {
            specifier: specifier.into(),
            exports: Vec::new(),
        }
    }

    /// Create a source module
    pub fn source(specifier: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            specifier: specifier.into(),
            kind: InternalModuleKind::Source(source.into()),
        }
    }
}

/// Builder for creating native internal modules
pub struct NativeModuleBuilder {
    specifier: String,
    exports: Vec<(String, InternalExport)>,
}

impl NativeModuleBuilder {
    /// Add a function export
    pub fn with_function(
        mut self,
        name: impl Into<String>,
        func: InternalFn,
        arity: usize,
    ) -> Self {
        let name = name.into();
        self.exports
            .push((name.clone(), InternalExport::Function { name, func, arity }));
        self
    }

    /// Add a value export
    pub fn with_value(mut self, name: impl Into<String>, value: JsValue) -> Self {
        self.exports
            .push((name.into(), InternalExport::Value(value)));
        self
    }

    /// Build the internal module
    pub fn build(self) -> InternalModule {
        InternalModule {
            specifier: self.specifier,
            kind: InternalModuleKind::Native(self.exports),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Runtime Configuration
// ═══════════════════════════════════════════════════════════════════════════════

/// Configuration for creating a Runtime
#[derive(Default)]
pub struct RuntimeConfig {
    /// Internal modules available for import
    pub internal_modules: Vec<InternalModule>,
    /// Timeout in milliseconds (0 = no timeout)
    pub timeout_ms: u64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Legacy Pending Slot (internal use)
// ═══════════════════════════════════════════════════════════════════════════════

/// A slot that can be filled with a value or error (internal use)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct PendingSlot {
    id: u64,
    value: Rc<RefCell<Option<Result<JsValue, JsError>>>>,
}

// PendingSlot is cheap to clone - just u64 + Rc increment
impl CheapClone for PendingSlot {}

#[allow(dead_code)]
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

    /// Create a runtime with configuration
    pub fn with_config(config: RuntimeConfig) -> Self {
        let mut runtime = Self::new();
        for module in config.internal_modules {
            runtime.register_internal_module(module);
        }
        if config.timeout_ms > 0 {
            runtime.interpreter.set_timeout_ms(config.timeout_ms);
        }
        runtime
    }

    /// Register an internal module for import
    pub fn register_internal_module(&mut self, module: InternalModule) {
        self.interpreter.register_internal_module(module);
    }

    /// Evaluate simple TypeScript/JavaScript code (no imports, no async)
    pub fn eval_simple(&mut self, source: &str) -> Result<JsValue, JsError> {
        self.interpreter.eval_simple(source)
    }

    /// Evaluate TypeScript/JavaScript code with full runtime support
    ///
    /// Returns RuntimeResult which may indicate:
    /// - Complete: execution finished with a value
    /// - NeedImports: modules need to be provided before continuing
    /// - Suspended: waiting for orders to be fulfilled
    pub fn eval(&mut self, source: &str) -> Result<RuntimeResult, JsError> {
        self.interpreter.eval(source)
    }

    /// Provide a module source for a pending import
    pub fn provide_module(&mut self, specifier: &str, source: &str) -> Result<(), JsError> {
        self.interpreter.provide_module(specifier, source)
    }

    /// Continue evaluation after providing modules or fulfilling orders
    pub fn continue_eval(&mut self) -> Result<RuntimeResult, JsError> {
        self.interpreter.continue_eval()
    }

    /// Fulfill orders with responses from the host
    pub fn fulfill_orders(
        &mut self,
        responses: Vec<OrderResponse>,
    ) -> Result<RuntimeResult, JsError> {
        self.interpreter.fulfill_orders(responses)?;
        self.continue_eval()
    }

    /// Set the GC threshold (0 = disable automatic collection)
    ///
    /// Lower values reduce peak memory but increase GC overhead.
    /// Higher values improve throughput but may use more memory.
    pub fn set_gc_threshold(&self, threshold: usize) {
        self.interpreter.heap.set_gc_threshold(threshold);
    }

    pub fn set_timeout_ms(&mut self, timeout_ms: u64) {
        self.interpreter.set_timeout_ms(timeout_ms);
    }

    /// Force a garbage collection cycle
    pub fn collect(&self) {
        self.interpreter.heap.collect();
    }

    /// Get GC statistics
    pub fn gc_stats(&self) -> gc::GcStats {
        self.interpreter.heap.stats()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
