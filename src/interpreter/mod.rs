//! Interpreter for executing TypeScript AST
//!
//! This module implements a minimal interpreter using the new guard-based GC.

// Builtin function implementations
pub mod builtins;

// Stack-based evaluation for suspendable execution
pub mod stack;

use crate::ast::{
    Argument, ArrayElement, ArrayPattern, AssignmentExpression, AssignmentOp, AssignmentTarget,
    BinaryExpression, BinaryOp, BlockStatement, CallExpression, ClassConstructor, ClassDeclaration,
    ClassExpression, ClassMember, ClassMethod, ClassProperty, ConditionalExpression, Decorator,
    Expression, ForInOfLeft, ForInit, FunctionParam, LiteralValue, LogicalExpression, LogicalOp,
    MemberExpression, MemberProperty, MethodKind, NewExpression, ObjectExpression,
    ObjectPatternProperty, ObjectProperty, ObjectPropertyKey, Pattern, Program, PropertyKind,
    SequenceExpression, Statement, TaggedTemplateExpression, TemplateLiteral, UnaryExpression,
    UnaryOp, UpdateExpression, UpdateOp, VariableDeclaration, VariableKind,
};
use crate::error::JsError;
use crate::gc::{Gc, Guard, Heap};
use crate::lexer::Span;
use crate::parser::Parser;
use crate::string_dict::StringDict;
use crate::value::{
    create_environment_unrooted, number_to_string, Binding, CheapClone, EnumData, EnvRef,
    EnvironmentData, ExoticObject, FunctionBody, GeneratorState, GeneratorStatus, Guarded,
    ImportBinding, InterpretedFunction, JsFunction, JsObject, JsString, JsValue, ModuleExport,
    NativeFn, NativeFunction, PromiseStatus, Property, PropertyKey, VarKey,
};
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;

/// Type alias for accessor map: property key -> (getter, setter)
type AccessorMap = FxHashMap<PropertyKey, (Option<Gc<JsObject>>, Option<Gc<JsObject>>)>;

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

/// Saved execution state for a suspended generator.
/// This captures everything needed to resume execution at a yield point.
pub struct SavedGeneratorExecution {
    /// The saved environment (function scope)
    pub env: Gc<JsObject>,
    /// Guard to keep the environment and all frame references alive during suspension
    pub guard: Guard<JsObject>,
    /// Saved frame stack
    pub frames: Vec<stack::Frame>,
    /// Saved value stack (as JsValues - guards are recreated on resume)
    pub values: Vec<JsValue>,
    /// Saved completion type
    pub completion: stack::StackCompletion,
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

    /// Stack of guards for environments (keeps them alive during execution).
    /// Environments are pushed when entering scopes and popped when leaving.
    /// This ensures that environments on the execution stack remain alive.
    env_guards: Vec<Guard<JsObject>>,

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

    /// Boolean.prototype (for boolean primitive methods)
    pub boolean_prototype: Gc<JsObject>,

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

    /// Promise.prototype (for Promise methods)
    pub promise_prototype: Gc<JsObject>,

    /// Generator.prototype (for generator methods)
    pub generator_prototype: Gc<JsObject>,

    // ═══════════════════════════════════════════════════════════════════════════
    // Error Prototypes (for creating proper error objects from JsError)
    // ═══════════════════════════════════════════════════════════════════════════
    /// Error.prototype
    pub error_prototype: Gc<JsObject>,

    /// TypeError.prototype
    pub type_error_prototype: Gc<JsObject>,

    /// ReferenceError.prototype
    pub reference_error_prototype: Gc<JsObject>,

    /// RangeError.prototype
    pub range_error_prototype: Gc<JsObject>,

    /// SyntaxError.prototype
    pub syntax_error_prototype: Gc<JsObject>,

    // ═══════════════════════════════════════════════════════════════════════════
    // Execution State
    // ═══════════════════════════════════════════════════════════════════════════
    /// Stores thrown value during exception propagation
    thrown_value: Option<JsValue>,

    /// Guard for the thrown value (keeps it alive during exception handling)
    thrown_guard: Option<Guard<JsObject>>,

    /// Exported values from the module
    /// Uses ModuleExport to distinguish direct exports (with live bindings) from re-exports
    pub exports: FxHashMap<JsString, ModuleExport>,

    /// Call stack for stack traces
    pub call_stack: Vec<StackFrame>,

    /// Counter for generating unique generator IDs
    next_generator_id: u64,

    /// Saved execution states for suspended generators (generator_id -> saved state)
    /// The execution state is moved here when a generator yields, and moved out when resumed.
    pub saved_generator_states: FxHashMap<u64, SavedGeneratorExecution>,

    // ═══════════════════════════════════════════════════════════════════════════
    // Timeout and Limits
    // ═══════════════════════════════════════════════════════════════════════════
    /// Execution timeout in milliseconds (0 = no timeout)
    timeout_ms: u64,

    /// When execution started (for timeout checking)
    execution_start: Option<std::time::Instant>,

    /// Step counter for batched timeout checking (only check every N steps)
    step_counter: u32,

    /// Maximum call stack depth (0 = no limit)
    /// Default is 256, but tests use a lower value (e.g., 50) to catch infinite recursion early
    max_call_depth: usize,

    // ═══════════════════════════════════════════════════════════════════════════
    // Module System
    // ═══════════════════════════════════════════════════════════════════════════
    /// Registered internal modules (specifier -> module definition)
    internal_modules: FxHashMap<String, crate::InternalModule>,

    /// Instantiated internal module objects (cached after first import)
    internal_module_cache: FxHashMap<String, Gc<JsObject>>,

    /// Loaded external modules (normalized path -> module namespace)
    loaded_modules: FxHashMap<crate::ModulePath, Gc<JsObject>>,

    /// The path of the main module (set by eval_with_path)
    main_module_path: Option<crate::ModulePath>,

    /// The path of the currently executing module (for resolving relative imports)
    current_module_path: Option<crate::ModulePath>,

    // ═══════════════════════════════════════════════════════════════════════════
    // Order System
    // ═══════════════════════════════════════════════════════════════════════════
    /// Counter for generating unique order IDs
    pub(crate) next_order_id: u64,

    /// Pending orders waiting for host fulfillment.
    /// Each Order contains a RuntimeValue that keeps the payload alive.
    pub(crate) pending_orders: Vec<crate::Order>,

    /// Map from OrderId -> (resolve_fn, reject_fn) for pending promises
    pub(crate) order_callbacks: FxHashMap<crate::OrderId, (Gc<JsObject>, Gc<JsObject>)>,

    /// Cancelled order IDs (from Promise.race losing, etc.)
    pub(crate) cancelled_orders: Vec<crate::OrderId>,

    /// Suspended execution state (if any)
    pub(crate) suspended_state: Option<stack::ExecutionState>,

    /// Pending program waiting for imports to be provided
    pub(crate) pending_program: Option<crate::ast::Program>,

    /// Pending module sources waiting for their imports to be satisfied
    /// Maps normalized path -> parsed program
    pub(crate) pending_module_sources: FxHashMap<crate::ModulePath, crate::ast::Program>,

    /// Pool of reusable ExecutionState objects to avoid repeated allocations.
    /// When an execution completes, its state is reset and returned to the pool.
    execution_state_pool: Vec<stack::ExecutionState>,
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
        let boolean_prototype = root_guard.alloc();
        let regexp_prototype = root_guard.alloc();
        let map_prototype = root_guard.alloc();
        let set_prototype = root_guard.alloc();
        let date_prototype = root_guard.alloc();
        let symbol_prototype = root_guard.alloc();
        let promise_prototype = root_guard.alloc();
        let generator_prototype = root_guard.alloc();

        // Create error prototypes (all rooted)
        let error_prototype = root_guard.alloc();
        let type_error_prototype = root_guard.alloc();
        let reference_error_prototype = root_guard.alloc();
        let range_error_prototype = root_guard.alloc();
        let syntax_error_prototype = root_guard.alloc();

        // Set up prototype chain - all prototypes inherit from object_prototype
        array_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        function_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        string_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        number_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        boolean_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        regexp_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        map_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        set_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        date_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        symbol_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        promise_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        generator_prototype.borrow_mut().prototype = Some(object_prototype.clone());

