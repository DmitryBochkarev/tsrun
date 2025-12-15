//! Interpreter for executing TypeScript AST
//!
//! This module implements a minimal interpreter using the new guard-based GC.

// Old implementation kept for reference in old_mod.rs (not compiled)
// mod old_mod;

// Builtin function implementations
pub mod builtins;

// Evaluation stack for suspendable execution (disabled - needs migration)
// pub mod eval_stack;

use crate::ast::{
    Argument, ArrayElement, ArrayPattern, AssignmentExpression, AssignmentOp, AssignmentTarget,
    BinaryExpression, BinaryOp, BlockStatement, CallExpression, ClassConstructor, ClassDeclaration,
    ClassExpression, ClassMember, ClassMethod, ClassProperty, ConditionalExpression,
    DoWhileStatement, Expression, ForInOfLeft, ForInStatement, ForInit, ForOfStatement,
    ForStatement, FunctionDeclaration, FunctionParam, IfStatement, LabeledStatement, LiteralValue,
    LogicalExpression, LogicalOp, MemberExpression, MemberProperty, MethodKind, NewExpression,
    ObjectExpression, ObjectPatternProperty, ObjectProperty, ObjectPropertyKey, Pattern, Program,
    SequenceExpression, Statement, SwitchStatement, TaggedTemplateExpression, TemplateLiteral,
    TryStatement, UnaryExpression, UnaryOp, UpdateExpression, UpdateOp, VariableDeclaration,
    VariableKind, WhileStatement,
};
use crate::error::JsError;
use crate::gc::{Gc, Guard, Heap};
use crate::lexer::Span;
use crate::parser::Parser;
use crate::string_dict::StringDict;
use crate::value::{
    create_environment_with_guard, Binding, CheapClone, EnvRef, EnvironmentData, ExoticObject,
    FunctionBody, Guarded, InterpretedFunction, JsFunction, JsObject, JsString, JsValue, NativeFn,
    NativeFunction, Property, PropertyKey,
};
use rustc_hash::FxHashMap;
use std::rc::Rc;

/// Completion record for control flow
/// Control flow completion type
#[derive(Debug)]
pub enum Completion {
    Normal(JsValue),
    Return(JsValue),
    Break(Option<JsString>),
    Continue(Option<JsString>),
}

// Re-export Guarded from value module - see value.rs for documentation

/// A stack frame for tracking call stack
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Function name (or "<anonymous>" for anonymous functions)
    pub function_name: String,
    /// Source location if available
    pub location: Option<(u32, u32)>, // (line, column)
}

/// GC statistics for debugging and monitoring
#[derive(Debug, Clone)]
pub struct GcStats {
    pub alive_count: usize,
    pub free_count: usize,
}

/// The interpreter state
pub struct Interpreter {
    // ═══════════════════════════════════════════════════════════════════════════
    // GC Infrastructure
    // ═══════════════════════════════════════════════════════════════════════════
    /// The GC heap managing all objects
    pub heap: Heap<JsObject>,

    /// Root guard for permanent objects (prototypes, global, global_env)
    root_guard: Guard<JsObject>,

    // ═══════════════════════════════════════════════════════════════════════════
    // Global State
    // ═══════════════════════════════════════════════════════════════════════════
    /// Global object
    pub global: Gc<JsObject>,

    /// Global environment (variable bindings for global scope)
    pub global_env: EnvRef,

    /// Current execution environment
    pub env: EnvRef,

    /// String dictionary for interning strings
    pub string_dict: StringDict,

    // ═══════════════════════════════════════════════════════════════════════════
    // Prototypes (all rooted via root_guard)
    // ═══════════════════════════════════════════════════════════════════════════
    /// Object.prototype
    pub object_prototype: Gc<JsObject>,

    /// Array.prototype
    pub array_prototype: Gc<JsObject>,

    /// Function.prototype
    pub function_prototype: Gc<JsObject>,

    /// String.prototype (for string primitive methods)
    pub string_prototype: Gc<JsObject>,

    /// Number.prototype (for number primitive methods)
    pub number_prototype: Gc<JsObject>,

    /// RegExp.prototype (for regexp methods)
    pub regexp_prototype: Gc<JsObject>,

    /// Map.prototype (for Map methods)
    pub map_prototype: Gc<JsObject>,

    /// Set.prototype (for Set methods)
    pub set_prototype: Gc<JsObject>,

    /// Date.prototype (for Date methods)
    pub date_prototype: Gc<JsObject>,

    /// Symbol.prototype (for Symbol methods)
    pub symbol_prototype: Gc<JsObject>,

    // ═══════════════════════════════════════════════════════════════════════════
    // Execution State
    // ═══════════════════════════════════════════════════════════════════════════
    /// Stores thrown value during exception propagation
    thrown_value: Option<JsValue>,

    /// Exported values from the module
    pub exports: FxHashMap<JsString, JsValue>,

    /// Call stack for stack traces
    pub call_stack: Vec<StackFrame>,

    // ═══════════════════════════════════════════════════════════════════════════
    // Timeout Control
    // ═══════════════════════════════════════════════════════════════════════════
    /// Execution timeout in milliseconds (0 = no timeout)
    timeout_ms: u64,

    /// When execution started (for timeout checking)
    execution_start: Option<std::time::Instant>,

    // ═══════════════════════════════════════════════════════════════════════════
    // Internal Module System
    // ═══════════════════════════════════════════════════════════════════════════
    /// Registered internal modules (specifier -> module definition)
    internal_modules: FxHashMap<String, crate::InternalModule>,

    /// Instantiated internal module objects (cached after first import)
    internal_module_cache: FxHashMap<String, Gc<JsObject>>,

    /// Loaded external modules (specifier -> module namespace)
    loaded_modules: FxHashMap<String, Gc<JsObject>>,

    // ═══════════════════════════════════════════════════════════════════════════
    // Order System
    // ═══════════════════════════════════════════════════════════════════════════
    /// Counter for generating unique order IDs
    pub(crate) next_order_id: u64,

    /// Pending orders waiting for host fulfillment
    pub(crate) pending_orders: Vec<crate::Order>,

    /// Guards keeping pending order payloads alive
    pub(crate) pending_order_guards: Vec<Guard<JsObject>>,

    /// Map from OrderId -> (resolve_fn, reject_fn) for pending promises
    pub(crate) order_callbacks: FxHashMap<crate::OrderId, (Gc<JsObject>, Gc<JsObject>)>,

    /// Cancelled order IDs (from Promise.race losing, etc.)
    pub(crate) cancelled_orders: Vec<crate::OrderId>,
}

impl Interpreter {
    /// Create a new interpreter instance
    pub fn new() -> Self {
        let heap: Heap<JsObject> = Heap::new();
        let root_guard = heap.create_guard();

        // Create prototypes (all rooted)
        let object_prototype = root_guard.alloc();
        let array_prototype = root_guard.alloc();
        let function_prototype = root_guard.alloc();
        let string_prototype = root_guard.alloc();
        let number_prototype = root_guard.alloc();
        let regexp_prototype = root_guard.alloc();
        let map_prototype = root_guard.alloc();
        let set_prototype = root_guard.alloc();
        let date_prototype = root_guard.alloc();
        let symbol_prototype = root_guard.alloc();

        // Set up prototype chain - all prototypes inherit from object_prototype
        array_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        function_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        string_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        number_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        regexp_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        map_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        set_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        date_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        symbol_prototype.borrow_mut().prototype = Some(object_prototype.clone());

        // Create global object (rooted)
        let global = root_guard.alloc();
        global.borrow_mut().prototype = Some(object_prototype.clone());

        // Create global environment (rooted, owned by global)
        let global_env = root_guard.alloc();
        {
            let mut env_ref = global_env.borrow_mut();
            env_ref.null_prototype = true;
            env_ref.exotic = ExoticObject::Environment(EnvironmentData::new());
        }

        let string_dict = StringDict::new();

        let mut interp = Self {
            heap,
            root_guard,
            global,
            global_env: global_env.clone(),
            env: global_env,
            string_dict,
            object_prototype,
            array_prototype,
            function_prototype,
            string_prototype,
            number_prototype,
            regexp_prototype,
            map_prototype,
            set_prototype,
            date_prototype,
            symbol_prototype,
            thrown_value: None,
            exports: FxHashMap::default(),
            call_stack: Vec::new(),
            timeout_ms: 3000, // Default 3 second timeout
            execution_start: None,
            // Internal module system
            internal_modules: FxHashMap::default(),
            internal_module_cache: FxHashMap::default(),
            loaded_modules: FxHashMap::default(),
            // Order system
            next_order_id: 1,
            pending_orders: Vec::new(),
            pending_order_guards: Vec::new(),
            order_callbacks: FxHashMap::default(),
            cancelled_orders: Vec::new(),
        };

        // Initialize built-in globals
        interp.init_globals();

        interp
    }

