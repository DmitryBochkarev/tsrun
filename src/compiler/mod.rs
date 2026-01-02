//! Bytecode compiler for TypeScript/JavaScript
//!
//! This module compiles the AST to bytecode for execution by the bytecode VM.
//! The bytecode uses a register-based design for better performance.

mod builder;
mod bytecode;
mod compile_expr;
mod compile_pattern;
mod compile_stmt;
mod hoist;

pub use builder::{BytecodeBuilder, JumpPlaceholder};
pub use bytecode::{BytecodeChunk, Constant, FunctionInfo, JumpTarget, Op, Register};

use crate::prelude::*;

use crate::ast::Program;
use crate::error::JsError;
use crate::value::JsString;
use builder::RegisterAllocator;

/// Compiler state for converting AST to bytecode
pub struct Compiler {
    /// Current bytecode builder
    builder: BytecodeBuilder,

    /// Loop context stack for break/continue
    loop_stack: Vec<LoopContext>,

    /// Label to loop index mapping
    labels: FxHashMap<JsString, usize>,

    /// Try block depth (for determining if we're in a try block)
    try_depth: usize,

    /// Set of variables that have been hoisted in the current scope
    /// Used to determine if we should emit DeclareVarHoisted or SetVar
    hoisted_vars: FxHashSet<JsString>,

    /// Loop variable redirects: when compiling for-loop updates, assignments to
    /// these variables should write to the register instead of the environment.
    /// This ensures closures capture pre-update values.
    loop_var_redirects: FxHashMap<JsString, Register>,

    /// Stack of class contexts for private field access
    /// Each class being compiled pushes its brand ID so inner code can access private fields
    class_context_stack: Vec<ClassContext>,

    /// Counter for generating unique class brand IDs
    next_class_brand: u32,

    /// Whether to track completion values (for eval)
    /// When true, register 0 is reserved for the completion value
    track_completion: bool,

    /// Source file path for stack traces (propagated to all nested chunks)
    source_file: Option<String>,
}

/// Context for a class being compiled (for private field handling)
#[derive(Clone)]
struct ClassContext {
    /// Unique brand ID for this class (for private field brand checking)
    brand: u32,
    /// Map from private field names (including #) to their info
    private_members: FxHashMap<JsString, PrivateMemberInfo>,
}

/// Information about a private class member
#[derive(Clone)]
#[allow(dead_code)]
struct PrivateMemberInfo {
    /// Whether this is a method (vs field)
    is_method: bool,
    /// Whether this is a static member
    is_static: bool,
}

/// Context for a loop (for break/continue handling)
struct LoopContext {
    /// Label for this loop (if any)
    label: Option<JsString>,
    /// Jump placeholders for break statements
    break_jumps: Vec<JumpPlaceholder>,
    /// Target instruction for continue
    continue_target: Option<usize>,
    /// Jump placeholders for continue before target is known
    continue_jumps: Vec<JumpPlaceholder>,
    /// Try depth when this loop started (for finally handling)
    try_depth: usize,
    /// Iterator register for for-of loops (for iterator close protocol)
    /// When set, break/return/throw should call iterator.return()
    iterator_reg: Option<Register>,
}

impl Compiler {
    /// Create a new compiler
    pub fn new() -> Self {
        Self {
            builder: BytecodeBuilder::new(),
            loop_stack: Vec::new(),
            labels: FxHashMap::default(),
            try_depth: 0,
            hoisted_vars: FxHashSet::default(),
            loop_var_redirects: FxHashMap::default(),
            class_context_stack: Vec::new(),
            next_class_brand: 0,
            track_completion: false,
            source_file: None,
        }
    }

    /// Create a new compiler with a source file path for stack traces
    pub fn with_source_file(source_file: String) -> Self {
        let mut compiler = Self::new();
        compiler.source_file = Some(source_file.clone());
        compiler.builder.set_source_file(source_file);
        compiler
    }

    /// Create a new compiler with completion value tracking enabled (for eval)
    fn new_with_completion_tracking() -> Self {
        let mut compiler = Self::new();
        compiler.track_completion = true;
        // Reserve register 0 for completion value and initialize to undefined
        let _ = compiler.builder.alloc_register(); // Reserve r0
        compiler.builder.emit(Op::LoadUndefined { dst: 0 });
        compiler
    }

    /// Generate a new unique class brand ID
    fn new_class_brand(&mut self) -> u32 {
        let brand = self.next_class_brand;
        self.next_class_brand += 1;
        brand
    }

    /// Get the current class context (if inside a class)
    #[allow(dead_code)]
    fn current_class_context(&self) -> Option<&ClassContext> {
        self.class_context_stack.last()
    }

