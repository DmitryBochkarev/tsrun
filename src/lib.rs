//! A minimal TypeScript runtime for embedding in applications.
//!
//! This crate provides a TypeScript interpreter written in Rust, designed for
//! configuration files where users benefit from IDE autocompletion, type checking,
//! and error highlighting. TypeScript features like enums, interfaces, and generics
//! are fully parsed; types are stripped at runtime (not type-checked).
//!
//! # TypeScript Features
//!
//! The interpreter supports TypeScript-specific syntax for better editor experience:
//!
//! - **Enums** - Numeric and string enums with reverse mappings
//! - **Interfaces & Types** - Parsed for IDE support, stripped at runtime
//! - **Decorators** - Class, method, property, and parameter decorators
//! - **Namespaces** - TypeScript namespace declarations
//! - **Generics** - Generic functions and classes
//! - **Parameter Properties** - `constructor(public x: number)` syntax
//!
//! # Quick Start
//!
//! ```
//! use tsrun::{Interpreter, StepResult};
//!
//! let mut interp = Interpreter::new();
//! interp.prepare(r#"
//!     enum Status { Active = 1, Inactive = 0 }
//!     interface Config { status: Status; }
//!     const cfg: Config = { status: Status.Active };
//!     cfg.status
//! "#, None).unwrap();
//!
//! loop {
//!     match interp.step().unwrap() {
//!         StepResult::Continue => continue,
//!         StepResult::Complete(value) => {
//!             assert_eq!(value.as_number(), Some(1.0));
//!             break;
//!         }
//!         _ => panic!("Unexpected result"),
//!     }
//! }
//! ```
//!
//! # Execution Model
//!
//! The interpreter uses step-based execution, giving hosts full control:
//!
//! - [`Interpreter::prepare`] - Compiles code and prepares for execution
//! - [`Interpreter::step`] - Executes one instruction, returns [`StepResult`]
//! - [`StepResult::NeedImports`] - Execution paused, waiting for ES modules
//! - [`StepResult::Suspended`] - Execution paused, waiting for async operations
//!
//! # Working with Values
//!
//! Use the [`api`] module for creating and manipulating JavaScript values:
//!
//! ```
//! use tsrun::{Interpreter, api};
//!
//! let mut interp = Interpreter::new();
//! let guard = api::create_guard(&interp);
//!
//! // Create objects from JSON
//! let user = api::create_from_json(&mut interp, &guard, &serde_json::json!({
//!     "name": "Alice",
//!     "scores": [95, 87, 92]
//! })).unwrap();
//!
//! // Read properties
//! let name = api::get_property(&user, "name").unwrap();
//! assert_eq!(name.as_str(), Some("Alice"));
//!
//! // Call methods on arrays
//! let scores = api::get_property(&user, "scores").unwrap();
//! let joined = api::call_method(&mut interp, &guard, &scores, "join", &["-".into()]).unwrap();
//! assert_eq!(joined.as_str(), Some("95-87-92"));
//! ```
//!
//! # Module Loading
//!
//! ES modules are loaded on-demand. When execution needs an import:
//!
//! ```
//! use tsrun::{Interpreter, StepResult, ModulePath};
//!
//! let mut interp = Interpreter::new();
//! interp.prepare(r#"import { x } from "./config.ts"; x"#, Some("/main.ts".into())).unwrap();
//!
//! loop {
//!     match interp.step().unwrap() {
//!         StepResult::Continue => continue,
//!         StepResult::NeedImports(imports) => {
//!             for import in imports {
//!                 // Host provides module source code
//!                 let source = "export const x = 42;";
//!                 interp.provide_module(import.resolved_path, source).unwrap();
//!             }
//!         }
//!         StepResult::Complete(value) => {
//!             assert_eq!(value.as_number(), Some(42.0));
//!             break;
//!         }
//!         _ => break,
//!     }
//! }
//! ```
//!
//! # Internal Modules
//!
//! Register Rust functions as importable modules:
//!
//! ```
//! use tsrun::{Interpreter, InterpreterConfig, InternalModule, JsValue, Guarded, JsError};
//!
//! fn get_version(
//!     _interp: &mut Interpreter,
//!     _this: JsValue,
//!     _args: &[JsValue]
//! ) -> Result<Guarded, JsError> {
//!     Ok(Guarded::unguarded(JsValue::from("1.0.0")))
//! }
//!
//! let config = InterpreterConfig {
//!     internal_modules: vec![
//!         InternalModule::native("app:version")
//!             .with_function("getVersion", get_version, 0)
//!             .build(),
//!     ],
//!     ..Default::default()
//! };
//! let interp = Interpreter::with_config(config);
//! // Now code can: import { getVersion } from "app:version";
//! ```
//!
//! # GC Safety
//!
//! Objects are garbage-collected. Use [`Guard`] to keep them alive:
//!
//! ```
//! use tsrun::{Interpreter, api};
//!
//! let mut interp = Interpreter::new();
//! let guard = api::create_guard(&interp);
//!
//! // Objects allocated with guard stay alive until guard is dropped
//! let obj = api::create_object(&mut interp, &guard).unwrap();
//! api::set_property(&obj, "x", 42.into()).unwrap();
//!
//! // guard dropped here - obj may be collected
//! ```

// ═══════════════════════════════════════════════════════════════════════════════
// no_std support
// ═══════════════════════════════════════════════════════════════════════════════

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod prelude;
use prelude::ToString;

