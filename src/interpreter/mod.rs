//! Interpreter for executing TypeScript AST
//!
//! This module implements a minimal interpreter using the new guard-based GC.

// Builtin function implementations
pub mod builtins;

// Bytecode virtual machine
pub mod bytecode_vm;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use crate::ast::{ImportSpecifier, Program, Statement};
use crate::error::JsError;
use crate::gc::{Gc, Guard, Heap};
use crate::parser::Parser;
use crate::string_dict::StringDict;
use crate::value::{
    create_environment_unrooted, create_environment_unrooted_with_capacity, Binding,
    BytecodeFunction, BytecodeGeneratorState, CheapClone, EnvRef, EnvironmentData, ExoticObject,
    GeneratorStatus, Guarded, ImportBinding, JsFunction, JsObject, JsString, JsSymbol, JsValue,
    ModuleExport, NativeFn, NativeFunction, PromiseStatus, Property, PropertyKey, VarKey,
};
use rustc_hash::FxHashMap;

use self::builtins::symbol::WellKnownSymbols;

// Re-export Guarded from value module - see value.rs for documentation

/// A stack frame for tracking call stack
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Function name (or "<anonymous>" for anonymous functions)
    pub function_name: String,
    /// Source location if available
    pub location: Option<(u32, u32)>, // (line, column)
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
    /// Exported values from the module
    /// Uses ModuleExport to distinguish direct exports (with live bindings) from re-exports
    pub exports: FxHashMap<JsString, ModuleExport>,

    /// Call stack for stack traces
    pub call_stack: Vec<StackFrame>,

    /// Counter for generating unique generator IDs
    next_generator_id: u64,

    /// Counter for generating unique symbol IDs
    next_symbol_id: u64,

    /// Symbol registry for Symbol.for() / Symbol.keyFor()
    symbol_registry: FxHashMap<JsString, JsSymbol>,

    /// Well-known symbols (Symbol.iterator, Symbol.toStringTag, etc.)
    pub well_known_symbols: WellKnownSymbols,

    /// Console timers for console.time() / console.timeEnd()
    console_timers: FxHashMap<String, Instant>,

    /// Console counters for console.count() / console.countReset()
    console_counters: FxHashMap<String, u64>,

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

    /// Suspended bytecode VM state (if any)
    pub(crate) suspended_vm_state: Option<bytecode_vm::VmSuspension>,

    /// Pending program waiting for imports to be provided
    pub(crate) pending_program: Option<crate::ast::Program>,

    /// Pending module sources waiting for their imports to be satisfied
    /// Maps normalized path -> parsed program
    pub(crate) pending_module_sources: FxHashMap<crate::ModulePath, crate::ast::Program>,
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

        // Initialize symbol counter and well-known symbols
        // Well-known symbols get IDs 1-12, next_symbol_id starts at 13
        let mut symbol_counter = 1u64;
        let well_known_symbols = WellKnownSymbols::new(&mut symbol_counter);

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
            exports: FxHashMap::default(),
            call_stack: Vec::new(),
            next_generator_id: 1,
            next_symbol_id: symbol_counter,
            symbol_registry: FxHashMap::default(),
            well_known_symbols,
            console_timers: FxHashMap::default(),
            console_counters: FxHashMap::default(),
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
            suspended_vm_state: None,
            pending_program: None,
            pending_module_sources: FxHashMap::default(),
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
    // Symbol Management
    // ═══════════════════════════════════════════════════════════════════════════

    /// Generate a unique symbol ID
    pub fn next_symbol_id(&mut self) -> u64 {
        let id = self.next_symbol_id;
        self.next_symbol_id += 1;
        id
    }

    /// Get a symbol from the registry by key (for Symbol.for)
    pub fn symbol_registry_get(&self, key: &JsString) -> Option<JsSymbol> {
        self.symbol_registry.get(key).cloned()
    }

    /// Insert a symbol into the registry (for Symbol.for)
    pub fn symbol_registry_insert(&mut self, key: JsString, symbol: JsSymbol) {
        self.symbol_registry.insert(key, symbol);
    }

    /// Find the key for a registered symbol (for Symbol.keyFor)
    pub fn symbol_registry_key_for(&self, symbol_id: u64) -> Option<JsString> {
        for (key, sym) in &self.symbol_registry {
            if sym.id() == symbol_id {
                return Some(key.cheap_clone());
            }
        }
        None
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Console State
    // ═══════════════════════════════════════════════════════════════════════════

    /// Start a console timer
    pub fn console_timer_start(&mut self, label: String) {
        self.console_timers.insert(label, Instant::now());
    }

    /// End a console timer and return elapsed milliseconds, or None if timer doesn't exist
    pub fn console_timer_end(&mut self, label: &str) -> Option<u128> {
        self.console_timers
            .remove(label)
            .map(|start| start.elapsed().as_millis())
    }

    /// Increment a console counter and return the new count
    pub fn console_counter_increment(&mut self, label: String) -> u64 {
        let count = self.console_counters.entry(label).or_insert(0);
        *count += 1;
        *count
    }

    /// Reset a console counter
    pub fn console_counter_reset(&mut self, label: &str) {
        self.console_counters.remove(label);
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
        use crate::compiler::Compiler;
        use bytecode_vm::BytecodeVM;

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

        // All imports satisfied - set up import bindings first
        self.setup_import_bindings(&program)?;

        // Now compile to bytecode and execute
        self.start_execution();

        // Compile the program to bytecode
        let chunk = Compiler::compile_program(&program)?;

        // Run the bytecode VM
        let vm_guard = self.heap.create_guard();
        let vm = BytecodeVM::with_guard(chunk, JsValue::Object(self.global.clone()), vm_guard);

        self.run_vm_to_completion(vm)
    }

    /// Run a bytecode VM to completion or suspension
    fn run_vm_to_completion(
        &mut self,
        mut vm: bytecode_vm::BytecodeVM,
    ) -> Result<crate::RuntimeResult, JsError> {
        use bytecode_vm::VmResult;

        let result = vm.run(self);

        match result {
            VmResult::Complete(guarded) => {
                // Check if there are pending orders to return
                if !self.pending_orders.is_empty() {
                    let pending = std::mem::take(&mut self.pending_orders);
                    let cancelled = std::mem::take(&mut self.cancelled_orders);
                    return Ok(crate::RuntimeResult::Suspended { pending, cancelled });
                }
                // Check for unfulfilled orders from previous suspension
                if !self.order_callbacks.is_empty() {
                    let cancelled = std::mem::take(&mut self.cancelled_orders);
                    return Ok(crate::RuntimeResult::Suspended {
                        pending: Vec::new(),
                        cancelled,
                    });
                }
                Ok(crate::RuntimeResult::Complete(
                    crate::RuntimeValue::from_guarded(guarded),
                ))
            }
            VmResult::Error(err) => Err(self.materialize_thrown_error(err)),
            VmResult::Suspend(suspension) => {
                // Save VM state for resumption
                self.suspended_vm_state = Some(suspension);
                let pending = std::mem::take(&mut self.pending_orders);
                let cancelled = std::mem::take(&mut self.cancelled_orders);
                Ok(crate::RuntimeResult::Suspended { pending, cancelled })
            }
            VmResult::Yield(_) | VmResult::YieldStar(_) => Err(JsError::internal_error(
                "Bytecode execution cannot yield at top level",
            )),
        }
    }

    /// Convert ThrownValue errors to RuntimeError with string data
    ///
    /// JsError::ThrownValue contains a JsValue that may have Gc pointers.
    /// These pointers become invalid when the interpreter/heap is dropped.
    /// This function extracts the error information while it's still valid.
    fn materialize_thrown_error(&mut self, error: JsError) -> JsError {
        match error {
            JsError::ThrownValue { guarded } => {
                // Extract error name and message from the thrown value
                if let JsValue::Object(obj) = &guarded.value {
                    let name_key = self.property_key("name");
                    let message_key = self.property_key("message");
                    let obj_ref = obj.borrow();
                    let name_val = obj_ref.get_property(&name_key);
                    let message_val = obj_ref.get_property(&message_key);
                    drop(obj_ref);

                    let name = name_val
                        .map(|v| self.to_js_string(&v).to_string())
                        .unwrap_or_else(|| "Error".to_string());
                    let message = message_val
                        .map(|v| self.to_js_string(&v).to_string())
                        .unwrap_or_default();
                    JsError::RuntimeError {
                        kind: name,
                        message,
                        stack: Vec::new(),
                    }
                } else {
                    // Non-object thrown value - convert to string
                    let message = self.to_js_string(&guarded.value).to_string();
                    JsError::RuntimeError {
                        kind: "Error".to_string(),
                        message,
                        stack: Vec::new(),
                    }
                }
            }
            // Pass through other error types unchanged
            other => other,
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

    /// Set up import bindings for a program before bytecode execution.
    /// This resolves all imports and creates bindings in the current environment
    /// so that the bytecode can reference imported values.
    fn setup_import_bindings(&mut self, program: &Program) -> Result<(), JsError> {
        for stmt in program.body.iter() {
            if let Statement::Import(import) = stmt {
                // Skip type-only imports
                if import.type_only {
                    continue;
                }

                let specifier = import.source.value.to_string();

                // Resolve the module
                let module_obj = self.resolve_module(&specifier)?;

                // Set up bindings for each import specifier
                for spec in &import.specifiers {
                    match spec {
                        ImportSpecifier::Named {
                            local, imported, ..
                        } => {
                            // import { foo as bar } from "mod" -> bar binds to mod.foo
                            let property_key = PropertyKey::String(imported.name.cheap_clone());
                            self.env_define_import(
                                local.name.cheap_clone(),
                                module_obj.cheap_clone(),
                                property_key,
                            );
                        }
                        ImportSpecifier::Default { local, .. } => {
                            // import foo from "mod" -> foo binds to mod.default
                            let property_key = PropertyKey::String(self.intern("default"));
                            self.env_define_import(
                                local.name.cheap_clone(),
                                module_obj.cheap_clone(),
                                property_key,
                            );
                        }
                        ImportSpecifier::Namespace { local, .. } => {
                            // import * as foo from "mod" -> foo binds to the entire module object
                            // For namespace imports, we define a regular binding to the module object
                            self.env_define(
                                local.name.cheap_clone(),
                                JsValue::Object(module_obj.cheap_clone()),
                                false, // immutable
                            );
                        }
                    }
                }
            }
        }
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

        // Set up import bindings before bytecode execution
        self.setup_import_bindings(&program)?;

        // Execute module using bytecode compilation
        let result = self.execute_program_bytecode(&program);

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
        use crate::compiler::Compiler;
        use bytecode_vm::BytecodeVM;

        // If we have a suspended VM state, resume it
        if let Some(suspension) = self.suspended_vm_state.take() {
            // Check if the promise was resolved
            let promise_status = {
                let obj_ref = suspension.waiting_on.borrow();
                if let ExoticObject::Promise(promise_state) = &obj_ref.exotic {
                    let status = promise_state.borrow().status.clone();
                    let result = promise_state.borrow().result.clone();
                    Some((status, result))
                } else {
                    None
                }
            };

            if let Some((status, result)) = promise_status {
                match status {
                    PromiseStatus::Fulfilled => {
                        let value = result.unwrap_or(JsValue::Undefined);
                        // Guard the value to prevent GC during execution
                        let vm_guard = self.heap.create_guard();
                        if let JsValue::Object(ref obj) = value {
                            vm_guard.guard(obj.cheap_clone());
                        }

                        // Create VM from saved state and resume
                        let mut vm = BytecodeVM::from_saved_state(
                            suspension.state,
                            JsValue::Object(self.global.clone()),
                            vm_guard,
                        );

                        // Set the resolved value in the resume register
                        vm.set_resume_value(suspension.resume_register, value);

                        return self.run_vm_to_completion(vm);
                    }
                    PromiseStatus::Rejected => {
                        let reason = result.unwrap_or(JsValue::Undefined);
                        // Guard the reason to prevent GC during execution
                        let vm_guard = self.heap.create_guard();
                        if let JsValue::Object(ref obj) = reason {
                            vm_guard.guard(obj.cheap_clone());
                        }

                        // Create VM from saved state and inject the exception
                        let mut vm = BytecodeVM::from_saved_state(
                            suspension.state,
                            JsValue::Object(self.global.clone()),
                            vm_guard,
                        );

                        // Inject the exception - this finds a try/catch handler if present
                        if vm.inject_exception(self, reason.clone()) {
                            // Handler found - run VM to handle the exception
                            return self.run_vm_to_completion(vm);
                        } else {
                            // No handler - propagate as error
                            let guarded = Guarded::from_value(reason, &self.heap);
                            return Err(JsError::thrown(guarded));
                        }
                    }
                    PromiseStatus::Pending => {
                        // Still pending - re-suspend
                        self.suspended_vm_state = Some(suspension);
                        let pending = std::mem::take(&mut self.pending_orders);
                        let cancelled = std::mem::take(&mut self.cancelled_orders);
                        return Ok(crate::RuntimeResult::Suspended { pending, cancelled });
                    }
                }
            } else {
                // Not a promise - should not happen
                return Err(JsError::internal_error(
                    "Suspended VM was waiting on non-promise",
                ));
            }
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

            // All imports satisfied - set up import bindings first
            self.setup_import_bindings(&program)?;

            // Now compile to bytecode and execute
            self.start_execution();

            // Compile the program to bytecode
            let chunk = Compiler::compile_program(&program)?;

            // Run the bytecode VM
            let vm_guard = self.heap.create_guard();
            let vm = BytecodeVM::with_guard(chunk, JsValue::Object(self.global.clone()), vm_guard);

            return self.run_vm_to_completion(vm);
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
    pub fn env_get(&self, name: &JsString) -> Result<JsValue, JsError> {
        let mut current = Some(self.env.cheap_clone());
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
            } else {
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

    /// Create a native function object.
    /// Caller provides the guard to control object lifetime.
    pub fn create_native_fn(
        &mut self,
        guard: &Guard<JsObject>,
        name: &str, // FIXME: make it JsString
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

    /// Create a bytecode function object.
    /// Caller provides the guard to control object lifetime.
    pub fn create_bytecode_function(
        &mut self,
        guard: &Guard<JsObject>,
        bc_func: BytecodeFunction,
    ) -> Gc<JsObject> {
        let length_key = PropertyKey::String(self.intern("length"));
        let name_key = PropertyKey::String(self.intern("name"));
        let proto_key = PropertyKey::String(self.intern("prototype"));
        let ctor_key = PropertyKey::String(self.intern("constructor"));

        // Get name, param count, and is_arrow from function_info
        let (func_name, param_count, is_arrow) = bc_func
            .chunk
            .function_info
            .as_ref()
            .map(|info| {
                let name = info
                    .name
                    .as_ref()
                    .map(|n| n.cheap_clone())
                    .unwrap_or_else(|| JsString::from(""));
                (name, info.param_count, info.is_arrow)
            })
            .unwrap_or_else(|| (JsString::from(""), 0, false));

        let func_obj = guard.alloc();
        {
            let mut f_ref = func_obj.borrow_mut();
            f_ref.prototype = Some(self.function_prototype.clone());
            f_ref.exotic = ExoticObject::Function(JsFunction::Bytecode(bc_func));
            // Set length property (number of formal parameters)
            f_ref.set_property(length_key, JsValue::Number(param_count as f64));
            // Set name property
            f_ref.set_property(name_key, JsValue::String(func_name));
        }

        // Regular functions (not arrow functions) need a .prototype property
        // This is the prototype object that will be used when the function is called with `new`
        if !is_arrow {
            let proto_obj = guard.alloc();
            proto_obj.borrow_mut().prototype = Some(self.object_prototype.clone());
            // Set prototype.constructor = function
            proto_obj
                .borrow_mut()
                .set_property(ctor_key, JsValue::Object(func_obj.cheap_clone()));
            // Set function.prototype = prototype object
            func_obj
                .borrow_mut()
                .set_property(proto_key, JsValue::Object(proto_obj));
        }

        func_obj
    }

    /// Create a bytecode generator function object.
    /// Caller provides the guard to control object lifetime.
    // NOTE: review
    pub fn create_bytecode_generator_function(
        &mut self,
        guard: &Guard<JsObject>,
        bc_func: BytecodeFunction,
    ) -> Gc<JsObject> {
        let length_key = PropertyKey::String(self.intern("length"));
        let name_key = PropertyKey::String(self.intern("name"));

        // Get name and param count from function_info
        let (func_name, param_count) = bc_func
            .chunk
            .function_info
            .as_ref()
            .map(|info| {
                let name = info
                    .name
                    .as_ref()
                    .map(|n| n.cheap_clone())
                    .unwrap_or_else(|| JsString::from(""));
                (name, info.param_count)
            })
            .unwrap_or_else(|| (JsString::from(""), 0));

        let func_obj = guard.alloc();
        {
            let mut f_ref = func_obj.borrow_mut();
            f_ref.prototype = Some(self.function_prototype.clone());
            f_ref.exotic = ExoticObject::Function(JsFunction::BytecodeGenerator(bc_func));
            f_ref.set_property(length_key, JsValue::Number(param_count as f64));
            f_ref.set_property(name_key, JsValue::String(func_name));
        }
        func_obj
    }

    /// Create a bytecode async function object.
    /// Caller provides the guard to control object lifetime.
    // NOTE: review
    pub fn create_bytecode_async_function(
        &mut self,
        guard: &Guard<JsObject>,
        bc_func: BytecodeFunction,
    ) -> Gc<JsObject> {
        let length_key = PropertyKey::String(self.intern("length"));
        let name_key = PropertyKey::String(self.intern("name"));

        // Get name and param count from function_info
        let (func_name, param_count) = bc_func
            .chunk
            .function_info
            .as_ref()
            .map(|info| {
                let name = info
                    .name
                    .as_ref()
                    .map(|n| n.cheap_clone())
                    .unwrap_or_else(|| JsString::from(""));
                (name, info.param_count)
            })
            .unwrap_or_else(|| (JsString::from(""), 0));

        let func_obj = guard.alloc();
        {
            let mut f_ref = func_obj.borrow_mut();
            f_ref.prototype = Some(self.function_prototype.clone());
            f_ref.exotic = ExoticObject::Function(JsFunction::BytecodeAsync(bc_func));
            f_ref.set_property(length_key, JsValue::Number(param_count as f64));
            f_ref.set_property(name_key, JsValue::String(func_name));
        }
        func_obj
    }

    /// Create a bytecode async generator function object.
    /// Caller provides the guard to control object lifetime.
    pub fn create_bytecode_async_generator_function(
        &mut self,
        guard: &Guard<JsObject>,
        bc_func: BytecodeFunction,
    ) -> Gc<JsObject> {
        let length_key = PropertyKey::String(self.intern("length"));
        let name_key = PropertyKey::String(self.intern("name"));

        // Get name and param count from function_info
        let (func_name, param_count) = bc_func
            .chunk
            .function_info
            .as_ref()
            .map(|info| {
                let name = info
                    .name
                    .as_ref()
                    .map(|n| n.cheap_clone())
                    .unwrap_or_else(|| JsString::from(""));
                (name, info.param_count)
            })
            .unwrap_or_else(|| (JsString::from(""), 0));

        let func_obj = guard.alloc();
        {
            let mut f_ref = func_obj.borrow_mut();
            f_ref.prototype = Some(self.function_prototype.clone());
            f_ref.exotic = ExoticObject::Function(JsFunction::BytecodeAsyncGenerator(bc_func));
            f_ref.set_property(length_key, JsValue::Number(param_count as f64));
            f_ref.set_property(name_key, JsValue::String(func_name));
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

    /// Convert a JsValue to its string representation using interned strings
    /// for common values (undefined, null, true, false).
    pub fn to_js_string(&mut self, value: &JsValue) -> JsString {
        match value {
            JsValue::Undefined => self.intern("undefined"),
            JsValue::Null => self.intern("null"),
            JsValue::Boolean(true) => self.intern("true"),
            JsValue::Boolean(false) => self.intern("false"),
            JsValue::Number(n) => JsString::from(crate::value::number_to_string(*n)),
            JsValue::String(s) => s.cheap_clone(),
            JsValue::Symbol(s) => match &s.description {
                Some(desc) => JsString::from(format!("Symbol({})", desc.as_str())),
                None => self.intern("Symbol()"),
            },
            JsValue::Object(obj) => {
                let borrowed = obj.borrow();
                match &borrowed.exotic {
                    ExoticObject::Number(n) => JsString::from(crate::value::number_to_string(*n)),
                    ExoticObject::StringObj(s) => s.cheap_clone(),
                    ExoticObject::Boolean(b) => {
                        if *b {
                            self.intern("true")
                        } else {
                            self.intern("false")
                        }
                    }
                    ExoticObject::Array { elements } => {
                        let strings: Vec<String> = elements
                            .iter()
                            .map(|v| match v {
                                JsValue::Null | JsValue::Undefined => String::new(),
                                JsValue::String(s) => s.to_string(),
                                JsValue::Number(n) => crate::value::number_to_string(*n),
                                JsValue::Boolean(true) => "true".to_string(),
                                JsValue::Boolean(false) => "false".to_string(),
                                _ => "[object Object]".to_string(),
                            })
                            .collect();
                        JsString::from(strings.join(","))
                    }
                    _ => self.intern("[object Object]"),
                }
            }
        }
    }

    /// Get the typeof result for a value as an interned string.
    pub fn type_of(&mut self, value: &JsValue) -> JsString {
        match value {
            JsValue::Undefined => self.intern("undefined"),
            JsValue::Null => self.intern("object"), // Historical quirk
            JsValue::Boolean(_) => self.intern("boolean"),
            JsValue::Number(_) => self.intern("number"),
            JsValue::String(_) => self.intern("string"),
            JsValue::Symbol(_) => self.intern("symbol"),
            JsValue::Object(obj) => {
                if obj.borrow().is_callable() {
                    self.intern("function")
                } else {
                    self.intern("object")
                }
            }
        }
    }

    /// Create a PropertyKey from a string, using interned strings.
    pub fn property_key(&mut self, s: &str) -> PropertyKey {
        // Fast path: check if it's an array index
        if let Some(first) = s.bytes().next() {
            if first.is_ascii_digit() {
                if let Ok(idx) = s.parse::<u32>() {
                    if idx.to_string() == s {
                        return PropertyKey::Index(idx);
                    }
                }
            }
        }
        PropertyKey::String(self.intern(s))
    }

    /// Create a PropertyKey from an already-interned JsString.
    pub fn property_key_from_js_string(&mut self, s: JsString) -> PropertyKey {
        // Fast path: check if it's an array index
        if let Some(first) = s.as_str().bytes().next() {
            if first.is_ascii_digit() {
                if let Ok(idx) = s.parse::<u32>() {
                    if idx.to_string() == s.as_str() {
                        return PropertyKey::Index(idx);
                    }
                }
            }
        }
        PropertyKey::String(s)
    }

    /// Create a PropertyKey from a JsValue.
    pub fn property_key_from_value(&mut self, value: &JsValue) -> PropertyKey {
        match value {
            JsValue::Number(n) => {
                let idx = *n as u32;
                if idx as f64 == *n && *n >= 0.0 {
                    PropertyKey::Index(idx)
                } else {
                    PropertyKey::String(self.to_js_string(value))
                }
            }
            JsValue::String(s) => self.property_key_from_js_string(s.cheap_clone()),
            JsValue::Symbol(s) => PropertyKey::Symbol(s.clone()),
            _ => PropertyKey::String(self.to_js_string(value)),
        }
    }

    /// Create a native function object, permanently rooted via `root_guard`.
    /// Use this for builtin constructors and methods during initialization.
    /// The function is permanently rooted and never collected.
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
            JsFunction::Native(f) => (f.name.cheap_clone(), f.arity),
            JsFunction::Bound(b) => {
                // Bound functions: compute name and length from target
                let (target_name, target_length) =
                    if let ExoticObject::Function(target_func) = &b.target.borrow().exotic {
                        match target_func {
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

    /// Register a method on an object (for builtin initialization).
    /// Uses root_guard internally - functions are permanently rooted.
    /// Per ECMAScript spec, builtin methods are: writable, non-enumerable, configurable
    pub fn register_method(
        &mut self,
        obj: &Gc<JsObject>,
        name: &str,
        func: NativeFn,
        arity: usize,
    ) {
        let func_obj = self.create_native_function(name, func, arity);
        let key = PropertyKey::String(self.intern(name));
        // Builtin methods: writable=true, enumerable=false, configurable=true
        let prop = Property::with_attributes(JsValue::Object(func_obj), true, false, true);
        obj.borrow_mut().define_property(key, prop);
    }

    /// Register Symbol.species getter on a constructor.
    /// Per ECMAScript spec, Symbol.species is a getter that returns `this`.
    /// Uses root_guard internally - the getter is permanently rooted.
    /// The property is non-enumerable and configurable.
    pub fn register_species_getter(&mut self, constructor: &Gc<JsObject>) {
        // Create the getter function that returns `this`
        let getter = self.create_native_function("get [Symbol.species]", species_getter, 0);

        // Get the well-known Symbol.species
        let well_known = self.well_known_symbols;
        let species_symbol = JsSymbol::new(well_known.species, Some(self.intern("Symbol.species")));
        let species_key = PropertyKey::Symbol(Box::new(species_symbol));

        // Create accessor property (no setter, non-enumerable, configurable)
        let mut prop = Property::accessor(Some(getter), None);
        prop.set_enumerable(false);
        prop.set_configurable(true);

        constructor.borrow_mut().define_property(species_key, prop);
    }

    /// Create a guard for a value to prevent it from being garbage collected.
    /// Returns Some(guard) if the value is an object, None for primitives.
    ///
    /// Use this to guard input values before operations that may trigger GC.
    /// The returned guard must be kept alive for the duration needed.
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
    // Generator Support
    // ═══════════════════════════════════════════════════════════════════════════

    /// Resume a bytecode generator from a suspended state
    pub fn resume_bytecode_generator(
        &mut self,
        gen_state: &Rc<RefCell<BytecodeGeneratorState>>,
    ) -> Result<Guarded, JsError> {
        use bytecode_vm::{BytecodeVM, VmResult};

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

        // Get generator info
        let (
            started,
            sent_value,
            saved_ip,
            saved_registers,
            saved_call_stack,
            saved_try_stack,
            chunk,
            yield_result_register,
            closure,
            args,
            func_env,
            current_env,
            this_value,
        ) = {
            let state = gen_state.borrow();
            (
                state.started,
                state.sent_value.clone(),
                state.saved_ip,
                state.saved_registers.clone(),
                state.saved_call_stack.clone(),
                state.saved_try_stack.clone(),
                state.chunk.clone(),
                state.yield_result_register,
                state.closure.clone(),
                state.args.clone(),
                state.func_env.clone(),
                state.current_env.clone(),
                state.this_value.clone(),
            )
        };

        // Save current environment
        let saved_env = self.env.cheap_clone();

        // For first call, create a new function environment from the closure
        // For subsequent calls, use the saved current environment (which may include block scopes)
        let (gen_env, env_guard) = if !started {
            // Create a new environment with closure as parent
            let (new_env, guard) =
                create_environment_unrooted(&self.heap, Some(closure.cheap_clone()));
            // Save it for future calls
            gen_state.borrow_mut().func_env = Some(new_env.cheap_clone());
            (new_env, Some(guard))
        } else if let Some(env) = current_env {
            // Use the saved current environment (includes any block scopes from yield point)
            (env, None)
        } else if let Some(env) = func_env {
            // Fallback to function environment
            (env, None)
        } else {
            // Fallback: create new environment (shouldn't happen)
            let (new_env, guard) =
                create_environment_unrooted(&self.heap, Some(closure.cheap_clone()));
            gen_state.borrow_mut().func_env = Some(new_env.cheap_clone());
            (new_env, Some(guard))
        };

        // Set the generator's environment as the current environment
        self.env = gen_env;
        if let Some(guard) = env_guard {
            self.push_env_guard(guard);
        }

        let vm_guard = self.heap.create_guard();

        // Run the generator
        let result = if !started {
            // First call - create a new VM and run from the beginning
            gen_state.borrow_mut().started = true;

            // Handle rest parameters for generators too
            let processed_args: Vec<JsValue> = if let Some(rest_idx) = chunk
                .function_info
                .as_ref()
                .and_then(|info| info.rest_param)
            {
                let mut result_args = Vec::with_capacity(rest_idx + 1);
                for i in 0..rest_idx {
                    result_args.push(args.get(i).cloned().unwrap_or(JsValue::Undefined));
                }
                let rest_elements: Vec<JsValue> = args.get(rest_idx..).unwrap_or_default().to_vec();
                let rest_array = self.create_array_from(&vm_guard, rest_elements);
                result_args.push(JsValue::Object(rest_array));
                result_args
            } else {
                args.clone()
            };

            // Create VM with arguments and the original this value
            let mut vm = BytecodeVM::with_guard_and_args(
                chunk,
                this_value.clone(),
                vm_guard,
                &processed_args,
            );

            match vm.run(self) {
                VmResult::Complete(guarded) => {
                    // Generator completed normally
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    self.env = saved_env;
                    Ok(builtins::create_generator_result(self, guarded.value, true))
                }
                VmResult::Yield(yield_result) => {
                    // Save the VM state and current environment for resumption
                    {
                        let mut state = gen_state.borrow_mut();
                        state.saved_ip = yield_result.state.ip;
                        state.saved_registers = yield_result.state.registers;
                        state.saved_call_stack = yield_result.state.frames;
                        state.saved_try_stack = yield_result.state.try_stack;
                        state.yield_result_register = Some(yield_result.resume_register);
                        // Save current environment (may include block scopes)
                        state.current_env = Some(self.env.cheap_clone());
                    }
                    self.env = saved_env;
                    Ok(builtins::create_generator_result(
                        self,
                        yield_result.value.value, // Extract JsValue from Guarded
                        false,
                    ))
                }
                VmResult::YieldStar(yield_star_result) => {
                    // For yield*, we need to iterate over the iterable
                    // Save state and start delegating
                    {
                        let mut state = gen_state.borrow_mut();
                        state.saved_ip = yield_star_result.state.ip;
                        state.saved_registers = yield_star_result.state.registers;
                        state.saved_call_stack = yield_star_result.state.frames;
                        state.saved_try_stack = yield_star_result.state.try_stack;
                        state.yield_result_register = Some(yield_star_result.resume_register);
                        // Save current environment (may include block scopes)
                        state.current_env = Some(self.env.cheap_clone());
                    }
                    // Delegate to the iterable - get its iterator and next value
                    self.start_yield_star_delegation(
                        gen_state,
                        yield_star_result.iterable.value, // Extract JsValue from Guarded
                        saved_env,
                    )
                }
                VmResult::Suspend(_) => {
                    // Should not happen for generators
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    self.env = saved_env;
                    Err(JsError::internal_error(
                        "Unexpected suspension in generator",
                    ))
                }
                VmResult::Error(e) => {
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    self.env = saved_env;
                    Err(e)
                }
            }
        } else {
            // Resume from saved state
            // Create guard for the saved state
            let state_guard = self.heap.create_guard();

            // Guard all objects in saved registers
            for val in &saved_registers {
                if let JsValue::Object(obj) = val {
                    state_guard.guard(obj.cheap_clone());
                }
            }

            // Guard saved environments in call frames
            for frame in &saved_call_stack {
                if let Some(ref env) = frame.saved_env {
                    state_guard.guard(env.cheap_clone());
                }
            }

            let saved_state = bytecode_vm::SavedVmState {
                frames: saved_call_stack,
                ip: saved_ip,
                chunk,
                registers: saved_registers,
                try_stack: saved_try_stack,
                guard: Some(state_guard),
                arguments: args.clone(),
                new_target: JsValue::Undefined,
            };

            // Create guard for the VM registers
            let vm_guard = self.heap.create_guard();
            let mut vm = BytecodeVM::from_saved_state(saved_state, this_value.clone(), vm_guard);

            // Check if we need to throw an exception (generator.throw())
            let throw_value = gen_state.borrow_mut().throw_value.take();
            if let Some(exception) = throw_value {
                // Inject the exception - if there's a handler, it will jump to catch
                // If no handler, the exception will propagate
                if !vm.inject_exception(self, exception.clone()) {
                    // No exception handler found, propagate the error
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    self.env = saved_env;
                    let guarded = Guarded::from_value(exception, &self.heap);
                    return Err(JsError::ThrownValue { guarded });
                }
                // Handler found - continue to run the VM which will execute the catch block
            } else {
                // Normal resume - set the sent value in the yield result register
                if let Some(resume_reg) = yield_result_register {
                    vm.set_reg(resume_reg, sent_value);
                }
            }

            match vm.run(self) {
                VmResult::Complete(guarded) => {
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    self.env = saved_env;
                    Ok(builtins::create_generator_result(self, guarded.value, true))
                }
                VmResult::Yield(yield_result) => {
                    // Save the VM state and current environment for next resumption
                    {
                        let mut state = gen_state.borrow_mut();
                        state.saved_ip = yield_result.state.ip;
                        state.saved_registers = yield_result.state.registers;
                        state.saved_call_stack = yield_result.state.frames;
                        state.saved_try_stack = yield_result.state.try_stack;
                        state.yield_result_register = Some(yield_result.resume_register);
                        // Save current environment (may include block scopes)
                        state.current_env = Some(self.env.cheap_clone());
                    }
                    self.env = saved_env;
                    Ok(builtins::create_generator_result(
                        self,
                        yield_result.value.value, // Extract JsValue from Guarded
                        false,
                    ))
                }
                VmResult::YieldStar(yield_star_result) => {
                    // Save state for yield* delegation
                    {
                        let mut state = gen_state.borrow_mut();
                        state.saved_ip = yield_star_result.state.ip;
                        state.saved_registers = yield_star_result.state.registers;
                        state.saved_call_stack = yield_star_result.state.frames;
                        state.saved_try_stack = yield_star_result.state.try_stack;
                        state.yield_result_register = Some(yield_star_result.resume_register);
                        // Save current environment (may include block scopes)
                        state.current_env = Some(self.env.cheap_clone());
                    }
                    self.start_yield_star_delegation(
                        gen_state,
                        yield_star_result.iterable.value, // Extract JsValue from Guarded
                        saved_env,
                    )
                }
                VmResult::Suspend(_) => {
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    self.env = saved_env;
                    Err(JsError::internal_error(
                        "Unexpected suspension in generator",
                    ))
                }
                VmResult::Error(e) => {
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    self.env = saved_env;
                    Err(e)
                }
            }
        };

        result
    }

    /// Start yield* delegation - get the first value from the iterable
    fn start_yield_star_delegation(
        &mut self,
        gen_state: &Rc<RefCell<BytecodeGeneratorState>>,
        iterable: JsValue,
        saved_env: Gc<JsObject>,
    ) -> Result<Guarded, JsError> {
        // For yield*, we need to:
        // 1. Get the iterator from the iterable
        // 2. Store it in the generator state for delegation
        // 3. Call next() to get the first value
        // 4. On subsequent next() calls, delegate to the stored iterator

        let is_async = gen_state.borrow().is_async;

        // Try to get Symbol.iterator method
        let JsValue::Object(obj) = &iterable else {
            self.env = saved_env;
            return Err(JsError::type_error("yield* value is not iterable"));
        };

        // Get Symbol.iterator (for async generators, also try Symbol.asyncIterator)
        let well_known = self.well_known_symbols;

        // Create a guard to keep the iterator and its contents alive throughout delegation
        let iter_guard = self.heap.create_guard();

        // Try to get the iterator method - for async generators, prefer Symbol.asyncIterator
        let sym_iterator = self.intern("Symbol.iterator");
        let sym_async_iterator = self.intern("Symbol.asyncIterator");
        let iterator_method = if is_async {
            // Try Symbol.asyncIterator first
            let async_iterator_symbol =
                JsSymbol::new(well_known.async_iterator, Some(sym_async_iterator));
            let async_iterator_key = PropertyKey::Symbol(Box::new(async_iterator_symbol));
            let async_method = obj.borrow().get_property(&async_iterator_key);

            if async_method.is_some() {
                async_method
            } else {
                // Fall back to Symbol.iterator
                let iterator_symbol =
                    JsSymbol::new(well_known.iterator, Some(sym_iterator.cheap_clone()));
                let iterator_key = PropertyKey::Symbol(Box::new(iterator_symbol));
                obj.borrow().get_property(&iterator_key)
            }
        } else {
            let iterator_symbol = JsSymbol::new(well_known.iterator, Some(sym_iterator));
            let iterator_key = PropertyKey::Symbol(Box::new(iterator_symbol));
            obj.borrow().get_property(&iterator_key)
        };

        let (iter_obj, next_method) = match iterator_method {
            Some(method) => {
                // Call the iterator method to get an iterator object
                let iter_result = self.call_function(method, iterable.clone(), &[])?;

                // Get next method from iterator
                let JsValue::Object(iter_obj) = iter_result.value else {
                    self.env = saved_env;
                    return Err(JsError::type_error("Iterator is not an object"));
                };

                // Guard the iterator object to keep it and its properties alive
                iter_guard.guard(iter_obj.cheap_clone());

                let next_key = self.property_key("next");
                let next_method = iter_obj
                    .borrow()
                    .get_property(&next_key)
                    .ok_or_else(|| JsError::type_error("Iterator has no next method"))?;

                (iter_obj, next_method)
            }
            None => {
                // Check if it's already an iterator (has .next())
                let next_key = self.property_key("next");
                if let Some(next_method) = obj.borrow().get_property(&next_key) {
                    iter_guard.guard(obj.cheap_clone());
                    (obj.cheap_clone(), next_method)
                } else {
                    self.env = saved_env;
                    return Err(JsError::type_error("yield* value is not iterable"));
                }
            }
        };

        // Call next() to get the first value
        let result = self.call_function(
            next_method.clone(),
            JsValue::Object(iter_obj.cheap_clone()),
            &[],
        )?;
        // Keep guard alive until the iterator is stored in generator state
        let _ = &iter_guard;

        // For async generators, the result might be a Promise - resolve it synchronously
        let actual_result = if is_async {
            if let JsValue::Object(result_obj) = &result.value {
                if matches!(result_obj.borrow().exotic, ExoticObject::Promise(_)) {
                    builtins::promise::resolve_promise_sync(self, result_obj)?
                } else {
                    result.value.clone()
                }
            } else {
                result.value.clone()
            }
        } else {
            result.value.clone()
        };

        // Extract value and done
        let (value, done) = self.extract_iterator_result(&actual_result);

        if done {
            // Iterator is immediately done, resume generator with return value
            gen_state.borrow_mut().sent_value = value;
            gen_state.borrow_mut().status = GeneratorStatus::Suspended;
            self.env = saved_env;
            // Resume the generator to continue after yield*
            self.resume_bytecode_generator(gen_state)
        } else {
            // Store the delegated iterator for future next() calls
            gen_state.borrow_mut().delegated_iterator = Some((iter_obj, next_method));
            gen_state.borrow_mut().status = GeneratorStatus::Suspended;
            self.env = saved_env;
            Ok(builtins::create_generator_result(self, value, false))
        }
    }

    /// Extract value and done from an iterator result object
    fn extract_iterator_result(&mut self, result: &JsValue) -> (JsValue, bool) {
        let JsValue::Object(obj) = result else {
            return (JsValue::Undefined, true);
        };

        let value_key = self.property_key("value");
        let done_key = self.property_key("done");

        let value = obj
            .borrow()
            .get_property(&value_key)
            .unwrap_or(JsValue::Undefined);

        let done = match obj.borrow().get_property(&done_key) {
            Some(JsValue::Boolean(b)) => b,
            _ => false,
        };

        (value, done)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Evaluation Entry Point
    // ═══════════════════════════════════════════════════════════════════════════

    /// Evaluate simple TypeScript/JavaScript code (no imports, no async orders).
    ///
    /// This uses the bytecode VM for execution.
    pub fn eval_simple(&mut self, source: &str) -> Result<JsValue, JsError> {
        self.eval_bytecode(source)
    }

    /// Run bytecode using the bytecode VM
    ///
    /// This is the bytecode execution entry point. It compiles the program
    /// to bytecode and then executes it using the bytecode VM.
    pub fn run_bytecode(
        &mut self,
        chunk: Rc<crate::compiler::BytecodeChunk>,
    ) -> Result<Guarded, JsError> {
        self.run_bytecode_with_this(chunk, JsValue::Object(self.global.clone()))
    }

    /// Run bytecode using the bytecode VM with a specific `this` value
    fn run_bytecode_with_this(
        &mut self,
        chunk: Rc<crate::compiler::BytecodeChunk>,
        this_value: JsValue,
    ) -> Result<Guarded, JsError> {
        use bytecode_vm::{BytecodeVM, VmResult};

        let vm_guard = self.heap.create_guard();
        let mut vm = BytecodeVM::with_guard(chunk, this_value, vm_guard);

        match vm.run(self) {
            VmResult::Complete(guarded) => Ok(guarded),
            VmResult::Error(err) => Err(err),
            VmResult::Suspend(_) => Err(JsError::internal_error(
                "Bytecode execution cannot suspend at top level",
            )),
            VmResult::Yield(_) | VmResult::YieldStar(_) => Err(JsError::internal_error(
                "Bytecode execution cannot yield at top level",
            )),
        }
    }

    /// Compile and run source code using bytecode VM
    pub fn eval_bytecode(&mut self, source: &str) -> Result<JsValue, JsError> {
        use crate::compiler::Compiler;

        let mut parser = crate::parser::Parser::new(source, &mut self.string_dict);
        let program = parser.parse_program()?;
        let chunk = Compiler::compile_program(&program)?;
        let result = self.run_bytecode(chunk)?;
        Ok(result.value)
    }

    /// Execute a program (AST) using bytecode compilation
    fn execute_program_bytecode(
        &mut self,
        program: &crate::ast::Program,
    ) -> Result<JsValue, JsError> {
        use crate::compiler::Compiler;

        let chunk = Compiler::compile_program(program)?;
        let result = self.run_bytecode(chunk)?;
        Ok(result.value)
    }

    /// Execute a program (AST) for eval with proper completion value tracking
    pub fn execute_program_for_eval(
        &mut self,
        program: &crate::ast::Program,
    ) -> Result<JsValue, JsError> {
        use crate::compiler::Compiler;

        let chunk = Compiler::compile_program_for_eval(program)?;
        let result = self.run_bytecode(chunk)?;
        Ok(result.value)
    }

    /// Execute a program (AST) for eval with a specific `this` value
    pub fn execute_program_for_eval_with_this(
        &mut self,
        program: &crate::ast::Program,
        this_value: JsValue,
    ) -> Result<JsValue, JsError> {
        use crate::compiler::Compiler;

        let chunk = Compiler::compile_program_for_eval(program)?;
        let result = self.run_bytecode_with_this(chunk, this_value)?;
        Ok(result.value)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Module Resolution (used by stack-based execution)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Resolve a module specifier to a module namespace object.
    ///
    /// The specifier is resolved relative to the current module path before
    /// looking up in the loaded modules cache.
    pub fn resolve_module(&mut self, specifier: &str) -> Result<Gc<JsObject>, JsError> {
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

        // Set up import bindings before bytecode execution
        self.setup_import_bindings(&program)?;

        // Execute the module body using bytecode
        let result = self.execute_program_bytecode(&program);

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

    /// Create a function from an InternalFn.
    /// Uses root_guard internally - the function is permanently rooted.
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
    /// For Date objects with "default" hint, uses "string" per ES spec (Date.prototype[@@toPrimitive]).
    /// Throws TypeError if neither method returns a primitive value (ES2015+ spec).
    fn coerce_to_primitive(&mut self, value: &JsValue, hint: &str) -> Result<JsValue, JsError> {
        let obj = match value {
            JsValue::Object(obj) => obj,
            // Already primitive
            _ => return Ok(value.clone()),
        };

        // Per ES spec: Date objects prefer "string" for "default" hint
        // This is what Date.prototype[@@toPrimitive] does
        let effective_hint = if hint == "default" {
            // Check if this is a Date object
            if matches!(obj.borrow().exotic, ExoticObject::Date { .. }) {
                "string"
            } else {
                hint
            }
        } else {
            hint
        };

        // Determine method order based on hint
        let (first_method, second_method) = if effective_hint == "string" {
            ("toString", "valueOf")
        } else {
            // "number" - try valueOf first
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

        // Per ECMAScript spec: if neither method returns a primitive, throw TypeError
        Err(JsError::type_error(
            "Cannot convert object to primitive value",
        ))
    }

    /// Convert value to number, handling ToPrimitive for objects (ToNumber abstract operation).
    /// This properly calls the object's valueOf/toString methods per ECMAScript spec.
    pub fn coerce_to_number(&mut self, value: &JsValue) -> Result<f64, JsError> {
        match value {
            JsValue::Symbol(_) => Err(JsError::type_error(
                "Cannot convert a Symbol value to a number",
            )),
            JsValue::Object(_) => {
                let prim = self.coerce_to_primitive(value, "number")?;
                // The primitive could be a Symbol if valueOf returns one
                if matches!(prim, JsValue::Symbol(_)) {
                    return Err(JsError::type_error(
                        "Cannot convert a Symbol value to a number",
                    ));
                }
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
                Ok(self.to_js_string(&prim))
            }
            _ => Ok(self.to_js_string(value)),
        }
    }

    /// ToObject abstract operation (ES2015+).
    /// Converts primitives to their wrapper objects. Throws TypeError for null/undefined.
    /// Returns a `Guarded` to keep wrapper objects alive until the caller is done with them.
    pub fn to_object(&mut self, value: JsValue) -> Result<Guarded, JsError> {
        match value {
            JsValue::Object(obj) => Ok(Guarded::unguarded(JsValue::Object(obj))),
            JsValue::Undefined | JsValue::Null => Err(JsError::type_error(
                "Cannot convert undefined or null to object",
            )),
            JsValue::Boolean(b) => {
                // Create Boolean wrapper object
                let guard = self.heap.create_guard();
                let gc_obj = guard.alloc();
                {
                    let mut obj_ref = gc_obj.borrow_mut();
                    obj_ref.prototype = Some(self.boolean_prototype.cheap_clone());
                    obj_ref.exotic = ExoticObject::Boolean(b);
                }
                Ok(Guarded::with_guard(JsValue::Object(gc_obj), guard))
            }
            JsValue::Number(n) => {
                // Create Number wrapper object
                let guard = self.heap.create_guard();
                let gc_obj = guard.alloc();
                {
                    let mut obj_ref = gc_obj.borrow_mut();
                    obj_ref.prototype = Some(self.number_prototype.cheap_clone());
                    obj_ref.exotic = ExoticObject::Number(n);
                }
                Ok(Guarded::with_guard(JsValue::Object(gc_obj), guard))
            }
            JsValue::String(s) => {
                // Create String wrapper object
                let guard = self.heap.create_guard();
                let gc_obj = guard.alloc();
                {
                    let mut obj_ref = gc_obj.borrow_mut();
                    obj_ref.prototype = Some(self.string_prototype.cheap_clone());
                    obj_ref.exotic = ExoticObject::StringObj(s);
                }
                Ok(Guarded::with_guard(JsValue::Object(gc_obj), guard))
            }
            JsValue::Symbol(sym) => {
                // Create Symbol wrapper object - use ordinary object with symbol prototype
                // Store the symbol value in the exotic slot so it can be retrieved
                let guard = self.heap.create_guard();
                let gc_obj = guard.alloc();
                {
                    let mut obj_ref = gc_obj.borrow_mut();
                    obj_ref.prototype = Some(self.symbol_prototype.cheap_clone());
                    obj_ref.exotic = ExoticObject::Symbol(sym);
                }
                Ok(Guarded::with_guard(JsValue::Object(gc_obj), guard))
            }
        }
    }

    // NOTE: review
    // FIXME: pass guard
    pub fn call_function(
        &mut self,
        callee: JsValue,
        this_value: JsValue,
        args: &[JsValue],
    ) -> Result<Guarded, JsError> {
        self.call_function_with_new_target(callee, this_value, args, JsValue::Undefined)
    }

    /// Call a function with an explicit new.target value (for constructor calls)
    pub fn call_function_with_new_target(
        &mut self,
        callee: JsValue,
        this_value: JsValue,
        args: &[JsValue],
        new_target: JsValue,
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
            JsFunction::Native(native) => {
                // Call native function - propagate the Guarded to preserve guard
                (native.func)(self, this_value, args)
            }

            JsFunction::Bytecode(bc_func) => {
                // Call bytecode-compiled function using the bytecode VM
                self.call_bytecode_function_with_new_target(bc_func, this_value, args, new_target)
            }

            JsFunction::BytecodeGenerator(bc_func) => {
                // Create a new bytecode generator object
                self.create_and_call_bytecode_generator(bc_func, this_value, args)
            }

            JsFunction::BytecodeAsync(bc_func) => {
                // Call async function - runs body and wraps result in Promise
                self.call_bytecode_async_function(bc_func, this_value, args)
            }

            JsFunction::BytecodeAsyncGenerator(bc_func) => {
                // Create a new bytecode async generator object
                self.create_and_call_bytecode_async_generator(bc_func, this_value, args)
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

    /// Call a bytecode-compiled function
    fn call_bytecode_function(
        &mut self,
        bc_func: BytecodeFunction,
        this_value: JsValue,
        args: &[JsValue],
    ) -> Result<Guarded, JsError> {
        self.call_bytecode_function_with_new_target(bc_func, this_value, args, JsValue::Undefined)
    }

    /// Call a bytecode-compiled function with an explicit new.target value
    // NOTE: review
    fn call_bytecode_function_with_new_target(
        &mut self,
        bc_func: BytecodeFunction,
        this_value: JsValue,
        args: &[JsValue],
        new_target: JsValue,
    ) -> Result<Guarded, JsError> {
        use crate::interpreter::bytecode_vm::{BytecodeVM, VmResult};

        // Get function info from the chunk
        let func_info = bc_func.chunk.function_info.as_ref();

        // Push call stack frame for stack traces
        let func_name = func_info
            .and_then(|info| info.name.as_ref())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "<anonymous>".to_string());

        self.call_stack.push(StackFrame {
            function_name: func_name,
            location: None,
        });

        // Create new environment for the function, with closure as parent
        let (func_env, func_guard) =
            create_environment_unrooted(&self.heap, Some(bc_func.closure.cheap_clone()));

        // Bind `this` in the function environment
        // For arrow functions, use captured_this; otherwise use provided this_value
        let effective_this = if let Some(captured) = bc_func.captured_this {
            *captured
        } else {
            this_value.clone()
        };

        {
            let this_name = self.intern("this");
            if let Some(data) = func_env.borrow_mut().as_environment_mut() {
                data.bindings.insert(
                    VarKey(this_name),
                    Binding {
                        value: effective_this.clone(),
                        mutable: false,
                        initialized: true,
                        import_binding: None,
                    },
                );
            }
        }

        // Set up environment for execution
        let saved_env = self.env.cheap_clone();
        self.env = func_env;
        self.push_env_guard(func_guard);

        // Handle rest parameters: if the function has a rest parameter, we need to
        // collect all extra arguments into an array at that parameter index
        let vm_guard = self.heap.create_guard();
        let processed_args: Vec<JsValue> =
            if let Some(rest_idx) = func_info.and_then(|info| info.rest_param) {
                let mut result_args = Vec::with_capacity(rest_idx + 1);

                // Copy regular parameters
                for i in 0..rest_idx {
                    result_args.push(args.get(i).cloned().unwrap_or(JsValue::Undefined));
                }

                // Collect remaining args into an array for the rest parameter
                let rest_elements: Vec<JsValue> = args.get(rest_idx..).unwrap_or_default().to_vec();
                let rest_array = self.create_array_from(&vm_guard, rest_elements);
                result_args.push(JsValue::Object(rest_array));

                result_args
            } else {
                args.to_vec()
            };

        // Create VM and run with args pre-populated in registers
        // Use new_target if provided (for constructor calls)
        let mut vm = BytecodeVM::with_guard_args_and_new_target(
            bc_func.chunk.clone(),
            effective_this,
            vm_guard,
            &processed_args,
            new_target,
        );

        let result = vm.run(self);

        // Restore environment
        self.pop_env_guard();
        self.env = saved_env;
        self.call_stack.pop();

        // Convert VM result to Guarded
        match result {
            VmResult::Complete(guarded) => Ok(guarded),
            VmResult::Error(e) => Err(e),
            VmResult::Suspend(_) => Err(JsError::internal_error(
                "Bytecode function suspended unexpectedly",
            )),
            VmResult::Yield(_) | VmResult::YieldStar(_) => Err(JsError::internal_error(
                "Bytecode function yielded unexpectedly",
            )),
        }
    }

    /// Call a bytecode async function - wraps result in Promise
    // NOTE: review
    fn call_bytecode_async_function(
        &mut self,
        bc_func: BytecodeFunction,
        this_value: JsValue,
        args: &[JsValue],
    ) -> Result<Guarded, JsError> {
        // Execute the function body synchronously
        let body_result = self.call_bytecode_function(bc_func, this_value, args);

        // Wrap result in Promise (fulfilled or rejected)
        match body_result {
            Ok(guarded) => {
                // Promise assimilation: if result is already a Promise, return it directly
                if let JsValue::Object(ref obj) = &guarded.value {
                    if matches!(obj.borrow().exotic, ExoticObject::Promise(_)) {
                        return Ok(guarded);
                    }
                }
                // Create fulfilled promise with the result
                let result = guarded.value;
                let guard = self.heap.create_guard();
                let promise = builtins::promise::create_fulfilled_promise(self, &guard, result);
                Ok(Guarded::with_guard(JsValue::Object(promise), guard))
            }
            Err(e) => {
                // Create rejected promise with the error
                let guard = self.heap.create_guard();
                let promise =
                    builtins::promise::create_rejected_promise(self, &guard, e.to_value());
                Ok(Guarded::with_guard(JsValue::Object(promise), guard))
            }
        }
    }

    /// Create a bytecode generator object when a generator function is called
    // NOTE: review
    fn create_and_call_bytecode_generator(
        &mut self,
        bc_func: BytecodeFunction,
        this_value: JsValue,
        args: &[JsValue],
    ) -> Result<Guarded, JsError> {
        use crate::value::{BytecodeGeneratorState, GeneratorStatus};

        // Generate a unique ID for this generator
        let gen_id = self.next_generator_id;
        self.next_generator_id = self.next_generator_id.wrapping_add(1);

        // Create the generator state with arguments
        let state = BytecodeGeneratorState {
            chunk: bc_func.chunk,
            closure: bc_func.closure,
            args: args.to_vec(),
            this_value,
            status: GeneratorStatus::Suspended,
            sent_value: JsValue::Undefined,
            id: gen_id,
            started: false,
            saved_ip: 0,
            saved_registers: Vec::new(),
            saved_call_stack: Vec::new(),
            saved_try_stack: Vec::new(),
            yield_result_register: None,
            func_env: None,           // Will be created on first call to next()
            current_env: None,        // Will be saved at each yield point
            delegated_iterator: None, // For yield* delegation
            is_async: false,          // Regular generator, not async
            throw_value: None,        // For generator.throw()
        };

        // Create the generator object with a guard
        let guard = self.heap.create_guard();
        let gen_obj = builtins::generator::create_bytecode_generator_object(self, &guard, state);

        Ok(Guarded::with_guard(JsValue::Object(gen_obj), guard))
    }

    /// Create a bytecode async generator object when an async generator function is called
    // NOTE: review
    fn create_and_call_bytecode_async_generator(
        &mut self,
        bc_func: BytecodeFunction,
        this_value: JsValue,
        args: &[JsValue],
    ) -> Result<Guarded, JsError> {
        use crate::value::{BytecodeGeneratorState, GeneratorStatus};

        // Generate a unique ID for this generator
        let gen_id = self.next_generator_id;
        self.next_generator_id = self.next_generator_id.wrapping_add(1);

        // Create the generator state with arguments (same as regular generator but with is_async=true)
        let state = BytecodeGeneratorState {
            chunk: bc_func.chunk,
            closure: bc_func.closure,
            args: args.to_vec(),
            this_value,
            status: GeneratorStatus::Suspended,
            sent_value: JsValue::Undefined,
            id: gen_id,
            started: false,
            saved_ip: 0,
            saved_registers: Vec::new(),
            saved_call_stack: Vec::new(),
            saved_try_stack: Vec::new(),
            yield_result_register: None,
            func_env: None,           // Will be created on first call to next()
            current_env: None,        // Will be saved at each yield point
            delegated_iterator: None, // For yield* delegation
            is_async: true,           // Async generator - next() returns Promise
            throw_value: None,        // For generator.throw()
        };

        // Create the generator object with a guard
        let guard = self.heap.create_guard();
        let gen_obj = builtins::generator::create_bytecode_generator_object(self, &guard, state);

        Ok(Guarded::with_guard(JsValue::Object(gen_obj), guard))
    }

    /// Collect all values from an iterable using the Symbol.iterator protocol.
    /// Returns a Vec of values if the object is iterable, or None if it doesn't have Symbol.iterator.
    /// For arrays, this falls back to directly reading array elements for efficiency.
    pub fn collect_iterator_values(
        &mut self,
        value: &JsValue,
    ) -> Result<Option<Vec<JsValue>>, JsError> {
        let JsValue::Object(obj) = value else {
            // Strings are iterable but handled separately
            if let JsValue::String(s) = value {
                return Ok(Some(
                    s.as_str()
                        .chars()
                        .map(|c| JsValue::String(JsString::from(c.to_string())))
                        .collect(),
                ));
            }
            return Ok(None);
        };

        // First check if it's a plain array - use fast path
        {
            let obj_ref = obj.borrow();
            if let Some(elements) = obj_ref.array_elements() {
                return Ok(Some(elements.to_vec()));
            }
        }

        // Check for Symbol.iterator method
        let well_known = self.well_known_symbols;
        let iterator_symbol =
            JsSymbol::new(well_known.iterator, Some(self.intern("Symbol.iterator")));
        let iterator_key = PropertyKey::Symbol(Box::new(iterator_symbol));

        let iterator_method = {
            let obj_ref = obj.borrow();
            obj_ref.get_property(&iterator_key)
        };

        let Some(JsValue::Object(method_obj)) = iterator_method else {
            return Ok(None);
        };

        // Call the iterator method to get the iterator object
        let Guarded {
            value: iterator_obj,
            guard: _iter_guard,
        } = self.call_function(JsValue::Object(method_obj), value.clone(), &[])?;

        let JsValue::Object(iterator) = iterator_obj else {
            return Err(JsError::type_error("Symbol.iterator must return an object"));
        };

        // Iterate: call next() until done is true
        let mut values = Vec::new();
        let next_key = PropertyKey::String(self.intern("next"));

        loop {
            // Get the next method
            let next_method = {
                let iter_ref = iterator.borrow();
                iter_ref.get_property(&next_key)
            };

            let Some(JsValue::Object(next_fn)) = next_method else {
                return Err(JsError::type_error("Iterator must have a next method"));
            };

            // Call next()
            let Guarded {
                value: result,
                guard: _result_guard,
            } = self.call_function(
                JsValue::Object(next_fn),
                JsValue::Object(iterator.clone()),
                &[],
            )?;

            let JsValue::Object(result_obj) = result else {
                return Err(JsError::type_error("Iterator next() must return an object"));
            };

            // Check done property
            let done = {
                let result_ref = result_obj.borrow();
                let done_key = PropertyKey::String(self.intern("done"));
                result_ref
                    .get_property(&done_key)
                    .map(|v| v.to_boolean())
                    .unwrap_or(false)
            };

            if done {
                break;
            }

            // Get value property
            let iter_value = {
                let result_ref = result_obj.borrow();
                let value_key = PropertyKey::String(self.intern("value"));
                result_ref
                    .get_property(&value_key)
                    .unwrap_or(JsValue::Undefined)
            };

            values.push(iter_value);
        }

        Ok(Some(values))
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

/// Native getter for Symbol.species that returns `this`.
/// Per ECMAScript spec, Symbol.species getter returns the constructor itself.
fn species_getter(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    // Symbol.species getter simply returns `this`
    Ok(Guarded::unguarded(this))
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
        // Error should mention 'continue' and indicate it's invalid outside a loop
        assert!(
            err_str.contains("continue")
                && (err_str.contains("loop") || err_str.contains("Illegal")),
            "Error should mention 'continue' and 'loop' or 'Illegal': {}",
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
        // Error should mention 'break' and indicate it's invalid outside a loop/switch
        assert!(
            err_str.contains("break")
                && (err_str.contains("loop")
                    || err_str.contains("switch")
                    || err_str.contains("Illegal")),
            "Error should mention 'break' and 'loop/switch' or 'Illegal': {}",
            err_str
        );
    }
}
