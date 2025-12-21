//! BytecodeBuilder - helper for emitting bytecode instructions
//!
//! Provides a convenient API for building bytecode chunks with
//! register allocation and jump patching support.

use super::bytecode::{
    BytecodeChunk, Constant, ConstantIndex, FunctionInfo, JumpTarget, Op, Register, SourceMapEntry,
};
use crate::error::JsError;
use crate::lexer::Span;
use crate::value::JsString;
use rustc_hash::FxHashMap;
use std::rc::Rc;

/// Placeholder for a jump that needs to be patched later
#[derive(Debug, Clone, Copy)]
pub struct JumpPlaceholder {
    /// Index of the jump instruction in the code
    pub instruction_index: usize,
}

/// Register allocator for bytecode compilation
#[derive(Debug)]
pub struct RegisterAllocator {
    /// Next available register
    next: u8,

    /// Stack of saved positions (for nested expressions)
    saved: Vec<u8>,

    /// Maximum register used (for determining register_count)
    max_used: u8,

    /// Free list for reusing registers
    free_list: Vec<u8>,
}

impl RegisterAllocator {
    /// Create a new register allocator
    pub fn new() -> Self {
        Self {
            next: 0,
            saved: Vec::new(),
            max_used: 0,
            free_list: Vec::new(),
        }
    }

    /// Allocate a register
    pub fn alloc(&mut self) -> Result<Register, JsError> {
        // First try to reuse a freed register
        if let Some(r) = self.free_list.pop() {
            return Ok(r);
        }

        // Otherwise allocate a new one
        if self.next == 255 {
            return Err(JsError::internal_error(
                "Too many registers needed (max 255)",
            ));
        }

        let r = self.next;
        self.next += 1;
        self.max_used = self.max_used.max(self.next);
        Ok(r)
    }

    /// Free a register for reuse
    pub fn free(&mut self, r: Register) {
        // Only add to free list if it's the most recently allocated
        // This keeps register usage contiguous
        if r == self.next.saturating_sub(1) {
            self.next = r;
        } else {
            self.free_list.push(r);
        }
    }

    /// Reserve a specific number of consecutive registers (for function args)
    pub fn reserve_range(&mut self, count: u8) -> Result<Register, JsError> {
        // Check for overflow - if checked_add returns None, we've exceeded u8::MAX
        if self.next.checked_add(count).is_none() {
            return Err(JsError::internal_error(
                "Too many registers needed (max 255)",
            ));
        }

        let start = self.next;
        self.next += count;
        self.max_used = self.max_used.max(self.next);
        Ok(start)
    }

    /// Save current allocation state (for nested expressions)
    pub fn save(&mut self) {
        self.saved.push(self.next);
    }

    /// Restore previous allocation state
    pub fn restore(&mut self) {
        if let Some(pos) = self.saved.pop() {
            self.next = pos;
            // Clear free list since we're restoring to an earlier state
            self.free_list.retain(|&r| r < pos);
        }
    }

    /// Get the maximum number of registers used
    pub fn max_used(&self) -> u8 {
        self.max_used
    }

    /// Get current register count (for determining register file size)
    pub fn current(&self) -> u8 {
        self.next
    }
}

impl Default for RegisterAllocator {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing bytecode chunks
pub struct BytecodeBuilder {
    /// Bytecode instructions
    code: Vec<Op>,

    /// Constant pool
    constants: Vec<Constant>,

    /// String constant deduplication map
    string_map: FxHashMap<JsString, ConstantIndex>,

    /// Number constant deduplication map
    number_map: FxHashMap<u64, ConstantIndex>,

    /// Source map entries
    source_map: Vec<SourceMapEntry>,

    /// Register allocator
    registers: RegisterAllocator,

    /// Current source span (for source map)
    current_span: Option<Span>,

