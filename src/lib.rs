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
pub mod compiler;
pub mod error;
pub mod gc;
pub mod interpreter;
pub mod lexer;
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

// Re-export serde conversion functions for JsValue <-> serde_json::Value
pub use interpreter::builtins::json::{
    js_value_to_json, json_to_js_value_with_guard, json_to_js_value_with_interp,
};

// Re-export order system types
// Note: Order, OrderId, OrderResponse, RuntimeResult, ModulePath, ImportRequest are defined in this module

// ═══════════════════════════════════════════════════════════════════════════════
// Order System Types
// ═══════════════════════════════════════════════════════════════════════════════

/// Unique identifier for an order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderId(pub u64);

/// An order is a request for an external effect.
/// The payload is a RuntimeValue that the host interprets to perform side effects.
/// The RuntimeValue keeps the payload alive until the order is fulfilled or dropped.
#[derive(Debug)]
pub struct Order {
    /// Unique identifier for this order
    pub id: OrderId,
    /// The JS value describing what operation to perform.
    /// Wrapped in RuntimeValue to keep it alive until the order is processed.
    pub payload: RuntimeValue,
}

/// Response to fulfill an order from the host
pub struct OrderResponse {
    /// The order ID this response is for
    pub id: OrderId,
    /// The result of the operation (success or error).
    /// Use `RuntimeValue::unguarded()` for primitives or
    /// `Runtime::create_response_object()` for objects.
    pub result: Result<RuntimeValue, JsError>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Runtime Value
// ═══════════════════════════════════════════════════════════════════════════════

/// A JS value with an attached guard that keeps it alive until dropped.
///
/// This struct ensures that GC-managed objects remain valid for as long as
/// the `RuntimeValue` exists. The guard is private to prevent accidental
/// extraction of the value without the guard.
///
/// # Example
/// ```ignore
/// let result = runtime.eval("{ foo: 42 }")?;
/// match result {
///     RuntimeResult::Complete(runtime_value) => {
///         // Value is guaranteed alive while runtime_value exists
///         let value = runtime_value.value();
///         println!("{:?}", value);
///     }
///     _ => {}
/// }
/// // Guard dropped here, object may be collected
/// ```
pub struct RuntimeValue {
    value: JsValue,
    _guard: Option<Guard<JsObject>>,
}

impl RuntimeValue {
    /// Create a RuntimeValue from an internal Guarded value
    pub(crate) fn from_guarded(guarded: Guarded) -> Self {
        Self {
            value: guarded.value,
            _guard: guarded.guard,
        }
    }

    /// Create a RuntimeValue with an explicit guard
    pub(crate) fn with_guard(value: JsValue, guard: Guard<JsObject>) -> Self {
        Self {
            value,
            _guard: Some(guard),
        }
    }

    /// Create an unguarded RuntimeValue (for primitives).
    /// Use this for values that don't need GC protection (strings, numbers, booleans, null, undefined).
    pub fn unguarded(value: JsValue) -> Self {
        Self {
            value,
            _guard: None,
        }
    }

    /// Get a reference to the value
    pub fn value(&self) -> &JsValue {
        &self.value
    }

    // NOTE: Do NOT add `into_value(self) -> JsValue` or similar methods that
    // extract the value without the guard. The guard must stay alive as long
    // as the value is in use. If you need to pass the value somewhere, pass
    // the entire RuntimeValue and let the receiver access it via .value().
}

impl std::ops::Deref for RuntimeValue {
    type Target = JsValue;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl std::fmt::Debug for RuntimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeValue")
            .field("value", &self.value)
            .field("guarded", &self._guard.is_some())
            .finish()
    }
}

impl PartialEq<JsValue> for RuntimeValue {
    fn eq(&self, other: &JsValue) -> bool {
        &self.value == other
    }
}

