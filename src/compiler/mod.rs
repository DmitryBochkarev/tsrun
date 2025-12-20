//! Bytecode compiler for TypeScript/JavaScript
//!
//! This module compiles the AST to bytecode for execution by the bytecode VM.
//! The bytecode uses a register-based design for better performance.

mod builder;
mod bytecode;
mod compile_expr;
mod compile_pattern;
mod compile_stmt;

pub use builder::{BytecodeBuilder, JumpPlaceholder};
pub use bytecode::{BytecodeChunk, Constant, FunctionInfo, JumpTarget, Op, Register};

use crate::ast::Program;
use crate::error::JsError;
use crate::value::JsString;
use builder::RegisterAllocator;
use rustc_hash::FxHashMap;
use std::rc::Rc;

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
}

impl Compiler {
    /// Create a new compiler
    pub fn new() -> Self {
        Self {
            builder: BytecodeBuilder::new(),
            loop_stack: Vec::new(),
            labels: FxHashMap::default(),
            try_depth: 0,
        }
    }

    /// Compile a program to bytecode
    pub fn compile_program(program: &Program) -> Result<Rc<BytecodeChunk>, JsError> {
        let mut compiler = Compiler::new();
        compiler.compile_statements(&program.body)?;
        compiler.builder.emit_halt();
        Ok(Rc::new(compiler.builder.finish()))
    }

    /// Compile a sequence of statements
    fn compile_statements(&mut self, statements: &[crate::ast::Statement]) -> Result<(), JsError> {
        for stmt in statements {
            self.compile_statement(stmt)?;
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
        let index = self.loop_stack.len();
        if let Some(ref l) = label {
            self.labels.insert(l.cheap_clone(), index);
        }
        self.loop_stack.push(LoopContext {
            label,
            break_jumps: Vec::new(),
            continue_target: None,
            continue_jumps: Vec::new(),
        });
    }

    /// Set the continue target for the current loop and patch any pending continue jumps
    fn set_continue_target(&mut self, target: usize) {
        if let Some(ctx) = self.loop_stack.last_mut() {
            ctx.continue_target = Some(target);
            // Patch any pending continue jumps that were emitted before the target was known
            for jump in ctx.continue_jumps.drain(..) {
                self.builder.patch_jump_to(jump, target as JumpTarget);
            }
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

    /// Add a break jump for the specified label (or innermost loop if None)
    fn add_break_jump(&mut self, label: Option<&JsString>) -> Result<JumpPlaceholder, JsError> {
        let jump = self.builder.emit_jump();

        let loop_idx = if let Some(l) = label {
            self.labels
                .get(l)
                .copied()
                .ok_or_else(|| JsError::syntax_error_simple(format!("Undefined label '{}'", l)))?
        } else {
            self.loop_stack.len().checked_sub(1).ok_or_else(|| {
                JsError::syntax_error_simple("'break' statement must be inside a loop or switch")
            })?
        };

        if let Some(ctx) = self.loop_stack.get_mut(loop_idx) {
            ctx.break_jumps.push(jump);
        }

        Ok(jump)
    }

    /// Add a continue jump for the specified label (or innermost loop if None)
    fn add_continue_jump(&mut self, label: Option<&JsString>) -> Result<(), JsError> {
        let loop_idx = if let Some(l) = label {
            self.labels
                .get(l)
                .copied()
                .ok_or_else(|| JsError::syntax_error_simple(format!("Undefined label '{}'", l)))?
        } else {
            self.loop_stack.len().checked_sub(1).ok_or_else(|| {
                JsError::syntax_error_simple("'continue' statement must be inside a loop")
            })?
        };

        if let Some(ctx) = self.loop_stack.get_mut(loop_idx) {
            if let Some(target) = ctx.continue_target {
                // Target is known, emit direct jump
                self.builder.emit_jump_to(target);
            } else {
                // Target not yet known, save placeholder
                let jump = self.builder.emit_jump();
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