    /// Function info (if compiling a function)
    function_info: Option<FunctionInfo>,
}

impl BytecodeBuilder {
    /// Create a new bytecode builder
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            string_map: FxHashMap::default(),
            number_map: FxHashMap::default(),
            source_map: Vec::new(),
            registers: RegisterAllocator::new(),
            current_span: None,
            function_info: None,
        }
    }

    /// Create a builder for a function
    pub fn for_function(info: FunctionInfo) -> Self {
        let mut builder = Self::new();
        builder.function_info = Some(info);
        builder
    }

    /// Get access to the register allocator
    pub fn registers(&mut self) -> &mut RegisterAllocator {
        &mut self.registers
    }

    /// Set the current source span for source map
    pub fn set_span(&mut self, span: Span) {
        self.current_span = Some(span);
    }

    /// Clear the current source span
    pub fn clear_span(&mut self) {
        self.current_span = None;
    }

    /// Emit an instruction and return its index
    pub fn emit(&mut self, op: Op) -> usize {
        let index = self.code.len();

        // Add source map entry if we have a span
        if let Some(span) = self.current_span {
            // Only add if different from the last entry
            let should_add = self
                .source_map
                .last()
                .is_none_or(|e| e.span.start != span.start);

            if should_add {
                self.source_map.push(SourceMapEntry {
                    bytecode_offset: index,
                    span,
                });
            }
        }

        self.code.push(op);
        index
    }

    /// Emit a jump instruction with a placeholder target
    pub fn emit_jump(&mut self) -> JumpPlaceholder {
        let index = self.emit(Op::Jump { target: 0 });
        JumpPlaceholder {
            instruction_index: index,
        }
    }

    /// Emit a conditional jump (if true) with a placeholder target
    pub fn emit_jump_if_true(&mut self, cond: Register) -> JumpPlaceholder {
        let index = self.emit(Op::JumpIfTrue { cond, target: 0 });
        JumpPlaceholder {
            instruction_index: index,
        }
    }

    /// Emit a conditional jump (if false) with a placeholder target
    pub fn emit_jump_if_false(&mut self, cond: Register) -> JumpPlaceholder {
        let index = self.emit(Op::JumpIfFalse { cond, target: 0 });
        JumpPlaceholder {
            instruction_index: index,
        }
    }

    /// Emit a conditional jump (if nullish) with a placeholder target
    pub fn emit_jump_if_nullish(&mut self, cond: Register) -> JumpPlaceholder {
        let index = self.emit(Op::JumpIfNullish { cond, target: 0 });
        JumpPlaceholder {
            instruction_index: index,
        }
    }

    /// Emit a conditional jump (if NOT nullish) with a placeholder target
    pub fn emit_jump_if_not_nullish(&mut self, cond: Register) -> JumpPlaceholder {
        let index = self.emit(Op::JumpIfNotNullish { cond, target: 0 });
        JumpPlaceholder {
            instruction_index: index,
        }
    }

    /// Emit a jump to a known target
    pub fn emit_jump_to(&mut self, target: usize) {
        self.emit(Op::Jump {
            target: target as JumpTarget,
        });
    }

    /// Patch a jump placeholder to jump to the current position
    pub fn patch_jump(&mut self, placeholder: JumpPlaceholder) {
        let target = self.code.len() as JumpTarget;
        self.patch_jump_to(placeholder, target);
    }

    /// Patch a jump placeholder to jump to a specific target
    ///
    /// NOTE: All Op variants with a JumpTarget field must be listed here.
    /// We explicitly list non-jump variants to get compile errors when new jump ops are added.
    pub fn patch_jump_to(&mut self, placeholder: JumpPlaceholder, target: JumpTarget) {
        if let Some(op) = self.code.get_mut(placeholder.instruction_index) {
            match op {
                // Jump instructions - patch the target
                Op::Jump { target: t } => *t = target,
                Op::JumpIfTrue { target: t, .. } => *t = target,
                Op::JumpIfFalse { target: t, .. } => *t = target,
                Op::JumpIfNullish { target: t, .. } => *t = target,
                Op::JumpIfNotNullish { target: t, .. } => *t = target,
                Op::IteratorDone { target: t, .. } => *t = target,

                // PushTry has targets but is patched via patch_try_targets()
                Op::PushTry { .. } => {}

                // All other opcodes - explicitly listed to catch new jump ops at compile time
                Op::LoadConst { .. }
                | Op::LoadUndefined { .. }
                | Op::LoadNull { .. }
                | Op::LoadBool { .. }
                | Op::LoadInt { .. }
                | Op::Move { .. }
                | Op::Add { .. }
                | Op::Sub { .. }
                | Op::Mul { .. }
                | Op::Div { .. }
                | Op::Mod { .. }
                | Op::Exp { .. }
                | Op::Eq { .. }
                | Op::NotEq { .. }
                | Op::StrictEq { .. }
                | Op::StrictNotEq { .. }
                | Op::Lt { .. }
                | Op::LtEq { .. }
                | Op::Gt { .. }
                | Op::GtEq { .. }
                | Op::BitAnd { .. }
                | Op::BitOr { .. }
                | Op::BitXor { .. }
                | Op::LShift { .. }
                | Op::RShift { .. }
                | Op::URShift { .. }
                | Op::In { .. }
                | Op::Instanceof { .. }
                | Op::Neg { .. }
                | Op::Plus { .. }
                | Op::Not { .. }
                | Op::BitNot { .. }
                | Op::Typeof { .. }
                | Op::Void { .. }
                | Op::GetVar { .. }
                | Op::SetVar { .. }
                | Op::DeclareVar { .. }
                | Op::DeclareVarHoisted { .. }
                | Op::GetGlobal { .. }
                | Op::SetGlobal { .. }
                | Op::CreateObject { .. }
                | Op::CreateArray { .. }
                | Op::GetProperty { .. }
                | Op::GetPropertyConst { .. }
                | Op::SetProperty { .. }
                | Op::SetPropertyConst { .. }
                | Op::DeleteProperty { .. }
                | Op::DeletePropertyConst { .. }
                | Op::DefineProperty { .. }
                | Op::Call { .. }
                | Op::CallSpread { .. }
                | Op::CallMethod { .. }
                | Op::Construct { .. }
                | Op::ConstructSpread { .. }
                | Op::Return { .. }
                | Op::ReturnUndefined
                | Op::CreateClosure { .. }
                | Op::CreateArrow { .. }
                | Op::CreateGenerator { .. }
                | Op::CreateAsync { .. }
                | Op::CreateAsyncGenerator { .. }
                | Op::Throw { .. }
                | Op::PopTry
                | Op::GetException { .. }
                | Op::Rethrow
                | Op::Await { .. }
                | Op::Yield { .. }
                | Op::YieldStar { .. }
                | Op::PushScope
                | Op::PopScope
                | Op::GetIterator { .. }
                | Op::GetKeysIterator { .. }
                | Op::GetAsyncIterator { .. }
                | Op::IteratorNext { .. }
                | Op::IteratorValue { .. }
                | Op::CreateClass { .. }
                | Op::DefineMethod { .. }
                | Op::DefineAccessor { .. }
                | Op::SuperCall { .. }
                | Op::SuperGet { .. }
                | Op::SuperGetConst { .. }
                | Op::SuperSet { .. }
                | Op::SuperSetConst { .. }
                | Op::SpreadArray { .. }
                | Op::CreateRestArray { .. }
                | Op::CreateObjectRest { .. }
                | Op::TemplateConcat { .. }
                | Op::TaggedTemplate { .. }
                | Op::Nop
                | Op::Halt
                | Op::Debugger
                | Op::Pop
                | Op::Dup { .. }
                | Op::LoadThis { .. }
                | Op::LoadArguments { .. }
                | Op::LoadNewTarget { .. } => {}
            }
        }
    }

    /// Patch PushTry instruction with catch and finally targets
    pub fn patch_try_targets(
        &mut self,
        idx: usize,
        catch_target: JumpTarget,
        finally_target: JumpTarget,
    ) {
        if let Some(Op::PushTry {
            catch_target: ct,
            finally_target: ft,
        }) = self.code.get_mut(idx)
        {
            *ct = catch_target;
            *ft = finally_target;
        }
    }

    /// Get the current instruction offset (for jump targets)
    pub fn current_offset(&self) -> usize {
        self.code.len()
    }

    /// Add a string constant to the pool (with deduplication)
    pub fn add_string(&mut self, s: JsString) -> Result<ConstantIndex, JsError> {
        if let Some(&idx) = self.string_map.get(&s) {
            return Ok(idx);
        }

        let idx = self.add_constant(Constant::String(s.cheap_clone()))?;
        self.string_map.insert(s, idx);
        Ok(idx)
    }

    /// Add a number constant to the pool (with deduplication)
    pub fn add_number(&mut self, n: f64) -> Result<ConstantIndex, JsError> {
        let bits = n.to_bits();
        if let Some(&idx) = self.number_map.get(&bits) {
            return Ok(idx);
        }

        let idx = self.add_constant(Constant::Number(n))?;
        self.number_map.insert(bits, idx);
        Ok(idx)
    }

    /// Add a constant to the pool
    pub fn add_constant(&mut self, constant: Constant) -> Result<ConstantIndex, JsError> {
        if self.constants.len() >= u16::MAX as usize {
            return Err(JsError::internal_error("Too many constants (max 65535)"));
        }

        let idx = self.constants.len() as ConstantIndex;
        self.constants.push(constant);
        Ok(idx)
    }

    /// Add a nested bytecode chunk (for functions)
    pub fn add_chunk(&mut self, chunk: BytecodeChunk) -> Result<ConstantIndex, JsError> {
        self.add_constant(Constant::Chunk(Rc::new(chunk)))
    }

    /// Add excluded keys for object rest destructuring
    pub fn add_excluded_keys(&mut self, keys: Vec<JsString>) -> Result<ConstantIndex, JsError> {
        self.add_constant(Constant::ExcludedKeys(keys))
    }

    /// Emit LoadConst for a string
    pub fn emit_load_string(&mut self, dst: Register, s: JsString) -> Result<(), JsError> {
        let idx = self.add_string(s)?;
        self.emit(Op::LoadConst { dst, idx });
        Ok(())
    }

    /// Emit LoadConst for a number
    pub fn emit_load_number(&mut self, dst: Register, n: f64) -> Result<(), JsError> {
        // Optimize small integers
        if n.fract() == 0.0 && n >= i32::MIN as f64 && n <= i32::MAX as f64 {
            let i = n as i32;
            // Use LoadInt for small integers
            if (-128..=127).contains(&i) {
                self.emit(Op::LoadInt { dst, value: i });
                return Ok(());
            }
        }

        let idx = self.add_number(n)?;
        self.emit(Op::LoadConst { dst, idx });
        Ok(())
    }

    /// Emit Halt instruction
    pub fn emit_halt(&mut self) {
        self.emit(Op::Halt);
    }

    /// Finish building and return the bytecode chunk
    pub fn finish(self) -> BytecodeChunk {
        BytecodeChunk {
            code: self.code,
            constants: self.constants,
            source_map: self.source_map,
            register_count: self.registers.max_used(),
            function_info: self.function_info,
        }
    }

    /// Allocate a register
    pub fn alloc_register(&mut self) -> Result<Register, JsError> {
        self.registers.alloc()
    }

    /// Free a register
    pub fn free_register(&mut self, r: Register) {
        self.registers.free(r);
    }

    /// Reserve a range of consecutive registers
    pub fn reserve_registers(&mut self, count: u8) -> Result<Register, JsError> {
        self.registers.reserve_range(count)
    }
}

impl Default for BytecodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Import CheapClone for JsString
use crate::value::CheapClone;
