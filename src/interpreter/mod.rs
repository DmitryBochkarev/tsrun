//! Interpreter for executing TypeScript AST

// Builtin function implementations (split into separate files)
pub mod builtins;

// Evaluation stack for suspendable execution
pub mod eval_stack;

// Import all builtin functions
use builtins::*;

use crate::ast::{
    Argument, ArrayElement, AssignmentExpression, AssignmentOp, AssignmentTarget, BinaryExpression,
    BinaryOp, BlockStatement, CallExpression, ClassConstructor, ClassDeclaration, ClassMember,
    ClassMethod, ClassProperty, ConditionalExpression, DoWhileStatement, EnumDeclaration,
    ExportDeclaration, Expression, ForInOfLeft, ForInStatement, ForInit, ForOfStatement,
    ForStatement, FunctionDeclaration, LiteralValue, LogicalExpression, LogicalOp,
    MemberExpression, MemberProperty, MethodKind, NamespaceDeclaration, NewExpression,
    ObjectPatternProperty, ObjectProperty, ObjectPropertyKey, Pattern, Program, Statement,
    UnaryExpression, UnaryOp, UpdateExpression, UpdateOp, VariableDeclaration, VariableKind,
    WhileStatement,
};
use crate::error::JsError;
use crate::gc::{GuardedGc, Space};
use crate::string_dict::StringDict;
use crate::value::{
    create_array, create_environment, create_function, create_object, Binding, CheapClone, EnvRef,
    ExoticObject, FunctionBody, GeneratorState, GeneratorStatus, InterpretedFunction, JsFunction,
    JsObject, JsObjectRef, JsString, JsValue, NativeFn, NativeFunction, PromiseStatus, Property,
    PropertyKey,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Completion record for control flow
#[derive(Debug)]
pub enum Completion {
    Normal(JsValue),
    Return(JsValue),
    Break(Option<JsString>),
    Continue(Option<JsString>),
}

/// A stack frame for tracking call stack
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Function name (or "<anonymous>" for anonymous functions)
    pub function_name: String,
    /// Source location if available
    pub location: Option<(u32, u32)>, // (line, column)
}

/// Context for generator execution
#[derive(Debug, Clone)]
pub struct GeneratorContext {
    /// Which yield point to stop at (0 = first yield)
    pub target_yield: usize,
    /// Current yield counter during execution
    pub current_yield: usize,
    /// Value to inject for the current yield (from next(value))
    pub sent_value: JsValue,
    /// Whether to throw the sent_value as an exception
    pub throw_value: bool,
}

/// Result of a yield point
pub enum YieldResult {
    /// Continue execution (skip this yield)
    Continue,
    /// Suspend at this yield with the given value
    Suspend(JsValue),
}

/// Result of processing a single evaluation frame
#[allow(dead_code)]
enum FrameResult {
    /// Continue processing more frames
    Continue,
    /// Suspend execution and return to host
    Suspend(crate::RuntimeResult),
}

/// GC statistics for debugging and monitoring
#[derive(Debug, Clone)]
pub struct GcStats {
    pub alive_count: usize,
    pub tracked_count: usize,
    pub roots_count: usize,
    pub free_count: usize,
    pub gc_threshold: usize,
    pub allocs_since_gc: usize,
}

/// The interpreter state
pub struct Interpreter {
    /// GC space managing all JavaScript objects
    pub gc_space: Space<JsObject>,
    /// Global object (rooted)
    pub global: JsObjectRef,
    /// Global environment (rooted) - bindings for global scope
    pub global_env: EnvRef,
    /// Current environment (GC-managed, changes during execution)
    pub env: EnvRef,
    /// String dictionary for deduplicating strings
    pub string_dict: StringDict,
    /// Object.prototype for all objects
    pub object_prototype: JsObjectRef,
    /// Array.prototype for all array instances
    pub array_prototype: JsObjectRef,
    /// String.prototype for string methods
    pub string_prototype: JsObjectRef,
    /// Number.prototype for number methods
    pub number_prototype: JsObjectRef,
    /// Function.prototype for function methods (call, apply, bind)
    pub function_prototype: JsObjectRef,
    /// Map.prototype for map methods
    pub map_prototype: JsObjectRef,
    /// Set.prototype for set methods
    pub set_prototype: JsObjectRef,
    /// Date.prototype for date methods
    pub date_prototype: JsObjectRef,
    /// RegExp.prototype for regex methods
    pub regexp_prototype: JsObjectRef,
    /// Error.prototype for error methods
    pub error_prototype: JsObjectRef,
    /// Symbol.prototype for symbol methods
    pub symbol_prototype: JsObjectRef,
    /// Generator.prototype for generator methods
    pub generator_prototype: JsObjectRef,
    /// Promise.prototype for promise methods
    pub promise_prototype: JsObjectRef,
    /// Stores thrown value during exception propagation
    thrown_value: Option<JsValue>,
    /// Exported values from the module
    pub exports: rustc_hash::FxHashMap<JsString, JsValue>,
    /// Call stack for stack traces
    pub call_stack: Vec<StackFrame>,
    /// Generator execution context (Some when executing inside a generator)
    generator_context: Option<GeneratorContext>,

    // ═══════════════════════════════════════════════════════════════
    // State machine execution (for suspendable evaluation)
    // ═══════════════════════════════════════════════════════════════
    /// Explicit evaluation stack (replaces Rust call stack for suspendable execution)
    eval_stack: Vec<eval_stack::EvalFrame>,
    /// Value stack for intermediate results during stack-based evaluation
    value_stack: Vec<JsValue>,
    /// Completion stack for tracking control flow during stack-based evaluation
    completion_stack: Vec<eval_stack::CompletionValue>,
    /// Counter for generating unique slot IDs
    next_slot_id: u64,
    /// Static imports collected from program, to be resolved before execution
    static_imports: Vec<StaticImport>,
    /// Index of next static import to process
    static_import_index: usize,
    /// Currently pending slot (if waiting for host)
    pending_slot: Option<crate::PendingSlot>,
    /// Program body saved for execution after imports resolved
    pending_program_body: Option<Vec<Statement>>,

    // ═══════════════════════════════════════════════════════════════
    // Timeout tracking
    // ═══════════════════════════════════════════════════════════════
    /// Execution start time (set when execution begins)
    execution_start: Option<std::time::Instant>,
    /// Maximum execution time in milliseconds (default: 3000ms)
    timeout_ms: u64,

    // ═══════════════════════════════════════════════════════════════
    // GC rooting for escaped values
    // ═══════════════════════════════════════════════════════════════
    /// Values returned from eval() that need to stay rooted until released
    escaped_values: Vec<JsValue>,
}

/// A static import declaration to be resolved
#[derive(Debug, Clone)]
pub struct StaticImport {
    /// Module specifier (e.g., "./module" or "lodash")
    pub specifier: String,
    /// How to bind the imported values
    pub bindings: eval_stack::ImportBindings,
}

/// Saved execution state for nested module loading
///
/// When loading a module during another module's execution,
/// we need to save and restore the execution state.
pub struct SavedExecutionState {
    eval_stack: Vec<eval_stack::EvalFrame>,
    value_stack: Vec<JsValue>,
    completion_stack: Vec<eval_stack::CompletionValue>,
    static_imports: Vec<StaticImport>,
    static_import_index: usize,
    pending_slot: Option<crate::PendingSlot>,
    pending_program_body: Option<Vec<Statement>>,
    exports: rustc_hash::FxHashMap<JsString, JsValue>,
}

impl Interpreter {
    /// Create a new interpreter with global environment
    pub fn new() -> Self {
        let mut gc_space = Space::with_capacity(4096);
        let string_dict = StringDict::with_common_strings();

        // Create global object and root it - also used as placeholder for prototypes
        let global = create_object(&mut gc_space);
        gc_space.add_root(&global);

        // Create global environment and root it
        let global_env = create_environment(&mut gc_space, None);
        gc_space.add_root(&global_env);

        // Add basic global values to the global environment
        {
            let mut env_ref = global_env.borrow_mut();
            if let Some(env_data) = env_ref.as_environment_mut() {
                env_data.bindings.insert(
                    JsString::from("undefined"),
                    Binding {
                        value: JsValue::Undefined,
                        mutable: false,
                        initialized: true,
                    },
                );
                env_data.bindings.insert(
                    JsString::from("NaN"),
                    Binding {
                        value: JsValue::Number(f64::NAN),
                        mutable: false,
                        initialized: true,
                    },
                );
                env_data.bindings.insert(
                    JsString::from("Infinity"),
                    Binding {
                        value: JsValue::Number(f64::INFINITY),
                        mutable: false,
                        initialized: true,
                    },
                );
            }
        }

        // Create interpreter with placeholder prototypes (global object)
        // We'll initialize the real prototypes below
        let placeholder = global.clone();
        let mut interp = Self {
            gc_space,
            global,
            global_env: global_env.clone(),
            env: global_env,
            string_dict,
            object_prototype: placeholder.clone(),
            array_prototype: placeholder.clone(),
            string_prototype: placeholder.clone(),
            number_prototype: placeholder.clone(),
            function_prototype: placeholder.clone(),
            map_prototype: placeholder.clone(),
            set_prototype: placeholder.clone(),
            date_prototype: placeholder.clone(),
            regexp_prototype: placeholder.clone(),
            error_prototype: placeholder.clone(),
            symbol_prototype: placeholder.clone(),
            generator_prototype: placeholder.clone(),
            promise_prototype: placeholder,
            thrown_value: None,
            exports: rustc_hash::FxHashMap::default(),
            call_stack: Vec::new(),
            generator_context: None,
            eval_stack: Vec::new(),
            value_stack: Vec::new(),
            completion_stack: Vec::new(),
            next_slot_id: 0,
            static_imports: Vec::new(),
            static_import_index: 0,
            pending_slot: None,
            pending_program_body: None,
            execution_start: None,
            timeout_ms: 3000,
            escaped_values: Vec::new(),
        };

        // Now initialize prototypes using &mut self
        interp.init_prototypes();
        interp
    }

    /// Initialize all builtin prototypes and constructors
    fn init_prototypes(&mut self) {
        // Create prototypes using builtin module functions (all rooted)
        let object_prototype = create_object_prototype(self);
        self.gc_space.add_root(&object_prototype);
        self.object_prototype = object_prototype;

        let array_prototype = create_array_prototype(self);
        self.gc_space.add_root(&array_prototype);
        self.array_prototype = array_prototype;

        let string_prototype = create_string_prototype(self);
        self.gc_space.add_root(&string_prototype);
        self.string_prototype = string_prototype;

        let number_prototype = create_number_prototype(self);
        self.gc_space.add_root(&number_prototype);
        self.number_prototype = number_prototype;

        let function_prototype = create_function_prototype(self);
        self.gc_space.add_root(&function_prototype);
        self.function_prototype = function_prototype;

        let map_prototype = create_map_prototype(self);
        self.gc_space.add_root(&map_prototype);
        self.map_prototype = map_prototype;

        let set_prototype = create_set_prototype(self);
        self.gc_space.add_root(&set_prototype);
        self.set_prototype = set_prototype;

        let date_prototype = create_date_prototype(self);
        self.gc_space.add_root(&date_prototype);
        self.date_prototype = date_prototype;

        let regexp_prototype = create_regexp_prototype(self);
        self.gc_space.add_root(&regexp_prototype);
        self.regexp_prototype = regexp_prototype;

        let error_prototype = create_error_prototype(self);
        self.gc_space.add_root(&error_prototype);
        self.error_prototype = error_prototype;

        let symbol_prototype = create_symbol_prototype(self);
        self.gc_space.add_root(&symbol_prototype);
        self.symbol_prototype = symbol_prototype;

        let generator_prototype = create_generator_prototype(self);
        self.gc_space.add_root(&generator_prototype);
        self.generator_prototype = generator_prototype;

        let promise_prototype = create_promise_prototype(self);
        self.gc_space.add_root(&promise_prototype);
        self.promise_prototype = promise_prototype;

        // Create and register constructors
        let object_constructor = create_object_constructor(self);
        self.env_define("Object", JsValue::Object(object_constructor), false);

        let array_constructor = create_array_constructor(self);
        self.env_define("Array", JsValue::Object(array_constructor), false);

        let string_constructor = create_string_constructor(self);
        self.env_define("String", JsValue::Object(string_constructor), false);

        let number_constructor = create_number_constructor(self);
        self.env_define("Number", JsValue::Object(number_constructor), false);

        let date_constructor = create_date_constructor(self);
        self.env_define("Date", JsValue::Object(date_constructor), false);

        let regexp_constructor = create_regexp_constructor(self);
        self.env_define("RegExp", JsValue::Object(regexp_constructor), false);

        let map_constructor = create_map_constructor(self);
        self.env_define("Map", JsValue::Object(map_constructor), false);

        let set_constructor = create_set_constructor(self);
        self.env_define("Set", JsValue::Object(set_constructor), false);

        // Create and register global objects
        let console = create_console_object(self);
        self.env_define("console", JsValue::Object(console), false);

        let json = create_json_object(self);
        self.env_define("JSON", JsValue::Object(json), false);

        let math = create_math_object(self);
        self.env_define("Math", JsValue::Object(math), false);

        // Register global functions
        register_global_functions(self);

        // Register error constructors
        let error_ctors = create_error_constructors(self);
        self.env_define("Error", JsValue::Object(error_ctors.error), false);
        self.env_define("TypeError", JsValue::Object(error_ctors.type_error), false);
        self.env_define(
            "ReferenceError",
            JsValue::Object(error_ctors.reference_error),
            false,
        );
        self.env_define(
            "SyntaxError",
            JsValue::Object(error_ctors.syntax_error),
            false,
        );
        self.env_define(
            "RangeError",
            JsValue::Object(error_ctors.range_error),
            false,
        );
        self.env_define("URIError", JsValue::Object(error_ctors.uri_error), false);
        self.env_define("EvalError", JsValue::Object(error_ctors.eval_error), false);

        // Register Symbol constructor
        let well_known_symbols = get_well_known_symbols();
        let symbol_constructor = create_symbol_constructor(self, &well_known_symbols);
        self.env_define("Symbol", JsValue::Object(symbol_constructor), false);

        // Register Promise constructor
        let promise_constructor = create_promise_constructor(self);
        self.env_define("Promise", JsValue::Object(promise_constructor), false);
    }

    /// Get the current stack trace as a formatted string
    pub fn format_stack_trace(&self, error_name: &str, message: &str) -> String {
        let mut trace = format!("{}: {}", error_name, message);
        for frame in self.call_stack.iter().rev() {
            if let Some((line, col)) = frame.location {
                trace.push_str(&format!(
                    "\n    at {} (line {}:{})",
                    frame.function_name, line, col
                ));
            } else {
                trace.push_str(&format!("\n    at {}", frame.function_name));
            }
        }
        trace
    }

    /// Intern a string in the dictionary, returning a shared JsString.
    ///
    /// Use this for frequently-used strings to avoid duplicate allocations.
    #[inline]
    pub fn intern(&mut self, s: &str) -> JsString {
        self.string_dict.get_or_insert(s)
    }

    /// Create a PropertyKey from an interned string.
    ///
    /// This is more efficient than `PropertyKey::from(s)` when the string
    /// is likely to be reused, as it shares the underlying allocation.
    #[inline]
    pub fn key(&mut self, s: &str) -> PropertyKey {
        PropertyKey::String(self.string_dict.get_or_insert(s))
    }

    /// Create an array with the proper prototype
    pub fn create_array(&mut self, elements: Vec<JsValue>) -> JsObjectRef {
        let arr = create_array(&mut self.gc_space, &mut self.string_dict, elements);
        arr.borrow_mut().prototype = Some(self.array_prototype.clone());
        arr
    }

    /// Create a plain object with the proper prototype
    pub fn create_object(&mut self) -> JsObjectRef {
        let obj = create_object(&mut self.gc_space);
        obj.borrow_mut().prototype = Some(self.object_prototype.clone());
        obj
    }

    /// Create a plain object with GC protection during construction.
    ///
    /// Use this when the object needs to survive GC while evaluating property
    /// values or other operations that may trigger allocation.
    pub fn create_object_guarded(&mut self) -> GuardedGc<JsObject> {
        let guarded = self.gc_space.alloc_guarded(JsObject::default());
        guarded.borrow_mut().prototype = Some(self.object_prototype.clone());
        guarded
    }

    /// Create an array object with GC protection during construction.
    ///
    /// Use this when the array needs to survive GC while additional
    /// operations may trigger allocation.
    pub fn create_array_guarded(&mut self, elements: Vec<JsValue>) -> GuardedGc<JsObject> {
        let len = elements.len() as u32;
        let mut obj = JsObject {
            exotic: ExoticObject::Array { length: len },
            prototype: Some(self.array_prototype.clone()),
            ..Default::default()
        };
        for (i, elem) in elements.into_iter().enumerate() {
            obj.set_property(PropertyKey::Index(i as u32), elem);
        }
        let length_key = self.key("length");
        obj.set_property(length_key, JsValue::Number(len as f64));
        self.gc_space.alloc_guarded(obj)
    }

    /// Guard a JsValue if it contains an object.
    ///
    /// Returns Some(GuardedGc) if the value is an object, None otherwise.
    /// Use this to protect values that need to survive across allocating operations.
    pub fn guard_value(&mut self, value: &JsValue) -> Option<GuardedGc<JsObject>> {
        if let JsValue::Object(obj) = value {
            Some(self.gc_space.guard(obj))
        } else {
            None
        }
    }

    /// Create a function object with the proper prototype
    pub fn create_function(&mut self, func: JsFunction) -> JsObjectRef {
        let obj = create_function(&mut self.gc_space, &mut self.string_dict, func);
        obj.borrow_mut().prototype = Some(self.function_prototype.clone());
        obj
    }

    /// Register a native method on a prototype object
    ///
    /// This is a helper for builtin registration that uses the string dictionary
    /// for property key interning.
    pub fn register_method(&mut self, obj: &JsObjectRef, name: &str, func: NativeFn, arity: usize) {
        let interned_name = self.intern(name);
        let f = create_function(
            &mut self.gc_space,
            &mut self.string_dict,
            JsFunction::Native(NativeFunction {
                name: interned_name.cheap_clone(),
                func,
                arity,
            }),
        );
        let key = PropertyKey::String(interned_name);
        obj.borrow_mut().set_property(key, JsValue::Object(f));
    }

    /// Create a generator result object { value, done }
    pub fn create_generator_result(&mut self, value: JsValue, done: bool) -> JsValue {
        let value_key = self.key("value");
        let done_key = self.key("done");
        let obj = self.create_object();
        {
            let mut o = obj.borrow_mut();
            o.set_property(value_key, value);
            o.set_property(done_key, JsValue::Boolean(done));
        }
        JsValue::Object(obj)
    }

