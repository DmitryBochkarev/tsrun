//! TypeScript interpreter for config/manifest generation
//!
//! # Example
//!
//! ```
//! use tsrun::{Runtime, StepResult};
//!
//! let mut runtime = Runtime::new();
//! runtime.prepare("1 + 2 * 3", None).unwrap();
//!
//! // Step until completion
//! loop {
//!     match runtime.step().unwrap() {
//!         StepResult::Continue => continue,
//!         StepResult::Complete(value) => {
//!             assert_eq!(value.as_number(), Some(7.0));
//!             break;
//!         }
//!         _ => panic!("Unexpected result"),
//!     }
//! }
//! ```

pub mod api;
pub mod ast;
pub mod compiler;
pub mod error;
pub mod gc;
pub(crate) mod interpreter;
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

// Re-export internal module builder for the order system
pub use interpreter::builtins::internal::create_eval_internal_module;

// Re-export order system types
// Note: Order, OrderId, OrderResponse, ModulePath, ImportRequest, StepResult are defined in this module

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
/// let result = runtime.run("{ foo: 42 }", None)?;
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

    // ═══════════════════════════════════════════════════════════════════════════════
    // Type Check Delegation Methods
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Check if this is undefined
    pub fn is_undefined(&self) -> bool {
        self.value.is_undefined()
    }

    /// Check if this is null
    pub fn is_null(&self) -> bool {
        self.value.is_null()
    }

    /// Check if this is null or undefined
    pub fn is_nullish(&self) -> bool {
        self.value.is_nullish()
    }

    /// Check if this is a boolean
    pub fn is_boolean(&self) -> bool {
        self.value.is_boolean()
    }

    /// Check if this is a number
    pub fn is_number(&self) -> bool {
        self.value.is_number()
    }

    /// Check if this is a string
    pub fn is_string(&self) -> bool {
        self.value.is_string()
    }

    /// Check if this is an object
    pub fn is_object(&self) -> bool {
        self.value.is_object()
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Value Extraction Delegation Methods
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Returns the boolean value if this is a Boolean, otherwise None
    pub fn as_bool(&self) -> Option<bool> {
        self.value.as_bool()
    }

    /// Returns the numeric value if this is a Number, otherwise None
    pub fn as_number(&self) -> Option<f64> {
        self.value.as_number()
    }

    /// Returns the string slice if this is a String, otherwise None
    pub fn as_str(&self) -> Option<&str> {
        self.value.as_str()
    }

    /// Returns a string describing the type of this value
    pub fn type_name(&self) -> &'static str {
        self.value.type_name()
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Array Inspection Methods (primitives only - complex values go through Runtime)
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Get the length of an array.
    ///
    /// Returns `None` if this is not an array.
    ///
    /// # Example
    /// ```ignore
    /// let arr = runtime.create_from_json(&json!([1, 2, 3, 4, 5]))?;
    /// assert_eq!(arr.len(), Some(5));
    /// ```
    pub fn len(&self) -> Option<usize> {
        let obj = self.value.as_object()?;
        let borrowed = obj.borrow();
        borrowed.array_length().map(|l| l as usize)
    }

    /// Check if the array is empty.
    ///
    /// Returns `None` if this is not an array.
    pub fn is_empty(&self) -> Option<bool> {
        self.len().map(|l| l == 0)
    }

    /// Check if this value is an array.
    ///
    /// # Example
    /// ```ignore
    /// let arr = runtime.create_from_json(&json!([1, 2, 3]))?;
    /// let obj = runtime.create_from_json(&json!({"x": 1}))?;
    /// assert!(arr.is_array());
    /// assert!(!obj.is_array());
    /// ```
    pub fn is_array(&self) -> bool {
        if let Some(obj) = self.value.as_object() {
            let borrowed = obj.borrow();
            borrowed.array_length().is_some()
        } else {
            false
        }
    }

    /// Get all property keys of an object.
    ///
    /// Returns an empty vector if this is not an object.
    ///
    /// # Example
    /// ```ignore
    /// let obj = runtime.create_from_json(&json!({"a": 1, "b": 2}))?;
    /// let keys = obj.keys();
    /// assert!(keys.contains(&"a".to_string()));
    /// assert!(keys.contains(&"b".to_string()));
    /// ```
    pub fn keys(&self) -> Vec<String> {
        if let Some(obj) = self.value.as_object() {
            let borrowed = obj.borrow();
            borrowed
                .properties
                .keys()
                .filter_map(|k| match k {
                    value::PropertyKey::String(s) => Some(s.to_string()),
                    value::PropertyKey::Index(i) => Some(i.to_string()),
                    value::PropertyKey::Symbol(_) => None,
                })
                .collect()
        } else {
            Vec::new()
        }
    }
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

impl std::fmt::Display for RuntimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.value, f)
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
// Step Result
// ═══════════════════════════════════════════════════════════════════════════════