pub mod api;
pub mod ast;
pub mod compiler;
pub mod error;
pub mod gc;
pub(crate) mod interpreter;
pub mod lexer;
pub mod parser;
pub mod platform;
pub mod string_dict;
pub mod value;

// C FFI module (only when c-api feature is enabled)
#[cfg(feature = "c-api")]
pub mod ffi;

// WebAssembly module (only when wasm feature is enabled on wasm32 target)
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub mod wasm;

use prelude::{Rc, String, Vec, format};

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
    /// `api::create_response_object()` for objects.
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
/// # Creating RuntimeValues
///
/// For primitives (no GC needed):
/// ```
/// use tsrun::{RuntimeValue, JsValue};
///
/// let num = RuntimeValue::unguarded(JsValue::from(42.0));
/// assert_eq!(num.as_number(), Some(42.0));
///
/// let text = RuntimeValue::unguarded(JsValue::from("hello"));
/// assert_eq!(text.as_str(), Some("hello"));
/// ```
///
/// For objects returned from execution:
/// ```
/// use tsrun::{Interpreter, StepResult};
///
/// let mut interp = Interpreter::new();
/// interp.prepare("({ x: 1, y: 2 })", None).unwrap();
///
/// loop {
///     match interp.step().unwrap() {
///         StepResult::Continue => continue,
///         StepResult::Complete(value) => {
///             // value is a RuntimeValue keeping the object alive
///             assert!(value.is_object());
///             break;
///         }
///         _ => break,
///     }
/// }
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

impl core::ops::Deref for RuntimeValue {
    type Target = JsValue;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl core::fmt::Debug for RuntimeValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RuntimeValue")
            .field("value", &self.value)
            .field("guarded", &self._guard.is_some())
            .finish()
    }
}

impl core::fmt::Display for RuntimeValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.value, f)
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
///
/// # Resolution Examples
///
/// ```
/// use tsrun::ModulePath;
///
/// // Relative paths resolve against a base
/// let base = ModulePath::new("/src/app/main.ts");
/// let resolved = ModulePath::resolve("./utils.ts", Some(&base));
/// assert_eq!(resolved.as_str(), "/src/app/utils.ts");
///
/// // Parent directory traversal
/// let resolved = ModulePath::resolve("../lib/helper.ts", Some(&base));
/// assert_eq!(resolved.as_str(), "/src/lib/helper.ts");
///
/// // Bare specifiers pass through for host resolution
/// let resolved = ModulePath::resolve("lodash", Some(&base));
/// assert_eq!(resolved.as_str(), "lodash");
///
/// // Absolute paths are just normalized
/// let resolved = ModulePath::resolve("/lib/../src/index.ts", None);
/// assert_eq!(resolved.as_str(), "/src/index.ts");
/// ```
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

impl core::fmt::Display for ModulePath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
///
/// # Execution Loop
///
/// ```
/// use tsrun::{Interpreter, StepResult};
///
/// fn run_to_completion(interp: &mut Interpreter) -> Option<f64> {
///     loop {
///         match interp.step().ok()? {
///             StepResult::Continue => continue,
///             StepResult::Complete(val) => return val.as_number(),
///             StepResult::NeedImports(_) => return None, // would need module loading
///             StepResult::Suspended { .. } => return None, // would need async handling
///             StepResult::Done => return None,
///         }
///     }
/// }
///
/// let mut interp = Interpreter::new();
/// interp.prepare("2 ** 10", None).unwrap();
/// assert_eq!(run_to_completion(&mut interp), Some(1024.0));
/// ```
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

/// Definition of an internal module that can be imported from JavaScript.
///
/// Internal modules allow you to expose Rust functions to JavaScript code.
/// They're imported using the specifier you define (e.g., `import { x } from "mymodule"`).
///
/// # Native Module (Rust functions)
///
/// ```
/// use tsrun::{InternalModule, JsValue, Guarded, JsError, Interpreter};
///
/// fn add(_: &mut Interpreter, _: JsValue, args: &[JsValue]) -> Result<Guarded, JsError> {
///     let a = args.first().and_then(|v| v.as_number()).unwrap_or(0.0);
///     let b = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0);
///     Ok(Guarded::unguarded(JsValue::from(a + b)))
/// }
///
/// let module = InternalModule::native("math:utils")
///     .with_function("add", add, 2)
///     .with_value("PI", JsValue::from(3.14159))
///     .build();
///
/// assert_eq!(module.specifier, "math:utils");
/// ```
///
/// # Source Module (TypeScript code)
///
/// ```
/// use tsrun::InternalModule;
///
/// let module = InternalModule::source("config:defaults", r#"
///     export const timeout = 5000;
///     export const retries = 3;
/// "#);
/// ```
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
// Interpreter Configuration
// ═══════════════════════════════════════════════════════════════════════════════

/// Configuration for creating an Interpreter
#[derive(Default)]
pub struct InterpreterConfig {
    /// Internal modules available for import
    pub internal_modules: Vec<InternalModule>,

    /// Custom RegExp provider.
    ///
    /// If `None`, uses the default provider:
    /// - `FancyRegexProvider` when `regex` feature is enabled
    /// - `NoOpRegExpProvider` otherwise
    pub regexp_provider: Option<Rc<dyn platform::RegExpProvider>>,
}

