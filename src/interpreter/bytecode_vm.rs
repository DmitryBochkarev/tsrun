//! Bytecode Virtual Machine
//!
//! This module implements the bytecode interpreter that executes compiled bytecode.
//! It uses a register-based design with up to 256 virtual registers per call frame.

use crate::compiler::{BytecodeChunk, Constant, Op, Register};
use crate::error::JsError;
use crate::gc::{Gc, Guard};
use crate::value::{
    BytecodeFunction, CheapClone, Guarded, JsObject, JsString, JsValue, Property, PropertyKey,
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
    /// The yielded value
    pub value: JsValue,
    /// Register to store the value passed to next() when resumed
    pub resume_register: Register,
    /// Saved VM state for resumption
    pub state: SavedVmState,
}

/// Generator yield* result
pub struct GeneratorYieldStar {
    /// The iterable to delegate to
    pub iterable: JsValue,
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
}

/// Pending completion to be executed after finally block
#[derive(Debug, Clone)]
pub enum PendingCompletion {
    /// Return this value after finally completes
    Return(JsValue),
    /// Rethrow this exception after finally completes
    Throw(JsValue),
    /// Break to target after finally completes
    Break { target: usize, try_depth: u8 },
    /// Continue to target after finally completes
    Continue { target: usize, try_depth: u8 },
}

/// The bytecode virtual machine
pub struct BytecodeVM {
    /// Current instruction pointer
    pub ip: usize,
    /// Current bytecode chunk being executed
    pub chunk: Rc<BytecodeChunk>,
    /// Register file
    pub registers: Vec<JsValue>,
    /// Guard keeping all register values alive
    register_guard: Guard<JsObject>,
    /// Call stack (return addresses)
    pub call_stack: Vec<CallFrame>,
    /// Exception handler stack
    pub try_stack: Vec<TryHandler>,
    /// Current `this` value
    this_value: JsValue,
    /// Current exception value (for catch blocks)
    exception_value: Option<JsValue>,
    /// Saved environment for scope restoration
    saved_env: Option<Gc<JsObject>>,
    /// Original arguments array (for `arguments` object)
    pub arguments: Vec<JsValue>,
    /// `new.target` value (constructor if called with new, undefined otherwise)
    pub new_target: JsValue,
    /// Pending completion to execute after finally block
    pending_completion: Option<PendingCompletion>,
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
            saved_env: None,
            arguments: Vec::new(),
            new_target: JsValue::Undefined,
            pending_completion: None,
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
            saved_env: None,
            arguments: args.to_vec(),
            new_target: JsValue::Undefined,
            pending_completion: None,
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
            saved_env: None,
            arguments: args.to_vec(),
            new_target,
            pending_completion: None,
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
    fn fetch(&mut self) -> Option<Op> {
        let op = self.chunk.get(self.ip)?.clone();
        self.ip += 1;
        Some(op)
    }

    /// Get a constant from the pool
    #[inline]
    fn get_constant(&self, idx: u16) -> Option<&Constant> {
        self.chunk.get_constant(idx)
    }

    /// Get a string constant from the pool
    fn get_string_constant(&self, idx: u16) -> Option<JsString> {
        match self.get_constant(idx)? {
            Constant::String(s) => Some(s.cheap_clone()),
            _ => None,
        }
    }