        // Set up error prototype chain
        // Error.prototype inherits from Object.prototype
        error_prototype.borrow_mut().prototype = Some(object_prototype.clone());
        // All specific error prototypes inherit from Error.prototype
        type_error_prototype.borrow_mut().prototype = Some(error_prototype.clone());
        reference_error_prototype.borrow_mut().prototype = Some(error_prototype.clone());
        range_error_prototype.borrow_mut().prototype = Some(error_prototype.clone());
        syntax_error_prototype.borrow_mut().prototype = Some(error_prototype.clone());

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
            env_guards: Vec::new(), // global_env is rooted via root_guard
            string_dict,
            object_prototype,
            array_prototype,
            function_prototype,
            string_prototype,
            number_prototype,
            boolean_prototype,
            regexp_prototype,
            map_prototype,
            set_prototype,
            date_prototype,
            symbol_prototype,
            promise_prototype,
            generator_prototype,
            error_prototype,
            type_error_prototype,
            reference_error_prototype,
            range_error_prototype,
            syntax_error_prototype,
            thrown_value: None,
            thrown_guard: None,
            exports: FxHashMap::default(),
            call_stack: Vec::new(),
            next_generator_id: 1,
            saved_generator_states: FxHashMap::default(),
            timeout_ms: 3000, // Default 3 second timeout
            execution_start: None,
            step_counter: 0,
            max_call_depth: 256, // Default limit to prevent Rust stack overflow
            // Module system
            internal_modules: FxHashMap::default(),
            internal_module_cache: FxHashMap::default(),
            loaded_modules: FxHashMap::default(),
            main_module_path: None,
            current_module_path: None,
            // Order system
            next_order_id: 1,
            pending_orders: Vec::new(),
            order_callbacks: FxHashMap::default(),
            cancelled_orders: Vec::new(),
            suspended_state: None,
            pending_program: None,
            pending_module_sources: FxHashMap::default(),
            execution_state_pool: Vec::new(),
        };

        // Initialize built-in globals
        interp.init_globals();

        // Register built-in internal modules
        interp.register_internal_module(builtins::create_eval_internal_module());

        interp
    }

    /// Initialize built-in global values
    fn init_globals(&mut self) {
        // For now, minimal globals - just define undefined and NaN
        let undefined_name = self.intern("undefined");
        self.env_define(undefined_name, JsValue::Undefined, false);

        let nan_name = self.intern("NaN");
        self.env_define(nan_name, JsValue::Number(f64::NAN), false);

        let infinity_name = self.intern("Infinity");
        self.env_define(infinity_name, JsValue::Number(f64::INFINITY), false);

        // Initialize Array builtin methods
        builtins::init_array_prototype(self);

        // Initialize String prototype methods
        builtins::init_string_prototype(self);

        // Initialize Function.prototype (call, apply, bind)
        builtins::init_function_prototype(self);

        // Initialize Function constructor (global Function function)
        let function_constructor = builtins::create_function_constructor(self);
        let function_name = self.intern("Function");
        self.env_define(function_name, JsValue::Object(function_constructor), false);

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
        let string_name = self.intern("String");
        self.env_define(string_name, JsValue::Object(string_constructor), false);

        // Initialize Array constructor (global Array function)
        let array_constructor = builtins::create_array_constructor(self);
        let array_name = self.intern("Array");
        self.env_define(array_name, JsValue::Object(array_constructor), false);

        // Initialize Object prototype and constructor
        builtins::init_object_prototype(self);
        let object_constructor = builtins::create_object_constructor(self);
        let object_name = self.intern("Object");
        self.env_define(object_name, JsValue::Object(object_constructor), false);

        // Initialize RegExp prototype and constructor
        builtins::init_regexp_prototype(self);
        let regexp_constructor = builtins::create_regexp_constructor(self);
        let regexp_name = self.intern("RegExp");
        self.env_define(regexp_name, JsValue::Object(regexp_constructor), false);

        // Initialize Number constructor (global Number function)
        let number_constructor = builtins::create_number_constructor(self);
        let number_name = self.intern("Number");
        self.env_define(number_name, JsValue::Object(number_constructor), false);

        // Initialize Boolean prototype and constructor (global Boolean function)
        builtins::init_boolean_prototype(self);
        let boolean_constructor = builtins::create_boolean_constructor(self);
        let boolean_name = self.intern("Boolean");
        self.env_define(boolean_name, JsValue::Object(boolean_constructor), false);

        // Initialize Promise prototype and constructor
        builtins::promise::init_promise_prototype(self);
        let promise_constructor = builtins::promise::create_promise_constructor(self);
        let promise_name = self.intern("Promise");
        self.env_define(promise_name, JsValue::Object(promise_constructor), false);

        // Initialize Generator prototype
        builtins::init_generator_prototype(self);

        // Initialize Proxy constructor and Reflect object
        builtins::proxy::init_proxy(self);
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

    /// Set the maximum call stack depth
    ///
    /// Default is 256. Set to 0 to disable limit (not recommended).
    /// Tests should use a lower value (e.g., 50) to catch infinite recursion early.
    pub fn set_max_call_depth(&mut self, depth: usize) {
        self.max_call_depth = depth;
    }

    /// Get the current maximum call stack depth
    pub fn max_call_depth(&self) -> usize {
        self.max_call_depth
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
    /// Only performs the actual time check every 1000 steps for performance.
    fn check_timeout(&mut self) -> Result<(), JsError> {
        // Skip check if timeout is disabled
        if self.timeout_ms == 0 {
            return Ok(());
        }

        // Only check every 1000 steps
        self.step_counter += 1;
        if self.step_counter < 1000 {
            return Ok(());
        }
        self.step_counter = 0;

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

    /// Evaluate TypeScript/JavaScript code with full runtime support.
    ///
    /// This is equivalent to `eval_with_path(source, None)` - relative imports
    /// will be resolved without a base path (treated as bare specifiers).
    ///
    /// Returns RuntimeResult which may indicate:
    /// - Complete: execution finished with a value
    /// - NeedImports: modules need to be provided before continuing
    /// - Suspended: waiting for orders to be fulfilled
    pub fn eval(&mut self, source: &str) -> Result<crate::RuntimeResult, JsError> {
        self.eval_with_path(source, None)
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
        module_path: Option<crate::ModulePath>,
    ) -> Result<crate::RuntimeResult, JsError> {
        // Set main module path if this is the entry point
        if self.main_module_path.is_none() {
            self.main_module_path = module_path.clone();
        }
        self.current_module_path = module_path.clone();

        // Parse the source
        let mut parser = Parser::new(source, &mut self.string_dict);
        let program = parser.parse_program()?;

        // Collect all import requests with resolved paths
        // For main module, importer is None (we pass module_path for resolution but not as importer)
        let imports = self.collect_import_requests_internal(&program, module_path.as_ref(), None);

        // Filter to only missing imports and deduplicate
        let missing = self.filter_missing_imports(imports);
        let missing = Self::dedupe_import_requests(missing);

        if !missing.is_empty() {
            // Save the program for later execution when imports are provided
            self.pending_program = Some(program);
            return Ok(crate::RuntimeResult::NeedImports(missing));
        }

        // All imports satisfied - execute using stack-based evaluation
        self.start_execution();

        // Hoist var declarations at program/global scope
        self.hoist_var_declarations(&program.body);

        let mut state = self.get_execution_state();
        state.init_for_program(&program);
        let result = self.run_to_completion_or_suspend(&mut state);

        // Return state to pool if execution completed (not suspended)
        if self.suspended_state.is_none() {
            self.return_execution_state(state);
        }

        // Convert ThrownValue errors to RuntimeError before returning
        // This is important because ThrownValue contains a Gc pointer that
        // will become invalid once the interpreter/heap is dropped
        result.map_err(|e| self.materialize_thrown_error(e))
    }

    /// Convert ThrownValue errors to RuntimeError with string data
    ///
    /// JsError::ThrownValue contains a JsValue that may have Gc pointers.
    /// These pointers become invalid when the interpreter/heap is dropped.
    /// This function extracts the error information while it's still valid.
    fn materialize_thrown_error(&self, error: JsError) -> JsError {
        match error {
            JsError::ThrownValue { value } => {
                // Extract error name and message from the thrown value
                if let JsValue::Object(obj) = &value {
                    let obj_ref = obj.borrow();
                    let name = obj_ref
                        .get_property(&PropertyKey::from("name"))
                        .map(|v| v.to_js_string().to_string())
                        .unwrap_or_else(|| "Error".to_string());
                    let message = obj_ref
                        .get_property(&PropertyKey::from("message"))
                        .map(|v| v.to_js_string().to_string())
                        .unwrap_or_default();
                    JsError::RuntimeError {
                        kind: name,
                        message,
                        stack: Vec::new(),
                    }
                } else {
                    // Non-object thrown value - convert to string
                    JsError::RuntimeError {
                        kind: "Error".to_string(),
                        message: value.to_js_string().to_string(),
                        stack: Vec::new(),
                    }
                }
            }
            // Pass through other error types unchanged
            other => other,
        }
    }

    /// Run execution until completion or suspension
    fn run_to_completion_or_suspend(
        &mut self,
        state: &mut stack::ExecutionState,
    ) -> Result<crate::RuntimeResult, JsError> {
        use stack::StepResult;

        // If we're resuming from suspension, check if the promise was resolved
        if let Some(promise) = state.waiting_on.take() {
            let obj_ref = promise.borrow();
            if let ExoticObject::Promise(promise_state) = &obj_ref.exotic {
                let status = promise_state.borrow().status.clone();
                let result = promise_state.borrow().result.clone();
                drop(obj_ref);

                match status {
                    PromiseStatus::Fulfilled => {
                        let value = result.unwrap_or(JsValue::Undefined);
                        // Guard the value to prevent GC during execution
                        let guard = self.guard_value(&value);
                        state.push_value(Guarded { value, guard });
                    }
                    PromiseStatus::Rejected => {
                        let reason = result.unwrap_or(JsValue::Undefined);
                        // Guard the reason to prevent GC during error propagation
                        self.thrown_guard = self.guard_value(&reason);
                        return Err(JsError::thrown(reason));
                    }
                    PromiseStatus::Pending => {
                        // Still pending - should not happen after fulfill_orders
                        // Re-suspend
                        state.waiting_on = Some(promise);
                        self.suspended_state = Some(std::mem::take(state));
                        let pending = std::mem::take(&mut self.pending_orders);
                        let cancelled = std::mem::take(&mut self.cancelled_orders);
                        return Ok(crate::RuntimeResult::Suspended { pending, cancelled });
                    }
                }
            } else {
                drop(obj_ref);
                // Not a promise anymore? Push undefined
                state.push_value(Guarded::unguarded(JsValue::Undefined));
            }
        }

        loop {
            match self.step(state) {
                StepResult::Continue => continue,
                StepResult::Done(guarded) => {
                    // Execution complete - check if there are pending orders to return
                    if !self.pending_orders.is_empty() {
                        let pending = std::mem::take(&mut self.pending_orders);
                        let cancelled = std::mem::take(&mut self.cancelled_orders);
                        return Ok(crate::RuntimeResult::Suspended { pending, cancelled });
                    }
                    // Also check for unfulfilled orders from previous suspension
                    // (e.g., Promise.all where only some orders were fulfilled)
                    if !self.order_callbacks.is_empty() {
                        // Return suspended with no new orders - the host already has them
                        let cancelled = std::mem::take(&mut self.cancelled_orders);
                        return Ok(crate::RuntimeResult::Suspended {
                            pending: Vec::new(),
                            cancelled,
                        });
                    }
                    return Ok(crate::RuntimeResult::Complete(
                        crate::RuntimeValue::from_guarded(guarded),
                    ));
                }
                StepResult::Error(error) => {
                    // Try to find a TryBlock frame to catch the error
                    if let Some(result) = self.handle_error(state, error) {
                        return match result {
                            StepResult::Error(e) => Err(e),
                            StepResult::Done(g) => Ok(crate::RuntimeResult::Complete(
                                crate::RuntimeValue::from_guarded(g),
                            )),
                            _ => continue,
                        };
                    }
                    // Error was handled, continue execution
                    continue;
                }
                StepResult::Suspend(_promise) => {
                    // Await on pending promise - save state and return Suspended
                    self.suspended_state = Some(std::mem::take(state));
                    let pending = std::mem::take(&mut self.pending_orders);
                    let cancelled = std::mem::take(&mut self.cancelled_orders);
                    return Ok(crate::RuntimeResult::Suspended { pending, cancelled });
                }
            }
        }
    }

    /// Provide a module source for a pending import.
    ///
    /// The `resolved_path` should be the normalized path from `ImportRequest.resolved_path`.
    /// The module is parsed and stored, but not executed until `continue_eval` is called.
    /// This allows collecting all needed imports before execution.
    pub fn provide_module(
        &mut self,
        resolved_path: crate::ModulePath,
        source: &str,
    ) -> Result<(), JsError> {
        // Parse the module
        let mut parser = Parser::new(source, &mut self.string_dict);
        let program = parser.parse_program()?;

        // Store the parsed program for later execution
        self.pending_module_sources.insert(resolved_path, program);

        Ok(())
    }

    /// Execute a pending module that has all its imports satisfied.
    fn execute_pending_module(&mut self, module_path: &crate::ModulePath) -> Result<(), JsError> {
        let program = self
            .pending_module_sources
            .remove(module_path)
            .ok_or_else(|| {
                JsError::internal_error(format!("Module '{}' not found", module_path))
            })?;

        // Save current state
        let saved_env = self.env.cheap_clone();
        let saved_module_path = self.current_module_path.take();

        // Set current module path for resolving nested imports
        self.current_module_path = Some(module_path.clone());

        // Create module environment (rooted so it persists for live bindings)
        let module_env = self.create_module_environment();
        // Root the module environment - it must persist for live bindings
        self.root_guard.guard(module_env.clone());
        self.env = module_env.cheap_clone();

        // Execute module using stack-based evaluation
        let result = self.execute_program_with_stack(&program);

        // Restore state
        self.env = saved_env;
        self.current_module_path = saved_module_path;

        result?;

        // Create module namespace object from exports
        let guard = self.heap.create_guard();
        let module_obj = self.create_object(&guard);

        // Drain exports to a vector to avoid borrow conflict
        let exports: Vec<_> = self.exports.drain().collect();

        // Create properties for exports with proper live binding support
        for (export_name, module_export) in exports {
            match module_export {
                ModuleExport::Direct { name, value } => {
                    // Check if there's a binding in the module environment
                    let has_binding = {
                        let env_ref = module_env.borrow();
                        if let Some(env_data) = env_ref.as_environment() {
                            let var_key = VarKey(name.cheap_clone());
                            env_data.bindings.contains_key(&var_key)
                        } else {
                            false
                        }
                    };

                    if has_binding {
                        // Direct export with binding: create getter for live binding
                        let getter_obj = guard.alloc();
                        {
                            let mut getter_ref = getter_obj.borrow_mut();
                            getter_ref.prototype = Some(self.function_prototype.cheap_clone());
                            getter_ref.exotic =
                                ExoticObject::Function(JsFunction::ModuleExportGetter {
                                    module_env: module_env.cheap_clone(),
                                    binding_name: name,
                                });
                        }

                        // Set as accessor property (getter only, no setter)
                        module_obj.borrow_mut().properties.insert(
                            PropertyKey::String(export_name),
                            Property::accessor(Some(getter_obj), None),
                        );
                    } else {
                        // Direct export without binding (e.g., namespace re-export: export * as ns)
                        // Use the stored value directly
                        module_obj
                            .borrow_mut()
                            .set_property(PropertyKey::String(export_name), value);
                    }
                }
                ModuleExport::ReExport {
                    source_module,
                    source_key,
                } => {
                    // Re-export: create getter that delegates to source module's property
                    // This enables live bindings through re-exports
                    let getter_obj = guard.alloc();
                    {
                        let mut getter_ref = getter_obj.borrow_mut();
                        getter_ref.prototype = Some(self.function_prototype.cheap_clone());
                        getter_ref.exotic =
                            ExoticObject::Function(JsFunction::ModuleReExportGetter {
                                source_module,
                                source_key,
                            });
                    }

                    // Set as accessor property (getter only, no setter)
                    module_obj.borrow_mut().properties.insert(
                        PropertyKey::String(export_name),
                        Property::accessor(Some(getter_obj), None),
                    );
                }
            }
        }

        // Root the module namespace object (lives forever)
        self.root_guard.guard(module_obj.clone());

        // Cache it by normalized path
        self.loaded_modules.insert(module_path.clone(), module_obj);

        Ok(())
    }

    /// Continue evaluation after providing modules or fulfilling orders
    pub fn continue_eval(&mut self) -> Result<crate::RuntimeResult, JsError> {
        // If we have a suspended execution state, resume it
        if let Some(mut state) = self.suspended_state.take() {
            // Resume execution from where we left off
            return self.run_to_completion_or_suspend(&mut state);
        }

        // First, try to execute any pending modules that have all their imports
        loop {
            // Collect all missing imports across all pending modules
            let mut all_missing: Vec<crate::ImportRequest> = Vec::new();
            let mut ready_modules: Vec<crate::ModulePath> = Vec::new();

            // Clone keys to avoid borrow issues
            let pending_keys: Vec<crate::ModulePath> =
                self.pending_module_sources.keys().cloned().collect();

            for module_path in &pending_keys {
                // Skip if already loaded
                if self.loaded_modules.contains_key(module_path) {
                    continue;
                }

                // Get the program to check its imports
                if let Some(program) = self.pending_module_sources.get(module_path) {
                    let imports = self.collect_import_requests(program, Some(module_path));
                    let missing = self.filter_missing_imports(imports);

                    if missing.is_empty() {
                        // This module is ready to execute
                        ready_modules.push(module_path.clone());
                    } else {
                        // Collect missing imports (dedupe by resolved path)
                        for req in missing {
                            let already_pending =
                                self.pending_module_sources.contains_key(&req.resolved_path);
                            let already_in_list = all_missing
                                .iter()
                                .any(|r| r.resolved_path == req.resolved_path);
                            if !already_pending && !already_in_list {
                                all_missing.push(req);
                            }
                        }
                    }
                }
            }

            // If we have modules ready to execute, execute them
            if !ready_modules.is_empty() {
                for module_path in ready_modules {
                    self.execute_pending_module(&module_path)?;
                }
                // Continue the loop to check if more modules are now ready
                continue;
            }

            // If we still have missing imports, return them
            if !all_missing.is_empty() {
                return Ok(crate::RuntimeResult::NeedImports(all_missing));
            }

            // No more pending modules to process
            break;
        }

        // If we have a pending program waiting for imports, check if we can execute it now
        if let Some(program) = self.pending_program.take() {
            // Re-check imports using the main module path as base
            let imports = self.collect_import_requests(&program, self.main_module_path.as_ref());
            let missing = self.filter_missing_imports(imports);
            let missing = Self::dedupe_import_requests(missing);

            if !missing.is_empty() {
                // Still missing imports - save program again and return
                self.pending_program = Some(program);
                return Ok(crate::RuntimeResult::NeedImports(missing));
            }

            // All imports satisfied - execute the program
            self.start_execution();
            let mut state = self.get_execution_state();
            state.init_for_program(&program);
            let result = self.run_to_completion_or_suspend(&mut state);
            // Return state to pool if execution completed (not suspended)
            if self.suspended_state.is_none() {
                self.return_execution_state(state);
            }
            return result;
        }

        // Check if there are pending orders to return
        if !self.pending_orders.is_empty() {
            let pending = std::mem::take(&mut self.pending_orders);
            let cancelled = std::mem::take(&mut self.cancelled_orders);
            return Ok(crate::RuntimeResult::Suspended { pending, cancelled });
        }

        // Also check for unfulfilled orders from previous suspension
        if !self.order_callbacks.is_empty() {
            let cancelled = std::mem::take(&mut self.cancelled_orders);
            return Ok(crate::RuntimeResult::Suspended {
                pending: Vec::new(),
                cancelled,
            });
        }

        // No pending orders - execution is complete
        Ok(crate::RuntimeResult::Complete(
            crate::RuntimeValue::unguarded(JsValue::Undefined),
        ))
    }

    /// Fulfill orders with responses from the host
    pub fn fulfill_orders(&mut self, responses: Vec<crate::OrderResponse>) -> Result<(), JsError> {
        // Process each response, keeping its RuntimeValue alive while we resolve
        for response in responses {
            if let Some((resolve_fn, reject_fn)) = self.order_callbacks.remove(&response.id) {
                match response.result {
                    Ok(runtime_value) => {
                        // Clone the value while runtime_value (and its guard) is still in scope.
                        // The guard keeps the object alive during call_function.
                        let value = runtime_value.value().clone();
                        self.call_function(
                            JsValue::Object(resolve_fn),
                            JsValue::Undefined,
                            &[value],
                        )?;
                        // runtime_value dropped here after call_function stores the value
                    }
                    Err(error) => {
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
                self.global_env.cheap_clone(),
            )));
        }
        env
    }

    /// Resolve a module specifier to a normalized path.
    ///
    /// Uses the current module path (or main module path) as the base for resolving
    /// relative imports like `./foo` or `../bar`.
    pub fn resolve_module_specifier(&self, specifier: &str) -> crate::ModulePath {
        let base = self
            .current_module_path
            .as_ref()
            .or(self.main_module_path.as_ref());
        crate::ModulePath::resolve(specifier, base)
    }

    /// Collect all import requests from a program, resolving relative paths.
    ///
    /// Uses the same path for both resolution base and importer.
    /// For main module imports, use `collect_import_requests_internal` instead.
    fn collect_import_requests(
        &self,
        program: &Program,
        module_path: Option<&crate::ModulePath>,
    ) -> Vec<crate::ImportRequest> {
        self.collect_import_requests_internal(program, module_path, module_path)
    }

    /// Collect all import requests from a program with separate resolution base and importer.
    ///
    /// - `resolve_base`: Used to resolve relative paths (e.g., ./foo becomes /project/src/foo)
    /// - `importer`: Stored in ImportRequest.importer (None for main module)
    fn collect_import_requests_internal(
        &self,
        program: &Program,
        resolve_base: Option<&crate::ModulePath>,
        importer: Option<&crate::ModulePath>,
    ) -> Vec<crate::ImportRequest> {
        use crate::ast::Statement;

        let mut imports = Vec::new();

        for stmt in program.body.iter() {
            let specifier = match stmt {
                Statement::Import(import) => Some(import.source.value.to_string()),
                Statement::Export(export) => {
                    // Re-export from another module: export { foo } from "./bar"
                    export.source.as_ref().map(|s| s.value.to_string())
                }
                _ => None,
            };

            if let Some(spec) = specifier {
                let resolved = crate::ModulePath::resolve(&spec, resolve_base);
                imports.push(crate::ImportRequest {
                    specifier: spec,
                    resolved_path: resolved,
                    importer: importer.cloned(),
                });
            }
        }

        imports
    }

    /// Filter import requests to only those that are missing (not internal, not already loaded).
    fn filter_missing_imports(
        &self,
        imports: Vec<crate::ImportRequest>,
    ) -> Vec<crate::ImportRequest> {
        imports
            .into_iter()
            .filter(|req| {
                // Internal modules are resolved automatically
                if self.is_internal_module(&req.specifier) {
                    return false;
                }
                // Already loaded modules don't need to be requested
                !self.loaded_modules.contains_key(&req.resolved_path)
            })
            .collect()
    }

    /// Deduplicate import requests by resolved path.
    fn dedupe_import_requests(imports: Vec<crate::ImportRequest>) -> Vec<crate::ImportRequest> {
        let mut seen = std::collections::HashSet::new();
        imports
            .into_iter()
            .filter(|req| seen.insert(req.resolved_path.clone()))
            .collect()
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Environment Operations
    // ═══════════════════════════════════════════════════════════════════════════

    /// Define a variable in the current environment
    pub fn env_define(&mut self, name: JsString, value: JsValue, mutable: bool) {
        let mut env_ref = self.env.borrow_mut();
        if let Some(data) = env_ref.as_environment_mut() {
            data.bindings.insert(
                VarKey(name),
                Binding {
                    value,
                    mutable,
                    initialized: true,
                    import_binding: None,
                },
            );
        }
    }

    /// Define an import binding in the current environment (for live bindings)
    pub fn env_define_import(
        &mut self,
        name: JsString,
        module_obj: Gc<JsObject>,
        property_key: PropertyKey,
    ) {
        let mut env_ref = self.env.borrow_mut();
        if let Some(data) = env_ref.as_environment_mut() {
            data.bindings.insert(
                VarKey(name),
                Binding {
                    value: JsValue::Undefined, // Not used for import bindings
                    mutable: false,            // Imports are always read-only
                    initialized: true,
                    import_binding: Some(ImportBinding {
                        module_obj,
                        property_key,
                    }),
                },
            );
        }
    }

    /// Get a variable from the environment chain
    // FIXME: return guarded
    pub fn env_get(&self, name: &JsString) -> Result<JsValue, JsError> {
        let mut current = Some(self.env.cheap_clone());
        let mut depth = 0;
        // Create VarKey once for pointer-based lookup
        let key = VarKey(name.cheap_clone());

        while let Some(env) = current {
            let env_ref = env.borrow();
            if let Some(data) = env_ref.as_environment() {
                if let Some(binding) = data.bindings.get(&key) {
                    if !binding.initialized {
                        return Err(JsError::reference_error(format!(
                            "Cannot access '{}' before initialization",
                            name
                        )));
                    }
                    // Handle import bindings (for live bindings)
                    if let Some(ref import_binding) = binding.import_binding {
                        return self.resolve_import_binding(import_binding);
                    }
                    return Ok(binding.value.clone());
                }
                current = data.outer.cheap_clone();
                depth += 1;
            } else {
                eprintln!("[env_get] {} NOT FOUND: env id={} at depth {} is NOT an environment! exotic={:?}",
                    name, env.id(), depth, std::mem::discriminant(&env_ref.exotic));
                break;
            }
        }

        // Check global object properties
        let global = self.global.borrow();
        if let Some(prop) = global.get_property(&PropertyKey::String(name.cheap_clone())) {
            return Ok(prop);
        }

        Err(JsError::reference_error(name.to_string()))
    }

    /// Resolve an import binding by reading from the module's environment
    /// This handles both direct exports (ModuleExportGetter) and re-exports (ModuleReExportGetter)
    fn resolve_import_binding(&self, import_binding: &ImportBinding) -> Result<JsValue, JsError> {
        self.resolve_module_property(&import_binding.module_obj, &import_binding.property_key)
    }

    /// Resolve a property from a module namespace object, handling live bindings
    /// This recursively resolves through re-export chains
    #[allow(clippy::only_used_in_recursion)]
    fn resolve_module_property(
        &self,
        module_obj: &Gc<JsObject>,
        prop_key: &PropertyKey,
    ) -> Result<JsValue, JsError> {
        // Get the property descriptor from the module namespace object
        let prop_desc = module_obj.borrow().get_property_descriptor(prop_key);

        match prop_desc {
            Some((prop, _)) if prop.is_accessor() => {
                // The property has a getter - could be ModuleExportGetter or ModuleReExportGetter
                if let Some(getter) = prop.getter() {
                    let getter_ref = getter.borrow();
                    match &getter_ref.exotic {
                        ExoticObject::Function(JsFunction::ModuleExportGetter {
                            module_env,
                            binding_name,
                        }) => {
                            // Direct export: read from the module's environment
                            let env_ref = module_env.borrow();
                            if let Some(env_data) = env_ref.as_environment() {
                                let var_key = VarKey(binding_name.cheap_clone());
                                if let Some(binding) = env_data.bindings.get(&var_key) {
                                    return Ok(binding.value.clone());
                                }
                            }
                        }
                        ExoticObject::Function(JsFunction::ModuleReExportGetter {
                            source_module,
                            source_key,
                        }) => {
                            // Re-export: recursively resolve from source module
                            return self.resolve_module_property(source_module, source_key);
                        }
                        _ => {}
                    }
                }
                Ok(JsValue::Undefined)
            }
            Some((prop, _)) => Ok(prop.value.clone()),
            None => Ok(JsValue::Undefined),
        }
    }

    /// Set a variable in the environment chain
    pub fn env_set(&mut self, name: &JsString, value: JsValue) -> Result<(), JsError> {
        let mut current = Some(self.env.clone());
        // Create VarKey once for pointer-based lookup
        let key = VarKey(name.cheap_clone());

        while let Some(env) = current {
            let mut env_ref = env.borrow_mut();
            if let Some(data) = env_ref.as_environment_mut() {
                if let Some(binding) = data.bindings.get_mut(&key) {
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

        Err(JsError::reference_error(name.to_string()))
    }

    /// Push a new scope and return the saved environment
    pub fn push_scope(&mut self) -> EnvRef {
        let (new_env, new_guard) =
            create_environment_unrooted(&self.heap, Some(self.env.cheap_clone()));

        let old_env = self.env.cheap_clone();
        self.env = new_env;
        self.env_guards.push(new_guard);
        old_env
    }

    /// Pop scope by restoring saved environment
    pub fn pop_scope(&mut self, saved_env: EnvRef) {
        self.env = saved_env;
        // Pop the guard that was pushed when this scope was created
        self.env_guards.pop();
    }

    /// Push an environment guard (for env changes without push_scope)
    pub fn push_env_guard(&mut self, guard: Guard<JsObject>) {
        self.env_guards.push(guard);
    }

    /// Pop an environment guard
    pub fn pop_env_guard(&mut self) {
        self.env_guards.pop();
    }

    /// Get the number of environment guards (for debugging)
    pub fn env_guards_len(&self) -> usize {
        self.env_guards.len()
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Object/Array/Function Creation
    // ═══════════════════════════════════════════════════════════════════════════

    /// Create a new plain object with `object_prototype`.
    /// Caller provides the guard to control object lifetime.
    pub fn create_object(&mut self, guard: &Guard<JsObject>) -> Gc<JsObject> {
        let obj = guard.alloc();
        obj.borrow_mut().prototype = Some(self.object_prototype.cheap_clone());
        obj
    }

    /// Create a new plain object without prototype.
    /// Caller provides the guard to control object lifetime.
    pub fn create_object_raw(&mut self, guard: &Guard<JsObject>) -> Gc<JsObject> {
        guard.alloc()
    }

    /// Create an object with pre-allocated property capacity.
    /// Use this when you know the number of properties upfront to avoid hashmap resizing.
    pub fn create_object_with_capacity(
        &mut self,
        guard: &Guard<JsObject>,
        capacity: usize,
    ) -> Gc<JsObject> {
        let obj = guard.alloc();
        {
            let mut obj_ref = obj.borrow_mut();
            obj_ref.prototype = Some(self.object_prototype.cheap_clone());
            obj_ref.properties.reserve(capacity);
        }
        obj
    }

    /// Create a RegExp literal object.
    /// Caller provides the guard to control object lifetime.
    fn create_regexp_literal(
        &mut self,
        guard: &Guard<JsObject>,
        pattern: &str,
        flags: &str,
    ) -> Gc<JsObject> {
        // Pre-intern all property keys
        let source_key = PropertyKey::String(self.intern("source"));
        let flags_key = PropertyKey::String(self.intern("flags"));
        let global_key = PropertyKey::String(self.intern("global"));
        let ignore_case_key = PropertyKey::String(self.intern("ignoreCase"));
        let multiline_key = PropertyKey::String(self.intern("multiline"));
        let dot_all_key = PropertyKey::String(self.intern("dotAll"));
        let unicode_key = PropertyKey::String(self.intern("unicode"));
        let sticky_key = PropertyKey::String(self.intern("sticky"));
        let last_index_key = PropertyKey::String(self.intern("lastIndex"));

        let regexp_obj = self.create_object_raw(guard);
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
        regexp_obj
    }

    /// Create a new array with elements and `array_prototype`.
    /// Caller provides the guard to control object lifetime.
    pub fn create_array_from(
        &mut self,
        guard: &Guard<JsObject>,
        elements: Vec<JsValue>,
    ) -> Gc<JsObject> {
        let arr = guard.alloc();
        {
            let mut arr_ref = arr.borrow_mut();
            arr_ref.prototype = Some(self.array_prototype.cheap_clone());
            arr_ref.exotic = ExoticObject::Array { elements };
        }
        arr
    }

    /// Create a new empty array with `array_prototype`.
    /// Caller provides the guard to control object lifetime.
    pub fn create_empty_array(&mut self, guard: &Guard<JsObject>) -> Gc<JsObject> {
        self.create_array_from(guard, Vec::new())
    }

    /// Create an interpreted function object.
    /// Caller provides the guard to control object lifetime.
    #[allow(clippy::too_many_arguments)]
    pub fn create_interpreted_function(
        &mut self,
        guard: &Guard<JsObject>,
        name: Option<JsString>,
        params: Rc<[FunctionParam]>,
        body: Rc<FunctionBody>,
        closure: EnvRef,
        span: Span,
        generator: bool,
        async_: bool,
    ) -> Gc<JsObject> {
        let func_obj = guard.alloc();
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
        func_obj
    }

    /// Create a native function object.
    /// Caller provides the guard to control object lifetime.
    pub fn create_native_fn(
        &mut self,
        guard: &Guard<JsObject>,
        name: &str,
        func: NativeFn,
        arity: usize,
    ) -> Gc<JsObject> {
        let name_str = self.intern(name);
        let func_obj = guard.alloc();
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

    // ═══════════════════════════════════════════════════════════════════════════
    // Builtin Helper Methods
    // ═══════════════════════════════════════════════════════════════════════════

    /// Intern a string
    pub fn intern(&mut self, s: &str) -> JsString {
        self.string_dict.get_or_insert(s)
    }

    /// Set the name of an anonymous function (for function name inference).
    /// This is called when an anonymous function/arrow function is assigned to a variable.
    /// Only sets the name if the function doesn't already have one.
    fn set_function_name(&self, value: &JsValue, name: JsString) {
        if let JsValue::Object(obj) = value {
            let mut obj_ref = obj.borrow_mut();
            if let ExoticObject::Function(JsFunction::Interpreted(interp)) = &mut obj_ref.exotic {
                if interp.name.is_none() {
                    interp.name = Some(name);
                }
            }
        }
    }

    /// Extract string key from ObjectPropertyKey (for destructuring)
    fn extract_property_key_string(&self, key: &ObjectPropertyKey) -> Option<JsString> {
        match key {
            ObjectPropertyKey::Identifier(id) => Some(id.name.cheap_clone()),
            ObjectPropertyKey::String(s) => Some(s.value.cheap_clone()),
            ObjectPropertyKey::Number(l) => {
                if let LiteralValue::Number(n) = &l.value {
                    Some(n.to_string().into())
                } else {
                    None
                }
            }
            ObjectPropertyKey::Computed(_) => None, // Can't statically determine
            ObjectPropertyKey::PrivateIdentifier(id) => Some(id.name.cheap_clone()),
        }
    }

    /// Create a rooted native function for global constructors.
    /// The function is permanently rooted and never collected.
    /// Use this for built-in constructors during initialization.
    pub fn create_native_function(
        &mut self,
        name: &str,
        func: NativeFn,
        arity: usize,
    ) -> Gc<JsObject> {
        let name_str = self.intern(name);
        let length_key = PropertyKey::String(self.intern("length"));
        let name_key = PropertyKey::String(self.intern("name"));
        let func_obj = self.root_guard.alloc();
        {
            let mut f_ref = func_obj.borrow_mut();
            f_ref.prototype = Some(self.function_prototype.clone());
            f_ref.exotic = ExoticObject::Function(JsFunction::Native(NativeFunction {
                name: name_str.cheap_clone(),
                func,
                arity,
            }));
            // Set length property (number of formal parameters)
            f_ref.set_property(length_key, JsValue::Number(arity as f64));
            // Set name property
            f_ref.set_property(name_key, JsValue::String(name_str));
        }
        func_obj
    }

    /// Create a function object from any JsFunction variant.
    /// Caller provides the guard to control object lifetime.
    pub fn create_js_function(
        &mut self,
        guard: &Guard<JsObject>,
        func: JsFunction,
    ) -> Gc<JsObject> {
        let length_key = PropertyKey::String(self.intern("length"));
        let name_key = PropertyKey::String(self.intern("name"));

        // Extract name and arity from the function
        let (func_name, arity) = match &func {
            JsFunction::Interpreted(f) => {
                let name = f.name.clone().unwrap_or_else(|| self.intern(""));
                let arity = f.params.len();
                (name, arity)
            }
            JsFunction::Native(f) => (f.name.cheap_clone(), f.arity),
            JsFunction::Bound(b) => {
                // Bound functions: compute name and length from target
                let (target_name, target_length) =
                    if let ExoticObject::Function(target_func) = &b.target.borrow().exotic {
                        match target_func {
                            JsFunction::Interpreted(f) => {
                                let name = f.name.as_ref().map(|n| n.as_str()).unwrap_or("");
                                let len = f
                                    .params
                                    .iter()
                                    .filter(|p| !matches!(p.pattern, Pattern::Rest(_)))
                                    .count();
                                (name.to_string(), len)
                            }
                            JsFunction::Native(f) => (f.name.to_string(), f.arity),
                            _ => (String::new(), 0),
                        }
                    } else {
                        (String::new(), 0)
                    };
                let name = self.intern(&format!("bound {}", target_name));
                let arity = target_length.saturating_sub(b.bound_args.len());
                (name, arity)
            }
            _ => (self.intern(""), 0),
        };

        let func_obj = guard.alloc();
        {
            let mut f_ref = func_obj.borrow_mut();
            f_ref.prototype = Some(self.function_prototype.clone());
            f_ref.exotic = ExoticObject::Function(func);
            // Set length property (number of formal parameters)
            f_ref.set_property(length_key, JsValue::Number(arity as f64));
            // Set name property
            f_ref.set_property(name_key, JsValue::String(func_name));
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
        let key = PropertyKey::String(self.intern(name));
        obj.borrow_mut()
            .set_property(key, JsValue::Object(func_obj));
    }

    /// Guard a value to prevent it from being garbage collected.
    /// Returns Some(guard) if the value is an object, None otherwise.
    pub fn guard_value(&mut self, value: &JsValue) -> Option<Guard<JsObject>> {
        if let JsValue::Object(obj) = value {
            let guard = self.heap.create_guard();
            guard.guard(obj.cheap_clone());
            Some(guard)
        } else {
            None
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ExecutionState Pool
    // ═══════════════════════════════════════════════════════════════════════════

    /// Maximum number of ExecutionStates to keep in the pool.
    const EXECUTION_STATE_POOL_MAX: usize = 4;

    /// Get an ExecutionState from the pool, or create a new one.
    /// The state is reset before being returned.
    fn get_execution_state(&mut self) -> stack::ExecutionState {
        self.execution_state_pool.pop().unwrap_or_default()
    }

    /// Return an ExecutionState to the pool for reuse.
    /// The state is reset before being added to the pool.
    fn return_execution_state(&mut self, mut state: stack::ExecutionState) {
        if self.execution_state_pool.len() < Self::EXECUTION_STATE_POOL_MAX {
            state.reset();
            self.execution_state_pool.push(state);
        }
        // Otherwise drop it - pool is full
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Generator Support
    // ═══════════════════════════════════════════════════════════════════════════

    /// Resume a generator execution using stack-based evaluation
    /// This executes the generator body until the next yield or return.
    ///
    /// State management:
    /// - On first call (started=false): create new execution state and environment
    /// - On subsequent calls: restore saved execution state from saved_generator_states
    /// - On yield: save execution state to saved_generator_states
    /// - On completion: remove saved state and mark generator as completed
    pub fn resume_generator(
        &mut self,
        gen_state: &Rc<RefCell<crate::value::GeneratorState>>,
    ) -> Result<Guarded, JsError> {
        use stack::{ExecutionState, Frame, StackCompletion, StepResult};

        // Check if generator is already completed
        {
            let state = gen_state.borrow();
            if state.status == GeneratorStatus::Completed {
                return Ok(builtins::create_generator_result(
                    self,
                    JsValue::Undefined,
                    true,
                ));
            }
        }

        // Get generator info and check if we have saved state
        let (gen_id, started, sent_value) = {
            let state = gen_state.borrow();
            (state.id, state.started, state.sent_value.clone())
        };

        // Save current interpreter environment
        let saved_env = self.env.cheap_clone();

        // Either restore saved state or create new one
        // We need to keep the env_guard alive during execution
        let mut _restored_env_guard: Option<Guard<JsObject>> = None;

        let mut exec_state = if started {
            // Restore saved execution state
            let saved = self.saved_generator_states.remove(&gen_id).ok_or_else(|| {
                JsError::internal_error("Generator marked as started but no saved state found")
            })?;

            // Restore interpreter environment
            self.env = saved.env;
            // Keep the guard alive during this call
            _restored_env_guard = Some(saved.guard);

            // Rebuild execution state from saved data
            let mut state = ExecutionState::new();
            state.frames = saved.frames;
            // Convert saved JsValues back to Guarded (without guards - they'll be protected by being on stack)
            state.values = saved.values.into_iter().map(Guarded::unguarded).collect();
            state.completion = saved.completion;

            // Push the sent value onto the value stack for the YieldResume frame to pick up
            state.push_value(Guarded::unguarded(sent_value));

            state
        } else {
            // First call - create new execution state
            let gs = gen_state.borrow();
            let body = gs.body.cheap_clone();
            let params = gs.params.cheap_clone();
            let args_clone = gs.args.clone();
            let closure = gs.closure.cheap_clone();
            drop(gs);

            // Create function environment
            let (func_env, func_guard) = create_environment_unrooted(&self.heap, Some(closure));
            self.env = func_env.cheap_clone();
            self.push_env_guard(func_guard);

            // Bind parameters
            for (i, param) in params.iter().enumerate() {
                let arg_value = args_clone.get(i).cloned().unwrap_or(JsValue::Undefined);
                if let Err(e) = self.bind_pattern(&param.pattern, arg_value, true) {
                    self.pop_env_guard();
                    self.env = saved_env;
                    return Err(e);
                }
            }

            // Mark as started
            gen_state.borrow_mut().started = true;

            // Create execution state with generator body
            let mut state = ExecutionState::new();
            state.push_frame(Frame::Program {
                statements: body.body.cheap_clone(),
                index: 0,
            });
            state
        };

        // Run until yield, return, or completion
        let result = loop {
            match self.step(&mut exec_state) {
                StepResult::Continue => continue,
                StepResult::Done(guarded) => {
                    // Generator completed normally - clean up saved state
                    self.saved_generator_states.remove(&gen_id);
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    // Restore caller's environment
                    self.env = saved_env.cheap_clone();
                    break Ok(builtins::create_generator_result(self, guarded.value, true));
                }
                StepResult::Suspend(_promise) => {
                    // Await encountered in async generator - not fully supported yet
                    // For now, treat as completion
                    self.saved_generator_states.remove(&gen_id);
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    // Restore caller's environment
                    self.env = saved_env.cheap_clone();
                    break Ok(builtins::create_generator_result(
                        self,
                        JsValue::Undefined,
                        true,
                    ));
                }
                StepResult::Error(e) => {
                    // Check if it's a yield
                    if let JsError::GeneratorYield { value } = e {
                        // Collect all Gc references from frames and values
                        let mut gc_refs: Vec<Gc<JsObject>> = Vec::new();
                        gc_refs.push(self.env.clone());
                        for frame in &exec_state.frames {
                            frame.collect_gc_refs(&mut gc_refs);
                        }
                        for guarded in &exec_state.values {
                            if let JsValue::Object(obj) = &guarded.value {
                                gc_refs.push(obj.clone());
                            }
                        }

                        // Create a guard and mark all collected references
                        let guard = self.heap.create_guard();
                        for gc_ref in gc_refs {
                            guard.guard(gc_ref);
                        }

                        // Save execution state for later resumption
                        let saved = SavedGeneratorExecution {
                            env: self.env.cheap_clone(),
                            guard,
                            frames: std::mem::take(&mut exec_state.frames),
                            values: exec_state.values.drain(..).map(|g| g.value).collect(),
                            completion: std::mem::replace(
                                &mut exec_state.completion,
                                StackCompletion::Normal,
                            ),
                        };
                        self.saved_generator_states.insert(gen_id, saved);

                        // Restore original environment
                        self.env = saved_env;

                        break Ok(builtins::create_generator_result(self, value, false));
                    }
                    // Real error - clean up and propagate
                    self.saved_generator_states.remove(&gen_id);
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    self.env = saved_env;
                    break Err(e);
                }
            }
        };

        result
    }

    /// Resume a generator with throw semantics (for Generator.prototype.throw)
    pub fn resume_generator_with_throw(
        &mut self,
        gen_state: &Rc<RefCell<crate::value::GeneratorState>>,
    ) -> Result<Guarded, JsError> {
        use stack::{ExecutionState, StackCompletion, StepResult};

        let (gen_id, exception, started) = {
            let state = gen_state.borrow();
            (state.id, state.sent_value.clone(), state.started)
        };

        // If generator hasn't started, we can't throw into it
        if !started {
            self.saved_generator_states.remove(&gen_id);
            gen_state.borrow_mut().status = GeneratorStatus::Completed;
            return Err(JsError::ThrownValue { value: exception });
        }

        // Get saved state
        let saved = match self.saved_generator_states.remove(&gen_id) {
            Some(s) => s,
            None => {
                gen_state.borrow_mut().status = GeneratorStatus::Completed;
                return Err(JsError::ThrownValue { value: exception });
            }
        };

        let saved_env = self.env.cheap_clone();

        // Restore interpreter environment
        self.env = saved.env;
        let _restored_env_guard = saved.guard;

        // Rebuild execution state from saved data
        let mut exec_state = ExecutionState::new();
        exec_state.frames = saved.frames;
        exec_state.values = saved.values.into_iter().map(Guarded::unguarded).collect();
        exec_state.completion = saved.completion;

        // Instead of pushing sent value, we trigger an error
        // The error will be handled by handle_error which will look for TryBlock frames

        // First, try to handle the thrown exception (ONCE, not in the loop)
        // This mimics what happens when an error occurs - handle_error looks for TryBlock
        if let Some(step_result) =
            self.handle_error(&mut exec_state, JsError::ThrownValue { value: exception })
        {
            match step_result {
                StepResult::Error(e) => {
                    // Error propagated out (no catch found)
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    self.env = saved_env;
                    return Err(e);
                }
                StepResult::Done(guarded) => {
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    self.env = saved_env;
                    return Ok(builtins::create_generator_result(self, guarded.value, true));
                }
                _ => {} // Continue below
            }
        }
        // Error was handled (caught by try/catch), continue execution

        // Run until yield, return, completion, or error handling
        let result = loop {
            match self.step(&mut exec_state) {
                StepResult::Continue => continue,
                StepResult::Done(guarded) => {
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    self.env = saved_env.cheap_clone();
                    break Ok(builtins::create_generator_result(self, guarded.value, true));
                }
                StepResult::Suspend(_) => {
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    self.env = saved_env.cheap_clone();
                    break Ok(builtins::create_generator_result(
                        self,
                        JsValue::Undefined,
                        true,
                    ));
                }
                StepResult::Error(e) => {
                    if let JsError::GeneratorYield { value } = e {
                        // Collect all Gc references from frames and values
                        let mut gc_refs: Vec<Gc<JsObject>> = Vec::new();
                        gc_refs.push(self.env.clone());
                        for frame in &exec_state.frames {
                            frame.collect_gc_refs(&mut gc_refs);
                        }
                        for guarded in &exec_state.values {
                            if let JsValue::Object(obj) = &guarded.value {
                                gc_refs.push(obj.clone());
                            }
                        }

                        // Create a guard and mark all collected references
                        let guard = self.heap.create_guard();
                        for gc_ref in gc_refs {
                            guard.guard(gc_ref);
                        }

                        // Save execution state for later resumption
                        let saved_state = SavedGeneratorExecution {
                            env: self.env.cheap_clone(),
                            guard,
                            frames: std::mem::take(&mut exec_state.frames),
                            values: exec_state.values.drain(..).map(|g| g.value).collect(),
                            completion: std::mem::replace(
                                &mut exec_state.completion,
                                StackCompletion::Normal,
                            ),
                        };
                        self.saved_generator_states.insert(gen_id, saved_state);

                        self.env = saved_env;
                        break Ok(builtins::create_generator_result(self, value, false));
                    }

                    // Real error - clean up and propagate
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    self.env = saved_env;
                    break Err(e);
                }
            }
        };

        result
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Evaluation Entry Point
    // ═══════════════════════════════════════════════════════════════════════════

    /// Evaluate source code and return the result
    ///
    /// Uses stack-based evaluation for suspendable execution.
    pub fn eval_simple(&mut self, source: &str) -> Result<JsValue, JsError> {
        self.eval_with_stack(source)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Declaration Execution (used by stack-based execution)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Execute a single statement using stack-based execution
    /// Used for static blocks and other simple statement execution contexts
    fn execute_simple_statement(&mut self, stmt: &Statement) -> Result<JsValue, JsError> {
        let mut state = self.get_execution_state();
        state.init_for_statement(stmt);
        let result = match self.run(&mut state) {
            stack::StepResult::Done(g) => Ok(g.value),
            stack::StepResult::Error(e) => Err(e),
            stack::StepResult::Suspend(_) => Err(JsError::type_error(
                "Statement execution cannot be suspended",
            )),
            stack::StepResult::Continue => Ok(JsValue::Undefined),
        };
        self.return_execution_state(state);
        result
    }

    fn execute_variable_declaration(&mut self, decl: &VariableDeclaration) -> Result<(), JsError> {
        let mutable = matches!(decl.kind, VariableKind::Let | VariableKind::Var);
        let is_var = decl.kind == VariableKind::Var;

        for declarator in decl.declarations.iter() {
            // Keep guard alive until bind_pattern transfers ownership to env
            let Guarded {
                value: init_value,
                guard: _init_guard,
            } = match &declarator.init {
                Some(expr) => self.evaluate_expression(expr)?,
                None => Guarded::unguarded(JsValue::Undefined),
            };

            // Function name inference: if binding a simple identifier to an anonymous function,
            // set the function's name to the variable name
            if let Pattern::Identifier(id) = &declarator.id {
                self.set_function_name(&init_value, id.name.cheap_clone());
            }

            if is_var {
                // For var, use assignment to the hoisted binding in outer scope
                // The variable was already hoisted to undefined, now we just assign
                self.assign_pattern(&declarator.id, init_value)?;
            } else {
                // For let/const, define in current scope
                // bind_pattern calls env_define which establishes ownership
                self.bind_pattern(&declarator.id, init_value, mutable)?;
            }
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

                // First pass: collect keys that are explicitly destructured
                let mut extracted_keys: Vec<JsString> = Vec::new();
                for prop in &obj_pattern.properties {
                    if let ObjectPatternProperty::KeyValue { key, .. } = prop {
                        if let Some(key_str) = self.extract_property_key_string(key) {
                            extracted_keys.push(key_str);
                        }
                    }
                }

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
                                ObjectPropertyKey::PrivateIdentifier(id) => id.name.cheap_clone(),
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
                        ObjectPatternProperty::Rest(rest) => {
                            // Create a new object with remaining properties
                            let guard = self.heap.create_guard();
                            let rest_obj = self.create_object(&guard);

                            // Copy all properties except those explicitly extracted
                            let obj_ref = obj.borrow();
                            for (key, prop) in obj_ref.properties.iter() {
                                let should_include = match key {
                                    PropertyKey::String(s) => {
                                        !extracted_keys.iter().any(|k| k == s)
                                    }
                                    PropertyKey::Symbol(_) => true, // Symbols are always included
                                    PropertyKey::Index(_) => true,  // Indices are always included
                                };
                                if should_include {
                                    rest_obj
                                        .borrow_mut()
                                        .set_property(key.clone(), prop.value.clone());
                                }
                            }
                            drop(obj_ref);

                            self.bind_pattern(&rest.argument, JsValue::Object(rest_obj), mutable)?;
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
                if let Some(elements) = obj_ref.array_elements() {
                    elements.to_vec()
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
                        let guard = self.heap.create_guard();
                        let rest_array = self.create_array_from(&guard, remaining);
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

    // ═══════════════════════════════════════════════════════════════════════════
    // Module Resolution (used by stack-based execution)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Resolve a module specifier to a module namespace object.
    ///
    /// The specifier is resolved relative to the current module path before
    /// looking up in the loaded modules cache.
    fn resolve_module(&mut self, specifier: &str) -> Result<Gc<JsObject>, JsError> {
        // Check internal modules first (use original specifier for internal modules)
        if let Some(module) = self.resolve_internal_module(specifier)? {
            return Ok(module);
        }

        // Resolve the specifier to a normalized path
        let resolved_path = self.resolve_module_specifier(specifier);

        // Check loaded external modules by resolved path
        if let Some(module) = self.loaded_modules.get(&resolved_path) {
            return Ok(module.clone());
        }

        Err(JsError::reference_error(format!(
            "Module '{}' not found (resolved to '{}')",
            specifier, resolved_path
        )))
    }

    /// Resolve an internal module (creates module object on first access)
    fn resolve_internal_module(
        &mut self,
        specifier: &str,
    ) -> Result<Option<Gc<JsObject>>, JsError> {
        // Return cached if exists
        if let Some(cached) = self.internal_module_cache.get(specifier) {
            return Ok(Some(cached.cheap_clone()));
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
        let guard = self.heap.create_guard();
        let module_obj = match module_kind {
            crate::InternalModuleKind::Native(exports) => {
                self.create_native_module_object(&guard, &exports)?
            }
            crate::InternalModuleKind::Source(source) => {
                self.create_source_module_object(&guard, specifier, &source)?
            }
        };

        // Root the module (lives forever)
        self.root_guard.guard(module_obj.clone());

        // Cache it
        self.internal_module_cache
            .insert(specifier.to_string(), module_obj.clone());

        Ok(Some(module_obj))
    }

    /// Create module object from native exports
    fn create_native_module_object(
        &mut self,
        guard: &Guard<JsObject>,
        exports: &[(String, crate::InternalExport)],
    ) -> Result<Gc<JsObject>, JsError> {
        let module_obj = self.create_object(guard);

        for (name, export) in exports {
            let key = PropertyKey::String(self.intern(name));
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

        Ok(module_obj)
    }

    /// Create module object from TypeScript source
    // FIXME: move up to other source parsing code?
    fn create_source_module_object(
        &mut self,
        guard: &Guard<JsObject>,
        _specifier: &str,
        source: &str,
    ) -> Result<Gc<JsObject>, JsError> {
        // Parse the source
        let mut parser = Parser::new(source, &mut self.string_dict);
        let program = parser.parse_program()?;

        // Save current environment and exports
        let saved_env = self.env.cheap_clone();
        let saved_exports = std::mem::take(&mut self.exports);

        // Create module environment (rooted so it persists for live bindings)
        let module_env = self.create_module_environment();
        // Root the module environment - it must persist for live bindings
        self.root_guard.guard(module_env.clone());
        self.env = module_env.cheap_clone();

        // Execute the module body using stack-based evaluation
        let result = self.execute_program_with_stack(&program);

        // Restore environment
        self.env = saved_env;

        // Handle errors
        result?;

        // Create module namespace object from exports
        let module_obj = self.create_object(guard);

        // Drain exports to a vector to avoid borrow conflict
        let exports: Vec<_> = self.exports.drain().collect();

        // Create properties for exports with proper live binding support
        for (export_name, module_export) in exports {
            match module_export {
                ModuleExport::Direct { name, value } => {
                    // Check if there's a binding in the module environment
                    let has_binding = {
                        let env_ref = module_env.borrow();
                        if let Some(env_data) = env_ref.as_environment() {
                            let var_key = VarKey(name.cheap_clone());
                            env_data.bindings.contains_key(&var_key)
                        } else {
                            false
                        }
                    };

                    if has_binding {
                        // Direct export with binding: create getter for live binding
                        let getter_obj = guard.alloc();
                        {
                            let mut getter_ref = getter_obj.borrow_mut();
                            getter_ref.prototype = Some(self.function_prototype.cheap_clone());
                            getter_ref.exotic =
                                ExoticObject::Function(JsFunction::ModuleExportGetter {
                                    module_env: module_env.cheap_clone(),
                                    binding_name: name,
                                });
                        }

                        // Set as accessor property (getter only, no setter)
                        module_obj.borrow_mut().properties.insert(
                            PropertyKey::String(export_name),
                            Property::accessor(Some(getter_obj), None),
                        );
                    } else {
                        // Direct export without binding (e.g., namespace re-export)
                        // Use the stored value directly
                        module_obj
                            .borrow_mut()
                            .set_property(PropertyKey::String(export_name), value);
                    }
                }
                ModuleExport::ReExport {
                    source_module,
                    source_key,
                } => {
                    // Re-export: create getter that delegates to source module's property
                    let getter_obj = guard.alloc();
                    {
                        let mut getter_ref = getter_obj.borrow_mut();
                        getter_ref.prototype = Some(self.function_prototype.cheap_clone());
                        getter_ref.exotic =
                            ExoticObject::Function(JsFunction::ModuleReExportGetter {
                                source_module,
                                source_key,
                            });
                    }

                    // Set as accessor property (getter only, no setter)
                    module_obj.borrow_mut().properties.insert(
                        PropertyKey::String(export_name),
                        Property::accessor(Some(getter_obj), None),
                    );
                }
            }
        }

        // Restore saved exports
        self.exports = saved_exports;

        Ok(module_obj)
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
        let class_guard = self.heap.create_guard();
        let constructor_fn = self.create_class_constructor(&class_guard, class)?;

        // Apply class decorators if any
        let final_class = if !class.decorators.is_empty() {
            // Evaluate decorators (top-to-bottom)
            let evaluated_decorators = self.evaluate_decorators(&class.decorators)?;

            // Apply decorators (bottom-to-top) and get the final class value
            self.apply_class_decorators(
                JsValue::Object(constructor_fn.cheap_clone()),
                evaluated_decorators,
                class.id.as_ref().map(|id| id.name.cheap_clone()),
                &class_guard,
            )?
        } else {
            JsValue::Object(constructor_fn.cheap_clone())
        };

        // Bind the class name (potentially the decorated/replaced class)
        // Once bound to environment, the class becomes reachable and protected
        if let Some(id) = &class.id {
            self.env_define(id.name.cheap_clone(), final_class, false);
        }

        // Execute static blocks - they can reference the class name
        // Use stack-based execution for static blocks
        for member in &class.body.members {
            if let ClassMember::StaticBlock(block) = member {
                // Execute each statement in the static block
                for stmt in block.body.iter() {
                    self.execute_simple_statement(stmt)?;
                }
            }
        }

        Ok(())
    }

    /// Execute an enum declaration - creates an enum exotic object with name->value mappings
    /// and reverse mappings for numeric values stored directly in EnumData
    pub fn execute_enum_declaration(
        &mut self,
        enum_decl: &crate::ast::EnumDeclaration,
    ) -> Result<(), JsError> {
        use crate::value::EnumMember;

        // Create an object for the enum with object prototype
        let guard = self.heap.create_guard();
        let enum_obj = self.create_object(&guard);

        // Root and define the enum object first so member initializers can reference it
        // (e.g., ReadWrite = Read | Write references FileAccess.Read)
        // FIXME: guard in proper scope of namespace/module
        self.root_guard.guard(enum_obj.cheap_clone());
        let enum_name = enum_decl.id.name.cheap_clone();
        self.env_define(
            enum_name.cheap_clone(),
            JsValue::Object(enum_obj.cheap_clone()),
            false,
        );

        // Track the current numeric value for auto-incrementing
        let mut current_value: f64 = 0.0;
        let mut members = Vec::with_capacity(enum_decl.members.len());

        for member in &enum_decl.members {
            // Evaluate initializer or use auto-incremented value
            let value = if let Some(init_expr) = &member.initializer {
                let guarded = self.evaluate_expression(init_expr)?;
                let val = guarded.value;
                // Update current_value for next auto-increment
                if let JsValue::Number(n) = &val {
                    current_value = n + 1.0;
                }
                val
            } else {
                let val = JsValue::Number(current_value);
                current_value += 1.0;
                val
            };

            // Add member to the list
            members.push(EnumMember {
                name: member.id.name.cheap_clone(),
                value: value.clone(),
            });

            // Also define the member name in the current scope so later members can reference it
            // (e.g., in `ReadWrite = Read | Write`, `Read` needs to be in scope)
            self.env_define(member.id.name.cheap_clone(), value, false);
        }

        // Set the exotic type to Enum with all members
        enum_obj.borrow_mut().exotic = ExoticObject::Enum(EnumData {
            name: enum_name,
            const_: enum_decl.const_,
            members,
        });

        Ok(())
    }

    /// Execute a namespace declaration - creates an object with exported members
    pub fn execute_namespace_declaration(
        &mut self,
        ns_decl: &crate::ast::NamespaceDeclaration,
    ) -> Result<(), JsError> {
        let ns_name = ns_decl.id.name.cheap_clone();

        // Check if namespace already exists (for merging)
        let ns_obj = if let Ok(existing) = self.env_get(&ns_name) {
            if let JsValue::Object(obj) = existing {
                obj
            } else {
                // Not an object, create new
                let guard = self.heap.create_guard();
                let obj = self.create_object(&guard);
                // FIXME: properly guard in module
                self.root_guard.guard(obj.cheap_clone());
                obj
            }
        } else {
            // Create new namespace object
            // FIXME: properly guard in module
            let guard = self.heap.create_guard();
            let obj = self.create_object(&guard);
            self.root_guard.guard(obj.cheap_clone());
            obj
        };

        // Define the namespace in the environment first (for self-references)
        self.env_define(
            ns_name.cheap_clone(),
            JsValue::Object(ns_obj.cheap_clone()),
            false,
        );

        // Save current exports and create fresh exports for namespace
        let saved_exports = std::mem::take(&mut self.exports);

        // Create a new scope for the namespace body with guard
        let saved_env = self.env.cheap_clone();
        let (new_env, ns_guard) =
            create_environment_unrooted(&self.heap, Some(self.env.cheap_clone()));
        self.env = new_env;
        self.push_env_guard(ns_guard);

        // Execute each statement in the namespace body
        for stmt in ns_decl.body.iter() {
            self.execute_namespace_statement(stmt, &ns_obj)?;
        }

        // Copy namespace exports to the namespace object
        // Drain to vec first to avoid borrow conflict
        let exports: Vec<_> = self.exports.drain().collect();
        for (name, module_export) in exports {
            // For namespaces, extract the value from ModuleExport
            let value = match module_export {
                ModuleExport::Direct { value, .. } => value,
                ModuleExport::ReExport {
                    source_module,
                    source_key,
                } => {
                    // Resolve the re-export value
                    self.resolve_module_property(&source_module, &source_key)
                        .unwrap_or(JsValue::Undefined)
                }
            };
            ns_obj
                .borrow_mut()
                .set_property(PropertyKey::String(name), value);
        }

        // Pop namespace guard and restore environment and exports
        self.pop_env_guard();
        self.env = saved_env;
        self.exports = saved_exports;

        Ok(())
    }

    /// Execute a statement within a namespace context
    fn execute_namespace_statement(
        &mut self,
        stmt: &Statement,
        _ns_obj: &Gc<JsObject>,
    ) -> Result<(), JsError> {
        match stmt {
            // Handle export declarations within namespace
            Statement::Export(export) => {
                if let Some(decl) = &export.declaration {
                    match decl.as_ref() {
                        Statement::FunctionDeclaration(func) => {
                            self.execute_function_declaration(func)?;
                            if let Some(id) = &func.id {
                                let value = self.env_get(&id.name)?;
                                self.exports.insert(
                                    id.name.cheap_clone(),
                                    ModuleExport::Direct {
                                        name: id.name.cheap_clone(),
                                        value,
                                    },
                                );
                            }
                        }
                        Statement::VariableDeclaration(var_decl) => {
                            self.execute_variable_declaration(var_decl)?;
                            for declarator in var_decl.declarations.iter() {
                                if let Pattern::Identifier(id) = &declarator.id {
                                    let value = self.env_get(&id.name)?;
                                    self.exports.insert(
                                        id.name.cheap_clone(),
                                        ModuleExport::Direct {
                                            name: id.name.cheap_clone(),
                                            value,
                                        },
                                    );
                                }
                            }
                        }
                        Statement::ClassDeclaration(class) => {
                            self.execute_class_declaration(class)?;
                            if let Some(id) = &class.id {
                                let value = self.env_get(&id.name)?;
                                self.exports.insert(
                                    id.name.cheap_clone(),
                                    ModuleExport::Direct {
                                        name: id.name.cheap_clone(),
                                        value,
                                    },
                                );
                            }
                        }
                        Statement::EnumDeclaration(enum_decl) => {
                            self.execute_enum_declaration(enum_decl)?;
                            let value = self.env_get(&enum_decl.id.name)?;
                            self.exports.insert(
                                enum_decl.id.name.cheap_clone(),
                                ModuleExport::Direct {
                                    name: enum_decl.id.name.cheap_clone(),
                                    value,
                                },
                            );
                        }
                        Statement::NamespaceDeclaration(nested_ns) => {
                            // Handle nested namespace
                            self.execute_namespace_declaration(nested_ns)?;
                            let value = self.env_get(&nested_ns.id.name)?;
                            self.exports.insert(
                                nested_ns.id.name.cheap_clone(),
                                ModuleExport::Direct {
                                    name: nested_ns.id.name.cheap_clone(),
                                    value,
                                },
                            );
                        }
                        // TypeScript-only declarations (interfaces, type aliases) - no runtime effect
                        Statement::InterfaceDeclaration(_) | Statement::TypeAlias(_) => {}
                        _ => {
                            // Execute other statements normally
                            self.execute_simple_statement(decl)?;
                        }
                    }
                }
            }
            // Non-exported declarations stay private to the namespace
            Statement::FunctionDeclaration(func) => {
                self.execute_function_declaration(func)?;
            }
            Statement::VariableDeclaration(var_decl) => {
                self.execute_variable_declaration(var_decl)?;
            }
            Statement::ClassDeclaration(class) => {
                self.execute_class_declaration(class)?;
            }
            Statement::EnumDeclaration(enum_decl) => {
                self.execute_enum_declaration(enum_decl)?;
            }
            Statement::NamespaceDeclaration(nested_ns) => {
                self.execute_namespace_declaration(nested_ns)?;
            }
            // TypeScript-only declarations - no runtime effect
            Statement::InterfaceDeclaration(_) | Statement::TypeAlias(_) => {}
            // Other statements
            _ => {
                self.execute_simple_statement(stmt)?;
            }
        }
        Ok(())
    }

    /// Execute a function declaration (used within namespace context)
    fn execute_function_declaration(
        &mut self,
        func: &crate::ast::FunctionDeclaration,
    ) -> Result<(), JsError> {
        let name = func.id.as_ref().map(|id| id.name.cheap_clone());
        let params = func.params.cheap_clone();
        let body = Rc::new(FunctionBody::Block(func.body.cheap_clone()));

        // Create function with guard
        let guard = self.heap.create_guard();
        let func_obj = self.create_interpreted_function(
            &guard,
            name.cheap_clone(),
            params,
            body,
            self.env.cheap_clone(),
            func.span,
            func.generator,
            func.async_,
        );

        // Create prototype property with constructor back-reference
        let prototype = self.create_object(&guard);
        let constructor_key = PropertyKey::String(self.intern("constructor"));
        prototype
            .borrow_mut()
            .set_property(constructor_key, JsValue::Object(func_obj.clone()));
        let proto_key = PropertyKey::String(self.intern("prototype"));
        func_obj
            .borrow_mut()
            .set_property(proto_key, JsValue::Object(prototype));

        // Transfer ownership to environment before guard is dropped
        if let Some(js_name) = name {
            self.env_define(js_name, JsValue::Object(func_obj), false);
        }

        Ok(())
    }

    fn create_class_constructor(
        &mut self,
        guard: &Guard<JsObject>,
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

        // Create prototype object using the passed guard
        let prototype = self.create_object(guard);

        // If we have a superclass, set up prototype chain
        if let Some(ref super_ctor) = super_constructor {
            let super_proto = super_ctor
                .borrow()
                .get_property(&PropertyKey::String(self.intern("prototype")));
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
        let mut instance_accessors_props: Vec<&ClassProperty> = Vec::new();
        let mut static_accessors_props: Vec<&ClassProperty> = Vec::new();

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
                    if prop.accessor {
                        // Auto-accessor properties are treated differently
                        if prop.static_ {
                            static_accessors_props.push(prop);
                        } else {
                            instance_accessors_props.push(prop);
                        }
                    } else if prop.static_ {
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
        // Use PropertyKey to properly handle computed keys (including numeric)
        #[allow(clippy::type_complexity)]
        let mut accessors: FxHashMap<
            PropertyKey,
            (Option<Gc<JsObject>>, Option<Gc<JsObject>>),
        > = FxHashMap::default();
        let mut regular_methods: Vec<(PropertyKey, Gc<JsObject>)> = Vec::new();
        // Collect all method guards at outer scope to keep decorated methods alive
        // until they are stored on prototype
        let mut all_method_guards: Vec<Guard<JsObject>> = Vec::new();

        for method in &instance_methods {
            let (method_key, method_name, is_private): (PropertyKey, JsString, bool) =
                match &method.key {
                    ObjectPropertyKey::Identifier(id) => (
                        PropertyKey::String(id.name.cheap_clone()),
                        id.name.cheap_clone(),
                        false,
                    ),
                    ObjectPropertyKey::String(s) => (
                        PropertyKey::String(s.value.cheap_clone()),
                        s.value.cheap_clone(),
                        false,
                    ),
                    ObjectPropertyKey::Number(lit) => match &lit.value {
                        LiteralValue::Number(n) => {
                            let key = PropertyKey::from_value(&JsValue::Number(*n));
                            let name = JsString::from(number_to_string(*n));
                            (key, name, false)
                        }
                        _ => continue,
                    },
                    ObjectPropertyKey::Computed(expr) => {
                        let Guarded {
                            value: key_val,
                            guard: _key_guard,
                        } = self.evaluate_expression(expr)?;
                        let key = PropertyKey::from_value(&key_val);
                        let name = key_val.to_js_string();
                        (key, name, false)
                    }
                    ObjectPropertyKey::PrivateIdentifier(id) => (
                        PropertyKey::String(id.name.cheap_clone()),
                        id.name.cheap_clone(),
                        true,
                    ),
                };

            let func = &method.value;
            let mut func_obj = self.create_interpreted_function(
                guard,
                Some(method_name.cheap_clone()),
                func.params.clone(), // Rc clone is cheap
                Rc::new(FunctionBody::Block(func.body.cheap_clone())),
                self.env.cheap_clone(),
                func.span,
                func.generator,
                func.async_,
            );

            // Store __super__ on method so super.method() works
            if let Some(ref super_ctor) = super_constructor {
                func_obj.borrow_mut().set_property(
                    PropertyKey::String(self.intern("__super__")),
                    JsValue::Object(super_ctor.cheap_clone()),
                );
            }

            // Apply parameter decorators if any (before method decorators)
            // TC39-style: decorators receive (target, context) where context.kind = "parameter"
            self.apply_parameter_decorators(
                JsValue::Object(prototype.cheap_clone()),
                method_name.cheap_clone(),
                &func.params,
                false, // is_static
                guard,
            )?;

            // Apply method decorators if any (in reverse order - bottom to top)
            // Push guards to all_method_guards to keep wrapped functions alive until stored on prototype
            if !method.decorators.is_empty() {
                let evaluated_decorators = self.evaluate_decorators(&method.decorators)?;
                let kind = match method.kind {
                    MethodKind::Get => "getter",
                    MethodKind::Set => "setter",
                    MethodKind::Method => "method",
                };
                for (decorator, _dec_guard) in evaluated_decorators.into_iter().rev() {
                    let (new_func, new_guard) = self.apply_method_decorator(
                        func_obj,
                        decorator,
                        method_name.cheap_clone(),
                        false, // is_static
                        is_private,
                        kind,
                        guard,
                    )?;
                    func_obj = new_func;
                    if let Some(g) = new_guard {
                        all_method_guards.push(g);
                    }
                }
            }

            match method.kind {
                MethodKind::Get => {
                    let entry = accessors.entry(method_key).or_insert((None, None));
                    entry.0 = Some(func_obj);
                }
                MethodKind::Set => {
                    let entry = accessors.entry(method_key).or_insert((None, None));
                    entry.1 = Some(func_obj);
                }
                MethodKind::Method => {
                    regular_methods.push((method_key, func_obj));
                }
            }
        }

        // Add accessor properties to prototype
        for (key, (getter, setter)) in accessors {
            prototype
                .borrow_mut()
                .define_property(key, Property::accessor(getter, setter));
        }

        // Process instance auto-accessor properties
        for prop in &instance_accessors_props {
            let name: JsString = match &prop.key {
                ObjectPropertyKey::Identifier(id) => id.name.cheap_clone(),
                ObjectPropertyKey::String(s) => s.value.cheap_clone(),
                _ => continue,
            };

            // Create the auto-accessor (returns getter/setter pair, possibly decorated)
            let (getter_obj, setter_obj) = self.create_auto_accessor(
                guard,
                name.cheap_clone(),
                prop.value.as_ref(),
                &prop.decorators,
                false, // is_static
            )?;

            // Add as accessor property on prototype
            prototype.borrow_mut().define_property(
                PropertyKey::String(name),
                Property::accessor(Some(getter_obj), Some(setter_obj)),
            );
        }

        // Add regular methods to prototype
        for (key, func_obj) in regular_methods {
            prototype
                .borrow_mut()
                .set_property(key, JsValue::Object(func_obj));
        }
        // Now that methods are stored on prototype, guards can be dropped
        let _ = all_method_guards;

        // Build constructor body that initializes instance fields then runs user constructor
        // Include decorators for field transformation
        let field_initializers: Vec<(JsString, Option<Expression>, Vec<Decorator>, bool)> =
            instance_fields
                .iter()
                .filter_map(|prop| {
                    let (name, is_private): (JsString, bool) = match &prop.key {
                        ObjectPropertyKey::Identifier(id) => (id.name.cheap_clone(), false),
                        ObjectPropertyKey::String(s) => (s.value.cheap_clone(), false),
                        ObjectPropertyKey::PrivateIdentifier(id) => (id.name.cheap_clone(), true),
                        _ => return None,
                    };
                    Some((
                        name,
                        prop.value.clone(),
                        prop.decorators.clone(),
                        is_private,
                    ))
                })
                .collect();

        // Create the constructor function
        let ctor_body = if let Some(ctor) = constructor {
            ctor.body.clone()
        } else {
            BlockStatement {
                body: Rc::from([]),
                span: class.span,
            }
        };

        let ctor_params = if let Some(ctor) = constructor {
            ctor.params.clone()
        } else {
            vec![]
        };

        let constructor_fn = self.create_interpreted_function(
            guard,
            class.id.as_ref().map(|id| id.name.cheap_clone()),
            Rc::from(ctor_params.clone()),
            Rc::new(FunctionBody::Block(Rc::new(ctor_body))),
            self.env.clone(),
            class.span,
            false,
            false,
        );

        // Apply constructor parameter decorators if any
        // TC39-style: decorators receive (target, context) where context.function = "constructor"
        let ctor_key = self.intern("constructor");
        self.apply_parameter_decorators(
            JsValue::Object(constructor_fn.cheap_clone()),
            ctor_key,
            &ctor_params,
            false, // constructors are not static
            guard,
        )?;

        // Store prototype on constructor
        constructor_fn.borrow_mut().set_property(
            PropertyKey::String(self.intern("prototype")),
            JsValue::Object(prototype.cheap_clone()),
        );

        // Store field initializers in __fields__ if there are any
        if !field_initializers.is_empty() {
            let mut field_values: Vec<(JsString, JsValue)> = Vec::new();
            for (name, value_expr, decorators, is_private) in field_initializers {
                // First evaluate the initial value
                let mut value = if let Some(expr) = value_expr {
                    self.evaluate_expression(&expr)
                        .map(|g| g.value)
                        .unwrap_or(JsValue::Undefined)
                } else {
                    JsValue::Undefined
                };

                // Apply field decorators if any
                if !decorators.is_empty() {
                    let evaluated_decorators = self.evaluate_decorators(&decorators)?;
                    let mut initializers: Vec<Guarded> = Vec::new();

                    // Evaluate decorators and collect initializer functions (in reverse order)
                    for (decorator, _dec_guard) in evaluated_decorators.into_iter().rev() {
                        if let Some(initializer) = self.apply_field_decorator(
                            decorator,
                            name.cheap_clone(),
                            false, // is_static
                            is_private,
                            guard,
                        )? {
                            initializers.push(initializer);
                        }
                    }

                    // Transform the initial value using all initializers
                    if !initializers.is_empty() {
                        value = self.transform_field_value(value, &initializers)?;
                    }
                }

                field_values.push((name, value));
            }

            // Create the fields array - fields are stored on constructor
            // so they'll be protected once set as a property
            let mut field_pairs: Vec<JsValue> = Vec::new();
            for (name, value) in field_values {
                let pair = self.create_array_from(guard, vec![JsValue::String(name), value]);
                field_pairs.push(JsValue::Object(pair));
            }

            let fields_array = self.create_array_from(guard, field_pairs);

            let fields_key = PropertyKey::String(self.intern("__fields__"));
            constructor_fn
                .borrow_mut()
                .set_property(fields_key, JsValue::Object(fields_array));
        }

        // Store super constructor if we have one
        if let Some(ref super_ctor) = super_constructor {
            constructor_fn.borrow_mut().set_property(
                PropertyKey::String(self.intern("__super__")),
                JsValue::Object(super_ctor.cheap_clone()),
            );
        }

        // Handle static methods
        // Use PropertyKey to properly handle computed keys (including numeric)
        #[allow(clippy::type_complexity)]
        let mut static_accessors: FxHashMap<
            PropertyKey,
            (Option<Gc<JsObject>>, Option<Gc<JsObject>>),
        > = FxHashMap::default();
        let mut static_regular_methods: Vec<(PropertyKey, Gc<JsObject>)> = Vec::new();
        // Collect all static method guards at outer scope to keep decorated methods alive
        let mut all_static_method_guards: Vec<Guard<JsObject>> = Vec::new();

        for method in &static_methods {
            let (method_key, method_name, is_private): (PropertyKey, JsString, bool) =
                match &method.key {
                    ObjectPropertyKey::Identifier(id) => (
                        PropertyKey::String(id.name.cheap_clone()),
                        id.name.cheap_clone(),
                        false,
                    ),
                    ObjectPropertyKey::String(s) => (
                        PropertyKey::String(s.value.cheap_clone()),
                        s.value.cheap_clone(),
                        false,
                    ),
                    ObjectPropertyKey::Number(lit) => match &lit.value {
                        LiteralValue::Number(n) => {
                            let key = PropertyKey::from_value(&JsValue::Number(*n));
                            let name = JsString::from(number_to_string(*n));
                            (key, name, false)
                        }
                        _ => continue,
                    },
                    ObjectPropertyKey::Computed(expr) => {
                        let Guarded {
                            value: key_val,
                            guard: _key_guard,
                        } = self.evaluate_expression(expr)?;
                        let key = PropertyKey::from_value(&key_val);
                        let name = key_val.to_js_string();
                        (key, name, false)
                    }
                    ObjectPropertyKey::PrivateIdentifier(id) => (
                        PropertyKey::String(id.name.cheap_clone()),
                        id.name.cheap_clone(),
                        true,
                    ),
                };

            let func = &method.value;
            let mut func_obj = self.create_interpreted_function(
                guard,
                Some(method_name.cheap_clone()),
                func.params.cheap_clone(),
                // FIXME: no need to wrap FunctionBody to rc
                Rc::new(FunctionBody::Block(func.body.cheap_clone())),
                self.env.clone(),
                func.span,
                func.generator,
                func.async_,
            );

            // Apply parameter decorators if any (before method decorators)
            // TC39-style: decorators receive (target, context) where context.static = true
            self.apply_parameter_decorators(
                JsValue::Object(constructor_fn.cheap_clone()),
                method_name.cheap_clone(),
                &func.params,
                true, // is_static
                guard,
            )?;

            // Apply method decorators if any (in reverse order - bottom to top)
            // Push guards to all_static_method_guards to keep wrapped functions alive
            if !method.decorators.is_empty() {
                let evaluated_decorators = self.evaluate_decorators(&method.decorators)?;
                let kind = match method.kind {
                    MethodKind::Get => "getter",
                    MethodKind::Set => "setter",
                    MethodKind::Method => "method",
                };
                for (decorator, _dec_guard) in evaluated_decorators.into_iter().rev() {
                    let (new_func, new_guard) = self.apply_method_decorator(
                        func_obj,
                        decorator,
                        method_name.cheap_clone(),
                        true, // is_static
                        is_private,
                        kind,
                        guard,
                    )?;
                    func_obj = new_func;
                    if let Some(g) = new_guard {
                        all_static_method_guards.push(g);
                    }
                }
            }

            match method.kind {
                MethodKind::Get => {
                    let entry = static_accessors.entry(method_key).or_insert((None, None));
                    entry.0 = Some(func_obj);
                }
                MethodKind::Set => {
                    let entry = static_accessors.entry(method_key).or_insert((None, None));
                    entry.1 = Some(func_obj);
                }
                MethodKind::Method => {
                    static_regular_methods.push((method_key, func_obj));
                }
            }
        }

        // Add static accessor properties
        for (key, (getter, setter)) in static_accessors {
            constructor_fn
                .borrow_mut()
                .define_property(key, Property::accessor(getter, setter));
        }

        // Process static auto-accessor properties
        for prop in &static_accessors_props {
            let name: JsString = match &prop.key {
                ObjectPropertyKey::Identifier(id) => id.name.cheap_clone(),
                ObjectPropertyKey::String(s) => s.value.cheap_clone(),
                _ => continue,
            };

            // Create the auto-accessor (returns getter/setter pair, possibly decorated)
            let (getter_obj, setter_obj) = self.create_auto_accessor(
                guard,
                name.cheap_clone(),
                prop.value.as_ref(),
                &prop.decorators,
                true, // is_static
            )?;

            // Add as accessor property on constructor (not prototype)
            constructor_fn.borrow_mut().define_property(
                PropertyKey::String(name),
                Property::accessor(Some(getter_obj), Some(setter_obj)),
            );
        }

        // Add static regular methods
        for (key, func_obj) in static_regular_methods {
            constructor_fn
                .borrow_mut()
                .set_property(key, JsValue::Object(func_obj));
        }
        // Now that static methods are stored, guards can be dropped
        let _ = all_static_method_guards;

        // Initialize static fields
        for prop in &static_fields {
            let (name, is_private): (JsString, bool) = match &prop.key {
                ObjectPropertyKey::Identifier(id) => (id.name.cheap_clone(), false),
                ObjectPropertyKey::String(s) => (s.value.cheap_clone(), false),
                ObjectPropertyKey::PrivateIdentifier(id) => (id.name.cheap_clone(), true),
                _ => continue,
            };

            let (mut value, _value_guard) = if let Some(expr) = &prop.value {
                let Guarded { value: v, guard: g } = self.evaluate_expression(expr)?;
                (v, g)
            } else {
                (JsValue::Undefined, None)
            };

            // Apply field decorators if any
            if !prop.decorators.is_empty() {
                let evaluated_decorators = self.evaluate_decorators(&prop.decorators)?;
                let mut initializers: Vec<Guarded> = Vec::new();

                // Evaluate decorators and collect initializer functions (in reverse order)
                for (decorator, _dec_guard) in evaluated_decorators.into_iter().rev() {
                    if let Some(initializer) = self.apply_field_decorator(
                        decorator,
                        name.cheap_clone(),
                        true, // is_static
                        is_private,
                        guard,
                    )? {
                        initializers.push(initializer);
                    }
                }

                // Transform the initial value using all initializers
                if !initializers.is_empty() {
                    value = self.transform_field_value(value, &initializers)?;
                }
            }

            constructor_fn
                .borrow_mut()
                .set_property(PropertyKey::String(name), value);
        }

        // Set prototype.constructor = constructor
        prototype.borrow_mut().set_property(
            PropertyKey::String(self.intern("constructor")),
            JsValue::Object(constructor_fn.cheap_clone()),
        );

        Ok(constructor_fn)
    }

    fn create_class_from_expression(
        &mut self,
        guard: &Guard<JsObject>,
        class_expr: &ClassExpression,
    ) -> Result<Gc<JsObject>, JsError> {
        // Convert ClassExpression to ClassDeclaration
        // FIXME: clones
        let class_decl = ClassDeclaration {
            id: class_expr.id.clone(),
            type_parameters: class_expr.type_parameters.clone(),
            super_class: class_expr.super_class.cheap_clone(),
            implements: class_expr.implements.clone(),
            body: class_expr.body.clone(),
            decorators: class_expr.decorators.clone(),
            abstract_: false,
            span: class_expr.span,
        };
        self.create_class_constructor(guard, &class_decl)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Decorator Evaluation
    // ═══════════════════════════════════════════════════════════════════════════

    /// Create a decorator context object with the given properties
    fn create_decorator_context(
        &mut self,
        guard: &Guard<JsObject>,
        kind: &str,
        name: Option<JsString>,
        is_static: bool,
        is_private: bool,
    ) -> Gc<JsObject> {
        self.create_decorator_context_with_initializers(
            guard, kind, name, is_static, is_private, None,
        )
    }

    /// Create a decorator context object with addInitializer support
    fn create_decorator_context_with_initializers(
        &mut self,
        guard: &Guard<JsObject>,
        kind: &str,
        name: Option<JsString>,
        is_static: bool,
        is_private: bool,
        initializers: Option<Gc<JsObject>>,
    ) -> Gc<JsObject> {
        let ctx = self.create_object(guard);
        ctx.borrow_mut().set_property(
            PropertyKey::String(self.intern("kind")),
            JsValue::String(self.intern(kind)),
        );
        if let Some(n) = name {
            ctx.borrow_mut()
                .set_property(PropertyKey::String(self.intern("name")), JsValue::String(n));
        }
        ctx.borrow_mut().set_property(
            PropertyKey::String(self.intern("static")),
            JsValue::Boolean(is_static),
        );
        ctx.borrow_mut().set_property(
            PropertyKey::String(self.intern("private")),
            JsValue::Boolean(is_private),
        );

        // Add addInitializer method if initializers array is provided
        if let Some(init_array) = initializers {
            // Create addInitializer function that captures the initializers array
            let add_init_fn = self.create_add_initializer_function(guard, init_array);
            ctx.borrow_mut().set_property(
                PropertyKey::String(self.intern("addInitializer")),
                JsValue::Object(add_init_fn),
            );
        }

        ctx
    }

    /// Create the addInitializer function that pushes to the initializers array
    fn create_add_initializer_function(
        &mut self,
        guard: &Guard<JsObject>,
        initializers: Gc<JsObject>,
    ) -> Gc<JsObject> {
        // Create a native function that captures the initializers array
        let func = self.create_native_fn(guard, "addInitializer", add_initializer_impl, 1);

        // Store the initializers array in the function's __initializers__ property
        func.borrow_mut().set_property(
            PropertyKey::String(self.intern("__initializers__")),
            JsValue::Object(initializers),
        );

        func
    }

    /// Evaluate all decorators in order (top-to-bottom for evaluation, bottom-to-top for application)
    /// Returns a vector of (decorator_function, evaluated) pairs
    fn evaluate_decorators(
        &mut self,
        decorators: &[Decorator],
    ) -> Result<Vec<(JsValue, Guard<JsObject>)>, JsError> {
        let mut results = Vec::with_capacity(decorators.len());
        for decorator in decorators {
            let Guarded { value, guard } = self.evaluate_expression(&decorator.expression)?;
            if let Some(g) = guard {
                results.push((value, g));
            } else {
                // Create a dummy guard for values that don't have one
                let dummy = self.heap.create_guard();
                results.push((value, dummy));
            }
        }
        Ok(results)
    }

    /// Apply class decorators to a class constructor
    /// Decorators are applied in reverse order (bottom-to-top)
    /// Initializers registered via context.addInitializer() are run after all decorators
    fn apply_class_decorators(
        &mut self,
        mut class_value: JsValue,
        decorators: Vec<(JsValue, Guard<JsObject>)>,
        class_name: Option<JsString>,
        guard: &Guard<JsObject>,
    ) -> Result<JsValue, JsError> {
        // Create an array to collect initializers from all decorators
        let initializers = self.create_empty_array(guard);

        // Apply decorators in reverse order (bottom-to-top)
        for (decorator, _dec_guard) in decorators.into_iter().rev() {
            // Create context for class decorator with addInitializer support
            let ctx = self.create_decorator_context_with_initializers(
                guard,
                "class",
                class_name.cheap_clone(),
                false,
                false,
                Some(initializers.cheap_clone()),
            );

            // Call the decorator with (class, context)
            let result = self.call_function(
                decorator.clone(),
                JsValue::Undefined,
                &[class_value.clone(), JsValue::Object(ctx)],
            )?;

            // If decorator returns undefined, keep original class
            // Otherwise use the returned value
            if !matches!(result.value, JsValue::Undefined) {
                class_value = result.value;
            }
        }

        // Run all registered initializers
        self.run_decorator_initializers(&initializers)?;

        Ok(class_value)
    }

    /// Run initializers registered via context.addInitializer()
    fn run_decorator_initializers(&mut self, initializers: &Gc<JsObject>) -> Result<(), JsError> {
        let length = {
            let arr_ref = initializers.borrow();
            arr_ref.array_length().unwrap_or(0)
        };

        for i in 0..length {
            let init_fn = {
                let arr_ref = initializers.borrow();
                arr_ref
                    .get_property(&PropertyKey::Index(i))
                    .unwrap_or(JsValue::Undefined)
            };

            if matches!(&init_fn, JsValue::Object(obj) if obj.borrow().is_callable()) {
                // Call initializer with undefined as this
                self.call_function(init_fn, JsValue::Undefined, &[])?;
            }
        }

        Ok(())
    }

    /// Apply method decorator and return the (possibly wrapped) method
    #[allow(clippy::too_many_arguments)]
    fn apply_method_decorator(
        &mut self,
        method_fn: Gc<JsObject>,
        decorator: JsValue,
        name: JsString,
        is_static: bool,
        is_private: bool,
        kind: &str,
        guard: &Guard<JsObject>,
    ) -> Result<(Gc<JsObject>, Option<Guard<JsObject>>), JsError> {
        // Create context object
        let ctx = self.create_decorator_context(guard, kind, Some(name), is_static, is_private);

        // Call the decorator with (method, context)
        let result = self.call_function(
            decorator,
            JsValue::Undefined,
            &[
                JsValue::Object(method_fn.cheap_clone()),
                JsValue::Object(ctx),
            ],
        )?;

        // If decorator returns undefined, keep original method
        // Otherwise use the returned function (with its guard to keep closure alive)
        match result.value {
            JsValue::Undefined => Ok((method_fn, None)),
            JsValue::Object(new_fn) => Ok((new_fn, result.guard)),
            _ => Err(JsError::type_error(
                "Method decorator must return a function or undefined",
            )),
        }
    }

    /// Apply field decorator and return the initializer transformer
    /// Field decorators return a function that transforms the initial value
    fn apply_field_decorator(
        &mut self,
        decorator: JsValue,
        name: JsString,
        is_static: bool,
        is_private: bool,
        guard: &Guard<JsObject>,
    ) -> Result<Option<Guarded>, JsError> {
        // Create context object
        let ctx = self.create_decorator_context(guard, "field", Some(name), is_static, is_private);

        // Call the decorator with (undefined, context) for fields
        // Field decorators receive undefined as first arg and return an initializer
        let result = self.call_function(
            decorator,
            JsValue::Undefined,
            &[JsValue::Undefined, JsValue::Object(ctx)],
        )?;

        // If decorator returns undefined, no transformation
        // Otherwise it should return an initializer function (with its guard to keep closure alive)
        match result.value {
            JsValue::Undefined => Ok(None),
            JsValue::Object(_) => Ok(Some(result)),
            _ => Ok(None),
        }
    }

    /// Transform a field's initial value using decorator initializers
    fn transform_field_value(
        &mut self,
        initial_value: JsValue,
        initializers: &[Guarded],
    ) -> Result<JsValue, JsError> {
        let mut value = initial_value;
        for initializer in initializers {
            let result =
                self.call_function(initializer.value.clone(), JsValue::Undefined, &[value])?;
            value = result.value;
        }
        Ok(value)
    }

    /// Create an auto-accessor property (TC39 Stage 3)
    /// Returns a (getter, setter) pair
    #[allow(clippy::too_many_arguments)]
    fn create_auto_accessor(
        &mut self,
        guard: &Guard<JsObject>,
        name: JsString,
        initial_value: Option<&Expression>,
        decorators: &[Decorator],
        is_static: bool,
    ) -> Result<(Gc<JsObject>, Gc<JsObject>), JsError> {
        // Create a unique storage key for this accessor
        let storage_key = self.intern(&format!("__accessor_{}__", name.as_str()));

        // Evaluate initial value if any
        let init_value = if let Some(expr) = initial_value {
            self.evaluate_expression(expr)?.value
        } else {
            JsValue::Undefined
        };

        // Create getter function
        let getter =
            self.create_accessor_getter(guard, storage_key.cheap_clone(), init_value.clone());

        // Create setter function
        let setter = self.create_accessor_setter(guard, storage_key);

        // If no decorators, return raw getter/setter
        if decorators.is_empty() {
            return Ok((getter, setter));
        }

        // Create target object with get/set methods for decorator
        let target = self.create_object(guard);
        target.borrow_mut().set_property(
            PropertyKey::String(self.intern("get")),
            JsValue::Object(getter.cheap_clone()),
        );
        target.borrow_mut().set_property(
            PropertyKey::String(self.intern("set")),
            JsValue::Object(setter.cheap_clone()),
        );

        // Apply decorators (bottom-to-top)
        let evaluated = self.evaluate_decorators(decorators)?;
        let mut current_target = JsValue::Object(target);

        for (decorator, _dec_guard) in evaluated.into_iter().rev() {
            // Create context for accessor decorator
            let ctx = self.create_decorator_context(
                guard,
                "accessor",
                Some(name.cheap_clone()),
                is_static,
                false,
            );

            // Call decorator with (target, context)
            let result = self.call_function(
                decorator,
                JsValue::Undefined,
                &[current_target.clone(), JsValue::Object(ctx)],
            )?;

            // If decorator returns an object, use it as new target
            if let JsValue::Object(_) = &result.value {
                current_target = result.value;
            }
        }

        // Extract getter/setter from result (or use original if unchanged)
        if let JsValue::Object(result_obj) = current_target {
            let result_ref = result_obj.borrow();
            let get_key = self.intern("get");
            let set_key = self.intern("set");

            let final_getter = if let Some(JsValue::Object(g)) =
                result_ref.get_property(&PropertyKey::String(get_key))
            {
                g.cheap_clone()
            } else {
                getter
            };

            let final_setter = if let Some(JsValue::Object(s)) =
                result_ref.get_property(&PropertyKey::String(set_key))
            {
                s.cheap_clone()
            } else {
                setter
            };

            Ok((final_getter, final_setter))
        } else {
            Ok((getter, setter))
        }
    }

    /// Create a getter function for an auto-accessor
    /// Uses a closure that captures the storage key and initial value
    fn create_accessor_getter(
        &mut self,
        guard: &Guard<JsObject>,
        storage_key: JsString,
        init_value: JsValue,
    ) -> Gc<JsObject> {
        // Create a function object with accessor metadata
        let func = self.create_object(guard);
        func.borrow_mut().prototype = Some(self.function_prototype.cheap_clone());

        // Store accessor metadata
        func.borrow_mut().set_property(
            PropertyKey::String(self.intern("__accessor_storage_key__")),
            JsValue::String(storage_key),
        );
        func.borrow_mut().set_property(
            PropertyKey::String(self.intern("__accessor_init_value__")),
            init_value,
        );
        func.borrow_mut().set_property(
            PropertyKey::String(self.intern("__accessor_kind__")),
            JsValue::String(self.intern("getter")),
        );

        // Mark as callable by adding a special function marker
        func.borrow_mut().exotic = ExoticObject::Function(JsFunction::AccessorGetter);

        func
    }

    /// Create a setter function for an auto-accessor
    /// Uses a closure that captures the storage key
    fn create_accessor_setter(
        &mut self,
        guard: &Guard<JsObject>,
        storage_key: JsString,
    ) -> Gc<JsObject> {
        // Create a function object with accessor metadata
        let func = self.create_object(guard);
        func.borrow_mut().prototype = Some(self.function_prototype.cheap_clone());

        // Store accessor metadata
        func.borrow_mut().set_property(
            PropertyKey::String(self.intern("__accessor_storage_key__")),
            JsValue::String(storage_key),
        );
        func.borrow_mut().set_property(
            PropertyKey::String(self.intern("__accessor_kind__")),
            JsValue::String(self.intern("setter")),
        );

        // Mark as callable by adding a special function marker
        func.borrow_mut().exotic = ExoticObject::Function(JsFunction::AccessorSetter);

        func
    }

    /// Apply parameter decorators to method parameters
    /// TC39-style context object: { kind: "parameter", name, function, index, ... }
    fn apply_parameter_decorators(
        &mut self,
        target: JsValue,
        property_key: JsString,
        params: &[FunctionParam],
        is_static: bool,
        guard: &Guard<JsObject>,
    ) -> Result<(), JsError> {
        for (index, param) in params.iter().enumerate() {
            if param.decorators.is_empty() {
                continue;
            }

            // Get parameter name if it's a simple identifier
            let param_name = match &param.pattern {
                Pattern::Identifier(id) => Some(id.name.cheap_clone()),
                _ => None,
            };

            // Evaluate and apply decorators for this parameter
            let evaluated = self.evaluate_decorators(&param.decorators)?;
            for (decorator, _dec_guard) in evaluated.into_iter().rev() {
                // Create TC39-style context object
                let ctx = self.create_object(guard);
                ctx.borrow_mut().set_property(
                    PropertyKey::String(self.intern("kind")),
                    JsValue::String(self.intern("parameter")),
                );
                if let Some(ref name) = param_name {
                    ctx.borrow_mut().set_property(
                        PropertyKey::String(self.intern("name")),
                        JsValue::String(name.cheap_clone()),
                    );
                }
                ctx.borrow_mut().set_property(
                    PropertyKey::String(self.intern("function")),
                    JsValue::String(property_key.cheap_clone()),
                );
                ctx.borrow_mut().set_property(
                    PropertyKey::String(self.intern("index")),
                    JsValue::Number(index as f64),
                );
                ctx.borrow_mut().set_property(
                    PropertyKey::String(self.intern("static")),
                    JsValue::Boolean(is_static),
                );

                // Call decorator with (target, context)
                let _result = self.call_function(
                    decorator,
                    JsValue::Undefined,
                    &[target.clone(), JsValue::Object(ctx)],
                )?;
                // Parameter decorators are called for side effects only (like metadata registration)
            }
        }
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
                    let guard = self.heap.create_guard();
                    let regexp_obj = self.create_regexp_literal(&guard, pattern, flags);
                    return Ok(Guarded::with_guard(JsValue::Object(regexp_obj), guard));
                }
                Ok(Guarded::unguarded(self.evaluate_literal(&lit.value)?))
            }

            // FIXME:
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
                // Rc clone is cheap (just ref count increment)
                let params = arrow.params.clone();
                let body = Rc::new(FunctionBody::from(arrow.body.cheap_clone()));

                let guard = self.heap.create_guard();
                let func_obj = self.create_interpreted_function(
                    &guard,
                    None,
                    params,
                    body,
                    self.env.cheap_clone(),
                    arrow.span,
                    false,
                    arrow.async_,
                );

                Ok(Guarded::with_guard(JsValue::Object(func_obj), guard))
            }

            Expression::Function(func) => {
                let name = func.id.as_ref().map(|id| id.name.cheap_clone());
                let params = func.params.cheap_clone();
                let body = Rc::new(FunctionBody::Block(func.body.cheap_clone()));

                let guard = self.heap.create_guard();
                let func_obj = self.create_interpreted_function(
                    &guard,
                    name,
                    params,
                    body,
                    self.env.cheap_clone(),
                    func.span,
                    func.generator,
                    func.async_,
                );

                // Set up prototype property with constructor back-reference
                // (regular functions, not arrow functions, have this)
                let proto_obj = self.create_object(&guard);
                let constructor_key = PropertyKey::String(self.intern("constructor"));
                proto_obj
                    .borrow_mut()
                    .set_property(constructor_key, JsValue::Object(func_obj.clone()));
                let prototype_key = PropertyKey::String(self.intern("prototype"));
                func_obj
                    .borrow_mut()
                    .set_property(prototype_key, JsValue::Object(proto_obj));

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
                let this_name = self.intern("this");
                Ok(Guarded::unguarded(
                    self.env_get(&this_name).unwrap_or(JsValue::Undefined),
                ))
            }

            // Class expression
            Expression::Class(class_expr) => {
                let guard = self.heap.create_guard();
                let constructor_fn = self.create_class_from_expression(&guard, class_expr)?;

                // Apply class decorators if any
                let final_value = if !class_expr.decorators.is_empty() {
                    let evaluated_decorators = self.evaluate_decorators(&class_expr.decorators)?;
                    self.apply_class_decorators(
                        JsValue::Object(constructor_fn),
                        evaluated_decorators,
                        class_expr.id.as_ref().map(|id| id.name.cheap_clone()),
                        &guard,
                    )?
                } else {
                    JsValue::Object(constructor_fn)
                };

                Ok(Guarded::with_guard(final_value, guard))
            }

            // Await expression - extract value from promise
            Expression::Await(await_expr) => {
                let Guarded {
                    value: promise_value,
                    guard: promise_guard,
                } = self.evaluate_expression(&await_expr.argument)?;

                // If the value is a promise, extract its result
                if let JsValue::Object(obj) = &promise_value {
                    let obj_ref = obj.borrow();
                    if let ExoticObject::Promise(state) = &obj_ref.exotic {
                        let state_ref = state.borrow();
                        match state_ref.status {
                            PromiseStatus::Fulfilled => {
                                let result = state_ref.result.clone().unwrap_or(JsValue::Undefined);
                                drop(state_ref);
                                drop(obj_ref);
                                return Ok(Guarded {
                                    value: result,
                                    guard: promise_guard,
                                });
                            }
                            PromiseStatus::Rejected => {
                                let reason = state_ref.result.clone().unwrap_or(JsValue::Undefined);
                                drop(state_ref);
                                drop(obj_ref);
                                // Re-throw the rejection as an error
                                return Err(JsError::thrown(reason));
                            }
                            PromiseStatus::Pending => {
                                // For pending promises, we would need to suspend execution
                                // For now, return undefined (will be enhanced later)
                                drop(state_ref);
                                drop(obj_ref);
                                return Ok(Guarded {
                                    value: JsValue::Undefined,
                                    guard: promise_guard,
                                });
                            }
                        }
                    }
                }

                // Not a promise - just return the value (await on non-promise returns the value)
                Ok(Guarded {
                    value: promise_value,
                    guard: promise_guard,
                })
            }

            // Spread is only valid in array/object literals and function calls, not as standalone expression
            Expression::Spread(_) => Err(JsError::syntax_error_simple(
                "Spread element is only valid in array or object literals",
            )),

            // Yield expression - should be handled by stack-based execution (step_expr)
            // This path is only reached for non-generator contexts (which is an error)
            Expression::Yield(yield_expr) => {
                // Evaluate the argument if present
                let value = if let Some(arg) = &yield_expr.argument {
                    let Guarded { value, .. } = self.evaluate_expression(arg)?;
                    value
                } else {
                    JsValue::Undefined
                };

                // Yield the value - this will be caught by resume_generator
                // which will save the execution state
                Err(JsError::GeneratorYield { value })
            }

            // Super expression handled specially in member access and calls
            _ => Ok(Guarded::unguarded(JsValue::Undefined)),
        }
    }

    fn evaluate_new(&mut self, new_expr: &NewExpression) -> Result<Guarded, JsError> {
        // Check if this is `new eval(...)` - eval is not a constructor
        if let Expression::Identifier(id) = &*new_expr.callee {
            if id.name.as_str() == "eval" {
                // Check that 'eval' refers to the global eval function
                if let Ok(JsValue::Object(eval_obj)) = self.env_get(&id.name) {
                    let eval_name = self.intern("eval");
                    let is_global_eval = {
                        if let Ok(JsValue::Object(global_eval)) = self.env_get(&eval_name) {
                            std::ptr::eq(
                                &*eval_obj.borrow() as *const _,
                                &*global_eval.borrow() as *const _,
                            )
                        } else {
                            false
                        }
                    };

                    if is_global_eval {
                        return Err(JsError::type_error("eval is not a constructor"));
                    }
                }
            }
        }

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
                Argument::Spread(spread) => {
                    let Guarded { value, guard } = self.evaluate_expression(&spread.argument)?;
                    if let Some(g) = guard {
                        _arg_guards.push(g);
                    }
                    // Spread the array elements into arguments
                    if let JsValue::Object(obj) = value {
                        let obj_ref = obj.borrow();
                        if let Some(elements) = obj_ref.array_elements() {
                            args.extend(elements.iter().cloned());
                        }
                    }
                }
            }
        }

        // Check if this is a proxy - use construct trap
        if let JsValue::Object(ctor_obj) = &constructor {
            let is_proxy = matches!(ctor_obj.borrow().exotic, ExoticObject::Proxy(_));
            if is_proxy {
                return builtins::proxy::proxy_construct(
                    self,
                    ctor_obj.clone(),
                    args,
                    constructor.clone(),
                );
            }
        }

        // Create a new object
        let new_guard = self.heap.create_guard();
        let new_obj = self.create_object(&new_guard);

        // Get the constructor's prototype and __fields__ properties
        let (proto_opt, fields_opt) = if let JsValue::Object(ctor) = &constructor {
            let ctor_ref = ctor.borrow();
            let proto = ctor_ref
                .get_property(&PropertyKey::String(self.intern("prototype")))
                .and_then(|v| {
                    if let JsValue::Object(p) = v {
                        Some(p)
                    } else {
                        None
                    }
                });
            let fields = ctor_ref
                .get_property(&PropertyKey::String(self.intern("__fields__")))
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
                match fields_ref.get_property(&PropertyKey::String(self.intern("length"))) {
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
        let this = JsValue::Object(new_obj.cheap_clone());
        let result = self.call_function(constructor, this.clone(), &args)?;

        // If constructor returns an object, use that; otherwise use the created object
        match result.value {
            JsValue::Object(obj) => {
                // Check if the returned object is the same as `this` (created by us)
                // If so, use our new_guard. Otherwise, use the result's guard.
                let is_same_object =
                    std::ptr::eq(&*obj.borrow() as *const _, &*new_obj.borrow() as *const _);
                if is_same_object {
                    Ok(Guarded::with_guard(JsValue::Object(obj), new_guard))
                } else {
                    // A different object was returned - use result's guard if any,
                    // but also keep new_guard since the returned object might reference it
                    Ok(Guarded {
                        value: JsValue::Object(obj),
                        guard: result.guard.or(Some(new_guard)),
                    })
                }
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
                // Use coerce_to_string for proper ToPrimitive handling
                let str_val = self.coerce_to_string(&val)?;
                result.push_str(str_val.as_ref());
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
        // FIXME: clone strings
        let strings_guard = self.heap.create_guard();
        let strings_arr = self.create_array_from(&strings_guard, strings.clone());

        // Add 'raw' property to strings array
        let raw: Vec<JsValue> = tagged
            .quasi
            .quasis
            .iter()
            .map(|q| JsValue::String(q.value.cheap_clone()))
            .collect();
        let raw_array = self.create_array_from(&strings_guard, raw);

        // Set raw property and transfer ownership
        let raw_key = PropertyKey::String(self.intern("raw"));
        strings_arr
            .borrow_mut()
            .set_property(raw_key, JsValue::Object(raw_array));

        // Evaluate all interpolated expressions (remaining arguments)
        let mut args = vec![JsValue::Object(strings_arr)];
        let mut _arg_guards: Vec<Guard<JsObject>> = vec![strings_guard];
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

        let result = match bin.operator {
            // Arithmetic - need ToPrimitive for object operands
            BinaryOp::Add => {
                // First convert objects to primitives with "default" hint (same as "number" for most cases)
                let left_prim = self.coerce_to_primitive(&left, "default")?;
                let right_prim = self.coerce_to_primitive(&right, "default")?;

                match (&left_prim, &right_prim) {
                    (JsValue::String(a), _) => {
                        JsValue::String(a.cheap_clone() + &right_prim.to_js_string())
                    }
                    (_, JsValue::String(b)) => {
                        JsValue::String(left_prim.to_js_string() + b.as_str())
                    }
                    _ => JsValue::Number(left_prim.to_number() + right_prim.to_number()),
                }
            }
            BinaryOp::Sub => {
                let left_num = self.coerce_to_number(&left)?;
                let right_num = self.coerce_to_number(&right)?;
                JsValue::Number(left_num - right_num)
            }
            BinaryOp::Mul => {
                let left_num = self.coerce_to_number(&left)?;
                let right_num = self.coerce_to_number(&right)?;
                JsValue::Number(left_num * right_num)
            }
            BinaryOp::Div => {
                let left_num = self.coerce_to_number(&left)?;
                let right_num = self.coerce_to_number(&right)?;
                JsValue::Number(left_num / right_num)
            }
            BinaryOp::Mod => {
                let left_num = self.coerce_to_number(&left)?;
                let right_num = self.coerce_to_number(&right)?;
                JsValue::Number(left_num % right_num)
            }
            BinaryOp::Exp => {
                let left_num = self.coerce_to_number(&left)?;
                let right_num = self.coerce_to_number(&right)?;
                JsValue::Number(left_num.powf(right_num))
            }

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
                let proto_key = PropertyKey::String(self.intern("prototype"));
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

                // Check if this is a proxy - use has trap
                let is_proxy = matches!(obj.borrow().exotic, ExoticObject::Proxy(_));
                if is_proxy {
                    let result = builtins::proxy::proxy_has(self, obj.clone(), &key)?;
                    JsValue::Boolean(result)
                } else {
                    JsValue::Boolean(obj.borrow().has_own_property(&key))
                }
            }
        };
        Ok(Guarded::unguarded(result))
    }

    /// Abstract Equality Comparison Algorithm (ECMAScript spec 7.2.14)
    /// Implements the == operator with type coercion
    fn abstract_equals(&mut self, left: &JsValue, right: &JsValue) -> bool {
        // If types are the same, use strict equality
        if std::mem::discriminant(left) == std::mem::discriminant(right) {
            return left.strict_equals(right);
        }

        match (left, right) {
            // 1. null == undefined and undefined == null
            (JsValue::Undefined, JsValue::Null) | (JsValue::Null, JsValue::Undefined) => true,

            // 2. Number == String: convert string to number
            (JsValue::Number(n), JsValue::String(s)) => *n == s.parse().unwrap_or(f64::NAN),
            (JsValue::String(s), JsValue::Number(n)) => s.parse().unwrap_or(f64::NAN) == *n,

            // 3. Boolean == anything: convert boolean to number and compare again
            (JsValue::Boolean(b), other) => {
                let num = if *b { 1.0 } else { 0.0 };
                self.abstract_equals(&JsValue::Number(num), other)
            }
            (other, JsValue::Boolean(b)) => {
                let num = if *b { 1.0 } else { 0.0 };
                self.abstract_equals(other, &JsValue::Number(num))
            }

            // 4. Object == String/Number/Symbol: convert object to primitive
            (JsValue::Object(_), JsValue::Number(_) | JsValue::String(_)) => {
                // ToPrimitive with default hint
                match self.coerce_to_primitive(left, "default") {
                    Ok(prim) => self.abstract_equals(&prim, right),
                    Err(_) => false,
                }
            }
            (JsValue::Number(_) | JsValue::String(_), JsValue::Object(_)) => {
                match self.coerce_to_primitive(right, "default") {
                    Ok(prim) => self.abstract_equals(left, &prim),
                    Err(_) => false,
                }
            }

            // 5. Object == Symbol: convert object to primitive
            (JsValue::Object(_), JsValue::Symbol(_)) => {
                match self.coerce_to_primitive(left, "default") {
                    Ok(prim) => self.abstract_equals(&prim, right),
                    Err(_) => false,
                }
            }
            (JsValue::Symbol(_), JsValue::Object(_)) => {
                match self.coerce_to_primitive(right, "default") {
                    Ok(prim) => self.abstract_equals(left, &prim),
                    Err(_) => false,
                }
            }

            // All other cases: not equal
            _ => false,
        }
    }

    /// ToPrimitive: Convert an object to a primitive value.
    /// For wrapper objects (Number, String, Boolean), this calls valueOf/toString.
    /// `hint` specifies preference: "number" tries valueOf first, "string" tries toString first.
    fn coerce_to_primitive(&mut self, value: &JsValue, hint: &str) -> Result<JsValue, JsError> {
        let obj = match value {
            JsValue::Object(obj) => obj,
            // Already primitive
            _ => return Ok(value.clone()),
        };

        // Determine method order based on hint
        let (first_method, second_method) = if hint == "string" {
            ("toString", "valueOf")
        } else {
            // "number" or "default" - try valueOf first
            ("valueOf", "toString")
        };

        // Try first method
        let first_key = PropertyKey::String(self.intern(first_method));
        let first_prop = obj.borrow().get_property(&first_key);
        if let Some(JsValue::Object(method)) = first_prop {
            if matches!(method.borrow().exotic, ExoticObject::Function(_)) {
                let result = self.call_function(JsValue::Object(method), value.clone(), &[])?;
                if !matches!(result.value, JsValue::Object(_)) {
                    return Ok(result.value);
                }
            }
        }

        // Try second method
        let second_key = PropertyKey::String(self.intern(second_method));
        if let Some(JsValue::Object(method)) = obj.borrow().get_property(&second_key) {
            if matches!(method.borrow().exotic, ExoticObject::Function(_)) {
                let result = self.call_function(JsValue::Object(method), value.clone(), &[])?;
                if !matches!(result.value, JsValue::Object(_)) {
                    return Ok(result.value);
                }
            }
        }

        // Fallback: return NaN for number hint, "[object Object]" for string hint
        if hint == "string" {
            Ok(JsValue::String(JsString::from("[object Object]")))
        } else {
            Ok(JsValue::Number(f64::NAN))
        }
    }

    /// Convert value to number, handling ToPrimitive for objects (ToNumber abstract operation).
    /// This properly calls the object's valueOf/toString methods per ECMAScript spec.
    pub fn coerce_to_number(&mut self, value: &JsValue) -> Result<f64, JsError> {
        match value {
            JsValue::Object(_) => {
                let prim = self.coerce_to_primitive(value, "number")?;
                Ok(prim.to_number())
            }
            _ => Ok(value.to_number()),
        }
    }

    /// Convert value to string, handling ToPrimitive for objects (ToString abstract operation).
    /// This properly calls the object's toString/valueOf methods per ECMAScript spec.
    pub fn coerce_to_string(&mut self, value: &JsValue) -> Result<JsString, JsError> {
        match value {
            JsValue::Object(_) => {
                // ToPrimitive with "string" hint - tries toString first, then valueOf
                let prim = self.coerce_to_primitive(value, "string")?;
                Ok(prim.to_js_string())
            }
            _ => Ok(value.to_js_string()),
        }
    }

    fn evaluate_unary(&mut self, un: &UnaryExpression) -> Result<Guarded, JsError> {
        // Handle delete specially - it needs to work on member expressions
        if un.operator == UnaryOp::Delete {
            return self.evaluate_delete(&un.argument);
        }

        // Handle typeof specially for identifiers - ECMAScript spec says
        // typeof on an undeclared variable should return "undefined", not throw
        if un.operator == UnaryOp::Typeof {
            // Handle TypeScript non-null assertion (x!)
            let arg = if let Expression::NonNull(non_null) = un.argument.as_ref() {
                non_null.expression.as_ref()
            } else {
                un.argument.as_ref()
            };

            if let Expression::Identifier(id) = arg {
                // Try to resolve identifier - if it fails, return "undefined"
                let result = match self.env_get(&id.name) {
                    Ok(value) => JsValue::String(JsString::from(value.type_of())),
                    Err(_) => JsValue::String(JsString::from("undefined")),
                };
                return Ok(Guarded::unguarded(result));
            }
        }

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
            UnaryOp::Delete => JsValue::Boolean(true), // Unreachable due to early return
        }))
    }

    fn evaluate_delete(&mut self, expr: &Expression) -> Result<Guarded, JsError> {
        // Handle TypeScript non-null assertion (x!)
        let expr = if let Expression::NonNull(non_null) = expr {
            non_null.expression.as_ref()
        } else {
            expr
        };

        match expr {
            Expression::Member(member) => {
                // Evaluate ONLY the object, not the full member expression
                let Guarded {
                    value: obj_val,
                    guard: _guard,
                } = self.evaluate_expression(&member.object)?;

                let JsValue::Object(obj) = obj_val else {
                    // Deleting from non-object returns true
                    return Ok(Guarded::unguarded(JsValue::Boolean(true)));
                };

                // Get the property key WITHOUT triggering proxy get trap
                let key = match &member.property {
                    crate::ast::MemberProperty::Identifier(id) => {
                        PropertyKey::String(id.name.cheap_clone())
                    }
                    crate::ast::MemberProperty::Expression(expr) => {
                        let Guarded {
                            value: val,
                            guard: _val_guard,
                        } = self.evaluate_expression(expr)?;
                        PropertyKey::from_value(&val)
                    }
                    crate::ast::MemberProperty::PrivateIdentifier(id) => {
                        PropertyKey::String(id.name.cheap_clone())
                    }
                };

                // Check if this is a proxy - use deleteProperty trap
                let is_proxy = matches!(obj.borrow().exotic, ExoticObject::Proxy(_));
                if is_proxy {
                    let result = builtins::proxy::proxy_delete_property(self, obj, &key)?;
                    return Ok(Guarded::unguarded(JsValue::Boolean(result)));
                }

                // Normal delete
                obj.borrow_mut().properties.remove(&key);
                Ok(Guarded::unguarded(JsValue::Boolean(true)))
            }
            Expression::Identifier(_) => {
                // Cannot delete local variables - always returns false
                Ok(Guarded::unguarded(JsValue::Boolean(false)))
            }
            other => {
                // For any other expression type, just evaluate it (for side effects)
                // and return true since we can't actually delete anything
                let _ = self.evaluate_expression(other)?;
                Ok(Guarded::unguarded(JsValue::Boolean(true)))
            }
        }
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
                                JsValue::Object(setter.cheap_clone()),
                                obj_val.clone(),
                                std::slice::from_ref(&final_value),
                            )?;
                        }
                        // If no setter, silently ignore in strict mode would throw, but we're lenient
                        return Ok(Guarded::unguarded(final_value));
                    }
                }

                // Check if this is a proxy
                let is_proxy = matches!(obj.borrow().exotic, ExoticObject::Proxy(_));
                if is_proxy {
                    // Use proxy set trap
                    builtins::proxy::proxy_set(
                        self,
                        obj.clone(),
                        key,
                        final_value.clone(),
                        obj_val.clone(),
                    )?;
                    return Ok(Guarded::unguarded(final_value));
                }

                // Handle __proto__ special property - set prototype instead of property
                if key.eq_str("__proto__") {
                    let new_proto = match &final_value {
                        JsValue::Object(proto_obj) => Some(proto_obj.cheap_clone()),
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

                // First pass: collect keys that are explicitly destructured
                let mut extracted_keys: Vec<JsString> = Vec::new();
                for prop in &obj_pattern.properties {
                    if let ObjectPatternProperty::KeyValue { key, .. } = prop {
                        if let Some(key_str) = self.extract_property_key_string(key) {
                            extracted_keys.push(key_str);
                        }
                    }
                }

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
                                ObjectPropertyKey::PrivateIdentifier(id) => id.name.cheap_clone(),
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
                        ObjectPatternProperty::Rest(rest) => {
                            // Create a new object with remaining properties
                            let guard = self.heap.create_guard();
                            let rest_obj = self.create_object(&guard);

                            // Copy all properties except those explicitly extracted
                            let obj_ref = obj.borrow();
                            for (key, prop) in obj_ref.properties.iter() {
                                let should_include = match key {
                                    PropertyKey::String(s) => {
                                        !extracted_keys.iter().any(|k| k == s)
                                    }
                                    PropertyKey::Symbol(_) => true,
                                    PropertyKey::Index(_) => true,
                                };
                                if should_include {
                                    rest_obj
                                        .borrow_mut()
                                        .set_property(key.clone(), prop.value.clone());
                                }
                            }
                            drop(obj_ref);

                            self.assign_pattern(&rest.argument, JsValue::Object(rest_obj))?;
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
                if let Some(elements) = obj_ref.array_elements() {
                    elements.to_vec()
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
                        let guard = self.heap.create_guard();
                        let rest_array = self.create_array_from(&guard, remaining);
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
                let JsValue::Object(obj) = obj_val.clone() else {
                    return Err(JsError::type_error("Cannot update property of non-object"));
                };
                let key = self.get_member_key(&member.property)?;

                // Check if this is a proxy - use proxy_get/proxy_set to properly delegate
                let is_proxy = matches!(obj.borrow().exotic, ExoticObject::Proxy(_));
                let current = if is_proxy {
                    let Guarded {
                        value,
                        guard: _guard,
                    } = builtins::proxy::proxy_get(
                        self,
                        obj.clone(),
                        key.clone(),
                        obj_val.clone(),
                    )?;
                    value
                } else {
                    obj.borrow()
                        .get_property(&key)
                        .unwrap_or(JsValue::Undefined)
                };

                let num = current.to_number();
                let new_val = match update.operator {
                    UpdateOp::Increment => JsValue::Number(num + 1.0),
                    UpdateOp::Decrement => JsValue::Number(num - 1.0),
                };

                if is_proxy {
                    builtins::proxy::proxy_set(self, obj, key, new_val.clone(), obj_val)?;
                } else {
                    obj.borrow_mut().set_property(key, new_val.clone());
                }

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
        // Check for direct eval call: eval(...)
        // Direct eval has access to the current scope, unlike indirect eval
        if let Expression::Identifier(id) = &*call.callee {
            if id.name.as_str() == "eval" {
                // Check that 'eval' refers to the global eval function
                if let Ok(JsValue::Object(eval_obj)) = self.env_get(&id.name) {
                    let eval_name = self.intern("eval");
                    let is_global_eval = {
                        if let Ok(JsValue::Object(global_eval)) = self.env_get(&eval_name) {
                            // Compare by Gc identity (same object)
                            std::ptr::eq(
                                &*eval_obj.borrow() as *const _,
                                &*global_eval.borrow() as *const _,
                            )
                        } else {
                            false
                        }
                    };

                    if is_global_eval {
                        // This is a direct eval call - execute in current scope
                        return self.execute_direct_eval(call);
                    }
                }
            }
        }

        let (callee, this_value, obj_guard) = match &*call.callee {
            // super(args) - call parent constructor
            Expression::Super(_) => {
                let super_name = self.intern("__super__");
                let super_constructor = self.env_get(&super_name)?;
                let this_name = self.intern("this");
                let this_val = self.env_get(&this_name)?;
                (super_constructor, this_val, None)
            }
            // super.method() - call parent method
            Expression::Member(member) if matches!(&*member.object, Expression::Super(_)) => {
                let super_name = self.intern("__super__");
                let super_constructor = self.env_get(&super_name)?;
                let this_name = self.intern("this");
                let this_val = self.env_get(&this_name)?;

                // Get the method from super's prototype
                let key = self.get_member_key(&member.property)?;
                let func = if let JsValue::Object(super_obj) = &super_constructor {
                    let proto_key = PropertyKey::String(self.intern("prototype"));
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

                // Handle optional chaining - if object is null/undefined, return undefined
                if member.optional && matches!(obj, JsValue::Null | JsValue::Undefined) {
                    return Err(JsError::type_error("undefined is not a function"));
                }

                // Cannot access properties on null/undefined - throw immediately
                // (this happens before argument evaluation per ECMAScript spec)
                if matches!(&obj, JsValue::Null | JsValue::Undefined) {
                    let key = self.get_member_key(&member.property)?;
                    let type_name = if matches!(&obj, JsValue::Null) {
                        "null"
                    } else {
                        "undefined"
                    };
                    return Err(JsError::type_error(format!(
                        "Cannot read properties of {} (reading '{}')",
                        type_name, key
                    )));
                }

                let key = self.get_member_key(&member.property)?;

                // Get the function, invoking getters if the property is an accessor
                let (func, extra_guard) = match &obj {
                    JsValue::Object(o) => {
                        // Check if this is a proxy - use proxy_get to properly delegate
                        let is_proxy = matches!(o.borrow().exotic, ExoticObject::Proxy(_));
                        if is_proxy {
                            let Guarded { value, guard } = builtins::proxy::proxy_get(
                                self,
                                o.clone(),
                                key.clone(),
                                obj.clone(),
                            )?;
                            (
                                Some(value).filter(|v| !matches!(v, JsValue::Undefined)),
                                guard,
                            )
                        } else {
                            // Check for accessor property - need to invoke getter
                            let prop_desc = o.borrow().get_property_descriptor(&key);
                            match prop_desc {
                                Some((prop, _)) if prop.is_accessor() => {
                                    // Invoke getter
                                    if let Some(getter) = prop.getter() {
                                        let Guarded {
                                            value: getter_val,
                                            guard: getter_guard,
                                        } = self.call_function(
                                            JsValue::Object(getter.clone()),
                                            obj.clone(),
                                            &[],
                                        )?;
                                        (Some(getter_val), getter_guard)
                                    } else {
                                        (None, None)
                                    }
                                }
                                _ => (o.borrow().get_property(&key), None),
                            }
                        }
                    }
                    JsValue::Number(_) => (self.number_prototype.borrow().get_property(&key), None),
                    JsValue::String(_) => {
                        // First check string-specific properties
                        if let PropertyKey::String(ref k) = key {
                            if k.as_str() == "length" {
                                (None, None) // length is not a function
                            } else {
                                (self.string_prototype.borrow().get_property(&key), None)
                            }
                        } else {
                            (None, None)
                        }
                    }
                    JsValue::Symbol(_) => (self.symbol_prototype.borrow().get_property(&key), None),
                    JsValue::Boolean(_) => {
                        (self.boolean_prototype.borrow().get_property(&key), None)
                    }
                    _ => (None, None),
                };

                // Keep extra guard alive (for values from getter calls)
                let _extra_guard = extra_guard;

                // Per ECMAScript spec, we don't throw here if the property is undefined/missing.
                // Arguments must be evaluated first, then the callable check happens in call_function.
                let callee = func.unwrap_or(JsValue::Undefined);
                (callee, obj, obj_guard)
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
                        if let Some(elements) = obj_ref.array_elements() {
                            args.extend(elements.iter().cloned());
                        }
                    }
                }
            }
        }

        // Call function - propagate guard from result
        self.call_function(callee, this_value, &args)
    }

    /// Execute a direct eval call in the current scope.
    ///
    /// Direct eval (`eval(...)` where eval is the identifier) has access to the
    /// calling scope. This is different from indirect eval which uses global scope.
    fn execute_direct_eval(&mut self, call: &CallExpression) -> Result<Guarded, JsError> {
        // Evaluate the first argument
        let arg = match call.arguments.first() {
            None => return Ok(Guarded::unguarded(JsValue::Undefined)),
            Some(crate::ast::Argument::Expression(expr)) => self.evaluate_expression(expr)?.value,
            Some(crate::ast::Argument::Spread(_)) => {
                return Err(JsError::syntax_error_simple(
                    "Spread argument not allowed in eval",
                ))
            }
        };

        // If argument is not a string, return it directly
        let code = match arg {
            JsValue::String(s) => s.to_string(),
            _ => return Ok(Guarded::unguarded(arg)),
        };

        // Execute the code in current scope (direct eval behavior)
        builtins::global::eval_code_in_scope(self, &code, false)
    }

    pub fn call_function(
        &mut self,
        callee: JsValue,
        this_value: JsValue,
        args: &[JsValue],
    ) -> Result<Guarded, JsError> {
        // Check call stack depth limit
        if self.max_call_depth > 0 && self.call_stack.len() >= self.max_call_depth {
            return Err(JsError::range_error(format!(
                "Maximum call stack size exceeded (depth {})",
                self.call_stack.len()
            )));
        }

        let JsValue::Object(func_obj) = callee else {
            return Err(JsError::type_error("Not a function"));
        };

        // Check if this is a proxy - use apply trap
        let is_proxy = matches!(func_obj.borrow().exotic, ExoticObject::Proxy(_));
        if is_proxy {
            return builtins::proxy::proxy_apply(self, func_obj, this_value, args.to_vec());
        }

        let func = {
            let obj_ref = func_obj.borrow();
            match &obj_ref.exotic {
                ExoticObject::Function(f) => f.clone(),
                _ => return Err(JsError::type_error("Not a function")),
            }
        };

        match func {
            JsFunction::Interpreted(interp) => {
                // Push call stack frame for stack traces and depth tracking
                let func_name = interp
                    .name
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "<anonymous>".to_string());
                self.call_stack.push(StackFrame {
                    function_name: func_name,
                    location: Some((interp.source_location.line, interp.source_location.column)),
                });

                // Handle generator functions - create generator object instead of executing
                if interp.generator {
                    let body = match &*interp.body {
                        FunctionBody::Block(block) => block.cheap_clone(),
                        FunctionBody::Expression(_) => {
                            return Err(JsError::type_error("Generator must have block body"));
                        }
                    };

                    // Allocate a unique ID for this generator
                    let gen_id = self.next_generator_id;
                    self.next_generator_id += 1;

                    let gen_state = GeneratorState {
                        body,
                        params: interp.params.cheap_clone(),
                        args: args.to_vec(),
                        closure: interp.closure,
                        status: GeneratorStatus::Suspended,
                        sent_value: JsValue::Undefined,
                        name: interp.name.cheap_clone(),
                        id: gen_id,
                        started: false,
                    };

                    let gen_obj = builtins::create_generator_object(self, gen_state);
                    self.call_stack.pop();
                    return Ok(Guarded::unguarded(JsValue::Object(gen_obj)));
                }

                // Create new environment with guard
                let (func_env, func_guard) =
                    create_environment_unrooted(&self.heap, Some(interp.closure));

                // Bind `this` in the function environment
                {
                    let this_name = self.intern("this");
                    if let Some(data) = func_env.borrow_mut().as_environment_mut() {
                        data.bindings.insert(
                            VarKey(this_name),
                            Binding {
                                value: this_value.clone(),
                                mutable: false,
                                initialized: true,
                                import_binding: None,
                            },
                        );
                    }
                }

                // Bind `__super__` if this is a class constructor with inheritance
                {
                    let super_name = self.intern("__super__");
                    let super_key = PropertyKey::String(super_name.cheap_clone());
                    if let Some(super_val) = func_obj.borrow().get_property(&super_key) {
                        if let Some(data) = func_env.borrow_mut().as_environment_mut() {
                            data.bindings.insert(
                                VarKey(super_name),
                                Binding {
                                    value: super_val,
                                    mutable: false,
                                    initialized: true,
                                    import_binding: None,
                                },
                            );
                        }
                    }
                }

                // Execute function body - set up environment first so bind_pattern works
                let saved_env = self.env.cheap_clone();
                self.env = func_env;
                self.push_env_guard(func_guard);

                // Create and bind the `arguments` object (array-like)
                {
                    let args_guard = self.heap.create_guard();
                    let args_array = self.create_array_from(&args_guard, args.to_vec());
                    let args_name = self.intern("arguments");
                    self.env_define(args_name, JsValue::Object(args_array), false);
                }

                // Bind parameters using bind_pattern for full destructuring support
                for (i, param) in interp.params.iter().enumerate() {
                    match &param.pattern {
                        Pattern::Rest(rest) => {
                            // Collect remaining arguments into an array
                            let rest_args: Vec<JsValue> =
                                args.get(i..).unwrap_or_default().to_vec();
                            let rest_guard = self.heap.create_guard();
                            let rest_array = self.create_array_from(&rest_guard, rest_args);

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
                        FunctionBody::Block(block) => {
                            // Execute function body using stack-based execution
                            let mut state = stack::ExecutionState::new();
                            state.push_frame(stack::Frame::Block {
                                statements: Rc::clone(&block.body),
                                index: 0,
                            });
                            match self.run(&mut state) {
                                stack::StepResult::Done(g) => {
                                    // Check if we got a return
                                    if matches!(state.completion, stack::StackCompletion::Return) {
                                        Ok((g.value, g.guard))
                                    } else {
                                        Ok((JsValue::Undefined, None))
                                    }
                                }
                                stack::StepResult::Error(e) => Err(e),
                                stack::StepResult::Suspend(_) => Err(JsError::type_error(
                                    "Function execution cannot be suspended",
                                )),
                                stack::StepResult::Continue => Ok((JsValue::Undefined, None)),
                            }
                        }
                        FunctionBody::Expression(expr) => match self.evaluate_expression(expr) {
                            Ok(Guarded { value, guard }) => Ok((value, guard)),
                            Err(e) => Err(e),
                        },
                    };

                // ALWAYS restore environment, even on error
                self.pop_env_guard();
                self.env = saved_env;

                // Handle async functions - wrap result in Promise
                if interp.async_ {
                    match body_result {
                        Ok((result, result_guard)) => {
                            // Promise assimilation: if result is already a Promise, return it directly
                            // This prevents double-wrapping (async function returning Promise should
                            // resolve to the inner Promise's value, not a Promise<Promise<T>>)
                            if let JsValue::Object(ref obj) = result {
                                if matches!(obj.borrow().exotic, ExoticObject::Promise(_)) {
                                    // Return the Promise directly, preserving its guard
                                    self.call_stack.pop();
                                    return Ok(Guarded {
                                        value: result,
                                        guard: result_guard,
                                    });
                                }
                            }
                            // Create fulfilled promise with the result
                            let guard = self.heap.create_guard();
                            let promise =
                                builtins::promise::create_fulfilled_promise(self, &guard, result);
                            self.call_stack.pop();
                            return Ok(Guarded::with_guard(JsValue::Object(promise), guard));
                        }
                        Err(e) => {
                            // Create rejected promise with the error
                            let guard = self.heap.create_guard();
                            let promise = builtins::promise::create_rejected_promise(
                                self,
                                &guard,
                                e.to_value(),
                            );
                            self.call_stack.pop();
                            return Ok(Guarded::with_guard(JsValue::Object(promise), guard));
                        }
                    }
                }

                // Now propagate the result or error (non-async case)
                // Pop call stack before returning (on success or error)
                match body_result {
                    Ok((result, result_guard)) => {
                        self.call_stack.pop();
                        Ok(Guarded {
                            value: result,
                            guard: result_guard,
                        })
                    }
                    Err(e) => {
                        self.call_stack.pop();
                        Err(e)
                    }
                }
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

            JsFunction::PromiseResolve(promise) => {
                let value = args.first().cloned().unwrap_or(JsValue::Undefined);
                builtins::promise::resolve_promise_value(self, &promise, value)?;
                Ok(Guarded::unguarded(JsValue::Undefined))
            }

            JsFunction::PromiseReject(promise) => {
                let reason = args.first().cloned().unwrap_or(JsValue::Undefined);
                builtins::promise::reject_promise_value(self, &promise, reason)?;
                Ok(Guarded::unguarded(JsValue::Undefined))
            }

            JsFunction::PromiseAllFulfill { state, index } => {
                let value = args.first().cloned().unwrap_or(JsValue::Undefined);
                builtins::promise::handle_promise_all_fulfill(self, &state, index, value)?;
                Ok(Guarded::unguarded(JsValue::Undefined))
            }

            JsFunction::PromiseAllReject(state) => {
                let reason = args.first().cloned().unwrap_or(JsValue::Undefined);
                builtins::promise::handle_promise_all_reject(self, &state, reason)?;
                Ok(Guarded::unguarded(JsValue::Undefined))
            }

            JsFunction::AccessorGetter => {
                // Auto-accessor getter - read from storage slot on `this`
                let storage_key_prop = self.intern("__accessor_storage_key__");
                let init_value_prop = self.intern("__accessor_init_value__");

                let func_ref = func_obj.borrow();
                let storage_key = func_ref
                    .get_property(&PropertyKey::String(storage_key_prop))
                    .and_then(|v| {
                        if let JsValue::String(s) = v {
                            Some(s)
                        } else {
                            None
                        }
                    });
                let init_val = func_ref.get_property(&PropertyKey::String(init_value_prop));
                drop(func_ref);

                if let Some(key) = storage_key {
                    if let JsValue::Object(this_obj) = &this_value {
                        let this_ref = this_obj.borrow();
                        if let Some(val) = this_ref.get_property(&PropertyKey::String(key)) {
                            return Ok(Guarded::unguarded(val));
                        }
                    }
                    // Return initial value if not yet set
                    if let Some(val) = init_val {
                        return Ok(Guarded::unguarded(val));
                    }
                }
                Ok(Guarded::unguarded(JsValue::Undefined))
            }

            JsFunction::AccessorSetter => {
                // Auto-accessor setter - write to storage slot on `this`
                let storage_key_prop = self.intern("__accessor_storage_key__");
                let value = args.first().cloned().unwrap_or(JsValue::Undefined);

                let func_ref = func_obj.borrow();
                let storage_key = func_ref
                    .get_property(&PropertyKey::String(storage_key_prop))
                    .and_then(|v| {
                        if let JsValue::String(s) = v {
                            Some(s)
                        } else {
                            None
                        }
                    });
                drop(func_ref);

                if let Some(key) = storage_key {
                    if let JsValue::Object(this_obj) = &this_value {
                        this_obj
                            .borrow_mut()
                            .set_property(PropertyKey::String(key), value);
                    }
                }
                Ok(Guarded::unguarded(JsValue::Undefined))
            }

            JsFunction::ModuleExportGetter {
                module_env,
                binding_name,
            } => {
                // Module export getter - read binding from module's environment
                let env_ref = module_env.borrow();
                if let Some(env_data) = env_ref.as_environment() {
                    let key = VarKey(binding_name.cheap_clone());
                    if let Some(binding) = env_data.bindings.get(&key) {
                        return Ok(Guarded::unguarded(binding.value.clone()));
                    }
                }
                Ok(Guarded::unguarded(JsValue::Undefined))
            }

            JsFunction::ModuleReExportGetter {
                source_module,
                source_key,
            } => {
                // Re-export getter - delegate to source module's property
                let value = self.resolve_module_property(&source_module, &source_key)?;
                Ok(Guarded::unguarded(value))
            }

            JsFunction::ProxyRevoke(proxy) => {
                // Revoke the associated proxy
                let mut proxy_ref = proxy.borrow_mut();
                if let ExoticObject::Proxy(ref mut data) = proxy_ref.exotic {
                    data.revoked = true;
                }
                Ok(Guarded::unguarded(JsValue::Undefined))
            }
        }
    }

    fn evaluate_member(&mut self, member: &MemberExpression) -> Result<Guarded, JsError> {
        let Guarded {
            value: obj,
            guard: obj_guard,
        } = self.evaluate_expression(&member.object)?;

        // Handle optional chaining - if object is null/undefined, return undefined
        if member.optional && matches!(obj, JsValue::Null | JsValue::Undefined) {
            return Ok(Guarded::unguarded(JsValue::Undefined));
        }

        let key = self.get_member_key(&member.property)?;

        // Get the value from the member access
        // Returns (value, optional_extra_guard) - extra guard for values from getter calls
        let (value, extra_guard) = match &obj {
            JsValue::Object(o) => {
                // Check if this is a proxy
                let is_proxy = matches!(o.borrow().exotic, ExoticObject::Proxy(_));
                if is_proxy {
                    // Use proxy get trap
                    let Guarded { value, guard } =
                        builtins::proxy::proxy_get(self, o.clone(), key, obj.clone())?;
                    (value, guard)
                } else if key.eq_str("__proto__") {
                    // Handle __proto__ special property
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
                                } = self.call_function(
                                    JsValue::Object(getter.clone()),
                                    obj.clone(),
                                    &[],
                                )?;
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
            JsValue::Boolean(_) => {
                // Look up methods on Boolean.prototype
                if let Some(method) = self.boolean_prototype.borrow().get_property(&key) {
                    (method, None)
                } else {
                    (JsValue::Undefined, None)
                }
            }
            JsValue::Null => {
                return Err(JsError::type_error(format!(
                    "Cannot read properties of null (reading '{}')",
                    key
                )));
            }
            JsValue::Undefined => {
                return Err(JsError::type_error(format!(
                    "Cannot read properties of undefined (reading '{}')",
                    key
                )));
            }
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
            MemberProperty::PrivateIdentifier(id) => Ok(PropertyKey::String(id.name.cheap_clone())),
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
                        if let Some(arr_elements) = obj_ref.array_elements() {
                            elements.extend(arr_elements.iter().cloned());
                        }
                    }
                }
                None => elements.push(JsValue::Undefined),
            }
        }

        // Create array with guard - elements ownership transferred to array
        let guard = self.heap.create_guard();
        let arr_obj = self.create_array_from(&guard, elements);
        Ok(Guarded::with_guard(JsValue::Object(arr_obj), guard))
    }

    fn evaluate_object(&mut self, obj_expr: &ObjectExpression) -> Result<Guarded, JsError> {
        // Pre-allocate for expected number of properties to avoid hashmap resizing
        let obj_guard = self.heap.create_guard();
        let obj = self.create_object_with_capacity(&obj_guard, obj_expr.properties.len());

        // Keep property value guards alive until ownership is transferred to obj
        let mut _prop_guards = Vec::new();

        // Collect accessors (getters/setters) by property key
        let mut accessors: AccessorMap = FxHashMap::default();

        for prop in &obj_expr.properties {
            match prop {
                ObjectProperty::Property(p) => {
                    let prop_key = match &p.key {
                        ObjectPropertyKey::Identifier(id) => {
                            PropertyKey::String(id.name.cheap_clone())
                        }
                        ObjectPropertyKey::String(s) => PropertyKey::from(s.value.cheap_clone()),
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
                            PropertyKey::String(id.name.cheap_clone())
                        }
                    };

                    // Handle getter/setter properties
                    match p.kind {
                        PropertyKind::Get => {
                            // Evaluate the getter function
                            let Guarded {
                                value: getter_val,
                                guard: getter_guard,
                            } = self.evaluate_expression(&p.value)?;

                            if let Some(g) = getter_guard {
                                _prop_guards.push(g);
                            }

                            if let JsValue::Object(getter_fn) = getter_val {
                                let entry = accessors.entry(prop_key).or_insert((None, None));
                                entry.0 = Some(getter_fn);
                            }
                            continue;
                        }
                        PropertyKind::Set => {
                            // Evaluate the setter function
                            let Guarded {
                                value: setter_val,
                                guard: setter_guard,
                            } = self.evaluate_expression(&p.value)?;

                            if let Some(g) = setter_guard {
                                _prop_guards.push(g);
                            }

                            if let JsValue::Object(setter_fn) = setter_val {
                                let entry = accessors.entry(prop_key).or_insert((None, None));
                                entry.1 = Some(setter_fn);
                            }
                            continue;
                        }
                        PropertyKind::Init => {
                            // Regular property - continue with normal processing
                        }
                    }

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
                            JsValue::Object(proto_obj) => Some(proto_obj.cheap_clone()),
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

        // Now define accessor properties
        for (key, (getter, setter)) in accessors {
            let accessor_prop = Property::accessor(getter, setter);
            obj.borrow_mut().properties.insert(key, accessor_prop);
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
                for declarator in decl.declarations.iter() {
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
                        for declarator in decl.declarations.iter() {
                            self.hoist_pattern_names(&declarator.id);
                        }
                    }
                }
                self.hoist_var_in_statement(&for_stmt.body);
            }
            Statement::ForIn(for_in) => {
                if let ForInOfLeft::Variable(decl) = &for_in.left {
                    if decl.kind == VariableKind::Var {
                        for declarator in decl.declarations.iter() {
                            self.hoist_pattern_names(&declarator.id);
                        }
                    }
                }
                self.hoist_var_in_statement(&for_in.body);
            }
            Statement::ForOf(for_of) => {
                if let ForInOfLeft::Variable(decl) = &for_of.left {
                    if decl.kind == VariableKind::Var {
                        for declarator in decl.declarations.iter() {
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
                for case in switch_stmt.cases.iter() {
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
            let key = VarKey(name.cheap_clone());
            data.bindings.contains_key(&key)
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

/// Native implementation of context.addInitializer()
/// This function captures the initializers array via the __initializers__ property
fn add_initializer_impl(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // Get the initializer function argument
    let initializer = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Validate that the argument is a function
    if !matches!(&initializer, JsValue::Object(obj) if obj.borrow().is_callable()) {
        return Err(JsError::type_error(
            "addInitializer callback must be a function",
        ));
    }

    // Get the __initializers__ array from the addInitializer function itself
    // The `this` binding is the context object (since addInitializer is called as context.addInitializer())
    // We stored the initializers array on the addInitializer function object
    if let JsValue::Object(ctx) = this {
        let ctx_ref = ctx.borrow();
        let key = interp.intern("addInitializer");
        if let Some(JsValue::Object(add_init_fn)) = ctx_ref.get_property(&PropertyKey::String(key))
        {
            let func_ref = add_init_fn.borrow();
            let init_key = interp.intern("__initializers__");
            if let Some(JsValue::Object(init_arr)) =
                func_ref.get_property(&PropertyKey::String(init_key))
            {
                // Push the initializer to the array using set_property which handles array growth
                drop(func_ref);
                let mut arr_ref = init_arr.borrow_mut();
                let index = arr_ref.array_length().unwrap_or(0);
                arr_ref.set_property(PropertyKey::Index(index), initializer);
            }
        }
    }

    Ok(Guarded::unguarded(JsValue::Undefined))
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