    /// Look up a private member across all enclosing classes
    /// Returns (brand, info) for the class that declared this private member
    fn lookup_private_member(&self, name: &JsString) -> Option<(u32, &PrivateMemberInfo)> {
        // Search from innermost to outermost class
        for ctx in self.class_context_stack.iter().rev() {
            if let Some(info) = ctx.private_members.get(name) {
                return Some((ctx.brand, info));
            }
        }
        None
    }

    /// Compile a program to bytecode
    pub fn compile_program(program: &Program) -> Result<Rc<BytecodeChunk>, JsError> {
        let mut compiler = Compiler::new();

        // First, hoist all var declarations and function declarations to the top
        compiler.emit_hoisted_declarations(&program.body)?;

        // Then compile the statements
        compiler.compile_statements(&program.body)?;
        compiler.builder.emit_halt();
        Ok(Rc::new(compiler.builder.finish()))
    }

    /// Compile a program to bytecode with source file path for stack traces
    pub fn compile_program_with_source(
        program: &Program,
        source_file: String,
    ) -> Result<Rc<BytecodeChunk>, JsError> {
        let mut compiler = Compiler::with_source_file(source_file);

        // First, hoist all var declarations and function declarations to the top
        compiler.emit_hoisted_declarations(&program.body)?;

        // Then compile the statements
        compiler.compile_statements(&program.body)?;
        compiler.builder.emit_halt();
        Ok(Rc::new(compiler.builder.finish()))
    }

    /// Compile a program for eval with completion value tracking
    /// Register 0 will contain the completion value when Halt is reached.
    pub fn compile_program_for_eval(program: &Program) -> Result<Rc<BytecodeChunk>, JsError> {
        let mut compiler = Compiler::new_with_completion_tracking();

        // First, hoist all var declarations and function declarations to the top
        compiler.emit_hoisted_declarations(&program.body)?;

        // Then compile the statements
        compiler.compile_statements(&program.body)?;
        compiler.builder.emit_halt();
        Ok(Rc::new(compiler.builder.finish()))
    }

    /// Compile a single statement to bytecode
    pub fn compile_statement(stmt: &crate::ast::Statement) -> Result<Rc<BytecodeChunk>, JsError> {
        let mut compiler = Compiler::new();
        compiler.compile_statement_impl(stmt)?;
        compiler.builder.emit_halt();
        Ok(Rc::new(compiler.builder.finish()))
    }

    /// Compile a function body directly (for JIT compilation of interpreted functions)
    /// This is a static entry point that compiles a function body to bytecode,
    /// handling parameter binding and function info metadata.
    pub fn compile_function_body_direct(
        params: &Rc<[crate::ast::FunctionParam]>,
        body: &[crate::ast::Statement],
        name: Option<JsString>,
        is_generator: bool,
        is_async: bool,
    ) -> Result<BytecodeChunk, JsError> {
        let mut compiler = Compiler::new();
        let chunk =
            compiler.compile_function_body(params, body, name, is_generator, is_async, false)?;
        Ok(chunk)
    }

    /// Compile a sequence of statements
    fn compile_statements(&mut self, statements: &[crate::ast::Statement]) -> Result<(), JsError> {
        for stmt in statements {
            self.compile_statement_impl(stmt)?;
        }
        Ok(())
    }

    /// Get the register allocator
    #[allow(dead_code)]
    fn registers(&mut self) -> &mut RegisterAllocator {
        self.builder.registers()
    }

    /// Push a loop context
    fn push_loop(&mut self, label: Option<JsString>) {
        self.push_loop_with_iterator(label, None);
    }

    /// Push a loop context with an iterator register (for for-of loops)
    fn push_loop_with_iterator(&mut self, label: Option<JsString>, iterator_reg: Option<Register>) {
        let index = self.loop_stack.len();
        if let Some(ref l) = label {
            self.labels.insert(l.cheap_clone(), index);
        }
        self.loop_stack.push(LoopContext {
            label,
            break_jumps: Vec::new(),
            continue_target: None,
            continue_jumps: Vec::new(),
            try_depth: self.try_depth,
            iterator_reg,
        });
    }

    /// Set the continue target for the current loop and patch any pending continue jumps
    /// Also propagates the continue target to parent labeled contexts that don't have one yet
    fn set_continue_target(&mut self, target: usize) {
        // Collect indices of contexts that need to have continue target set
        // This includes the current loop and any parent labeled contexts without a continue target
        let len = self.loop_stack.len();
        if len == 0 {
            return;
        }

        // Collect pending continue jumps from all contexts that will share this target
        let mut all_pending_jumps: Vec<JumpPlaceholder> = Vec::new();

        // Start from the current (innermost) context and work backwards
        // Set continue target for the current loop
        if let Some(ctx) = self.loop_stack.get_mut(len - 1) {
            ctx.continue_target = Some(target);
            all_pending_jumps.append(&mut ctx.continue_jumps);
        }

        // Propagate to parent labeled contexts that don't have a continue target yet
        // A labeled context directly wrapping a loop shares the loop's continue target
        for i in (0..len - 1).rev() {
            if let Some(ctx) = self.loop_stack.get_mut(i) {
                // Only propagate if this is a labeled context and it doesn't have a continue target
                if ctx.label.is_some() && ctx.continue_target.is_none() {
                    ctx.continue_target = Some(target);
                    all_pending_jumps.append(&mut ctx.continue_jumps);
                } else {
                    // Stop propagating if we hit a context that's not a label wrapper
                    break;
                }
            }
        }

        // Patch all pending continue jumps
        for jump in all_pending_jumps {
            self.builder.patch_jump_to(jump, target as JumpTarget);
        }
    }