    /// Get the super constructor from the current function's __super__ property
    fn get_super_constructor(&self, interp: &mut Interpreter) -> Result<JsValue, JsError> {
        // Look up __super__ in the current function's properties
        // The function is stored in the closure environment
        let super_key = PropertyKey::String(interp.intern("__super__"));

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
        let super_key = PropertyKey::String(interp.intern("__super__"));
        let super_target_key = PropertyKey::String(interp.intern("__super_target__"));

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
                        value: value.value,
                        resume_register,
                        state: self.save_state(interp),
                    });
                }
                Ok(OpResult::YieldStar {
                    iterable,
                    resume_register,
                }) => {
                    return VmResult::YieldStar(GeneratorYieldStar {
                        iterable: iterable.value,
                        resume_register,
                        state: self.save_state(interp),
                    });
                }
                Err(e) => {
                    // Try to find an exception handler
                    if let Some(handler_ip) = self.find_exception_handler() {
                        self.ip = handler_ip;
                        let exc_val = self.error_to_value(interp, &e);
                        // Guard exception value if it's an object
                        if let JsValue::Object(obj) = &exc_val {
                            self.register_guard.guard(obj.cheap_clone());
                        }
                        self.exception_value = Some(exc_val);
                        continue;
                    }
                    return VmResult::Error(e);
                }
            }
        }
    }

    /// Convert an error to a JS value
    fn error_to_value(&self, interp: &mut Interpreter, error: &JsError) -> JsValue {
        match error {
            JsError::ThrownValue { value } => value.clone(),
            _ => {
                // Create an error object using the proper error type
                use crate::interpreter::builtins::error::create_error_object;
                let (value, _guard) = create_error_object(interp, error);
                value
            }
        }
    }

    /// Find an exception handler for the current position
    fn find_exception_handler(&mut self) -> Option<usize> {
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
                    });
                }
                return Some(handler.catch_ip);
            }
            if handler.finally_ip > 0 {
                return Some(handler.finally_ip);
            }
        }
        None
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
        if let Some(JsValue::Object(obj)) = &self.exception_value {
            guard.guard(obj.cheap_clone());
        }

        // Guard saved_env if present
        if let Some(ref env) = self.saved_env {
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

    /// Restore VM state from suspension
    #[allow(dead_code)]
    pub fn restore_state(&mut self, state: SavedVmState, guard: Guard<JsObject>) {
        // Guard this_value if it's an object
        if let JsValue::Object(obj) = &self.this_value {
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

        self.call_stack = state.frames;
        self.ip = state.ip;
        self.chunk = state.chunk;
        self.registers = state.registers;
        self.try_stack = state.try_stack;
        self.register_guard = guard;
        self.arguments = state.arguments;
        self.new_target = state.new_target;
    }

    /// Restore VM state from generator yield and set the sent value
    pub fn restore_from_yield(
        &mut self,
        state: SavedVmState,
        resume_register: Register,
        sent_value: JsValue,
        guard: Guard<JsObject>,
    ) {
        // Guard this_value if it's an object
        if let JsValue::Object(obj) = &self.this_value {
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

        // Guard sent value if it's an object
        if let JsValue::Object(obj) = &sent_value {
            guard.guard(obj.cheap_clone());
        }

        self.call_stack = state.frames;
        self.ip = state.ip;
        self.chunk = state.chunk;
        self.registers = state.registers;
        self.try_stack = state.try_stack;
        self.register_guard = guard;
        self.arguments = state.arguments;
        self.new_target = state.new_target;
        // Set the value passed to next() in the resume register
        self.set_reg(resume_register, sent_value);
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
            saved_env: None,
            arguments: state.arguments,
            new_target: state.new_target,
            pending_completion: None,
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
    pub fn inject_exception(&mut self, exception: JsValue) -> bool {
        // Guard exception value if it's an object
        if let JsValue::Object(obj) = &exception {
            self.register_guard.guard(obj.cheap_clone());
        }

        // Try to find an exception handler
        if let Some(handler_ip) = self.find_exception_handler() {
            self.ip = handler_ip;
            self.exception_value = Some(exception);
            true
        } else {
            // No handler found - store exception for propagation
            self.exception_value = Some(exception);
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
                let value = match self.get_constant(idx) {
                    Some(Constant::String(s)) => JsValue::String(s.cheap_clone()),
                    Some(Constant::Number(n)) => JsValue::Number(*n),
                    Some(Constant::RegExp { pattern, flags }) => {
                        let guard = interp.heap.create_guard();
                        let obj =
                            interp.create_regexp_literal(&guard, pattern.as_str(), flags.as_str());
                        JsValue::Object(obj)
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
                let left_val = self.get_reg(left).clone();
                let right_val = self.get_reg(right).clone();

                // First convert objects to primitives with "default" hint
                let left_prim = interp.coerce_to_primitive(&left_val, "default")?;
                let right_prim = interp.coerce_to_primitive(&right_val, "default")?;

                let result = match (&left_prim, &right_prim) {
                    (JsValue::String(a), _) => {
                        JsValue::String(a.cheap_clone() + &right_prim.to_js_string())
                    }
                    (_, JsValue::String(b)) => {
                        JsValue::String(left_prim.to_js_string() + b.as_str())
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

                let prop_key = PropertyKey::from_value(key);
                let has_prop = obj_ref.borrow().has_own_property(&prop_key);
                self.set_reg(dst, JsValue::Boolean(has_prop));
                Ok(OpResult::Continue)
            }

            Op::Instanceof { dst, left, right } => {
                let left_val = self.get_reg(left);
                let right_val = self.get_reg(right);

                // right must be a constructor (function with prototype)
                let JsValue::Object(right_obj) = right_val else {
                    return Err(JsError::type_error(
                        "Right-hand side of 'instanceof' is not an object",
                    ));
                };

                // Get right.prototype
                let proto_key = PropertyKey::String(interp.intern("prototype"));
                let right_proto = right_obj.borrow().get_property(&proto_key);
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

            Op::Break { target, try_depth } => self.execute_break(target as usize, try_depth),

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
                let global = interp.global.clone();
                let prop_key = PropertyKey::from(name.as_str());
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
                let result = self.get_property_value(interp, obj_val, key_val)?;
                self.set_reg(dst, result);
                Ok(OpResult::Continue)
            }

            Op::GetPropertyConst { dst, obj, key } => {
                let obj_val = self.get_reg(obj);
                let key = self
                    .get_string_constant(key)
                    .ok_or_else(|| JsError::internal_error("Invalid property key constant"))?;
                let key_val = JsValue::String(key);
                let result = self.get_property_value(interp, obj_val, &key_val)?;
                self.set_reg(dst, result);
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
                let obj_val = self.get_reg(obj).clone();
                let key_val = self.get_reg(key).clone();

                match &obj_val {
                    JsValue::Null => {
                        return Err(JsError::type_error("Cannot delete property of null"));
                    }
                    JsValue::Undefined => {
                        return Err(JsError::type_error("Cannot delete property of undefined"));
                    }
                    JsValue::Object(obj_ref) => {
                        let prop_key = PropertyKey::from_value(&key_val);

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
                let obj_val = self.get_reg(obj).clone();
                let key = self
                    .get_string_constant(key)
                    .ok_or_else(|| JsError::internal_error("Invalid property key constant"))?;

                match &obj_val {
                    JsValue::Null => {
                        return Err(JsError::type_error("Cannot delete property of null"));
                    }
                    JsValue::Undefined => {
                        return Err(JsError::type_error("Cannot delete property of undefined"));
                    }
                    JsValue::Object(obj_ref) => {
                        let prop_key = PropertyKey::from(key.as_str());

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
                let val = self.get_reg(value).clone();

                if let JsValue::Object(obj_ref) = obj_val {
                    let prop_key = PropertyKey::from_value(key_val);
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
                let callee_val = self.get_reg(callee).clone();
                let this_val = self.get_reg(this).clone();

                let mut args = Vec::with_capacity(argc as usize);
                for i in 0..argc {
                    args.push(self.get_reg(args_start + i).clone());
                }

                let result = interp.call_function(callee_val, this_val, &args)?;
                self.set_reg(dst, result.value);
                Ok(OpResult::Continue)
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

                let result = interp.call_function(callee_val, this_val, &args)?;
                self.set_reg(dst, result.value);
                Ok(OpResult::Continue)
            }

            Op::CallMethod {
                dst,
                obj,
                method,
                args_start,
                argc,
            } => {
                let obj_val = self.get_reg(obj).clone();
                let method_name = self
                    .get_string_constant(method)
                    .ok_or_else(|| JsError::internal_error("Invalid method name constant"))?;

                let callee =
                    self.get_property_value(interp, &obj_val, &JsValue::String(method_name))?;

                let mut args = Vec::with_capacity(argc as usize);
                for i in 0..argc {
                    args.push(self.get_reg(args_start + i).clone());
                }

                let result = interp.call_function(callee, obj_val, &args)?;
                self.set_reg(dst, result.value);
                Ok(OpResult::Continue)
            }

            Op::Construct {
                dst,
                callee,
                args_start,
                argc,
            } => {
                let callee_val = self.get_reg(callee).clone();

                let mut args = Vec::with_capacity(argc as usize);
                for i in 0..argc {
                    args.push(self.get_reg(args_start + i).clone());
                }

                // Inline constructor call logic (similar to evaluate_new)
                let JsValue::Object(ctor) = &callee_val else {
                    return Err(JsError::type_error("Constructor is not a callable object"));
                };

                // Create a new object
                let new_guard = interp.heap.create_guard();
                let new_obj = interp.create_object(&new_guard);

                // Get the constructor's prototype
                let proto_key = PropertyKey::String(interp.intern("prototype"));
                if let Some(JsValue::Object(proto)) = ctor.borrow().get_property(&proto_key) {
                    new_obj.borrow_mut().prototype = Some(proto.cheap_clone());
                }

                // Call the constructor with `this` set to the new object
                let this = JsValue::Object(new_obj.cheap_clone());
                let result = interp.call_function(callee_val.clone(), this.clone(), &args)?;

                // If constructor returns an object, use that; otherwise use the created object
                let final_val = match result.value {
                    JsValue::Object(obj) => JsValue::Object(obj),
                    _ => JsValue::Object(new_obj),
                };

                self.set_reg(dst, final_val);
                Ok(OpResult::Continue)
            }

            Op::ConstructSpread {
                dst,
                callee,
                args_start,
                argc: _,
            } => {
                // ConstructSpread: args_start points to an array of arguments
                let callee_val = self.get_reg(callee).clone();
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

                // Inline constructor call logic (same as Construct)
                let JsValue::Object(ctor) = &callee_val else {
                    return Err(JsError::type_error("Constructor is not a callable object"));
                };

                // Create a new object
                let new_guard = interp.heap.create_guard();
                let new_obj = interp.create_object(&new_guard);

                // Get the constructor's prototype
                let proto_key = PropertyKey::String(interp.intern("prototype"));
                if let Some(JsValue::Object(proto)) = ctor.borrow().get_property(&proto_key) {
                    new_obj.borrow_mut().prototype = Some(proto.cheap_clone());
                }

                // Call the constructor with `this` set to the new object
                let this = JsValue::Object(new_obj.cheap_clone());
                let result = interp.call_function(callee_val.clone(), this.clone(), &args)?;

                // If constructor returns an object, use that; otherwise use the created object
                let final_val = match result.value {
                    JsValue::Object(obj) => JsValue::Object(obj),
                    _ => JsValue::Object(new_obj),
                };

                self.set_reg(dst, final_val);
                Ok(OpResult::Continue)
            }

            Op::Return { value } => {
                let return_val = self.get_reg(value).clone();
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
                Err(JsError::ThrownValue { value: val })
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
                });
                Ok(OpResult::Continue)
            }

            Op::PopTry => {
                self.try_stack.pop();
                Ok(OpResult::Continue)
            }

            Op::FinallyEnd => {
                // Complete any pending return/throw/break/continue after finally block finishes
                if let Some(pending) = self.pending_completion.take() {
                    match pending {
                        PendingCompletion::Return(val) => {
                            // Continue with the return (recursively handles nested finally blocks)
                            return self.execute_return(val, interp);
                        }
                        PendingCompletion::Throw(val) => {
                            // Re-throw the exception after finally
                            return Err(JsError::ThrownValue { value: val });
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
                let val = self.exception_value.take().unwrap_or(JsValue::Undefined);
                self.set_reg(dst, val);
                Ok(OpResult::Continue)
            }

            Op::Rethrow => {
                if let Some(val) = self.exception_value.take() {
                    Err(JsError::ThrownValue { value: val })
                } else {
                    Err(JsError::internal_error("No exception to rethrow"))
                }
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Async/Generator
            // ═══════════════════════════════════════════════════════════════════════════
            Op::Await { dst, promise } => {
                use crate::value::{ExoticObject, PromiseStatus};

                let promise_val = self.get_reg(promise).clone();

                // Check if it's a promise
                if let JsValue::Object(obj) = &promise_val {
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
                                return Err(JsError::thrown(reason));
                            }
                            PromiseStatus::Pending => {
                                // Suspend execution and wait for promise resolution
                                drop(state_ref);
                                drop(obj_ref);
                                return Ok(OpResult::Suspend {
                                    promise: guarded_js_value(promise_val, interp),
                                    resume_register: dst,
                                });
                            }
                        }
                    }
                }

                // Not a promise - treat as resolved value (await 42 === 42)
                self.set_reg(dst, promise_val);
                Ok(OpResult::Continue)
            }

            Op::Yield { dst, value } => {
                let yield_val = self.get_reg(value).clone();
                // Return a Yield result - the generator will be suspended
                // The dst register will receive the value passed to next() when resumed
                Ok(OpResult::Yield {
                    value: guarded_js_value(yield_val, interp),
                    resume_register: dst,
                })
            }

            Op::YieldStar { dst, iterable } => {
                // yield* delegates to another iterator
                let iterable_val = self.get_reg(iterable).clone();
                Ok(OpResult::YieldStar {
                    iterable: guarded_js_value(iterable_val, interp),
                    resume_register: dst,
                })
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Scope Management
            // ═══════════════════════════════════════════════════════════════════════════
            Op::PushScope => {
                let env = interp.push_scope();
                // Guard the saved environment
                self.register_guard.guard(env.cheap_clone());
                self.saved_env = Some(env);
                Ok(OpResult::Continue)
            }

            Op::PopScope => {
                if let Some(env) = self.saved_env.take() {
                    interp.pop_scope(env);
                }
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Iteration
            // ═══════════════════════════════════════════════════════════════════════════
            Op::GetIterator { dst, obj } => {
                let obj_val = self.get_reg(obj).clone();

                // For arrays and strings, create an internal array iterator
                // The iterator is stored as an object with internal index state
                match &obj_val {
                    JsValue::Object(obj_ref) => {
                        // Check if it's an array - use direct element iteration
                        if obj_ref.borrow().array_elements().is_some() {
                            // Create an iterator object with the array and index
                            // Use register_guard to keep it alive across loop iterations
                            let iter = interp.create_object(&self.register_guard);
                            iter.borrow_mut().set_property(
                                PropertyKey::from("__array__"),
                                JsValue::Object(obj_ref.clone()),
                            );
                            iter.borrow_mut()
                                .set_property(PropertyKey::from("__index__"), JsValue::Number(0.0));
                            self.set_reg(dst, JsValue::Object(iter));
                            return Ok(OpResult::Continue);
                        }

                        // For non-array objects, try Symbol.iterator
                        let well_known =
                            crate::interpreter::builtins::symbol::get_well_known_symbols();
                        let iterator_symbol = crate::value::JsSymbol::new(
                            well_known.iterator,
                            Some("Symbol.iterator".to_string()),
                        );
                        let iterator_key = PropertyKey::Symbol(Box::new(iterator_symbol));

                        let iterator_method = obj_ref.borrow().get_property(&iterator_key);

                        if let Some(JsValue::Object(method_obj)) = iterator_method {
                            // Call the iterator method
                            let result =
                                interp.call_function(JsValue::Object(method_obj), obj_val, &[])?;
                            // Guard the result iterator object
                            if let JsValue::Object(iter_obj) = &result.value {
                                self.register_guard.guard(iter_obj.cheap_clone());
                            }
                            self.set_reg(dst, result.value);
                        } else {
                            return Err(JsError::type_error("Object is not iterable"));
                        }
                    }
                    JsValue::String(s) => {
                        // Create a string iterator
                        // Use register_guard to keep it alive across loop iterations
                        let iter = interp.create_object(&self.register_guard);
                        iter.borrow_mut().set_property(
                            PropertyKey::from("__string__"),
                            JsValue::String(s.cheap_clone()),
                        );
                        iter.borrow_mut()
                            .set_property(PropertyKey::from("__index__"), JsValue::Number(0.0));
                        self.set_reg(dst, JsValue::Object(iter));
                    }
                    _ => {
                        return Err(JsError::type_error("Object is not iterable"));
                    }
                }
                Ok(OpResult::Continue)
            }

            Op::GetKeysIterator { dst, obj } => {
                let obj_val = self.get_reg(obj).clone();

                // Create a keys iterator that iterates over enumerable property keys
                let keys: Vec<JsValue> = match &obj_val {
                    JsValue::Object(obj_ref) => {
                        let obj_borrowed = obj_ref.borrow();
                        let mut result = Vec::new();

                        // For arrays, first add all array indices
                        if let Some(elements) = obj_borrowed.array_elements() {
                            for i in 0..elements.len() {
                                result.push(JsValue::String(JsString::from(i.to_string())));
                            }
                        }

                        // Then add own enumerable property keys (excluding indices already added)
                        for k in obj_borrowed.properties.keys() {
                            match k {
                                PropertyKey::String(s) => {
                                    result.push(JsValue::String(s.cheap_clone()));
                                }
                                PropertyKey::Index(i) => {
                                    // Only add if not an array (arrays already handled above)
                                    if obj_borrowed.array_elements().is_none() {
                                        result.push(JsValue::String(JsString::from(i.to_string())));
                                    }
                                }
                                _ => {} // Skip symbols for for-in
                            }
                        }
                        result
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
                let iter = interp.create_object(&self.register_guard);
                let keys_arr = interp.create_array_from(&self.register_guard, keys);
                iter.borrow_mut()
                    .set_property(PropertyKey::from("__keys__"), JsValue::Object(keys_arr));
                iter.borrow_mut()
                    .set_property(PropertyKey::from("__index__"), JsValue::Number(0.0));
                self.set_reg(dst, JsValue::Object(iter));
                Ok(OpResult::Continue)
            }

            Op::GetAsyncIterator { dst: _, obj: _ } => Err(JsError::internal_error(
                "Async iterators not yet implemented in VM",
            )),

            Op::IteratorNext { dst, iterator } => {
                let iter_val = self.get_reg(iterator).clone();

                let JsValue::Object(iter_obj) = iter_val else {
                    return Err(JsError::type_error("Iterator is not an object"));
                };

                // Check if this is our internal array iterator
                let array_prop = iter_obj
                    .borrow()
                    .get_property(&PropertyKey::from("__array__"));
                if let Some(JsValue::Object(arr_ref)) = array_prop {
                    let index = match iter_obj
                        .borrow()
                        .get_property(&PropertyKey::from("__index__"))
                    {
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
                    iter_obj.borrow_mut().set_property(
                        PropertyKey::from("__index__"),
                        JsValue::Number((index + 1) as f64),
                    );

                    // Create result object { value, done }
                    let guard = interp.heap.create_guard();
                    let result = interp.create_object(&guard);
                    result
                        .borrow_mut()
                        .set_property(PropertyKey::from("value"), value);
                    result
                        .borrow_mut()
                        .set_property(PropertyKey::from("done"), JsValue::Boolean(done));
                    self.set_reg(dst, JsValue::Object(result));
                    return Ok(OpResult::Continue);
                }

                // Check if this is our internal string iterator
                let string_prop = iter_obj
                    .borrow()
                    .get_property(&PropertyKey::from("__string__"));
                if let Some(JsValue::String(s)) = string_prop {
                    let index = match iter_obj
                        .borrow()
                        .get_property(&PropertyKey::from("__index__"))
                    {
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
                    iter_obj.borrow_mut().set_property(
                        PropertyKey::from("__index__"),
                        JsValue::Number((index + 1) as f64),
                    );

                    // Create result object { value, done }
                    let guard = interp.heap.create_guard();
                    let result = interp.create_object(&guard);
                    result
                        .borrow_mut()
                        .set_property(PropertyKey::from("value"), value);
                    result
                        .borrow_mut()
                        .set_property(PropertyKey::from("done"), JsValue::Boolean(done));
                    self.set_reg(dst, JsValue::Object(result));
                    return Ok(OpResult::Continue);
                }

                // Check if this is our internal keys iterator (for for-in)
                let keys_prop = iter_obj
                    .borrow()
                    .get_property(&PropertyKey::from("__keys__"));
                if let Some(JsValue::Object(keys_arr)) = keys_prop {
                    let index = match iter_obj
                        .borrow()
                        .get_property(&PropertyKey::from("__index__"))
                    {
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
                    iter_obj.borrow_mut().set_property(
                        PropertyKey::from("__index__"),
                        JsValue::Number((index + 1) as f64),
                    );

                    // Create result object { value, done }
                    let guard = interp.heap.create_guard();
                    let result = interp.create_object(&guard);
                    result
                        .borrow_mut()
                        .set_property(PropertyKey::from("value"), value);
                    result
                        .borrow_mut()
                        .set_property(PropertyKey::from("done"), JsValue::Boolean(done));
                    self.set_reg(dst, JsValue::Object(result));
                    return Ok(OpResult::Continue);
                }

                // For custom iterators, call next() method
                let next_key = PropertyKey::from("next");
                let next_method = iter_obj.borrow().get_property(&next_key);

                if let Some(JsValue::Object(next_fn)) = next_method {
                    let result = interp.call_function(
                        JsValue::Object(next_fn),
                        JsValue::Object(iter_obj),
                        &[],
                    )?;
                    self.set_reg(dst, result.value);
                } else {
                    return Err(JsError::type_error("Iterator must have a next method"));
                }

                Ok(OpResult::Continue)
            }

            Op::IteratorDone { result, target } => {
                let result_val = self.get_reg(result);

                let done = if let JsValue::Object(obj_ref) = result_val {
                    match obj_ref.borrow().get_property(&PropertyKey::from("done")) {
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
                        .get_property(&PropertyKey::from("value"))
                        .unwrap_or(JsValue::Undefined)
                } else {
                    JsValue::Undefined
                };

                self.set_reg(dst, value);
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
                let ctor_val = self.get_reg(constructor).clone();
                let JsValue::Object(ctor_obj) = ctor_val else {
                    return Err(JsError::type_error("Class constructor must be a function"));
                };

                // Create prototype object
                let guard = interp.heap.create_guard();
                let prototype = interp.create_object(&guard);

                // Handle superclass if provided
                let super_val = self.get_reg(super_class).clone();
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

                // Set constructor.prototype = prototype
                ctor_obj.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("prototype")),
                    JsValue::Object(prototype.cheap_clone()),
                );

                // Set prototype.constructor = constructor
                prototype.borrow_mut().set_property(
                    PropertyKey::String(interp.intern("constructor")),
                    JsValue::Object(ctor_obj.cheap_clone()),
                );

                self.set_reg(dst, JsValue::Object(ctor_obj));
                Ok(OpResult::Continue)
            }

            Op::DefineMethod {
                class,
                name,
                method,
                is_static,
            } => {
                let class_val = self.get_reg(class).clone();
                let JsValue::Object(class_obj) = class_val else {
                    return Err(JsError::type_error("Class is not an object"));
                };

                let method_val = self.get_reg(method).clone();
                let method_name = self
                    .get_string_constant(name)
                    .ok_or_else(|| JsError::internal_error("Invalid method name constant"))?;

                // Store __super__ and __super_target__ on method for super access
                if let JsValue::Object(method_obj) = &method_val {
                    // Copy __super__ from class constructor
                    if let Some(super_val) = class_obj
                        .borrow()
                        .get_property(&PropertyKey::String(interp.intern("__super__")))
                    {
                        method_obj.borrow_mut().set_property(
                            PropertyKey::String(interp.intern("__super__")),
                            super_val,
                        );
                    }

                    // Copy __super_target__ from class constructor
                    if let Some(super_target) = class_obj
                        .borrow()
                        .get_property(&PropertyKey::String(interp.intern("__super_target__")))
                    {
                        method_obj.borrow_mut().set_property(
                            PropertyKey::String(interp.intern("__super_target__")),
                            super_target,
                        );
                    }
                }

                // Use from_value to handle numeric string keys correctly (e.g., "2" -> Index(2))
                let prop_key = PropertyKey::from_value(&JsValue::String(method_name.cheap_clone()));

                if is_static {
                    // Add to class constructor directly
                    class_obj.borrow_mut().set_property(prop_key, method_val);
                } else {
                    // Add to prototype
                    let proto_key = PropertyKey::String(interp.intern("prototype"));
                    if let Some(JsValue::Object(proto)) =
                        class_obj.borrow().get_property(&proto_key)
                    {
                        proto.borrow_mut().set_property(prop_key, method_val);
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
                let class_val = self.get_reg(class).clone();
                let JsValue::Object(class_obj) = class_val else {
                    return Err(JsError::type_error("Class is not an object"));
                };

                let getter_val = self.get_reg(getter).clone();
                let setter_val = self.get_reg(setter).clone();
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
                    PropertyKey::from_value(&JsValue::String(accessor_name.cheap_clone()));
                let (existing_getter, existing_setter) = {
                    let target_ref = target.borrow();
                    if let Some(prop) = target_ref.properties.get(&prop_key) {
                        (prop.getter().cloned(), prop.setter().cloned())
                    } else {
                        (None, None)
                    }
                };

                // Merge with existing accessors
                let final_getter = new_getter.or(existing_getter);
                let final_setter = new_setter.or(existing_setter);

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

                let mut args = Vec::with_capacity(argc as usize);
                for i in 0..argc {
                    args.push(self.get_reg(args_start + i).clone());
                }

                // Call super constructor with current this
                let this = self.this_value.clone();
                let result = interp.call_function(super_ctor, this, &args)?;
                self.set_reg(dst, result.value);
                Ok(OpResult::Continue)
            }

            Op::SuperGet { dst, key } => {
                let key_val = self.get_reg(key).clone();
                let super_target = self.get_super_target(interp)?;
                let value = self.get_property_value(interp, &super_target, &key_val)?;
                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            Op::SuperGetConst { dst, key } => {
                let key_str = self
                    .get_string_constant(key)
                    .ok_or_else(|| JsError::internal_error("Invalid super property key"))?;
                let super_target = self.get_super_target(interp)?;
                let value =
                    self.get_property_value(interp, &super_target, &JsValue::String(key_str))?;
                self.set_reg(dst, value);
                Ok(OpResult::Continue)
            }

            Op::SuperSet { key, value } => {
                let key_val = self.get_reg(key).clone();
                let set_value = self.get_reg(value).clone();
                let super_target = self.get_super_target(interp)?;

                if let JsValue::Object(obj) = super_target {
                    let prop_key = PropertyKey::from_value(&key_val);
                    obj.borrow_mut().set_property(prop_key, set_value);
                }
                Ok(OpResult::Continue)
            }

            Op::SuperSetConst { key, value } => {
                let key_str = self
                    .get_string_constant(key)
                    .ok_or_else(|| JsError::internal_error("Invalid super property key"))?;
                let set_value = self.get_reg(value).clone();
                let super_target = self.get_super_target(interp)?;

                if let JsValue::Object(obj) = super_target {
                    obj.borrow_mut()
                        .set_property(PropertyKey::String(key_str), set_value);
                }
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Spread/Rest
            // ═══════════════════════════════════════════════════════════════════════════
            Op::SpreadArray { dst, src } => {
                // Spread elements from src iterable onto the dst array
                // dst should already be an array - we append elements to it
                let src_val = self.get_reg(src).clone();
                let dst_val = self.get_reg(dst).clone();

                let elements_to_add: Vec<JsValue> = match &src_val {
                    JsValue::Object(obj_ref) => {
                        if let Some(elems) = obj_ref.borrow().array_elements() {
                            elems.to_vec()
                        } else {
                            // Try iterator protocol
                            match interp.collect_iterator_values(&src_val) {
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
                let iter_val = self.get_reg(iter_reg).clone();

                let mut elements = Vec::new();

                if let JsValue::Object(iter_obj) = iter_val {
                    // Check for internal array iterator
                    let array_prop = iter_obj
                        .borrow()
                        .get_property(&PropertyKey::from("__array__"));
                    if let Some(JsValue::Object(arr_ref)) = array_prop {
                        let index = match iter_obj
                            .borrow()
                            .get_property(&PropertyKey::from("__index__"))
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
                let src_val = self.get_reg(src).clone();

                // Get excluded keys from constant pool
                let excluded = match self.chunk.constants.get(excluded_keys as usize) {
                    Some(Constant::ExcludedKeys(keys)) => keys.clone(),
                    _ => vec![],
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
                let dst_val = self.get_reg(dst).clone();
                let src_val = self.get_reg(src).clone();

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
                for i in 0..count {
                    let val = self.get_reg(start + i).clone();
                    // For objects, call toString method; for primitives, use to_js_string
                    let str_val = if let JsValue::Object(obj) = &val {
                        // Check if object has a custom toString method
                        let to_string_key = PropertyKey::String(interp.intern("toString"));
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
                                    Ok(Guarded { value, guard: _ }) => value.to_js_string(),
                                    Err(_) => val.to_js_string(),
                                }
                            } else {
                                val.to_js_string()
                            }
                        } else {
                            val.to_js_string()
                        }
                    } else {
                        val.to_js_string()
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
                let raw_key = PropertyKey::String(JsString::from("raw"));
                strings_arr
                    .borrow_mut()
                    .set_property(raw_key, JsValue::Object(raw_arr));

                // Build args: [strings_array, ...expressions]
                let mut args = vec![JsValue::Object(strings_arr)];
                for i in 0..exprs_count {
                    args.push(self.get_reg(exprs_start + i).clone());
                }

                // Get the tag function and this value
                let tag_fn = self.get_reg(tag).clone();
                let this_val = self.get_reg(this).clone();

                // Call the tag function
                let result = interp.call_function(tag_fn, this_val, &args)?;

                self.set_reg(dst, result.value);
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
                Ok(OpResult::Halt(guarded_js_value(result, interp)))
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
        }
    }

    /// Get a property value from an object, invoking getters if present
    fn get_property_value(
        &self,
        interp: &mut Interpreter,
        obj: &JsValue,
        key: &JsValue,
    ) -> Result<JsValue, JsError> {
        match obj {
            JsValue::Object(obj_ref) => {
                // Handle __proto__ special property - return prototype
                if let JsValue::String(k) = key {
                    if k.as_str() == "__proto__" {
                        return Ok(obj_ref
                            .borrow()
                            .prototype
                            .as_ref()
                            .map(|p| JsValue::Object(p.clone()))
                            .unwrap_or(JsValue::Null));
                    }
                }

                let prop_key = PropertyKey::from_value(key);
                // Get property descriptor to check for accessor properties
                let prop_desc = obj_ref.borrow().get_property_descriptor(&prop_key);
                match prop_desc {
                    Some((prop, _)) if prop.is_accessor() => {
                        // Property has a getter - invoke it
                        if let Some(getter) = prop.getter() {
                            let result = interp.call_function(
                                JsValue::Object(getter.clone()),
                                obj.clone(),
                                &[],
                            )?;
                            Ok(result.value)
                        } else {
                            Ok(JsValue::Undefined)
                        }
                    }
                    Some((prop, _)) => Ok(prop.value.clone()),
                    None => Ok(JsValue::Undefined),
                }
            }
            JsValue::String(s) => match key {
                JsValue::String(k) if k.as_str() == "length" => {
                    Ok(JsValue::Number(s.as_str().chars().count() as f64))
                }
                JsValue::Number(n) => {
                    let idx = *n as usize;
                    if let Some(c) = s.as_str().chars().nth(idx) {
                        return Ok(JsValue::String(JsString::from(c.to_string())));
                    }
                    Ok(JsValue::Undefined)
                }
                _ => {
                    let prop_key = PropertyKey::from_value(key);
                    if let Some(val) = interp.string_prototype.borrow().get_property(&prop_key) {
                        Ok(val.clone())
                    } else {
                        Ok(JsValue::Undefined)
                    }
                }
            },
            JsValue::Number(_) => {
                let prop_key = PropertyKey::from_value(key);
                if let Some(val) = interp.number_prototype.borrow().get_property(&prop_key) {
                    Ok(val.clone())
                } else {
                    Ok(JsValue::Undefined)
                }
            }
            JsValue::Boolean(_) => {
                let prop_key = PropertyKey::from_value(key);
                if let Some(val) = interp.boolean_prototype.borrow().get_property(&prop_key) {
                    Ok(val.clone())
                } else {
                    Ok(JsValue::Undefined)
                }
            }
            JsValue::Null => Err(JsError::type_error("Cannot read properties of null")),
            JsValue::Undefined => Err(JsError::type_error("Cannot read properties of undefined")),
            JsValue::Symbol(sym) => {
                // Symbols have a description property
                if let JsValue::String(k) = key {
                    if k.as_str() == "description" {
                        return Ok(sym
                            .description
                            .as_ref()
                            .map(|d| JsValue::String(JsString::from(d.as_str())))
                            .unwrap_or(JsValue::Undefined));
                    }
                }
                // Other symbol prototype methods
                let prop_key = PropertyKey::from_value(key);
                if let Some(val) = interp.symbol_prototype.borrow().get_property(&prop_key) {
                    Ok(val.clone())
                } else {
                    Ok(JsValue::Undefined)
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

                let prop_key = PropertyKey::from_value(key);

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

            // Save the pending return
            self.pending_completion = Some(PendingCompletion::Return(return_val));

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
            Ok(OpResult::Halt(guarded_js_value(return_val, interp)))
        }
    }

    /// Execute a break, running any pending finally blocks first
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
}

fn guarded_js_value(val: JsValue, interp: &Interpreter) -> Guarded {
    match &val {
        JsValue::Object(obj) => {
            let guard = interp.heap.create_guard();
            guard.guard(obj.cheap_clone());
            Guarded {
                value: val,
                guard: Some(guard),
            }
        }
        _ => Guarded {
            value: val,
            guard: None,
        },
    }
}