impl PartialEq<RuntimeValue> for JsValue {
    fn eq(&self, other: &RuntimeValue) -> bool {
        self == &other.value
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Module Path System
// ═══════════════════════════════════════════════════════════════════════════════

/// A normalized, absolute module path.
///
/// Module paths are always stored in normalized form:
/// - No `.` or `..` segments
/// - Forward slashes only
/// - No trailing slashes
/// - Absolute (starts with `/` or is a bare specifier like `lodash`)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModulePath(String);

impl ModulePath {
    /// Create a ModulePath from an already-normalized absolute path.
    /// Use `resolve` for relative paths.
    pub fn new(path: impl Into<String>) -> Self {
        ModulePath(path.into())
    }

    /// Get the path as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the directory portion of this path (everything before the last `/`)
    pub fn parent(&self) -> Option<&str> {
        self.0.rfind('/').and_then(|idx| self.0.get(..idx))
    }

    /// Check if this is a relative specifier (starts with `.` or `..`)
    pub fn is_relative(specifier: &str) -> bool {
        specifier.starts_with("./") || specifier.starts_with("../")
    }

    /// Check if this is a bare specifier (not relative, not absolute)
    /// e.g., "lodash", "react", "eval:internal"
    pub fn is_bare(specifier: &str) -> bool {
        !specifier.starts_with('/') && !Self::is_relative(specifier)
    }

    /// Resolve a specifier relative to a base path.
    ///
    /// - Relative specifiers (`./foo`, `../bar`) are resolved against the base's directory
    /// - Absolute specifiers (`/foo/bar`) are normalized and returned as-is
    /// - Bare specifiers (`lodash`) are returned as-is (for the host to resolve)
    pub fn resolve(specifier: &str, base: Option<&ModulePath>) -> ModulePath {
        if Self::is_bare(specifier) {
            // Bare specifier - return as-is for host resolution
            return ModulePath(specifier.to_string());
        }

        if specifier.starts_with('/') {
            // Absolute path - just normalize
            return ModulePath(Self::normalize_path(specifier));
        }

        // Relative path - resolve against base
        let base_dir = base.and_then(|b| b.parent()).unwrap_or("");

        let combined = if base_dir.is_empty() {
            specifier.to_string()
        } else {
            format!("{}/{}", base_dir, specifier)
        };

        ModulePath(Self::normalize_path(&combined))
    }

    /// Normalize a path by resolving `.` and `..` segments
    fn normalize_path(path: &str) -> String {
        let mut segments: Vec<&str> = Vec::new();

        for segment in path.split('/') {
            match segment {
                "" | "." => {
                    // Skip empty segments and current directory markers
                }
                ".." => {
                    // Go up one directory
                    segments.pop();
                }
                s => {
                    segments.push(s);
                }
            }
        }

        // Reconstruct path
        if path.starts_with('/') {
            format!("/{}", segments.join("/"))
        } else {
            segments.join("/")
        }
    }
}

impl std::fmt::Display for ModulePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ModulePath {
    fn from(s: &str) -> Self {
        ModulePath::new(s)
    }
}

impl From<String> for ModulePath {
    fn from(s: String) -> Self {
        ModulePath::new(s)
    }
}

/// A pending import request with context about where it was requested from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRequest {
    /// The original specifier as written in the source code
    pub specifier: String,
    /// The resolved absolute path (for deduplication)
    pub resolved_path: ModulePath,
    /// The module that requested this import (None for main module)
    pub importer: Option<ModulePath>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Runtime Result
// ═══════════════════════════════════════════════════════════════════════════════

/// Result of running the interpreter
#[derive(Debug)]
pub enum RuntimeResult {
    /// Execution completed with a final value.
    /// The RuntimeValue keeps the result alive until dropped.
    Complete(RuntimeValue),

    /// Need these modules before execution can start.
    /// Contains import requests with resolved paths and importer context.
    NeedImports(Vec<ImportRequest>),

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

    /// Evaluate TypeScript/JavaScript code with full runtime support.
    ///
    /// This is equivalent to `eval_with_path(source, None)` - relative imports
    /// will be resolved without a base path (treated as bare specifiers).
    ///
    /// Returns RuntimeResult which may indicate:
    /// - Complete: execution finished with a value
    /// - NeedImports: modules need to be provided before continuing
    /// - Suspended: waiting for orders to be fulfilled
    pub fn eval(&mut self, source: &str) -> Result<RuntimeResult, JsError> {
        self.interpreter.eval(source)
    }

    /// Evaluate TypeScript/JavaScript code with a known module path.
    ///
    /// The `module_path` is used as the base for resolving relative imports.
    /// For example, if `module_path` is `/project/src/main.ts` and the code
    /// contains `import { foo } from "./utils"`, it will resolve to
    /// `/project/src/utils`.
    ///
    /// Returns RuntimeResult which may indicate:
    /// - Complete: execution finished with a value
    /// - NeedImports: modules need to be provided before continuing
    /// - Suspended: waiting for orders to be fulfilled
    pub fn eval_with_path(
        &mut self,
        source: &str,
        module_path: impl Into<ModulePath>,
    ) -> Result<RuntimeResult, JsError> {
        self.interpreter
            .eval_with_path(source, Some(module_path.into()))
    }

    /// Provide a module source for a pending import.
    ///
    /// The `resolved_path` should be the `ImportRequest.resolved_path` from
    /// the `NeedImports` result. This ensures proper deduplication of modules
    /// even when they are imported with different relative paths.
    pub fn provide_module(
        &mut self,
        resolved_path: impl Into<ModulePath>,
        source: &str,
    ) -> Result<(), JsError> {
        self.interpreter
            .provide_module(resolved_path.into(), source)
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
        self.interpreter.fulfill_orders(responses);
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

    /// Set the maximum call stack depth
    ///
    /// Default is 256. Set to 0 to disable limit (not recommended).
    /// Tests should use a lower value (e.g., 50) to catch infinite recursion early.
    pub fn set_max_call_depth(&mut self, depth: usize) {
        self.interpreter.set_max_call_depth(depth);
    }

    /// Force a garbage collection cycle
    pub fn collect(&self) {
        self.interpreter.heap.collect();
    }

    /// Get GC statistics
    pub fn gc_stats(&self) -> gc::GcStats {
        self.interpreter.heap.stats()
    }

    /// Create a RuntimeValue from a serde_json value for use in OrderResponse.
    ///
    /// This is the recommended way to create object responses for orders.
    /// The returned RuntimeValue keeps the object alive until it is consumed.
    ///
    /// # Example
    /// ```ignore
    /// use serde_json::json;
    ///
    /// let response_value = runtime.create_response_object(&json!({
    ///     "id": 1,
    ///     "name": "John",
    ///     "items": [1, 2, 3]
    /// }))?;
    ///
    /// let response = OrderResponse {
    ///     id: order.id,
    ///     result: Ok(response_value),
    /// };
    /// ```
    pub fn create_response_object(
        &mut self,
        json: &serde_json::Value,
    ) -> Result<RuntimeValue, JsError> {
        let guard = self.interpreter.heap.create_guard();
        let value = interpreter::builtins::json::json_to_js_value_with_guard(
            &mut self.interpreter,
            json,
            &guard,
        )?;
        Ok(RuntimeValue::with_guard(value, guard))
    }

    /// Create an unresolved Promise that can be resolved or rejected later.
    ///
    /// This is useful when the host wants to return a Promise from `fulfill_orders`
    /// that will be resolved asynchronously (e.g., when a network request completes).
    ///
    /// The returned `RuntimeValue` contains the Promise and keeps it alive.
    /// Store it and later call `resolve_promise` or `reject_promise` to settle it.
    ///
    /// # Example
    /// ```ignore
    /// // Create an unresolved promise
    /// let promise = runtime.create_promise();
    ///
    /// // Return it as the response to an order
    /// runtime.fulfill_orders(vec![OrderResponse {
    ///     id: order.id,
    ///     result: Ok(promise.clone()),
    /// }])?;
    ///
    /// // Later, when the async operation completes:
    /// runtime.resolve_promise(&promise, result_value)?;
    /// ```
    pub fn create_promise(&mut self) -> RuntimeValue {
        let guard = self.interpreter.heap.create_guard();
        let promise = interpreter::builtins::promise::create_promise(&mut self.interpreter, &guard);
        RuntimeValue::with_guard(JsValue::Object(promise), guard)
    }

    /// Create an unresolved Promise linked to an order for cancellation tracking.
    ///
    /// Similar to `create_promise()`, but the Promise is associated with the given
    /// order ID. When this Promise "loses" in a `Promise.race()`, the order ID will
    /// be included in the `cancelled` list of `RuntimeResult::Suspended`.
    ///
    /// Use this when returning a Promise as an order response to enable automatic
    /// cancellation notification when the Promise is no longer needed.
    ///
    /// # Example
    /// ```ignore
    /// let order_id = order.id;
    /// let promise = runtime.create_order_promise(order_id);
    ///
    /// runtime.fulfill_orders(vec![OrderResponse {
    ///     id: order_id,
    ///     result: Ok(promise.clone()),
    /// }])?;
    ///
    /// // If this Promise loses in a Promise.race(), order_id will be in
    /// // RuntimeResult::Suspended { cancelled: vec![order_id], ... }
    /// ```
    pub fn create_order_promise(&mut self, order_id: OrderId) -> RuntimeValue {
        let guard = self.interpreter.heap.create_guard();
        let promise = interpreter::builtins::promise::create_order_promise(
            &mut self.interpreter,
            &guard,
            order_id,
        );
        RuntimeValue::with_guard(JsValue::Object(promise), guard)
    }

    /// Resolve a Promise that was created with `create_promise`.
    ///
    /// This will fulfill the Promise with the given value and trigger any
    /// `.then()` handlers. Any async contexts waiting on this Promise will
    /// be moved to the ready queue.
    ///
    /// Returns `RuntimeResult` which may be:
    /// - `Complete`: all execution finished
    /// - `Suspended`: still waiting for other Promises or orders
    ///
    /// # Errors
    /// Returns an error if the value is not a Promise.
    pub fn resolve_promise(
        &mut self,
        promise: &RuntimeValue,
        value: RuntimeValue,
    ) -> Result<RuntimeResult, JsError> {
        let JsValue::Object(promise_obj) = promise.value() else {
            return Err(JsError::type_error("Expected a Promise object"));
        };

        interpreter::builtins::promise::resolve_promise_value(
            &mut self.interpreter,
            promise_obj,
            value.value().clone(),
        )?;

        // Check if any waiting contexts are now ready
        self.interpreter.check_resolved_promises_public();

        self.continue_eval()
    }

    /// Reject a Promise that was created with `create_promise`.
    ///
    /// This will reject the Promise with the given reason and trigger any
    /// `.catch()` or rejection handlers. Any async contexts waiting on this
    /// Promise will be moved to the ready queue with an error.
    ///
    /// Returns `RuntimeResult` which may be:
    /// - `Complete`: all execution finished
    /// - `Suspended`: still waiting for other Promises or orders
    ///
    /// # Errors
    /// Returns an error if the value is not a Promise.
    pub fn reject_promise(
        &mut self,
        promise: &RuntimeValue,
        reason: RuntimeValue,
    ) -> Result<RuntimeResult, JsError> {
        let JsValue::Object(promise_obj) = promise.value() else {
            return Err(JsError::type_error("Expected a Promise object"));
        };

        interpreter::builtins::promise::reject_promise_value(
            &mut self.interpreter,
            promise_obj,
            reason.value().clone(),
        )?;

        // Check if any waiting contexts are now ready
        self.interpreter.check_resolved_promises_public();

        self.continue_eval()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
