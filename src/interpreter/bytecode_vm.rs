//! Bytecode Virtual Machine
//!
//! This module implements the bytecode interpreter that executes compiled bytecode.
//! It uses a register-based design with up to 256 virtual registers per call frame.

use crate::compiler::{BytecodeChunk, Constant, Op, Register};
use crate::error::JsError;
use crate::gc::{Gc, Guard};
use crate::value::{
    BytecodeFunction, CheapClone, ExoticObject, Guarded, JsFunction, JsObject, JsString, JsValue,
    Property, PropertyKey,
};
use std::rc::Rc;

use super::Interpreter;

/// Result of VM execution
pub enum VmResult {
    /// Execution completed with a value
    Complete(Guarded),
    /// Need to suspend for await/yield
    Suspend(VmSuspension),
    /// Generator yielded a value
    Yield(GeneratorYield),
    /// Generator yielded via yield*
    YieldStar(GeneratorYieldStar),
    /// Error occurred
    Error(JsError),
}

/// Generator yield result
pub struct GeneratorYield {
    /// The yielded value (guarded to keep alive during suspension)
    pub value: Guarded,
    /// Register to store the value passed to next() when resumed
    pub resume_register: Register,
    /// Saved VM state for resumption
    pub state: SavedVmState,
}

/// Generator yield* result
pub struct GeneratorYieldStar {
    /// The iterable to delegate to (guarded to keep alive during suspension)
    pub iterable: Guarded,
    /// Register to store the final value when delegation completes
    pub resume_register: Register,
    /// Saved VM state for resumption
    pub state: SavedVmState,
}

/// Suspension state for async/generator
pub struct VmSuspension {
    /// The promise or generator we're waiting on
    pub waiting_on: Gc<JsObject>,
    /// Saved VM state for resumption
    pub state: SavedVmState,
    /// Register to store the resume value (for await)
    pub resume_register: Register,
}

/// Saved VM state for suspension/resumption
pub struct SavedVmState {
    /// Saved call frames
    pub frames: Vec<CallFrame>,
    /// Current instruction pointer
    pub ip: usize,
    /// Current bytecode chunk
    pub chunk: Rc<BytecodeChunk>,
    /// Register values
    pub registers: Vec<JsValue>,
    /// Exception handlers
    pub try_stack: Vec<TryHandler>,
    /// Guard to keep all objects in registers alive during suspension
    pub guard: Option<Guard<JsObject>>,
    /// Original arguments for `arguments` object
    pub arguments: Vec<JsValue>,
    /// new.target value
    pub new_target: JsValue,
}

/// A call frame in the VM
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Return instruction pointer
    pub return_ip: usize,
    /// Return bytecode chunk
    pub return_chunk: Rc<BytecodeChunk>,
    /// Base register index in the register file
    pub registers_base: usize,
    /// Register to store return value
    pub return_register: Register,
    /// Saved environment for restoration on return
    pub saved_env: Option<Gc<JsObject>>,
}

/// Exception handler for try/catch/finally
#[derive(Debug, Clone)]
pub struct TryHandler {
    /// Instruction pointer for catch block
    pub catch_ip: usize,
    /// Instruction pointer for finally block (0 = no finally)
    pub finally_ip: usize,
    /// Register count at time of push (for stack unwinding)
    pub registers_snapshot: usize,
    /// Call frame depth at time of push
    pub frame_depth: usize,
    /// Scope depth at time of push (saved_env_stack.len())
    /// Used to pop any scopes that were entered in the try block when catching
    pub scope_depth: usize,
    /// Iterator register for for-of iterator close (None for regular try)
    /// When set, this is a PushIterTry handler that should close the iterator on exception
    pub iterator_reg: Option<Register>,
}

/// Pending completion to be executed after finally block
pub enum PendingCompletion {
    /// Return this value after finally completes
    Return(Guarded),
    /// Rethrow this exception after finally completes
    Throw(Guarded),
    /// Break to target after finally completes
    Break { target: usize, try_depth: u8 },
    /// Continue to target after finally completes
    Continue { target: usize, try_depth: u8 },
}

/// A saved VM frame for the trampoline call stack
/// This replaces Rust call stack recursion with an explicit stack
pub struct TrampolineFrame {
    /// Saved instruction pointer
    pub ip: usize,
    /// Saved bytecode chunk
    pub chunk: Rc<BytecodeChunk>,
    /// Saved registers
    pub registers: Vec<JsValue>,
    /// Saved this value
    pub this_value: JsValue,
    /// Saved VM call stack
    pub vm_call_stack: Vec<CallFrame>,
    /// Saved try handlers
    pub try_stack: Vec<TryHandler>,
    /// Saved exception value (guarded to keep it alive during finally block execution)
    pub exception_value: Option<Guarded>,
    /// Saved environment stack
    pub saved_env_stack: Vec<Gc<JsObject>>,
    /// Saved arguments
    pub arguments: Vec<JsValue>,
    /// Saved new.target
    pub new_target: JsValue,
    /// Saved current constructor
    pub current_constructor: Option<Gc<JsObject>>,
    /// Saved pending completion
    pub pending_completion: Option<PendingCompletion>,
    /// Register to store the return value in
    pub return_register: Register,
    /// Saved interpreter environment
    pub saved_interp_env: Gc<JsObject>,
    /// Guard that was protecting this frame's register values.
    /// This is ONLY for the saved `registers` - not for exception_value or other fields.
    pub register_guard: Guard<JsObject>,
    /// For construct calls: the new object to fall back to if constructor doesn't return an object
    pub construct_new_obj: Option<Gc<JsObject>>,
    /// For async function calls: wrap result in a Promise when returning
    pub is_async: bool,
}

/// The bytecode virtual machine
pub struct BytecodeVM {
    /// Current instruction pointer
    pub ip: usize,
    /// Current bytecode chunk being executed
    pub chunk: Rc<BytecodeChunk>,
    /// Register file
    pub registers: Vec<JsValue>,
    /// Guard keeping all register values alive.
    ///
    /// IMPORTANT: This guard is ONLY for values stored in `registers`.
    /// Do NOT use it for exception_value, OpResult values, PendingCompletion,
    /// or any other non-register storage. Those must have their own dedicated guards.
    register_guard: Guard<JsObject>,
    /// Call stack (return addresses)
    pub call_stack: Vec<CallFrame>,
    /// Exception handler stack
    pub try_stack: Vec<TryHandler>,
    /// Current `this` value
    this_value: JsValue,
    /// Current exception value (for catch blocks)
    /// Guarded to keep the exception alive during finally block execution
    exception_value: Option<Guarded>,
    /// Stack of saved environments for nested scope restoration
    saved_env_stack: Vec<Gc<JsObject>>,
    /// Original arguments array (for `arguments` object)
    pub arguments: Vec<JsValue>,
    /// `new.target` value (constructor if called with new, undefined otherwise)
    pub new_target: JsValue,
    /// Current constructor being executed (for super() lookups in derived classes)
    current_constructor: Option<Gc<JsObject>>,
    /// Pending completion to execute after finally block
    pending_completion: Option<PendingCompletion>,
    /// Trampoline call stack - replaces Rust recursion with explicit stack
    trampoline_stack: Vec<TrampolineFrame>,
    /// Pool of reusable register files to reduce allocation overhead
    register_pool: Vec<Vec<JsValue>>,
    /// Pool of reusable argument vectors to reduce allocation overhead
    arguments_pool: Vec<Vec<JsValue>>,
}

impl BytecodeVM {
    /// Create a new VM with a guard for keeping objects alive
    pub fn with_guard(
        chunk: Rc<BytecodeChunk>,
        this_value: JsValue,
        guard: Guard<JsObject>,
    ) -> Self {
        let register_count = chunk.register_count as usize;
        // Guard this_value if it's an object
        if let JsValue::Object(obj) = &this_value {
            guard.guard(obj.cheap_clone());
        }
        Self {
            ip: 0,
            chunk,
            registers: vec![JsValue::Undefined; register_count.max(1)],
            register_guard: guard,
            call_stack: Vec::new(),
            try_stack: Vec::new(),
            this_value,
            exception_value: None,
            saved_env_stack: Vec::new(),
            arguments: Vec::new(),
            new_target: JsValue::Undefined,
            current_constructor: None,
            pending_completion: None,
            trampoline_stack: Vec::new(),
            register_pool: Vec::new(),
            arguments_pool: Vec::new(),
        }
    }

    /// Create a new VM with a guard and pre-populated function arguments.
    /// Arguments are placed in registers 0, 1, 2, ... before execution starts.
    /// The bytecode's DeclareVar ops will read from these registers.
    pub fn with_guard_and_args(
        chunk: Rc<BytecodeChunk>,
        this_value: JsValue,
        guard: Guard<JsObject>,
        args: &[JsValue],
    ) -> Self {
        let register_count = chunk.register_count as usize;
        let mut registers = vec![JsValue::Undefined; register_count.max(1)];

        // Guard this_value if it's an object
        if let JsValue::Object(obj) = &this_value {
            guard.guard(obj.cheap_clone());
        }

        // Pre-populate registers with arguments
        for (i, arg) in args.iter().enumerate() {
            if i < registers.len() {
                if let Some(slot) = registers.get_mut(i) {
                    if let JsValue::Object(obj) = arg {
                        guard.guard(obj.cheap_clone());
                    }
                    *slot = arg.clone();
                }
            }
        }

        Self {
            ip: 0,
            chunk,
            registers,
            register_guard: guard,
            call_stack: Vec::new(),
            try_stack: Vec::new(),
            this_value,
            exception_value: None,
            saved_env_stack: Vec::new(),
            arguments: args.to_vec(),
            new_target: JsValue::Undefined,
            current_constructor: None,
            pending_completion: None,
            trampoline_stack: Vec::new(),
            register_pool: Vec::new(),
            arguments_pool: Vec::new(),
        }
    }

    /// Create a new VM with a guard, arguments, and new.target value.
    pub fn with_guard_args_and_new_target(
        chunk: Rc<BytecodeChunk>,
        this_value: JsValue,
        guard: Guard<JsObject>,
        args: &[JsValue],
        new_target: JsValue,
    ) -> Self {
        let register_count = chunk.register_count as usize;
        let mut registers = vec![JsValue::Undefined; register_count.max(1)];

        // Guard this_value if it's an object
        if let JsValue::Object(obj) = &this_value {
            guard.guard(obj.cheap_clone());
        }

        // Guard new_target if it's an object
        if let JsValue::Object(obj) = &new_target {
            guard.guard(obj.cheap_clone());
        }

        // Pre-populate registers with arguments
        for (i, arg) in args.iter().enumerate() {
            if i < registers.len() {
                if let Some(slot) = registers.get_mut(i) {
                    if let JsValue::Object(obj) = arg {
                        guard.guard(obj.cheap_clone());
                    }
                    *slot = arg.clone();
                }
            }
        }

        Self {
            ip: 0,
            chunk,
            registers,
            register_guard: guard,
            call_stack: Vec::new(),
            try_stack: Vec::new(),
            this_value,
            exception_value: None,
            saved_env_stack: Vec::new(),
            arguments: args.to_vec(),
            new_target,
            current_constructor: None,
            pending_completion: None,
            trampoline_stack: Vec::new(),
            register_pool: Vec::new(),
            arguments_pool: Vec::new(),
        }
    }

    /// Acquire a register file from the pool, or allocate a new one
    #[inline]
    fn acquire_registers(&mut self, size: usize) -> Vec<JsValue> {
        let size = size.max(1);
        // Try to find an existing frame of sufficient size
        if let Some(pos) = self.register_pool.iter().position(|f| f.capacity() >= size) {
            let mut frame = self.register_pool.swap_remove(pos);
            frame.clear();
            frame.resize(size, JsValue::Undefined);
            return frame;
        }
        // Allocate new frame
        vec![JsValue::Undefined; size]
    }

    /// Return a register file to the pool for reuse.
    /// Also clears the register_guard to remove stale roots that could waste
    /// GC cycles iterating over now-pooled objects.
    #[inline]
    fn release_registers(&mut self, mut registers: Vec<JsValue>) {
        // Clear register_guard BEFORE clearing registers - this removes the guard's
        // roots while the objects still have valid ref_counts from the registers Vec.
        // If we cleared registers first, the guard would have stale roots pointing
        // to potentially-pooled GcBoxes.
        self.register_guard.clear();
        // Clear the registers to drop any references
        registers.clear();
        // Keep pool size reasonable (e.g., max 16 frames)
        if self.register_pool.len() < 16 {
            self.register_pool.push(registers);
        }
    }

    /// Acquire an empty arguments vector with given capacity from pool
    #[inline]
    fn acquire_arguments_vec(&mut self, capacity: usize) -> Vec<JsValue> {
        if let Some(pos) = self
            .arguments_pool
            .iter()
            .position(|v| v.capacity() >= capacity)
        {
            let mut vec = self.arguments_pool.swap_remove(pos);
            vec.clear();
            return vec;
        }
        Vec::with_capacity(capacity)
    }

    /// Return an arguments vector to the pool for reuse
    #[inline]
    fn release_arguments(&mut self, mut args: Vec<JsValue>) {
        args.clear();
        // Keep pool size reasonable (max 16)
        if self.arguments_pool.len() < 16 {
            self.arguments_pool.push(args);
        }
    }

    /// Get register value
    #[inline]
    fn get_reg(&self, r: Register) -> &JsValue {
        self.registers
            .get(r as usize)
            .unwrap_or(&JsValue::Undefined)
    }

    /// Set register value
    #[inline]
    pub fn set_reg(&mut self, r: Register, value: JsValue) {
        let idx = r as usize;
        debug_assert!(
            idx < self.registers.len(),
            "Register index {} out of bounds (max {})",
            idx,
            self.registers.len()
        );
        if idx < self.registers.len() {
            if let Some(slot) = self.registers.get_mut(idx) {
                if let JsValue::Object(obj) = &value {
                    self.register_guard.guard(obj.clone());
                }
                if let JsValue::Object(obj) = &slot {
                    self.register_guard.unguard(obj);
                }
                *slot = value;
            }
        }
    }

    /// Fetch the next instruction and advance IP
    #[inline]
    fn fetch(&mut self) -> Option<Op> {
        let op = *self.chunk.get(self.ip)?;
        self.ip += 1;
        Some(op)
    }

    /// Get a constant from the pool
    #[inline]
    fn get_constant(&self, idx: u16) -> Option<&Constant> {
        self.chunk.get_constant(idx)
    }

    /// Get a string constant from the pool
    #[inline]
    fn get_string_constant(&self, idx: u16) -> Option<JsString> {
        match self.get_constant(idx)? {
            Constant::String(s) => Some(s.cheap_clone()),
            _ => None,
        }
    }

    /// Get the super constructor from the current function's __super__ property
    fn get_super_constructor(&self, interp: &mut Interpreter) -> Result<JsValue, JsError> {
        // Look up __super__ in the current function's properties
        let super_key = PropertyKey::String(interp.intern("__super__"));

        // First, check if we have a current_constructor set (for construct calls)
        // This is the most reliable way to find super() in derived class constructors
        if let Some(ref ctor) = self.current_constructor {
            if let Some(super_val) = ctor.borrow().get_property(&super_key) {
                return Ok(super_val);
            }
        }

        if let JsValue::Object(this_obj) = &self.this_value {
            // For static methods: `this` IS the class constructor, check directly on it
            if let Some(super_val) = this_obj.borrow().get_property(&super_key) {
                return Ok(super_val);
            }

            // For instance methods: look up constructor from prototype chain
            if let Some(proto) = &this_obj.borrow().prototype {
                if let Some(JsValue::Object(ctor_obj)) = proto
                    .borrow()
                    .get_property(&PropertyKey::String(interp.intern("constructor")))
                {
                    if let Some(super_val) = ctor_obj.borrow().get_property(&super_key) {
                        return Ok(super_val);
                    }
                }
            }
        }

        Err(JsError::syntax_error_simple(
            "'super' keyword is only valid inside a class method",
        ))
    }

    /// Get the super target object for super.x property access
    fn get_super_target(&self, interp: &mut Interpreter) -> Result<JsValue, JsError> {
        // First, try to look up __super_target__ from the current environment
        // This is set when entering a method via the trampoline
        let super_target_name = interp.intern("__super_target__");
        if let Ok(target) = interp.env_get(&super_target_name) {
            return Ok(target);
        }

        // Fallback: look up from this value's prototype chain (old behavior)
        let super_key = PropertyKey::String(interp.intern("__super__"));
        let super_target_key = PropertyKey::String(super_target_name);

        if let JsValue::Object(this_obj) = &self.this_value {
            // For static methods: `this` IS the class constructor
            // If `this` has __super__ directly, use __super__ (parent constructor)
            // as the target for looking up static methods
            if this_obj.borrow().get_property(&super_key).is_some() {
                // Static method context: super.x looks up on parent constructor
                if let Some(target) = this_obj.borrow().get_property(&super_key) {
                    return Ok(target);
                }
            }

            // For instance methods: look up constructor from prototype chain
            // and use __super_target__ (parent prototype) for property access
            if let Some(proto) = &this_obj.borrow().prototype {
                if let Some(JsValue::Object(ctor_obj)) = proto
                    .borrow()
                    .get_property(&PropertyKey::String(interp.intern("constructor")))
                {
                    if let Some(target) = ctor_obj.borrow().get_property(&super_target_key) {
                        return Ok(target);
                    }
                }
            }
        }

        Err(JsError::syntax_error_simple(
            "'super' keyword is only valid inside a class method",
        ))
    }

    /// Execute bytecode until completion, suspension, or error
    /// Uses a trampoline pattern to avoid Rust stack overflow on deep JS call chains
    pub fn run(&mut self, interp: &mut Interpreter) -> VmResult {
        loop {
            // Check timeout periodically
            if let Err(e) = interp.check_timeout() {
                return VmResult::Error(e);
            }

            let Some(op) = self.fetch() else {
                // End of bytecode - return last result or undefined
                let result = self
                    .registers
                    .first()
                    .cloned()
                    .unwrap_or(JsValue::Undefined);

                // Check if we have a trampoline frame to return to
                if let Some(frame) = self.trampoline_stack.pop() {
                    // Restore state from trampoline frame
                    self.restore_from_trampoline_frame(interp, frame, result);
                    continue;
                }

                let guard = interp.heap.create_guard();
                if let JsValue::Object(obj) = &result {
                    guard.guard(obj.cheap_clone());
                }
                return VmResult::Complete(Guarded {
                    value: result,
                    guard: Some(guard),
                });
            };

            match self.execute_op(interp, op) {
                Ok(OpResult::Continue) => continue,
                Ok(OpResult::Halt(value)) => {
                    // Check if we have a trampoline frame to return to
                    if let Some(frame) = self.trampoline_stack.pop() {
                        // Restore state from trampoline frame
                        self.restore_from_trampoline_frame(interp, frame, value.value);
                        continue;
                    }
                    return VmResult::Complete(value);
                }
                Ok(OpResult::Suspend {
                    promise,
                    resume_register,
                }) => {
                    // Extract the object from the guarded value
                    if let JsValue::Object(obj) = promise.value {
                        return VmResult::Suspend(VmSuspension {
                            waiting_on: obj,
                            state: self.save_state(interp),
                            resume_register,
                        });
                    } else {
                        return VmResult::Error(JsError::internal_error(
                            "Suspend expects an object",
                        ));
                    }
                }
                Ok(OpResult::Yield {
                    value,
                    resume_register,
                }) => {
                    return VmResult::Yield(GeneratorYield {
                        value, // Pass Guarded directly
                        resume_register,
                        state: self.save_state(interp),
                    });
                }
                Ok(OpResult::YieldStar {
                    iterable,
                    resume_register,
                }) => {
                    return VmResult::YieldStar(GeneratorYieldStar {
                        iterable, // Pass Guarded directly
                        resume_register,
                        state: self.save_state(interp),
                    });
                }
                Ok(OpResult::Call {
                    callee,
                    this_value,
                    args,
                    return_register,
                    new_target,
                    is_super_call,
                    guard: _guard, // Guard keeps values alive until trampoline frame is pushed
                }) => {
                    // Trampoline: save current state and switch to called function
                    match self.setup_trampoline_call(
                        interp,
                        callee,
                        this_value,
                        args,
                        return_register,
                        new_target,
                        is_super_call,
                    ) {
                        Ok(()) => continue,
                        Err(e) => {
                            // Try to find an exception handler, unwinding trampoline if needed
                            if let Err(e) = self.handle_error_with_trampoline_unwind(interp, e) {
                                return VmResult::Error(e);
                            }
                            continue;
                        }
                    }
                }
                Ok(OpResult::Construct {
                    callee,
                    this_value,
                    args,
                    return_register,
                    new_target,
                    new_obj,
                    guard: _guard, // Guard keeps values alive until trampoline frame is pushed
                }) => {
                    // Trampoline for construct: save current state and switch to constructor
                    match self.setup_trampoline_construct(
                        interp,
                        callee,
                        this_value,
                        args,
                        return_register,
                        new_target,
                        new_obj,
                    ) {
                        Ok(()) => continue,
                        Err(e) => {
                            // Try to find an exception handler, unwinding trampoline if needed
                            if let Err(e) = self.handle_error_with_trampoline_unwind(interp, e) {
                                return VmResult::Error(e);
                            }
                            continue;
                        }
                    }
                }
                Err(e) => {
                    // Try to find an exception handler, unwinding trampoline if needed
                    if let Err(e) = self.handle_error_with_trampoline_unwind(interp, e) {
                        return VmResult::Error(e);
                    }
                    continue;
                }
            }
        }
    }

