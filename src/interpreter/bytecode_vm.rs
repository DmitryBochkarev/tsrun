//! Bytecode Virtual Machine
//!
//! This module implements the bytecode interpreter that executes compiled bytecode.
//! It uses a register-based design with up to 256 virtual registers per call frame.

use crate::compiler::{BytecodeChunk, Constant, Op, Register};
use crate::error::JsError;
use crate::gc::{Gc, Guard};
use crate::value::{CheapClone, Guarded, JsObject, JsString, JsValue, PropertyKey};
use std::rc::Rc;

use super::Interpreter;

/// Result of VM execution
pub enum VmResult {
    /// Execution completed with a value
    Complete(Guarded),
    /// Need to suspend for await/yield
    Suspend(VmSuspension),
    /// Error occurred
    Error(JsError),
}

/// Suspension state for async/generator
pub struct VmSuspension {
    /// The promise or generator we're waiting on
    pub waiting_on: Gc<JsObject>,
    /// Saved VM state for resumption
    pub state: SavedVmState,
}

/// Saved VM state for suspension/resumption
pub struct SavedVmState {
    /// Saved call frames
    pub frames: Vec<CallFrame>,
    /// Current instruction pointer
    pub ip: usize,
    /// Current bytecode chunk
    pub chunk: Rc<BytecodeChunk>,
    /// Register values (as JsValues - guards recreated on resume)
    pub registers: Vec<JsValue>,
    /// Exception handlers
    pub try_stack: Vec<TryHandler>,
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

/// The bytecode virtual machine
pub struct BytecodeVM {
    /// Current instruction pointer
    ip: usize,
    /// Current bytecode chunk being executed
    chunk: Rc<BytecodeChunk>,
    /// Register file
    registers: Vec<JsValue>,
    /// Guard keeping all register values alive
    #[allow(dead_code)]
    register_guard: Option<Guard<JsObject>>,
    /// Call stack (return addresses)
    call_stack: Vec<CallFrame>,
    /// Exception handler stack
    try_stack: Vec<TryHandler>,
    /// Current `this` value
    this_value: JsValue,
    /// Current exception value (for catch blocks)
    exception_value: Option<JsValue>,
    /// Saved environment for scope restoration
    saved_env: Option<Gc<JsObject>>,
}

impl BytecodeVM {
    /// Create a new VM for executing the given bytecode chunk
    pub fn new(chunk: Rc<BytecodeChunk>, this_value: JsValue) -> Self {
        let register_count = chunk.register_count as usize;
        Self {
            ip: 0,
            chunk,
            registers: vec![JsValue::Undefined; register_count.max(1)],
            register_guard: None,
            call_stack: Vec::new(),
            try_stack: Vec::new(),
            this_value,
            exception_value: None,
            saved_env: None,
        }
    }