    /// Create a module object with the given exports
    ///
    /// This is used to create module namespace objects for import resolution.
    pub fn create_module_object(&mut self, exports: Vec<(String, JsValue)>) -> JsValue {
        // Intern all keys first
        let keyed_exports: Vec<_> = exports
            .into_iter()
            .map(|(name, value)| (self.key(&name), value))
            .collect();
        let obj = create_object(&mut self.gc_space);
        {
            let mut obj_ref = obj.borrow_mut();
            obj_ref.prototype = Some(self.object_prototype.clone());
            for (key, value) in keyed_exports {
                obj_ref.set_property(key, value);
            }
        }
        JsValue::Object(obj)
    }

    /// Run garbage collection manually
    pub fn collect_garbage(&mut self) {
        self.gc_space.collect();
    }

    /// Get GC statistics
    pub fn gc_stats(&self) -> GcStats {
        GcStats {
            alive_count: self.gc_space.alive_count(),
            tracked_count: self.gc_space.tracked_count(),
            roots_count: self.gc_space.roots_count(),
            free_count: self.gc_space.free_count(),
            gc_threshold: self.gc_space.gc_threshold(),
            allocs_since_gc: self.gc_space.allocs_since_gc(),
        }
    }

    /// Get the current GC threshold
    pub fn gc_threshold(&self) -> usize {
        self.gc_space.gc_threshold()
    }

    /// Set the GC threshold
    ///
    /// - `0`: Disable threshold-based collection (only collect when memory exhausted)
    /// - `n > 0`: Collect after every `n` allocations
    ///
    /// Lower values reduce peak memory but increase GC overhead.
    pub fn set_gc_threshold(&mut self, threshold: usize) {
        self.gc_space.set_gc_threshold(threshold);
    }

    /// Capture an escaping value to keep it rooted until released
    ///
    /// Values returned from eval() are added as GC roots to prevent collection.
    /// Call release_escaped_values() when you're done with them.
    pub fn capture_escaping_value(&mut self, value: &JsValue) {
        // Add any objects in the value as GC roots
        if let JsValue::Object(obj) = value {
            self.gc_space.add_root(obj);
        }
        self.escaped_values.push(value.clone());
    }