/// Result of executing a single step.
///
/// Step-based execution gives the host full control over when execution should stop.
/// The host calls `step()` repeatedly until it receives a terminal result
/// (`Complete`, `NeedImports`, `Suspended`), or decides to stop early.
#[derive(Debug)]
pub enum StepResult {
    /// Executed one instruction, more to execute.
    /// Call `step()` again to continue.
    Continue,

    /// Execution completed with a final value.
    Complete(RuntimeValue),

    /// Need these modules before execution can continue.
    /// Call `provide_module()` for each import, then call `step()` again.
    NeedImports(Vec<ImportRequest>),

    /// Execution suspended waiting for orders to be fulfilled.
    /// Call `fulfill_orders()` with responses, then call `step()` again.
    Suspended {
        /// Orders waiting for fulfillment
        pending: Vec<Order>,
        /// Orders that were cancelled (e.g., Promise.race loser)
        cancelled: Vec<OrderId>,
    },

    /// No active execution to step.
    /// Call `prepare()` first to start execution.
    Done,
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
        runtime
    }

    /// Register an internal module for import
    pub fn register_internal_module(&mut self, module: InternalModule) {
        self.interpreter.register_internal_module(module);
    }

    /// Provide a module source for a pending import.
    ///
    /// The `resolved_path` should be the `ImportRequest.resolved_path` from
    /// the `NeedImports` result. This ensures proper deduplication of modules
    /// even when they are imported with different relative paths.
    ///
    /// After providing all required modules, call `step()` to continue execution.
    pub fn provide_module(
        &mut self,
        resolved_path: impl Into<ModulePath>,
        source: &str,
    ) -> Result<(), JsError> {
        self.interpreter
            .provide_module(resolved_path.into(), source)
    }

    /// Fulfill orders with responses from the host.
    ///
    /// Stores the responses for when execution resumes.
    /// After calling this, call `step()` to continue execution.
    pub fn fulfill_orders(&mut self, responses: Vec<OrderResponse>) {
        self.interpreter.fulfill_orders(responses);
    }

    /// Set the GC threshold (0 = disable automatic collection)
    ///
    /// Lower values reduce peak memory but increase GC overhead.
    /// Higher values improve throughput but may use more memory.
    pub fn set_gc_threshold(&self, threshold: usize) {
        self.interpreter.heap.set_gc_threshold(threshold);
    }

    /// Execute a single bytecode instruction.
    ///
    /// This method provides step-by-step execution for host-controlled interruption.
    /// Call `prepare()` first to compile the code, then call `step()` repeatedly
    /// until a terminal result is returned.
    ///
    /// # Returns
    /// - `StepResult::Continue` - One instruction executed, more to go
    /// - `StepResult::Complete(value)` - Execution finished
    /// - `StepResult::NeedImports(...)` - Need modules before continuing
    /// - `StepResult::Suspended(...)` - Waiting for order fulfillment
    /// - `StepResult::Done` - No active execution (call `prepare()` first)
    ///
    /// # Example
    /// ```ignore
    /// let mut runtime = Runtime::new();
    /// runtime.prepare("let x = 0; while (true) { x++; }", None)?;
    ///
    /// // Run for at most 1000 steps
    /// for _ in 0..1000 {
    ///     match runtime.step()? {
    ///         StepResult::Continue => continue,
    ///         StepResult::Complete(value) => {
    ///             println!("Completed: {:?}", value);
    ///             break;
    ///         }
    ///         StepResult::Done => break,
    ///         _ => break,
    ///     }
    /// }
    /// ```
    #[inline(always)]
    pub fn step(&mut self) -> Result<StepResult, JsError> {
        self.interpreter.step()
    }

    /// Prepare code for step-based execution without running it.
    ///
    /// After calling this, use `step()` to execute one instruction at a time,
    /// or `run_to_completion()` to run until a terminal result.
    ///
    /// # Returns
    /// - `StepResult::Continue` - Ready to step
    /// - `StepResult::NeedImports(...)` - Need modules before continuing
    ///
    /// # Example
    /// ```ignore
    /// let mut runtime = Runtime::new();
    ///
    /// // Prepare and run with step limit
    /// runtime.prepare("let x = 0; while (true) { x++; }", None)?;
    /// for _ in 0..1000 {
    ///     match runtime.step()? {
    ///         StepResult::Continue => continue,
    ///         StepResult::Complete(value) => {
    ///             println!("Completed: {:?}", value);
    ///             break;
    ///         }
    ///         _ => break,
    ///     }
    /// }
    /// ```
    pub fn prepare(&mut self, source: &str, path: Option<&str>) -> Result<StepResult, JsError> {
        self.interpreter.prepare(source, path.map(ModulePath::new))
    }

    /// Get the current call stack depth.
    ///
    /// Returns the depth of the JavaScript call stack (trampoline stack + interpreter call stack).
    /// This can be used by the host to implement stack depth limits.
    ///
    /// Returns 0 if no execution is active.
    pub fn call_depth(&self) -> usize {
        self.interpreter.call_depth()
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