    /// Create a new VM with a guard for keeping objects alive
    pub fn with_guard(
        chunk: Rc<BytecodeChunk>,
        this_value: JsValue,
        guard: Guard<JsObject>,
    ) -> Self {
        let register_count = chunk.register_count as usize;
        Self {
            ip: 0,
            chunk,
            registers: vec![JsValue::Undefined; register_count.max(1)],
            register_guard: Some(guard),
            call_stack: Vec::new(),
            try_stack: Vec::new(),
            this_value,
            exception_value: None,
            saved_env: None,
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
    fn set_reg(&mut self, r: Register, value: JsValue) {
        let idx = r as usize;
        if idx < self.registers.len() {
            if let Some(slot) = self.registers.get_mut(idx) {
                *slot = value;
            }
        }
    }

    /// Fetch the next instruction and advance IP
    #[inline]
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
                return VmResult::Complete(Guarded {
                    value: result,
                    guard: Some(guard),
                });
            };

            match self.execute_op(interp, op) {
                Ok(OpResult::Continue) => continue,
                Ok(OpResult::Halt(value)) => {
                    let guard = interp.heap.create_guard();
                    return VmResult::Complete(Guarded {
                        value,
                        guard: Some(guard),
                    });
                }
                Ok(OpResult::Suspend(obj)) => {
                    return VmResult::Suspend(VmSuspension {
                        waiting_on: obj,
                        state: self.save_state(),
                    });
                }
                Err(e) => {
                    // Try to find an exception handler
                    if let Some(handler_ip) = self.find_exception_handler() {
                        self.ip = handler_ip;
                        self.exception_value = Some(self.error_to_value(interp, &e));
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
                // Create an error object
                let guard = interp.heap.create_guard();
                let obj = interp.create_object(&guard);
                obj.borrow_mut().set_property(
                    PropertyKey::from("message"),
                    JsValue::String(JsString::from(error.to_string())),
                );
                obj.borrow_mut().set_property(
                    PropertyKey::from("name"),
                    JsValue::String(JsString::from("Error")),
                );
                JsValue::Object(obj)
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
                return Some(handler.catch_ip);
            }
            if handler.finally_ip > 0 {
                return Some(handler.finally_ip);
            }
        }
        None
    }

    /// Save VM state for suspension
    fn save_state(&self) -> SavedVmState {
        SavedVmState {
            frames: self.call_stack.clone(),
            ip: self.ip,
            chunk: self.chunk.clone(),
            registers: self.registers.clone(),
            try_stack: self.try_stack.clone(),
        }
    }

    /// Restore VM state from suspension
    #[allow(dead_code)]
    pub fn restore_state(&mut self, state: SavedVmState, guard: Guard<JsObject>) {
        self.call_stack = state.frames;
        self.ip = state.ip;
        self.chunk = state.chunk;
        self.registers = state.registers;
        self.try_stack = state.try_stack;
        self.register_guard = Some(guard);
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
                let left_val = self.get_reg(left).to_number();
                let right_val = self.get_reg(right).to_number();
                self.set_reg(dst, JsValue::Number(left_val - right_val));
                Ok(OpResult::Continue)
            }

            Op::Mul { dst, left, right } => {
                let left_val = self.get_reg(left).to_number();
                let right_val = self.get_reg(right).to_number();
                self.set_reg(dst, JsValue::Number(left_val * right_val));
                Ok(OpResult::Continue)
            }

            Op::Div { dst, left, right } => {
                let left_val = self.get_reg(left).to_number();
                let right_val = self.get_reg(right).to_number();
                self.set_reg(dst, JsValue::Number(left_val / right_val));
                Ok(OpResult::Continue)
            }

            Op::Mod { dst, left, right } => {
                let left_val = self.get_reg(left).to_number();
                let right_val = self.get_reg(right).to_number();
                self.set_reg(dst, JsValue::Number(left_val % right_val));
                Ok(OpResult::Continue)
            }

            Op::Exp { dst, left, right } => {
                let left_val = self.get_reg(left).to_number();
                let right_val = self.get_reg(right).to_number();
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
                let val = self.get_reg(src).to_number();
                self.set_reg(dst, JsValue::Number(-val));
                Ok(OpResult::Continue)
            }

            Op::Plus { dst, src } => {
                let val = self.get_reg(src).to_number();
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
                self.set_reg(dst, JsValue::String(JsString::from(type_str)));
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
                    .set_property(PropertyKey::from(name.as_str()), value);
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
                self.set_property_value(obj_val, key_val, val)?;
                Ok(OpResult::Continue)
            }

            Op::SetPropertyConst { obj, key, value } => {
                let obj_val = self.get_reg(obj);
                let key = self
                    .get_string_constant(key)
                    .ok_or_else(|| JsError::internal_error("Invalid property key constant"))?;
                let val = self.get_reg(value).clone();
                let key_val = JsValue::String(key);
                self.set_property_value(obj_val, &key_val, val)?;
                Ok(OpResult::Continue)
            }

            Op::DeleteProperty { dst, obj, key } => {
                let obj_val = self.get_reg(obj);
                let key_val = self.get_reg(key);

                if let JsValue::Object(obj_ref) = obj_val {
                    let prop_key = PropertyKey::from_value(key_val);
                    obj_ref.borrow_mut().properties.remove(&prop_key);
                    self.set_reg(dst, JsValue::Boolean(true));
                } else {
                    self.set_reg(dst, JsValue::Boolean(false));
                }
                Ok(OpResult::Continue)
            }

            Op::DeletePropertyConst { dst, obj, key } => {
                let obj_val = self.get_reg(obj);
                let key = self
                    .get_string_constant(key)
                    .ok_or_else(|| JsError::internal_error("Invalid property key constant"))?;

                if let JsValue::Object(obj_ref) = obj_val {
                    let prop_key = PropertyKey::from(key.as_str());
                    obj_ref.borrow_mut().properties.remove(&prop_key);
                    self.set_reg(dst, JsValue::Boolean(true));
                } else {
                    self.set_reg(dst, JsValue::Boolean(false));
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

            Op::Return { value } => {
                let return_val = self.get_reg(value).clone();

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
                    Ok(OpResult::Halt(return_val))
                }
            }

            Op::ReturnUndefined => {
                if let Some(frame) = self.call_stack.pop() {
                    self.ip = frame.return_ip;
                    self.chunk = frame.return_chunk;
                    self.registers.truncate(frame.registers_base);
                    if let Some(env) = frame.saved_env {
                        interp.env = env;
                    }
                    self.set_reg(frame.return_register, JsValue::Undefined);
                    Ok(OpResult::Continue)
                } else {
                    Ok(OpResult::Halt(JsValue::Undefined))
                }
            }

            Op::CreateClosure { dst, chunk_idx: _ }
            | Op::CreateArrow { dst, chunk_idx: _ }
            | Op::CreateGenerator { dst, chunk_idx: _ }
            | Op::CreateAsync { dst, chunk_idx: _ }
            | Op::CreateAsyncGenerator { dst, chunk_idx: _ } => {
                // Stub: function creation requires more complex handling
                self.set_reg(dst, JsValue::Undefined);
                Err(JsError::internal_error(
                    "Function creation in bytecode VM not yet implemented",
                ))
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
            // Async/Generator (stub implementations)
            // ═══════════════════════════════════════════════════════════════════════════
            Op::Await { dst: _, promise } => {
                let promise_val = self.get_reg(promise);
                if let JsValue::Object(obj) = promise_val {
                    Ok(OpResult::Suspend(obj.clone()))
                } else {
                    Err(JsError::internal_error(
                        "Await on non-promise not yet implemented in VM",
                    ))
                }
            }

            Op::Yield { dst: _, value: _ } => {
                Err(JsError::internal_error("Yield not yet implemented in VM"))
            }

            Op::YieldStar {
                dst: _,
                iterable: _,
            } => Err(JsError::internal_error(
                "YieldStar not yet implemented in VM",
            )),

            // ═══════════════════════════════════════════════════════════════════════════
            // Scope Management
            // ═══════════════════════════════════════════════════════════════════════════
            Op::PushScope => {
                self.saved_env = Some(interp.push_scope());
                Ok(OpResult::Continue)
            }

            Op::PopScope => {
                if let Some(env) = self.saved_env.take() {
                    interp.pop_scope(env);
                }
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Iteration (stub implementations)
            // ═══════════════════════════════════════════════════════════════════════════
            Op::GetIterator { dst: _, obj: _ } => Err(JsError::internal_error(
                "GetIterator not yet implemented in VM",
            )),

            Op::GetAsyncIterator { dst: _, obj: _ } => Err(JsError::internal_error(
                "Async iterators not yet implemented in VM",
            )),

            Op::IteratorNext {
                dst: _,
                iterator: _,
            } => Err(JsError::internal_error(
                "IteratorNext not yet implemented in VM",
            )),

            Op::IteratorDone { result: _, target } => {
                // Stub: always done
                self.ip = target as usize;
                Ok(OpResult::Continue)
            }

            Op::IteratorValue { dst, result: _ } => {
                self.set_reg(dst, JsValue::Undefined);
                Ok(OpResult::Continue)
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Class Operations (stub implementations)
            // ═══════════════════════════════════════════════════════════════════════════
            Op::CreateClass { .. }
            | Op::DefineMethod { .. }
            | Op::DefineAccessor { .. }
            | Op::SuperCall { .. }
            | Op::SuperGet { .. }
            | Op::SuperGetConst { .. }
            | Op::SuperSet { .. }
            | Op::SuperSetConst { .. } => {
                Err(JsError::internal_error("Classes not yet implemented in VM"))
            }

            // ═══════════════════════════════════════════════════════════════════════════
            // Spread/Rest
            // ═══════════════════════════════════════════════════════════════════════════
            Op::SpreadArray { dst: _, src: _ } => Err(JsError::internal_error(
                "SpreadArray not yet implemented in VM",
            )),

            Op::CreateRestArray {
                dst: _,
                start_index: _,
            } => Err(JsError::internal_error(
                "CreateRestArray not yet implemented in VM",
            )),

            // ═══════════════════════════════════════════════════════════════════════════
            // Template Literals
            // ═══════════════════════════════════════════════════════════════════════════
            Op::TemplateConcat { dst, start, count } => {
                let mut result = String::new();
                for i in 0..count {
                    let val = self.get_reg(start + i);
                    result.push_str(val.to_js_string().as_str());
                }
                self.set_reg(dst, JsValue::String(JsString::from(result)));
                Ok(OpResult::Continue)
            }

            Op::TaggedTemplate { .. } => Err(JsError::internal_error(
                "TaggedTemplate not yet implemented in VM",
            )),

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
                Ok(OpResult::Halt(result))
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

            Op::LoadArguments { dst: _ } => Err(JsError::internal_error(
                "LoadArguments not yet implemented in VM",
            )),

            Op::LoadNewTarget { dst: _ } => Err(JsError::internal_error(
                "LoadNewTarget not yet implemented in VM",
            )),
        }
    }

    /// Get a property value from an object
    fn get_property_value(
        &self,
        interp: &Interpreter,
        obj: &JsValue,
        key: &JsValue,
    ) -> Result<JsValue, JsError> {
        match obj {
            JsValue::Object(obj_ref) => {
                let prop_key = PropertyKey::from_value(key);
                if let Some(val) = obj_ref.borrow().get_property(&prop_key) {
                    Ok(val.clone())
                } else {
                    Ok(JsValue::Undefined)
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
            JsValue::Symbol(_) => Ok(JsValue::Undefined),
        }
    }

    /// Set a property value on an object
    fn set_property_value(
        &self,
        obj: &JsValue,
        key: &JsValue,
        value: JsValue,
    ) -> Result<(), JsError> {
        match obj {
            JsValue::Object(obj_ref) => {
                let prop_key = PropertyKey::from_value(key);
                obj_ref.borrow_mut().set_property(prop_key, value);
                Ok(())
            }
            JsValue::Null => Err(JsError::type_error("Cannot set properties of null")),
            JsValue::Undefined => Err(JsError::type_error("Cannot set properties of undefined")),
            _ => Ok(()),
        }
    }
}

/// Result of executing a single opcode
enum OpResult {
    /// Continue to next instruction
    Continue,
    /// Halt with a value
    Halt(JsValue),
    /// Suspend execution (for await/yield)
    Suspend(Gc<JsObject>),
}