    /// Release all escaped values, allowing them to be garbage collected
    ///
    /// Call this when you're done with all values returned from eval().
    pub fn release_escaped_values(&mut self) {
        // Remove GC roots for all escaped objects
        for value in &self.escaped_values {
            if let JsValue::Object(obj) = value {
                self.gc_space.remove_root(obj);
            }
        }
        self.escaped_values.clear();
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Environment Operations (GC-managed environments)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Allocate a new environment with the given outer scope
    ///
    /// The new environment is automatically rooted to prevent GC collection.
    pub fn env_alloc(&mut self, outer: Option<EnvRef>) -> EnvRef {
        let env = create_environment(&mut self.gc_space, outer);
        self.gc_space.add_root(&env);
        env
    }

    /// Restore a previous environment, unrooting the current one
    ///
    /// Call this when exiting a scope to allow GC to collect the environment.
    pub fn env_restore(&mut self, old_env: EnvRef) {
        self.gc_space.remove_root(&self.env);
        self.env = old_env;
    }

    /// Define a binding in the current environment
    pub fn env_define(&mut self, name: impl Into<JsString>, value: JsValue, mutable: bool) {
        let name = name.into();
        let mut env_ref = self.env.borrow_mut();
        if let Some(env_data) = env_ref.as_environment_mut() {
            env_data.bindings.insert(
                name,
                Binding {
                    value,
                    mutable,
                    initialized: true,
                },
            );
        }
    }

    /// Define a binding in a specific environment
    pub fn env_define_in(
        &mut self,
        env: &EnvRef,
        name: impl Into<JsString>,
        value: JsValue,
        mutable: bool,
    ) {
        let name = name.into();
        let mut env_ref = env.borrow_mut();
        if let Some(env_data) = env_ref.as_environment_mut() {
            env_data.bindings.insert(
                name,
                Binding {
                    value,
                    mutable,
                    initialized: true,
                },
            );
        }
    }

    /// Get a binding value from the current environment (walks the scope chain)
    pub fn env_get_binding(&self, name: &JsString) -> Result<JsValue, JsError> {
        self.env_get_binding_in(&self.env, name)
    }

    /// Get a binding value from a specific environment (walks the scope chain)
    pub fn env_get_binding_in(&self, env: &EnvRef, name: &JsString) -> Result<JsValue, JsError> {
        let mut current = Some(env.cheap_clone());
        while let Some(env_ref) = current.take() {
            let borrowed = env_ref.borrow();
            if let Some(env_data) = borrowed.as_environment() {
                if let Some(binding) = env_data.bindings.get(name) {
                    if !binding.initialized {
                        return Err(JsError::reference_error(format!(
                            "Cannot access '{}' before initialization",
                            name
                        )));
                    }
                    return Ok(binding.value.clone());
                }
                let outer = env_data.outer.as_ref().map(|o| o.cheap_clone());
                drop(borrowed);
                current = outer;
            } else {
                break;
            }
        }
        Err(JsError::reference_error(format!("{} is not defined", name)))
    }

    /// Set a binding value in the current environment (walks the scope chain)
    pub fn env_set_binding(&mut self, name: &JsString, value: JsValue) -> Result<(), JsError> {
        self.env_set_binding_in(&self.env.cheap_clone(), name, value)
    }

    /// Set a binding value in a specific environment (walks the scope chain)
    pub fn env_set_binding_in(
        &mut self,
        env: &EnvRef,
        name: &JsString,
        value: JsValue,
    ) -> Result<(), JsError> {
        let mut current = Some(env.cheap_clone());
        while let Some(env_ref) = current.take() {
            let mut borrowed = env_ref.borrow_mut();
            if let Some(env_data) = borrowed.as_environment_mut() {
                if let Some(binding) = env_data.bindings.get_mut(name) {
                    if !binding.mutable {
                        return Err(JsError::type_error(format!(
                            "Assignment to constant variable '{}'",
                            name
                        )));
                    }
                    binding.value = value;
                    return Ok(());
                }
                let outer = env_data.outer.as_ref().map(|o| o.cheap_clone());
                drop(borrowed);
                current = outer;
            } else {
                break;
            }
        }
        Err(JsError::reference_error(format!("{} is not defined", name)))
    }

    /// Check if a binding exists in the current environment (walks the scope chain)
    pub fn env_has_binding(&self, name: &JsString) -> bool {
        self.env_has_binding_in(&self.env, name)
    }

    /// Check if a binding exists in a specific environment (walks the scope chain)
    pub fn env_has_binding_in(&self, env: &EnvRef, name: &JsString) -> bool {
        let mut current = Some(env.cheap_clone());
        while let Some(env_ref) = current.take() {
            let borrowed = env_ref.borrow();
            if let Some(env_data) = borrowed.as_environment() {
                if env_data.bindings.contains_key(name) {
                    return true;
                }
                let outer = env_data.outer.as_ref().map(|o| o.cheap_clone());
                drop(borrowed);
                current = outer;
            } else {
                break;
            }
        }
        false
    }

    /// Check if a binding exists only in the current environment (no chain walk)
    pub fn env_has_own_binding(&self, name: &JsString) -> bool {
        self.env_has_own_binding_in(&self.env, name)
    }

    /// Check if a binding exists only in a specific environment (no chain walk)
    pub fn env_has_own_binding_in(&self, env: &EnvRef, name: &JsString) -> bool {
        let borrowed = env.borrow();
        if let Some(env_data) = borrowed.as_environment() {
            env_data.bindings.contains_key(name)
        } else {
            false
        }
    }

    /// Delete a binding from the current environment
    pub fn env_delete(&mut self, name: &JsString) {
        self.env_delete_in(&self.env.cheap_clone(), name)
    }

    /// Delete a binding from a specific environment
    pub fn env_delete_in(&mut self, env: &EnvRef, name: &JsString) {
        let mut borrowed = env.borrow_mut();
        if let Some(env_data) = borrowed.as_environment_mut() {
            env_data.bindings.remove(name);
        }
    }

    /// Set the execution timeout in milliseconds
    ///
    /// Default is 3000ms (3 seconds). Set to 0 to disable timeout.
    pub fn set_timeout_ms(&mut self, timeout_ms: u64) {
        self.timeout_ms = timeout_ms;
    }

    /// Get the current execution timeout in milliseconds
    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    /// Resume a generator, executing until the next yield or completion
    pub fn resume_generator(
        &mut self,
        gen_state: &Rc<RefCell<GeneratorState>>,
    ) -> Result<JsValue, JsError> {
        let (body, closure, target_yield, sent_value, _status, params, args) = {
            let state = gen_state.borrow();
            if state.state == GeneratorStatus::Completed {
                return Ok(self.create_generator_result(JsValue::Undefined, true));
            }
            // Clone Rc refs and values to release borrow before execution
            (
                state.body.cheap_clone(),    // Rc clone - cheap
                state.closure.cheap_clone(), // EnvRef (JsObjectRef) clone - cheap
                state.stmt_index,
                state.sent_value.clone(), // JsValue clone - may contain Rc types
                state.state.clone(),      // enum Copy
                state.params.cheap_clone(), // Rc clone - cheap
                state.args.clone(),       // Vec<JsValue> clone - needed for parameter values
            )
        };

        // Save the outer generator context (for yield* delegation support)
        let saved_generator_context = self.generator_context.take();

        // Set up generator context for this generator
        self.generator_context = Some(GeneratorContext {
            target_yield,
            current_yield: 0,
            sent_value,
            throw_value: false,
        });

        // Save current environment and set up generator environment
        let saved_env = self.env.cheap_clone();
        self.env = self.env_alloc(Some(closure.cheap_clone()));

        // Bind parameters
        for (i, param) in params.iter().enumerate() {
            let arg = args.get(i).cloned().unwrap_or(JsValue::Undefined);
            self.bind_pattern(&param.pattern, arg, true)?;
        }

        // Execute the generator body
        let result = self.execute_generator_body(&body.body);

        // Restore environment (env_restore unroots the current env)
        self.env_restore(saved_env);

        // Get the final generator context state and restore outer context
        let ctx = self.generator_context.take();
        self.generator_context = saved_generator_context;

        match result {
            Ok(Completion::Normal(_)) => {
                // Generator completed normally
                gen_state.borrow_mut().state = GeneratorStatus::Completed;
                Ok(self.create_generator_result(JsValue::Undefined, true))
            }
            Ok(Completion::Return(val)) => {
                // Generator returned
                gen_state.borrow_mut().state = GeneratorStatus::Completed;
                Ok(self.create_generator_result(val, true))
            }
            Err(JsError::GeneratorYield { value }) => {
                // Generator yielded - update state for next resume
                if let Some(ctx) = ctx {
                    gen_state.borrow_mut().stmt_index = ctx.current_yield;
                }
                Ok(self.create_generator_result(value, false))
            }
            Err(e) => {
                // Generator threw an error
                gen_state.borrow_mut().state = GeneratorStatus::Completed;
                Err(e)
            }
            _ => {
                gen_state.borrow_mut().state = GeneratorStatus::Completed;
                Ok(self.create_generator_result(JsValue::Undefined, true))
            }
        }
    }

    /// Resume a generator with a thrown exception
    pub fn resume_generator_with_throw(
        &mut self,
        gen_state: &Rc<RefCell<GeneratorState>>,
    ) -> Result<JsValue, JsError> {
        let (body, closure, target_yield, sent_value, params, args) = {
            let state = gen_state.borrow();
            if state.state == GeneratorStatus::Completed {
                return Err(JsError::type_error("Generator is already completed"));
            }
            // Clone Rc refs and values to release borrow before execution
            (
                state.body.cheap_clone(),    // Rc clone - cheap
                state.closure.cheap_clone(), // EnvRef (JsObjectRef) clone - cheap
                state.stmt_index,
                state.sent_value.clone(), // JsValue clone - may contain Rc types
                state.params.cheap_clone(), // Rc clone - cheap
                state.args.clone(),       // Vec<JsValue> clone - needed for parameter values
            )
        };

        // Save the outer generator context (for yield* delegation support)
        let saved_generator_context = self.generator_context.take();

        // Set up generator context with throw flag
        self.generator_context = Some(GeneratorContext {
            target_yield,
            current_yield: 0,
            sent_value,
            throw_value: true,
        });

        // Save current environment and set up generator environment
        let saved_env = self.env.cheap_clone();
        self.env = self.env_alloc(Some(closure.cheap_clone()));

        // Bind parameters
        for (i, param) in params.iter().enumerate() {
            let arg = args.get(i).cloned().unwrap_or(JsValue::Undefined);
            self.bind_pattern(&param.pattern, arg, true)?;
        }

        // Execute the generator body
        let result = self.execute_generator_body(&body.body);

        // Restore environment (env_restore unroots the current env)
        self.env_restore(saved_env);

        // Get the final generator context state and restore outer context
        let ctx = self.generator_context.take();
        self.generator_context = saved_generator_context;

        match result {
            Ok(Completion::Normal(_)) => {
                gen_state.borrow_mut().state = GeneratorStatus::Completed;
                Ok(self.create_generator_result(JsValue::Undefined, true))
            }
            Ok(Completion::Return(val)) => {
                gen_state.borrow_mut().state = GeneratorStatus::Completed;
                Ok(self.create_generator_result(val, true))
            }
            Err(JsError::GeneratorYield { value }) => {
                if let Some(ctx) = ctx {
                    gen_state.borrow_mut().stmt_index = ctx.current_yield;
                }
                Ok(self.create_generator_result(value, false))
            }
            Err(e) => {
                gen_state.borrow_mut().state = GeneratorStatus::Completed;
                Err(e)
            }
            _ => {
                gen_state.borrow_mut().state = GeneratorStatus::Completed;
                Ok(self.create_generator_result(JsValue::Undefined, true))
            }
        }
    }

    /// Execute generator body statements
    fn execute_generator_body(&mut self, stmts: &[Statement]) -> Result<Completion, JsError> {
        let mut result = Completion::Normal(JsValue::Undefined);
        for stmt in stmts {
            result = self.execute_statement(stmt)?;
            match &result {
                Completion::Return(_) => return Ok(result),
                Completion::Break(_) | Completion::Continue(_) => return Ok(result),
                _ => {}
            }
        }
        Ok(result)
    }

    /// Execute a program using the stack-based execution model
    ///
    /// This method supports suspension at import/await points by returning
    /// RuntimeResult::ImportAwaited or RuntimeResult::AsyncAwaited.
    pub fn execute(&mut self, program: &Program) -> Result<crate::RuntimeResult, JsError> {
        // Start the execution timer
        self.execution_start = Some(std::time::Instant::now());

        // Hoist var declarations at global scope
        self.hoist_var_declarations(&program.body);

        // Initialize the execution state
        self.eval_stack.clear();
        self.value_stack.clear();
        self.completion_stack.clear();
        self.static_imports.clear();
        self.static_import_index = 0;
        self.pending_slot = None;

        // Collect static imports from the program
        self.collect_static_imports(&program.body);

        // Save program body for execution after imports are resolved
        // Filter out import statements (they're handled separately)
        let non_import_stmts: Vec<Statement> = program
            .body
            .iter()
            .filter(|s| !matches!(s, Statement::Import(_)))
            .cloned()
            .collect();
        self.pending_program_body = Some(non_import_stmts);

        // Process imports first (hoisted), then run program
        self.process_next_import_or_execute()
    }

    /// Continue execution after a pending slot has been filled
    pub fn continue_execution(&mut self) -> Result<crate::RuntimeResult, JsError> {
        // Check if there's a pending slot and it's been filled
        if let Some(slot) = self.pending_slot.take() {
            match slot.take() {
                Some(Ok(module_value)) => {
                    // Bind the import
                    let import = self
                        .static_imports
                        .get(self.static_import_index.saturating_sub(1))
                        .ok_or_else(|| JsError::internal_error("import index out of bounds"))?;
                    // ImportBindings clone - enum with String fields
                    self.bind_import(&import.bindings.clone(), module_value)?;
                }
                Some(Err(error)) => {
                    // Error loading module - propagate it
                    return Err(error);
                }
                None => {
                    return Err(JsError::type_error(
                        "continue_eval called but slot was not filled",
                    ));
                }
            }
        }

        // Continue with next import or program execution
        self.process_next_import_or_execute()
    }

    /// Save the current execution state for nested module loading
    ///
    /// Call this before executing a nested module, then restore after.
    pub fn save_execution_state(&mut self) -> SavedExecutionState {
        SavedExecutionState {
            eval_stack: std::mem::take(&mut self.eval_stack),
            value_stack: std::mem::take(&mut self.value_stack),
            completion_stack: std::mem::take(&mut self.completion_stack),
            static_imports: std::mem::take(&mut self.static_imports),
            static_import_index: self.static_import_index,
            pending_slot: self.pending_slot.take(),
            pending_program_body: self.pending_program_body.take(),
            exports: std::mem::take(&mut self.exports),
        }
    }

    /// Restore a previously saved execution state
    ///
    /// Call this after a nested module has finished executing.
    pub fn restore_execution_state(&mut self, state: SavedExecutionState) {
        self.eval_stack = state.eval_stack;
        self.value_stack = state.value_stack;
        self.completion_stack = state.completion_stack;
        self.static_imports = state.static_imports;
        self.static_import_index = state.static_import_index;
        self.pending_slot = state.pending_slot;
        self.pending_program_body = state.pending_program_body;
        self.exports = state.exports;
    }

    /// Collect static import declarations from statements
    fn collect_static_imports(&mut self, statements: &[Statement]) {
        use crate::ast::ImportSpecifier;

        for stmt in statements {
            if let Statement::Import(import_decl) = stmt {
                // Skip type-only imports
                if import_decl.type_only {
                    continue;
                }

                let specifier = import_decl.source.value.clone();

                // Convert import specifiers to bindings
                let bindings = if import_decl.specifiers.is_empty() {
                    // Side-effect only import: import './module'
                    eval_stack::ImportBindings::SideEffect
                } else {
                    let mut named = Vec::new();
                    let mut default_local = None;
                    let mut namespace_local = None;

                    for spec in &import_decl.specifiers {
                        match spec {
                            ImportSpecifier::Named {
                                local, imported, ..
                            } => {
                                // String clones - needed to store in ImportBindings
                                named.push((imported.name.clone(), local.name.clone()));
                            }
                            ImportSpecifier::Default { local, .. } => {
                                default_local = Some(local.name.clone()); // String clone
                            }
                            ImportSpecifier::Namespace { local, .. } => {
                                namespace_local = Some(local.name.clone()); // String clone
                            }
                        }
                    }

                    if let Some(local) = namespace_local {
                        eval_stack::ImportBindings::Namespace(local)
                    } else if let Some(local) = default_local {
                        if named.is_empty() {
                            eval_stack::ImportBindings::Default(local)
                        } else {
                            // Has both default and named - treat as named with "default" key
                            let default_key = self.intern("default");
                            named.insert(0, (default_key, local));
                            eval_stack::ImportBindings::Named(named)
                        }
                    } else {
                        eval_stack::ImportBindings::Named(named)
                    }
                };

                self.static_imports.push(StaticImport {
                    specifier: specifier.to_string(),
                    bindings,
                });
            }
        }
    }

    /// Process the next import or start program execution
    fn process_next_import_or_execute(&mut self) -> Result<crate::RuntimeResult, JsError> {
        // Check if there are more imports to process
        if let Some(import) = self.static_imports.get(self.static_import_index) {
            // String clone - specifier needed for both self and return value
            let specifier = import.specifier.clone();
            let slot = crate::PendingSlot::new(self.generate_slot_id());

            self.static_import_index += 1;
            self.pending_slot = Some(slot.cheap_clone());

            return Ok(crate::RuntimeResult::ImportAwaited { slot, specifier });
        }

        // All imports resolved - start program execution
        if let Some(stmts) = self.pending_program_body.take() {
            if !stmts.is_empty() {
                self.eval_stack.push(eval_stack::EvalFrame::ExecuteProgram {
                    statements: stmts,
                    index: 0,
                });
            }
        }

        // Run until completion or suspension
        self.run_stack()
    }

    /// Bind import values to the environment
    fn bind_import(
        &mut self,
        bindings: &eval_stack::ImportBindings,
        module_value: JsValue,
    ) -> Result<(), JsError> {
        match bindings {
            eval_stack::ImportBindings::Named(pairs) => {
                // module_value is an object with exports as properties
                let JsValue::Object(module_obj) = &module_value else {
                    return Err(JsError::type_error("Module is not an object"));
                };

                for (imported, local) in pairs {
                    let key = self.key(imported.as_ref());
                    let value = module_obj
                        .borrow()
                        .get_property(&key)
                        .unwrap_or(JsValue::Undefined);
                    // String clone - env.define takes ownership
                    self.env_define(local.clone(), value, false);
                }
            }
            eval_stack::ImportBindings::Default(local) => {
                // Get 'default' export from module
                let JsValue::Object(module_obj) = &module_value else {
                    return Err(JsError::type_error("Module is not an object"));
                };

                let default_key = self.key("default");
                let value = module_obj
                    .borrow()
                    .get_property(&default_key)
                    .unwrap_or(JsValue::Undefined);
                // String clone - env.define takes ownership
                self.env_define(local.clone(), value, false);
            }
            eval_stack::ImportBindings::Namespace(local) => {
                // String clone - env.define takes ownership
                // Bind the entire module object
                self.env_define(local.clone(), module_value, false);
            }
            eval_stack::ImportBindings::SideEffect => {
                // No bindings needed, just executed for side effects
            }
        }
        Ok(())
    }

    /// Check if execution has exceeded the timeout
    ///
    /// Returns an error if the timeout has been exceeded, otherwise Ok(()).
    /// If timeout_ms is 0, the timeout is disabled.
    fn check_timeout(&self) -> Result<(), JsError> {
        // Skip check if timeout is disabled
        if self.timeout_ms == 0 {
            return Ok(());
        }

        if let Some(start) = self.execution_start {
            let elapsed = start.elapsed();
            let elapsed_ms = elapsed.as_millis() as u64;
            if elapsed_ms > self.timeout_ms {
                return Err(JsError::Timeout {
                    timeout_ms: self.timeout_ms,
                    elapsed_ms,
                });
            }
        }
        Ok(())
    }

    /// Main execution loop for stack-based evaluation
    ///
    /// Processes frames from eval_stack until:
    /// - Stack is empty (returns Complete)
    /// - Suspension point reached (returns ImportAwaited/AsyncAwaited)
    /// - Timeout exceeded (returns error)
    /// - Error occurs
    fn run_stack(&mut self) -> Result<crate::RuntimeResult, JsError> {
        while let Some(frame) = self.eval_stack.pop() {
            // Check for timeout before processing each frame
            self.check_timeout()?;

            match self.process_frame(frame)? {
                FrameResult::Continue => continue,
                FrameResult::Suspend(result) => return Ok(result),
            }
        }

        // Stack is empty - execution complete
        let result = self.value_stack.pop().unwrap_or(JsValue::Undefined);
        Ok(crate::RuntimeResult::Complete(result))
    }

    /// Process a single evaluation frame
    fn process_frame(&mut self, frame: eval_stack::EvalFrame) -> Result<FrameResult, JsError> {
        use eval_stack::EvalFrame;

        match frame {
            EvalFrame::ExecuteProgram { statements, index } => {
                self.process_execute_program(statements, index)
            }

            EvalFrame::ExecuteStmt(stmt) => self.process_execute_stmt(*stmt),

            EvalFrame::EvaluateExpr(expr) => self.process_evaluate_expr(*expr),

            // For frames not yet converted, use existing recursive methods
            // These will be implemented as we convert more expression/statement types
            _ => {
                // This is a fallback for frames not yet implemented
                // Should not reach here in normal operation with current hybrid approach
                Err(JsError::type_error(format!(
                    "Unhandled frame type in stack execution: {:?}",
                    std::mem::discriminant(&frame)
                )))
            }
        }
    }

    /// Process ExecuteProgram frame
    fn process_execute_program(
        &mut self,
        statements: Vec<Statement>,
        index: usize,
    ) -> Result<FrameResult, JsError> {
        if index >= statements.len() {
            // All statements executed - result is on value_stack or Undefined
            if self.value_stack.is_empty() {
                self.value_stack.push(JsValue::Undefined);
            }
            return Ok(FrameResult::Continue);
        }

        // Push frame for remaining statements
        if index + 1 < statements.len() {
            self.eval_stack.push(eval_stack::EvalFrame::ExecuteProgram {
                statements: statements.clone(),
                index: index + 1,
            });
        }

        // Execute current statement using existing method (hybrid approach)
        let stmt = statements
            .get(index)
            .ok_or_else(|| JsError::internal_error("statement index out of bounds"))?;
        match self.execute_statement(stmt)? {
            Completion::Normal(val) => {
                // Replace top of value stack with new value
                self.value_stack.clear();
                self.value_stack.push(val);
            }
            Completion::Return(val) => {
                // Clear remaining program execution and return
                self.eval_stack
                    .retain(|f| !matches!(f, eval_stack::EvalFrame::ExecuteProgram { .. }));
                self.value_stack.clear();
                self.value_stack.push(val);
            }
            Completion::Break(_) => {
                return Err(JsError::syntax_error("Illegal break statement", 0, 0));
            }
            Completion::Continue(_) => {
                return Err(JsError::syntax_error("Illegal continue statement", 0, 0));
            }
        }

        Ok(FrameResult::Continue)
    }

    /// Process ExecuteStmt frame
    fn process_execute_stmt(&mut self, stmt: Statement) -> Result<FrameResult, JsError> {
        // Use existing execute_statement method (hybrid approach)
        match self.execute_statement(&stmt)? {
            Completion::Normal(val) => {
                self.value_stack.push(val);
            }
            Completion::Return(val) => {
                // TODO: Handle return properly in stack context
                self.value_stack.push(val);
            }
            Completion::Break(_) | Completion::Continue(_) => {
                // TODO: Handle break/continue in stack context
            }
        }
        Ok(FrameResult::Continue)
    }

    /// Process EvaluateExpr frame
    fn process_evaluate_expr(&mut self, expr: Expression) -> Result<FrameResult, JsError> {
        // For now, delegate all expressions to existing evaluate method (hybrid approach)
        // This will be gradually converted to pure stack-based execution
        let value = self.evaluate(&expr)?;
        self.value_stack.push(value);
        Ok(FrameResult::Continue)
    }

    /// Generate a unique slot ID
    pub fn generate_slot_id(&mut self) -> u64 {
        let id = self.next_slot_id;
        self.next_slot_id += 1;
        id
    }

    /// Execute a statement
    pub fn execute_statement(&mut self, stmt: &Statement) -> Result<Completion, JsError> {
        match stmt {
            Statement::Expression(expr) => {
                let value = self.evaluate(&expr.expression)?;
                Ok(Completion::Normal(value))
            }

            Statement::VariableDeclaration(decl) => {
                self.execute_variable_declaration(decl)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::FunctionDeclaration(decl) => {
                self.execute_function_declaration(decl)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Block(block) => self.execute_block(block),

            Statement::If(if_stmt) => {
                let test = self.evaluate(&if_stmt.test)?;
                if test.to_boolean() {
                    self.execute_statement(&if_stmt.consequent)
                } else if let Some(alt) = &if_stmt.alternate {
                    self.execute_statement(alt)
                } else {
                    Ok(Completion::Normal(JsValue::Undefined))
                }
            }

            Statement::While(while_stmt) => {
                loop {
                    // Check for timeout at each iteration
                    self.check_timeout()?;

                    let test = self.evaluate(&while_stmt.test)?;
                    if !test.to_boolean() {
                        break;
                    }

                    match self.execute_statement(&while_stmt.body)? {
                        Completion::Break(None) => break,
                        Completion::Break(label) => return Ok(Completion::Break(label)),
                        Completion::Continue(None) => continue,
                        Completion::Continue(label) => return Ok(Completion::Continue(label)),
                        Completion::Return(val) => return Ok(Completion::Return(val)),
                        Completion::Normal(_) => {}
                    }
                }
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::DoWhile(do_while) => {
                loop {
                    // Check for timeout at each iteration
                    self.check_timeout()?;

                    match self.execute_statement(&do_while.body)? {
                        Completion::Break(None) => break,
                        Completion::Break(label) => return Ok(Completion::Break(label)),
                        Completion::Continue(None) => {}
                        Completion::Continue(label) => return Ok(Completion::Continue(label)),
                        Completion::Return(val) => return Ok(Completion::Return(val)),
                        Completion::Normal(_) => {}
                    }

                    let test = self.evaluate(&do_while.test)?;
                    if !test.to_boolean() {
                        break;
                    }
                }
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::For(for_stmt) => self.execute_for(for_stmt),

            Statement::ForIn(for_in) => self.execute_for_in(for_in),

            Statement::ForOf(for_of) => self.execute_for_of(for_of),

            Statement::Return(ret) => {
                let value = if let Some(arg) = &ret.argument {
                    self.evaluate(arg)?
                } else {
                    JsValue::Undefined
                };
                Ok(Completion::Return(value))
            }

            Statement::Break(brk) => Ok(Completion::Break(
                brk.label.as_ref().map(|l| l.name.clone()),
            )),

            Statement::Continue(cont) => Ok(Completion::Continue(
                cont.label.as_ref().map(|l| l.name.clone()),
            )),

            Statement::Throw(throw) => {
                let value = self.evaluate(&throw.argument)?;
                self.thrown_value = Some(value);
                Err(JsError::Thrown)
            }

            Statement::Try(try_stmt) => {
                let result = self.execute_block(&try_stmt.block);

                match result {
                    Ok(completion) => {
                        if let Some(finalizer) = &try_stmt.finalizer {
                            self.execute_block(finalizer)?;
                        }
                        Ok(completion)
                    }
                    Err(err) => {
                        if let Some(handler) = &try_stmt.handler {
                            // Get the error value - either from thrown_value or create from error
                            let error_value = match &err {
                                JsError::Thrown => {
                                    self.thrown_value.take().unwrap_or(JsValue::Undefined)
                                }
                                JsError::ThrownValue { value } => value.clone(),
                                _ => JsValue::from(err.to_string()),
                            };

                            // Bind catch parameter
                            let prev_env = self.env.cheap_clone();
                            self.env = self.env_alloc(Some(self.env.cheap_clone()));

                            if let Some(param) = &handler.param {
                                self.bind_pattern(param, error_value, true)?;
                            }

                            let result = self.execute_block(&handler.body);
                            let catch_env = self.env.cheap_clone();
                            self.env = prev_env;
                            self.gc_space.remove_root(&catch_env);

                            if let Some(finalizer) = &try_stmt.finalizer {
                                self.execute_block(finalizer)?;
                            }

                            result
                        } else if let Some(finalizer) = &try_stmt.finalizer {
                            self.execute_block(finalizer)?;
                            // Re-throw
                            Err(err)
                        } else {
                            Err(err)
                        }
                    }
                }
            }

            Statement::Switch(switch) => {
                let discriminant = self.evaluate(&switch.discriminant)?;
                let mut matched = false;
                let mut default_index = None;

                // Find matching case or default
                for (i, case) in switch.cases.iter().enumerate() {
                    if case.test.is_none() {
                        default_index = Some(i);
                        continue;
                    }

                    if !matched {
                        if let Some(test_expr) = &case.test {
                            let test = self.evaluate(test_expr)?;
                            if discriminant.strict_equals(&test) {
                                matched = true;
                            }
                        }
                    }

                    if matched {
                        for stmt in &case.consequent {
                            match self.execute_statement(stmt)? {
                                Completion::Break(_) => {
                                    return Ok(Completion::Normal(JsValue::Undefined))
                                }
                                Completion::Return(val) => return Ok(Completion::Return(val)),
                                Completion::Continue(label) => {
                                    return Ok(Completion::Continue(label))
                                }
                                Completion::Normal(_) => {}
                            }
                        }
                    }
                }

                // Fall through to default if no match
                if !matched {
                    if let Some(idx) = default_index {
                        for case in switch.cases.iter().skip(idx) {
                            for stmt in &case.consequent {
                                match self.execute_statement(stmt)? {
                                    Completion::Break(_) => {
                                        return Ok(Completion::Normal(JsValue::Undefined))
                                    }
                                    Completion::Return(val) => return Ok(Completion::Return(val)),
                                    Completion::Continue(label) => {
                                        return Ok(Completion::Continue(label))
                                    }
                                    Completion::Normal(_) => {}
                                }
                            }
                        }
                    }
                }

                Ok(Completion::Normal(JsValue::Undefined))
            }

            // TypeScript declarations - no-ops at runtime
            Statement::TypeAlias(_) | Statement::InterfaceDeclaration(_) => {
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::EnumDeclaration(enum_decl) => {
                self.execute_enum(enum_decl)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::ClassDeclaration(class) => {
                self.execute_class_declaration(class)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Import(_) => {
                // Import handling would require module resolution
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Export(export_decl) => {
                self.execute_export(export_decl)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::NamespaceDeclaration(ns) => {
                self.execute_namespace(ns)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Empty | Statement::Debugger => Ok(Completion::Normal(JsValue::Undefined)),

            Statement::Labeled(labeled) => {
                let label_name = labeled.label.name.clone();
                // Execute loop statements with the label so they can handle labeled break/continue
                match labeled.body.as_ref() {
                    Statement::For(for_stmt) => {
                        self.execute_for_labeled(for_stmt, Some(&label_name))
                    }
                    Statement::ForIn(for_in) => {
                        self.execute_for_in_labeled(for_in, Some(&label_name))
                    }
                    Statement::ForOf(for_of) => {
                        self.execute_for_of_labeled(for_of, Some(&label_name))
                    }
                    Statement::While(while_stmt) => {
                        self.execute_while_labeled(while_stmt, Some(&label_name))
                    }
                    Statement::DoWhile(do_while) => {
                        self.execute_do_while_labeled(do_while, Some(&label_name))
                    }
                    _ => {
                        // Non-loop statements - just handle break with matching label
                        match self.execute_statement(&labeled.body)? {
                            Completion::Break(Some(ref l)) if l == &label_name => {
                                Ok(Completion::Normal(JsValue::Undefined))
                            }
                            other => Ok(other),
                        }
                    }
                }
            }
        }
    }

    fn execute_variable_declaration(&mut self, decl: &VariableDeclaration) -> Result<(), JsError> {
        let mutable = decl.kind != VariableKind::Const;
        let is_var = decl.kind == VariableKind::Var;

        for declarator in &decl.declarations {
            let value = if let Some(init) = &declarator.init {
                self.evaluate(init)?
            } else {
                JsValue::Undefined
            };

            if is_var {
                // For var, use set to update the hoisted binding in outer scope
                self.bind_pattern_var(&declarator.id, value)?;
            } else {
                // For let/const, define in current scope
                self.bind_pattern(&declarator.id, value, mutable)?;
            }
        }

        Ok(())
    }

    /// Hoist var declarations to the current scope (function-scoped hoisting)
    /// This defines all var-declared variables as undefined before execution
    fn hoist_var_declarations(&mut self, statements: &[Statement]) {
        for stmt in statements {
            self.hoist_var_in_statement(stmt);
        }
    }

    fn hoist_var_in_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(decl) if decl.kind == VariableKind::Var => {
                for declarator in &decl.declarations {
                    self.hoist_pattern_names(&declarator.id);
                }
            }
            Statement::Block(block) => {
                // Var declarations inside blocks are still hoisted to function scope
                self.hoist_var_declarations(&block.body);
            }
            Statement::If(if_stmt) => {
                self.hoist_var_in_statement(&if_stmt.consequent);
                if let Some(alt) = &if_stmt.alternate {
                    self.hoist_var_in_statement(alt);
                }
            }
            Statement::For(for_stmt) => {
                if let Some(ForInit::Variable(decl)) = &for_stmt.init {
                    if decl.kind == VariableKind::Var {
                        for declarator in &decl.declarations {
                            self.hoist_pattern_names(&declarator.id);
                        }
                    }
                }
                self.hoist_var_in_statement(&for_stmt.body);
            }
            Statement::ForIn(for_in) => {
                if let ForInOfLeft::Variable(decl) = &for_in.left {
                    if decl.kind == VariableKind::Var {
                        for declarator in &decl.declarations {
                            self.hoist_pattern_names(&declarator.id);
                        }
                    }
                }
                self.hoist_var_in_statement(&for_in.body);
            }
            Statement::ForOf(for_of) => {
                if let ForInOfLeft::Variable(decl) = &for_of.left {
                    if decl.kind == VariableKind::Var {
                        for declarator in &decl.declarations {
                            self.hoist_pattern_names(&declarator.id);
                        }
                    }
                }
                self.hoist_var_in_statement(&for_of.body);
            }
            Statement::While(while_stmt) => {
                self.hoist_var_in_statement(&while_stmt.body);
            }
            Statement::DoWhile(do_while) => {
                self.hoist_var_in_statement(&do_while.body);
            }
            Statement::Try(try_stmt) => {
                self.hoist_var_declarations(&try_stmt.block.body);
                if let Some(catch) = &try_stmt.handler {
                    self.hoist_var_declarations(&catch.body.body);
                }
                if let Some(finally) = &try_stmt.finalizer {
                    self.hoist_var_declarations(&finally.body);
                }
            }
            Statement::Switch(switch_stmt) => {
                for case in &switch_stmt.cases {
                    self.hoist_var_declarations(&case.consequent);
                }
            }
            Statement::Labeled(labeled) => {
                self.hoist_var_in_statement(&labeled.body);
            }
            _ => {}
        }
    }

    /// Extract variable names from a pattern and define them as undefined (hoisted)
    fn hoist_pattern_names(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Identifier(id) => {
                // Only hoist if not already defined in this scope
                if !self.env_has_own_binding(&id.name) {
                    self.env_define(id.name.clone(), JsValue::Undefined, true);
                }
            }
            Pattern::Object(obj_pat) => {
                for prop in &obj_pat.properties {
                    match prop {
                        ObjectPatternProperty::KeyValue { value, .. } => {
                            self.hoist_pattern_names(value);
                        }
                        ObjectPatternProperty::Rest(rest) => {
                            self.hoist_pattern_names(&rest.argument);
                        }
                    }
                }
            }
            Pattern::Array(arr_pat) => {
                for pat in arr_pat.elements.iter().flatten() {
                    self.hoist_pattern_names(pat);
                }
            }
            Pattern::Rest(rest) => {
                self.hoist_pattern_names(&rest.argument);
            }
            Pattern::Assignment(assign) => {
                self.hoist_pattern_names(&assign.left);
            }
        }
    }

    fn execute_function_declaration(&mut self, decl: &FunctionDeclaration) -> Result<(), JsError> {
        let func = InterpretedFunction {
            name: decl.id.as_ref().map(|id| id.name.clone()),
            params: Rc::from(decl.params.as_slice()), // Rc wrap for cheap cloning
            body: Rc::new(FunctionBody::Block(decl.body.clone())), // Rc wrap for cheap cloning
            closure: self.env.cheap_clone(),
            source_location: decl.span,
            generator: decl.generator,
            async_: decl.async_,
        };
        let func_obj = self.create_function(JsFunction::Interpreted(func));

        if let Some(id) = &decl.id {
            self.env_define(id.name.clone(), JsValue::Object(func_obj), true);
        }

        Ok(())
    }

    fn execute_class_declaration(&mut self, class: &ClassDeclaration) -> Result<(), JsError> {
        let constructor_fn = self.create_class_constructor(class)?;

        // Bind the class name first (so static blocks can reference it)
        if let Some(id) = &class.id {
            self.env_define(
                id.name.clone(),
                JsValue::Object(constructor_fn.clone()),
                false,
            );
        }

        // Now execute static blocks - they can reference the class name
        for member in &class.body.members {
            if let ClassMember::StaticBlock(block) = member {
                // Execute the static block's statements
                for stmt in &block.body {
                    if let Completion::Return(_) = self.execute_statement(stmt)? {
                        // Static blocks shouldn't have returns, but handle it gracefully
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn create_class_constructor(
        &mut self,
        class: &ClassDeclaration,
    ) -> Result<JsObjectRef, JsError> {
        // Handle extends - evaluate superclass first
        let super_constructor: Option<JsObjectRef> =
            if let Some(super_class_expr) = &class.super_class {
                let super_val = self.evaluate(super_class_expr)?;
                if let JsValue::Object(sc) = super_val {
                    Some(sc)
                } else {
                    return Err(JsError::type_error(
                        "Class extends value is not a constructor",
                    ));
                }
            } else {
                None
            };

        // Create prototype object with guard - it needs to survive GC during
        // method creation which may allocate many function objects
        let prototype_guarded = self.create_object_guarded();

        // If we have a superclass, set up prototype chain
        if let Some(ref super_ctor) = super_constructor {
            let proto_key = self.key("prototype");
            let super_proto = super_ctor.borrow().get_property(&proto_key);
            if let Some(JsValue::Object(sp)) = super_proto {
                prototype_guarded.borrow_mut().prototype = Some(sp);
            }
        }

        // Find constructor and collect methods/properties
        let mut constructor: Option<&ClassConstructor> = None;
        let mut instance_fields: Vec<&ClassProperty> = Vec::new();
        let mut static_fields: Vec<&ClassProperty> = Vec::new();
        let mut instance_methods: Vec<&ClassMethod> = Vec::new();
        let mut static_methods: Vec<&ClassMethod> = Vec::new();

        for member in &class.body.members {
            match member {
                ClassMember::Constructor(ctor) => {
                    constructor = Some(ctor);
                }
                ClassMember::Method(method) => {
                    if method.static_ {
                        static_methods.push(method);
                    } else {
                        instance_methods.push(method);
                    }
                }
                ClassMember::Property(prop) => {
                    if prop.static_ {
                        static_fields.push(prop);
                    } else {
                        instance_fields.push(prop);
                    }
                }
                ClassMember::StaticBlock(_) => {
                    // Static blocks are collected and executed later
                }
            }
        }

        // Collect getters, setters, and regular methods separately
        // We need to combine getters and setters with the same name into one accessor property
        let mut accessors: rustc_hash::FxHashMap<
            JsString,
            (Option<JsObjectRef>, Option<JsObjectRef>),
        > = rustc_hash::FxHashMap::default();
        let mut regular_methods: Vec<(JsString, JsObjectRef)> = Vec::new();

        for method in &instance_methods {
            let method_name: JsString = match &method.key {
                ObjectPropertyKey::Identifier(id) => id.name.clone(),
                ObjectPropertyKey::String(s) => s.value.clone(),
                ObjectPropertyKey::Number(lit) => match &lit.value {
                    LiteralValue::Number(n) => JsString::from(n.to_string()),
                    _ => continue,
                },
                ObjectPropertyKey::Computed(_) => continue, // Skip computed for now
                ObjectPropertyKey::PrivateIdentifier(id) => JsString::from(format!("#{}", id.name)),
            };

            let func = &method.value;
            let interpreted = InterpretedFunction {
                name: Some(method_name.clone()),
                params: Rc::from(func.params.as_slice()), // Rc wrap for cheap cloning
                body: Rc::new(FunctionBody::Block(func.body.clone())), // Rc wrap for cheap cloning
                closure: self.env.cheap_clone(),
                source_location: func.span,
                generator: func.generator,
                async_: func.async_,
            };
            let func_obj = self.create_function(JsFunction::Interpreted(interpreted));

            // If we have a superclass, store __super__ on the method so super.method() works
            if let Some(ref super_ctor) = super_constructor {
                let super_key = self.key("__super__");
                func_obj
                    .borrow_mut()
                    .set_property(super_key, JsValue::Object(super_ctor.cheap_clone()));
            }

            match method.kind {
                MethodKind::Get => {
                    let entry = accessors.entry(method_name).or_insert((None, None));
                    entry.0 = Some(func_obj);
                }
                MethodKind::Set => {
                    let entry = accessors.entry(method_name).or_insert((None, None));
                    entry.1 = Some(func_obj);
                }
                MethodKind::Method => {
                    regular_methods.push((method_name, func_obj));
                }
            }
        }

        // Add accessor properties
        for (name, (getter, setter)) in accessors {
            prototype_guarded.borrow_mut().define_property(
                PropertyKey::String(name),
                Property::accessor(getter, setter),
            );
        }

        // Add regular methods
        for (name, func_obj) in regular_methods {
            prototype_guarded
                .borrow_mut()
                .set_property(PropertyKey::String(name), JsValue::Object(func_obj));
        }

        // Build constructor body that initializes instance fields then runs user constructor
        // We store instance fields info in the constructor function
        let field_initializers: Vec<(JsString, Option<Expression>)> = instance_fields
            .iter()
            .filter_map(|prop| {
                let name: JsString = match &prop.key {
                    ObjectPropertyKey::Identifier(id) => id.name.clone(),
                    ObjectPropertyKey::String(s) => s.value.clone(),
                    ObjectPropertyKey::PrivateIdentifier(id) => {
                        JsString::from(format!("#{}", id.name))
                    }
                    _ => return None,
                };
                Some((name, prop.value.clone()))
            })
            .collect();

        // Create the constructor function with field initializers stored in closure
        let ctor_body = if let Some(ctor) = constructor {
            ctor.body.clone()
        } else {
            // Default constructor - empty body
            BlockStatement {
                body: vec![],
                span: class.span,
            }
        };

        let ctor_params = if let Some(ctor) = constructor {
            ctor.params.clone()
        } else {
            vec![]
        };

        // Store field initializers in a special property so evaluate_new can access them
        let constructor_fn = self.create_function(JsFunction::Interpreted(InterpretedFunction {
            name: class.id.as_ref().map(|id| id.name.clone()),
            params: Rc::from(ctor_params), // Rc wrap for cheap cloning
            body: Rc::new(FunctionBody::Block(ctor_body)), // Rc wrap for cheap cloning
            closure: self.env.cheap_clone(),
            source_location: class.span,
            generator: false, // Constructors cannot be generators
            async_: false,    // Constructors cannot be async
        }));

        // Guard the constructor_fn to prevent GC from collecting it during
        // field initializer evaluation and static method creation
        let _constructor_guard = self.gc_space.guard(&constructor_fn);

        // Store prototype on constructor - this makes prototype reachable from constructor
        {
            let proto_key = self.key("prototype");
            let mut ctor = constructor_fn.borrow_mut();
            ctor.set_property(proto_key, JsValue::Object(prototype_guarded.take()));

            // Store field initializers as internal data
            // For now, we'll evaluate them at class definition time and store as default values
        }

        // Store field info that will be evaluated at construction time
        // We need a way to pass this to the new operator
        // For now, let's store the field expressions in a special way
        if !field_initializers.is_empty() {
            // First, evaluate all field values
            let mut field_values: Vec<(JsString, JsValue)> = Vec::new();
            for (name, value_expr) in field_initializers {
                let value = if let Some(expr) = value_expr {
                    self.evaluate(&expr).unwrap_or(JsValue::Undefined)
                } else {
                    JsValue::Undefined
                };
                field_values.push((name, value));
            }

            // Then create the fields array
            let mut field_pairs: Vec<JsValue> = Vec::new();
            for (name, value) in field_values {
                let pair = self.create_array(vec![JsValue::String(name), value]);
                field_pairs.push(JsValue::Object(pair));
            }

            let fields_array = self.create_array(field_pairs);
            let fields_key = self.key("__fields__");
            constructor_fn
                .borrow_mut()
                .set_property(fields_key, JsValue::Object(fields_array));
        }

        // Store super constructor if we have one
        if let Some(ref super_ctor) = super_constructor {
            let super_key = self.key("__super__");
            constructor_fn
                .borrow_mut()
                .set_property(super_key, JsValue::Object(super_ctor.cheap_clone()));
        }

        // Collect static getters, setters, and regular methods separately
        let mut static_accessors: rustc_hash::FxHashMap<
            JsString,
            (Option<JsObjectRef>, Option<JsObjectRef>),
        > = rustc_hash::FxHashMap::default();
        let mut static_regular_methods: Vec<(JsString, JsObjectRef)> = Vec::new();

        for method in &static_methods {
            let method_name: JsString = match &method.key {
                ObjectPropertyKey::Identifier(id) => id.name.clone(),
                ObjectPropertyKey::String(s) => s.value.clone(),
                ObjectPropertyKey::Number(lit) => match &lit.value {
                    LiteralValue::Number(n) => JsString::from(n.to_string()),
                    _ => continue,
                },
                ObjectPropertyKey::Computed(_) => continue,
                ObjectPropertyKey::PrivateIdentifier(id) => JsString::from(format!("#{}", id.name)),
            };

            let func = &method.value;
            let interpreted = InterpretedFunction {
                name: Some(method_name.clone()),
                params: Rc::from(func.params.as_slice()), // Rc wrap for cheap cloning
                body: Rc::new(FunctionBody::Block(func.body.clone())), // Rc wrap for cheap cloning
                closure: self.env.cheap_clone(),
                source_location: func.span,
                generator: func.generator,
                async_: func.async_,
            };
            let func_obj = self.create_function(JsFunction::Interpreted(interpreted));

            match method.kind {
                MethodKind::Get => {
                    let entry = static_accessors.entry(method_name).or_insert((None, None));
                    entry.0 = Some(func_obj);
                }
                MethodKind::Set => {
                    let entry = static_accessors.entry(method_name).or_insert((None, None));
                    entry.1 = Some(func_obj);
                }
                MethodKind::Method => {
                    static_regular_methods.push((method_name, func_obj));
                }
            }
        }

        // Add static accessor properties
        for (name, (getter, setter)) in static_accessors {
            constructor_fn.borrow_mut().define_property(
                PropertyKey::String(name),
                Property::accessor(getter, setter),
            );
        }

        // Add static regular methods
        for (name, func_obj) in static_regular_methods {
            constructor_fn
                .borrow_mut()
                .set_property(PropertyKey::String(name), JsValue::Object(func_obj));
        }

        // Initialize static fields
        for prop in &static_fields {
            let name = match &prop.key {
                ObjectPropertyKey::Identifier(id) => id.name.clone(),
                ObjectPropertyKey::String(s) => s.value.clone(),
                // Private fields are stored with # prefix to match how they're looked up
                ObjectPropertyKey::PrivateIdentifier(id) => JsString::from(format!("#{}", id.name)),
                _ => continue,
            };

            let value = if let Some(expr) = &prop.value {
                self.evaluate(expr)?
            } else {
                JsValue::Undefined
            };

            constructor_fn
                .borrow_mut()
                .set_property(PropertyKey::String(name), value);
        }

        // Set prototype.constructor = constructor
        // Get the prototype from the constructor since we already stored it there
        let proto_key = self.key("prototype");
        let ctor_key = self.key("constructor");
        if let Some(JsValue::Object(prototype)) = constructor_fn.borrow().get_property(&proto_key) {
            prototype
                .borrow_mut()
                .set_property(ctor_key, JsValue::Object(constructor_fn.cheap_clone()));
        }

        Ok(constructor_fn)
    }

    fn execute_enum(&mut self, enum_decl: &EnumDeclaration) -> Result<(), JsError> {
        let obj = self.create_object();
        let mut next_value = 0i32;

        // Collect member names for cleanup after enum processing
        let mut member_names: Vec<JsString> = Vec::new();

        for member in &enum_decl.members {
            let value = if let Some(init) = &member.initializer {
                let val = self.evaluate(init)?;
                if let JsValue::Number(n) = val {
                    next_value = n as i32 + 1;
                }
                val
            } else {
                let val = JsValue::Number(next_value as f64);
                next_value += 1;
                val
            };

            // Forward mapping: name -> value
            obj.borrow_mut()
                .set_property(PropertyKey::String(member.id.name.clone()), value.clone());

            // Reverse mapping for numeric enums: value -> name
            if let JsValue::Number(n) = &value {
                let key = self.key(&n.to_string());
                obj.borrow_mut()
                    .set_property(key, JsValue::String(member.id.name.clone()));
            }

            // Define each member in current scope so later members can reference it
            // (e.g., ReadWrite = Read | Write)
            self.env_define(member.id.name.clone(), value, false);
            member_names.push(member.id.name.clone());
        }

        // Remove temporary member bindings from scope
        for name in member_names {
            self.env_delete(&name);
        }

        self.env_define(enum_decl.id.name.clone(), JsValue::Object(obj), false);
        Ok(())
    }

    fn execute_namespace(&mut self, ns: &NamespaceDeclaration) -> Result<(), JsError> {
        let name = ns.id.name.clone();

        // Check if namespace already exists (for merging)
        let ns_obj = if let Ok(JsValue::Object(obj)) = self.env_get_binding(&name) {
            obj
        } else {
            self.create_object()
        };

        // Save current exports and create new scope for namespace
        let saved_exports = std::mem::take(&mut self.exports);
        let saved_env = self.env.cheap_clone();
        self.env = self.env_alloc(Some(self.env.cheap_clone()));

        // Execute statements in namespace body
        for stmt in &ns.body {
            // Handle export statements specially
            if let Statement::Export(export_decl) = stmt {
                self.execute_namespace_export(export_decl, &ns_obj)?;
            } else {
                self.execute_statement(stmt)?;
            }
        }

        // Restore environment and exports
        self.env = saved_env;
        self.exports = saved_exports;

        // Define the namespace in the current environment
        self.env_define(name, JsValue::Object(ns_obj), false);
        Ok(())
    }

    fn execute_namespace_export(
        &mut self,
        export_decl: &ExportDeclaration,
        ns_obj: &JsObjectRef,
    ) -> Result<(), JsError> {
        if let Some(declaration) = &export_decl.declaration {
            match declaration.as_ref() {
                Statement::FunctionDeclaration(func_decl) => {
                    self.execute_function_declaration(func_decl)?;
                    if let Some(id) = &func_decl.id {
                        let value = self.env_get_binding(&id.name)?;
                        ns_obj
                            .borrow_mut()
                            .set_property(PropertyKey::String(id.name.clone()), value);
                    }
                }
                Statement::VariableDeclaration(var_decl) => {
                    self.execute_variable_declaration(var_decl)?;
                    // Extract names from declarations
                    for decl in &var_decl.declarations {
                        self.export_pattern_to_namespace(&decl.id, ns_obj)?;
                    }
                }
                Statement::ClassDeclaration(class_decl) => {
                    self.execute_class_declaration(class_decl)?;
                    if let Some(id) = &class_decl.id {
                        let value = self.env_get_binding(&id.name)?;
                        ns_obj
                            .borrow_mut()
                            .set_property(PropertyKey::String(id.name.clone()), value);
                    }
                }
                Statement::NamespaceDeclaration(inner_ns) => {
                    self.execute_namespace(inner_ns)?;
                    let value = self.env_get_binding(&inner_ns.id.name)?;
                    ns_obj
                        .borrow_mut()
                        .set_property(PropertyKey::String(inner_ns.id.name.clone()), value);
                }
                _ => {
                    self.execute_statement(declaration)?;
                }
            }
        }
        Ok(())
    }

    fn export_pattern_to_namespace(
        &self,
        pattern: &Pattern,
        ns_obj: &JsObjectRef,
    ) -> Result<(), JsError> {
        match pattern {
            Pattern::Identifier(id) => {
                let value = self.env_get_binding(&id.name)?;
                ns_obj
                    .borrow_mut()
                    .set_property(PropertyKey::String(id.name.clone()), value);
            }
            Pattern::Object(obj_pat) => {
                for prop in &obj_pat.properties {
                    match prop {
                        ObjectPatternProperty::KeyValue { value, .. } => {
                            self.export_pattern_to_namespace(value, ns_obj)?;
                        }
                        ObjectPatternProperty::Rest(rest) => {
                            self.export_pattern_to_namespace(&rest.argument, ns_obj)?;
                        }
                    }
                }
            }
            Pattern::Array(arr_pat) => {
                for elem in arr_pat.elements.iter().flatten() {
                    self.export_pattern_to_namespace(elem, ns_obj)?;
                }
            }
            Pattern::Rest(rest) => {
                self.export_pattern_to_namespace(&rest.argument, ns_obj)?;
            }
            Pattern::Assignment(assign_pat) => {
                self.export_pattern_to_namespace(&assign_pat.left, ns_obj)?;
            }
        }
        Ok(())
    }

    fn execute_export(&mut self, export_decl: &ExportDeclaration) -> Result<(), JsError> {
        // Handle export with declaration: export function foo() {}, export const x = 1
        if let Some(declaration) = &export_decl.declaration {
            match declaration.as_ref() {
                Statement::FunctionDeclaration(func_decl) => {
                    self.execute_function_declaration(func_decl)?;
                    if let Some(id) = &func_decl.id {
                        let name = id.name.clone();
                        if let Ok(value) = self.env_get_binding(&name) {
                            self.exports.insert(name, value);
                        }
                    }
                }
                Statement::VariableDeclaration(var_decl) => {
                    self.execute_variable_declaration(var_decl)?;
                    // Export each declared variable
                    for declarator in &var_decl.declarations {
                        let names = self.get_pattern_names(&declarator.id);
                        for name in names {
                            let js_name: JsString = name.into();
                            if let Ok(value) = self.env_get_binding(&js_name) {
                                self.exports.insert(js_name, value);
                            }
                        }
                    }
                }
                Statement::ClassDeclaration(class_decl) => {
                    self.execute_class_declaration(class_decl)?;
                    if let Some(id) = &class_decl.id {
                        let name = id.name.clone();
                        if let Ok(value) = self.env_get_binding(&name) {
                            self.exports.insert(name, value);
                        }
                    }
                }
                Statement::TypeAlias(_) | Statement::InterfaceDeclaration(_) => {
                    // Type-only exports - no runtime effect
                }
                Statement::EnumDeclaration(enum_decl) => {
                    self.execute_enum(enum_decl)?;
                    let name = enum_decl.id.name.clone();
                    if let Ok(value) = self.env_get_binding(&name) {
                        self.exports.insert(name, value);
                    }
                }
                Statement::NamespaceDeclaration(ns_decl) => {
                    self.execute_namespace(ns_decl)?;
                    let name = ns_decl.id.name.clone();
                    if let Ok(value) = self.env_get_binding(&name) {
                        self.exports.insert(name, value);
                    }
                }
                _ => {
                    // Other declarations that we may not support yet
                }
            }
        }

        // Handle export specifiers: export { foo, bar }
        for spec in &export_decl.specifiers {
            let local_name = &spec.local.name;
            let exported_name = &spec.exported.name;
            if let Ok(value) = self.env_get_binding(local_name) {
                self.exports.insert(exported_name.clone(), value);
            }
        }

        // Handle export default
        if export_decl.default {
            if let Some(declaration) = &export_decl.declaration {
                match declaration.as_ref() {
                    // export default function foo() {} - named function
                    Statement::FunctionDeclaration(func_decl) => {
                        self.execute_function_declaration(func_decl)?;
                        if let Some(id) = &func_decl.id {
                            if let Ok(value) = self.env_get_binding(&id.name) {
                                let default_key = self.intern("default");
                                self.exports.insert(default_key, value);
                            }
                        }
                    }
                    // export default class Foo {}
                    Statement::ClassDeclaration(class_decl) => {
                        self.execute_class_declaration(class_decl)?;
                        if let Some(id) = &class_decl.id {
                            if let Ok(value) = self.env_get_binding(&id.name) {
                                let default_key = self.intern("default");
                                self.exports.insert(default_key, value);
                            }
                        }
                    }
                    // export default expression (handled via Expression statement)
                    Statement::Expression(expr_stmt) => {
                        let value = self.evaluate(&expr_stmt.expression)?;
                        let default_key = self.intern("default");
                        self.exports.insert(default_key, value);
                    }
                    _ => {
                        // Other default exports not yet supported
                    }
                }
            }
        }

        Ok(())
    }

    /// Get variable names from a pattern for export tracking
    fn get_pattern_names(&self, pattern: &Pattern) -> Vec<String> {
        let mut names = Vec::new();
        self.collect_pattern_names(pattern, &mut names);
        names
    }

    fn execute_block(&mut self, block: &BlockStatement) -> Result<Completion, JsError> {
        let prev_env = self.env.cheap_clone();
        self.env = self.env_alloc(Some(self.env.cheap_clone()));

        let mut result = Completion::Normal(JsValue::Undefined);

        for stmt in &block.body {
            result = self.execute_statement(stmt)?;
            match &result {
                Completion::Normal(_) => {}
                _ => break,
            }
        }

        let block_env = self.env.cheap_clone();
        self.env = prev_env;
        self.gc_space.remove_root(&block_env);
        Ok(result)
    }

    fn execute_for(&mut self, for_stmt: &ForStatement) -> Result<Completion, JsError> {
        self.execute_for_labeled(for_stmt, None)
    }

    fn execute_for_labeled(
        &mut self,
        for_stmt: &ForStatement,
        label: Option<&JsString>,
    ) -> Result<Completion, JsError> {
        let prev_env = self.env.cheap_clone();
        self.env = self.env_alloc(Some(self.env.cheap_clone()));

        // Track let-declared loop variables for per-iteration binding
        let mut let_var_names: Vec<String> = Vec::new();
        let is_let_loop = if let Some(ForInit::Variable(decl)) = &for_stmt.init {
            decl.kind == VariableKind::Let || decl.kind == VariableKind::Const
        } else {
            false
        };

        // Init
        if let Some(init) = &for_stmt.init {
            match init {
                ForInit::Variable(decl) => {
                    // Collect let/const variable names for per-iteration binding
                    if is_let_loop {
                        for declarator in &decl.declarations {
                            self.collect_pattern_names(&declarator.id, &mut let_var_names);
                        }
                    }
                    self.execute_variable_declaration(decl)?;
                }
                ForInit::Expression(expr) => {
                    self.evaluate(expr)?;
                }
            }
        }

        let loop_env = self.env.cheap_clone();

        // Loop
        loop {
            // Check for timeout at each iteration
            self.check_timeout()?;

            // Test
            if let Some(test) = &for_stmt.test {
                let test_val = self.evaluate(test)?;
                if !test_val.to_boolean() {
                    break;
                }
            }

            // For let/const loops, create per-iteration scope
            let iter_env = if is_let_loop && !let_var_names.is_empty() {
                let iter_env = self.env_alloc(Some(loop_env.cheap_clone()));
                // Copy current values into the per-iteration scope
                for name in &let_var_names {
                    let js_name: JsString = name.clone().into();
                    if let Ok(val) = self.env_get_binding(&js_name) {
                        self.env_define_in(&iter_env, js_name, val, true);
                    }
                }
                self.env = iter_env.cheap_clone();
                Some(iter_env)
            } else {
                None
            };

            // Body
            match self.execute_statement(&for_stmt.body)? {
                Completion::Break(None) => {
                    if let Some(env) = iter_env {
                        self.gc_space.remove_root(&env);
                    }
                    self.env = loop_env.cheap_clone();
                    break;
                }
                Completion::Break(Some(ref l)) if label == Some(l) => {
                    if let Some(env) = iter_env {
                        self.gc_space.remove_root(&env);
                    }
                    self.gc_space.remove_root(&loop_env);
                    self.env = prev_env;
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => {
                    if let Some(env) = iter_env {
                        self.gc_space.remove_root(&env);
                    }
                    self.gc_space.remove_root(&loop_env);
                    self.env = prev_env;
                    return Ok(Completion::Break(lbl));
                }
                Completion::Continue(None) => {}
                Completion::Continue(Some(ref l)) if label == Some(l) => {
                    // Continue with matching label - continue this loop
                }
                Completion::Continue(lbl) => {
                    if let Some(env) = iter_env {
                        self.gc_space.remove_root(&env);
                    }
                    self.gc_space.remove_root(&loop_env);
                    self.env = prev_env;
                    return Ok(Completion::Continue(lbl));
                }
                Completion::Return(val) => {
                    if let Some(env) = iter_env {
                        self.gc_space.remove_root(&env);
                    }
                    self.gc_space.remove_root(&loop_env);
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {}
            }

            // Update - copy values back to loop env, update, then continue
            if is_let_loop && !let_var_names.is_empty() {
                // Copy updated values back to loop env before update
                for name in &let_var_names {
                    let js_name: JsString = name.clone().into();
                    if let Ok(val) = self.env_get_binding(&js_name) {
                        let _ = self.env_set_binding_in(&loop_env, &js_name, val);
                    }
                }
                // Free the per-iteration environment
                if let Some(env) = iter_env {
                    self.gc_space.remove_root(&env);
                }
                self.env = loop_env.cheap_clone();
            }

            if let Some(update) = &for_stmt.update {
                self.evaluate(update)?;
            }
        }

        self.gc_space.remove_root(&loop_env);
        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    /// Collect all variable names from a pattern
    #[allow(clippy::only_used_in_recursion)]
    fn collect_pattern_names(&self, pattern: &Pattern, names: &mut Vec<String>) {
        match pattern {
            Pattern::Identifier(id) => names.push(id.name.to_string()),
            Pattern::Object(obj) => {
                for prop in &obj.properties {
                    match prop {
                        ObjectPatternProperty::KeyValue { value, .. } => {
                            self.collect_pattern_names(value, names);
                        }
                        ObjectPatternProperty::Rest(rest) => {
                            self.collect_pattern_names(&rest.argument, names);
                        }
                    }
                }
            }
            Pattern::Array(arr) => {
                for p in arr.elements.iter().flatten() {
                    self.collect_pattern_names(p, names);
                }
            }
            Pattern::Assignment(assign) => {
                self.collect_pattern_names(&assign.left, names);
            }
            Pattern::Rest(rest) => {
                self.collect_pattern_names(&rest.argument, names);
            }
        }
    }

    fn execute_for_in(&mut self, for_in: &ForInStatement) -> Result<Completion, JsError> {
        self.execute_for_in_labeled(for_in, None)
    }

    fn execute_for_in_labeled(
        &mut self,
        for_in: &ForInStatement,
        label: Option<&JsString>,
    ) -> Result<Completion, JsError> {
        let right = self.evaluate(&for_in.right)?;

        let keys = match &right {
            JsValue::Object(obj) => obj
                .borrow()
                .properties
                .iter()
                .filter(|(key, prop)| prop.enumerable && !key.is_symbol())
                .map(|(key, _)| key.to_string())
                .collect::<Vec<_>>(),
            _ => vec![],
        };

        let prev_env = self.env.cheap_clone();

        for key in keys {
            // Check for timeout at each iteration
            self.check_timeout()?;

            let iter_env = self.env_alloc(Some(prev_env.cheap_clone()));
            self.env = iter_env.cheap_clone();

            let key_value = JsValue::String(JsString::from(key));

            match &for_in.left {
                ForInOfLeft::Variable(decl) => {
                    let mutable = decl.kind != VariableKind::Const;
                    if let Some(declarator) = decl.declarations.first() {
                        self.bind_pattern(&declarator.id, key_value, mutable)?;
                    }
                }
                ForInOfLeft::Pattern(pattern) => {
                    self.bind_pattern(pattern, key_value, true)?;
                }
            }

            match self.execute_statement(&for_in.body)? {
                Completion::Break(None) => {
                    self.gc_space.remove_root(&iter_env);
                    break;
                }
                Completion::Break(Some(ref l)) if label == Some(l) => {
                    self.gc_space.remove_root(&iter_env);
                    self.env = prev_env;
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => {
                    self.gc_space.remove_root(&iter_env);
                    self.env = prev_env;
                    return Ok(Completion::Break(lbl));
                }
                Completion::Continue(None) => {
                    self.gc_space.remove_root(&iter_env);
                    continue;
                }
                Completion::Continue(Some(ref l)) if label == Some(l) => {
                    // Continue with matching label - continue this loop
                    self.gc_space.remove_root(&iter_env);
                    continue;
                }
                Completion::Continue(lbl) => {
                    self.gc_space.remove_root(&iter_env);
                    self.env = prev_env;
                    return Ok(Completion::Continue(lbl));
                }
                Completion::Return(val) => {
                    self.gc_space.remove_root(&iter_env);
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {
                    self.gc_space.remove_root(&iter_env);
                }
            }
        }

        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_for_of(&mut self, for_of: &ForOfStatement) -> Result<Completion, JsError> {
        self.execute_for_of_labeled(for_of, None)
    }

    fn execute_for_of_labeled(
        &mut self,
        for_of: &ForOfStatement,
        label: Option<&JsString>,
    ) -> Result<Completion, JsError> {
        let right = self.evaluate(&for_of.right)?;

        // Check if it's a generator - handle differently since we iterate by calling .next()
        if let JsValue::Object(ref obj) = right {
            let is_generator = matches!(obj.borrow().exotic, ExoticObject::Generator(_));
            if is_generator {
                return self.execute_for_of_generator(for_of, label, obj.cheap_clone());
            }
        }

        // For non-generators, collect items upfront
        let items = match &right {
            JsValue::Object(obj) => {
                let obj_ref = obj.borrow();
                match &obj_ref.exotic {
                    ExoticObject::Array { length } => {
                        let mut items = Vec::with_capacity(*length as usize);
                        for i in 0..*length {
                            if let Some(val) = obj_ref.get_property(&PropertyKey::Index(i)) {
                                items.push(val);
                            } else {
                                items.push(JsValue::Undefined);
                            }
                        }
                        items
                    }
                    _ => vec![],
                }
            }
            JsValue::String(s) => s
                .as_str()
                .chars()
                .map(|c| JsValue::from(c.to_string()))
                .collect(),
            _ => vec![],
        };

        let prev_env = self.env.cheap_clone();

        for item in items {
            // Check for timeout at each iteration
            self.check_timeout()?;

            let iter_env = self.env_alloc(Some(prev_env.cheap_clone()));
            self.env = iter_env.cheap_clone();

            match &for_of.left {
                ForInOfLeft::Variable(decl) => {
                    let mutable = decl.kind != VariableKind::Const;
                    if let Some(declarator) = decl.declarations.first() {
                        self.bind_pattern(&declarator.id, item, mutable)?;
                    }
                }
                ForInOfLeft::Pattern(pattern) => {
                    self.bind_pattern(pattern, item, true)?;
                }
            }

            match self.execute_statement(&for_of.body)? {
                Completion::Break(None) => {
                    self.gc_space.remove_root(&iter_env);
                    break;
                }
                Completion::Break(Some(ref l)) if label == Some(l) => {
                    self.gc_space.remove_root(&iter_env);
                    self.env = prev_env;
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => {
                    self.gc_space.remove_root(&iter_env);
                    self.env = prev_env;
                    return Ok(Completion::Break(lbl));
                }
                Completion::Continue(None) => {
                    self.gc_space.remove_root(&iter_env);
                    continue;
                }
                Completion::Continue(Some(ref l)) if label == Some(l) => {
                    // Continue with matching label - continue this loop
                    self.gc_space.remove_root(&iter_env);
                    continue;
                }
                Completion::Continue(lbl) => {
                    self.gc_space.remove_root(&iter_env);
                    self.env = prev_env;
                    return Ok(Completion::Continue(lbl));
                }
                Completion::Return(val) => {
                    self.gc_space.remove_root(&iter_env);
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {
                    self.gc_space.remove_root(&iter_env);
                }
            }
        }

        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    /// Execute for...of over a generator object by calling .next() repeatedly
    fn execute_for_of_generator(
        &mut self,
        for_of: &ForOfStatement,
        label: Option<&JsString>,
        gen_obj: JsObjectRef,
    ) -> Result<Completion, JsError> {
        let prev_env = self.env.cheap_clone();

        loop {
            // Check for timeout at each iteration
            self.check_timeout()?;

            // Call generator.next()
            let next_result = builtins::generator::generator_next(
                self,
                JsValue::Object(gen_obj.cheap_clone()),
                &[],
            )?;

            // Get 'done' and 'value' from the result
            let done_key = self.key("done");
            let value_key = self.key("value");
            let (done, value) = if let JsValue::Object(result_obj) = next_result {
                let result_ref = result_obj.borrow();
                let done = result_ref
                    .get_property(&done_key)
                    .unwrap_or(JsValue::Boolean(false));
                let value = result_ref
                    .get_property(&value_key)
                    .unwrap_or(JsValue::Undefined);
                (done.to_boolean(), value)
            } else {
                (true, JsValue::Undefined)
            };

            // If done, exit the loop
            if done {
                break;
            }

            // Bind the value to the loop variable
            let iter_env = self.env_alloc(Some(prev_env.cheap_clone()));
            self.env = iter_env.cheap_clone();

            match &for_of.left {
                ForInOfLeft::Variable(decl) => {
                    let mutable = decl.kind != VariableKind::Const;
                    if let Some(declarator) = decl.declarations.first() {
                        self.bind_pattern(&declarator.id, value, mutable)?;
                    }
                }
                ForInOfLeft::Pattern(pattern) => {
                    self.bind_pattern(pattern, value, true)?;
                }
            }

            // Execute the loop body
            match self.execute_statement(&for_of.body)? {
                Completion::Break(None) => {
                    self.gc_space.remove_root(&iter_env);
                    break;
                }
                Completion::Break(Some(ref l)) if label == Some(l) => {
                    self.gc_space.remove_root(&iter_env);
                    self.env = prev_env;
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => {
                    self.gc_space.remove_root(&iter_env);
                    self.env = prev_env;
                    return Ok(Completion::Break(lbl));
                }
                Completion::Continue(None) => {
                    self.gc_space.remove_root(&iter_env);
                    continue;
                }
                Completion::Continue(Some(ref l)) if label == Some(l) => {
                    self.gc_space.remove_root(&iter_env);
                    continue;
                }
                Completion::Continue(lbl) => {
                    self.gc_space.remove_root(&iter_env);
                    self.env = prev_env;
                    return Ok(Completion::Continue(lbl));
                }
                Completion::Return(val) => {
                    self.gc_space.remove_root(&iter_env);
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {
                    self.gc_space.remove_root(&iter_env);
                }
            }
        }

        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_while_labeled(
        &mut self,
        while_stmt: &WhileStatement,
        label: Option<&JsString>,
    ) -> Result<Completion, JsError> {
        loop {
            // Check for timeout at each iteration
            self.check_timeout()?;

            let test = self.evaluate(&while_stmt.test)?;
            if !test.to_boolean() {
                break;
            }

            match self.execute_statement(&while_stmt.body)? {
                Completion::Break(None) => break,
                Completion::Break(Some(ref l)) if label == Some(l) => {
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => return Ok(Completion::Break(lbl)),
                Completion::Continue(None) => continue,
                Completion::Continue(Some(ref l)) if label == Some(l) => {
                    // Continue with matching label - continue this loop
                    continue;
                }
                Completion::Continue(lbl) => return Ok(Completion::Continue(lbl)),
                Completion::Return(val) => return Ok(Completion::Return(val)),
                Completion::Normal(_) => {}
            }
        }
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_do_while_labeled(
        &mut self,
        do_while: &DoWhileStatement,
        label: Option<&JsString>,
    ) -> Result<Completion, JsError> {
        loop {
            // Check for timeout at each iteration
            self.check_timeout()?;

            match self.execute_statement(&do_while.body)? {
                Completion::Break(None) => break,
                Completion::Break(Some(ref l)) if label == Some(l) => {
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => return Ok(Completion::Break(lbl)),
                Completion::Continue(None) => {}
                Completion::Continue(Some(ref l)) if label == Some(l) => {
                    // Continue with matching label - skip to test
                }
                Completion::Continue(lbl) => return Ok(Completion::Continue(lbl)),
                Completion::Return(val) => return Ok(Completion::Return(val)),
                Completion::Normal(_) => {}
            }

            let test = self.evaluate(&do_while.test)?;
            if !test.to_boolean() {
                break;
            }
        }
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn bind_pattern(
        &mut self,
        pattern: &Pattern,
        value: JsValue,
        mutable: bool,
    ) -> Result<(), JsError> {
        match pattern {
            Pattern::Identifier(id) => {
                // String clone - env.define takes ownership
                self.env_define(id.name.clone(), value, mutable);
                Ok(())
            }

            Pattern::Object(obj_pattern) => {
                let obj = match &value {
                    JsValue::Object(o) => o.cheap_clone(),
                    _ => return Err(JsError::type_error("Cannot destructure non-object")),
                };

                for prop in &obj_pattern.properties {
                    match prop {
                        ObjectPatternProperty::KeyValue {
                            key,
                            value: pattern,
                            ..
                        } => {
                            let key_str: JsString = match key {
                                ObjectPropertyKey::Identifier(id) => id.name.cheap_clone(),
                                ObjectPropertyKey::String(s) => s.value.cheap_clone(),
                                ObjectPropertyKey::Number(l) => {
                                    if let LiteralValue::Number(n) = &l.value {
                                        n.to_string().into()
                                    } else {
                                        continue;
                                    }
                                }
                                ObjectPropertyKey::Computed(_) => continue,
                                ObjectPropertyKey::PrivateIdentifier(id) => {
                                    format!("#{}", id.name).into()
                                }
                            };

                            let prop_value = obj
                                .borrow()
                                .get_property(&PropertyKey::String(key_str))
                                .unwrap_or(JsValue::Undefined);

                            self.bind_pattern(pattern, prop_value, mutable)?;
                        }
                        ObjectPatternProperty::Rest(rest) => {
                            // Collect remaining properties
                            let rest_obj = self.create_object();
                            // Simplified - would need to track which keys were already destructured
                            self.bind_pattern(&rest.argument, JsValue::Object(rest_obj), mutable)?;
                        }
                    }
                }

                Ok(())
            }

            Pattern::Array(arr_pattern) => {
                let items: Vec<JsValue> = match &value {
                    JsValue::Object(obj) => {
                        let obj_ref = obj.borrow();
                        if let ExoticObject::Array { length } = &obj_ref.exotic {
                            let mut items = Vec::with_capacity(*length as usize);
                            for i in 0..*length {
                                items.push(
                                    obj_ref
                                        .get_property(&PropertyKey::Index(i))
                                        .unwrap_or(JsValue::Undefined),
                                );
                            }
                            items
                        } else {
                            vec![]
                        }
                    }
                    _ => vec![],
                };

                for (i, elem) in arr_pattern.elements.iter().enumerate() {
                    if let Some(pattern) = elem {
                        match pattern {
                            Pattern::Rest(rest) => {
                                let remaining: Vec<JsValue> =
                                    items.iter().skip(i).cloned().collect();
                                let rest_array = self.create_array(remaining);
                                self.bind_pattern(
                                    &rest.argument,
                                    JsValue::Object(rest_array),
                                    mutable,
                                )?;
                                break;
                            }
                            _ => {
                                let val = items.get(i).cloned().unwrap_or(JsValue::Undefined);
                                self.bind_pattern(pattern, val, mutable)?;
                            }
                        }
                    }
                }

                Ok(())
            }

            Pattern::Assignment(assign) => {
                let val = if value == JsValue::Undefined {
                    self.evaluate(&assign.right)?
                } else {
                    value
                };
                self.bind_pattern(&assign.left, val, mutable)
            }

            Pattern::Rest(rest) => self.bind_pattern(&rest.argument, value, mutable),
        }
    }

    /// Bind a pattern using var semantics (set existing hoisted binding)
    fn bind_pattern_var(&mut self, pattern: &Pattern, value: JsValue) -> Result<(), JsError> {
        match pattern {
            Pattern::Identifier(id) => {
                // For var, the binding was hoisted, so we need to set it
                // Try to set in existing scope chain; if not found, define in current
                if self.env_has_binding(&id.name) {
                    self.env_set_binding(&id.name, value)?;
                } else {
                    // Fallback: define if somehow not hoisted
                    // String clone - env.define takes ownership
                    self.env_define(id.name.clone(), value, true);
                }
                Ok(())
            }

            Pattern::Object(obj_pattern) => {
                let obj = match &value {
                    JsValue::Object(o) => o.cheap_clone(),
                    _ => return Err(JsError::type_error("Cannot destructure non-object")),
                };

                for prop in &obj_pattern.properties {
                    match prop {
                        ObjectPatternProperty::KeyValue {
                            key,
                            value: pattern,
                            ..
                        } => {
                            let key_str: JsString = match key {
                                ObjectPropertyKey::Identifier(id) => id.name.cheap_clone(),
                                ObjectPropertyKey::String(s) => s.value.cheap_clone(),
                                ObjectPropertyKey::Number(l) => {
                                    if let LiteralValue::Number(n) = &l.value {
                                        n.to_string().into()
                                    } else {
                                        continue;
                                    }
                                }
                                ObjectPropertyKey::Computed(_) => continue,
                                ObjectPropertyKey::PrivateIdentifier(id) => {
                                    format!("#{}", id.name).into()
                                }
                            };

                            let prop_value = obj
                                .borrow()
                                .get_property(&PropertyKey::String(key_str))
                                .unwrap_or(JsValue::Undefined);

                            self.bind_pattern_var(pattern, prop_value)?;
                        }
                        ObjectPatternProperty::Rest(rest) => {
                            let rest_obj = self.create_object();
                            self.bind_pattern_var(&rest.argument, JsValue::Object(rest_obj))?;
                        }
                    }
                }

                Ok(())
            }

            Pattern::Array(arr_pattern) => {
                let items: Vec<JsValue> = match &value {
                    JsValue::Object(obj) => {
                        let obj_ref = obj.borrow();
                        if let ExoticObject::Array { length } = &obj_ref.exotic {
                            let mut items = Vec::with_capacity(*length as usize);
                            for i in 0..*length {
                                items.push(
                                    obj_ref
                                        .get_property(&PropertyKey::Index(i))
                                        .unwrap_or(JsValue::Undefined),
                                );
                            }
                            items
                        } else {
                            vec![]
                        }
                    }
                    _ => vec![],
                };

                for (i, elem) in arr_pattern.elements.iter().enumerate() {
                    if let Some(pattern) = elem {
                        match pattern {
                            Pattern::Rest(rest) => {
                                let remaining: Vec<JsValue> =
                                    items.iter().skip(i).cloned().collect();
                                let rest_array = self.create_array(remaining);
                                self.bind_pattern_var(&rest.argument, JsValue::Object(rest_array))?;
                                break;
                            }
                            _ => {
                                let val = items.get(i).cloned().unwrap_or(JsValue::Undefined);
                                self.bind_pattern_var(pattern, val)?;
                            }
                        }
                    }
                }

                Ok(())
            }

            Pattern::Assignment(assign) => {
                let val = if value == JsValue::Undefined {
                    self.evaluate(&assign.right)?
                } else {
                    value
                };
                self.bind_pattern_var(&assign.left, val)
            }

            Pattern::Rest(rest) => self.bind_pattern_var(&rest.argument, value),
        }
    }

    /// Evaluate an expression
    pub fn evaluate(&mut self, expr: &Expression) -> Result<JsValue, JsError> {
        match expr {
            Expression::Literal(lit) => self.evaluate_literal(&lit.value),

            Expression::Identifier(id) => self.env_get_binding(&id.name),

            Expression::This(_) => {
                // Look up 'this' from the environment
                Ok(self
                    .env_get_binding(&JsString::from("this"))
                    .unwrap_or(JsValue::Undefined))
            }

            Expression::Array(arr) => {
                // Guard elements as they're evaluated to prevent GC from corrupting them
                let mut elements = vec![];
                let mut _element_guards = vec![];
                for elem in &arr.elements {
                    match elem {
                        Some(ArrayElement::Expression(e)) => {
                            let val = self.evaluate(e)?;
                            if let Some(guard) = self.guard_value(&val) {
                                _element_guards.push(guard);
                            }
                            elements.push(val);
                        }
                        Some(ArrayElement::Spread(spread)) => {
                            let val = self.evaluate(&spread.argument)?;
                            if let JsValue::Object(obj) = val {
                                let obj_ref = obj.borrow();
                                if let ExoticObject::Array { length } = &obj_ref.exotic {
                                    for i in 0..*length {
                                        if let Some(v) =
                                            obj_ref.get_property(&PropertyKey::Index(i))
                                        {
                                            if let Some(guard) = self.guard_value(&v) {
                                                _element_guards.push(guard);
                                            }
                                            elements.push(v);
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            elements.push(JsValue::Undefined);
                        }
                    }
                }
                Ok(JsValue::Object(self.create_array(elements)))
            }

            Expression::Object(obj) => {
                // Use GuardedGc to protect the object during property evaluation.
                // Property value evaluation (e.g., nested arrays/objects) may allocate,
                // which can trigger GC. The guard keeps the object alive until we're done.
                let result = self.create_object_guarded();

                // Set prototype first if __proto__ is specified
                for prop in &obj.properties {
                    if let ObjectProperty::Property(p) = prop {
                        let key_str = match &p.key {
                            ObjectPropertyKey::Identifier(id) => Some(id.name.as_str()),
                            ObjectPropertyKey::String(s) => Some(s.value.as_str()),
                            _ => None,
                        };
                        if key_str == Some("__proto__") {
                            let proto_value = self.evaluate(&p.value)?;
                            if let JsValue::Object(proto) = proto_value {
                                result.borrow_mut().prototype = Some(proto);
                            } else if matches!(proto_value, JsValue::Null) {
                                result.borrow_mut().prototype = None;
                            }
                        }
                    }
                }
                // Then set other properties
                for prop in &obj.properties {
                    match prop {
                        ObjectProperty::Property(p) => {
                            let key = self.evaluate_property_key(&p.key)?;
                            // Skip __proto__ since we handled it above
                            if key.eq_str("__proto__") {
                                continue;
                            }
                            let value = if p.method {
                                // Method shorthand - would need to handle this specially
                                self.evaluate(&p.value)?
                            } else {
                                self.evaluate(&p.value)?
                            };
                            result.borrow_mut().set_property(key, value);
                        }
                        ObjectProperty::Spread(spread) => {
                            let val = self.evaluate(&spread.argument)?;
                            if let JsValue::Object(src) = val {
                                let src_ref = src.borrow();
                                for (key, prop) in src_ref.properties.iter() {
                                    if prop.enumerable {
                                        result
                                            .borrow_mut()
                                            .set_property(key.clone(), prop.value.clone());
                                    }
                                }
                            }
                        }
                    }
                }

                // Guard is dropped here, protection removed
                Ok(JsValue::Object(result.take()))
            }

            Expression::Function(func) => {
                let interpreted = InterpretedFunction {
                    name: func.id.as_ref().map(|id| id.name.clone()),
                    params: Rc::from(func.params.as_slice()), // Rc wrap for cheap cloning
                    body: Rc::new(FunctionBody::Block(func.body.clone())), // Rc wrap for cheap cloning
                    closure: self.env.cheap_clone(),
                    source_location: func.span,
                    generator: func.generator,
                    async_: func.async_,
                };
                Ok(JsValue::Object(
                    self.create_function(JsFunction::Interpreted(interpreted)),
                ))
            }

            Expression::ArrowFunction(arrow) => {
                let interpreted = InterpretedFunction {
                    name: None,
                    params: Rc::from(arrow.params.as_slice()), // Rc wrap for cheap cloning
                    body: Rc::new(arrow.body.clone().into()),  // Rc wrap for cheap cloning
                    closure: self.env.cheap_clone(),
                    source_location: arrow.span,
                    generator: false, // Arrow functions cannot be generators
                    async_: arrow.async_,
                };
                Ok(JsValue::Object(
                    self.create_function(JsFunction::Interpreted(interpreted)),
                ))
            }

            Expression::Unary(unary) => self.evaluate_unary(unary),
            Expression::Binary(binary) => self.evaluate_binary(binary),
            Expression::Logical(logical) => self.evaluate_logical(logical),
            Expression::Conditional(cond) => self.evaluate_conditional(cond),
            Expression::Assignment(assign) => self.evaluate_assignment(assign),
            Expression::Update(update) => self.evaluate_update(update),
            Expression::Member(member) => self.evaluate_member(member),
            Expression::Call(call) => self.evaluate_call(call),
            Expression::New(new) => self.evaluate_new(new),

            Expression::Sequence(seq) => {
                let mut result = JsValue::Undefined;
                for expr in &seq.expressions {
                    result = self.evaluate(expr)?;
                }
                Ok(result)
            }

            Expression::Template(template) => {
                let mut result = String::new();
                for (i, quasi) in template.quasis.iter().enumerate() {
                    result.push_str(quasi.value.as_ref());
                    if let Some(expr) = template.expressions.get(i) {
                        let val = self.evaluate(expr)?;
                        result.push_str(val.to_js_string().as_ref());
                    }
                }
                Ok(JsValue::String(JsString::from(result)))
            }

            Expression::TaggedTemplate(tagged) => {
                // Evaluate the tag function
                let tag_fn = self.evaluate(&tagged.tag)?;

                // Build the strings array (first argument) with guard protection.
                // Creating raw_array and evaluating expressions may trigger GC.
                let strings: Vec<JsValue> = tagged
                    .quasi
                    .quasis
                    .iter()
                    .map(|q| JsValue::String(q.value.clone()))
                    .collect();
                let strings_arr_obj = self.create_array_guarded(strings);

                // Add 'raw' property to strings array (same as cooked for now)
                // TODO: properly handle raw strings with escape sequences
                let raw: Vec<JsValue> = tagged
                    .quasi
                    .quasis
                    .iter()
                    .map(|q| JsValue::String(q.value.clone()))
                    .collect();
                let raw_array = JsValue::Object(self.create_array(raw));
                let raw_key = self.key("raw");
                strings_arr_obj
                    .borrow_mut()
                    .set_property(raw_key, raw_array);

                let strings_array = JsValue::Object(strings_arr_obj.as_gc().clone());

                // Evaluate all interpolated expressions (remaining arguments)
                let mut args = vec![strings_array];
                for expr in &tagged.quasi.expressions {
                    args.push(self.evaluate(expr)?);
                }

                // Drop guard, then call the tag function
                drop(strings_arr_obj);
                self.call_function(tag_fn, JsValue::Undefined, &args)
            }

            Expression::Parenthesized(inner, _) => self.evaluate(inner),

            // TypeScript expressions - evaluate the inner expression
            Expression::TypeAssertion(ta) => self.evaluate(&ta.expression),
            Expression::NonNull(nn) => self.evaluate(&nn.expression),

            Expression::Spread(spread) => self.evaluate(&spread.argument),

            Expression::Await(await_expr) => {
                // Evaluate the awaited expression
                let value = self.evaluate(&await_expr.argument)?;

                // If it's a promise, unwrap its value synchronously
                if let JsValue::Object(obj) = &value {
                    let obj_ref = obj.borrow();
                    if let ExoticObject::Promise(state) = &obj_ref.exotic {
                        let state_ref = state.borrow();
                        match state_ref.status {
                            PromiseStatus::Fulfilled => {
                                return Ok(state_ref.result.clone().unwrap_or(JsValue::Undefined));
                            }
                            PromiseStatus::Rejected => {
                                let reason = state_ref.result.clone().unwrap_or(JsValue::Undefined);
                                return Err(JsError::thrown(reason));
                            }
                            PromiseStatus::Pending => {
                                // For synchronous execution, pending promises just resolve to undefined
                                // In a real async runtime, we would suspend here
                                return Ok(JsValue::Undefined);
                            }
                        }
                    }
                }

                // If not a promise, just return the value as-is
                Ok(value)
            }

            Expression::Import(import_expr) => {
                // Dynamic import() - evaluate the source expression and return a Promise
                // In our sync model, we create a pending promise. In a real async runtime,
                // this would suspend and return RuntimeResult::ImportAwaited
                let source_value = self.evaluate(&import_expr.source)?;
                let _specifier = source_value.to_js_string().to_string();

                // For now, create a pending promise that will need to be resolved by the host
                // In a full implementation, this would interact with the module loader
                let promise = builtins::promise::create_promise(self);

                // Store the specifier so the host can know what module to load
                // For now, we'll just return a pending promise
                // The host would need to call continue_eval with the module

                Ok(JsValue::Object(promise))
            }

            Expression::Yield(yield_expr) => {
                // Check if we're in a generator context
                let _ctx = self
                    .generator_context
                    .as_mut()
                    .ok_or_else(|| JsError::syntax_error("yield outside of generator", 0, 0))?;

                // Evaluate the yield argument
                let value = if let Some(ref arg) = yield_expr.argument {
                    // Handle yield* delegation
                    if yield_expr.delegate {
                        // yield* delegates to another iterable
                        let iterable = self.evaluate(arg)?;
                        // Simplified implementation: collect from array or generator
                        if let JsValue::Object(obj) = iterable {
                            // Check type and get info without holding the borrow
                            let (is_array, length, gen_state) = {
                                let obj_ref = obj.borrow();
                                match &obj_ref.exotic {
                                    ExoticObject::Array { length } => (true, *length, None),
                                    ExoticObject::Generator(gen) => {
                                        (false, 0, Some(gen.cheap_clone()))
                                    }
                                    _ => return Err(JsError::type_error("yield* on non-iterable")),
                                }
                            };

                            if is_array {
                                // Yield each array element
                                for i in 0..length {
                                    let elem = obj
                                        .borrow()
                                        .get_property(&PropertyKey::Index(i))
                                        .unwrap_or(JsValue::Undefined);

                                    let ctx = self.generator_context.as_mut().ok_or_else(|| {
                                        JsError::internal_error("generator context missing")
                                    })?;
                                    if ctx.current_yield == ctx.target_yield {
                                        ctx.current_yield += 1;
                                        return Err(JsError::GeneratorYield { value: elem });
                                    }
                                    ctx.current_yield += 1;
                                }
                                return Ok(JsValue::Undefined);
                            } else if let Some(gen) = gen_state {
                                // Delegate to another generator
                                let done_key = self.key("done");
                                let value_key = self.key("value");
                                loop {
                                    let result = self.resume_generator(&gen)?;
                                    let JsValue::Object(res_obj) = &result else {
                                        return Ok(JsValue::Undefined);
                                    };
                                    let done = res_obj
                                        .borrow()
                                        .get_property(&done_key)
                                        .map(|v| v.to_boolean())
                                        .unwrap_or(false);
                                    let value = res_obj
                                        .borrow()
                                        .get_property(&value_key)
                                        .unwrap_or(JsValue::Undefined);

                                    if done {
                                        return Ok(value);
                                    }

                                    let ctx = self.generator_context.as_mut().ok_or_else(|| {
                                        JsError::internal_error("generator context missing")
                                    })?;
                                    if ctx.current_yield == ctx.target_yield {
                                        ctx.current_yield += 1;
                                        return Err(JsError::GeneratorYield { value });
                                    }
                                    ctx.current_yield += 1;
                                }
                            } else {
                                return Err(JsError::type_error("yield* on non-iterable"));
                            }
                        } else {
                            return Err(JsError::type_error("yield* on non-iterable"));
                        }
                    } else {
                        self.evaluate(arg)?
                    }
                } else {
                    JsValue::Undefined
                };

                // Re-get the mutable context after evaluation
                let ctx = self
                    .generator_context
                    .as_mut()
                    .ok_or_else(|| JsError::syntax_error("yield outside of generator", 0, 0))?;

                // Check if we should throw
                if ctx.throw_value && ctx.current_yield == ctx.target_yield {
                    let exc = ctx.sent_value.clone();
                    ctx.current_yield += 1;
                    return Err(JsError::type_error(format!("Generator throw: {:?}", exc)));
                }

                // Check if this is the target yield point
                if ctx.current_yield == ctx.target_yield {
                    // Suspend here
                    ctx.current_yield += 1;
                    return Err(JsError::GeneratorYield { value });
                }

                // Not our target yet - skip this yield and return the sent value
                ctx.current_yield += 1;

                // Return the value that was sent via next(value) for this yield
                Ok(ctx.sent_value.clone())
            }

            Expression::Super(_) => {
                // Return __super__ from environment so it can be called or have properties accessed
                // super() calls the parent constructor with current this
                // super.method() accesses parent prototype method
                self.env_get_binding(&JsString::from("__super__"))
                    .map_err(|_| {
                        JsError::reference_error("'super' keyword is not available in this context")
                    })
            }

            Expression::Class(class_expr) => {
                // Convert ClassExpression to ClassDeclaration-like structure and create constructor
                let class_decl = ClassDeclaration {
                    id: class_expr.id.clone(),
                    type_parameters: class_expr.type_parameters.clone(),
                    super_class: class_expr.super_class.clone(),
                    implements: class_expr.implements.clone(),
                    body: class_expr.body.clone(),
                    decorators: class_expr.decorators.clone(),
                    abstract_: false,
                    span: class_expr.span,
                };
                let constructor_fn = self.create_class_constructor(&class_decl)?;
                Ok(JsValue::Object(constructor_fn))
            }

            Expression::OptionalChain(chain) => {
                // Simplified optional chain handling
                self.evaluate(&chain.base)
            }
        }
    }

    fn evaluate_literal(&mut self, value: &LiteralValue) -> Result<JsValue, JsError> {
        Ok(match value {
            LiteralValue::Null => JsValue::Null,
            LiteralValue::Undefined => JsValue::Undefined,
            LiteralValue::Boolean(b) => JsValue::Boolean(*b),
            LiteralValue::Number(n) => JsValue::Number(*n),
            LiteralValue::String(s) => JsValue::String(s.clone()),
            LiteralValue::BigInt(s) => {
                // TODO: Implement proper BigInt type
                // For now, convert to Number (loses precision for large values)
                JsValue::Number(s.parse::<f64>().unwrap_or(f64::NAN))
            }
            LiteralValue::RegExp { pattern, flags } => {
                // Create RegExp object with proper prototype and properties
                // Intern all keys before creating object
                let source_key = self.key("source");
                let flags_key = self.key("flags");
                let global_key = self.key("global");
                let ignore_case_key = self.key("ignoreCase");
                let multiline_key = self.key("multiline");
                let dot_all_key = self.key("dotAll");
                let unicode_key = self.key("unicode");
                let sticky_key = self.key("sticky");
                let last_index_key = self.key("lastIndex");
                let source_val = self.intern(pattern);
                let flags_val = self.intern(flags);

                let regexp_obj = self.create_object();
                {
                    let mut obj = regexp_obj.borrow_mut();
                    // String clones - needed for ExoticObject storage
                    obj.exotic = ExoticObject::RegExp {
                        pattern: pattern.clone(),
                        flags: flags.clone(),
                    };
                    obj.prototype = Some(self.regexp_prototype.cheap_clone());
                    obj.set_property(source_key, JsValue::String(source_val));
                    obj.set_property(flags_key, JsValue::String(flags_val));
                    obj.set_property(global_key, JsValue::Boolean(flags.contains('g')));
                    obj.set_property(ignore_case_key, JsValue::Boolean(flags.contains('i')));
                    obj.set_property(multiline_key, JsValue::Boolean(flags.contains('m')));
                    obj.set_property(dot_all_key, JsValue::Boolean(flags.contains('s')));
                    obj.set_property(unicode_key, JsValue::Boolean(flags.contains('u')));
                    obj.set_property(sticky_key, JsValue::Boolean(flags.contains('y')));
                    obj.set_property(last_index_key, JsValue::Number(0.0));
                }
                JsValue::Object(regexp_obj)
            }
        })
    }

    fn evaluate_property_key(&mut self, key: &ObjectPropertyKey) -> Result<PropertyKey, JsError> {
        Ok(match key {
            ObjectPropertyKey::Identifier(id) => PropertyKey::String(id.name.clone()),
            ObjectPropertyKey::String(s) => PropertyKey::String(s.value.clone()),
            ObjectPropertyKey::Number(lit) => {
                if let LiteralValue::Number(n) = &lit.value {
                    PropertyKey::from_value(&JsValue::Number(*n))
                } else {
                    self.key("undefined")
                }
            }
            ObjectPropertyKey::Computed(expr) => {
                let val = self.evaluate(expr)?;
                PropertyKey::from_value(&val)
            }
            ObjectPropertyKey::PrivateIdentifier(id) => {
                // Private fields are stored with # prefix
                self.key(&format!("#{}", id.name))
            }
        })
    }

    fn evaluate_unary(&mut self, unary: &UnaryExpression) -> Result<JsValue, JsError> {
        let arg = self.evaluate(&unary.argument)?;

        Ok(match unary.operator {
            UnaryOp::Minus => JsValue::Number(-arg.to_number()),
            UnaryOp::Plus => JsValue::Number(arg.to_number()),
            UnaryOp::Not => JsValue::Boolean(!arg.to_boolean()),
            UnaryOp::BitNot => JsValue::Number(!(arg.to_number() as i32) as f64),
            UnaryOp::Typeof => JsValue::String(JsString::from(arg.type_of())),
            UnaryOp::Void => JsValue::Undefined,
            UnaryOp::Delete => {
                // Simplified - would need to actually delete property
                JsValue::Boolean(true)
            }
        })
    }

    fn evaluate_binary(&mut self, binary: &BinaryExpression) -> Result<JsValue, JsError> {
        let left = self.evaluate(&binary.left)?;
        let right = self.evaluate(&binary.right)?;

        Ok(match binary.operator {
            // Arithmetic
            BinaryOp::Add => {
                if left.is_string() || right.is_string() {
                    let ls = left.to_js_string();
                    let rs = right.to_js_string();
                    JsValue::String(ls + &rs)
                } else {
                    JsValue::Number(left.to_number() + right.to_number())
                }
            }
            BinaryOp::Sub => JsValue::Number(left.to_number() - right.to_number()),
            BinaryOp::Mul => JsValue::Number(left.to_number() * right.to_number()),
            BinaryOp::Div => JsValue::Number(left.to_number() / right.to_number()),
            BinaryOp::Mod => JsValue::Number(left.to_number() % right.to_number()),
            BinaryOp::Exp => JsValue::Number(left.to_number().powf(right.to_number())),

            // Comparison
            BinaryOp::Lt => JsValue::Boolean(left.to_number() < right.to_number()),
            BinaryOp::LtEq => JsValue::Boolean(left.to_number() <= right.to_number()),
            BinaryOp::Gt => JsValue::Boolean(left.to_number() > right.to_number()),
            BinaryOp::GtEq => JsValue::Boolean(left.to_number() >= right.to_number()),

            // Equality
            BinaryOp::Eq => {
                // Abstract equality - simplified
                JsValue::Boolean(left.strict_equals(&right))
            }
            BinaryOp::NotEq => JsValue::Boolean(!left.strict_equals(&right)),
            BinaryOp::StrictEq => JsValue::Boolean(left.strict_equals(&right)),
            BinaryOp::StrictNotEq => JsValue::Boolean(!left.strict_equals(&right)),

            // Bitwise
            BinaryOp::BitAnd => {
                JsValue::Number((left.to_number() as i32 & right.to_number() as i32) as f64)
            }
            BinaryOp::BitOr => {
                JsValue::Number((left.to_number() as i32 | right.to_number() as i32) as f64)
            }
            BinaryOp::BitXor => {
                JsValue::Number((left.to_number() as i32 ^ right.to_number() as i32) as f64)
            }
            BinaryOp::LShift => JsValue::Number(
                ((left.to_number() as i32) << (right.to_number() as u32 & 0x1f)) as f64,
            ),
            BinaryOp::RShift => JsValue::Number(
                ((left.to_number() as i32) >> (right.to_number() as u32 & 0x1f)) as f64,
            ),
            BinaryOp::URShift => JsValue::Number(
                (((left.to_number() as i32) as u32) >> (right.to_number() as u32 & 0x1f)) as f64,
            ),

            // Other
            BinaryOp::In => {
                if let JsValue::Object(obj) = right {
                    let key = crate::value::PropertyKey::from_value(&left);
                    JsValue::Boolean(obj.borrow().has_own_property(&key))
                } else {
                    return Err(JsError::type_error(
                        "Cannot use 'in' operator on non-object",
                    ));
                }
            }
            BinaryOp::Instanceof => {
                // Check if left is an instance of right (constructor)
                let JsValue::Object(constructor) = &right else {
                    return Err(JsError::type_error(
                        "Right-hand side of 'instanceof' is not an object",
                    ));
                };

                // Get the prototype property of the constructor
                let proto_key = self.key("prototype");
                let proto_val = constructor.borrow().get_property(&proto_key);
                let Some(JsValue::Object(constructor_proto)) = proto_val else {
                    return Ok(JsValue::Boolean(false));
                };

                // Check if left is an object
                let JsValue::Object(instance) = &left else {
                    return Ok(JsValue::Boolean(false));
                };

                // Walk the prototype chain of the instance
                let mut current_proto = instance.borrow().prototype.clone();
                while let Some(proto) = current_proto {
                    if crate::gc::Gc::ptr_eq(&proto, &constructor_proto) {
                        return Ok(JsValue::Boolean(true));
                    }
                    current_proto = proto.borrow().prototype.clone();
                }
                JsValue::Boolean(false)
            }
        })
    }

    fn evaluate_logical(&mut self, logical: &LogicalExpression) -> Result<JsValue, JsError> {
        let left = self.evaluate(&logical.left)?;

        match logical.operator {
            LogicalOp::And => {
                if !left.to_boolean() {
                    Ok(left)
                } else {
                    self.evaluate(&logical.right)
                }
            }
            LogicalOp::Or => {
                if left.to_boolean() {
                    Ok(left)
                } else {
                    self.evaluate(&logical.right)
                }
            }
            LogicalOp::NullishCoalescing => {
                if left.is_null_or_undefined() {
                    self.evaluate(&logical.right)
                } else {
                    Ok(left)
                }
            }
        }
    }

    fn evaluate_conditional(&mut self, cond: &ConditionalExpression) -> Result<JsValue, JsError> {
        let test = self.evaluate(&cond.test)?;
        if test.to_boolean() {
            self.evaluate(&cond.consequent)
        } else {
            self.evaluate(&cond.alternate)
        }
    }

    fn evaluate_assignment(&mut self, assign: &AssignmentExpression) -> Result<JsValue, JsError> {
        let right = self.evaluate(&assign.right)?;

        let value = if assign.operator != AssignmentOp::Assign {
            let left = match &assign.left {
                AssignmentTarget::Identifier(id) => {
                    self.env_get_binding(&id.name).unwrap_or(JsValue::Undefined)
                }
                AssignmentTarget::Member(member) => self.evaluate_member(member)?,
                AssignmentTarget::Pattern(_) => {
                    return Err(JsError::syntax_error("Invalid assignment target", 0, 0));
                }
            };

            match assign.operator {
                AssignmentOp::AddAssign => {
                    if left.is_string() || right.is_string() {
                        JsValue::String(left.to_js_string() + &right.to_js_string())
                    } else {
                        JsValue::Number(left.to_number() + right.to_number())
                    }
                }
                AssignmentOp::SubAssign => JsValue::Number(left.to_number() - right.to_number()),
                AssignmentOp::MulAssign => JsValue::Number(left.to_number() * right.to_number()),
                AssignmentOp::DivAssign => JsValue::Number(left.to_number() / right.to_number()),
                AssignmentOp::ModAssign => JsValue::Number(left.to_number() % right.to_number()),
                AssignmentOp::ExpAssign => {
                    JsValue::Number(left.to_number().powf(right.to_number()))
                }
                AssignmentOp::BitAndAssign => {
                    JsValue::Number((left.to_number() as i32 & right.to_number() as i32) as f64)
                }
                AssignmentOp::BitOrAssign => {
                    JsValue::Number((left.to_number() as i32 | right.to_number() as i32) as f64)
                }
                AssignmentOp::BitXorAssign => {
                    JsValue::Number((left.to_number() as i32 ^ right.to_number() as i32) as f64)
                }
                AssignmentOp::LShiftAssign => JsValue::Number(
                    ((left.to_number() as i32) << (right.to_number() as u32 & 0x1f)) as f64,
                ),
                AssignmentOp::RShiftAssign => JsValue::Number(
                    ((left.to_number() as i32) >> (right.to_number() as u32 & 0x1f)) as f64,
                ),
                AssignmentOp::URShiftAssign => JsValue::Number(
                    (((left.to_number() as i32) as u32) >> (right.to_number() as u32 & 0x1f))
                        as f64,
                ),
                AssignmentOp::AndAssign => {
                    if !left.to_boolean() {
                        left
                    } else {
                        right
                    }
                }
                AssignmentOp::OrAssign => {
                    if left.to_boolean() {
                        left
                    } else {
                        right
                    }
                }
                AssignmentOp::NullishAssign => {
                    if left.is_null_or_undefined() {
                        right
                    } else {
                        left
                    }
                }
                AssignmentOp::Assign => {
                    return Err(JsError::internal_error(
                        "Assign should be handled separately",
                    ))
                }
            }
        } else {
            right
        };

        match &assign.left {
            AssignmentTarget::Identifier(id) => {
                self.env_set_binding(&id.name, value.clone())?;
            }
            AssignmentTarget::Member(member) => {
                self.set_member(member, value.clone())?;
            }
            AssignmentTarget::Pattern(pattern) => {
                self.bind_pattern(pattern, value.clone(), true)?;
            }
        }

        Ok(value)
    }

    fn evaluate_update(&mut self, update: &UpdateExpression) -> Result<JsValue, JsError> {
        let old_value = self.evaluate(&update.argument)?;
        let old_num = old_value.to_number();

        let new_value = match update.operator {
            UpdateOp::Increment => JsValue::Number(old_num + 1.0),
            UpdateOp::Decrement => JsValue::Number(old_num - 1.0),
        };

        // Set the new value
        match update.argument.as_ref() {
            Expression::Identifier(id) => {
                self.env_set_binding(&id.name, new_value.clone())?;
            }
            Expression::Member(member) => {
                self.set_member(member, new_value.clone())?;
            }
            _ => return Err(JsError::syntax_error("Invalid update target", 0, 0)),
        }

        Ok(if update.prefix {
            new_value
        } else {
            JsValue::Number(old_num)
        })
    }

    fn evaluate_member(&mut self, member: &MemberExpression) -> Result<JsValue, JsError> {
        let object = self.evaluate(&member.object)?;

        let key = match &member.property {
            MemberProperty::Identifier(id) => PropertyKey::String(id.name.clone()),
            MemberProperty::Expression(expr) => {
                let val = self.evaluate(expr)?;
                PropertyKey::from_value(&val)
            }
            MemberProperty::PrivateIdentifier(id) => {
                // Private fields are stored with # prefix
                self.key(&format!("#{}", id.name))
            }
        };

        // JsValue clone - needed because we match on it but also use object later for getters
        match object.clone() {
            JsValue::Object(obj) => {
                // Handle __proto__ special property
                if key.eq_str("__proto__") {
                    return Ok(obj
                        .borrow()
                        .prototype
                        .as_ref()
                        .map(|p| JsValue::Object(p.cheap_clone()))
                        .unwrap_or(JsValue::Null));
                }

                // First, try own properties and prototype chain with accessor support
                // We need to drop the borrow before calling the getter
                let property_result = {
                    if let Some((prop, _)) = obj.borrow().get_property_descriptor(&key) {
                        // If this is an accessor property with a getter, return the getter
                        if let Some(ref getter) = prop.getter {
                            Some((true, Some(getter.cheap_clone()), JsValue::Undefined))
                        } else if prop.is_accessor() {
                            // Getter-less accessor property returns undefined
                            Some((false, None, JsValue::Undefined))
                        } else {
                            // JsValue clone - may be cheap or expensive depending on variant
                            Some((false, None, prop.value.clone()))
                        }
                    } else {
                        None
                    }
                };

                if let Some((is_getter, getter, value)) = property_result {
                    if is_getter {
                        if let Some(getter_fn) = getter {
                            return self.call_function(JsValue::Object(getter_fn), object, &[]);
                        }
                    }
                    return Ok(value);
                }

                // For functions, check Function.prototype
                if obj.borrow().is_callable() {
                    if let Some(method) = self.function_prototype.borrow().get_property(&key) {
                        return Ok(method);
                    }
                }
                // Fall back to Object.prototype for ordinary objects
                // (but not for objects created with Object.create(null))
                if !obj.borrow().null_prototype {
                    if let Some(method) = self.object_prototype.borrow().get_property(&key) {
                        return Ok(method);
                    }
                }
                Ok(JsValue::Undefined)
            }
            JsValue::String(s) => {
                // String indexing
                if let crate::value::PropertyKey::Index(i) = key {
                    if let Some(ch) = s.as_str().chars().nth(i as usize) {
                        return Ok(JsValue::String(JsString::from(ch.to_string())));
                    }
                }
                if key.eq_str("length") {
                    return Ok(JsValue::Number(s.len() as f64));
                }
                // Look up on String.prototype
                if let Some(method) = self.string_prototype.borrow().get_property(&key) {
                    return Ok(method);
                }
                Ok(JsValue::Undefined)
            }
            JsValue::Number(_) => {
                // Look up on Number.prototype
                if let Some(method) = self.number_prototype.borrow().get_property(&key) {
                    return Ok(method);
                }
                Ok(JsValue::Undefined)
            }
            JsValue::Symbol(ref s) => {
                // Handle special symbol properties
                if key.eq_str("description") {
                    return Ok(match &s.description {
                        Some(desc) => JsValue::String(JsString::from(desc.as_str())),
                        None => JsValue::Undefined,
                    });
                }
                // Look up on Symbol.prototype
                if let Some(method) = self.symbol_prototype.borrow().get_property(&key) {
                    return Ok(method);
                }
                Ok(JsValue::Undefined)
            }
            _ => Ok(JsValue::Undefined),
        }
    }

    fn set_member(&mut self, member: &MemberExpression, value: JsValue) -> Result<(), JsError> {
        // Guard the value to prevent GC from collecting it during object/key evaluation
        let _value_guard = self.guard_value(&value);

        let object = self.evaluate(&member.object)?;

        let key = match &member.property {
            MemberProperty::Identifier(id) => PropertyKey::String(id.name.clone()),
            MemberProperty::Expression(expr) => {
                let val = self.evaluate(expr)?;
                PropertyKey::from_value(&val)
            }
            MemberProperty::PrivateIdentifier(id) => {
                // Private fields are stored with # prefix
                self.key(&format!("#{}", id.name))
            }
        };

        match object.clone() {
            JsValue::Object(obj) => {
                // Handle __proto__ special property
                if key.eq_str("__proto__") {
                    let new_proto = match value {
                        JsValue::Null => None,
                        JsValue::Object(proto) => Some(proto),
                        _ => return Ok(()), // Ignore non-object/null values
                    };
                    obj.borrow_mut().prototype = new_proto;
                    return Ok(());
                }

                // Check if there's an accessor property with a setter
                // We need to drop the borrow before calling the setter
                let setter_fn = {
                    if let Some((prop, _)) = obj.borrow().get_property_descriptor(&key) {
                        if prop.is_accessor() {
                            if let Some(ref setter) = prop.setter {
                                Some(setter.cheap_clone())
                            } else {
                                // Accessor property without setter - silently ignore in non-strict mode
                                return Ok(());
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                if let Some(setter) = setter_fn {
                    self.call_function(JsValue::Object(setter), object, &[value])?;
                    return Ok(());
                }

                obj.borrow_mut().set_property(key, value);
                Ok(())
            }
            _ => Err(JsError::type_error("Cannot set property on non-object")),
        }
    }

    fn evaluate_call(&mut self, call: &CallExpression) -> Result<JsValue, JsError> {
        // Determine 'this' binding
        // For super() calls, use the current this value
        // For super.method() calls, also use the current this value
        let this_value = if let Expression::Super(_) = call.callee.as_ref() {
            // super() - call parent constructor with current this
            self.env_get_binding(&JsString::from("this"))
                .unwrap_or(JsValue::Undefined)
        } else if let Expression::Member(member) = call.callee.as_ref() {
            if let Expression::Super(_) = member.object.as_ref() {
                // super.method() - call with current this
                self.env_get_binding(&JsString::from("this"))
                    .unwrap_or(JsValue::Undefined)
            } else {
                self.evaluate(&member.object)?
            }
        } else {
            JsValue::Undefined
        };

        // Guard this_value to prevent GC from collecting it during
        // callee evaluation and argument evaluation
        let _this_guard = self.guard_value(&this_value);

        // For super.method(), we need to look up the method on the super prototype
        let callee = if let Expression::Member(member) = call.callee.as_ref() {
            if let Expression::Super(_) = member.object.as_ref() {
                // Get super constructor
                let super_ctor =
                    self.env_get_binding(&JsString::from("__super__"))
                        .map_err(|_| {
                            JsError::reference_error(
                                "'super' keyword is not available in this context",
                            )
                        })?;
                // Get super prototype
                if let JsValue::Object(ctor) = super_ctor {
                    let proto_key = self.key("prototype");
                    let proto = ctor.borrow().get_property(&proto_key);
                    if let Some(JsValue::Object(proto_obj)) = proto {
                        // Get the method from prototype
                        let key = match &member.property {
                            MemberProperty::Identifier(id) => PropertyKey::String(id.name.clone()),
                            MemberProperty::Expression(expr) => {
                                let val = self.evaluate(expr)?;
                                PropertyKey::from_value(&val)
                            }
                            MemberProperty::PrivateIdentifier(id) => {
                                self.key(&format!("#{}", id.name))
                            }
                        };
                        proto_obj
                            .borrow()
                            .get_property(&key)
                            .unwrap_or(JsValue::Undefined)
                    } else {
                        return Err(JsError::type_error("Super has no prototype"));
                    }
                } else {
                    return Err(JsError::type_error("Super is not an object"));
                }
            } else {
                self.evaluate(&call.callee)?
            }
        } else {
            self.evaluate(&call.callee)?
        };

        // Guard callee to prevent GC from collecting it during argument evaluation
        let _callee_guard = self.guard_value(&callee);

        // Evaluate arguments. We need to guard each argument as it's evaluated
        // to prevent GC from collecting them during subsequent evaluations.
        let mut args = vec![];
        let mut _arg_guards = vec![];
        for arg in &call.arguments {
            match arg {
                Argument::Expression(expr) => {
                    let val = self.evaluate(expr)?;
                    if let Some(guard) = self.guard_value(&val) {
                        _arg_guards.push(guard);
                    }
                    args.push(val);
                }
                Argument::Spread(spread) => {
                    let val = self.evaluate(&spread.argument)?;
                    if let JsValue::Object(obj) = val {
                        let obj_ref = obj.borrow();
                        if let ExoticObject::Array { length } = &obj_ref.exotic {
                            for i in 0..*length {
                                if let Some(v) = obj_ref.get_property(&PropertyKey::Index(i)) {
                                    if let Some(guard) = self.guard_value(&v) {
                                        _arg_guards.push(guard);
                                    }
                                    args.push(v);
                                }
                            }
                        }
                    }
                }
            }
        }

        self.call_function(callee, this_value, &args)
    }

    fn evaluate_new(&mut self, new_expr: &NewExpression) -> Result<JsValue, JsError> {
        let callee = self.evaluate(&new_expr.callee)?;

        // Guard callee during argument evaluation
        let _callee_guard = self.guard_value(&callee);

        // Evaluate arguments with guards to prevent GC from collecting them
        let mut args = vec![];
        let mut _arg_guards = vec![];
        for arg in &new_expr.arguments {
            match arg {
                Argument::Expression(expr) => {
                    let val = self.evaluate(expr)?;
                    if let Some(guard) = self.guard_value(&val) {
                        _arg_guards.push(guard);
                    }
                    args.push(val);
                }
                Argument::Spread(spread) => {
                    let val = self.evaluate(&spread.argument)?;
                    if let JsValue::Object(obj) = val {
                        let obj_ref = obj.borrow();
                        if let ExoticObject::Array { length } = &obj_ref.exotic {
                            for i in 0..*length {
                                if let Some(v) = obj_ref.get_property(&PropertyKey::Index(i)) {
                                    if let Some(guard) = self.guard_value(&v) {
                                        _arg_guards.push(guard);
                                    }
                                    args.push(v);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Create new object with prototype from constructor
        // Use GuardedGc to protect the new object during prototype setup and constructor call.
        // The constructor call allocates (new environment), which can trigger GC.
        let new_obj = self.create_object_guarded();

        // Get prototype from constructor.prototype and set it on the new object
        let proto_key = self.key("prototype");
        let fields_key = self.key("__fields__");
        if let JsValue::Object(ctor_obj) = &callee {
            let ctor_ref = ctor_obj.borrow();
            if let Some(JsValue::Object(proto)) = ctor_ref.get_property(&proto_key) {
                drop(ctor_ref);
                new_obj.borrow_mut().prototype = Some(proto);
            } else {
                drop(ctor_ref);
            }

            // Initialize instance fields from __fields__
            let fields = ctor_obj.borrow().get_property(&fields_key);
            if let Some(JsValue::Object(fields_arr)) = fields {
                let fields_ref = fields_arr.borrow();
                if let ExoticObject::Array { length } = fields_ref.exotic {
                    for i in 0..length {
                        if let Some(JsValue::Object(pair)) =
                            fields_ref.get_property(&PropertyKey::Index(i))
                        {
                            let pair_ref = pair.borrow();
                            if let Some(JsValue::String(name)) =
                                pair_ref.get_property(&PropertyKey::Index(0))
                            {
                                let value = pair_ref
                                    .get_property(&PropertyKey::Index(1))
                                    .unwrap_or(JsValue::Undefined);
                                drop(pair_ref);
                                new_obj
                                    .borrow_mut()
                                    .set_property(PropertyKey::String(name), value);
                            }
                        }
                    }
                }
            }
        }

        // Determine if this is a native or interpreted constructor
        let is_native = if let JsValue::Object(ctor_obj) = &callee {
            matches!(
                ctor_obj.borrow().exotic,
                ExoticObject::Function(JsFunction::Native(_))
            )
        } else {
            false
        };

        // Call constructor (guard still active)
        let result = self.call_function(callee, JsValue::Object(new_obj.as_gc().clone()), &args)?;

        // Extract the unguarded Gc for return
        let new_obj_gc = new_obj.take();

        if is_native {
            // Native constructors (Date, Error, etc.) create and return their own objects.
            // We should use their return value.
            match result {
                JsValue::Object(_) => Ok(result),
                _ => Ok(JsValue::Object(new_obj_gc)),
            }
        } else {
            // Interpreted constructors modify `this` (new_obj) in place.
            // We should NOT use the return value because call_function doesn't distinguish
            // between explicit `return obj;` and normal block completion (last expression value).
            // This fixes the bug where `this.x = {obj}` would incorrectly return `{obj}`.
            // TODO: Add proper support for constructor explicit object return via Completion type.
            Ok(JsValue::Object(new_obj_gc))
        }
    }

    /// Execute an async function and return a Promise
    fn execute_async_function(
        &mut self,
        interpreted: InterpretedFunction,
        this_value: JsValue,
        args: Vec<JsValue>,
    ) -> Result<JsValue, JsError> {
        // Create a promise to return
        let promise = builtins::promise::create_promise(self);

        // Push stack frame
        let func_name = interpreted
            .name
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "<anonymous>".to_string());
        let span = interpreted.source_location;
        let location = Some((span.line, span.column));
        self.call_stack.push(StackFrame {
            function_name: func_name,
            location,
        });

        let prev_env = self.env.cheap_clone();
        self.env = self.env_alloc(Some(interpreted.closure.cheap_clone()));

        // Bind 'this' value
        self.env_define("this", this_value, false);

        // Create and bind 'arguments' object
        let arguments_obj = self.create_array(args.clone());
        self.env_define("arguments", JsValue::Object(arguments_obj), false);

        // Hoist var declarations
        if let FunctionBody::Block(block) = interpreted.body.as_ref() {
            self.hoist_var_declarations(&block.body);
        }

        // Bind parameters
        for (i, param) in interpreted.params.iter().enumerate() {
            if let Pattern::Rest(rest) = &param.pattern {
                let rest_args: Vec<JsValue> = args.get(i..).unwrap_or_default().to_vec();
                let rest_array = JsValue::Object(self.create_array(rest_args));
                self.bind_pattern(&rest.argument, rest_array, true)?;
                break;
            } else {
                let arg = args.get(i).cloned().unwrap_or(JsValue::Undefined);
                self.bind_pattern(&param.pattern, arg, true)?;
            }
        }

        // Execute body and resolve/reject the promise
        let execution_result = match interpreted.body.as_ref() {
            FunctionBody::Block(block) => self.execute_block(block),
            FunctionBody::Expression(expr) => self.evaluate(expr).map(Completion::Normal),
        };

        let func_env = self.env.cheap_clone();
        self.env = prev_env;
        self.gc_space.remove_root(&func_env);
        self.call_stack.pop();

        match execution_result {
            Ok(completion) => {
                let value = match completion {
                    Completion::Return(val) => val,
                    Completion::Normal(val) => val,
                    _ => JsValue::Undefined,
                };
                builtins::promise::resolve_promise_value(self, &promise, value)?;
            }
            Err(e) => {
                let error_value = e.to_value();
                builtins::promise::reject_promise_value(self, &promise, error_value)?;
            }
        }

        Ok(JsValue::Object(promise))
    }

    pub fn call_function(
        &mut self,
        callee: JsValue,
        this_value: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, JsError> {
        let JsValue::Object(obj) = callee else {
            return Err(JsError::type_error("Not a function"));
        };

        let func = {
            let obj_ref = obj.borrow();
            match &obj_ref.exotic {
                // JsFunction clone - expensive for Interpreted (contains AST), cheap for Native/Bound
                ExoticObject::Function(f) => f.clone(),
                _ => return Err(JsError::type_error("Not a function")),
            }
        };

        match func {
            JsFunction::Interpreted(interpreted) => {
                // If this is a generator function, create a Generator object instead of executing
                if interpreted.generator {
                    let body = match interpreted.body.as_ref() {
                        FunctionBody::Block(block) => Rc::new(block.clone()),
                        FunctionBody::Expression(_) => {
                            return Err(JsError::type_error("Generator must have block body"));
                        }
                    };

                    let gen_state = GeneratorState {
                        body,
                        params: interpreted.params.cheap_clone(), // Rc clone - cheap
                        args: args.to_vec(),
                        closure: interpreted.closure, // EnvId is Copy
                        state: GeneratorStatus::Suspended,
                        stmt_index: 0,
                        sent_value: JsValue::Undefined,
                        name: interpreted.name.clone(),
                    };

                    let gen_obj = create_generator_object(self, gen_state);
                    return Ok(JsValue::Object(gen_obj));
                }

                // If this is an async function, execute it and wrap result in a Promise
                if interpreted.async_ {
                    return self.execute_async_function(interpreted, this_value, args.to_vec());
                }

                // Push stack frame
                let func_name = interpreted
                    .name
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "<anonymous>".to_string());
                let span = interpreted.source_location;
                let location = Some((span.line, span.column));
                self.call_stack.push(StackFrame {
                    function_name: func_name,
                    location,
                });

                let prev_env = self.env.cheap_clone();
                self.env = self.env_alloc(Some(interpreted.closure.cheap_clone()));

                // Bind 'this' value
                self.env_define("this", this_value.clone(), false);

                // Create and bind 'arguments' object (array-like object with all args)
                let arguments_obj = self.create_array(args.to_vec());
                self.env_define("arguments", JsValue::Object(arguments_obj), false);

                // Check if function has __super__ (for class constructors/methods)
                let super_key = self.key("__super__");
                let super_ctor = obj.borrow().get_property(&super_key);
                if let Some(super_val) = super_ctor {
                    self.env_define("__super__", super_val, false);
                }

                // Hoist var declarations before anything else
                if let FunctionBody::Block(block) = interpreted.body.as_ref() {
                    self.hoist_var_declarations(&block.body);
                }

                // Bind parameters
                for (i, param) in interpreted.params.iter().enumerate() {
                    // Check if this is a rest parameter
                    if let Pattern::Rest(rest) = &param.pattern {
                        // Collect remaining arguments into an array
                        let rest_args: Vec<JsValue> = args.get(i..).unwrap_or_default().to_vec();
                        let rest_array = JsValue::Object(self.create_array(rest_args));
                        self.bind_pattern(&rest.argument, rest_array, true)?;
                        break; // Rest param must be last
                    } else {
                        let arg = args.get(i).cloned().unwrap_or(JsValue::Undefined);
                        self.bind_pattern(&param.pattern, arg, true)?;
                    }
                }

                // Execute body - must restore environment even if an error occurs
                let result = match interpreted.body.as_ref() {
                    FunctionBody::Block(block) => self.execute_block(block),
                    FunctionBody::Expression(expr) => self.evaluate(expr).map(Completion::Normal),
                };

                // Always restore environment and clean up, even on error
                let func_env = self.env.cheap_clone();
                self.env = prev_env;
                self.gc_space.remove_root(&func_env);
                self.call_stack.pop();

                // Now handle the result
                match result {
                    Ok(completion) => match completion {
                        Completion::Return(val) => Ok(val),
                        Completion::Normal(val) => Ok(val),
                        _ => Ok(JsValue::Undefined),
                    },
                    Err(e) => Err(e),
                }
            }

            JsFunction::Native(native) => {
                // Push stack frame for native functions too
                self.call_stack.push(StackFrame {
                    function_name: native.name.to_string(),
                    location: None,
                });
                let result = (native.func)(self, this_value, args);
                self.call_stack.pop();
                result
            }

            JsFunction::Bound(bound_data) => {
                // For bound functions:
                // - Use the bound this value (ignore the passed this_value)
                // - Prepend bound args to the call args
                // JsValue clone - may be cheap or expensive depending on variant
                let bound_this = bound_data.this_arg.clone();
                // Vec<JsValue> clone - needed to prepend to args
                let mut full_args = bound_data.bound_args.clone();
                full_args.extend_from_slice(args);

                // Call the target function with bound this and combined args
                self.call_function(
                    JsValue::Object(bound_data.target.cheap_clone()),
                    bound_this,
                    &full_args,
                )
            }

            JsFunction::PromiseResolve(promise) => {
                let value = args.first().cloned().unwrap_or(JsValue::Undefined);
                builtins::promise::resolve_promise_value(self, &promise, value)?;
                Ok(JsValue::Undefined)
            }

            JsFunction::PromiseReject(promise) => {
                let reason = args.first().cloned().unwrap_or(JsValue::Undefined);
                builtins::promise::reject_promise_value(self, &promise, reason)?;
                Ok(JsValue::Undefined)
            }
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

// No Drop impl needed - arena-based environments have no reference cycles