    /// Initialize built-in global values
    fn init_globals(&mut self) {
        // For now, minimal globals - just define undefined and NaN
        let undefined_name = self.string_dict.get_or_insert("undefined");
        self.env_define(undefined_name, JsValue::Undefined, false);

        let nan_name = self.string_dict.get_or_insert("NaN");
        self.env_define(nan_name, JsValue::Number(f64::NAN), false);

        let infinity_name = self.string_dict.get_or_insert("Infinity");
        self.env_define(infinity_name, JsValue::Number(f64::INFINITY), false);

        // Initialize Array builtin methods
        builtins::init_array_prototype(self);

        // Initialize String prototype methods
        builtins::init_string_prototype(self);

        // Initialize Function.prototype (call, apply, bind)
        builtins::init_function_prototype(self);

        // Initialize Math global object
        builtins::init_math(self);

        // Initialize JSON global object
        builtins::init_json(self);

        // Initialize console global object
        builtins::init_console(self);

        // Initialize Number prototype methods
        builtins::init_number_prototype(self);

        // Initialize Error constructor
        builtins::init_error(self);

        // Initialize global functions (parseInt, parseFloat, isNaN, isFinite, URI functions)
        builtins::init_global_functions(self);

        // Initialize Map constructor and prototype
        builtins::init_map(self);

        // Initialize Set constructor and prototype
        builtins::init_set(self);

        // Initialize Date constructor and prototype
        builtins::init_date(self);

        // Initialize Symbol constructor and prototype
        builtins::init_symbol(self);

        // Initialize String constructor (global String function)
        let string_constructor = builtins::create_string_constructor(self);
        let string_name = self.string_dict.get_or_insert("String");
        self.env_define(string_name, JsValue::Object(string_constructor), false);

        // Initialize Array constructor (global Array function)
        let array_constructor = builtins::create_array_constructor(self);
        let array_name = self.string_dict.get_or_insert("Array");
        self.env_define(array_name, JsValue::Object(array_constructor), false);

        // Initialize Object prototype and constructor
        builtins::init_object_prototype(self);
        let object_constructor = builtins::create_object_constructor(self);
        let object_name = self.string_dict.get_or_insert("Object");
        self.env_define(object_name, JsValue::Object(object_constructor), false);

        // Initialize RegExp prototype and constructor
        builtins::init_regexp_prototype(self);
        let regexp_constructor = builtins::create_regexp_constructor(self);
        let regexp_name = self.string_dict.get_or_insert("RegExp");
        self.env_define(regexp_name, JsValue::Object(regexp_constructor), false);

        // Initialize Number constructor (global Number function)
        let number_constructor = builtins::create_number_constructor(self);
        let number_name = self.string_dict.get_or_insert("Number");
        self.env_define(number_name, JsValue::Object(number_constructor), false);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Timeout Control
    // ═══════════════════════════════════════════════════════════════════════════

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

    /// Start the execution timer
    fn start_execution(&mut self) {
        if self.timeout_ms > 0 && self.execution_start.is_none() {
            self.execution_start = Some(std::time::Instant::now());
        }
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

    // ═══════════════════════════════════════════════════════════════════════════
    // Internal Module System
    // ═══════════════════════════════════════════════════════════════════════════

    /// Register an internal module for import
    pub fn register_internal_module(&mut self, module: crate::InternalModule) {
        self.internal_modules
            .insert(module.specifier.clone(), module);
    }

    /// Check if a specifier is an internal module
    pub fn is_internal_module(&self, specifier: &str) -> bool {
        self.internal_modules.contains_key(specifier)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Full Runtime API (imports + orders)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Evaluate TypeScript/JavaScript code with full runtime support
    ///
    /// Returns RuntimeResult which may indicate:
    /// - Complete: execution finished with a value
    /// - NeedImports: modules need to be provided before continuing
    /// - Suspended: waiting for orders to be fulfilled
    pub fn eval(&mut self, source: &str) -> Result<crate::RuntimeResult, JsError> {
        // Parse the source
        let mut parser = Parser::new(source, &mut self.string_dict);
        let program = parser.parse_program()?;

        // Collect all import specifiers
        let imports = self.collect_imports(&program);

        // Check which imports are missing (not internal and not already loaded)
        let missing: Vec<String> = imports
            .into_iter()
            .filter(|spec| {
                !self.is_internal_module(spec) && !self.loaded_modules.contains_key(spec)
            })
            .collect();

        if !missing.is_empty() {
            return Ok(crate::RuntimeResult::NeedImports(missing));
        }

        // All imports satisfied - execute
        let result = self.execute_program(&program);

        match result {
            Ok(value) => {
                // Check if there are pending orders
                if !self.pending_orders.is_empty() {
                    let pending = std::mem::take(&mut self.pending_orders);
                    let cancelled = std::mem::take(&mut self.cancelled_orders);
                    Ok(crate::RuntimeResult::Suspended { pending, cancelled })
                } else {
                    Ok(crate::RuntimeResult::Complete(value))
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Provide a module source for a pending import
    pub fn provide_module(&mut self, specifier: &str, source: &str) -> Result<(), JsError> {
        // Parse and execute the module
        let mut parser = Parser::new(source, &mut self.string_dict);
        let program = parser.parse_program()?;

        // Save current environment
        let saved_env = self.env.clone();

        // Create module environment
        let module_env = self.create_module_environment();
        self.env = module_env;

        // Execute module
        let result = self.execute_program(&program);

        // Restore environment
        self.env = saved_env;

        result?;

        // Create module namespace object from exports
        let (module_obj, _guard) = self.create_object_with_guard();

        // Drain exports to a vector to avoid borrow conflict
        let exports: Vec<_> = self.exports.drain().collect();

        // Copy exports to module object
        for (name, value) in exports {
            let key = self.key(name.as_str());
            module_obj.borrow_mut().set_property(key, value);
        }

        // Root the module (lives forever)
        self.root_guard.guard(module_obj.clone());

        // Cache it
        self.loaded_modules
            .insert(specifier.to_string(), module_obj);

        Ok(())
    }

    /// Continue evaluation after providing modules or fulfilling orders
    pub fn continue_eval(&mut self) -> Result<crate::RuntimeResult, JsError> {
        // Check if there are pending orders
        if !self.pending_orders.is_empty() {
            let pending = std::mem::take(&mut self.pending_orders);
            let cancelled = std::mem::take(&mut self.cancelled_orders);
            return Ok(crate::RuntimeResult::Suspended { pending, cancelled });
        }

        // Otherwise, execution is complete
        Ok(crate::RuntimeResult::Complete(JsValue::Undefined))
    }

    /// Fulfill orders with responses from the host
    pub fn fulfill_orders(&mut self, responses: Vec<crate::OrderResponse>) -> Result<(), JsError> {
        for response in responses {
            if let Some((resolve_fn, reject_fn)) = self.order_callbacks.remove(&response.id) {
                match response.result {
                    Ok(value) => {
                        // Call resolve(value)
                        self.call_function(
                            JsValue::Object(resolve_fn),
                            JsValue::Undefined,
                            &[value],
                        )?;
                    }
                    Err(error) => {
                        // Create error object and call reject
                        let error_msg = JsValue::String(JsString::from(error.to_string()));
                        self.call_function(
                            JsValue::Object(reject_fn),
                            JsValue::Undefined,
                            &[error_msg],
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Create a module environment (for executing modules)
    fn create_module_environment(&mut self) -> Gc<JsObject> {
        let env = self.root_guard.alloc();
        {
            let mut env_ref = env.borrow_mut();
            env_ref.null_prototype = true;
            env_ref.exotic = ExoticObject::Environment(EnvironmentData::with_outer(Some(
                self.global_env.clone(),
            )));
        }
        env
    }

    /// Collect all import specifiers from a program
    fn collect_imports(&self, program: &Program) -> Vec<String> {
        use crate::ast::Statement;

        let mut imports = Vec::new();

        for stmt in &program.body {
            match stmt {
                Statement::Import(import) => {
                    imports.push(import.source.value.to_string());
                }
                Statement::Export(export) => {
                    // Re-export from another module: export { foo } from "./bar"
                    if let Some(ref source) = export.source {
                        imports.push(source.value.to_string());
                    }
                }
                _ => {}
            }
        }

        imports
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Environment Operations
    // ═══════════════════════════════════════════════════════════════════════════

    /// Define a variable in the current environment
    pub fn env_define(&mut self, name: JsString, value: JsValue, mutable: bool) {
        let mut env_ref = self.env.borrow_mut();
        if let Some(data) = env_ref.as_environment_mut() {
            data.bindings.insert(
                name,
                Binding {
                    value,
                    mutable,
                    initialized: true,
                },
            );
        }
    }

    /// Get a variable from the environment chain
    pub fn env_get(&self, name: &JsString) -> Result<JsValue, JsError> {
        let mut current = Some(self.env.clone());

        while let Some(env) = current {
            let env_ref = env.borrow();
            if let Some(data) = env_ref.as_environment() {
                if let Some(binding) = data.bindings.get(name) {
                    if !binding.initialized {
                        return Err(JsError::reference_error(format!(
                            "Cannot access '{}' before initialization",
                            name
                        )));
                    }
                    return Ok(binding.value.clone());
                }
                current = data.outer.clone();
            } else {
                break;
            }
        }

        // Check global object properties
        let global = self.global.borrow();
        if let Some(prop) = global.get_property(&PropertyKey::String(name.cheap_clone())) {
            return Ok(prop);
        }

        Err(JsError::reference_error(format!("{} is not defined", name)))
    }

    /// Set a variable in the environment chain
    pub fn env_set(&mut self, name: &JsString, value: JsValue) -> Result<(), JsError> {
        let mut current = Some(self.env.clone());

        while let Some(env) = current {
            let mut env_ref = env.borrow_mut();
            if let Some(data) = env_ref.as_environment_mut() {
                if let Some(binding) = data.bindings.get_mut(name) {
                    if !binding.mutable {
                        return Err(JsError::type_error(format!(
                            "Assignment to constant variable '{}'",
                            name
                        )));
                    }
                    // Update binding value - Gc clone/drop handles ref_count automatically
                    binding.value = value;
                    return Ok(());
                }
                let outer = data.outer.clone();
                drop(env_ref);
                current = outer;
            } else {
                break;
            }
        }

        Err(JsError::reference_error(format!("{} is not defined", name)))
    }

    /// Push a new scope and return the saved environment
    pub fn push_scope(&mut self) -> EnvRef {
        let new_env = create_environment_with_guard(&self.root_guard, Some(self.env.clone()));

        let old_env = self.env.clone();
        self.env = new_env;
        old_env
    }

    /// Pop scope by restoring saved environment
    pub fn pop_scope(&mut self, saved_env: EnvRef) {
        self.env = saved_env;
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Object Creation Helpers (Temporary Guard Pattern)
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // Objects are created with a temporary guard that keeps them alive until
    // ownership is transferred to a permanent owner (environment, parent object).
    // The caller MUST transfer ownership before the temp guard is dropped.
    //
    // Pattern:
    //   let (obj, _temp) = self.create_object_with_guard();
    //   // _temp is dropped here, but obj is still alive via parent
    // ═══════════════════════════════════════════════════════════════════════════

    /// Create a new plain object with a temporary guard.
    /// Returns (object, temp_guard). Caller must transfer ownership before guard is dropped.
    pub fn create_object_with_guard(&mut self) -> (Gc<JsObject>, Guard<JsObject>) {
        let temp = self.heap.create_guard();
        let obj = temp.alloc();
        obj.borrow_mut().prototype = Some(self.object_prototype.clone());
        (obj, temp)
    }

    /// Create a RegExp literal object with a temporary guard.
    /// Used when evaluating /pattern/flags syntax.
    fn create_regexp_literal(&mut self, pattern: &str, flags: &str) -> Result<Guarded, JsError> {
        // Pre-intern all property keys
        let source_key = self.key("source");
        let flags_key = self.key("flags");
        let global_key = self.key("global");
        let ignore_case_key = self.key("ignoreCase");
        let multiline_key = self.key("multiline");
        let dot_all_key = self.key("dotAll");
        let unicode_key = self.key("unicode");
        let sticky_key = self.key("sticky");
        let last_index_key = self.key("lastIndex");

        let (regexp_obj, guard) = self.create_object_with_guard();
        {
            let mut obj = regexp_obj.borrow_mut();
            obj.exotic = ExoticObject::RegExp {
                pattern: pattern.to_string(),
                flags: flags.to_string(),
            };
            obj.prototype = Some(self.regexp_prototype.clone());
            obj.set_property(source_key, JsValue::String(JsString::from(pattern)));
            obj.set_property(flags_key, JsValue::String(JsString::from(flags)));
            obj.set_property(global_key, JsValue::Boolean(flags.contains('g')));
            obj.set_property(ignore_case_key, JsValue::Boolean(flags.contains('i')));
            obj.set_property(multiline_key, JsValue::Boolean(flags.contains('m')));
            obj.set_property(dot_all_key, JsValue::Boolean(flags.contains('s')));
            obj.set_property(unicode_key, JsValue::Boolean(flags.contains('u')));
            obj.set_property(sticky_key, JsValue::Boolean(flags.contains('y')));
            obj.set_property(last_index_key, JsValue::Number(0.0));
        }
        // Update ownership
        Ok(Guarded::with_guard(JsValue::Object(regexp_obj), guard))
    }

    /// Create a new array with elements and a temporary guard.
    /// Returns (array, temp_guard). Caller must transfer ownership before guard is dropped.
    pub fn create_array_with_guard(
        &mut self,
        elements: Vec<JsValue>,
    ) -> (Gc<JsObject>, Guard<JsObject>) {
        let len = elements.len() as u32;
        let temp = self.heap.create_guard();
        let arr = temp.alloc();
        {
            let mut arr_ref = arr.borrow_mut();
            arr_ref.prototype = Some(self.array_prototype.clone());
            arr_ref.exotic = ExoticObject::Array { length: len };

            for (i, elem) in elements.iter().enumerate() {
                arr_ref.set_property(PropertyKey::Index(i as u32), elem.clone());
            }

            // length should be writable but not enumerable
            let length_key = PropertyKey::String(self.string_dict.get_or_insert("length"));
            arr_ref.properties.insert(
                length_key,
                Property::with_attributes(JsValue::Number(len as f64), true, false, false),
            );
        }

        (arr, temp)
    }

    /// Create a function object with a temporary guard.
    /// Returns (function, temp_guard). Caller must transfer ownership before guard is dropped.
    #[allow(clippy::too_many_arguments)]
    fn create_function_with_guard(
        &mut self,
        name: Option<JsString>,
        params: Rc<[FunctionParam]>,
        body: Rc<FunctionBody>,
        closure: EnvRef,
        span: Span,
        generator: bool,
        async_: bool,
    ) -> (Gc<JsObject>, Guard<JsObject>) {
        let temp = self.heap.create_guard();
        let func_obj = temp.alloc();
        {
            let mut f_ref = func_obj.borrow_mut();
            f_ref.prototype = Some(self.function_prototype.clone());
            f_ref.exotic = ExoticObject::Function(JsFunction::Interpreted(InterpretedFunction {
                name,
                params,
                body,
                closure,
                source_location: span,
                generator,
                async_,
            }));
        }
        (func_obj, temp)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Builtin Helper Methods
    // ═══════════════════════════════════════════════════════════════════════════

    /// Intern a string
    pub fn intern(&mut self, s: &str) -> JsString {
        self.string_dict.get_or_insert(s)
    }

    /// Create a PropertyKey from a string
    pub fn key(&mut self, s: &str) -> PropertyKey {
        PropertyKey::String(self.string_dict.get_or_insert(s))
    }

    /// Create an array with a temporary guard.
    /// Returns (array, temp_guard). Caller must transfer ownership before guard is dropped.
    pub fn create_array(&mut self, elements: Vec<JsValue>) -> (Gc<JsObject>, Guard<JsObject>) {
        let len = elements.len() as u32;
        let temp = self.heap.create_guard();
        let arr = temp.alloc();
        {
            let mut arr_ref = arr.borrow_mut();
            arr_ref.prototype = Some(self.array_prototype.clone());
            arr_ref.exotic = ExoticObject::Array { length: len };

            for (i, elem) in elements.iter().enumerate() {
                arr_ref.set_property(PropertyKey::Index(i as u32), elem.clone());
            }

            arr_ref.set_property(
                PropertyKey::String(self.string_dict.get_or_insert("length")),
                JsValue::Number(len as f64),
            );
        }

        (arr, temp)
    }

    /// Create a rooted native function (for use during initialization)
    pub fn create_native_function(
        &mut self,
        name: &str,
        func: NativeFn,
        arity: usize,
    ) -> Gc<JsObject> {
        let name_str = self.string_dict.get_or_insert(name);
        let func_obj = self.root_guard.alloc();
        {
            let mut f_ref = func_obj.borrow_mut();
            f_ref.prototype = Some(self.function_prototype.clone());
            f_ref.exotic = ExoticObject::Function(JsFunction::Native(NativeFunction {
                name: name_str,
                func,
                arity,
            }));
        }
        func_obj
    }

    /// Create a function object from any JsFunction variant (for bind, etc.)
    pub fn create_function(&mut self, func: JsFunction) -> Gc<JsObject> {
        let func_obj = self.root_guard.alloc();
        {
            let mut f_ref = func_obj.borrow_mut();
            f_ref.prototype = Some(self.function_prototype.clone());
            f_ref.exotic = ExoticObject::Function(func);
        }
        func_obj
    }

    /// Register a method on an object (for builtin initialization)
    pub fn register_method(
        &mut self,
        obj: &Gc<JsObject>,
        name: &str,
        func: NativeFn,
        arity: usize,
    ) {
        let func_obj = self.create_native_function(name, func, arity);
        let key = self.key(name);
        obj.borrow_mut()
            .set_property(key, JsValue::Object(func_obj));
    }

    /// Guard a value to prevent it from being garbage collected.
    /// Returns Some(guard) if the value is an object, None otherwise.
    pub fn guard_value(&mut self, value: &JsValue) -> Option<Guard<JsObject>> {
        if let JsValue::Object(obj) = value {
            let guard = self.heap.create_guard();
            guard.guard(obj.clone());
            Some(guard)
        } else {
            None
        }
    }

    /// Create a guarded scope for multiple objects
    pub fn guarded_scope(&mut self) -> Guard<JsObject> {
        self.heap.create_guard()
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Evaluation Entry Point
    // ═══════════════════════════════════════════════════════════════════════════

    /// Evaluate source code and return the result
    pub fn eval_simple(&mut self, source: &str) -> Result<JsValue, JsError> {
        let mut parser = Parser::new(source, &mut self.string_dict);
        let program = parser.parse_program()?;
        self.execute_program(&program)
    }

    /// Execute a program
    fn execute_program(&mut self, program: &Program) -> Result<JsValue, JsError> {
        // Start execution timer for timeout checking
        self.start_execution();

        let mut result = JsValue::Undefined;

        for stmt in &program.body {
            match self.execute_statement(stmt)? {
                Completion::Normal(val) => result = val,
                Completion::Return(val) => return Ok(val),
                Completion::Break(_) => {
                    return Err(JsError::syntax_error_simple("Illegal break statement"));
                }
                Completion::Continue(_) => {
                    return Err(JsError::syntax_error_simple("Illegal continue statement"));
                }
            }
        }

        Ok(result)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Statement Execution
    // ═══════════════════════════════════════════════════════════════════════════

    fn execute_statement(&mut self, stmt: &Statement) -> Result<Completion, JsError> {
        match stmt {
            Statement::Expression(expr_stmt) => {
                // Expression statement - value is discarded, guard dropped at end of scope
                let Guarded {
                    value: val,
                    guard: _guard,
                } = self.evaluate_expression(&expr_stmt.expression)?;
                Ok(Completion::Normal(val))
            }

            Statement::VariableDeclaration(decl) => {
                self.execute_variable_declaration(decl)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Block(block) => self.execute_block(block),

            Statement::If(if_stmt) => self.execute_if(if_stmt),

            Statement::Return(ret) => {
                // Return value - guard passed to caller via Completion::Return
                let Guarded {
                    value,
                    guard: _return_guard,
                } = match &ret.argument {
                    Some(expr) => self.evaluate_expression(expr)?,
                    None => Guarded::unguarded(JsValue::Undefined),
                };
                // Root the returned object to keep it alive until caller can establish ownership
                if let JsValue::Object(ref obj) = value {
                    self.root_guard.guard(obj.clone());
                }
                Ok(Completion::Return(value))
            }

            Statement::Labeled(labeled) => self.execute_labeled(labeled),

            Statement::While(while_stmt) => self.execute_while(while_stmt),

            Statement::DoWhile(do_while_stmt) => self.execute_do_while(do_while_stmt),

            Statement::For(for_stmt) => self.execute_for(for_stmt),

            Statement::ForIn(for_in) => self.execute_for_in(for_in),

            Statement::ForOf(for_of) => self.execute_for_of(for_of),

            Statement::FunctionDeclaration(func) => {
                self.execute_function_declaration(func)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Switch(switch_stmt) => self.execute_switch(switch_stmt),

            Statement::Try(try_stmt) => self.execute_try(try_stmt),

            Statement::Throw(throw) => {
                let Guarded {
                    value,
                    guard: _guard,
                } = self.evaluate_expression(&throw.argument)?;
                // If throwing an object, root it to keep it alive until caught
                if let JsValue::Object(ref obj) = value {
                    self.root_guard.guard(obj.clone());
                }
                self.thrown_value = Some(value);
                Err(JsError::Thrown)
            }

            Statement::Break(brk) => Ok(Completion::Break(
                brk.label.as_ref().map(|l| l.name.cheap_clone()),
            )),

            Statement::Continue(cont) => Ok(Completion::Continue(
                cont.label.as_ref().map(|l| l.name.cheap_clone()),
            )),

            Statement::Empty => Ok(Completion::Normal(JsValue::Undefined)),

            Statement::ClassDeclaration(class) => {
                self.execute_class_declaration(class)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Import(import) => {
                self.execute_import(import)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Export(export) => {
                self.execute_export(export)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            // For now, skip unimplemented statements
            _ => Ok(Completion::Normal(JsValue::Undefined)),
        }
    }

    fn execute_variable_declaration(&mut self, decl: &VariableDeclaration) -> Result<(), JsError> {
        let mutable = matches!(decl.kind, VariableKind::Let | VariableKind::Var);

        for declarator in &decl.declarations {
            // Keep guard alive until bind_pattern transfers ownership to env
            let Guarded {
                value: init_value,
                guard: _init_guard,
            } = match &declarator.init {
                Some(expr) => self.evaluate_expression(expr)?,
                None => Guarded::unguarded(JsValue::Undefined),
            };

            // bind_pattern calls env_define which establishes ownership
            self.bind_pattern(&declarator.id, init_value, mutable)?;
            // _init_guard dropped here after ownership transferred
        }

        Ok(())
    }

    fn bind_pattern(
        &mut self,
        pattern: &Pattern,
        value: JsValue,
        mutable: bool,
    ) -> Result<(), JsError> {
        match pattern {
            Pattern::Identifier(id) => {
                let name = id.name.cheap_clone();
                // env_define establishes ownership for object values
                self.env_define(name, value, mutable);
                Ok(())
            }

            Pattern::Object(obj_pattern) => {
                let obj = match &value {
                    JsValue::Object(o) => o.clone(),
                    _ => return Err(JsError::type_error("Cannot destructure non-object")),
                };

                for prop in &obj_pattern.properties {
                    match prop {
                        ObjectPatternProperty::KeyValue {
                            key,
                            value: pat,
                            shorthand,
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
                                .get_property(&PropertyKey::String(key_str.cheap_clone()))
                                .unwrap_or(JsValue::Undefined);

                            // For shorthand { x }, bind directly. For { x = default }, use bind_pattern
                            if *shorthand {
                                // Check if it's a simple identifier or has a default value
                                if matches!(pat, Pattern::Identifier(_)) {
                                    self.env_define(key_str, prop_value, mutable);
                                } else {
                                    // It's shorthand with default (e.g., { y = 10 })
                                    self.bind_pattern(pat, prop_value, mutable)?;
                                }
                            } else {
                                self.bind_pattern(pat, prop_value, mutable)?;
                            }
                        }
                        ObjectPatternProperty::Rest(_rest) => {
                            // Rest patterns in object destructuring not yet implemented
                            return Err(JsError::internal_error(
                                "Object rest patterns not yet implemented",
                            ));
                        }
                    }
                }

                Ok(())
            }

            Pattern::Array(arr_pattern) => self.bind_array_pattern(arr_pattern, &value, mutable),

            Pattern::Rest(rest) => {
                // Rest at top level in bind_pattern means we have an identifier to bind
                self.bind_pattern(&rest.argument, value, mutable)
            }

            Pattern::Assignment(assign_pat) => {
                // Assignment pattern: { x = defaultValue }
                let (val, _guard) = if matches!(value, JsValue::Undefined) {
                    // Use default value
                    let Guarded { value: v, guard: g } =
                        self.evaluate_expression(&assign_pat.right)?;
                    (v, g)
                } else {
                    (value, None)
                };
                self.bind_pattern(&assign_pat.left, val, mutable)
            }
        }
    }

    fn bind_array_pattern(
        &mut self,
        arr_pattern: &ArrayPattern,
        value: &JsValue,
        mutable: bool,
    ) -> Result<(), JsError> {
        let items: Vec<JsValue> = match value {
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
            if let Some(pat) = elem {
                match pat {
                    Pattern::Rest(rest) => {
                        // Collect remaining items into an array
                        let remaining: Vec<JsValue> = items.get(i..).unwrap_or(&[]).to_vec();
                        let (rest_array, _guard) = self.create_array_with_guard(remaining);
                        self.bind_pattern(&rest.argument, JsValue::Object(rest_array), mutable)?;
                        break; // Rest must be last
                    }
                    _ => {
                        let item = items.get(i).cloned().unwrap_or(JsValue::Undefined);
                        self.bind_pattern(pat, item, mutable)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn execute_block(&mut self, block: &BlockStatement) -> Result<Completion, JsError> {
        let saved_env = self.push_scope();

        let mut result = Completion::Normal(JsValue::Undefined);
        for stmt in &block.body {
            result = self.execute_statement(stmt)?;
            match &result {
                Completion::Normal(_) => {}
                _ => break,
            }
        }

        self.pop_scope(saved_env);
        Ok(result)
    }

    fn execute_if(&mut self, if_stmt: &IfStatement) -> Result<Completion, JsError> {
        let Guarded {
            value: condition,
            guard: _guard,
        } = self.evaluate_expression(&if_stmt.test)?;

        if condition.to_boolean() {
            self.execute_statement(&if_stmt.consequent)
        } else if let Some(ref alternate) = if_stmt.alternate {
            self.execute_statement(alternate)
        } else {
            Ok(Completion::Normal(JsValue::Undefined))
        }
    }

    fn execute_while(&mut self, while_stmt: &WhileStatement) -> Result<Completion, JsError> {
        loop {
            // Check for timeout at each iteration
            self.check_timeout()?;
            let Guarded {
                value: condition,
                guard: _guard,
            } = self.evaluate_expression(&while_stmt.test)?;
            if !condition.to_boolean() {
                break;
            }

            match self.execute_statement(&while_stmt.body)? {
                Completion::Normal(_) | Completion::Continue(None) => {}
                Completion::Break(None) => break,
                Completion::Return(v) => return Ok(Completion::Return(v)),
                c @ (Completion::Break(_) | Completion::Continue(_)) => return Ok(c),
            }
        }

        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_for(&mut self, for_stmt: &ForStatement) -> Result<Completion, JsError> {
        // Handle var declarations BEFORE creating loop scope (var is function-scoped)
        let has_var_init =
            matches!(&for_stmt.init, Some(ForInit::Variable(d)) if d.kind == VariableKind::Var);
        if has_var_init {
            if let Some(ForInit::Variable(decl)) = &for_stmt.init {
                self.execute_variable_declaration(decl)?;
            }
        }

        let saved_env = self.push_scope();

        // Extract loop variable names for let/const (for per-iteration binding)
        let loop_vars: Vec<(JsString, bool)> = match &for_stmt.init {
            Some(ForInit::Variable(decl)) if decl.kind != VariableKind::Var => {
                let mutable = decl.kind == VariableKind::Let;
                decl.declarations
                    .iter()
                    .filter_map(|d| {
                        if let Pattern::Identifier(id) = &d.id {
                            Some((JsString::from(id.name.as_str()), mutable))
                        } else {
                            None // Skip destructuring patterns for now
                        }
                    })
                    .collect()
            }
            _ => vec![],
        };
        let has_per_iteration_binding = !loop_vars.is_empty();

        // Init (let/const declarations go in loop scope, or handle expressions)
        if let Some(ref init) = for_stmt.init {
            match init {
                ForInit::Variable(decl) => {
                    // Only execute non-var declarations here (var already handled above)
                    if decl.kind != VariableKind::Var {
                        self.execute_variable_declaration(decl)?;
                    }
                }
                ForInit::Expression(expr) => {
                    // Init expression - value discarded
                    let Guarded {
                        value: _,
                        guard: _guard,
                    } = self.evaluate_expression(expr)?;
                }
            }
        }

        // Per ES spec, create initial per-iteration environment before loop starts
        // This is where the initial values live
        if has_per_iteration_binding {
            let loop_scope_env = self.env.clone();
            let iter_env = create_environment_with_guard(&self.root_guard, Some(saved_env.clone()));
            for (name, mutable) in &loop_vars {
                let value = self.env_get(name)?;
                let mut env_ref = iter_env.borrow_mut();
                if let Some(data) = env_ref.as_environment_mut() {
                    data.bindings.insert(
                        name.cheap_clone(),
                        Binding {
                            value,
                            mutable: *mutable,
                            initialized: true,
                        },
                    );
                }
                drop(env_ref);
            }
            // Discard loop_scope_env - from now on we use per-iteration environments
            let _ = loop_scope_env;
            self.env = iter_env;
        }

        // Loop
        loop {
            // Check for timeout at each iteration
            self.check_timeout()?;

            // Test (runs in current per-iteration environment)
            if let Some(ref test) = for_stmt.test {
                let Guarded {
                    value: condition,
                    guard: _guard,
                } = self.evaluate_expression(test)?;
                if !condition.to_boolean() {
                    break;
                }
            }

            // Body (runs in current per-iteration environment)
            let body_result = self.execute_statement(&for_stmt.body)?;

            // Handle control flow
            match body_result {
                Completion::Normal(_) | Completion::Continue(None) => {}
                Completion::Break(None) => break,
                Completion::Return(v) => {
                    self.pop_scope(saved_env);
                    return Ok(Completion::Return(v));
                }
                c @ (Completion::Break(_) | Completion::Continue(_)) => {
                    self.pop_scope(saved_env);
                    return Ok(c);
                }
            }

            // Per ES spec: Create NEW per-iteration env BEFORE update
            // This preserves the body's captured values while update modifies new env
            if has_per_iteration_binding {
                let current_env = self.env.clone();
                let new_iter_env =
                    create_environment_with_guard(&self.root_guard, Some(saved_env.clone()));
                for (name, mutable) in &loop_vars {
                    // Copy value from current iteration environment
                    let value = {
                        let env_ref = current_env.borrow();
                        if let Some(data) = env_ref.as_environment() {
                            data.bindings
                                .get(name)
                                .map(|b| b.value.clone())
                                .unwrap_or(JsValue::Undefined)
                        } else {
                            JsValue::Undefined
                        }
                    };
                    let mut env_ref = new_iter_env.borrow_mut();
                    if let Some(data) = env_ref.as_environment_mut() {
                        data.bindings.insert(
                            name.cheap_clone(),
                            Binding {
                                value,
                                mutable: *mutable,
                                initialized: true,
                            },
                        );
                    }
                }
                self.env = new_iter_env;
            }

            // Update (runs in NEW per-iteration environment, not the one body used)
            if let Some(ref update) = for_stmt.update {
                let Guarded {
                    value: _,
                    guard: _guard,
                } = self.evaluate_expression(update)?;
            }
        }

        self.pop_scope(saved_env);
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_do_while(&mut self, do_while: &DoWhileStatement) -> Result<Completion, JsError> {
        loop {
            // Check for timeout at each iteration
            self.check_timeout()?;

            match self.execute_statement(&do_while.body)? {
                Completion::Normal(_) | Completion::Continue(None) => {}
                Completion::Break(None) => break,
                Completion::Return(v) => return Ok(Completion::Return(v)),
                c @ (Completion::Break(_) | Completion::Continue(_)) => return Ok(c),
            }

            let Guarded {
                value: condition,
                guard: _guard,
            } = self.evaluate_expression(&do_while.test)?;
            if !condition.to_boolean() {
                break;
            }
        }

        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_for_in(&mut self, for_in: &ForInStatement) -> Result<Completion, JsError> {
        let Guarded {
            value: right,
            guard: _right_guard,
        } = self.evaluate_expression(&for_in.right)?;

        let keys = match &right {
            JsValue::Object(obj) => {
                let obj_ref = obj.borrow();
                obj_ref
                    .properties
                    .iter()
                    .filter(|(key, prop)| prop.enumerable() && !key.is_symbol())
                    .map(|(key, _)| key.to_string())
                    .collect::<Vec<_>>()
            }
            _ => vec![],
        };

        let prev_env = self.env.clone();

        for key in keys {
            // Check for timeout at each iteration
            self.check_timeout()?;

            // Per-iteration environment
            let iter_env = create_environment_with_guard(&self.root_guard, Some(prev_env.clone()));
            self.env = iter_env;

            let key_value = JsValue::String(JsString::from(key));

            match &for_in.left {
                ForInOfLeft::Variable(decl) => {
                    let mutable = decl.kind != VariableKind::Const;
                    if let Some(declarator) = decl.declarations.first() {
                        self.bind_pattern(&declarator.id, key_value, mutable)?;
                    }
                }
                ForInOfLeft::Pattern(pattern) => {
                    self.assign_pattern(pattern, key_value)?;
                }
            }

            match self.execute_statement(&for_in.body)? {
                Completion::Break(None) => {
                    self.env = prev_env;
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => {
                    self.env = prev_env;
                    return Ok(Completion::Break(lbl));
                }
                Completion::Continue(None) => continue,
                Completion::Continue(lbl) => {
                    self.env = prev_env;
                    return Ok(Completion::Continue(lbl));
                }
                Completion::Return(val) => {
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {}
            }
        }

        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_for_of(&mut self, for_of: &ForOfStatement) -> Result<Completion, JsError> {
        let Guarded {
            value: right,
            guard: _right_guard,
        } = self.evaluate_expression(&for_of.right)?;

        // Collect items to iterate over
        let items = match &right {
            JsValue::Object(obj) => {
                let obj_ref = obj.borrow();
                match &obj_ref.exotic {
                    ExoticObject::Array { length } => {
                        let mut items = Vec::with_capacity(*length as usize);
                        for i in 0..*length {
                            items.push(
                                obj_ref
                                    .get_property(&PropertyKey::Index(i))
                                    .unwrap_or(JsValue::Undefined),
                            );
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

        let prev_env = self.env.clone();

        for item in items {
            // Check for timeout at each iteration
            self.check_timeout()?;

            // Per-iteration environment
            let iter_env = create_environment_with_guard(&self.root_guard, Some(prev_env.clone()));
            self.env = iter_env;

            match &for_of.left {
                ForInOfLeft::Variable(decl) => {
                    let mutable = decl.kind != VariableKind::Const;
                    if let Some(declarator) = decl.declarations.first() {
                        self.bind_pattern(&declarator.id, item, mutable)?;
                    }
                }
                ForInOfLeft::Pattern(pattern) => {
                    self.assign_pattern(pattern, item)?;
                }
            }

            match self.execute_statement(&for_of.body)? {
                Completion::Break(None) => {
                    self.env = prev_env;
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => {
                    self.env = prev_env;
                    return Ok(Completion::Break(lbl));
                }
                Completion::Continue(None) => continue,
                Completion::Continue(lbl) => {
                    self.env = prev_env;
                    return Ok(Completion::Continue(lbl));
                }
                Completion::Return(val) => {
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {}
            }
        }

        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_switch(&mut self, switch: &SwitchStatement) -> Result<Completion, JsError> {
        let Guarded {
            value: discriminant,
            guard: _disc_guard,
        } = self.evaluate_expression(&switch.discriminant)?;
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
                    let Guarded {
                        value: test,
                        guard: _test_guard,
                    } = self.evaluate_expression(test_expr)?;
                    if discriminant.strict_equals(&test) {
                        matched = true;
                    }
                }
            }

            if matched {
                for stmt in &case.consequent {
                    match self.execute_statement(stmt)? {
                        Completion::Break(None) => {
                            return Ok(Completion::Normal(JsValue::Undefined))
                        }
                        Completion::Return(val) => return Ok(Completion::Return(val)),
                        Completion::Continue(label) => return Ok(Completion::Continue(label)),
                        Completion::Break(label) => return Ok(Completion::Break(label)),
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
                            Completion::Break(None) => {
                                return Ok(Completion::Normal(JsValue::Undefined))
                            }
                            Completion::Return(val) => return Ok(Completion::Return(val)),
                            Completion::Continue(label) => return Ok(Completion::Continue(label)),
                            Completion::Break(label) => return Ok(Completion::Break(label)),
                            Completion::Normal(_) => {}
                        }
                    }
                }
            }
        }

        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_try(&mut self, try_stmt: &TryStatement) -> Result<Completion, JsError> {
        let result = self.execute_block(&try_stmt.block);

        match result {
            Ok(completion) => {
                // No error thrown, run finalizer if present
                if let Some(ref finalizer) = try_stmt.finalizer {
                    self.execute_block(finalizer)?;
                }
                Ok(completion)
            }
            Err(err) => {
                if let Some(ref handler) = try_stmt.handler {
                    // Get the error value
                    let error_value = match &err {
                        JsError::Thrown => self.thrown_value.take().unwrap_or(JsValue::Undefined),
                        JsError::ThrownValue { value } => value.clone(),
                        _ => JsValue::from(err.to_string()),
                    };

                    // Create catch scope
                    let prev_env = self.env.clone();
                    let catch_env =
                        create_environment_with_guard(&self.root_guard, Some(prev_env.clone()));
                    self.env = catch_env;

                    // Bind error parameter if present
                    if let Some(ref param) = handler.param {
                        self.bind_pattern(param, error_value, true)?;
                    }

                    let catch_result = self.execute_block(&handler.body);
                    self.env = prev_env;

                    // Run finalizer if present
                    if let Some(ref finalizer) = try_stmt.finalizer {
                        self.execute_block(finalizer)?;
                    }

                    catch_result
                } else if let Some(ref finalizer) = try_stmt.finalizer {
                    // No handler, just run finalizer and re-throw
                    self.execute_block(finalizer)?;
                    Err(err)
                } else {
                    // No handler or finalizer
                    Err(err)
                }
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Import/Export Implementation
    // ═══════════════════════════════════════════════════════════════════════════

    fn execute_import(&mut self, import: &crate::ast::ImportDeclaration) -> Result<(), JsError> {
        let specifier = import.source.value.to_string();

        // Resolve the module
        let module_obj = self.resolve_module(&specifier)?;

        // Bind imported names to current environment
        for spec in &import.specifiers {
            match spec {
                crate::ast::ImportSpecifier::Named {
                    local, imported, ..
                } => {
                    // import { foo as bar } from "module"
                    let import_key = self.key(imported.name.as_str());
                    let value = module_obj
                        .borrow()
                        .get_property(&import_key)
                        .unwrap_or(JsValue::Undefined);
                    self.env_define(local.name.cheap_clone(), value, false);
                }
                crate::ast::ImportSpecifier::Default { local, .. } => {
                    // import foo from "module"
                    let default_key = self.key("default");
                    let value = module_obj
                        .borrow()
                        .get_property(&default_key)
                        .unwrap_or(JsValue::Undefined);
                    self.env_define(local.name.cheap_clone(), value, false);
                }
                crate::ast::ImportSpecifier::Namespace { local, .. } => {
                    // import * as foo from "module"
                    self.env_define(
                        local.name.cheap_clone(),
                        JsValue::Object(module_obj.clone()),
                        false,
                    );
                }
            }
        }

        Ok(())
    }

    fn execute_export(&mut self, export: &crate::ast::ExportDeclaration) -> Result<(), JsError> {
        // Handle export declaration (e.g., export function foo() {})
        if let Some(decl) = &export.declaration {
            self.execute_statement(decl)?;

            // For named declarations, add to exports
            match decl.as_ref() {
                Statement::FunctionDeclaration(func) => {
                    if let Some(id) = &func.id {
                        let value = self.env_get(&id.name)?;
                        let export_name = if export.default {
                            JsString::from("default")
                        } else {
                            id.name.cheap_clone()
                        };
                        self.exports.insert(export_name, value);
                    }
                }
                Statement::VariableDeclaration(var_decl) => {
                    for declarator in &var_decl.declarations {
                        if let Pattern::Identifier(id) = &declarator.id {
                            let value = self.env_get(&id.name)?;
                            self.exports.insert(id.name.cheap_clone(), value);
                        }
                    }
                }
                Statement::ClassDeclaration(class) => {
                    if let Some(id) = &class.id {
                        let value = self.env_get(&id.name)?;
                        let export_name = if export.default {
                            JsString::from("default")
                        } else {
                            id.name.cheap_clone()
                        };
                        self.exports.insert(export_name, value);
                    }
                }
                _ => {}
            }
        }

        // Handle re-exports: export { foo } from "module"
        if let Some(source) = &export.source {
            let module_obj = self.resolve_module(source.value.as_ref())?;
            for spec in &export.specifiers {
                let import_key = self.key(spec.local.name.as_str());
                let value = module_obj
                    .borrow()
                    .get_property(&import_key)
                    .unwrap_or(JsValue::Undefined);
                self.exports.insert(spec.exported.name.cheap_clone(), value);
            }
        } else if !export.specifiers.is_empty() {
            // Handle named exports: export { foo, bar }
            for spec in &export.specifiers {
                let value = self.env_get(&spec.local.name)?;
                self.exports.insert(spec.exported.name.cheap_clone(), value);
            }
        }

        Ok(())
    }

    /// Resolve a module specifier to a module namespace object
    fn resolve_module(&mut self, specifier: &str) -> Result<Gc<JsObject>, JsError> {
        // Check internal modules first
        if let Some(module) = self.resolve_internal_module(specifier)? {
            return Ok(module);
        }

        // Check loaded external modules
        if let Some(module) = self.loaded_modules.get(specifier) {
            return Ok(module.clone());
        }

        Err(JsError::reference_error(format!(
            "Module '{}' not found",
            specifier
        )))
    }

    /// Resolve an internal module (creates module object on first access)
    fn resolve_internal_module(
        &mut self,
        specifier: &str,
    ) -> Result<Option<Gc<JsObject>>, JsError> {
        // Return cached if exists
        if let Some(cached) = self.internal_module_cache.get(specifier) {
            return Ok(Some(cached.clone()));
        }

        // Check if it's a registered internal module
        if !self.internal_modules.contains_key(specifier) {
            return Ok(None);
        }

        // Get module definition - we need to clone to avoid borrow issues
        let module_kind = {
            let module = self
                .internal_modules
                .get(specifier)
                .ok_or_else(|| JsError::internal_error("Module disappeared"))?;
            module.kind.clone()
        };

        // Create module object based on kind
        // Returns (module_obj, temp_guard) - we must root before temp_guard is dropped
        let (module_obj, _temp_guard) = match module_kind {
            crate::InternalModuleKind::Native(exports) => {
                self.create_native_module_object(&exports)?
            }
            crate::InternalModuleKind::Source(source) => {
                self.create_source_module_object(specifier, &source)?
            }
        };

        // Root the module (lives forever) - must happen before _temp_guard is dropped
        self.root_guard.guard(module_obj.clone());

        // Cache it
        self.internal_module_cache
            .insert(specifier.to_string(), module_obj.clone());

        Ok(Some(module_obj))
    }

    /// Create module object from native exports
    fn create_native_module_object(
        &mut self,
        exports: &[(String, crate::InternalExport)],
    ) -> Result<(Gc<JsObject>, Guard<JsObject>), JsError> {
        let (module_obj, guard) = self.create_object_with_guard();

        for (name, export) in exports {
            let key = self.key(name);
            let value = match export {
                crate::InternalExport::Function {
                    name: fn_name,
                    func,
                    arity,
                } => {
                    let fn_obj = self.create_internal_function(fn_name, *func, *arity);
                    JsValue::Object(fn_obj)
                }
                crate::InternalExport::Value(v) => v.clone(),
            };
            module_obj.borrow_mut().set_property(key, value);
        }

        Ok((module_obj, guard))
    }

    /// Create module object from TypeScript source
    fn create_source_module_object(
        &mut self,
        _specifier: &str,
        source: &str,
    ) -> Result<(Gc<JsObject>, Guard<JsObject>), JsError> {
        // Parse the source
        let mut parser = Parser::new(source, &mut self.string_dict);
        let program = parser.parse_program()?;

        // Save current environment and exports
        let saved_env = self.env.clone();
        let saved_exports = std::mem::take(&mut self.exports);

        // Create module environment
        let module_env = self.create_module_environment();
        self.env = module_env;

        // Execute the module body
        let result = self.execute_program(&program);

        // Restore environment
        self.env = saved_env;

        // Handle errors
        result?;

        // Create module namespace object from exports
        let (module_obj, guard) = self.create_object_with_guard();

        // Drain exports to a vector to avoid borrow conflict
        let exports: Vec<_> = self.exports.drain().collect();

        // Copy exports to module object
        for (name, value) in exports {
            let key = self.key(name.as_str());
            module_obj.borrow_mut().set_property(key, value);
        }

        // Restore saved exports
        self.exports = saved_exports;

        Ok((module_obj, guard))
    }

    /// Create a function from an InternalFn
    fn create_internal_function(
        &mut self,
        name: &str,
        func: crate::InternalFn,
        arity: usize,
    ) -> Gc<JsObject> {
        // InternalFn and NativeFn have the same signature, so we can use it directly
        self.create_native_function(name, func, arity)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Class Implementation
    // ═══════════════════════════════════════════════════════════════════════════

    fn execute_class_declaration(&mut self, class: &ClassDeclaration) -> Result<(), JsError> {
        let constructor_fn = self.create_class_constructor(class)?;

        // Bind the class name first (so static blocks can reference it)
        if let Some(id) = &class.id {
            self.env_define(
                JsString::from(id.name.as_str()),
                JsValue::Object(constructor_fn.cheap_clone()),
                false,
            );
        }

        // Execute static blocks - they can reference the class name
        for member in &class.body.members {
            if let ClassMember::StaticBlock(block) = member {
                for stmt in &block.body {
                    if let Completion::Return(_) = self.execute_statement(stmt)? {
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
    ) -> Result<Gc<JsObject>, JsError> {
        // Handle extends - evaluate superclass first
        let (super_constructor, _super_guard): (Option<Gc<JsObject>>, Option<Guard<JsObject>>) =
            if let Some(super_class_expr) = &class.super_class {
                let Guarded {
                    value: super_val,
                    guard,
                } = self.evaluate_expression(super_class_expr)?;
                if let JsValue::Object(sc) = super_val {
                    (Some(sc), guard)
                } else {
                    return Err(JsError::type_error(
                        "Class extends value is not a constructor",
                    ));
                }
            } else {
                (None, None)
            };

        // Create prototype object
        let (prototype, _proto_guard) = self.create_object_with_guard();
        self.root_guard.guard(prototype.clone());

        // If we have a superclass, set up prototype chain
        if let Some(ref super_ctor) = super_constructor {
            let proto_key = self.key("prototype");
            let super_proto = super_ctor.borrow().get_property(&proto_key);
            if let Some(JsValue::Object(sp)) = super_proto {
                prototype.borrow_mut().prototype = Some(sp.cheap_clone());
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
        #[allow(clippy::type_complexity)]
        let mut accessors: FxHashMap<JsString, (Option<Gc<JsObject>>, Option<Gc<JsObject>>)> =
            FxHashMap::default();
        let mut regular_methods: Vec<(JsString, Gc<JsObject>)> = Vec::new();

        for method in &instance_methods {
            let method_name: JsString = match &method.key {
                ObjectPropertyKey::Identifier(id) => JsString::from(id.name.as_str()),
                ObjectPropertyKey::String(s) => s.value.cheap_clone(),
                ObjectPropertyKey::Number(lit) => match &lit.value {
                    LiteralValue::Number(n) => JsString::from(n.to_string()),
                    _ => continue,
                },
                ObjectPropertyKey::Computed(_) => continue,
                ObjectPropertyKey::PrivateIdentifier(id) => JsString::from(format!("#{}", id.name)),
            };

            let func = &method.value;
            let (func_obj, _func_guard) = self.create_function_with_guard(
                Some(method_name.cheap_clone()),
                Rc::from(func.params.as_slice()),
                Rc::new(FunctionBody::Block(func.body.clone())),
                self.env.clone(),
                func.span,
                func.generator,
                func.async_,
            );
            self.root_guard.guard(func_obj.clone());

            // Store __super__ on method so super.method() works
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

        // Add accessor properties to prototype
        for (name, (getter, setter)) in accessors {
            prototype.borrow_mut().define_property(
                PropertyKey::String(name),
                Property::accessor(getter, setter),
            );
        }

        // Add regular methods to prototype
        for (name, func_obj) in regular_methods {
            prototype
                .borrow_mut()
                .set_property(PropertyKey::String(name), JsValue::Object(func_obj));
        }

        // Build constructor body that initializes instance fields then runs user constructor
        let field_initializers: Vec<(JsString, Option<Expression>)> = instance_fields
            .iter()
            .filter_map(|prop| {
                let name: JsString = match &prop.key {
                    ObjectPropertyKey::Identifier(id) => JsString::from(id.name.as_str()),
                    ObjectPropertyKey::String(s) => s.value.cheap_clone(),
                    ObjectPropertyKey::PrivateIdentifier(id) => {
                        JsString::from(format!("#{}", id.name))
                    }
                    _ => return None,
                };
                Some((name, prop.value.clone()))
            })
            .collect();

        // Create the constructor function
        let ctor_body = if let Some(ctor) = constructor {
            ctor.body.clone()
        } else {
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

        let (constructor_fn, _ctor_guard) = self.create_function_with_guard(
            class.id.as_ref().map(|id| JsString::from(id.name.as_str())),
            Rc::from(ctor_params),
            Rc::new(FunctionBody::Block(ctor_body)),
            self.env.clone(),
            class.span,
            false,
            false,
        );
        self.root_guard.guard(constructor_fn.clone());

        // Store prototype on constructor
        {
            let proto_key = self.key("prototype");
            constructor_fn
                .borrow_mut()
                .set_property(proto_key, JsValue::Object(prototype.cheap_clone()));
        }

        // Store field initializers in __fields__ if there are any
        if !field_initializers.is_empty() {
            let mut field_values: Vec<(JsString, JsValue)> = Vec::new();
            for (name, value_expr) in field_initializers {
                let value = if let Some(expr) = value_expr {
                    self.evaluate_expression(&expr)
                        .map(|g| g.value)
                        .unwrap_or(JsValue::Undefined)
                } else {
                    JsValue::Undefined
                };
                field_values.push((name, value));
            }

            // Create the fields array
            let mut field_pairs: Vec<JsValue> = Vec::new();
            for (name, value) in field_values {
                let (pair, _pair_guard) =
                    self.create_array_with_guard(vec![JsValue::String(name), value]);
                self.root_guard.guard(pair.clone());
                field_pairs.push(JsValue::Object(pair));
            }

            let (fields_array, _fields_guard) = self.create_array_with_guard(field_pairs);
            self.root_guard.guard(fields_array.clone());

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

        // Handle static methods
        #[allow(clippy::type_complexity)]
        let mut static_accessors: FxHashMap<
            JsString,
            (Option<Gc<JsObject>>, Option<Gc<JsObject>>),
        > = FxHashMap::default();
        let mut static_regular_methods: Vec<(JsString, Gc<JsObject>)> = Vec::new();

        for method in &static_methods {
            let method_name: JsString = match &method.key {
                ObjectPropertyKey::Identifier(id) => JsString::from(id.name.as_str()),
                ObjectPropertyKey::String(s) => s.value.cheap_clone(),
                ObjectPropertyKey::Number(lit) => match &lit.value {
                    LiteralValue::Number(n) => JsString::from(n.to_string()),
                    _ => continue,
                },
                ObjectPropertyKey::Computed(_) => continue,
                ObjectPropertyKey::PrivateIdentifier(id) => JsString::from(format!("#{}", id.name)),
            };

            let func = &method.value;
            let (func_obj, _func_guard) = self.create_function_with_guard(
                Some(method_name.cheap_clone()),
                Rc::from(func.params.as_slice()),
                Rc::new(FunctionBody::Block(func.body.clone())),
                self.env.clone(),
                func.span,
                func.generator,
                func.async_,
            );
            self.root_guard.guard(func_obj.clone());

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
                ObjectPropertyKey::Identifier(id) => JsString::from(id.name.as_str()),
                ObjectPropertyKey::String(s) => s.value.cheap_clone(),
                ObjectPropertyKey::PrivateIdentifier(id) => JsString::from(format!("#{}", id.name)),
                _ => continue,
            };

            let (value, _value_guard) = if let Some(expr) = &prop.value {
                let Guarded { value: v, guard: g } = self.evaluate_expression(expr)?;
                (v, g)
            } else {
                (JsValue::Undefined, None)
            };

            constructor_fn
                .borrow_mut()
                .set_property(PropertyKey::String(name), value);
        }

        // Set prototype.constructor = constructor
        {
            let ctor_key = self.key("constructor");
            prototype
                .borrow_mut()
                .set_property(ctor_key, JsValue::Object(constructor_fn.cheap_clone()));
        }

        Ok(constructor_fn)
    }

    fn create_class_from_expression(
        &mut self,
        class_expr: &ClassExpression,
    ) -> Result<Gc<JsObject>, JsError> {
        // Convert ClassExpression to ClassDeclaration
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
        self.create_class_constructor(&class_decl)
    }

    fn execute_labeled(&mut self, labeled: &LabeledStatement) -> Result<Completion, JsError> {
        let label_name = labeled.label.name.cheap_clone();

        // Execute loop statements with the label so they can handle labeled break/continue
        match labeled.body.as_ref() {
            Statement::For(for_stmt) => self.execute_for_labeled(for_stmt, Some(&label_name)),
            Statement::ForIn(for_in) => self.execute_for_in_labeled(for_in, Some(&label_name)),
            Statement::ForOf(for_of) => self.execute_for_of_labeled(for_of, Some(&label_name)),
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

    fn execute_while_labeled(
        &mut self,
        while_stmt: &WhileStatement,
        label: Option<&JsString>,
    ) -> Result<Completion, JsError> {
        loop {
            // Check for timeout at each iteration
            self.check_timeout()?;
            let Guarded {
                value: condition,
                guard: _guard,
            } = self.evaluate_expression(&while_stmt.test)?;
            if !condition.to_boolean() {
                break;
            }

            match self.execute_statement(&while_stmt.body)? {
                Completion::Normal(_) | Completion::Continue(None) => {}
                Completion::Continue(Some(ref l)) if label == Some(l) => continue,
                c @ Completion::Continue(_) => return Ok(c),
                Completion::Break(None) => break,
                Completion::Break(Some(ref l)) if label == Some(l) => break,
                c @ Completion::Break(_) => return Ok(c),
                Completion::Return(v) => return Ok(Completion::Return(v)),
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
                Completion::Normal(_) | Completion::Continue(None) => {}
                Completion::Continue(Some(ref l)) if label == Some(l) => {}
                c @ Completion::Continue(_) => return Ok(c),
                Completion::Break(None) => break,
                Completion::Break(Some(ref l)) if label == Some(l) => break,
                c @ Completion::Break(_) => return Ok(c),
                Completion::Return(v) => return Ok(Completion::Return(v)),
            }

            let Guarded {
                value: condition,
                guard: _guard,
            } = self.evaluate_expression(&do_while.test)?;
            if !condition.to_boolean() {
                break;
            }
        }

        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_for_labeled(
        &mut self,
        for_stmt: &ForStatement,
        label: Option<&JsString>,
    ) -> Result<Completion, JsError> {
        // Create new scope for the for loop variables
        let saved_env = self.push_scope();

        // Initialize
        if let Some(ref init) = for_stmt.init {
            match init {
                ForInit::Variable(decl) => self.execute_variable_declaration(decl)?,
                ForInit::Expression(expr) => {
                    let Guarded {
                        value: _,
                        guard: _guard,
                    } = self.evaluate_expression(expr)?;
                }
            }
        }

        loop {
            // Check for timeout at each iteration
            self.check_timeout()?;

            // Test
            if let Some(ref test) = for_stmt.test {
                let Guarded {
                    value: condition,
                    guard: _guard,
                } = self.evaluate_expression(test)?;
                if !condition.to_boolean() {
                    break;
                }
            }

            // Body
            match self.execute_statement(&for_stmt.body)? {
                Completion::Normal(_) | Completion::Continue(None) => {}
                Completion::Continue(Some(ref l)) if label == Some(l) => {}
                c @ Completion::Continue(_) => {
                    self.pop_scope(saved_env);
                    return Ok(c);
                }
                Completion::Break(None) => break,
                Completion::Break(Some(ref l)) if label == Some(l) => break,
                c @ Completion::Break(_) => {
                    self.pop_scope(saved_env);
                    return Ok(c);
                }
                Completion::Return(v) => {
                    self.pop_scope(saved_env);
                    return Ok(Completion::Return(v));
                }
            }

            // Update
            if let Some(ref update) = for_stmt.update {
                let Guarded {
                    value: _,
                    guard: _guard,
                } = self.evaluate_expression(update)?;
            }
        }

        self.pop_scope(saved_env);
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_for_in_labeled(
        &mut self,
        for_in: &ForInStatement,
        label: Option<&JsString>,
    ) -> Result<Completion, JsError> {
        let Guarded {
            value: right,
            guard: _right_guard,
        } = self.evaluate_expression(&for_in.right)?;

        let keys = match &right {
            JsValue::Object(obj) => {
                let obj_ref = obj.borrow();
                obj_ref
                    .properties
                    .iter()
                    .filter(|(key, prop)| prop.enumerable() && !key.is_symbol())
                    .map(|(key, _)| key.to_string())
                    .collect::<Vec<_>>()
            }
            _ => vec![],
        };

        let prev_env = self.env.clone();

        for key in keys {
            // Check for timeout at each iteration
            self.check_timeout()?;

            // Per-iteration environment
            let iter_env = create_environment_with_guard(&self.root_guard, Some(prev_env.clone()));
            self.env = iter_env;

            let key_value = JsValue::String(JsString::from(key));

            match &for_in.left {
                ForInOfLeft::Variable(decl) => {
                    let mutable = decl.kind != VariableKind::Const;
                    if let Some(declarator) = decl.declarations.first() {
                        self.bind_pattern(&declarator.id, key_value, mutable)?;
                    }
                }
                ForInOfLeft::Pattern(pattern) => {
                    self.assign_pattern(pattern, key_value)?;
                }
            }

            match self.execute_statement(&for_in.body)? {
                Completion::Break(None) => {
                    self.env = prev_env;
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(Some(ref l)) if label == Some(l) => {
                    self.env = prev_env;
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => {
                    self.env = prev_env;
                    return Ok(Completion::Break(lbl));
                }
                Completion::Continue(None) => continue,
                Completion::Continue(Some(ref l)) if label == Some(l) => continue,
                Completion::Continue(lbl) => {
                    self.env = prev_env;
                    return Ok(Completion::Continue(lbl));
                }
                Completion::Return(val) => {
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {}
            }
        }

        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_for_of_labeled(
        &mut self,
        for_of: &ForOfStatement,
        label: Option<&JsString>,
    ) -> Result<Completion, JsError> {
        let Guarded {
            value: right,
            guard: _right_guard,
        } = self.evaluate_expression(&for_of.right)?;

        // Collect items to iterate over
        let items = match &right {
            JsValue::Object(obj) => {
                let obj_ref = obj.borrow();
                match &obj_ref.exotic {
                    ExoticObject::Array { length } => {
                        let mut items = Vec::with_capacity(*length as usize);
                        for i in 0..*length {
                            items.push(
                                obj_ref
                                    .get_property(&PropertyKey::Index(i))
                                    .unwrap_or(JsValue::Undefined),
                            );
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

        let prev_env = self.env.clone();

        for item in items {
            // Check for timeout at each iteration
            self.check_timeout()?;

            // Per-iteration environment
            let iter_env = create_environment_with_guard(&self.root_guard, Some(prev_env.clone()));
            self.env = iter_env;

            match &for_of.left {
                ForInOfLeft::Variable(decl) => {
                    let mutable = decl.kind != VariableKind::Const;
                    if let Some(declarator) = decl.declarations.first() {
                        self.bind_pattern(&declarator.id, item, mutable)?;
                    }
                }
                ForInOfLeft::Pattern(pattern) => {
                    self.assign_pattern(pattern, item)?;
                }
            }

            match self.execute_statement(&for_of.body)? {
                Completion::Break(None) => {
                    self.env = prev_env;
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(Some(ref l)) if label == Some(l) => {
                    self.env = prev_env;
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => {
                    self.env = prev_env;
                    return Ok(Completion::Break(lbl));
                }
                Completion::Continue(None) => continue,
                Completion::Continue(Some(ref l)) if label == Some(l) => continue,
                Completion::Continue(lbl) => {
                    self.env = prev_env;
                    return Ok(Completion::Continue(lbl));
                }
                Completion::Return(val) => {
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {}
            }
        }

        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_function_declaration(&mut self, func: &FunctionDeclaration) -> Result<(), JsError> {
        let name = func.id.as_ref().map(|id| id.name.cheap_clone());
        let params: Rc<[_]> = func.params.clone().into();
        let body = Rc::new(FunctionBody::Block(func.body.clone()));

        // Create function with temp guard
        let (func_obj, _temp) = self.create_function_with_guard(
            name.clone(),
            params,
            body,
            self.env.clone(),
            func.span,
            func.generator,
            func.async_,
        );

        // Transfer ownership to environment before temp guard is dropped
        if let Some(js_name) = name {
            self.env_define(js_name, JsValue::Object(func_obj), true);
        }
        // _temp is dropped here, but func_obj is alive via env ownership

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Expression Evaluation
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // Returns Guarded { value, guard } - the guard keeps newly created objects alive
    // until ownership is transferred to an environment or parent object.
    // ═══════════════════════════════════════════════════════════════════════════

    fn evaluate_expression(&mut self, expr: &Expression) -> Result<Guarded, JsError> {
        match expr {
            Expression::Literal(lit) => {
                // Handle RegExp literals specially since they need to create objects
                if let LiteralValue::RegExp { pattern, flags } = &lit.value {
                    return self.create_regexp_literal(pattern, flags);
                }
                Ok(Guarded::unguarded(self.evaluate_literal(&lit.value)?))
            }

            Expression::Identifier(id) => Ok(Guarded::unguarded(self.env_get(&id.name)?)),

            Expression::Binary(bin) => self.evaluate_binary(bin),

            Expression::Unary(un) => self.evaluate_unary(un),

            Expression::Logical(log) => self.evaluate_logical(log),

            Expression::Conditional(cond) => self.evaluate_conditional(cond),

            Expression::Assignment(assign) => self.evaluate_assignment(assign),

            Expression::Call(call) => self.evaluate_call(call),

            Expression::Member(member) => self.evaluate_member(member),

            Expression::Array(arr) => self.evaluate_array(arr),

            Expression::Object(obj) => self.evaluate_object(obj),

            Expression::ArrowFunction(arrow) => {
                let params: Rc<[_]> = arrow.params.clone().into();
                let body = Rc::new(FunctionBody::from(arrow.body.clone()));

                let (func_obj, guard) = self.create_function_with_guard(
                    None,
                    params,
                    body,
                    self.env.clone(),
                    arrow.span,
                    false,
                    arrow.async_,
                );

                Ok(Guarded::with_guard(JsValue::Object(func_obj), guard))
            }

            Expression::Function(func) => {
                let name = func.id.as_ref().map(|id| id.name.cheap_clone());
                let params: Rc<[_]> = func.params.clone().into();
                let body = Rc::new(FunctionBody::Block(func.body.clone()));

                let (func_obj, guard) = self.create_function_with_guard(
                    name,
                    params,
                    body,
                    self.env.clone(),
                    func.span,
                    func.generator,
                    func.async_,
                );

                Ok(Guarded::with_guard(JsValue::Object(func_obj), guard))
            }

            Expression::Parenthesized(inner, _) => self.evaluate_expression(inner),

            // TypeScript type assertions - just evaluate the expression, ignore the type
            Expression::TypeAssertion(ta) => self.evaluate_expression(&ta.expression),
            Expression::NonNull(nn) => self.evaluate_expression(&nn.expression),

            // Template literals
            Expression::Template(template) => self.evaluate_template_literal(template),
            Expression::TaggedTemplate(tagged) => self.evaluate_tagged_template(tagged),

            // Update expressions (++i, i++, --i, i--)
            Expression::Update(update) => self.evaluate_update(update),

            // Sequence expressions (a, b, c)
            Expression::Sequence(seq) => self.evaluate_sequence(seq),

            // New expression (constructor call)
            Expression::New(new_expr) => self.evaluate_new(new_expr),

            // This expression
            Expression::This(_) => {
                let this_name = JsString::from("this");
                Ok(Guarded::unguarded(
                    self.env_get(&this_name).unwrap_or(JsValue::Undefined),
                ))
            }

            // Class expression
            Expression::Class(class_expr) => {
                let constructor_fn = self.create_class_from_expression(class_expr)?;
                // Create guard for the returned object
                let (_, guard) = self.create_object_with_guard();
                guard.guard(constructor_fn.clone());
                Ok(Guarded::with_guard(JsValue::Object(constructor_fn), guard))
            }

            _ => Ok(Guarded::unguarded(JsValue::Undefined)),
        }
    }

    fn evaluate_new(&mut self, new_expr: &NewExpression) -> Result<Guarded, JsError> {
        // Evaluate the constructor, keeping guard alive during the call
        let Guarded {
            value: constructor,
            guard: _ctor_guard,
        } = self.evaluate_expression(&new_expr.callee)?;

        // Evaluate arguments, collecting guards
        let mut args = Vec::new();
        let mut _arg_guards = Vec::new();
        for arg in &new_expr.arguments {
            match arg {
                Argument::Expression(expr) => {
                    let Guarded { value, guard } = self.evaluate_expression(expr)?;
                    args.push(value);
                    if let Some(g) = guard {
                        _arg_guards.push(g);
                    }
                }
                Argument::Spread(_) => {
                    return Err(JsError::type_error("Spread in new not yet supported"));
                }
            }
        }

        // Create a new object
        let (new_obj, new_guard) = self.create_object_with_guard();

        // Get the constructor's prototype and __fields__ properties
        let (proto_opt, fields_opt) = if let JsValue::Object(ctor) = &constructor {
            let ctor_ref = ctor.borrow();
            let proto = ctor_ref
                .get_property(&PropertyKey::from("prototype"))
                .and_then(|v| {
                    if let JsValue::Object(p) = v {
                        Some(p)
                    } else {
                        None
                    }
                });
            let fields = ctor_ref
                .get_property(&PropertyKey::from("__fields__"))
                .and_then(|v| {
                    if let JsValue::Object(f) = v {
                        Some(f)
                    } else {
                        None
                    }
                });
            (proto, fields)
        } else {
            (None, None)
        };

        // Set prototype
        if let Some(proto) = proto_opt {
            new_obj.borrow_mut().prototype = Some(proto.cheap_clone());
        }

        // Initialize instance fields from __fields__
        if let Some(fields_array) = fields_opt {
            // Get length property
            let len = {
                let fields_ref = fields_array.borrow();
                match fields_ref.get_property(&PropertyKey::from("length")) {
                    Some(JsValue::Number(n)) => n as usize,
                    _ => 0,
                }
            };

            for i in 0..len {
                let pair_opt = {
                    let fields_ref = fields_array.borrow();
                    fields_ref.get_property(&PropertyKey::from(i.to_string()))
                };

                if let Some(JsValue::Object(pair)) = pair_opt {
                    let (name_opt, value_opt) = {
                        let pair_ref = pair.borrow();
                        // Each pair is [name, value]
                        (
                            pair_ref.get_property(&PropertyKey::from("0")),
                            pair_ref.get_property(&PropertyKey::from("1")),
                        )
                    };

                    if let (Some(JsValue::String(name)), Some(value)) = (name_opt, value_opt) {
                        new_obj
                            .borrow_mut()
                            .set_property(PropertyKey::String(name), value);
                    }
                }
            }
        }

        // Call the constructor with `this` set to the new object
        let this = JsValue::Object(new_obj);
        let result = self.call_function(constructor, this.clone(), &args)?;

        // If constructor returns an object, use that; otherwise use the created object
        match result.value {
            JsValue::Object(obj) => {
                // The result already has a guard from call_function
                Ok(Guarded {
                    value: JsValue::Object(obj),
                    guard: result.guard,
                })
            }
            _ => Ok(Guarded::with_guard(this, new_guard)),
        }
    }

    /// Evaluate a template literal (e.g., `hello ${name}`)
    fn evaluate_template_literal(
        &mut self,
        template: &TemplateLiteral,
    ) -> Result<Guarded, JsError> {
        let mut result = String::new();
        for (i, quasi) in template.quasis.iter().enumerate() {
            result.push_str(quasi.value.as_ref());
            if let Some(expr) = template.expressions.get(i) {
                let Guarded {
                    value: val,
                    guard: _guard,
                } = self.evaluate_expression(expr)?;
                result.push_str(val.to_js_string().as_ref());
            }
        }
        Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
    }

    /// Evaluate a tagged template expression (e.g., tag`hello ${name}`)
    fn evaluate_tagged_template(
        &mut self,
        tagged: &TaggedTemplateExpression,
    ) -> Result<Guarded, JsError> {
        // Evaluate the tag function
        let Guarded {
            value: tag_fn,
            guard: _tag_guard,
        } = self.evaluate_expression(&tagged.tag)?;

        // Build the strings array (first argument)
        let strings: Vec<JsValue> = tagged
            .quasi
            .quasis
            .iter()
            .map(|q| JsValue::String(q.value.cheap_clone()))
            .collect();
        let (strings_arr, strings_guard) = self.create_array_with_guard(strings.clone());

        // Add 'raw' property to strings array
        let raw: Vec<JsValue> = tagged
            .quasi
            .quasis
            .iter()
            .map(|q| JsValue::String(q.value.cheap_clone()))
            .collect();
        let (raw_array, _raw_guard) = self.create_array_with_guard(raw);

        // Set raw property and transfer ownership
        let raw_key = PropertyKey::String(self.string_dict.get_or_insert("raw"));
        strings_arr
            .borrow_mut()
            .set_property(raw_key, JsValue::Object(raw_array));

        // Evaluate all interpolated expressions (remaining arguments)
        let mut args = vec![JsValue::Object(strings_arr)];
        let mut _arg_guards = vec![strings_guard];
        for expr in &tagged.quasi.expressions {
            let Guarded { value, guard } = self.evaluate_expression(expr)?;
            args.push(value);
            if let Some(g) = guard {
                _arg_guards.push(g);
            }
        }

        // Call the tag function - propagate guard
        self.call_function(tag_fn, JsValue::Undefined, &args)
    }

    fn evaluate_literal(&self, lit: &LiteralValue) -> Result<JsValue, JsError> {
        Ok(match lit {
            LiteralValue::Null => JsValue::Null,
            LiteralValue::Undefined => JsValue::Undefined,
            LiteralValue::Boolean(b) => JsValue::Boolean(*b),
            LiteralValue::Number(n) => JsValue::Number(*n),
            LiteralValue::String(s) => JsValue::String(s.cheap_clone()),
            LiteralValue::BigInt(s) => {
                // Parse BigInt string to number (loses precision for large values)
                JsValue::Number(s.parse().unwrap_or(0.0))
            }
            LiteralValue::RegExp { .. } => JsValue::Undefined,
        })
    }

    fn evaluate_binary(&mut self, bin: &BinaryExpression) -> Result<Guarded, JsError> {
        let Guarded {
            value: left,
            guard: _left_guard,
        } = self.evaluate_expression(&bin.left)?;
        let Guarded {
            value: right,
            guard: _right_guard,
        } = self.evaluate_expression(&bin.right)?;

        Ok(Guarded::unguarded(match bin.operator {
            // Arithmetic
            BinaryOp::Add => match (&left, &right) {
                (JsValue::String(a), _) => JsValue::String(a.cheap_clone() + &right.to_js_string()),
                (_, JsValue::String(b)) => JsValue::String(left.to_js_string() + b.as_str()),
                _ => JsValue::Number(left.to_number() + right.to_number()),
            },
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
            BinaryOp::StrictEq => JsValue::Boolean(left.strict_equals(&right)),
            BinaryOp::StrictNotEq => JsValue::Boolean(!left.strict_equals(&right)),
            BinaryOp::Eq => JsValue::Boolean(self.abstract_equals(&left, &right)),
            BinaryOp::NotEq => JsValue::Boolean(!self.abstract_equals(&left, &right)),

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
            BinaryOp::LShift => {
                let lhs = left.to_number() as i32;
                let rhs = (right.to_number() as u32) & 0x1f;
                JsValue::Number((lhs << rhs) as f64)
            }
            BinaryOp::RShift => {
                let lhs = left.to_number() as i32;
                let rhs = (right.to_number() as u32) & 0x1f;
                JsValue::Number((lhs >> rhs) as f64)
            }
            BinaryOp::URShift => {
                // Must convert through i32 first for proper handling of negative numbers
                // -1.0 as u32 = 0 (wrong), but -1.0 as i32 as u32 = 4294967295 (correct)
                let lhs = (left.to_number() as i32) as u32;
                let rhs = ((right.to_number() as i32) as u32) & 0x1f;
                JsValue::Number((lhs >> rhs) as f64)
            }

            // instanceof operator
            BinaryOp::Instanceof => {
                // left instanceof right
                // right must be a constructor (function with prototype)
                let JsValue::Object(right_obj) = &right else {
                    return Err(JsError::type_error(
                        "Right-hand side of 'instanceof' is not an object",
                    ));
                };

                // Get right.prototype
                let proto_key = self.key("prototype");
                let right_proto = right_obj.borrow().get_property(&proto_key);
                let Some(JsValue::Object(right_proto_obj)) = right_proto else {
                    return Err(JsError::type_error(
                        "Function has non-object prototype in instanceof check",
                    ));
                };

                // Check if left's prototype chain contains right.prototype
                let JsValue::Object(left_obj) = &left else {
                    return Ok(Guarded::unguarded(JsValue::Boolean(false)));
                };

                let mut current = left_obj.borrow().prototype.clone();
                let target_id = right_proto_obj.id();
                while let Some(proto) = current {
                    if proto.id() == target_id {
                        return Ok(Guarded::unguarded(JsValue::Boolean(true)));
                    }
                    current = proto.borrow().prototype.clone();
                }
                JsValue::Boolean(false)
            }

            // in operator
            BinaryOp::In => {
                // "key" in object
                let JsValue::Object(obj) = &right else {
                    return Err(JsError::type_error(
                        "Cannot use 'in' operator to search for property in non-object",
                    ));
                };
                let key = PropertyKey::from(left.to_js_string());
                JsValue::Boolean(obj.borrow().has_own_property(&key))
            }
        }))
    }

    fn abstract_equals(&self, left: &JsValue, right: &JsValue) -> bool {
        match (left, right) {
            (JsValue::Undefined, JsValue::Null) | (JsValue::Null, JsValue::Undefined) => true,
            (JsValue::Number(a), JsValue::String(b)) => *a == b.parse().unwrap_or(f64::NAN),
            (JsValue::String(a), JsValue::Number(b)) => a.parse().unwrap_or(f64::NAN) == *b,
            _ => left.strict_equals(right),
        }
    }

    fn evaluate_unary(&mut self, un: &UnaryExpression) -> Result<Guarded, JsError> {
        let Guarded {
            value: operand,
            guard: _guard,
        } = self.evaluate_expression(&un.argument)?;

        Ok(Guarded::unguarded(match un.operator {
            UnaryOp::Minus => JsValue::Number(-operand.to_number()),
            UnaryOp::Plus => JsValue::Number(operand.to_number()),
            UnaryOp::Not => JsValue::Boolean(!operand.to_boolean()),
            UnaryOp::BitNot => JsValue::Number(!(operand.to_number() as i32) as f64),
            UnaryOp::Typeof => JsValue::String(JsString::from(operand.type_of())),
            UnaryOp::Void => JsValue::Undefined,
            UnaryOp::Delete => JsValue::Boolean(true),
        }))
    }

    fn evaluate_logical(&mut self, log: &LogicalExpression) -> Result<Guarded, JsError> {
        let left = self.evaluate_expression(&log.left)?;

        match log.operator {
            LogicalOp::And => {
                if !left.value.to_boolean() {
                    Ok(left)
                } else {
                    self.evaluate_expression(&log.right)
                }
            }
            LogicalOp::Or => {
                if left.value.to_boolean() {
                    Ok(left)
                } else {
                    self.evaluate_expression(&log.right)
                }
            }
            LogicalOp::NullishCoalescing => {
                if left.value.is_null_or_undefined() {
                    self.evaluate_expression(&log.right)
                } else {
                    Ok(left)
                }
            }
        }
    }

    fn evaluate_conditional(&mut self, cond: &ConditionalExpression) -> Result<Guarded, JsError> {
        let Guarded {
            value: test,
            guard: _guard,
        } = self.evaluate_expression(&cond.test)?;

        if test.to_boolean() {
            self.evaluate_expression(&cond.consequent)
        } else {
            self.evaluate_expression(&cond.alternate)
        }
    }

    fn evaluate_assignment(&mut self, assign: &AssignmentExpression) -> Result<Guarded, JsError> {
        // Evaluate RHS and keep the guard alive until ownership is transferred
        let Guarded {
            value,
            guard: _rhs_guard,
        } = self.evaluate_expression(&assign.right)?;

        match &assign.left {
            AssignmentTarget::Identifier(id) => {
                let name = id.name.cheap_clone();
                let final_value = match assign.operator {
                    AssignmentOp::Assign => value,
                    AssignmentOp::AddAssign => {
                        let current = self.env_get(&name)?;
                        match (&current, &value) {
                            (JsValue::String(a), _) => {
                                JsValue::String(a.cheap_clone() + &value.to_js_string())
                            }
                            _ => JsValue::Number(current.to_number() + value.to_number()),
                        }
                    }
                    AssignmentOp::SubAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(current.to_number() - value.to_number())
                    }
                    AssignmentOp::MulAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(current.to_number() * value.to_number())
                    }
                    AssignmentOp::DivAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(current.to_number() / value.to_number())
                    }
                    AssignmentOp::ModAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(current.to_number() % value.to_number())
                    }
                    AssignmentOp::ExpAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(current.to_number().powf(value.to_number()))
                    }
                    AssignmentOp::BitAndAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(
                            (current.to_number() as i32 & value.to_number() as i32) as f64,
                        )
                    }
                    AssignmentOp::BitOrAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(
                            (current.to_number() as i32 | value.to_number() as i32) as f64,
                        )
                    }
                    AssignmentOp::BitXorAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(
                            (current.to_number() as i32 ^ value.to_number() as i32) as f64,
                        )
                    }
                    AssignmentOp::LShiftAssign => {
                        let current = self.env_get(&name)?;
                        let lhs = current.to_number() as i32;
                        let rhs = (value.to_number() as u32) & 0x1f;
                        JsValue::Number((lhs << rhs) as f64)
                    }
                    AssignmentOp::RShiftAssign => {
                        let current = self.env_get(&name)?;
                        let lhs = current.to_number() as i32;
                        let rhs = (value.to_number() as u32) & 0x1f;
                        JsValue::Number((lhs >> rhs) as f64)
                    }
                    AssignmentOp::URShiftAssign => {
                        let current = self.env_get(&name)?;
                        let lhs = (current.to_number() as i32) as u32;
                        let rhs = ((value.to_number() as i32) as u32) & 0x1f;
                        JsValue::Number((lhs >> rhs) as f64)
                    }
                    _ => value,
                };
                // env_set establishes ownership, so _rhs_guard can be dropped after this
                self.env_set(&name, final_value.clone())?;
                Ok(Guarded::unguarded(final_value))
            }
            AssignmentTarget::Member(member) => {
                let Guarded {
                    value: obj_val,
                    guard: _obj_guard,
                } = self.evaluate_expression(&member.object)?;
                let JsValue::Object(ref obj) = obj_val else {
                    return Err(JsError::type_error("Cannot set property of non-object"));
                };

                let key = self.get_member_key(&member.property)?;

                // For compound assignments, get current value (using getter if present)
                let final_value = if assign.operator != AssignmentOp::Assign {
                    let current = {
                        let prop_desc = obj.borrow().get_property_descriptor(&key);
                        match prop_desc {
                            Some((prop, _)) if prop.is_accessor() => {
                                if let Some(getter) = prop.getter() {
                                    let Guarded {
                                        value: getter_val,
                                        guard: _getter_guard,
                                    } = self.call_function(
                                        JsValue::Object(getter.clone()),
                                        obj_val.clone(),
                                        &[],
                                    )?;
                                    getter_val
                                } else {
                                    JsValue::Undefined
                                }
                            }
                            Some((prop, _)) => prop.value,
                            None => JsValue::Undefined,
                        }
                    };

                    // Apply compound operator
                    match assign.operator {
                        AssignmentOp::AddAssign => match (&current, &value) {
                            (JsValue::String(a), _) => {
                                JsValue::String(a.cheap_clone() + &value.to_js_string())
                            }
                            _ => JsValue::Number(current.to_number() + value.to_number()),
                        },
                        AssignmentOp::SubAssign => {
                            JsValue::Number(current.to_number() - value.to_number())
                        }
                        AssignmentOp::MulAssign => {
                            JsValue::Number(current.to_number() * value.to_number())
                        }
                        AssignmentOp::DivAssign => {
                            JsValue::Number(current.to_number() / value.to_number())
                        }
                        AssignmentOp::ModAssign => {
                            JsValue::Number(current.to_number() % value.to_number())
                        }
                        AssignmentOp::ExpAssign => {
                            JsValue::Number(current.to_number().powf(value.to_number()))
                        }
                        AssignmentOp::BitAndAssign => JsValue::Number(
                            (current.to_number() as i32 & value.to_number() as i32) as f64,
                        ),
                        AssignmentOp::BitOrAssign => JsValue::Number(
                            (current.to_number() as i32 | value.to_number() as i32) as f64,
                        ),
                        AssignmentOp::BitXorAssign => JsValue::Number(
                            (current.to_number() as i32 ^ value.to_number() as i32) as f64,
                        ),
                        AssignmentOp::LShiftAssign => {
                            let lhs = current.to_number() as i32;
                            let rhs = (value.to_number() as u32) & 0x1f;
                            JsValue::Number((lhs << rhs) as f64)
                        }
                        AssignmentOp::RShiftAssign => {
                            let lhs = current.to_number() as i32;
                            let rhs = (value.to_number() as u32) & 0x1f;
                            JsValue::Number((lhs >> rhs) as f64)
                        }
                        AssignmentOp::URShiftAssign => {
                            let lhs = (current.to_number() as i32) as u32;
                            let rhs = ((value.to_number() as i32) as u32) & 0x1f;
                            JsValue::Number((lhs >> rhs) as f64)
                        }
                        _ => value,
                    }
                } else {
                    value
                };

                // Check for setter before setting property
                let prop_desc = obj.borrow().get_property_descriptor(&key);
                if let Some((prop, _)) = prop_desc {
                    if prop.is_accessor() {
                        if let Some(setter) = prop.setter() {
                            // Call the setter with the value
                            self.call_function(
                                JsValue::Object(setter.clone()),
                                obj_val.clone(),
                                std::slice::from_ref(&final_value),
                            )?;
                        }
                        // If no setter, silently ignore in strict mode would throw, but we're lenient
                        return Ok(Guarded::unguarded(final_value));
                    }
                }

                // Handle __proto__ special property - set prototype instead of property
                if key.eq_str("__proto__") {
                    let new_proto = match &final_value {
                        JsValue::Object(proto_obj) => Some(proto_obj.clone()),
                        JsValue::Null => None,
                        _ => {
                            // Non-object, non-null values are ignored for __proto__ set
                            return Ok(Guarded::unguarded(final_value));
                        }
                    };
                    obj.borrow_mut().prototype = new_proto;
                    return Ok(Guarded::unguarded(final_value));
                }

                // Not an accessor - set property directly
                obj.borrow_mut().set_property(key, final_value.clone());
                // _rhs_guard dropped here, but ownership transferred to obj

                Ok(Guarded::unguarded(final_value))
            }
            AssignmentTarget::Pattern(pattern) => {
                // Destructuring assignment: [a, b] = [1, 2] or { x, y } = obj
                // Only simple assignment is supported (not compound like +=)
                if assign.operator != AssignmentOp::Assign {
                    return Err(JsError::syntax_error_simple(
                        "Destructuring assignment only supports = operator",
                    ));
                }
                self.assign_pattern(pattern, value.clone())?;
                Ok(Guarded::unguarded(value))
            }
        }
    }

    /// Assign values to an existing pattern (for destructuring assignment)
    /// Unlike bind_pattern, this sets existing variables rather than defining new ones
    fn assign_pattern(&mut self, pattern: &Pattern, value: JsValue) -> Result<(), JsError> {
        match pattern {
            Pattern::Identifier(id) => {
                self.env_set(&id.name, value)?;
                Ok(())
            }

            Pattern::Array(arr_pattern) => self.assign_array_pattern(arr_pattern, &value),

            Pattern::Object(obj_pattern) => {
                let obj = match &value {
                    JsValue::Object(o) => o.clone(),
                    _ => return Err(JsError::type_error("Cannot destructure non-object")),
                };

                for prop in &obj_pattern.properties {
                    match prop {
                        ObjectPatternProperty::KeyValue {
                            key,
                            value: pat,
                            shorthand,
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
                                .get_property(&PropertyKey::String(key_str.cheap_clone()))
                                .unwrap_or(JsValue::Undefined);

                            if *shorthand {
                                self.env_set(&key_str, prop_value)?;
                            } else {
                                self.assign_pattern(pat, prop_value)?;
                            }
                        }
                        ObjectPatternProperty::Rest(_) => {
                            return Err(JsError::internal_error(
                                "Object rest patterns not yet implemented",
                            ));
                        }
                    }
                }

                Ok(())
            }

            Pattern::Rest(rest) => self.assign_pattern(&rest.argument, value),

            Pattern::Assignment(assign_pat) => {
                let (val, _guard) = if matches!(value, JsValue::Undefined) {
                    let Guarded { value: v, guard: g } =
                        self.evaluate_expression(&assign_pat.right)?;
                    (v, g)
                } else {
                    (value, None)
                };
                self.assign_pattern(&assign_pat.left, val)
            }
        }
    }

    fn assign_array_pattern(
        &mut self,
        arr_pattern: &ArrayPattern,
        value: &JsValue,
    ) -> Result<(), JsError> {
        let items: Vec<JsValue> = match value {
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
            if let Some(pat) = elem {
                match pat {
                    Pattern::Rest(rest) => {
                        let remaining: Vec<JsValue> = items.get(i..).unwrap_or(&[]).to_vec();
                        let (rest_array, _guard) = self.create_array_with_guard(remaining);
                        self.assign_pattern(&rest.argument, JsValue::Object(rest_array))?;
                        break;
                    }
                    _ => {
                        let item = items.get(i).cloned().unwrap_or(JsValue::Undefined);
                        self.assign_pattern(pat, item)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Evaluate update expression (++i, i++, --i, i--)
    fn evaluate_update(&mut self, update: &UpdateExpression) -> Result<Guarded, JsError> {
        // Get the current value and update target
        match update.argument.as_ref() {
            Expression::Identifier(id) => {
                let current = self.env_get(&id.name)?;
                let num = current.to_number();
                let new_val = match update.operator {
                    UpdateOp::Increment => JsValue::Number(num + 1.0),
                    UpdateOp::Decrement => JsValue::Number(num - 1.0),
                };
                self.env_set(&id.name, new_val.clone())?;
                // Prefix returns new value, postfix returns old value
                if update.prefix {
                    Ok(Guarded::unguarded(new_val))
                } else {
                    Ok(Guarded::unguarded(JsValue::Number(num)))
                }
            }
            Expression::Member(member) => {
                let Guarded {
                    value: obj_val,
                    guard: _obj_guard,
                } = self.evaluate_expression(&member.object)?;
                let JsValue::Object(obj) = obj_val else {
                    return Err(JsError::type_error("Cannot update property of non-object"));
                };
                let key = self.get_member_key(&member.property)?;
                let current = obj
                    .borrow()
                    .get_property(&key)
                    .unwrap_or(JsValue::Undefined);
                let num = current.to_number();
                let new_val = match update.operator {
                    UpdateOp::Increment => JsValue::Number(num + 1.0),
                    UpdateOp::Decrement => JsValue::Number(num - 1.0),
                };
                obj.borrow_mut().set_property(key, new_val.clone());
                // Prefix returns new value, postfix returns old value
                if update.prefix {
                    Ok(Guarded::unguarded(new_val))
                } else {
                    Ok(Guarded::unguarded(JsValue::Number(num)))
                }
            }
            _ => Err(JsError::syntax_error_simple(
                "Invalid left-hand side in update expression",
            )),
        }
    }

    /// Evaluate sequence expression (a, b, c) - returns the last value
    fn evaluate_sequence(&mut self, seq: &SequenceExpression) -> Result<Guarded, JsError> {
        let mut result = Guarded::unguarded(JsValue::Undefined);
        for expr in &seq.expressions {
            result = self.evaluate_expression(expr)?;
        }
        Ok(result)
    }

    fn evaluate_call(&mut self, call: &CallExpression) -> Result<Guarded, JsError> {
        let (callee, this_value, obj_guard) = match &*call.callee {
            // super(args) - call parent constructor
            Expression::Super(_) => {
                let super_name = JsString::from("__super__");
                let super_constructor = self.env_get(&super_name)?;
                let this_name = JsString::from("this");
                let this_val = self.env_get(&this_name)?;
                (super_constructor, this_val, None)
            }
            // super.method() - call parent method
            Expression::Member(member) if matches!(&*member.object, Expression::Super(_)) => {
                let super_name = JsString::from("__super__");
                let super_constructor = self.env_get(&super_name)?;
                let this_name = JsString::from("this");
                let this_val = self.env_get(&this_name)?;

                // Get the method from super's prototype
                let key = self.get_member_key(&member.property)?;
                let func = if let JsValue::Object(super_obj) = &super_constructor {
                    let proto_key = self.key("prototype");
                    if let Some(JsValue::Object(proto)) =
                        super_obj.borrow().get_property(&proto_key)
                    {
                        proto.borrow().get_property(&key)
                    } else {
                        None
                    }
                } else {
                    None
                };

                match func {
                    Some(f) => (f, this_val, None),
                    None => return Err(JsError::type_error(format!("{} is not a function", key))),
                }
            }
            Expression::Member(member) => {
                let Guarded {
                    value: obj,
                    guard: obj_guard,
                } = self.evaluate_expression(&member.object)?;
                let key = self.get_member_key(&member.property)?;

                let func = match &obj {
                    JsValue::Object(o) => o.borrow().get_property(&key),
                    JsValue::Number(_) => self.number_prototype.borrow().get_property(&key),
                    JsValue::String(_) => {
                        // First check string-specific properties
                        if let PropertyKey::String(ref k) = key {
                            if k.as_str() == "length" {
                                None // length is not a function
                            } else {
                                self.string_prototype.borrow().get_property(&key)
                            }
                        } else {
                            None
                        }
                    }
                    JsValue::Symbol(_) => self.symbol_prototype.borrow().get_property(&key),
                    _ => None,
                };

                match func {
                    Some(f) => (f, obj, obj_guard),
                    None => return Err(JsError::type_error(format!("{} is not a function", key))),
                }
            }
            _ => {
                let Guarded {
                    value: callee,
                    guard,
                } = self.evaluate_expression(&call.callee)?;
                (callee, JsValue::Undefined, guard)
            }
        };

        // Keep the object guard alive during the call
        let _obj_guard = obj_guard;

        // Evaluate arguments, keeping guards alive until call completes
        let mut args = Vec::new();
        let mut _arg_guards = Vec::new();
        for arg in &call.arguments {
            match arg {
                crate::ast::Argument::Expression(expr) => {
                    let Guarded { value, guard } = self.evaluate_expression(expr)?;
                    args.push(value);
                    if let Some(g) = guard {
                        _arg_guards.push(g);
                    }
                }
                crate::ast::Argument::Spread(spread) => {
                    let Guarded { value, guard } = self.evaluate_expression(&spread.argument)?;
                    if let Some(g) = guard {
                        _arg_guards.push(g);
                    }
                    // Spread the array elements into arguments
                    if let JsValue::Object(obj) = value {
                        let obj_ref = obj.borrow();
                        if let ExoticObject::Array { length } = &obj_ref.exotic {
                            for i in 0..*length {
                                let elem = obj_ref
                                    .get_property(&PropertyKey::Index(i))
                                    .unwrap_or(JsValue::Undefined);
                                args.push(elem);
                            }
                        }
                    }
                }
            }
        }

        // Call function - propagate guard from result
        self.call_function(callee, this_value, &args)
    }

    pub fn call_function(
        &mut self,
        callee: JsValue,
        this_value: JsValue,
        args: &[JsValue],
    ) -> Result<Guarded, JsError> {
        let JsValue::Object(func_obj) = callee else {
            return Err(JsError::type_error("Not a function"));
        };

        let func = {
            let obj_ref = func_obj.borrow();
            match &obj_ref.exotic {
                ExoticObject::Function(f) => f.clone(),
                _ => return Err(JsError::type_error("Not a function")),
            }
        };

        match func {
            JsFunction::Interpreted(interp) => {
                // Create new environment
                let func_env =
                    create_environment_with_guard(&self.root_guard, Some(interp.closure));

                // Bind `this` in the function environment
                {
                    let this_name = JsString::from("this");
                    if let Some(data) = func_env.borrow_mut().as_environment_mut() {
                        data.bindings.insert(
                            this_name,
                            Binding {
                                value: this_value.clone(),
                                mutable: false,
                                initialized: true,
                            },
                        );
                    }
                }

                // Bind `__super__` if this is a class constructor with inheritance
                {
                    let super_key = self.key("__super__");
                    if let Some(super_val) = func_obj.borrow().get_property(&super_key) {
                        let super_name = JsString::from("__super__");
                        if let Some(data) = func_env.borrow_mut().as_environment_mut() {
                            data.bindings.insert(
                                super_name,
                                Binding {
                                    value: super_val,
                                    mutable: false,
                                    initialized: true,
                                },
                            );
                        }
                    }
                }

                // Execute function body - set up environment first so bind_pattern works
                let saved_env = self.env.clone();
                self.env = func_env;

                // Create and bind the `arguments` object (array-like)
                {
                    let (args_array, _guard) = self.create_array_with_guard(args.to_vec());
                    let args_name = JsString::from("arguments");
                    self.env_define(args_name, JsValue::Object(args_array), false);
                }

                // Bind parameters using bind_pattern for full destructuring support
                for (i, param) in interp.params.iter().enumerate() {
                    match &param.pattern {
                        Pattern::Rest(rest) => {
                            // Collect remaining arguments into an array
                            let rest_args: Vec<JsValue> =
                                args.get(i..).unwrap_or_default().to_vec();
                            let (rest_array, _guard) = self.create_array_with_guard(rest_args);

                            // Bind the rest pattern (usually an identifier)
                            self.bind_pattern(&rest.argument, JsValue::Object(rest_array), true)?;
                            break; // Rest param must be last
                        }
                        _ => {
                            // Use bind_pattern for all other patterns (Identifier, Object, Array, Assignment)
                            let arg_val = args.get(i).cloned().unwrap_or(JsValue::Undefined);
                            self.bind_pattern(&param.pattern, arg_val, true)?;
                        }
                    }
                }

                // Hoist var declarations before executing body
                if let FunctionBody::Block(block) = &*interp.body {
                    self.hoist_var_declarations(&block.body);
                }

                let body_result: Result<(JsValue, Option<Guard<JsObject>>), JsError> =
                    match &*interp.body {
                        FunctionBody::Block(block) => match self.execute_block(block) {
                            Ok(Completion::Return(v)) => Ok((v, None)),
                            Ok(_) => Ok((JsValue::Undefined, None)),
                            Err(e) => Err(e),
                        },
                        FunctionBody::Expression(expr) => match self.evaluate_expression(expr) {
                            Ok(Guarded { value, guard }) => Ok((value, guard)),
                            Err(e) => Err(e),
                        },
                    };

                // ALWAYS restore environment, even on error
                self.env = saved_env;

                // Now propagate the result or error
                let (result, result_guard) = body_result?;

                // Propagate guard from expression body arrow functions
                Ok(Guarded {
                    value: result,
                    guard: result_guard,
                })
            }

            JsFunction::Native(native) => {
                // Call native function - propagate the Guarded to preserve guard
                (native.func)(self, this_value, args)
            }

            JsFunction::Bound(bound) => {
                // Call bound function: use bound this and prepend bound args
                let mut full_args = bound.bound_args.clone();
                full_args.extend_from_slice(args);
                self.call_function(
                    JsValue::Object(bound.target),
                    bound.this_arg.clone(),
                    &full_args,
                )
            }

            JsFunction::PromiseResolve(_) | JsFunction::PromiseReject(_) => Err(
                JsError::internal_error("Promise functions not yet implemented"),
            ),
        }
    }

    fn evaluate_member(&mut self, member: &MemberExpression) -> Result<Guarded, JsError> {
        let Guarded {
            value: obj,
            guard: obj_guard,
        } = self.evaluate_expression(&member.object)?;
        let key = self.get_member_key(&member.property)?;

        // Get the value from the member access
        // Returns (value, optional_extra_guard) - extra guard for values from getter calls
        let (value, extra_guard) = match &obj {
            JsValue::Object(o) => {
                // Handle __proto__ special property
                if key.eq_str("__proto__") {
                    let proto = o.borrow().prototype.clone();
                    match proto {
                        Some(p) => (JsValue::Object(p), None),
                        None => (JsValue::Null, None),
                    }
                } else {
                    // Get the property descriptor to check for getters
                    let prop_desc = o.borrow().get_property_descriptor(&key);
                    match prop_desc {
                        Some((prop, _)) if prop.is_accessor() => {
                            // Property has a getter - invoke it
                            if let Some(getter) = prop.getter() {
                                let Guarded {
                                    value: getter_val,
                                    guard: getter_guard,
                                } =
                                    self.call_function(JsValue::Object(getter.clone()), obj.clone(), &[])?;
                                (getter_val, getter_guard)
                            } else {
                                (JsValue::Undefined, None)
                            }
                        }
                        Some((prop, _)) => (prop.value, None),
                        None => (JsValue::Undefined, None),
                    }
                }
            }
            JsValue::String(s) => {
                if let PropertyKey::Index(i) = key {
                    let chars: Vec<char> = s.as_str().chars().collect();
                    if let Some(c) = chars.get(i as usize) {
                        (JsValue::String(JsString::from(c.to_string())), None)
                    } else {
                        (JsValue::Undefined, None)
                    }
                } else if let PropertyKey::String(ref k) = key {
                    if k.as_str() == "length" {
                        (JsValue::Number(s.as_str().chars().count() as f64), None)
                    } else if let Some(method) = self.string_prototype.borrow().get_property(&key) {
                        // Look up methods on String.prototype
                        (method, None)
                    } else {
                        (JsValue::Undefined, None)
                    }
                } else {
                    (JsValue::Undefined, None)
                }
            }
            JsValue::Number(_) => {
                // Look up methods on Number.prototype
                if let Some(method) = self.number_prototype.borrow().get_property(&key) {
                    (method, None)
                } else {
                    (JsValue::Undefined, None)
                }
            }
            JsValue::Symbol(ref sym) => {
                // Handle .description property for symbols
                if let PropertyKey::String(ref k) = key {
                    if k.as_str() == "description" {
                        match &sym.description {
                            Some(desc) => (JsValue::String(JsString::from(desc.as_str())), None),
                            None => (JsValue::Undefined, None),
                        }
                    } else if let Some(method) = self.symbol_prototype.borrow().get_property(&key) {
                        // Look up methods on Symbol.prototype
                        (method, None)
                    } else {
                        (JsValue::Undefined, None)
                    }
                } else {
                    (JsValue::Undefined, None)
                }
            }
            _ => (JsValue::Undefined, None),
        };

        // Use getter's guard if present, otherwise the object's guard
        let final_guard = extra_guard.or(obj_guard);
        Ok(Guarded {
            value,
            guard: final_guard,
        })
    }

    fn get_member_key(&mut self, property: &MemberProperty) -> Result<PropertyKey, JsError> {
        match property {
            MemberProperty::Identifier(id) => Ok(PropertyKey::String(id.name.cheap_clone())),
            MemberProperty::Expression(expr) => {
                let Guarded {
                    value: val,
                    guard: _val_guard,
                } = self.evaluate_expression(expr)?;
                Ok(PropertyKey::from_value(&val))
            }
            MemberProperty::PrivateIdentifier(id) => {
                Ok(PropertyKey::String(JsString::from(format!("#{}", id.name))))
            }
        }
    }

    fn evaluate_array(&mut self, arr: &crate::ast::ArrayExpression) -> Result<Guarded, JsError> {
        // Collect elements, keeping guards alive until array is created
        let mut elements = Vec::new();
        let mut _elem_guards = Vec::new();

        for elem in &arr.elements {
            match elem {
                Some(ArrayElement::Expression(expr)) => {
                    let Guarded { value, guard } = self.evaluate_expression(expr)?;
                    elements.push(value);
                    if let Some(g) = guard {
                        _elem_guards.push(g);
                    }
                }
                Some(ArrayElement::Spread(spread)) => {
                    let Guarded { value, guard } = self.evaluate_expression(&spread.argument)?;
                    if let Some(g) = guard {
                        _elem_guards.push(g);
                    }
                    // Spread the array elements into the new array
                    if let JsValue::Object(obj) = value {
                        let obj_ref = obj.borrow();
                        if let ExoticObject::Array { length } = &obj_ref.exotic {
                            for i in 0..*length {
                                let elem = obj_ref
                                    .get_property(&PropertyKey::Index(i))
                                    .unwrap_or(JsValue::Undefined);
                                elements.push(elem);
                            }
                        }
                    }
                }
                None => elements.push(JsValue::Undefined),
            }
        }

        // Create array with guard - elements ownership transferred to array
        let (arr_obj, guard) = self.create_array_with_guard(elements);
        Ok(Guarded::with_guard(JsValue::Object(arr_obj), guard))
    }

    fn evaluate_object(&mut self, obj_expr: &ObjectExpression) -> Result<Guarded, JsError> {
        let (obj, obj_guard) = self.create_object_with_guard();

        // Keep property value guards alive until ownership is transferred to obj
        let mut _prop_guards = Vec::new();

        for prop in &obj_expr.properties {
            match prop {
                ObjectProperty::Property(p) => {
                    let prop_key = match &p.key {
                        ObjectPropertyKey::Identifier(id) => {
                            PropertyKey::String(id.name.cheap_clone())
                        }
                        ObjectPropertyKey::String(s) => PropertyKey::String(s.value.cheap_clone()),
                        ObjectPropertyKey::Number(lit) => {
                            if let LiteralValue::Number(n) = &lit.value {
                                PropertyKey::from_value(&JsValue::Number(*n))
                            } else {
                                continue;
                            }
                        }
                        ObjectPropertyKey::Computed(expr) => {
                            let Guarded {
                                value: k,
                                guard: _k_guard,
                            } = self.evaluate_expression(expr)?;
                            PropertyKey::from_value(&k)
                        }
                        ObjectPropertyKey::PrivateIdentifier(id) => {
                            PropertyKey::String(JsString::from(format!("#{}", id.name)))
                        }
                    };

                    let Guarded {
                        value: prop_val,
                        guard: prop_guard,
                    } = if p.shorthand {
                        // Shorthand: { x } means { x: x }
                        if let ObjectPropertyKey::Identifier(id) = &p.key {
                            Guarded::unguarded(self.env_get(&id.name)?)
                        } else {
                            self.evaluate_expression(&p.value)?
                        }
                    } else {
                        self.evaluate_expression(&p.value)?
                    };

                    // Handle __proto__ special property in object literals
                    if prop_key.eq_str("__proto__") {
                        let new_proto = match &prop_val {
                            JsValue::Object(proto_obj) => Some(proto_obj.clone()),
                            JsValue::Null => None,
                            _ => {
                                // Non-object, non-null values are ignored for __proto__
                                continue;
                            }
                        };
                        obj.borrow_mut().prototype = new_proto;
                        // Keep prop_guard alive
                        if let Some(g) = prop_guard {
                            _prop_guards.push(g);
                        }
                        continue;
                    }

                    // Keep prop_guard alive
                    if let Some(g) = prop_guard {
                        _prop_guards.push(g);
                    }

                    obj.borrow_mut().set_property(prop_key, prop_val);
                }
                ObjectProperty::Spread(spread) => {
                    // Evaluate the spread argument
                    let Guarded {
                        value: spread_val,
                        guard: spread_guard,
                    } = self.evaluate_expression(&spread.argument)?;

                    // Keep guard alive
                    if let Some(g) = spread_guard {
                        _prop_guards.push(g);
                    }

                    // If it's an object, copy all its enumerable own properties
                    if let JsValue::Object(spread_obj) = spread_val {
                        let spread_ref = spread_obj.borrow();
                        // Copy all string properties (not symbol keys for now)
                        for (key, prop) in spread_ref.properties.iter() {
                            // Skip non-enumerable properties
                            if !prop.enumerable() {
                                continue;
                            }
                            obj.borrow_mut()
                                .set_property(key.clone(), prop.value.clone());
                        }
                    }
                    // If it's null or undefined, just skip (like JS does)
                    // Other primitives are also skipped
                }
            }
        }

        Ok(Guarded::with_guard(JsValue::Object(obj), obj_guard))
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Var Hoisting
    // ═══════════════════════════════════════════════════════════════════════════

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
                    self.env_define(id.name.cheap_clone(), JsValue::Undefined, true);
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

    /// Check if a binding exists in the current scope (not parent scopes)
    fn env_has_own_binding(&self, name: &JsString) -> bool {
        let env_ref = self.env.borrow();
        if let Some(data) = env_ref.as_environment() {
            data.bindings.contains_key(name)
        } else {
            false
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unsigned_right_shift_basic() {
        let mut interp = Interpreter::new();
        let result = interp.eval_simple("32 >>> 2").unwrap();
        assert_eq!(result, JsValue::Number(8.0));
    }

    #[test]
    fn test_unsigned_right_shift_negative() {
        let mut interp = Interpreter::new();
        assert_eq!(
            interp.eval_simple("-1 >>> 0").unwrap(),
            JsValue::Number(4294967295.0)
        );
    }

    #[test]
    fn test_basic_arithmetic() {
        let mut interp = Interpreter::new();
        assert_eq!(interp.eval_simple("1 + 2").unwrap(), JsValue::Number(3.0));
        assert_eq!(interp.eval_simple("3 * 4").unwrap(), JsValue::Number(12.0));
        assert_eq!(interp.eval_simple("10 / 2").unwrap(), JsValue::Number(5.0));
    }

    #[test]
    fn test_continue_outside_loop_error() {
        let mut interp = Interpreter::new();
        let result = interp.eval_simple("continue;");
        assert!(result.is_err(), "continue outside loop should error");
        let err = result.unwrap_err();
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("Illegal"),
            "Error should mention 'Illegal': {}",
            err_str
        );
    }

    #[test]
    fn test_break_outside_loop_error() {
        let mut interp = Interpreter::new();
        let result = interp.eval_simple("break;");
        assert!(result.is_err(), "break outside loop should error");
        let err = result.unwrap_err();
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("Illegal"),
            "Error should mention 'Illegal': {}",
            err_str
        );
    }
}