    /// Pop a loop context and patch break jumps
    fn pop_loop(&mut self) -> Option<LoopContext> {
        let ctx = self.loop_stack.pop()?;
        if let Some(ref label) = ctx.label {
            self.labels.remove(label);
        }
        // Patch all break jumps to current position
        for jump in &ctx.break_jumps {
            self.builder.patch_jump(*jump);
        }
        Some(ctx)
    }

    /// Set loop variable redirects for for-loop update expressions.
    /// When these are set, assignments to the specified variables will write
    /// to registers instead of the environment, preserving closure semantics.
    fn set_loop_var_redirects(&mut self, redirects: Vec<(JsString, Register)>) {
        self.loop_var_redirects = redirects.into_iter().collect();
    }

    /// Clear loop variable redirects
    fn clear_loop_var_redirects(&mut self) {
        self.loop_var_redirects.clear();
    }

    /// Check if a variable has a loop redirect, returning the register if so
    fn get_loop_var_redirect(&self, name: &JsString) -> Option<Register> {
        self.loop_var_redirects.get(name).copied()
    }

    /// Add a break jump for the specified label (or innermost loop if None)
    fn add_break_jump(&mut self, label: Option<&JsString>) -> Result<JumpPlaceholder, JsError> {
        let loop_idx = if let Some(l) = label {
            self.labels.get(l).copied().ok_or_else(|| {
                JsError::syntax_error_simple(format!(
                    "Illegal break statement: undefined label '{}'",
                    l
                ))
            })?
        } else {
            self.loop_stack
                .len()
                .checked_sub(1)
                .ok_or_else(|| JsError::syntax_error_simple("Illegal break statement"))?
        };

        // Get the target loop's try_depth
        let target_try_depth = self
            .loop_stack
            .get(loop_idx)
            .map(|ctx| ctx.try_depth as u8)
            .unwrap_or(0);

        // Emit IteratorClose before break if this is a for-of loop
        // Also need to close iterators for any enclosing for-of loops we're breaking out of
        for i in (loop_idx..self.loop_stack.len()).rev() {
            if let Some(iter_reg) = self.loop_stack.get(i).and_then(|ctx| ctx.iterator_reg) {
                self.builder.emit(Op::IteratorClose { iterator: iter_reg });
            }
        }

        // Emit Break opcode with placeholder target
        let idx = self.builder.emit(Op::Break {
            target: 0,
            try_depth: target_try_depth,
        });
        let jump = JumpPlaceholder {
            instruction_index: idx,
        };

        if let Some(ctx) = self.loop_stack.get_mut(loop_idx) {
            ctx.break_jumps.push(jump);
        }

        Ok(jump)
    }

    /// Add a continue jump for the specified label (or innermost loop if None)
    fn add_continue_jump(&mut self, label: Option<&JsString>) -> Result<(), JsError> {
        let loop_idx = if let Some(l) = label {
            self.labels.get(l).copied().ok_or_else(|| {
                JsError::syntax_error_simple(format!(
                    "Illegal continue statement: undefined label '{}'",
                    l
                ))
            })?
        } else {
            self.loop_stack
                .len()
                .checked_sub(1)
                .ok_or_else(|| JsError::syntax_error_simple("Illegal continue statement"))?
        };

        // Get the target loop's try_depth
        let target_try_depth = self
            .loop_stack
            .get(loop_idx)
            .map(|ctx| ctx.try_depth)
            .unwrap_or(0) as u8;

        if let Some(ctx) = self.loop_stack.get_mut(loop_idx) {
            if let Some(target) = ctx.continue_target {
                // Target is known, emit Continue with known target
                self.builder.emit(Op::Continue {
                    target: target as u32,
                    try_depth: target_try_depth,
                });
            } else {
                // Target not yet known, save placeholder
                let idx = self.builder.emit(Op::Continue {
                    target: 0,
                    try_depth: target_try_depth,
                });
                let jump = JumpPlaceholder {
                    instruction_index: idx,
                };
                ctx.continue_jumps.push(jump);
            }
        }

        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

// Import CheapClone for JsString
use crate::value::CheapClone;