    /// Set up a trampoline call - save current state and switch to the called function
    /// If `is_super_call` is true, the callee will be set as the current constructor for proper super() lookup
    fn setup_trampoline_call(
        &mut self,
        interp: &mut Interpreter,
        callee: JsValue,
        this_value: JsValue,
        args: Vec<JsValue>,
        return_register: Register,
        new_target: JsValue,
        is_super_call: bool,
    ) -> Result<(), JsError> {
        use crate::value::{ExoticObject, JsFunction};

        // Check call stack depth limit
        // Use only trampoline_stack.len() since that represents actual JS call depth
        // (interp.call_stack is also updated but for stack traces, so counting both would double-count)
        let total_depth = self.trampoline_stack.len();
        if interp.max_call_depth > 0 && total_depth >= interp.max_call_depth {
            return Err(JsError::range_error(format!(
                "Maximum call stack size exceeded (depth {})",
                total_depth
            )));
        }

        let JsValue::Object(func_obj) = &callee else {
            return Err(JsError::type_error("Not a function"));
        };

        // Check if this is a proxy
        let is_proxy = matches!(func_obj.borrow().exotic, ExoticObject::Proxy(_));
        if is_proxy {
            // For proxies, fall back to recursive call (they're rare)
            let result = crate::interpreter::builtins::proxy::proxy_apply(
                interp,
                func_obj.cheap_clone(),
                this_value,
                args,
            )?;
            self.set_reg(return_register, result.value);
            return Ok(());
        }

        let func = {
            let obj_ref = func_obj.borrow();
            match &obj_ref.exotic {
                ExoticObject::Function(f) => f.clone(),
                _ => return Err(JsError::type_error("Not a function")),
            }
        };

        match func {
            JsFunction::Bytecode(bc_func) => {
                // This is what we want to trampoline!
                self.push_trampoline_frame_and_call_bytecode(
                    interp,
                    func_obj.cheap_clone(),
                    bc_func,
                    this_value,
                    args,
                    return_register,
                    new_target,
                    false, // not async
                    is_super_call,
                )?;
                Ok(())
            }
            JsFunction::Native(native) => {
                // Native functions are quick, call directly
                let result = (native.func)(interp, this_value, &args)?;
                self.set_reg(return_register, result.value);
                Ok(())
            }
            JsFunction::Bound(bound) => {
                // Unwrap bound function and trampoline to target
                let target = JsValue::Object(bound.target.cheap_clone());
                let bound_this = bound.this_arg.clone();
                let mut full_args = bound.bound_args.clone();
                full_args.extend(args);
                self.setup_trampoline_call(
                    interp,
                    target,
                    bound_this,
                    full_args,
                    return_register,
                    new_target,
                    is_super_call, // pass through super call flag
                )
            }
            JsFunction::BytecodeGenerator(bc_func) => {
                // Generators just create a generator object without running the body.
                // The body runs when .next() is called. Handle directly without recursion.
                use crate::value::{BytecodeGeneratorState, GeneratorStatus};

                let gen_id = interp.next_generator_id;
                interp.next_generator_id = interp.next_generator_id.wrapping_add(1);

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
                    func_env: None,
                    current_env: None,
                    delegated_iterator: None,
                    is_async: false,
                    throw_value: None,
                };

                let gen_obj = super::builtins::generator::create_bytecode_generator_object(
                    interp,
                    &self.register_guard,
                    state,
                );
                self.set_reg(return_register, JsValue::Object(gen_obj));
                Ok(())
            }
            JsFunction::BytecodeAsyncGenerator(bc_func) => {
                // Async generators just create an async generator object without running the body.
                // The body runs when .next() is called. Handle directly without recursion.
                use crate::value::{BytecodeGeneratorState, GeneratorStatus};

                let gen_id = interp.next_generator_id;
                interp.next_generator_id = interp.next_generator_id.wrapping_add(1);

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
                    func_env: None,
                    current_env: None,
                    delegated_iterator: None,
                    is_async: true, // Async generator
                    throw_value: None,
                };

                let gen_obj = super::builtins::generator::create_bytecode_generator_object(
                    interp,
                    &self.register_guard,
                    state,
                );
                self.set_reg(return_register, JsValue::Object(gen_obj));
                Ok(())
            }
            JsFunction::BytecodeAsync(bc_func) => {
                // Async functions run their body and wrap result in Promise.
                // Use trampoline to run the body - the is_async flag causes the result
                // to be wrapped in a Promise when the frame is popped.
                self.push_trampoline_frame_and_call_bytecode(
                    interp,
                    func_obj.cheap_clone(),
                    bc_func,
                    this_value,
                    args,
                    return_register,
                    new_target,
                    true,          // is_async - wrap result in Promise
                    is_super_call, // pass through super call flag
                )?;
                Ok(())
            }
            // For all other function types, fall back to the interpreter's call_function
            // This includes PromiseResolve, PromiseReject, PromiseAllFulfill, AccessorGetter, etc.
            _ => {
                let result =
                    interp.call_function_with_new_target(callee, this_value, &args, new_target)?;
                self.set_reg(return_register, result.value);
                Ok(())
            }
        }
    }

    /// Set up a trampoline construct call - save current state and switch to the constructor
    /// This is similar to setup_trampoline_call but stores the new_obj in the frame
    /// so that if the constructor doesn't return an object, we can use the new_obj
    #[allow(clippy::too_many_arguments)]
    fn setup_trampoline_construct(
        &mut self,
        interp: &mut Interpreter,
        callee: JsValue,
        this_value: JsValue,
        args: Vec<JsValue>,
        return_register: Register,
        new_target: JsValue,
        new_obj: Gc<JsObject>,
    ) -> Result<(), JsError> {
        use crate::value::{ExoticObject, JsFunction};

        // Check call stack depth limit
        let total_depth = self.trampoline_stack.len();
        if interp.max_call_depth > 0 && total_depth >= interp.max_call_depth {
            return Err(JsError::range_error(format!(
                "Maximum call stack size exceeded (depth {})",
                total_depth
            )));
        }

        let JsValue::Object(func_obj) = &callee else {
            return Err(JsError::type_error("Not a constructor"));
        };

        let func = {
            let obj_ref = func_obj.borrow();
            match &obj_ref.exotic {
                ExoticObject::Function(f) => f.clone(),
                _ => return Err(JsError::type_error("Not a constructor")),
            }
        };

        match func {
            JsFunction::Bytecode(bc_func) => {
                // This is what we want to trampoline!
                self.push_trampoline_frame_and_call_bytecode_construct(
                    interp,
                    func_obj.cheap_clone(),
                    bc_func,
                    this_value,
                    args,
                    return_register,
                    new_target,
                    new_obj,
                )?;
                Ok(())
            }
            JsFunction::Bound(bound) => {
                // Unwrap bound function and trampoline to target
                let target = JsValue::Object(bound.target.cheap_clone());
                let mut full_args = bound.bound_args.clone();
                full_args.extend(args);
                self.setup_trampoline_construct(
                    interp,
                    target,
                    this_value,
                    full_args,
                    return_register,
                    new_target,
                    new_obj,
                )
            }
            // For all other function types, fall back to the interpreter's call_function
            // and handle the object/non-object return value
            _ => {
                let result =
                    interp.call_function_with_new_target(callee, this_value, &args, new_target)?;
                // If constructor returned an object, use that; otherwise use the new_obj
                let final_val = match result.value {
                    JsValue::Object(obj) => JsValue::Object(obj),
                    _ => JsValue::Object(new_obj),
                };
                self.set_reg(return_register, final_val);
                Ok(())
            }
        }
    }

    /// Push current state onto trampoline stack and set up for bytecode function call
    /// If is_async is true, the result will be wrapped in a Promise when the frame is popped
    #[allow(clippy::too_many_arguments)]
    fn push_trampoline_frame_and_call_bytecode(
        &mut self,
        interp: &mut Interpreter,
        func_obj: Gc<JsObject>,
        bc_func: BytecodeFunction,
        this_value: JsValue,
        args: Vec<JsValue>,
        return_register: Register,
        new_target: JsValue,
        is_async: bool,
        is_super_call: bool,
    ) -> Result<(), JsError> {
        use crate::interpreter::{create_environment_unrooted_with_capacity, Binding, VarKey};

        // Get function info from the chunk
        let func_info = bc_func.chunk.function_info.as_ref();

        // Push call stack frame for stack traces
        let func_name = func_info
            .and_then(|info| info.name.as_ref())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "<anonymous>".to_string());

        interp.call_stack.push(crate::interpreter::StackFrame {
            function_name: func_name,
            location: None,
        });

        // Calculate environment capacity: params + this + potential arguments + some slack
        // Use binding_count if available, otherwise estimate from param_count
        let env_capacity = func_info
            .map(|info| {
                if info.binding_count > 0 {
                    info.binding_count
                } else {
                    // Estimate: params + this + arguments + a few locals
                    info.param_count + 4
                }
            })
            .unwrap_or(8);

        // Create new environment for the function, with closure as parent
        let (func_env, func_guard) = create_environment_unrooted_with_capacity(
            &interp.heap,
            Some(bc_func.closure.cheap_clone()),
            env_capacity,
        );

        // Bind `this` in the function environment
        let effective_this = if let Some(captured) = bc_func.captured_this {
            *captured
        } else {
            this_value.clone()
        };

        {
            let this_name = interp.intern("this");
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

        // Bind `__super__` if this function has it (for class methods with super)
        {
            let super_name = interp.intern("__super__");
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

        // Bind `__super_target__` for super.x property access
        {
            let super_target_name = interp.intern("__super_target__");
            let super_target_key = PropertyKey::String(super_target_name.cheap_clone());
            if let Some(super_target_val) = func_obj.borrow().get_property(&super_target_key) {
                if let Some(data) = func_env.borrow_mut().as_environment_mut() {
                    data.bindings.insert(
                        VarKey(super_target_name),
                        Binding {
                            value: super_target_val,
                            mutable: false,
                            initialized: true,
                            import_binding: None,
                        },
                    );
                }
            }
        }

        // Save current interpreter environment
        let saved_interp_env = interp.env.cheap_clone();
        interp.env = func_env;
        interp.push_env_guard(func_guard);

        // Handle rest parameters - separate args for registers vs arguments object
        let new_guard = interp.heap.create_guard();
        let (processed_args, new_arguments): (Option<Vec<JsValue>>, Vec<JsValue>) =
            if let Some(rest_idx) = func_info.and_then(|info| info.rest_param) {
                // Rest param case: create processed version for registers
                let mut result_args = Vec::with_capacity(rest_idx + 1);
                for i in 0..rest_idx {
                    result_args.push(args.get(i).cloned().unwrap_or(JsValue::Undefined));
                }
                let rest_elements: Vec<JsValue> = args.get(rest_idx..).unwrap_or_default().to_vec();
                let rest_array = interp.create_array_from(&new_guard, rest_elements);
                result_args.push(JsValue::Object(rest_array));
                (Some(result_args), args)
            } else {
                // No rest params: use args directly (no clone needed!)
                (None, args)
            };
        // Use processed args for registers if available, otherwise use original args
        let register_args = processed_args.as_ref().unwrap_or(&new_arguments);

        // Guard all values for the new frame
        if let JsValue::Object(obj) = &effective_this {
            new_guard.guard(obj.cheap_clone());
        }
        if let JsValue::Object(obj) = &new_target {
            new_guard.guard(obj.cheap_clone());
        }
        for arg in register_args {
            if let JsValue::Object(obj) = arg {
                new_guard.guard(obj.cheap_clone());
            }
        }

        // Create the new register file for the called function (from pool if available)
        let register_count = bc_func.chunk.register_count as usize;
        let mut new_registers = self.acquire_registers(register_count);
        for (i, arg) in register_args.iter().enumerate() {
            if i < new_registers.len() {
                if let Some(slot) = new_registers.get_mut(i) {
                    *slot = arg.clone();
                }
            }
        }

        // Save current VM state to trampoline stack
        let old_guard = std::mem::replace(&mut self.register_guard, new_guard);
        // new_arguments already set from rest parameter handling above - no extra allocation needed
        let frame = TrampolineFrame {
            ip: self.ip,
            chunk: self.chunk.cheap_clone(),
            registers: std::mem::replace(&mut self.registers, new_registers),
            this_value: std::mem::replace(&mut self.this_value, effective_this),
            vm_call_stack: std::mem::take(&mut self.call_stack),
            try_stack: std::mem::take(&mut self.try_stack),
            exception_value: self.exception_value.take(),
            saved_env_stack: std::mem::take(&mut self.saved_env_stack),
            arguments: std::mem::replace(&mut self.arguments, new_arguments),
            new_target: std::mem::replace(&mut self.new_target, new_target),
            current_constructor: self.current_constructor.take(),
            pending_completion: self.pending_completion.take(),
            return_register,
            saved_interp_env,
            register_guard: old_guard,
            construct_new_obj: None,
            is_async,
        };
        self.trampoline_stack.push(frame);

        // For super() calls, set the current constructor so super() lookups work correctly
        if is_super_call {
            self.current_constructor = Some(func_obj);
        }

        // Set up VM for the called function
        self.ip = 0;
        self.chunk = bc_func.chunk;

        Ok(())
    }

    /// Push current state onto trampoline stack for a construct call
    /// This is like push_trampoline_frame_and_call_bytecode but stores the new_obj
    /// in the frame so it can be used if the constructor doesn't return an object
    #[allow(clippy::too_many_arguments)]
    fn push_trampoline_frame_and_call_bytecode_construct(
        &mut self,
        interp: &mut Interpreter,
        func_obj: Gc<JsObject>,
        bc_func: BytecodeFunction,
        this_value: JsValue,
        args: Vec<JsValue>,
        return_register: Register,
        new_target: JsValue,
        construct_new_obj: Gc<JsObject>,
    ) -> Result<(), JsError> {
        use crate::interpreter::{create_environment_unrooted_with_capacity, Binding, VarKey};

        // Get function info from the chunk
        let func_info = bc_func.chunk.function_info.as_ref();

        // Push call stack frame for stack traces
        let func_name = func_info
            .and_then(|info| info.name.as_ref())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "<anonymous>".to_string());

        interp.call_stack.push(crate::interpreter::StackFrame {
            function_name: func_name,
            location: None,
        });

        // Calculate environment capacity: params + this + potential arguments + some slack
        // Use binding_count if available, otherwise estimate from param_count
        let env_capacity = func_info
            .map(|info| {
                if info.binding_count > 0 {
                    info.binding_count
                } else {
                    // Estimate: params + this + arguments + a few locals
                    info.param_count + 4
                }
            })
            .unwrap_or(8);

        // Create new environment for the function, with closure as parent
        let (func_env, func_guard) = create_environment_unrooted_with_capacity(
            &interp.heap,
            Some(bc_func.closure.cheap_clone()),
            env_capacity,
        );

        // Bind `this` in the function environment
        let effective_this = if let Some(captured) = bc_func.captured_this {
            *captured
        } else {
            this_value.clone()
        };

        {
            let this_name = interp.intern("this");
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

        // Bind `__super__` if this function has it (for class methods with super)
        {
            let super_name = interp.intern("__super__");
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

        // Bind `__super_target__` for super.x property access
        {
            let super_target_name = interp.intern("__super_target__");
            let super_target_key = PropertyKey::String(super_target_name.cheap_clone());
            if let Some(super_target_val) = func_obj.borrow().get_property(&super_target_key) {
                if let Some(data) = func_env.borrow_mut().as_environment_mut() {
                    data.bindings.insert(
                        VarKey(super_target_name),
                        Binding {
                            value: super_target_val,
                            mutable: false,
                            initialized: true,
                            import_binding: None,
                        },
                    );
                }
            }
        }

        // Save current interpreter environment
        let saved_interp_env = interp.env.cheap_clone();
        interp.env = func_env;
        interp.push_env_guard(func_guard);

        // Handle rest parameters - separate args for registers vs arguments object
        let new_guard = interp.heap.create_guard();
        let (processed_args, new_arguments): (Option<Vec<JsValue>>, Vec<JsValue>) =
            if let Some(rest_idx) = func_info.and_then(|info| info.rest_param) {
                // Rest param case: create processed version for registers
                let mut result_args = Vec::with_capacity(rest_idx + 1);
                for i in 0..rest_idx {
                    result_args.push(args.get(i).cloned().unwrap_or(JsValue::Undefined));
                }
                let rest_elements: Vec<JsValue> = args.get(rest_idx..).unwrap_or_default().to_vec();
                let rest_array = interp.create_array_from(&new_guard, rest_elements);
                result_args.push(JsValue::Object(rest_array));
                (Some(result_args), args)
            } else {
                // No rest params: use args directly (no clone needed!)
                (None, args)
            };
        // Use processed args for registers if available, otherwise use original args
        let register_args = processed_args.as_ref().unwrap_or(&new_arguments);

        // Guard all values for the new frame
        if let JsValue::Object(obj) = &effective_this {
            new_guard.guard(obj.cheap_clone());
        }
        if let JsValue::Object(obj) = &new_target {
            new_guard.guard(obj.cheap_clone());
        }
        for arg in register_args {
            if let JsValue::Object(obj) = arg {
                new_guard.guard(obj.cheap_clone());
            }
        }
        // Also guard the construct_new_obj
        new_guard.guard(construct_new_obj.cheap_clone());

        // Create the new register file for the called function (from pool if available)
        let register_count = bc_func.chunk.register_count as usize;
        let mut new_registers = self.acquire_registers(register_count);
        for (i, arg) in register_args.iter().enumerate() {
            if i < new_registers.len() {
                if let Some(slot) = new_registers.get_mut(i) {
                    *slot = arg.clone();
                }
            }
        }

        // Save current VM state to trampoline stack
        let old_guard = std::mem::replace(&mut self.register_guard, new_guard);
        // new_arguments already set from rest parameter handling above - no extra allocation needed
        let frame = TrampolineFrame {
            ip: self.ip,
            chunk: self.chunk.cheap_clone(),
            registers: std::mem::replace(&mut self.registers, new_registers),
            this_value: std::mem::replace(&mut self.this_value, effective_this),
            vm_call_stack: std::mem::take(&mut self.call_stack),
            try_stack: std::mem::take(&mut self.try_stack),
            exception_value: self.exception_value.take(),
            saved_env_stack: std::mem::take(&mut self.saved_env_stack),
            arguments: std::mem::replace(&mut self.arguments, new_arguments),
            new_target: std::mem::replace(&mut self.new_target, new_target),
            current_constructor: self.current_constructor.take(),
            pending_completion: self.pending_completion.take(),
            return_register,
            saved_interp_env,
            register_guard: old_guard,
            construct_new_obj: Some(construct_new_obj),
            is_async: false, // Construct calls are never async
        };
        self.trampoline_stack.push(frame);

        // Set up the current constructor for super() lookups
        self.current_constructor = Some(func_obj);

        // Set up VM for the called function
        self.ip = 0;
        self.chunk = bc_func.chunk;

        Ok(())
    }

    /// Restore VM state from a trampoline frame after a function returns
    fn restore_from_trampoline_frame(
        &mut self,
        interp: &mut Interpreter,
        frame: TrampolineFrame,
        return_value: JsValue,
    ) {
        // Release current registers back to pool before restoring
        let current_registers = std::mem::take(&mut self.registers);
        self.release_registers(current_registers);

        // Release current arguments back to pool before restoring
        let current_arguments = std::mem::take(&mut self.arguments);
        self.release_arguments(current_arguments);

        // Restore VM state
        self.ip = frame.ip;
        self.chunk = frame.chunk;
        self.registers = frame.registers;
        self.register_guard = frame.register_guard;
        self.this_value = frame.this_value;
        self.call_stack = frame.vm_call_stack;
        self.try_stack = frame.try_stack;
        self.exception_value = frame.exception_value;
        self.saved_env_stack = frame.saved_env_stack;
        self.arguments = frame.arguments;
        self.new_target = frame.new_target;
        self.current_constructor = frame.current_constructor;
        self.pending_completion = frame.pending_completion;

        // Restore interpreter environment
        interp.pop_env_guard();
        interp.env = frame.saved_interp_env;
        interp.call_stack.pop();

        // For construct calls: if constructor didn't return an object, use the new object
        let intermediate_value = if let Some(new_obj) = frame.construct_new_obj {
            match return_value {
                JsValue::Object(obj) => JsValue::Object(obj),
                _ => JsValue::Object(new_obj),
            }
        } else {
            return_value
        };

        // For async calls: wrap result in a Promise
        let final_value = if frame.is_async {
            use crate::value::ExoticObject;
            // Promise assimilation: if result is already a Promise, return it directly
            if let JsValue::Object(ref obj) = &intermediate_value {
                if matches!(obj.borrow().exotic, ExoticObject::Promise(_)) {
                    intermediate_value
                } else {
                    // Wrap non-Promise value in a fulfilled Promise
                    let promise = super::builtins::promise::create_fulfilled_promise(
                        interp,
                        &self.register_guard,
                        intermediate_value,
                    );
                    JsValue::Object(promise)
                }
            } else {
                // Wrap primitive value in a fulfilled Promise
                let promise = super::builtins::promise::create_fulfilled_promise(
                    interp,
                    &self.register_guard,
                    intermediate_value,
                );
                JsValue::Object(promise)
            }
        } else {
            intermediate_value
        };

        // Store return value in the designated register
        // Guard it with the restored frame's guard
        if let JsValue::Object(obj) = &final_value {
            self.register_guard.guard(obj.cheap_clone());
        }
        self.set_reg(frame.return_register, final_value);
    }

    /// Convert an error to a guarded JS value (takes ownership to avoid re-guarding)
    fn error_to_guarded(&self, interp: &mut Interpreter, error: JsError) -> Guarded {
        match error {
            JsError::ThrownValue { guarded } => guarded,
            other => {
                // Create an error object using the proper error type
                use crate::interpreter::builtins::error::create_error_object;
                let (value, guard) = create_error_object(interp, &other);
                Guarded { value, guard }
            }
        }
    }

    /// Find an exception handler for the current position
    /// Returns (handler_ip, scope_depth) where scope_depth is the number of scopes to unwind
    fn find_exception_handler(&mut self, interp: &mut Interpreter) -> Option<usize> {
        while let Some(handler) = self.try_stack.pop() {
            // Unwind to this handler's frame depth
            while self.call_stack.len() > handler.frame_depth {
                self.call_stack.pop();
            }

            if handler.catch_ip > 0 {
                // If there's also a finally, push a handler for it
                // so break/continue in catch will still run finally
                if handler.finally_ip > 0 {
                    self.try_stack.push(TryHandler {
                        catch_ip: 0, // Clear catch so it won't re-catch
                        finally_ip: handler.finally_ip,
                        registers_snapshot: handler.registers_snapshot,
                        frame_depth: handler.frame_depth,
                        scope_depth: handler.scope_depth,
                        iterator_reg: handler.iterator_reg,
                    });
                }
                // Unwind any scopes that were entered in the try block
                // This is like executing PopScope for each scope that was entered
                while self.saved_env_stack.len() > handler.scope_depth {
                    if let Some(saved_env) = self.saved_env_stack.pop() {
                        interp.pop_scope(saved_env);
                    }
                }
                return Some(handler.catch_ip);
            }
            if handler.finally_ip > 0 {
                // Also unwind scopes for finally handlers
                while self.saved_env_stack.len() > handler.scope_depth {
                    if let Some(saved_env) = self.saved_env_stack.pop() {
                        interp.pop_scope(saved_env);
                    }
                }
                return Some(handler.finally_ip);
            }
        }
        None
    }

    /// Handle an error, including unwinding the trampoline stack to find handlers.
    /// Returns Ok(()) if a handler was found and execution should continue,
    /// or Err(e) if no handler was found (caller should return the error).
    fn handle_error_with_trampoline_unwind(
        &mut self,
        interp: &mut Interpreter,
        e: JsError,
    ) -> Result<(), JsError> {
        // First check for handler in current frame
        if let Some(handler_ip) = self.find_exception_handler(interp) {
            self.ip = handler_ip;
            self.exception_value = Some(self.error_to_guarded(interp, e));
            return Ok(());
        }

        // Unwind trampoline stack to find a handler in parent frames
        while let Some(frame) = self.trampoline_stack.pop() {
            let is_async_frame = frame.is_async;
            let return_register = frame.return_register;

            // Release current registers back to pool before restoring
            let current_registers = std::mem::take(&mut self.registers);
            self.release_registers(current_registers);

            // Release current arguments back to pool before restoring
            let current_arguments = std::mem::take(&mut self.arguments);
            self.release_arguments(current_arguments);

            // Unwind current frame's scopes before restoring - if the called function
            // had any scopes pushed (e.g., from PushScope in its body), we need to pop
            // them and their guards before switching to the caller's saved_env_stack
            while let Some(saved_env) = self.saved_env_stack.pop() {
                interp.pop_scope(saved_env);
            }

            // Restore state from frame
            self.ip = frame.ip;
            self.chunk = frame.chunk;
            self.registers = frame.registers;
            self.register_guard = frame.register_guard;
            self.this_value = frame.this_value;
            self.call_stack = frame.vm_call_stack;
            self.try_stack = frame.try_stack;
            self.exception_value = frame.exception_value;
            self.saved_env_stack = frame.saved_env_stack;
            self.arguments = frame.arguments;
            self.new_target = frame.new_target;
            self.pending_completion = frame.pending_completion;

            // Restore interpreter environment
            interp.pop_env_guard();
            interp.env = frame.saved_interp_env;
            interp.call_stack.pop();

            // For async frames: convert error to rejected Promise instead of propagating
            if is_async_frame {
                let error_guarded = self.error_to_guarded(interp, e);
                let promise = super::builtins::promise::create_rejected_promise(
                    interp,
                    &self.register_guard,
                    error_guarded.value,
                );
                // error_guarded.guard keeps the reason alive until promise is created
                drop(error_guarded.guard);
                self.register_guard.guard(promise.cheap_clone());
                self.set_reg(return_register, JsValue::Object(promise));
                return Ok(());
            }

            // Check for exception handler in this frame
            if let Some(handler_ip) = self.find_exception_handler(interp) {
                self.ip = handler_ip;
                self.exception_value = Some(self.error_to_guarded(interp, e));
                return Ok(());
            }
        }

        // No handler found - return the error back to caller
        Err(e)
    }

    /// Save VM state for suspension
    /// Creates a guard to keep all objects in registers alive during suspension
    fn save_state(&self, interp: &Interpreter) -> SavedVmState {
        let guard = interp.heap.create_guard();

        // Guard all objects in registers
        for val in &self.registers {
            if let JsValue::Object(obj) = val {
                guard.guard(obj.cheap_clone());
            }
        }

        // Guard saved environments in call frames
        for frame in &self.call_stack {
            if let Some(ref env) = frame.saved_env {
                guard.guard(env.cheap_clone());
            }
        }

        // Guard this_value if it's an object
        if let JsValue::Object(obj) = &self.this_value {
            guard.guard(obj.cheap_clone());
        }

        // Guard exception_value if it's an object
        // (exception_value is already Guarded, but we also add it to the saved state guard
        // for consistency when the saved state is restored)
        if let Some(Guarded {
            value: JsValue::Object(obj),
            ..
        }) = &self.exception_value
        {
            guard.guard(obj.cheap_clone());
        }

        // Guard saved_env_stack entries
        for env in &self.saved_env_stack {
            guard.guard(env.cheap_clone());
        }

        SavedVmState {
            frames: self.call_stack.clone(),
            ip: self.ip,
            chunk: self.chunk.clone(),
            registers: self.registers.clone(),
            try_stack: self.try_stack.clone(),
            guard: Some(guard),
            arguments: self.arguments.clone(),
            new_target: self.new_target.clone(),
        }
    }

    /// Create a new VM from saved state (for generator resumption)
    /// The guard must protect all objects in the saved registers
    pub fn from_saved_state(
        state: SavedVmState,
        this_value: JsValue,
        guard: Guard<JsObject>,
    ) -> Self {
        // Guard this_value if it's an object
        if let JsValue::Object(obj) = &this_value {
            guard.guard(obj.cheap_clone());
        }

        // Guard all objects in the restored registers
        for val in &state.registers {
            if let JsValue::Object(obj) = val {
                guard.guard(obj.cheap_clone());
            }
        }

        // Guard saved environments in call frames
        for frame in &state.frames {
            if let Some(ref env) = frame.saved_env {
                guard.guard(env.cheap_clone());
            }
        }

        Self {
            ip: state.ip,
            chunk: state.chunk,
            registers: state.registers,
            register_guard: guard,
            call_stack: state.frames,
            try_stack: state.try_stack,
            this_value,
            exception_value: None,
            saved_env_stack: Vec::new(),
            arguments: state.arguments,
            new_target: state.new_target,
            current_constructor: None,
            pending_completion: None,
            trampoline_stack: Vec::new(),
            register_pool: Vec::new(),
            arguments_pool: Vec::new(),
        }
    }

    /// Set the resume value for await resumption
    /// This stores the resolved promise value in the specified register
    pub fn set_resume_value(&mut self, register: Register, value: JsValue) {
        // Guard the value if it's an object
        if let JsValue::Object(ref obj) = value {
            self.register_guard.guard(obj.cheap_clone());
        }
        self.set_reg(register, value);
    }

    /// Inject an exception into the VM for generator.throw()
    /// This sets up the VM to handle the exception as if it was thrown at the current position.
    /// Returns true if an exception handler was found, false if the exception should propagate.
    pub fn inject_exception(&mut self, interp: &mut Interpreter, exception: JsValue) -> bool {
        // Create guarded exception value
        let guarded = Guarded::from_value(exception, &interp.heap);

        // Try to find an exception handler
        if let Some(handler_ip) = self.find_exception_handler(interp) {
            self.ip = handler_ip;
            self.exception_value = Some(guarded);
            true
        } else {
            // No handler found - store exception for propagation
            self.exception_value = Some(guarded);
            false
        }
    }

    /// Execute a single opcode
    fn execute_op(&mut self, interp: &mut Interpreter, op: Op) -> Result<OpResult, JsError> {
        match op {
            // ═══════════════════════════════════════════════════════════════════════════
            // Constants & Register Operations
            // ═══════════════════════════════════════════════════════════════════════════
            Op::LoadConst { dst, idx } => {
                let (value, _guard) = match self.get_constant(idx) {
                    Some(Constant::String(s)) => (JsValue::String(s.cheap_clone()), None),
                    Some(Constant::Number(n)) => (JsValue::Number(*n), None),
                    Some(Constant::RegExp { pattern, flags }) => {
                        let guard = interp.heap.create_guard();
                        let obj =
                            interp.create_regexp_literal(&guard, pattern.as_str(), flags.as_str());
                        (JsValue::Object(obj), Some(guard))
                    }
                    Some(Constant::Chunk(_)) => {
                        return Err(JsError::internal_error("Cannot load chunk as value"));
                    }
                    Some(Constant::TemplateStrings { .. }) => {
                        return Err(JsError::internal_error(
                            "Cannot load template strings as value",
                        ));
                    }
                    Some(Constant::ExcludedKeys(_)) => {
                        return Err(JsError::internal_error(
                            "Cannot load excluded keys as value",
                        ));
                    }
                    None => return Err(JsError::internal_error("Invalid constant index")),
                };
                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            Op::LoadUndefined { dst } => {
                self.set_reg(dst, JsValue::Undefined);
                Ok(OpResult::Continue)
            }

            Op::LoadNull { dst } => {
                self.set_reg(dst, JsValue::Null);
                Ok(OpResult::Continue)
            }

            Op::LoadBool { dst, value } => {
                self.set_reg(dst, JsValue::Boolean(value));
                Ok(OpResult::Continue)
            }

            Op::LoadInt { dst, value } => {
                self.set_reg(dst, JsValue::Number(value as f64));
                Ok(OpResult::Continue)
            }

            Op::Move { dst, src } => {
                let value = self.get_reg(src).clone();
                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Binary Arithmetic Operations
            // ═══════════════════════════════════════════════════════════════════════════
            Op::Add { dst, left, right } => {
                let left_val = self.get_reg(left);
                let right_val = self.get_reg(right);

                // First convert objects to primitives with "default" hint
                let left_prim = interp.coerce_to_primitive(left_val, "default")?;
                let right_prim = interp.coerce_to_primitive(right_val, "default")?;

                let result = match (&left_prim, &right_prim) {
                    (JsValue::String(a), _) => {
                        let right_str = interp.to_js_string(&right_prim);
                        JsValue::String(a.cheap_clone() + right_str.as_str())
                    }
                    (_, JsValue::String(b)) => {
                        let left_str = interp.to_js_string(&left_prim);
                        JsValue::String(left_str + b.as_str())
                    }
                    _ => JsValue::Number(left_prim.to_number() + right_prim.to_number()),
                };
                self.set_reg(dst, result);
                Ok(OpResult::Continue)
            }

            Op::Sub { dst, left, right } => {
                let left_val = interp.coerce_to_number(self.get_reg(left))?;
                let right_val = interp.coerce_to_number(self.get_reg(right))?;
                self.set_reg(dst, JsValue::Number(left_val - right_val));
                Ok(OpResult::Continue)
            }

            Op::Mul { dst, left, right } => {
                let left_val = interp.coerce_to_number(self.get_reg(left))?;
                let right_val = interp.coerce_to_number(self.get_reg(right))?;
                self.set_reg(dst, JsValue::Number(left_val * right_val));
                Ok(OpResult::Continue)
            }

            Op::Div { dst, left, right } => {
                let left_val = interp.coerce_to_number(self.get_reg(left))?;
                let right_val = interp.coerce_to_number(self.get_reg(right))?;
                self.set_reg(dst, JsValue::Number(left_val / right_val));
                Ok(OpResult::Continue)
            }

            Op::Mod { dst, left, right } => {
                let left_val = interp.coerce_to_number(self.get_reg(left))?;
                let right_val = interp.coerce_to_number(self.get_reg(right))?;
                self.set_reg(dst, JsValue::Number(left_val % right_val));
                Ok(OpResult::Continue)
            }

            Op::Exp { dst, left, right } => {
                let left_val = interp.coerce_to_number(self.get_reg(left))?;
                let right_val = interp.coerce_to_number(self.get_reg(right))?;
                self.set_reg(dst, JsValue::Number(left_val.powf(right_val)));
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Comparison Operations
            // ═══════════════════════════════════════════════════════════════════════════
            Op::Eq { dst, left, right } => {
                let left_val = self.get_reg(left);
                let right_val = self.get_reg(right);
                let result = interp.abstract_equals(left_val, right_val);
                self.set_reg(dst, JsValue::Boolean(result));
                Ok(OpResult::Continue)
            }

            Op::NotEq { dst, left, right } => {
                let left_val = self.get_reg(left);
                let right_val = self.get_reg(right);
                let result = interp.abstract_equals(left_val, right_val);
                self.set_reg(dst, JsValue::Boolean(!result));
                Ok(OpResult::Continue)
            }

            Op::StrictEq { dst, left, right } => {
                let left_val = self.get_reg(left);
                let right_val = self.get_reg(right);
                let result = left_val.strict_equals(right_val);
                self.set_reg(dst, JsValue::Boolean(result));
                Ok(OpResult::Continue)
            }

            Op::StrictNotEq { dst, left, right } => {
                let left_val = self.get_reg(left);
                let right_val = self.get_reg(right);
                let result = !left_val.strict_equals(right_val);
                self.set_reg(dst, JsValue::Boolean(result));
                Ok(OpResult::Continue)
            }

            Op::Lt { dst, left, right } => {
                let left_val = self.get_reg(left).to_number();
                let right_val = self.get_reg(right).to_number();
                self.set_reg(dst, JsValue::Boolean(left_val < right_val));
                Ok(OpResult::Continue)
            }

            Op::LtEq { dst, left, right } => {
                let left_val = self.get_reg(left).to_number();
                let right_val = self.get_reg(right).to_number();
                self.set_reg(dst, JsValue::Boolean(left_val <= right_val));
                Ok(OpResult::Continue)
            }

            Op::Gt { dst, left, right } => {
                let left_val = self.get_reg(left).to_number();
                let right_val = self.get_reg(right).to_number();
                self.set_reg(dst, JsValue::Boolean(left_val > right_val));
                Ok(OpResult::Continue)
            }

            Op::GtEq { dst, left, right } => {
                let left_val = self.get_reg(left).to_number();
                let right_val = self.get_reg(right).to_number();
                self.set_reg(dst, JsValue::Boolean(left_val >= right_val));
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Bitwise Operations
            // ═══════════════════════════════════════════════════════════════════════════
            Op::BitAnd { dst, left, right } => {
                let left_val = self.get_reg(left).to_number() as i32;
                let right_val = self.get_reg(right).to_number() as i32;
                self.set_reg(dst, JsValue::Number((left_val & right_val) as f64));
                Ok(OpResult::Continue)
            }

            Op::BitOr { dst, left, right } => {
                let left_val = self.get_reg(left).to_number() as i32;
                let right_val = self.get_reg(right).to_number() as i32;
                self.set_reg(dst, JsValue::Number((left_val | right_val) as f64));
                Ok(OpResult::Continue)
            }

            Op::BitXor { dst, left, right } => {
                let left_val = self.get_reg(left).to_number() as i32;
                let right_val = self.get_reg(right).to_number() as i32;
                self.set_reg(dst, JsValue::Number((left_val ^ right_val) as f64));
                Ok(OpResult::Continue)
            }

            Op::LShift { dst, left, right } => {
                let left_val = self.get_reg(left).to_number() as i32;
                let right_val = (self.get_reg(right).to_number() as u32) & 0x1F;
                self.set_reg(dst, JsValue::Number((left_val << right_val) as f64));
                Ok(OpResult::Continue)
            }

            Op::RShift { dst, left, right } => {
                let left_val = self.get_reg(left).to_number() as i32;
                let right_val = (self.get_reg(right).to_number() as u32) & 0x1F;
                self.set_reg(dst, JsValue::Number((left_val >> right_val) as f64));
                Ok(OpResult::Continue)
            }

            Op::URShift { dst, left, right } => {
                let left_val = (self.get_reg(left).to_number() as i32) as u32;
                let right_val = ((self.get_reg(right).to_number() as i32) as u32) & 0x1F;
                self.set_reg(dst, JsValue::Number((left_val >> right_val) as f64));
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Special Binary Operations
            // ═══════════════════════════════════════════════════════════════════════════
            Op::In { dst, left, right } => {
                let key = self.get_reg(left);
                let obj = self.get_reg(right);

                let JsValue::Object(obj_ref) = obj else {
                    return Err(JsError::type_error(
                        "Cannot use 'in' operator with non-object",
                    ));
                };

                let prop_key = interp.property_key_from_value(key);

                // Check if this is a proxy - delegate to proxy_has if so
                let has_prop = if matches!(obj_ref.borrow().exotic, ExoticObject::Proxy(_)) {
                    crate::interpreter::builtins::proxy::proxy_has(
                        interp,
                        obj_ref.cheap_clone(),
                        &prop_key,
                    )?
                } else {
                    obj_ref.borrow().has_own_property(&prop_key)
                };

                self.set_reg(dst, JsValue::Boolean(has_prop));
                Ok(OpResult::Continue)
            }

            Op::Instanceof { dst, left, right } => {
                let left_val = self.get_reg(left);
                let right_val = self.get_reg(right);

                // right must be an object
                let JsValue::Object(right_obj) = right_val else {
                    return Err(JsError::type_error(
                        "Right-hand side of 'instanceof' is not an object",
                    ));
                };

                // Step 1: Check for Symbol.hasInstance method (custom instanceof behavior)
                let well_known = interp.well_known_symbols;
                let has_instance_symbol = crate::value::JsSymbol::new(
                    well_known.has_instance,
                    Some(interp.intern("Symbol.hasInstance")),
                );
                let has_instance_key = PropertyKey::Symbol(Box::new(has_instance_symbol));

                // Look up Symbol.hasInstance on right object (and its prototype chain)
                let has_instance_method = right_obj.borrow().get_property(&has_instance_key);

                if let Some(JsValue::Object(method_obj)) = has_instance_method {
                    if method_obj.borrow().is_callable() {
                        // Call the custom Symbol.hasInstance method
                        let result = interp.call_function(
                            JsValue::Object(method_obj),
                            right_val.clone(),
                            std::slice::from_ref(left_val),
                        )?;
                        // Convert result to boolean
                        self.set_reg(dst, JsValue::Boolean(result.value.to_boolean()));
                        return Ok(OpResult::Continue);
                    }
                }

                // Step 2: Fall back to OrdinaryHasInstance
                // right must be callable for OrdinaryHasInstance
                if !right_obj.borrow().is_callable() {
                    return Err(JsError::type_error(
                        "Right-hand side of 'instanceof' is not callable",
                    ));
                }

                // Get right.prototype
                let proto_key = PropertyKey::String(interp.intern("prototype"));
                let right_proto = right_obj.borrow().get_property(&proto_key);

                // If prototype is not an object, throw TypeError
                let Some(JsValue::Object(right_proto_obj)) = right_proto else {
                    return Err(JsError::type_error(
                        "Function has non-object prototype in instanceof check",
                    ));
                };

                // Check if left's prototype chain contains right.prototype
                let result = if let JsValue::Object(left_obj) = left_val {
                    let mut current = left_obj.borrow().prototype.clone();
                    let target_id = right_proto_obj.id();
                    let mut found = false;
                    while let Some(proto) = current {
                        if proto.id() == target_id {
                            found = true;
                            break;
                        }
                        current = proto.borrow().prototype.clone();
                    }
                    found
                } else {
                    false
                };

                self.set_reg(dst, JsValue::Boolean(result));
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Unary Operations
            // ═══════════════════════════════════════════════════════════════════════════
            Op::Neg { dst, src } => {
                let val = interp.coerce_to_number(self.get_reg(src))?;
                self.set_reg(dst, JsValue::Number(-val));
                Ok(OpResult::Continue)
            }

            Op::Plus { dst, src } => {
                let val = interp.coerce_to_number(self.get_reg(src))?;
                self.set_reg(dst, JsValue::Number(val));
                Ok(OpResult::Continue)
            }

            Op::Not { dst, src } => {
                let val = self.get_reg(src).to_boolean();
                self.set_reg(dst, JsValue::Boolean(!val));
                Ok(OpResult::Continue)
            }

            Op::BitNot { dst, src } => {
                let val = self.get_reg(src).to_number() as i32;
                self.set_reg(dst, JsValue::Number((!val) as f64));
                Ok(OpResult::Continue)
            }

            Op::Typeof { dst, src } => {
                let type_str = match self.get_reg(src) {
                    JsValue::Undefined => "undefined",
                    JsValue::Null => "object",
                    JsValue::Boolean(_) => "boolean",
                    JsValue::Number(_) => "number",
                    JsValue::String(_) => "string",
                    JsValue::Symbol(_) => "symbol",
                    JsValue::Object(obj) => {
                        if obj.borrow().is_callable() {
                            "function"
                        } else {
                            "object"
                        }
                    }
                };
                self.set_reg(dst, JsValue::String(interp.intern(type_str)));
                Ok(OpResult::Continue)
            }

            Op::Void { dst, src: _ } => {
                self.set_reg(dst, JsValue::Undefined);
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Control Flow
            // ═══════════════════════════════════════════════════════════════════════════
            Op::Jump { target } => {
                self.ip = target as usize;
                Ok(OpResult::Continue)
            }

            Op::JumpIfTrue { cond, target } => {
                if self.get_reg(cond).to_boolean() {
                    self.ip = target as usize;
                }
                Ok(OpResult::Continue)
            }

            Op::JumpIfFalse { cond, target } => {
                if !self.get_reg(cond).to_boolean() {
                    self.ip = target as usize;
                }
                Ok(OpResult::Continue)
            }

            Op::JumpIfNullish { cond, target } => {
                let val = self.get_reg(cond);
                if matches!(val, JsValue::Null | JsValue::Undefined) {
                    self.ip = target as usize;
                }
                Ok(OpResult::Continue)
            }

            Op::JumpIfNotNullish { cond, target } => {
                let val = self.get_reg(cond);
                if !matches!(val, JsValue::Null | JsValue::Undefined) {
                    self.ip = target as usize;
                }
                Ok(OpResult::Continue)
            }

            // NOTE: review
            Op::Break { target, try_depth } => self.execute_break(target as usize, try_depth),

            // NOTE: review
            Op::Continue { target, try_depth } => self.execute_continue(target as usize, try_depth),

            // ═══════════════════════════════════════════════════════════════════════════
            // Variable Access
            // ═══════════════════════════════════════════════════════════════════════════
            Op::GetVar { dst, name } => {
                let name = self
                    .get_string_constant(name)
                    .ok_or_else(|| JsError::internal_error("Invalid variable name constant"))?;
                let value = interp.env_get(&name)?;
                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            Op::TryGetVar { dst, name } => {
                let name = self
                    .get_string_constant(name)
                    .ok_or_else(|| JsError::internal_error("Invalid variable name constant"))?;
                // Try to get the variable, return undefined if not found
                let value = interp.env_get(&name).unwrap_or(JsValue::Undefined);
                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            Op::SetVar { name, src } => {
                let name = self
                    .get_string_constant(name)
                    .ok_or_else(|| JsError::internal_error("Invalid variable name constant"))?;
                let value = self.get_reg(src).clone();
                interp.env_set(&name, value)?;
                Ok(OpResult::Continue)
            }

            Op::DeclareVar {
                name,
                init,
                mutable,
            } => {
                let name = self
                    .get_string_constant(name)
                    .ok_or_else(|| JsError::internal_error("Invalid variable name constant"))?;
                let value = self.get_reg(init).clone();
                interp.env_define(name, value, mutable);
                Ok(OpResult::Continue)
            }

            Op::DeclareVarHoisted { name, init } => {
                let name = self
                    .get_string_constant(name)
                    .ok_or_else(|| JsError::internal_error("Invalid variable name constant"))?;
                let value = self.get_reg(init).clone();
                interp.env_define(name, value, true);
                Ok(OpResult::Continue)
            }

            Op::GetGlobal { dst, name } => {
                let name = self
                    .get_string_constant(name)
                    .ok_or_else(|| JsError::internal_error("Invalid global name constant"))?;
                let global = interp.global.cheap_clone();
                let prop_key = PropertyKey::String(name);
                let value = global
                    .borrow()
                    .get_property(&prop_key)
                    .unwrap_or(JsValue::Undefined);
                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            Op::SetGlobal { name, src } => {
                let name = self
                    .get_string_constant(name)
                    .ok_or_else(|| JsError::internal_error("Invalid global name constant"))?;
                let value = self.get_reg(src).clone();
                let global = interp.global.clone();
                global
                    .borrow_mut()
                    .set_property(PropertyKey::String(name), value);
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Object/Array Operations
            // ═══════════════════════════════════════════════════════════════════════════
            Op::CreateObject { dst } => {
                let guard = interp.heap.create_guard();
                let obj = interp.create_object(&guard);
                self.set_reg(dst, JsValue::Object(obj));
                Ok(OpResult::Continue)
            }

            Op::CreateArray { dst, start, count } => {
                let mut elements = Vec::with_capacity(count as usize);
                for i in 0..count {
                    elements.push(self.get_reg(start + i as u8).clone());
                }
                let guard = interp.heap.create_guard();
                let arr = interp.create_array_from(&guard, elements);
                self.set_reg(dst, JsValue::Object(arr));
                Ok(OpResult::Continue)
            }

            Op::GetProperty { dst, obj, key } => {
                let obj_val = self.get_reg(obj);
                let key_val = self.get_reg(key);
                let Guarded { value, .. } = self.get_property_value(interp, obj_val, key_val)?;
                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            Op::GetPropertyConst { dst, obj, key } => {
                let obj_val = self.get_reg(obj);
                let key = self
                    .get_string_constant(key)
                    .ok_or_else(|| JsError::internal_error("Invalid property key constant"))?;
                let key_val = JsValue::String(key);
                let Guarded { value, .. } = self.get_property_value(interp, obj_val, &key_val)?;
                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            Op::SetProperty { obj, key, value } => {
                let obj_val = self.get_reg(obj);
                let key_val = self.get_reg(key);
                let val = self.get_reg(value).clone();
                self.set_property_value(interp, obj_val, key_val, val)?;
                Ok(OpResult::Continue)
            }

            Op::SetPropertyConst { obj, key, value } => {
                let obj_val = self.get_reg(obj);
                let key = self
                    .get_string_constant(key)
                    .ok_or_else(|| JsError::internal_error("Invalid property key constant"))?;
                let val = self.get_reg(value).clone();
                let key_val = JsValue::String(key);
                self.set_property_value(interp, obj_val, &key_val, val)?;
                Ok(OpResult::Continue)
            }

            Op::DeleteProperty { dst, obj, key } => {
                let obj_val = self.get_reg(obj);
                let key_val = self.get_reg(key);

                match obj_val {
                    JsValue::Null => {
                        return Err(JsError::type_error("Cannot delete property of null"));
                    }
                    JsValue::Undefined => {
                        return Err(JsError::type_error("Cannot delete property of undefined"));
                    }
                    JsValue::Object(obj_ref) => {
                        let prop_key = interp.property_key_from_value(key_val);

                        // Check if this is a proxy - delegate to proxy_delete_property if so
                        if matches!(obj_ref.borrow().exotic, ExoticObject::Proxy(_)) {
                            let result =
                                crate::interpreter::builtins::proxy::proxy_delete_property(
                                    interp,
                                    obj_ref.cheap_clone(),
                                    &prop_key,
                                )?;
                            self.set_reg(dst, JsValue::Boolean(result));
                        } else {
                            // Check if property is configurable before deleting
                            {
                                let obj_borrowed = obj_ref.borrow();
                                if let Some(prop) = obj_borrowed.properties.get(&prop_key) {
                                    if !prop.configurable() {
                                        return Err(JsError::type_error(format!(
                                            "Cannot delete property '{}' of object",
                                            prop_key
                                        )));
                                    }
                                }
                            }

                            // For arrays, handle index deletion specially
                            {
                                let mut obj_borrowed = obj_ref.borrow_mut();
                                if let PropertyKey::Index(idx) = &prop_key {
                                    if let Some(elements) = obj_borrowed.array_elements_mut() {
                                        let idx = *idx as usize;
                                        if idx < elements.len() {
                                            // Set to undefined (creating a hole)
                                            if let Some(elem) = elements.get_mut(idx) {
                                                *elem = JsValue::Undefined;
                                            }
                                        }
                                    }
                                }

                                obj_borrowed.properties.remove(&prop_key);
                            }
                            self.set_reg(dst, JsValue::Boolean(true));
                        }
                    }
                    // Primitives: delete returns true
                    JsValue::Number(_)
                    | JsValue::String(_)
                    | JsValue::Boolean(_)
                    | JsValue::Symbol(_) => {
                        self.set_reg(dst, JsValue::Boolean(true));
                    }
                }
                Ok(OpResult::Continue)
            }

            Op::DeletePropertyConst { dst, obj, key } => {
                let obj_val = self.get_reg(obj);
                let key = self
                    .get_string_constant(key)
                    .ok_or_else(|| JsError::internal_error("Invalid property key constant"))?;

                match obj_val {
                    JsValue::Null => {
                        return Err(JsError::type_error("Cannot delete property of null"));
                    }
                    JsValue::Undefined => {
                        return Err(JsError::type_error("Cannot delete property of undefined"));
                    }
                    JsValue::Object(obj_ref) => {
                        let prop_key = PropertyKey::String(key);

                        // Check if this is a proxy - delegate to proxy_delete_property if so
                        if matches!(obj_ref.borrow().exotic, ExoticObject::Proxy(_)) {
                            let result =
                                crate::interpreter::builtins::proxy::proxy_delete_property(
                                    interp,
                                    obj_ref.cheap_clone(),
                                    &prop_key,
                                )?;
                            self.set_reg(dst, JsValue::Boolean(result));
                        } else {
                            // Check if property is configurable before deleting
                            {
                                let obj_borrowed = obj_ref.borrow();
                                if let Some(prop) = obj_borrowed.properties.get(&prop_key) {
                                    if !prop.configurable() {
                                        return Err(JsError::type_error(format!(
                                            "Cannot delete property '{}' of object",
                                            prop_key
                                        )));
                                    }
                                }
                            }

                            obj_ref.borrow_mut().properties.remove(&prop_key);
                            self.set_reg(dst, JsValue::Boolean(true));
                        }
                    }
                    // Primitives: delete returns true
                    JsValue::Number(_)
                    | JsValue::String(_)
                    | JsValue::Boolean(_)
                    | JsValue::Symbol(_) => {
                        self.set_reg(dst, JsValue::Boolean(true));
                    }
                }
                Ok(OpResult::Continue)
            }

            Op::DefineProperty {
                obj,
                key,
                value,
                flags: _,
            } => {
                let obj_val = self.get_reg(obj);
                let key_val = self.get_reg(key);

                if let JsValue::Object(obj_ref) = obj_val {
                    let prop_key = interp.property_key_from_value(key_val);
                    let val = self.get_reg(value).clone();
                    obj_ref.borrow_mut().set_property(prop_key, val);
                }
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Function Operations
            // ═══════════════════════════════════════════════════════════════════════════
            Op::Call {
                dst,
                callee,
                this,
                args_start,
                argc,
            } => {
                // Acquire args vec first (mutable borrow), then get register values
                let mut args = self.acquire_arguments_vec(argc as usize);
                for i in 0..argc {
                    args.push(self.get_reg(args_start + i).clone());
                }
                let callee_val = self.get_reg(callee).clone();
                let this_val = self.get_reg(this).clone();

                // Create guard and protect all object values
                let guard = interp.heap.create_guard();
                callee_val.guard_by(&guard);
                this_val.guard_by(&guard);
                for arg in &args {
                    arg.guard_by(&guard);
                }

                // Use trampoline for function calls
                Ok(OpResult::Call {
                    callee: callee_val,
                    this_value: this_val,
                    args,
                    return_register: dst,
                    new_target: JsValue::Undefined,
                    is_super_call: false,
                    guard,
                })
            }

            Op::CallSpread {
                dst,
                callee,
                this,
                args_start,
                argc: _,
            } => {
                // CallSpread: args_start points to an array of arguments
                // We extract the array elements and call the function with them
                let callee_val = self.get_reg(callee).clone();
                let this_val = self.get_reg(this).clone();
                let args_val = self.get_reg(args_start).clone();

                let args: Vec<JsValue> = if let JsValue::Object(arr_ref) = &args_val {
                    if let Some(elems) = arr_ref.borrow().array_elements() {
                        elems.to_vec()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                // Create guard and protect all object values
                let guard = interp.heap.create_guard();
                callee_val.guard_by(&guard);
                this_val.guard_by(&guard);
                for arg in &args {
                    arg.guard_by(&guard);
                }

                // Use trampoline for function calls
                Ok(OpResult::Call {
                    callee: callee_val,
                    this_value: this_val,
                    args,
                    return_register: dst,
                    new_target: JsValue::Undefined,
                    is_super_call: false,
                    guard,
                })
            }

            Op::DirectEval { dst, arg } => {
                // Direct eval - executes code in the current lexical scope
                let arg_val = self.get_reg(arg);

                // If argument is not a string, return it directly
                let code = match &arg_val {
                    JsValue::String(s) => s,
                    _ => {
                        self.set_reg(dst, arg_val.clone());
                        return Ok(OpResult::Continue);
                    }
                };

                // Execute the code in current scope with the current `this` value
                let Guarded {
                    value,
                    guard: _guard,
                } = crate::interpreter::builtins::global::eval_code_in_scope_with_this(
                    interp,
                    code.as_str(),
                    false,
                    self.this_value.clone(),
                )?;
                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            Op::CallMethod {
                dst,
                obj,
                method,
                args_start,
                argc,
            } => {
                // Acquire args vec first (mutable borrow), then get register values
                let mut args = self.acquire_arguments_vec(argc as usize);
                for i in 0..argc {
                    args.push(self.get_reg(args_start + i).clone());
                }

                let obj_val = self.get_reg(obj).clone();
                let method_name = self
                    .get_string_constant(method)
                    .ok_or_else(|| JsError::internal_error("Invalid method name constant"))?;

                let Guarded {
                    value: callee,
                    guard: callee_guard,
                } = self.get_property_value(interp, &obj_val, &JsValue::String(method_name))?;

                // Create guard and protect all object values
                let guard = interp.heap.create_guard();
                callee.guard_by(&guard);
                obj_val.guard_by(&guard);
                for arg in &args {
                    arg.guard_by(&guard);
                }
                // Transfer callee guard to our guard (if any)
                if let Some(cg) = callee_guard {
                    if let JsValue::Object(obj) = &callee {
                        guard.guard(obj.cheap_clone());
                    }
                    drop(cg);
                }

                // Use trampoline for function calls
                Ok(OpResult::Call {
                    callee,
                    this_value: obj_val,
                    args,
                    return_register: dst,
                    new_target: JsValue::Undefined,
                    is_super_call: false,
                    guard,
                })
            }

            Op::Construct {
                dst,
                callee,
                args_start,
                argc,
            } => {
                // Acquire args vec first (mutable borrow), then get register values
                let mut args = self.acquire_arguments_vec(argc as usize);
                for i in 0..argc {
                    args.push(self.get_reg(args_start + i).clone());
                }
                let callee_val = self.get_reg(callee).clone();

                // Inline constructor call logic (similar to evaluate_new)
                let JsValue::Object(ctor) = &callee_val else {
                    return Err(JsError::type_error("Constructor is not a callable object"));
                };

                // Check if this is a proxy - delegate to proxy_construct if so
                if matches!(ctor.borrow().exotic, ExoticObject::Proxy(_)) {
                    let Guarded {
                        value,
                        guard: _guard,
                    } = crate::interpreter::builtins::proxy::proxy_construct(
                        interp,
                        ctor.cheap_clone(),
                        args,
                        callee_val.clone(), // new.target is the proxy itself
                    )?;
                    self.set_reg(dst, value);
                    return Ok(OpResult::Continue);
                }

                // Check if this is `new eval()` - eval is not a constructor
                if let ExoticObject::Function(JsFunction::Native(native)) = &ctor.borrow().exotic {
                    if native.name.as_str() == "eval" {
                        return Err(JsError::type_error("eval is not a constructor"));
                    }
                }

                // Create guard for OpResult values
                let guard = interp.heap.create_guard();
                guard.guard(ctor.cheap_clone());

                // Create a new object
                let new_obj = interp.create_object(&guard);

                // Get the constructor's prototype
                let proto_key = PropertyKey::String(interp.intern("prototype"));
                if let Some(JsValue::Object(proto)) = ctor.borrow().get_property(&proto_key) {
                    new_obj.borrow_mut().prototype = Some(proto.cheap_clone());
                }

                // Guard all object values in args
                for arg in &args {
                    arg.guard_by(&guard);
                }

                // Use trampoline for constructor call
                let this = JsValue::Object(new_obj.cheap_clone());
                Ok(OpResult::Construct {
                    callee: callee_val.clone(),
                    this_value: this,
                    args,
                    return_register: dst,
                    new_target: callee_val, // new.target is the constructor itself
                    new_obj,
                    guard,
                })
            }

            Op::ConstructSpread {
                dst,
                callee,
                args_start,
                argc: _,
            } => {
                // ConstructSpread: args_start points to an array of arguments
                let callee_val = self.get_reg(callee);
                let args_val = self.get_reg(args_start);

                let args: Vec<JsValue> = if let JsValue::Object(arr_ref) = &args_val {
                    if let Some(elems) = arr_ref.borrow().array_elements() {
                        elems.to_vec()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                // Inline constructor call logic (same as Construct)
                let JsValue::Object(ctor) = callee_val else {
                    return Err(JsError::type_error("Constructor is not a callable object"));
                };

                // Check if this is a proxy - delegate to proxy_construct if so
                if matches!(ctor.borrow().exotic, ExoticObject::Proxy(_)) {
                    let Guarded {
                        value,
                        guard: _guard,
                    } = crate::interpreter::builtins::proxy::proxy_construct(
                        interp,
                        ctor.cheap_clone(),
                        args,
                        callee_val.clone(), // new.target is the proxy itself
                    )?;
                    self.set_reg(dst, value);
                    return Ok(OpResult::Continue);
                }

                // Check if this is `new eval()` - eval is not a constructor
                if let ExoticObject::Function(JsFunction::Native(native)) = &ctor.borrow().exotic {
                    if native.name.as_str() == "eval" {
                        return Err(JsError::type_error("eval is not a constructor"));
                    }
                }

                // Create guard for OpResult values
                let guard = interp.heap.create_guard();
                guard.guard(ctor.cheap_clone());

                // Create a new object
                let new_obj = interp.create_object(&guard);

                // Get the constructor's prototype
                let proto_key = PropertyKey::String(interp.intern("prototype"));
                if let Some(JsValue::Object(proto)) = ctor.borrow().get_property(&proto_key) {
                    new_obj.borrow_mut().prototype = Some(proto.cheap_clone());
                }

                // Guard all object values in args
                for arg in &args {
                    arg.guard_by(&guard);
                }

                // Use trampoline for constructor call
                let this = JsValue::Object(new_obj.cheap_clone());
                Ok(OpResult::Construct {
                    callee: callee_val.clone(),
                    this_value: this,
                    args,
                    return_register: dst,
                    new_target: callee_val.clone(), // new.target is the constructor itself
                    new_obj,
                    guard,
                })
            }

            Op::Return { value } => {
                let return_val = self.get_reg(value).clone();
                // NOTE: review
                self.execute_return(return_val, interp)
            }

            Op::ReturnUndefined => self.execute_return(JsValue::Undefined, interp),

            Op::CreateClosure { dst, chunk_idx } => {
                // Get the function bytecode chunk from constants
                let chunk = match self.get_constant(chunk_idx) {
                    Some(Constant::Chunk(c)) => c.clone(),
                    _ => return Err(JsError::internal_error("Invalid closure chunk index")),
                };

                // Create a BytecodeFunction with the current environment as closure
                let bc_func = BytecodeFunction {
                    chunk,
                    closure: interp.env.cheap_clone(),
                    captured_this: None, // Regular functions don't capture this
                };

                // Create function object
                let guard = interp.heap.create_guard();
                let func_obj = interp.create_bytecode_function(&guard, bc_func);
                self.set_reg(dst, JsValue::Object(func_obj));
                Ok(OpResult::Continue)
            }

            Op::CreateArrow { dst, chunk_idx } => {
                // Get the function bytecode chunk from constants
                let chunk = match self.get_constant(chunk_idx) {
                    Some(Constant::Chunk(c)) => c.clone(),
                    _ => return Err(JsError::internal_error("Invalid arrow chunk index")),
                };

                // Check if this is an async arrow function
                let is_async = chunk
                    .function_info
                    .as_ref()
                    .is_some_and(|info| info.is_async);

                // Arrow functions capture lexical this
                let bc_func = BytecodeFunction {
                    chunk,
                    closure: interp.env.cheap_clone(),
                    captured_this: Some(Box::new(self.this_value.clone())),
                };

                // Create function object - use async variant for async arrow functions
                let guard = interp.heap.create_guard();
                let func_obj = if is_async {
                    interp.create_bytecode_async_function(&guard, bc_func)
                } else {
                    interp.create_bytecode_function(&guard, bc_func)
                };
                self.set_reg(dst, JsValue::Object(func_obj));
                Ok(OpResult::Continue)
            }

            Op::CreateGenerator { dst, chunk_idx } => {
                // Get the generator function bytecode chunk from constants
                let chunk = match self.get_constant(chunk_idx) {
                    Some(Constant::Chunk(c)) => c.clone(),
                    _ => return Err(JsError::internal_error("Invalid generator chunk index")),
                };

                // Create a BytecodeGenerator function with the current environment as closure
                let bc_func = BytecodeFunction {
                    chunk,
                    closure: interp.env.cheap_clone(),
                    captured_this: None,
                };

                // Create function object with the BytecodeGenerator variant
                let guard = interp.heap.create_guard();
                let func_obj = interp.create_bytecode_generator_function(&guard, bc_func);
                self.set_reg(dst, JsValue::Object(func_obj));
                Ok(OpResult::Continue)
            }

            Op::CreateAsync { dst, chunk_idx } => {
                // Get the async function bytecode chunk from constants
                let chunk = match self.get_constant(chunk_idx) {
                    Some(Constant::Chunk(c)) => c.clone(),
                    _ => {
                        return Err(JsError::internal_error(
                            "Invalid async function chunk index",
                        ))
                    }
                };

                // Create a BytecodeAsync function with the current environment as closure
                let bc_func = BytecodeFunction {
                    chunk,
                    closure: interp.env.cheap_clone(),
                    captured_this: None,
                };

                // Create function object with the BytecodeAsync variant
                let guard = interp.heap.create_guard();
                let func_obj = interp.create_bytecode_async_function(&guard, bc_func);
                self.set_reg(dst, JsValue::Object(func_obj));
                Ok(OpResult::Continue)
            }

            Op::CreateAsyncGenerator { dst, chunk_idx } => {
                // Get the async generator function bytecode chunk from constants
                let chunk = match self.get_constant(chunk_idx) {
                    Some(Constant::Chunk(c)) => c.clone(),
                    _ => {
                        return Err(JsError::internal_error(
                            "Invalid async generator chunk index",
                        ))
                    }
                };

                // Create a BytecodeAsyncGenerator function with the current environment as closure
                let bc_func = BytecodeFunction {
                    chunk,
                    closure: interp.env.cheap_clone(),
                    captured_this: None,
                };

                // Create function object with the BytecodeAsyncGenerator variant
                let guard = interp.heap.create_guard();
                let func_obj = interp.create_bytecode_async_generator_function(&guard, bc_func);
                self.set_reg(dst, JsValue::Object(func_obj));
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Exception Handling
            // ═══════════════════════════════════════════════════════════════════════════
            Op::Throw { value } => {
                let val = self.get_reg(value).clone();
                let guarded = Guarded::from_value(val, &interp.heap);
                Err(JsError::ThrownValue { guarded })
            }

            Op::PushTry {
                catch_target,
                finally_target,
            } => {
                self.try_stack.push(TryHandler {
                    catch_ip: catch_target as usize,
                    finally_ip: finally_target as usize,
                    registers_snapshot: self.registers.len(),
                    frame_depth: self.call_stack.len(),
                    scope_depth: self.saved_env_stack.len(),
                    iterator_reg: None,
                });
                Ok(OpResult::Continue)
            }

            Op::PopTry => {
                self.try_stack.pop();
                Ok(OpResult::Continue)
            }

            Op::PushIterTry {
                iterator,
                catch_target,
            } => {
                self.try_stack.push(TryHandler {
                    catch_ip: catch_target as usize,
                    finally_ip: 0, // No finally for iterator try
                    registers_snapshot: self.registers.len(),
                    frame_depth: self.call_stack.len(),
                    scope_depth: self.saved_env_stack.len(),
                    iterator_reg: Some(iterator),
                });
                Ok(OpResult::Continue)
            }

            Op::PopIterTry => {
                // Pop the iterator try handler (normal completion, no exception)
                self.try_stack.pop();
                Ok(OpResult::Continue)
            }

            Op::FinallyEnd => {
                // Complete any pending return/throw/break/continue after finally block finishes
                if let Some(pending) = self.pending_completion.take() {
                    match pending {
                        PendingCompletion::Return(guarded) => {
                            // Continue with the return (recursively handles nested finally blocks)
                            return self.execute_return(guarded.value, interp);
                        }
                        PendingCompletion::Throw(guarded) => {
                            // Re-throw the exception after finally
                            return Err(JsError::ThrownValue { guarded });
                        }
                        PendingCompletion::Break { target, try_depth } => {
                            // Continue with the break (recursively handles nested finally blocks)
                            return self.execute_break(target, try_depth);
                        }
                        PendingCompletion::Continue { target, try_depth } => {
                            // Continue with the continue (recursively handles nested finally blocks)
                            return self.execute_continue(target, try_depth);
                        }
                    }
                }
                Ok(OpResult::Continue)
            }

            Op::GetException { dst } => {
                let val = self
                    .exception_value
                    .take()
                    .map(|g| g.value)
                    .unwrap_or(JsValue::Undefined);
                self.set_reg(dst, val);
                Ok(OpResult::Continue)
            }

            Op::Rethrow => {
                if let Some(guarded) = self.exception_value.take() {
                    Err(JsError::ThrownValue { guarded })
                } else {
                    Err(JsError::internal_error("No exception to rethrow"))
                }
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Async/Generator
            // ═══════════════════════════════════════════════════════════════════════════
            Op::Await { dst, promise } => {
                use crate::value::{ExoticObject, PromiseStatus};

                let promise_val = self.get_reg(promise);

                // Check if it's a promise
                if let JsValue::Object(obj) = promise_val {
                    let obj_ref = obj.borrow();
                    if let ExoticObject::Promise(state) = &obj_ref.exotic {
                        let state_ref = state.borrow();
                        match state_ref.status {
                            PromiseStatus::Fulfilled => {
                                // Extract the resolved value
                                let result = state_ref.result.clone().unwrap_or(JsValue::Undefined);
                                drop(state_ref);
                                drop(obj_ref);
                                self.set_reg(dst, result);
                                return Ok(OpResult::Continue);
                            }
                            PromiseStatus::Rejected => {
                                // Throw the rejection reason
                                let reason = state_ref.result.clone().unwrap_or(JsValue::Undefined);
                                drop(state_ref);
                                drop(obj_ref);
                                let guarded = Guarded::from_value(reason, &interp.heap);
                                return Err(JsError::thrown(guarded));
                            }
                            PromiseStatus::Pending => {
                                // Suspend execution and wait for promise resolution
                                drop(state_ref);
                                drop(obj_ref);
                                return Ok(OpResult::Suspend {
                                    promise: Guarded::from_value(promise_val.clone(), &interp.heap),
                                    resume_register: dst,
                                });
                            }
                        }
                    }
                }

                // Not a promise - treat as resolved value (await 42 === 42)
                self.set_reg(dst, promise_val.clone());
                Ok(OpResult::Continue)
            }

            Op::Yield { dst, value } => {
                let yield_val = self.get_reg(value).clone();
                // Return a Yield result - the generator will be suspended
                // The dst register will receive the value passed to next() when resumed
                Ok(OpResult::Yield {
                    value: Guarded::from_value(yield_val, &interp.heap),
                    resume_register: dst,
                })
            }

            Op::YieldStar { dst, iterable } => {
                // yield* delegates to another iterator
                let iterable_val = self.get_reg(iterable).clone();
                Ok(OpResult::YieldStar {
                    iterable: Guarded::from_value(iterable_val, &interp.heap),
                    resume_register: dst,
                })
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Scope Management
            // ═══════════════════════════════════════════════════════════════════════════
            Op::PushScope => {
                let env = interp.push_scope();
                // Push the saved environment onto the stack
                self.saved_env_stack.push(env);
                Ok(OpResult::Continue)
            }

            Op::PopScope => {
                if let Some(env) = self.saved_env_stack.pop() {
                    interp.pop_scope(env);
                }
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Iteration
            // ═══════════════════════════════════════════════════════════════════════════
            Op::GetIterator { dst, obj } => {
                let obj_val = self.get_reg(obj);

                // For arrays and strings, create an internal array iterator
                // The iterator is stored as an object with internal index state
                match obj_val {
                    JsValue::Object(obj_ref) => {
                        // Check if it's a proxy first - need to get Symbol.iterator through proxy trap
                        let is_proxy = matches!(obj_ref.borrow().exotic, ExoticObject::Proxy(_));

                        if is_proxy {
                            // For proxies, use proxy_get to get Symbol.iterator method
                            let well_known = interp.well_known_symbols;
                            let iterator_symbol = crate::value::JsSymbol::new(
                                well_known.iterator,
                                Some(interp.intern("Symbol.iterator")),
                            );
                            let iterator_key = PropertyKey::Symbol(Box::new(iterator_symbol));

                            let iterator_method_result =
                                crate::interpreter::builtins::proxy::proxy_get(
                                    interp,
                                    obj_ref.cheap_clone(),
                                    iterator_key,
                                    obj_val.clone(),
                                )?;

                            if let JsValue::Object(method_obj) = iterator_method_result.value {
                                // Call the iterator method with the proxy as `this`
                                let Guarded {
                                    value,
                                    guard: _guard,
                                } = interp.call_function(
                                    JsValue::Object(method_obj),
                                    obj_val.clone(),
                                    &[],
                                )?;
                                self.set_reg(dst, value);
                            } else {
                                return Err(JsError::type_error("Object is not iterable"));
                            }
                            return Ok(OpResult::Continue);
                        }

                        // Check if it's an array - use direct element iteration
                        if obj_ref.borrow().array_elements().is_some() {
                            // Create an iterator object with the array and index
                            // Use register_guard to keep it alive across loop iterations
                            let guard = interp.heap.create_guard();
                            let iter = interp.create_object(&guard);
                            iter.borrow_mut().set_property(
                                PropertyKey::String(interp.intern("__array__")),
                                JsValue::Object(obj_ref.clone()),
                            );
                            iter.borrow_mut().set_property(
                                PropertyKey::String(interp.intern("__index__")),
                                JsValue::Number(0.0),
                            );
                            self.set_reg(dst, JsValue::Object(iter));
                            return Ok(OpResult::Continue);
                        }

                        // For non-array objects, try Symbol.iterator
                        let well_known = interp.well_known_symbols;
                        let iterator_symbol = crate::value::JsSymbol::new(
                            well_known.iterator,
                            Some(interp.intern("Symbol.iterator")),
                        );
                        let iterator_key = PropertyKey::Symbol(Box::new(iterator_symbol));

                        let iterator_method = obj_ref.borrow().get_property(&iterator_key);

                        if let Some(JsValue::Object(method_obj)) = iterator_method {
                            // Call the iterator method
                            let Guarded {
                                value,
                                guard: _guard,
                            } = interp.call_function(
                                JsValue::Object(method_obj),
                                obj_val.clone(),
                                &[],
                            )?;
                            self.set_reg(dst, value);
                        } else {
                            return Err(JsError::type_error("Object is not iterable"));
                        }
                    }
                    JsValue::String(s) => {
                        // Create a string iterator
                        let guard = interp.heap.create_guard();
                        let iter = interp.create_object(&guard);
                        iter.borrow_mut().set_property(
                            PropertyKey::String(interp.intern("__string__")),
                            JsValue::String(s.cheap_clone()),
                        );
                        iter.borrow_mut().set_property(
                            PropertyKey::String(interp.intern("__index__")),
                            JsValue::Number(0.0),
                        );
                        self.set_reg(dst, JsValue::Object(iter));
                    }
                    _ => {
                        return Err(JsError::type_error("Object is not iterable"));
                    }
                }
                Ok(OpResult::Continue)
            }

            Op::GetKeysIterator { dst, obj } => {
                let obj_val = self.get_reg(obj);

                // Create a keys iterator that iterates over enumerable property keys
                let keys: Vec<JsValue> = match obj_val {
                    JsValue::Object(obj_ref) => {
                        // Check if this is a proxy - use proxy_own_keys if so
                        let is_proxy = matches!(obj_ref.borrow().exotic, ExoticObject::Proxy(_));

                        if is_proxy {
                            // Get keys through proxy trap
                            let Guarded {
                                value,
                                guard: _guard,
                            } = crate::interpreter::builtins::proxy::proxy_own_keys(
                                interp,
                                obj_ref.cheap_clone(),
                            )?;

                            // proxy_own_keys returns an array of keys
                            if let JsValue::Object(keys_arr) = value {
                                if let Some(elements) = keys_arr.borrow().array_elements() {
                                    elements.to_vec()
                                } else {
                                    Vec::new()
                                }
                            } else {
                                Vec::new()
                            }
                        } else {
                            let obj_borrowed = obj_ref.borrow();
                            let mut result = Vec::new();

                            // For arrays, first add all array indices
                            if let Some(elements) = obj_borrowed.array_elements() {
                                for i in 0..elements.len() {
                                    result.push(JsValue::String(JsString::from(i.to_string())));
                                }
                            }

                            // Then add own enumerable property keys (excluding indices already added)
                            for (k, prop) in obj_borrowed.properties.iter() {
                                // Only include enumerable properties
                                if !prop.enumerable() {
                                    continue;
                                }
                                match k {
                                    PropertyKey::String(s) => {
                                        result.push(JsValue::String(s.cheap_clone()));
                                    }
                                    PropertyKey::Index(i) => {
                                        // Only add if not an array (arrays already handled above)
                                        if obj_borrowed.array_elements().is_none() {
                                            result.push(JsValue::String(JsString::from(
                                                i.to_string(),
                                            )));
                                        }
                                    }
                                    _ => {} // Skip symbols for for-in
                                }
                            }
                            result
                        }
                    }
                    JsValue::String(s) => {
                        // For strings, iterate over character indices
                        (0..s.as_str().chars().count())
                            .map(|i| JsValue::String(JsString::from(i.to_string())))
                            .collect()
                    }
                    JsValue::Null | JsValue::Undefined => {
                        // for-in on null/undefined should just not iterate
                        Vec::new()
                    }
                    _ => Vec::new(),
                };

                // Create a keys array iterator
                let guard = interp.heap.create_guard();
                let iter = interp.create_object(&guard);
                let keys_arr = interp.create_array_from(&self.register_guard, keys);
                iter.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("__keys__")),
                    JsValue::Object(keys_arr),
                );
                iter.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("__index__")),
                    JsValue::Number(0.0),
                );
                self.set_reg(dst, JsValue::Object(iter));
                Ok(OpResult::Continue)
            }

            Op::GetAsyncIterator { dst, obj } => {
                let obj_val = self.get_reg(obj);

                // For async iteration, we first try Symbol.asyncIterator, then fall back to Symbol.iterator.
                // For arrays without Symbol.asyncIterator, we use the same internal iterator as sync iteration.
                // The Await opcode that follows IteratorNext will handle awaiting each value.

                match obj_val {
                    JsValue::Object(obj_ref) => {
                        // Check if it's a proxy first
                        let is_proxy = matches!(obj_ref.borrow().exotic, ExoticObject::Proxy(_));

                        if is_proxy {
                            // For proxies, try Symbol.asyncIterator first, then Symbol.iterator
                            let well_known = interp.well_known_symbols;

                            // Try Symbol.asyncIterator first
                            let async_iterator_symbol = crate::value::JsSymbol::new(
                                well_known.async_iterator,
                                Some(interp.intern("Symbol.asyncIterator")),
                            );
                            let async_iterator_key =
                                PropertyKey::Symbol(Box::new(async_iterator_symbol));

                            let async_method_result =
                                crate::interpreter::builtins::proxy::proxy_get(
                                    interp,
                                    obj_ref.cheap_clone(),
                                    async_iterator_key,
                                    obj_val.clone(),
                                )?;

                            if let JsValue::Object(method_obj) = async_method_result.value {
                                // Call the async iterator method with the proxy as `this`
                                let Guarded {
                                    value,
                                    guard: _guard,
                                } = interp.call_function(
                                    JsValue::Object(method_obj),
                                    obj_val.clone(),
                                    &[],
                                )?;
                                self.set_reg(dst, value);
                                return Ok(OpResult::Continue);
                            }

                            // Fall back to Symbol.iterator
                            let iterator_symbol = crate::value::JsSymbol::new(
                                well_known.iterator,
                                Some(interp.intern("Symbol.iterator")),
                            );
                            let iterator_key = PropertyKey::Symbol(Box::new(iterator_symbol));

                            let iterator_method_result =
                                crate::interpreter::builtins::proxy::proxy_get(
                                    interp,
                                    obj_ref.cheap_clone(),
                                    iterator_key,
                                    obj_val.clone(),
                                )?;

                            if let JsValue::Object(method_obj) = iterator_method_result.value {
                                let Guarded {
                                    value,
                                    guard: _guard,
                                } = interp.call_function(
                                    JsValue::Object(method_obj),
                                    obj_val.clone(),
                                    &[],
                                )?;
                                self.set_reg(dst, value);
                            } else {
                                return Err(JsError::type_error("Object is not async iterable"));
                            }
                            return Ok(OpResult::Continue);
                        }

                        // Check for Symbol.asyncIterator first
                        let well_known = interp.well_known_symbols;
                        let async_iterator_symbol = crate::value::JsSymbol::new(
                            well_known.async_iterator,
                            Some(interp.intern("Symbol.asyncIterator")),
                        );
                        let async_iterator_key =
                            PropertyKey::Symbol(Box::new(async_iterator_symbol));

                        let async_method = obj_ref.borrow().get_property(&async_iterator_key);

                        if let Some(JsValue::Object(method_obj)) = async_method {
                            // Call the async iterator method
                            let Guarded {
                                value,
                                guard: _guard,
                            } = interp.call_function(
                                JsValue::Object(method_obj),
                                obj_val.clone(),
                                &[],
                            )?;
                            self.set_reg(dst, value);
                            return Ok(OpResult::Continue);
                        }

                        // Check if it's an array - use direct element iteration
                        // The Await opcode will handle awaiting each element (promise or plain value)
                        if obj_ref.borrow().array_elements().is_some() {
                            let guard = interp.heap.create_guard();
                            let iter = interp.create_object(&guard);
                            iter.borrow_mut().set_property(
                                PropertyKey::String(interp.intern("__array__")),
                                JsValue::Object(obj_ref.clone()),
                            );
                            iter.borrow_mut().set_property(
                                PropertyKey::String(interp.intern("__index__")),
                                JsValue::Number(0.0),
                            );
                            self.set_reg(dst, JsValue::Object(iter));
                            return Ok(OpResult::Continue);
                        }

                        // Try Symbol.iterator as fallback
                        let iterator_symbol = crate::value::JsSymbol::new(
                            well_known.iterator,
                            Some(interp.intern("Symbol.iterator")),
                        );
                        let iterator_key = PropertyKey::Symbol(Box::new(iterator_symbol));

                        let iterator_method = obj_ref.borrow().get_property(&iterator_key);

                        if let Some(JsValue::Object(method_obj)) = iterator_method {
                            let Guarded {
                                value,
                                guard: _guard,
                            } = interp.call_function(
                                JsValue::Object(method_obj),
                                obj_val.clone(),
                                &[],
                            )?;
                            self.set_reg(dst, value);
                        } else {
                            return Err(JsError::type_error("Object is not async iterable"));
                        }
                    }
                    JsValue::String(s) => {
                        // Create a string iterator (same as sync - Await will handle values)
                        let guard = interp.heap.create_guard();
                        let iter = interp.create_object(&guard);
                        iter.borrow_mut().set_property(
                            PropertyKey::String(interp.intern("__string__")),
                            JsValue::String(s.cheap_clone()),
                        );
                        iter.borrow_mut().set_property(
                            PropertyKey::String(interp.intern("__index__")),
                            JsValue::Number(0.0),
                        );
                        self.set_reg(dst, JsValue::Object(iter));
                    }
                    _ => {
                        return Err(JsError::type_error("Object is not async iterable"));
                    }
                }
                Ok(OpResult::Continue)
            }

            Op::IteratorNext { dst, iterator } => {
                let iter_val = self.get_reg(iterator);

                let JsValue::Object(iter_obj) = iter_val else {
                    return Err(JsError::type_error("Iterator is not an object"));
                };

                // Check if this is our internal array iterator
                let array_prop = iter_obj
                    .borrow()
                    .get_property(&PropertyKey::String(interp.intern("__array__")));
                if let Some(JsValue::Object(arr_ref)) = array_prop {
                    // If the array is a proxy, fall through to custom iterator path
                    // which will call the next() method (which handles proxies properly)
                    let index_key = PropertyKey::String(interp.intern("__index__"));
                    let is_proxy = matches!(arr_ref.borrow().exotic, ExoticObject::Proxy(_));
                    if !is_proxy {
                        let index = match iter_obj.borrow().get_property(&index_key) {
                            Some(JsValue::Number(n)) => n as usize,
                            _ => 0,
                        };

                        let elements = arr_ref.borrow().array_elements().map(|e| e.to_vec());
                        let (value, done) = if let Some(elems) = elements {
                            if index < elems.len() {
                                let val = elems.get(index).cloned().unwrap_or(JsValue::Undefined);
                                (val, false)
                            } else {
                                (JsValue::Undefined, true)
                            }
                        } else {
                            (JsValue::Undefined, true)
                        };

                        // Update index
                        iter_obj
                            .borrow_mut()
                            .set_property(index_key, JsValue::Number((index + 1) as f64));

                        // Create result object { value, done }
                        let guard = interp.heap.create_guard();
                        let result = interp.create_object(&guard);
                        result
                            .borrow_mut()
                            .set_property(PropertyKey::String(interp.intern("value")), value);
                        result.borrow_mut().set_property(
                            PropertyKey::String(interp.intern("done")),
                            JsValue::Boolean(done),
                        );
                        self.set_reg(dst, JsValue::Object(result));
                        return Ok(OpResult::Continue);
                    }
                }

                // Check if this is our internal string iterator
                let string_prop = iter_obj
                    .borrow()
                    .get_property(&PropertyKey::String(interp.intern("__string__")));
                if let Some(JsValue::String(s)) = string_prop {
                    let index_key = PropertyKey::String(interp.intern("__index__"));
                    let index = match iter_obj.borrow().get_property(&index_key) {
                        Some(JsValue::Number(n)) => n as usize,
                        _ => 0,
                    };

                    let chars: Vec<char> = s.as_str().chars().collect();
                    let (value, done) = if index < chars.len() {
                        let val = chars
                            .get(index)
                            .map(|c| JsValue::String(JsString::from(c.to_string())))
                            .unwrap_or(JsValue::Undefined);
                        (val, false)
                    } else {
                        (JsValue::Undefined, true)
                    };

                    // Update index
                    iter_obj
                        .borrow_mut()
                        .set_property(index_key, JsValue::Number((index + 1) as f64));

                    // Create result object { value, done }
                    let guard = interp.heap.create_guard();
                    let result = interp.create_object(&guard);
                    let value_key = interp.property_key("value");
                    let done_key = interp.property_key("done");
                    result.borrow_mut().set_property(value_key, value);
                    result
                        .borrow_mut()
                        .set_property(done_key, JsValue::Boolean(done));
                    self.set_reg(dst, JsValue::Object(result));
                    return Ok(OpResult::Continue);
                }

                // Check if this is our internal keys iterator (for for-in)
                let index_key = PropertyKey::String(interp.intern("__index__"));
                let keys_prop = iter_obj
                    .borrow()
                    .get_property(&PropertyKey::String(interp.intern("__keys__")));
                if let Some(JsValue::Object(keys_arr)) = keys_prop {
                    let index = match iter_obj.borrow().get_property(&index_key) {
                        Some(JsValue::Number(n)) => n as usize,
                        _ => 0,
                    };

                    let elements = keys_arr.borrow().array_elements().map(|e| e.to_vec());
                    let (value, done) = if let Some(elems) = elements {
                        if index < elems.len() {
                            let val = elems.get(index).cloned().unwrap_or(JsValue::Undefined);
                            (val, false)
                        } else {
                            (JsValue::Undefined, true)
                        }
                    } else {
                        (JsValue::Undefined, true)
                    };

                    // Update index
                    iter_obj
                        .borrow_mut()
                        .set_property(index_key, JsValue::Number((index + 1) as f64));

                    // Create result object { value, done }
                    let guard = interp.heap.create_guard();
                    let result = interp.create_object(&guard);
                    let value_key = interp.property_key("value");
                    let done_key = interp.property_key("done");
                    result.borrow_mut().set_property(value_key, value);
                    result
                        .borrow_mut()
                        .set_property(done_key, JsValue::Boolean(done));
                    self.set_reg(dst, JsValue::Object(result));
                    return Ok(OpResult::Continue);
                }

                // For custom iterators, call next() method
                let next_method = iter_obj
                    .borrow()
                    .get_property(&PropertyKey::String(interp.intern("next")));

                if let Some(JsValue::Object(next_fn)) = next_method {
                    let Guarded {
                        value,
                        guard: _guard,
                    } = interp.call_function(
                        JsValue::Object(next_fn),
                        JsValue::Object(iter_obj.clone()),
                        &[],
                    )?;
                    self.set_reg(dst, value);
                } else {
                    return Err(JsError::type_error("Iterator must have a next method"));
                }

                Ok(OpResult::Continue)
            }

            Op::IteratorDone { result, target } => {
                let result_val = self.get_reg(result);

                let done = if let JsValue::Object(obj_ref) = result_val {
                    match obj_ref
                        .borrow()
                        .get_property(&PropertyKey::String(interp.intern("done")))
                    {
                        Some(JsValue::Boolean(b)) => b,
                        _ => false,
                    }
                } else {
                    true
                };

                if done {
                    self.ip = target as usize;
                }
                Ok(OpResult::Continue)
            }

            Op::IteratorValue { dst, result } => {
                let result_val = self.get_reg(result);

                let value = if let JsValue::Object(obj_ref) = result_val {
                    obj_ref
                        .borrow()
                        .get_property(&PropertyKey::String(interp.intern("value")))
                        .unwrap_or(JsValue::Undefined)
                } else {
                    JsValue::Undefined
                };

                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            Op::IteratorClose { iterator } => {
                let iter_val = self.get_reg(iterator);

                // Check if iterator has a return() method
                if let JsValue::Object(iter_obj) = iter_val {
                    // For internal array/string/keys iterators, there's no return method
                    // Only check for custom iterators that have a 'return' property
                    let return_key = PropertyKey::String(interp.intern("return"));
                    let return_method = iter_obj.borrow().get_property(&return_key);

                    if let Some(JsValue::Object(return_fn)) = return_method {
                        // Call the return method
                        // Per ES spec 7.4.6: If Type(innerResult.[[value]]) is not Object,
                        // throw a TypeError exception.
                        let Guarded {
                            value,
                            guard: _guard,
                        } = interp.call_function(
                            JsValue::Object(return_fn),
                            JsValue::Object(iter_obj.clone()),
                            &[],
                        )?;

                        // Check that result is an object
                        if !matches!(value, JsValue::Object(_)) {
                            return Err(JsError::type_error("Iterator result is not an object"));
                        }
                    }
                }

                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Class Operations
            // ═══════════════════════════════════════════════════════════════════════════
            Op::CreateClass {
                dst,
                constructor,
                super_class,
            } => {
                // Get constructor function - it should be a function object
                let ctor_val = self.get_reg(constructor);
                let JsValue::Object(ctor_obj) = ctor_val else {
                    return Err(JsError::type_error("Class constructor must be a function"));
                };

                // Create prototype object
                let guard = interp.heap.create_guard();
                let prototype = interp.create_object(&guard);

                // Handle superclass if provided
                let super_val = self.get_reg(super_class);
                if !matches!(super_val, JsValue::Undefined) {
                    let JsValue::Object(super_ctor) = &super_val else {
                        return Err(JsError::type_error(
                            "Class extends value is not a constructor",
                        ));
                    };

                    // Set prototype chain: prototype.__proto__ = superClass.prototype
                    let proto_key = PropertyKey::String(interp.intern("prototype"));
                    if let Some(JsValue::Object(super_proto)) =
                        super_ctor.borrow().get_property(&proto_key)
                    {
                        prototype.borrow_mut().prototype = Some(super_proto.cheap_clone());
                    }

                    // Store __super__ on constructor for super() calls
                    ctor_obj.borrow_mut().set_property(
                        PropertyKey::String(interp.intern("__super__")),
                        JsValue::Object(super_ctor.cheap_clone()),
                    );

                    // Store __super_target__ for super.x property access
                    if let Some(sp) = super_ctor
                        .borrow()
                        .get_property(&PropertyKey::String(interp.intern("prototype")))
                    {
                        ctor_obj.borrow_mut().set_property(
                            PropertyKey::String(interp.intern("__super_target__")),
                            sp,
                        );
                    }
                }

                // Set constructor.prototype = prototype (non-writable, non-enumerable, non-configurable per spec)
                ctor_obj.borrow_mut().define_property(
                    PropertyKey::String(interp.intern("prototype")),
                    Property::with_attributes(
                        JsValue::Object(prototype.cheap_clone()),
                        false,
                        false,
                        false,
                    ),
                );

                // Set prototype.constructor = constructor (non-enumerable, writable, configurable per spec)
                prototype.borrow_mut().define_property(
                    PropertyKey::String(interp.intern("constructor")),
                    Property::with_attributes(
                        JsValue::Object(ctor_obj.cheap_clone()),
                        true,
                        false,
                        true,
                    ),
                );

                self.set_reg(dst, JsValue::Object(ctor_obj.clone()));
                Ok(OpResult::Continue)
            }

            Op::DefineMethod {
                class,
                name,
                method,
                is_static,
            } => {
                let class_val = self.get_reg(class);
                let JsValue::Object(class_obj) = class_val else {
                    return Err(JsError::type_error("Class is not an object"));
                };

                let method_val = self.get_reg(method);
                let method_name = self
                    .get_string_constant(name)
                    .ok_or_else(|| JsError::internal_error("Invalid method name constant"))?;

                // Store __super__ and __super_target__ on method for super access
                if let JsValue::Object(method_obj) = method_val {
                    // Copy __super__ from class constructor
                    let super_key = PropertyKey::String(interp.intern("__super__"));
                    if let Some(super_val) = class_obj.borrow().get_property(&super_key) {
                        method_obj
                            .borrow_mut()
                            .set_property(super_key.clone(), super_val.clone());

                        // For static methods, __super_target__ = parent constructor (__super__)
                        // For instance methods, __super_target__ = parent prototype (from class)
                        if is_static {
                            method_obj.borrow_mut().set_property(
                                PropertyKey::String(interp.intern("__super_target__")),
                                super_val,
                            );
                        } else if let Some(super_target) = class_obj
                            .borrow()
                            .get_property(&PropertyKey::String(interp.intern("__super_target__")))
                        {
                            method_obj.borrow_mut().set_property(
                                PropertyKey::String(interp.intern("__super_target__")),
                                super_target,
                            );
                        }
                    }
                }

                // Use from_value to handle numeric string keys correctly (e.g., "2" -> Index(2))
                let prop_key =
                    interp.property_key_from_value(&JsValue::String(method_name.cheap_clone()));

                if is_static {
                    // Add to class constructor directly
                    // Methods are non-enumerable, writable, configurable (per spec)
                    class_obj.borrow_mut().define_property(
                        prop_key,
                        Property::with_attributes(method_val.clone(), true, false, true),
                    );
                } else {
                    // Add to prototype
                    // Methods are non-enumerable, writable, configurable (per spec)
                    let proto_key = PropertyKey::String(interp.intern("prototype"));
                    if let Some(JsValue::Object(proto)) =
                        class_obj.borrow().get_property(&proto_key)
                    {
                        proto.borrow_mut().define_property(
                            prop_key,
                            Property::with_attributes(method_val.clone(), true, false, true),
                        );
                    }
                }

                Ok(OpResult::Continue)
            }

            Op::DefineAccessor {
                class,
                name,
                getter,
                setter,
                is_static,
            } => {
                let class_val = self.get_reg(class);
                let JsValue::Object(class_obj) = class_val else {
                    return Err(JsError::type_error("Class is not an object"));
                };

                let getter_val = self.get_reg(getter);
                let setter_val = self.get_reg(setter);
                let accessor_name = self
                    .get_string_constant(name)
                    .ok_or_else(|| JsError::internal_error("Invalid accessor name constant"))?;

                // Extract function objects (undefined means keep existing)
                let new_getter = if let JsValue::Object(g) = getter_val {
                    Some(g)
                } else {
                    None
                };
                let new_setter = if let JsValue::Object(s) = setter_val {
                    Some(s)
                } else {
                    None
                };

                // Get target object (class for static, prototype for instance)
                let target = if is_static {
                    class_obj.cheap_clone()
                } else {
                    let proto_key = PropertyKey::String(interp.intern("prototype"));
                    if let Some(JsValue::Object(proto)) =
                        class_obj.borrow().get_property(&proto_key)
                    {
                        proto
                    } else {
                        return Ok(OpResult::Continue);
                    }
                };

                // Get existing accessor property if any
                // Use from_value to handle numeric string keys correctly (e.g., "2" -> Index(2))
                let prop_key =
                    interp.property_key_from_value(&JsValue::String(accessor_name.cheap_clone()));
                let (existing_getter, existing_setter) = {
                    let target_ref = target.borrow();
                    if let Some(prop) = target_ref.properties.get(&prop_key) {
                        (prop.getter().cloned(), prop.setter().cloned())
                    } else {
                        (None, None)
                    }
                };

                // Merge with existing accessors
                let final_getter = new_getter.cloned().or(existing_getter);
                let final_setter = new_setter.cloned().or(existing_setter);

                // Create accessor property
                let property = Property::accessor(final_getter, final_setter);
                target.borrow_mut().define_property(prop_key, property);

                Ok(OpResult::Continue)
            }

            Op::DefineMethodComputed {
                class,
                key,
                method,
                is_static,
            } => {
                let class_val = self.get_reg(class);
                let JsValue::Object(class_obj) = class_val else {
                    return Err(JsError::type_error("Class is not an object"));
                };

                let method_val = self.get_reg(method);
                let key_val = self.get_reg(key);

                // Convert key to string for property name
                let method_name = interp.to_js_string(key_val);

                // Store __super__ and __super_target__ on method for super access
                if let JsValue::Object(method_obj) = &method_val {
                    // Copy __super__ from class constructor
                    let super_key = PropertyKey::String(interp.intern("__super__"));
                    if let Some(super_val) = class_obj.borrow().get_property(&super_key) {
                        method_obj
                            .borrow_mut()
                            .set_property(super_key.clone(), super_val.clone());

                        // For static methods, __super_target__ = parent constructor (__super__)
                        // For instance methods, __super_target__ = parent prototype (from class)
                        if is_static {
                            method_obj.borrow_mut().set_property(
                                PropertyKey::String(interp.intern("__super_target__")),
                                super_val,
                            );
                        } else if let Some(super_target) = class_obj
                            .borrow()
                            .get_property(&PropertyKey::String(interp.intern("__super_target__")))
                        {
                            method_obj.borrow_mut().set_property(
                                PropertyKey::String(interp.intern("__super_target__")),
                                super_target,
                            );
                        }
                    }
                }

                // Use from_value to handle numeric string keys correctly (e.g., "2" -> Index(2))
                let prop_key = interp.property_key_from_value(&JsValue::String(method_name));

                if is_static {
                    // Add to class constructor directly
                    // Methods are non-enumerable, writable, configurable (per spec)
                    class_obj.borrow_mut().define_property(
                        prop_key,
                        Property::with_attributes(method_val.clone(), true, false, true),
                    );
                } else {
                    // Add to prototype
                    // Methods are non-enumerable, writable, configurable (per spec)
                    let proto_key = PropertyKey::String(interp.intern("prototype"));
                    if let Some(JsValue::Object(proto)) =
                        class_obj.borrow().get_property(&proto_key)
                    {
                        proto.borrow_mut().define_property(
                            prop_key,
                            Property::with_attributes(method_val.clone(), true, false, true),
                        );
                    }
                }

                Ok(OpResult::Continue)
            }

            Op::DefineAccessorComputed {
                class,
                key,
                getter,
                setter,
                is_static,
            } => {
                let class_val = self.get_reg(class);
                let JsValue::Object(class_obj) = class_val else {
                    return Err(JsError::type_error("Class is not an object"));
                };

                let getter_val = self.get_reg(getter);
                let setter_val = self.get_reg(setter);
                let key_val = self.get_reg(key);

                // Convert key to string for accessor name
                let accessor_name = interp.to_js_string(key_val);

                // Extract function objects (undefined means keep existing)
                let new_getter = if let JsValue::Object(g) = getter_val {
                    Some(g)
                } else {
                    None
                };
                let new_setter = if let JsValue::Object(s) = setter_val {
                    Some(s)
                } else {
                    None
                };

                // Get target object (class for static, prototype for instance)
                let target = if is_static {
                    class_obj.cheap_clone()
                } else {
                    let proto_key = PropertyKey::String(interp.intern("prototype"));
                    if let Some(JsValue::Object(proto)) =
                        class_obj.borrow().get_property(&proto_key)
                    {
                        proto
                    } else {
                        return Ok(OpResult::Continue);
                    }
                };

                // Get existing accessor property if any
                // Use from_value to handle numeric string keys correctly (e.g., "2" -> Index(2))
                let prop_key = interp.property_key_from_value(&JsValue::String(accessor_name));
                let (existing_getter, existing_setter) = {
                    let target_ref = target.borrow();
                    if let Some(prop) = target_ref.properties.get(&prop_key) {
                        (prop.getter().cloned(), prop.setter().cloned())
                    } else {
                        (None, None)
                    }
                };

                // Merge with existing accessors
                let final_getter = new_getter.cloned().or(existing_getter);
                let final_setter = new_setter.cloned().or(existing_setter);

                // Create accessor property
                let property = Property::accessor(final_getter, final_setter);
                target.borrow_mut().define_property(prop_key, property);

                Ok(OpResult::Continue)
            }

            Op::SuperCall {
                dst,
                args_start,
                argc,
            } => {
                // Get the current function's __super__ property (parent constructor)
                let super_ctor = self.get_super_constructor(interp)?;

                let mut args = self.acquire_arguments_vec(argc as usize);
                for i in 0..argc {
                    args.push(self.get_reg(args_start + i).clone());
                }

                // Create guard and protect all object values
                let guard = interp.heap.create_guard();
                super_ctor.guard_by(&guard);
                self.this_value.guard_by(&guard);
                for arg in &args {
                    arg.guard_by(&guard);
                }

                // Call super constructor with current this - use trampoline
                let this = self.this_value.clone();
                Ok(OpResult::Call {
                    callee: super_ctor,
                    this_value: this,
                    args,
                    return_register: dst,
                    new_target: JsValue::Undefined,
                    is_super_call: true, // This is a super() call
                    guard,
                })
            }

            Op::SuperCallSpread { dst, args_array } => {
                // Get the current function's __super__ property (parent constructor)
                let super_ctor = self.get_super_constructor(interp)?;

                // Extract arguments from the array
                let args_val = self.get_reg(args_array);
                let args: Vec<JsValue> = if let JsValue::Object(arr_ref) = args_val {
                    if let Some(elems) = arr_ref.borrow().array_elements() {
                        elems.to_vec()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                // Create guard and protect all object values
                let guard = interp.heap.create_guard();
                super_ctor.guard_by(&guard);
                self.this_value.guard_by(&guard);
                for arg in &args {
                    arg.guard_by(&guard);
                }

                // Call super constructor with current this - use trampoline
                let this = self.this_value.clone();
                Ok(OpResult::Call {
                    callee: super_ctor,
                    this_value: this,
                    args,
                    return_register: dst,
                    new_target: JsValue::Undefined,
                    is_super_call: true, // This is a super() call
                    guard,
                })
            }

            Op::SuperGet { dst, key } => {
                let key_val = self.get_reg(key);
                let super_target = self.get_super_target(interp)?;
                let Guarded { value, .. } =
                    self.get_property_value(interp, &super_target, key_val)?;
                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            Op::SuperGetConst { dst, key } => {
                let key_str = self
                    .get_string_constant(key)
                    .ok_or_else(|| JsError::internal_error("Invalid super property key"))?;
                let super_target = self.get_super_target(interp)?;
                let Guarded { value, .. } =
                    self.get_property_value(interp, &super_target, &JsValue::String(key_str))?;
                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            Op::SuperSet { key, value } => {
                // In JavaScript, super.x = value sets the property on `this`, not on the super prototype
                // The lookup is done through super (for semantics), but the assignment is to `this`
                let key_val = self.get_reg(key);

                if let JsValue::Object(this_obj) = &self.this_value {
                    let prop_key = interp.property_key_from_value(key_val);
                    let set_value = self.get_reg(value);
                    this_obj
                        .borrow_mut()
                        .set_property(prop_key, set_value.clone());
                }
                Ok(OpResult::Continue)
            }

            Op::SuperSetConst { key, value } => {
                // In JavaScript, super.x = value sets the property on `this`, not on the super prototype
                let key_str = self
                    .get_string_constant(key)
                    .ok_or_else(|| JsError::internal_error("Invalid super property key"))?;

                if let JsValue::Object(this_obj) = &self.this_value {
                    let set_value = self.get_reg(value);
                    this_obj
                        .borrow_mut()
                        .set_property(PropertyKey::String(key_str), set_value.clone());
                }
                Ok(OpResult::Continue)
            }

            Op::ApplyClassDecorator {
                class,
                decorator,
                class_name,
                initializers,
            } => {
                let class_val = self.get_reg(class);
                let decorator_val = self.get_reg(decorator);
                let initializers_arr = self.get_reg(initializers);

                // Get class name for context (None if class_name is MAX)
                let name = if class_name == u16::MAX {
                    None
                } else {
                    self.get_string_constant(class_name)
                };

                // Create decorator context object
                let guard = interp.heap.create_guard();
                let ctx = interp.create_object(&guard);

                // Set context.kind = "class"
                ctx.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("kind")),
                    JsValue::String(interp.intern("class")),
                );

                // Set context.name if we have a class name
                if let Some(n) = name {
                    ctx.borrow_mut().set_property(
                        PropertyKey::String(interp.intern("name")),
                        JsValue::String(n),
                    );
                }

                // Set context.static = false (classes don't have static flag)
                // This is for consistency with method/field decorators
                // Note: TC39 spec doesn't define static for class decorators,
                // but some tests expect it to be undefined

                // Store the initializers array on context so addInitializer can access it
                let init_key = interp.intern("__initializers__");
                ctx.borrow_mut()
                    .set_property(PropertyKey::String(init_key), initializers_arr.clone());

                // Create addInitializer function that pushes to context.__initializers__
                let add_init_fn = interp.create_native_fn(
                    &guard,
                    "addInitializer",
                    |interp, this, args| {
                        // Get the callback from args
                        let callback = args.first().cloned().unwrap_or(JsValue::Undefined);

                        // Get __initializers__ array from this (the context object)
                        if let JsValue::Object(ctx_obj) = this {
                            let init_key = interp.intern("__initializers__");
                            if let Some(JsValue::Object(arr)) = ctx_obj
                                .borrow()
                                .get_property(&PropertyKey::String(init_key))
                            {
                                // Push callback to the array using array_elements_mut
                                let mut arr_ref = arr.borrow_mut();
                                if let Some(elements) = arr_ref.array_elements_mut() {
                                    elements.push(callback.clone());
                                }
                            }
                        }
                        Ok(crate::value::Guarded::unguarded(JsValue::Undefined))
                    },
                    1,
                );
                ctx.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("addInitializer")),
                    JsValue::Object(add_init_fn),
                );

                // Call decorator(class, context)
                let Guarded {
                    value,
                    guard: _guard,
                } = interp.call_function(
                    decorator_val.clone(),
                    JsValue::Undefined,
                    &[class_val.clone(), JsValue::Object(ctx)],
                )?;

                // If decorator returns undefined, keep original class; otherwise use return value
                if matches!(value, JsValue::Undefined) {
                    // Keep original class value in register
                } else {
                    self.set_reg(class, value);
                }

                Ok(OpResult::Continue)
            }

            Op::RunClassInitializers {
                class,
                initializers,
            } => {
                let class_val = self.get_reg(class);
                let initializers_val = self.get_reg(initializers);

                // Get the initializers array and call each function with class as `this`
                if let JsValue::Object(arr) = initializers_val {
                    // Clone elements to avoid borrow issues during iteration
                    let callbacks: Vec<JsValue> = {
                        let arr_ref = arr.borrow();
                        if let crate::value::ExoticObject::Array { ref elements } = arr_ref.exotic {
                            elements.clone()
                        } else {
                            Vec::new()
                        }
                    };

                    for callback in callbacks {
                        // Call the initializer with class as `this`
                        interp.call_function(callback, class_val.clone(), &[])?;
                    }
                }

                Ok(OpResult::Continue)
            }

            Op::ApplyMethodDecorator {
                method,
                decorator,
                name,
                kind,
                is_static,
                is_private,
            } => {
                let method_val = self.get_reg(method);
                let decorator_val = self.get_reg(decorator);
                let method_name = self.get_string_constant(name);

                // Create decorator context object
                let guard = interp.heap.create_guard();
                let ctx = interp.create_object(&guard);

                // Set context.kind based on kind byte (0 = method, 1 = getter, 2 = setter)
                let kind_str = match kind {
                    0 => "method",
                    1 => "getter",
                    2 => "setter",
                    _ => "method",
                };
                ctx.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("kind")),
                    JsValue::String(interp.intern(kind_str)),
                );

                // Set context.name
                if let Some(n) = method_name {
                    ctx.borrow_mut().set_property(
                        PropertyKey::String(interp.intern("name")),
                        JsValue::String(n),
                    );
                }

                // Set context.static
                ctx.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("static")),
                    JsValue::Boolean(is_static),
                );

                // Set context.private
                ctx.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("private")),
                    JsValue::Boolean(is_private),
                );

                // Call decorator(method, context)
                let Guarded {
                    value,
                    guard: _guard,
                } = interp.call_function(
                    decorator_val.clone(),
                    JsValue::Undefined,
                    &[method_val.clone(), JsValue::Object(ctx)],
                )?;

                // If decorator returns undefined, keep original method; otherwise use return value
                if matches!(value, JsValue::Undefined) {
                    // Keep original method value in register
                } else {
                    self.set_reg(method, value);
                }

                Ok(OpResult::Continue)
            }

            Op::ApplyParameterDecorator {
                target,
                decorator,
                method_name,
                param_name,
                param_index,
                is_static,
            } => {
                let target_val = self.get_reg(target);
                let decorator_val = self.get_reg(decorator);
                let method_name_str = self.get_string_constant(method_name);
                let param_name_str = self.get_string_constant(param_name);

                // Create decorator context object
                let guard = interp.heap.create_guard();
                let ctx = interp.create_object(&guard);

                // Set context.kind = "parameter"
                ctx.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("kind")),
                    JsValue::String(interp.intern("parameter")),
                );

                // Set context.name (parameter name)
                if let Some(n) = param_name_str {
                    if !n.is_empty() {
                        ctx.borrow_mut().set_property(
                            PropertyKey::String(interp.intern("name")),
                            JsValue::String(n),
                        );
                    }
                }

                // Set context.function (method name)
                if let Some(n) = method_name_str {
                    ctx.borrow_mut().set_property(
                        PropertyKey::String(interp.intern("function")),
                        JsValue::String(n),
                    );
                }

                // Set context.index (parameter index)
                ctx.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("index")),
                    JsValue::Number(f64::from(param_index)),
                );

                // Set context.static
                ctx.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("static")),
                    JsValue::Boolean(is_static),
                );

                // Call decorator(target, context)
                // Parameter decorators are called for side effects only (like metadata registration)
                let _result = interp.call_function(
                    decorator_val.clone(),
                    JsValue::Undefined,
                    &[target_val.clone(), JsValue::Object(ctx)],
                )?;

                Ok(OpResult::Continue)
            }

            Op::ApplyFieldDecorator {
                dst,
                decorator,
                name,
                is_static,
                is_private,
                is_accessor,
            } => {
                let decorator_val = self.get_reg(decorator);
                let field_name = self.get_string_constant(name);

                // Create decorator context object
                let guard = interp.heap.create_guard();
                let ctx = interp.create_object(&guard);

                // Set context.kind = "field" or "accessor" for auto-accessors
                let kind_str = if is_accessor { "accessor" } else { "field" };
                ctx.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("kind")),
                    JsValue::String(interp.intern(kind_str)),
                );

                // Set context.name
                if let Some(n) = field_name {
                    ctx.borrow_mut().set_property(
                        PropertyKey::String(interp.intern("name")),
                        JsValue::String(n),
                    );
                }

                // Set context.static
                ctx.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("static")),
                    JsValue::Boolean(is_static),
                );

                // Set context.private
                ctx.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("private")),
                    JsValue::Boolean(is_private),
                );

                // Call decorator(undefined, context)
                // Field decorators receive undefined as first arg and return an initializer transformer
                let Guarded {
                    value,
                    guard: _guard,
                } = interp.call_function(
                    decorator_val.clone(),
                    JsValue::Undefined,
                    &[JsValue::Undefined, JsValue::Object(ctx)],
                )?;

                // Store the result (initializer transformer or undefined)
                self.set_reg(dst, value);

                Ok(OpResult::Continue)
            }

            Op::StoreFieldInitializer {
                class,
                name,
                initializer,
            } => {
                let class_val = self.get_reg(class);
                let initializer_val = self.get_reg(initializer);
                let field_name = self
                    .get_string_constant(name)
                    .unwrap_or_else(|| interp.intern(""));

                if let JsValue::Object(class_obj) = class_val {
                    // Get or create __field_initializers__ object
                    let init_key = interp.intern("__field_initializers__");
                    #[allow(clippy::map_clone)]
                    let inits_obj = {
                        let borrowed = class_obj.borrow();
                        borrowed
                            .get_property(&PropertyKey::String(init_key.cheap_clone()))
                            .map(|v| v.clone())
                    };

                    let guard = interp.heap.create_guard();
                    let inits = match inits_obj {
                        Some(JsValue::Object(obj)) => obj,
                        _ => {
                            // Create new __field_initializers__ object
                            let new_obj = interp.create_object_raw(&guard);
                            class_obj.borrow_mut().set_property(
                                PropertyKey::String(init_key),
                                JsValue::Object(new_obj.cheap_clone()),
                            );
                            new_obj
                        }
                    };

                    // Store the initializer for this field
                    inits
                        .borrow_mut()
                        .set_property(PropertyKey::String(field_name), initializer_val.clone());
                }

                Ok(OpResult::Continue)
            }

            Op::GetFieldInitializer { dst, class, name } => {
                let class_val = self.get_reg(class);
                let field_name = self
                    .get_string_constant(name)
                    .unwrap_or_else(|| interp.intern(""));

                let mut initializer = JsValue::Undefined;

                if let JsValue::Object(class_obj) = class_val {
                    let init_key = interp.intern("__field_initializers__");
                    let borrowed = class_obj.borrow();
                    if let Some(JsValue::Object(inits)) =
                        borrowed.get_property(&PropertyKey::String(init_key))
                    {
                        if let Some(init) = inits
                            .borrow()
                            .get_property(&PropertyKey::String(field_name))
                        {
                            initializer = init.clone();
                        }
                    }
                }

                self.set_reg(dst, initializer);
                Ok(OpResult::Continue)
            }

            Op::ApplyFieldInitializer { value, initializer } => {
                let init_val = self.get_reg(initializer);

                // If initializer is a function, call it with the value
                if matches!(&init_val, JsValue::Object(_)) {
                    let value_val = self.get_reg(value);
                    let result = interp.call_function(
                        init_val.clone(),
                        JsValue::Undefined,
                        std::slice::from_ref(value_val),
                    )?;
                    self.set_reg(value, result.value);
                }
                // If initializer is undefined, keep original value

                Ok(OpResult::Continue)
            }

            Op::DefineAutoAccessor {
                class,
                name,
                init_value,
                target_dst,
                is_static,
            } => {
                let class_val = self.get_reg(class);
                let JsValue::Object(class_obj) = class_val else {
                    return Err(JsError::type_error("Class is not an object"));
                };

                let accessor_name = self
                    .get_string_constant(name)
                    .unwrap_or_else(|| interp.intern(""));

                let init_val = self.get_reg(init_value).clone();

                // Create a unique storage key for this accessor
                let storage_key =
                    interp.intern(&format!("__accessor_{}__", accessor_name.as_str()));

                let guard = interp.heap.create_guard();

                // Create getter function (AccessorGetter)
                let getter = interp.create_object(&guard);
                getter.borrow_mut().prototype = Some(interp.function_prototype.cheap_clone());
                getter.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("__accessor_storage_key__")),
                    JsValue::String(storage_key.cheap_clone()),
                );
                getter.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("__accessor_init_value__")),
                    init_val,
                );
                getter.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("__accessor_kind__")),
                    JsValue::String(interp.intern("getter")),
                );
                getter.borrow_mut().exotic = ExoticObject::Function(JsFunction::AccessorGetter);

                // Create setter function (AccessorSetter)
                let setter = interp.create_object(&guard);
                setter.borrow_mut().prototype = Some(interp.function_prototype.cheap_clone());
                setter.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("__accessor_storage_key__")),
                    JsValue::String(storage_key),
                );
                setter.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("__accessor_kind__")),
                    JsValue::String(interp.intern("setter")),
                );
                setter.borrow_mut().exotic = ExoticObject::Function(JsFunction::AccessorSetter);

                // Get target object (class for static, prototype for instance)
                let target = if is_static {
                    class_obj.cheap_clone()
                } else {
                    let proto_key = PropertyKey::String(interp.intern("prototype"));
                    if let Some(JsValue::Object(proto)) =
                        class_obj.borrow().get_property(&proto_key)
                    {
                        proto
                    } else {
                        return Err(JsError::type_error("Class has no prototype"));
                    }
                };

                // Define the accessor property on target
                let prop_key = PropertyKey::String(accessor_name.cheap_clone());
                let property =
                    Property::accessor(Some(getter.cheap_clone()), Some(setter.cheap_clone()));
                target.borrow_mut().define_property(prop_key, property);

                // Create target object { get, set } for decorators
                let target_obj = interp.create_object(&guard);
                target_obj.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("get")),
                    JsValue::Object(getter),
                );
                target_obj.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("set")),
                    JsValue::Object(setter),
                );

                self.set_reg(target_dst, JsValue::Object(target_obj));
                Ok(OpResult::Continue)
            }

            Op::StoreAutoAccessor {
                class,
                name,
                accessor_obj,
                is_static,
            } => {
                let class_val = self.get_reg(class);
                let JsValue::Object(class_obj) = class_val else {
                    return Err(JsError::type_error("Class is not an object"));
                };

                let accessor_name = self
                    .get_string_constant(name)
                    .unwrap_or_else(|| interp.intern(""));

                let accessor_val = self.get_reg(accessor_obj);

                // Get target object (class for static, prototype for instance)
                let target = if is_static {
                    class_obj.cheap_clone()
                } else {
                    let proto_key = PropertyKey::String(interp.intern("prototype"));
                    if let Some(JsValue::Object(proto)) =
                        class_obj.borrow().get_property(&proto_key)
                    {
                        proto
                    } else {
                        return Err(JsError::type_error("Class has no prototype"));
                    }
                };

                // Extract getter and setter from the accessor object
                let (final_getter, final_setter) = if let JsValue::Object(obj) = accessor_val {
                    let obj_ref = obj.borrow();
                    let get_key = interp.intern("get");
                    let set_key = interp.intern("set");

                    let getter = if let Some(JsValue::Object(g)) =
                        obj_ref.get_property(&PropertyKey::String(get_key))
                    {
                        Some(g.cheap_clone())
                    } else {
                        None
                    };

                    let setter = if let Some(JsValue::Object(s)) =
                        obj_ref.get_property(&PropertyKey::String(set_key))
                    {
                        Some(s.cheap_clone())
                    } else {
                        None
                    };

                    (getter, setter)
                } else {
                    (None, None)
                };

                // Define the accessor property on target
                let prop_key = PropertyKey::String(accessor_name);
                let property = Property::accessor(final_getter, final_setter);
                target.borrow_mut().define_property(prop_key, property);

                Ok(OpResult::Continue)
            }

            Op::ApplyAutoAccessorDecorator {
                target,
                decorator,
                name,
                is_static,
            } => {
                let decorator_val = self.get_reg(decorator);
                let target_val = self.get_reg(target);
                let accessor_name = self.get_string_constant(name);

                // Create decorator context object
                let guard = interp.heap.create_guard();
                let ctx = interp.create_object(&guard);

                // Set context.kind = "accessor"
                ctx.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("kind")),
                    JsValue::String(interp.intern("accessor")),
                );

                // Set context.name
                if let Some(n) = accessor_name {
                    ctx.borrow_mut().set_property(
                        PropertyKey::String(interp.intern("name")),
                        JsValue::String(n),
                    );
                }

                // Set context.static
                ctx.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("static")),
                    JsValue::Boolean(is_static),
                );

                // Set context.private = false (public auto-accessors)
                ctx.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("private")),
                    JsValue::Boolean(false),
                );

                // Call decorator(target, context)
                let Guarded {
                    value,
                    guard: _guard,
                } = interp.call_function(
                    decorator_val.clone(),
                    JsValue::Undefined,
                    &[target_val.clone(), JsValue::Object(ctx)],
                )?;

                // If decorator returns an object, use it as new target
                // Otherwise keep the original target
                if matches!(&value, JsValue::Object(_)) {
                    self.set_reg(target, value);
                }

                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Spread/Rest
            // ═══════════════════════════════════════════════════════════════════════════
            Op::SpreadArray { dst, src } => {
                // Spread elements from src iterable onto the dst array
                // dst should already be an array - we append elements to it
                let src_val = self.get_reg(src);
                let dst_val = self.get_reg(dst);

                let elements_to_add: Vec<JsValue> = match &src_val {
                    JsValue::Object(obj_ref) => {
                        if let Some(elems) = obj_ref.borrow().array_elements() {
                            elems.to_vec()
                        } else {
                            // Try iterator protocol
                            match interp.collect_iterator_values(src_val) {
                                Ok(Some(values)) => values,
                                Ok(None) => Vec::new(),
                                Err(e) => return Err(e),
                            }
                        }
                    }
                    JsValue::String(s) => s
                        .as_str()
                        .chars()
                        .map(|c| JsValue::String(JsString::from(c.to_string())))
                        .collect(),
                    _ => Vec::new(),
                };

                // Append elements to the destination array
                if let JsValue::Object(dst_arr) = dst_val {
                    if let Some(existing) = dst_arr.borrow_mut().array_elements_mut() {
                        existing.extend(elements_to_add);
                    }
                }
                Ok(OpResult::Continue)
            }

            // NOTE: review
            Op::CreateRestArray { dst, start_index } => {
                // Create an array from remaining iterator elements
                // This is used for rest patterns like [...rest] = arr
                // The iterator state is assumed to be in the register before this one
                // We need to collect all remaining elements from the current iterator

                // For now, this opcode is context-dependent - it needs the iterator
                // that was being used. We'll check if there's an internal iterator in scope.
                // This is a simplified implementation that works with the pattern compiler.

                // Look for the iterator in a previous register (typically dst - 3 based on pattern)
                // This is a heuristic - the pattern compiler allocates registers in a specific order
                let iter_reg = dst.saturating_sub(3);
                let iter_val = self.get_reg(iter_reg);

                let mut elements = Vec::new();

                if let JsValue::Object(iter_obj) = iter_val {
                    // Check for internal array iterator
                    let array_prop = iter_obj
                        .borrow()
                        .get_property(&PropertyKey::String(interp.intern("__array__")));
                    if let Some(JsValue::Object(arr_ref)) = array_prop {
                        let index = match iter_obj
                            .borrow()
                            .get_property(&PropertyKey::String(interp.intern("__index__")))
                        {
                            Some(JsValue::Number(n)) => n as usize,
                            _ => start_index as usize,
                        };

                        if let Some(elems) = arr_ref.borrow().array_elements() {
                            for i in index..elems.len() {
                                if let Some(val) = elems.get(i) {
                                    elements.push(val.clone());
                                }
                            }
                        }
                    }
                }

                let guard = interp.heap.create_guard();
                let arr = interp.create_array_from(&guard, elements);
                self.set_reg(dst, JsValue::Object(arr));
                Ok(OpResult::Continue)
            }

            Op::CreateObjectRest {
                dst,
                src,
                excluded_keys,
            } => {
                // Create an object with all properties from src except excluded_keys
                let src_val = self.get_reg(src);

                // Get excluded keys from constant pool
                let excluded = match self.chunk.constants.get(excluded_keys as usize) {
                    Some(Constant::ExcludedKeys(keys)) => keys,
                    _ => &vec![],
                };

                let guard = interp.heap.create_guard();
                let result = interp.create_object(&guard);

                if let JsValue::Object(src_obj) = src_val {
                    // Copy all enumerable own properties except excluded ones
                    let src_borrowed = src_obj.borrow();
                    for (key, prop) in src_borrowed.properties.iter() {
                        // Skip non-enumerable properties
                        if !prop.enumerable() {
                            continue;
                        }

                        // Check if this key should be excluded
                        let should_exclude = match key {
                            PropertyKey::String(s) => {
                                excluded.iter().any(|k| k.as_str() == s.as_str())
                            }
                            PropertyKey::Symbol(_) | PropertyKey::Index(_) => false,
                        };

                        if !should_exclude {
                            result
                                .borrow_mut()
                                .set_property(key.clone(), prop.value.clone());
                        }
                    }
                }

                self.set_reg(dst, JsValue::Object(result));
                Ok(OpResult::Continue)
            }

            Op::SpreadObject { dst, src } => {
                // Copy all enumerable own properties from src to dst
                let dst_val = self.get_reg(dst);
                let src_val = self.get_reg(src);

                if let (JsValue::Object(dst_obj), JsValue::Object(src_obj)) = (&dst_val, &src_val) {
                    // Collect properties first to avoid borrow issues
                    let props_to_copy: Vec<_> = {
                        let src_borrowed = src_obj.borrow();
                        src_borrowed
                            .properties
                            .iter()
                            .filter(|(_, prop)| prop.enumerable())
                            .map(|(key, prop)| (key.clone(), prop.value.clone()))
                            .collect()
                    };

                    // Copy properties to destination
                    let mut dst_borrowed = dst_obj.borrow_mut();
                    for (key, value) in props_to_copy {
                        dst_borrowed.set_property(key, value);
                    }
                }
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Template Literals
            // ═══════════════════════════════════════════════════════════════════════════
            Op::TemplateConcat { dst, start, count } => {
                let mut result = String::new();
                let to_string_key = PropertyKey::String(interp.intern("toString"));
                for i in 0..count {
                    let val = self.get_reg(start + i);
                    // For objects, call toString method; for primitives, use to_js_string
                    let str_val = if let JsValue::Object(obj) = &val {
                        // Check if object has a custom toString method
                        if let Some(JsValue::Object(func_obj)) =
                            obj.borrow().get_property(&to_string_key)
                        {
                            if func_obj.borrow().is_callable() {
                                // Call toString()
                                match interp.call_function(
                                    JsValue::Object(func_obj.clone()),
                                    val.clone(),
                                    &[],
                                ) {
                                    Ok(Guarded { value, guard: _ }) => interp.to_js_string(&value),
                                    Err(_) => interp.to_js_string(val),
                                }
                            } else {
                                interp.to_js_string(val)
                            }
                        } else {
                            interp.to_js_string(val)
                        }
                    } else {
                        interp.to_js_string(val)
                    };
                    result.push_str(str_val.as_str());
                }
                self.set_reg(dst, JsValue::String(JsString::from(result)));
                Ok(OpResult::Continue)
            }

            Op::TaggedTemplate {
                dst,
                tag,
                this,
                template,
                exprs_start,
                exprs_count,
            } => {
                // Get the template strings constant
                let template_const =
                    self.chunk.constants.get(template as usize).ok_or_else(|| {
                        JsError::internal_error("Invalid template constant index")
                    })?;

                let (cooked, raw) = match template_const {
                    Constant::TemplateStrings { cooked, raw } => (cooked.clone(), raw.clone()),
                    _ => return Err(JsError::internal_error("Expected TemplateStrings constant")),
                };

                // Create the strings array (cooked strings)
                let guard = interp.heap.create_guard();
                let strings: Vec<JsValue> = cooked
                    .iter()
                    .map(|s| JsValue::String(s.cheap_clone()))
                    .collect();
                let strings_arr = interp.create_array_from(&guard, strings);

                // Create the raw strings array
                let raw_strings: Vec<JsValue> = raw
                    .iter()
                    .map(|s| JsValue::String(s.cheap_clone()))
                    .collect();
                let raw_arr = interp.create_array_from(&guard, raw_strings);

                // Add 'raw' property to strings array
                let raw_key = PropertyKey::String(interp.intern("raw"));
                strings_arr
                    .borrow_mut()
                    .set_property(raw_key, JsValue::Object(raw_arr));

                // Build args: [strings_array, ...expressions]
                let mut args = vec![JsValue::Object(strings_arr)];
                for i in 0..exprs_count {
                    args.push(self.get_reg(exprs_start + i).clone());
                }

                // Get the tag function and this value
                let tag_fn = self.get_reg(tag);
                let this_val = self.get_reg(this);

                // Call the tag function
                let Guarded {
                    value,
                    guard: _guard,
                } = interp.call_function(tag_fn.clone(), this_val.clone(), &args)?;

                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Function Name Inference
            // ═══════════════════════════════════════════════════════════════════════════
            Op::SetFunctionName { func, name } => {
                let func_val = self.get_reg(func);
                let name_str = self
                    .get_string_constant(name)
                    .ok_or_else(|| JsError::internal_error("Invalid function name constant"))?;

                // Only set name if value is a function object without a name
                if let JsValue::Object(obj) = func_val {
                    let mut obj_ref = obj.borrow_mut();

                    // Check if this is a function and if it doesn't have a name already
                    let should_set_name =
                        if let crate::value::ExoticObject::Function(func) = &obj_ref.exotic {
                            // Check if function already has a non-empty name
                            let has_name = match func {
                                JsFunction::Native(f) => !f.name.as_str().is_empty(),
                                JsFunction::Bytecode(bc)
                                | JsFunction::BytecodeGenerator(bc)
                                | JsFunction::BytecodeAsync(bc)
                                | JsFunction::BytecodeAsyncGenerator(bc) => bc
                                    .chunk
                                    .function_info
                                    .as_ref()
                                    .and_then(|info| info.name.as_ref())
                                    .is_some_and(|n| !n.as_str().is_empty()),
                                JsFunction::Bound(_) => true, // Bound functions already have names
                                // Internal functions don't need names set
                                JsFunction::PromiseResolve(_)
                                | JsFunction::PromiseReject(_)
                                | JsFunction::PromiseAllFulfill { .. }
                                | JsFunction::PromiseAllReject(_)
                                | JsFunction::AccessorGetter
                                | JsFunction::AccessorSetter
                                | JsFunction::ModuleExportGetter { .. }
                                | JsFunction::ModuleReExportGetter { .. }
                                | JsFunction::ProxyRevoke(_) => true,
                            };
                            // Also check if there's already an own name property set
                            let name_key = PropertyKey::String(interp.intern("name"));
                            let has_own_name = obj_ref.get_own_property(&name_key).is_some();
                            !has_name && !has_own_name
                        } else {
                            false // Not a function
                        };

                    if should_set_name {
                        let name_key = PropertyKey::String(interp.intern("name"));
                        obj_ref.define_property(
                            name_key,
                            Property::with_attributes(
                                JsValue::String(name_str),
                                false, // not writable
                                false, // not enumerable
                                true,  // configurable
                            ),
                        );
                    }
                }

                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Miscellaneous
            // ═══════════════════════════════════════════════════════════════════════════
            Op::Nop => Ok(OpResult::Continue),

            Op::Halt => {
                let result = self
                    .registers
                    .first()
                    .cloned()
                    .unwrap_or(JsValue::Undefined);
                Ok(OpResult::Halt(Guarded::from_value(result, &interp.heap)))
            }

            Op::Debugger => Ok(OpResult::Continue),

            Op::Pop => Ok(OpResult::Continue),

            Op::Dup { dst, src } => {
                let value = self.get_reg(src).clone();
                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            Op::LoadThis { dst } => {
                self.set_reg(dst, self.this_value.clone());
                Ok(OpResult::Continue)
            }

            Op::LoadArguments { dst } => {
                // Create an arguments object (array-like) from the stored arguments
                let guard = interp.heap.create_guard();
                let args_array = interp.create_array_from(&guard, self.arguments.clone());
                self.set_reg(dst, JsValue::Object(args_array));
                Ok(OpResult::Continue)
            }

            Op::LoadNewTarget { dst } => {
                // Return the new.target value
                self.set_reg(dst, self.new_target.clone());
                Ok(OpResult::Continue)
            }

            Op::GetPrivateField {
                dst,
                obj,
                class_brand,
                field_name,
            } => {
                let obj_val = self.get_reg(obj);
                let field_name_str = self.get_string_constant(field_name).ok_or_else(|| {
                    JsError::internal_error("Invalid private field name constant")
                })?;

                let JsValue::Object(obj_ref) = obj_val else {
                    return Err(JsError::type_error(format!(
                        "Cannot read private member {} from non-object",
                        field_name_str
                    )));
                };

                let key = crate::value::PrivateFieldKey::new(class_brand, field_name_str);
                let value = obj_ref
                    .borrow()
                    .get_private_field(&key)
                    .cloned()
                    .ok_or_else(|| {
                        JsError::type_error(format!(
                            "Cannot read private member {} from an object whose class did not declare it",
                            key.field_name
                        ))
                    })?;

                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            Op::SetPrivateField {
                obj,
                class_brand,
                field_name,
                value,
            } => {
                let obj_val = self.get_reg(obj);
                let field_name_str = self.get_string_constant(field_name).ok_or_else(|| {
                    JsError::internal_error("Invalid private field name constant")
                })?;
                let val = self.get_reg(value).clone();

                let JsValue::Object(obj_ref) = obj_val else {
                    return Err(JsError::type_error(format!(
                        "Cannot write private member {} to non-object",
                        field_name_str
                    )));
                };

                let key =
                    crate::value::PrivateFieldKey::new(class_brand, field_name_str.cheap_clone());

                // Check that this object has this private field (brand check)
                if !obj_ref.borrow().has_private_field(&key) {
                    return Err(JsError::type_error(format!(
                        "Cannot write private member {} to an object whose class did not declare it",
                        field_name_str
                    )));
                }

                obj_ref.borrow_mut().set_private_field(key, val);
                Ok(OpResult::Continue)
            }

            Op::DefinePrivateField {
                obj,
                class_brand,
                field_name,
                value,
            } => {
                let obj_val = self.get_reg(obj);
                let field_name_str = self.get_string_constant(field_name).ok_or_else(|| {
                    JsError::internal_error("Invalid private field name constant")
                })?;
                let val = self.get_reg(value).clone();

                let JsValue::Object(obj_ref) = obj_val else {
                    return Err(JsError::type_error(
                        "Cannot define private field on non-object",
                    ));
                };

                let key = crate::value::PrivateFieldKey::new(class_brand, field_name_str);
                obj_ref.borrow_mut().set_private_field(key, val);
                Ok(OpResult::Continue)
            }

            Op::DefinePrivateMethod {
                class,
                class_brand,
                method_name,
                method,
                is_static,
            } => {
                let class_val = self.get_reg(class);
                let method_name_str = self.get_string_constant(method_name).ok_or_else(|| {
                    JsError::internal_error("Invalid private method name constant")
                })?;
                let method_val = self.get_reg(method).clone();

                let JsValue::Object(class_obj) = class_val else {
                    return Err(JsError::type_error("Class is not an object"));
                };

                // For static private methods, install directly on the class constructor
                if is_static {
                    let key = crate::value::PrivateFieldKey::new(class_brand, method_name_str);
                    class_obj.borrow_mut().set_private_field(key, method_val);
                } else {
                    // For instance private methods, we store them on the constructor
                    // under a special key (__private_methods__) so that the constructor
                    // can install them on new instances.
                    // Each class stores its private methods in a map keyed by field name.
                    let private_methods_key =
                        PropertyKey::String(interp.intern("__private_methods__"));

                    let methods_map = {
                        let class_borrowed = class_obj.borrow();
                        class_borrowed
                            .get_own_property(&private_methods_key)
                            .and_then(|p| {
                                if let JsValue::Object(obj) = &p.value {
                                    Some(obj.cheap_clone())
                                } else {
                                    None
                                }
                            })
                    };

                    let methods_obj = if let Some(existing) = methods_map {
                        existing
                    } else {
                        // Create a new object to store private methods
                        let guard = interp.heap.create_guard();
                        let new_obj = interp.create_object_raw(&guard);
                        class_obj.borrow_mut().set_property(
                            private_methods_key.clone(),
                            JsValue::Object(new_obj.cheap_clone()),
                        );
                        new_obj
                    };

                    // Store method with a key that includes class_brand for brand checking
                    // Key format: "brand:field_name"
                    let storage_key = PropertyKey::String(JsString::from(format!(
                        "{}:{}",
                        class_brand,
                        method_name_str.as_str()
                    )));
                    methods_obj
                        .borrow_mut()
                        .set_property(storage_key, method_val);
                }

                Ok(OpResult::Continue)
            }

            Op::InstallPrivateMethod {
                class_brand,
                method_name,
            } => {
                let method_name_str = self.get_string_constant(method_name).ok_or_else(|| {
                    JsError::internal_error("Invalid private method name constant")
                })?;

                // Get new.target (the class constructor)
                let JsValue::Object(new_target) = &self.new_target else {
                    return Err(JsError::internal_error(
                        "InstallPrivateMethod requires new.target to be an object",
                    ));
                };

                // Get __private_methods__ from new.target
                let private_methods_key = PropertyKey::String(interp.intern("__private_methods__"));
                let methods_obj = {
                    let new_target_borrowed = new_target.borrow();
                    new_target_borrowed
                        .get_own_property(&private_methods_key)
                        .and_then(|p| {
                            if let JsValue::Object(obj) = &p.value {
                                Some(obj.cheap_clone())
                            } else {
                                None
                            }
                        })
                };

                if let Some(methods) = methods_obj {
                    // Look up method by brand:name key
                    let storage_key = PropertyKey::String(interp.intern(&format!(
                        "{}:{}",
                        class_brand,
                        method_name_str.as_str()
                    )));
                    let method_val = methods
                        .borrow()
                        .get_own_property(&storage_key)
                        .map(|p| p.value.clone())
                        .unwrap_or(JsValue::Undefined);

                    if !matches!(method_val, JsValue::Undefined) {
                        // Install on this
                        let this_val = self.this_value.clone();
                        let JsValue::Object(this_obj) = this_val else {
                            return Err(JsError::type_error("this is not an object"));
                        };

                        let key = crate::value::PrivateFieldKey::new(class_brand, method_name_str);
                        this_obj.borrow_mut().set_private_field(key, method_val);
                    }
                }

                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Module Operations
            // ═══════════════════════════════════════════════════════════════════════════
            Op::ExportBinding {
                export_name,
                binding_name,
                value,
            } => {
                let export_name_str = self
                    .get_string_constant(export_name)
                    .ok_or_else(|| JsError::internal_error("Invalid export name constant"))?;
                let binding_name_str = self
                    .get_string_constant(binding_name)
                    .ok_or_else(|| JsError::internal_error("Invalid binding name constant"))?;
                let val = self.get_reg(value).clone();

                // Store in interpreter's exports map
                interp.exports.insert(
                    export_name_str,
                    crate::value::ModuleExport::Direct {
                        name: binding_name_str,
                        value: val,
                    },
                );

                Ok(OpResult::Continue)
            }

            Op::ExportNamespace {
                export_name,
                module_specifier,
            } => {
                let export_name_str = self
                    .get_string_constant(export_name)
                    .ok_or_else(|| JsError::internal_error("Invalid export name constant"))?;
                let specifier_str = self
                    .get_string_constant(module_specifier)
                    .ok_or_else(|| JsError::internal_error("Invalid module specifier constant"))?;

                // Resolve the module and get its namespace object
                let module_obj = interp.resolve_module(specifier_str.as_ref())?;

                // Store the module namespace as a direct export
                // (not a live binding - namespace objects are already live)
                interp.exports.insert(
                    export_name_str.cheap_clone(),
                    crate::value::ModuleExport::Direct {
                        name: export_name_str,
                        value: JsValue::Object(module_obj),
                    },
                );

                Ok(OpResult::Continue)
            }

            Op::ReExport {
                export_name,
                source_module,
                source_key,
            } => {
                let export_name_str = self
                    .get_string_constant(export_name)
                    .ok_or_else(|| JsError::internal_error("Invalid export name constant"))?;
                let source_specifier = self
                    .get_string_constant(source_module)
                    .ok_or_else(|| JsError::internal_error("Invalid source module constant"))?;
                let source_key_str = self
                    .get_string_constant(source_key)
                    .ok_or_else(|| JsError::internal_error("Invalid source key constant"))?;

                // Resolve the source module
                let source_module_obj = interp.resolve_module(source_specifier.as_ref())?;

                // Store as a re-export with delegation to the source module
                interp.exports.insert(
                    export_name_str,
                    crate::value::ModuleExport::ReExport {
                        source_module: source_module_obj,
                        source_key: PropertyKey::String(source_key_str),
                    },
                );

                Ok(OpResult::Continue)
            }
        }
    }

    /// Get a property value from an object, invoking getters if present.
    /// Returns a Guarded to keep newly allocated objects alive (e.g., from getters or proxies).
    fn get_property_value(
        &self,
        interp: &mut Interpreter,
        obj: &JsValue,
        key: &JsValue,
    ) -> Result<Guarded, JsError> {
        match obj {
            JsValue::Object(obj_ref) => {
                // Check if this is a proxy - delegate to proxy_get if so
                if matches!(obj_ref.borrow().exotic, ExoticObject::Proxy(_)) {
                    let prop_key = interp.property_key_from_value(key);
                    // proxy_get already returns Guarded
                    return crate::interpreter::builtins::proxy::proxy_get(
                        interp,
                        obj_ref.cheap_clone(),
                        prop_key,
                        obj.clone(),
                    );
                }

                // Handle __proto__ special property - return prototype
                if let JsValue::String(k) = key {
                    if k.as_str() == "__proto__" {
                        return Ok(Guarded::unguarded(
                            obj_ref
                                .borrow()
                                .prototype
                                .as_ref()
                                .map(|p| JsValue::Object(p.clone()))
                                .unwrap_or(JsValue::Null),
                        ));
                    }
                }

                let prop_key = interp.property_key_from_value(key);
                // Get property descriptor to check for accessor properties
                let prop_desc = obj_ref.borrow().get_property_descriptor(&prop_key);
                match prop_desc {
                    Some((prop, _)) if prop.is_accessor() => {
                        // Property has a getter - invoke it
                        if let Some(getter) = prop.getter() {
                            // call_function already returns Guarded
                            interp.call_function(JsValue::Object(getter.clone()), obj.clone(), &[])
                        } else {
                            Ok(Guarded::unguarded(JsValue::Undefined))
                        }
                    }
                    Some((prop, _)) => Ok(Guarded::unguarded(prop.value.clone())),
                    None => Ok(Guarded::unguarded(JsValue::Undefined)),
                }
            }
            JsValue::String(s) => match key {
                JsValue::String(k) if k.as_str() == "length" => Ok(Guarded::unguarded(
                    JsValue::Number(s.as_str().chars().count() as f64),
                )),
                JsValue::Number(n) => {
                    let idx = *n as usize;
                    if let Some(c) = s.as_str().chars().nth(idx) {
                        return Ok(Guarded::unguarded(JsValue::String(JsString::from(
                            c.to_string(),
                        ))));
                    }
                    Ok(Guarded::unguarded(JsValue::Undefined))
                }
                _ => {
                    let prop_key = interp.property_key_from_value(key);
                    if let Some(val) = interp.string_prototype.borrow().get_property(&prop_key) {
                        Ok(Guarded::unguarded(val.clone()))
                    } else {
                        Ok(Guarded::unguarded(JsValue::Undefined))
                    }
                }
            },
            JsValue::Number(_) => {
                let prop_key = interp.property_key_from_value(key);
                if let Some(val) = interp.number_prototype.borrow().get_property(&prop_key) {
                    Ok(Guarded::unguarded(val.clone()))
                } else {
                    Ok(Guarded::unguarded(JsValue::Undefined))
                }
            }
            JsValue::Boolean(_) => {
                let prop_key = interp.property_key_from_value(key);
                if let Some(val) = interp.boolean_prototype.borrow().get_property(&prop_key) {
                    Ok(Guarded::unguarded(val.clone()))
                } else {
                    Ok(Guarded::unguarded(JsValue::Undefined))
                }
            }
            JsValue::Null => Err(JsError::type_error("Cannot read properties of null")),
            JsValue::Undefined => Err(JsError::type_error("Cannot read properties of undefined")),
            JsValue::Symbol(sym) => {
                // Symbols have a description property
                if let JsValue::String(k) = key {
                    if k.as_str() == "description" {
                        return Ok(Guarded::unguarded(
                            sym.description
                                .as_ref()
                                .map(|d| JsValue::String(d.cheap_clone()))
                                .unwrap_or(JsValue::Undefined),
                        ));
                    }
                }
                // Other symbol prototype methods
                let prop_key = interp.property_key_from_value(key);
                if let Some(val) = interp.symbol_prototype.borrow().get_property(&prop_key) {
                    Ok(Guarded::unguarded(val.clone()))
                } else {
                    Ok(Guarded::unguarded(JsValue::Undefined))
                }
            }
        }
    }

    /// Set a property value on an object, invoking setters if present
    fn set_property_value(
        &self,
        interp: &mut Interpreter,
        obj: &JsValue,
        key: &JsValue,
        value: JsValue,
    ) -> Result<(), JsError> {
        match obj {
            JsValue::Object(obj_ref) => {
                // Check if this is a proxy - delegate to proxy_set if so
                if matches!(obj_ref.borrow().exotic, ExoticObject::Proxy(_)) {
                    let prop_key = interp.property_key_from_value(key);
                    crate::interpreter::builtins::proxy::proxy_set(
                        interp,
                        obj_ref.cheap_clone(),
                        prop_key,
                        value,
                        obj.clone(),
                    )?;
                    return Ok(());
                }

                // Handle __proto__ special property - set prototype
                if let JsValue::String(k) = key {
                    if k.as_str() == "__proto__" {
                        match &value {
                            JsValue::Object(proto) => {
                                obj_ref.borrow_mut().prototype = Some(proto.clone());
                            }
                            JsValue::Null => {
                                obj_ref.borrow_mut().prototype = None;
                            }
                            _ => {
                                // Non-object, non-null values are ignored for __proto__ set
                            }
                        }
                        return Ok(());
                    }
                }

                let prop_key = interp.property_key_from_value(key);

                // Check if object is frozen/sealed or property is non-writable
                // First, check for accessor or non-writable property (including prototype chain)
                let setter_to_call = {
                    let obj_borrowed = obj_ref.borrow();
                    // Use get_property_descriptor to search prototype chain for setters
                    if let Some((prop, _from_proto)) =
                        obj_borrowed.get_property_descriptor(&prop_key)
                    {
                        if prop.is_accessor() {
                            // Clone setter for later invocation
                            Some(prop.setter().cloned())
                        } else if !prop.writable() {
                            return Err(JsError::type_error(format!(
                                "Cannot assign to read only property '{}'",
                                prop_key
                            )));
                        } else {
                            None
                        }
                    } else if !obj_borrowed.extensible {
                        return Err(JsError::type_error(format!(
                            "Cannot add property '{}' to non-extensible object",
                            prop_key
                        )));
                    } else {
                        None
                    }
                };

                // Call setter if we have one
                if let Some(maybe_setter) = setter_to_call {
                    if let Some(setter) = maybe_setter {
                        interp.call_function(JsValue::Object(setter), obj.clone(), &[value])?;
                    } else {
                        // Accessor property with no setter - throw TypeError in strict mode
                        return Err(JsError::type_error(format!(
                            "Cannot set property '{}' which has only a getter",
                            prop_key
                        )));
                    }
                    // Accessor property handled, return
                    return Ok(());
                }

                // Regular data property
                obj_ref.borrow_mut().set_property(prop_key, value);
                Ok(())
            }
            JsValue::Null => Err(JsError::type_error("Cannot set properties of null")),
            JsValue::Undefined => Err(JsError::type_error("Cannot set properties of undefined")),
            _ => Ok(()),
        }
    }

    /// Execute a return, running any pending finally blocks first
    // NOTE: review
    fn execute_return(
        &mut self,
        return_val: JsValue,
        interp: &mut Interpreter,
    ) -> Result<OpResult, JsError> {
        // Check if there's a try handler with a finally block that needs to run
        // We need to find try handlers for the current function (same call frame depth)
        let current_frame_depth = self.call_stack.len();

        // Find try handlers that belong to the current function
        if let Some(handler_idx) = self
            .try_stack
            .iter()
            .rposition(|h| h.frame_depth == current_frame_depth && h.finally_ip != 0)
        {
            // There's a finally block that needs to run
            let handler = self
                .try_stack
                .get(handler_idx)
                .cloned()
                .ok_or_else(|| JsError::internal_error("Missing try handler"))?;

            // Save the pending return with a guard to keep it alive during finally execution
            self.pending_completion = Some(PendingCompletion::Return(Guarded::from_value(
                return_val,
                &interp.heap,
            )));

            // Pop the try handler (we're exiting this try block)
            self.try_stack.truncate(handler_idx);

            // Jump to the finally block
            self.ip = handler.finally_ip;

            return Ok(OpResult::Continue);
        }

        // No finally block, do normal return
        if let Some(frame) = self.call_stack.pop() {
            self.ip = frame.return_ip;
            self.chunk = frame.return_chunk;
            self.registers.truncate(frame.registers_base);
            if let Some(env) = frame.saved_env {
                interp.env = env;
            }
            self.set_reg(frame.return_register, return_val);
            Ok(OpResult::Continue)
        } else {
            Ok(OpResult::Halt(Guarded::from_value(
                return_val,
                &interp.heap,
            )))
        }
    }

    /// Execute a break, running any pending finally blocks first
    // NOTE: review
    fn execute_break(&mut self, target: usize, try_depth: u8) -> Result<OpResult, JsError> {
        // Check if there's a try handler with a finally block between us and the target
        let target_try_depth = try_depth as usize;

        // Find the first try handler ABOVE target depth that has a finally block
        if let Some(handler_idx) = self
            .try_stack
            .iter()
            .enumerate()
            .skip(target_try_depth)
            .find(|(_, h)| h.finally_ip != 0)
            .map(|(i, _)| i)
        {
            // There's a finally block that needs to run
            let handler = self
                .try_stack
                .get(handler_idx)
                .cloned()
                .ok_or_else(|| JsError::internal_error("Missing try handler"))?;

            // Save the pending break
            self.pending_completion = Some(PendingCompletion::Break { target, try_depth });

            // Pop the try handler (we're exiting this try block)
            self.try_stack.truncate(handler_idx);

            // Jump to the finally block
            self.ip = handler.finally_ip;

            return Ok(OpResult::Continue);
        }

        // No finally block, do normal break (just jump)
        // Also pop try handlers down to the target level
        self.try_stack.truncate(target_try_depth);
        self.ip = target;
        Ok(OpResult::Continue)
    }

    /// Execute a continue, running any pending finally blocks first
    // NOTE: review
    fn execute_continue(&mut self, target: usize, try_depth: u8) -> Result<OpResult, JsError> {
        // Check if there's a try handler with a finally block between us and the target
        let target_try_depth = try_depth as usize;

        // Find the first try handler ABOVE target depth that has a finally block
        if let Some(handler_idx) = self
            .try_stack
            .iter()
            .enumerate()
            .skip(target_try_depth)
            .find(|(_, h)| h.finally_ip != 0)
            .map(|(i, _)| i)
        {
            // There's a finally block that needs to run
            let handler = self
                .try_stack
                .get(handler_idx)
                .cloned()
                .ok_or_else(|| JsError::internal_error("Missing try handler"))?;

            // Save the pending continue
            self.pending_completion = Some(PendingCompletion::Continue { target, try_depth });

            // Pop the try handler (we're exiting this try block)
            self.try_stack.truncate(handler_idx);

            // Jump to the finally block
            self.ip = handler.finally_ip;

            return Ok(OpResult::Continue);
        }

        // No finally block, do normal continue (just jump)
        // Also pop try handlers down to the target level
        self.try_stack.truncate(target_try_depth);
        self.ip = target;
        Ok(OpResult::Continue)
    }
}

/// Result of executing a single opcode
enum OpResult {
    /// Continue to next instruction
    Continue,
    /// Halt with a value
    Halt(Guarded),
    /// Suspend execution (for await)
    Suspend {
        promise: Guarded,
        resume_register: Register,
    },
    /// Yield a value (for generators)
    Yield {
        value: Guarded,
        resume_register: Register,
    },
    /// Yield* delegate to another iterator
    YieldStar {
        iterable: Guarded,
        resume_register: Register,
    },
    /// Call a function (for trampoline)
    Call {
        callee: JsValue,
        this_value: JsValue,
        args: Vec<JsValue>,
        return_register: Register,
        new_target: JsValue,
        /// If true, this is a super() call and the callee should be set as current_constructor
        is_super_call: bool,
        /// Guard keeping all object values alive between OpResult creation and trampoline handling
        guard: Guard<JsObject>,
    },
    /// Construct a new object (for trampoline)
    Construct {
        callee: JsValue,
        this_value: JsValue,
        args: Vec<JsValue>,
        return_register: Register,
        new_target: JsValue,
        /// The new object to use if constructor doesn't return an object
        new_obj: Gc<JsObject>,
        /// Guard keeping all object values alive between OpResult creation and trampoline handling
        guard: Guard<JsObject>,
    },
}
