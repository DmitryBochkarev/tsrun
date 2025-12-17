//! Stack-based evaluation for suspendable execution
//!
//! This module implements a trampolined interpreter that can suspend
//! at await points and resume later with a value.

// Allow many arguments for internal step_for_* functions - these handle loop state
// and grouping into a struct would add complexity without benefit
#![allow(clippy::too_many_arguments)]

use crate::ast::{
    BinaryOp, Expression, ForInOfLeft, ForInStatement, ForInit, ForOfStatement, ForStatement,
    LiteralValue, LogicalOp, Pattern, Program, Statement, UnaryOp, VariableDeclarator,
    VariableKind,
};
use crate::error::JsError;
use crate::gc::Gc;
use crate::value::{
    Binding, CheapClone, ExoticObject, Guarded, JsObject, JsString, JsValue, PromiseStatus,
};
use std::rc::Rc;

use super::{create_environment_unrooted, Interpreter};

// ═══════════════════════════════════════════════════════════════════════════════
// Stack Types
// ═══════════════════════════════════════════════════════════════════════════════

/// A value on the value stack, wrapped in Guarded to maintain GC safety
pub type StackValue = Guarded;

/// Boxed label for loop frames - saves 8 bytes per frame since labels are rare
pub type LoopLabel = Option<Box<JsString>>;

/// Convert LoopLabel to Option<&JsString> for comparison
#[inline]
fn label_ref(label: &LoopLabel) -> Option<&JsString> {
    label.as_ref().map(|b| b.as_ref())
}

/// Result of executing one step
pub enum StepResult {
    /// Continue execution (more frames to process)
    Continue,
    /// Execution complete with final value
    Done(Guarded),
    /// Suspended waiting for a promise to resolve
    /// Contains the promise object that we're waiting on
    Suspend(Gc<JsObject>),
    /// Error occurred
    Error(JsError),
}

/// Completion type for control flow
#[derive(Debug, Clone)]
pub enum StackCompletion {
    Normal,
    Return,
    Break(Option<JsString>),
    Continue(Option<JsString>),
    Throw,
}

/// Data for FinallyBlock frame, boxed to reduce Frame enum size
pub struct FinallyBlockData {
    pub saved_result: Option<JsValue>,
    pub saved_error: Option<JsError>,
    pub saved_completion: StackCompletion,
}

/// Data for ForOfGenerator frames, boxed to reduce Frame enum size
/// (contains two Gc<JsObject> fields = 48 bytes)
pub struct ForOfGeneratorData {
    pub generator: Gc<JsObject>,
    pub gen_state: Rc<std::cell::RefCell<crate::value::GeneratorState>>,
    pub left: Rc<ForInOfLeft>,
    pub body: Rc<Statement>,
    pub label: LoopLabel,
    pub saved_env: Gc<JsObject>,
}

/// A frame on the evaluation stack
///
/// Each frame represents a pending operation. The interpreter processes
/// frames in LIFO order, pushing new frames when sub-expressions need
/// to be evaluated.
pub enum Frame {
    // ═══════════════════════════════════════════════════════════════════════
    // Program/Statement Execution
    // ═══════════════════════════════════════════════════════════════════════
    /// Execute program statements sequentially
    Program {
        statements: Rc<[Statement]>,
        index: usize,
    },

    /// Execute a single statement
    Stmt(Rc<Statement>),

    /// Statement completed, check completion type
    StmtComplete,

    /// Execute remaining statements in block
    Block {
        statements: Rc<[Statement]>,
        index: usize,
    },

    /// Expression statement: keep result on stack
    ExprStmtComplete,

    // ═══════════════════════════════════════════════════════════════════════
    // Expression Evaluation
    // ═══════════════════════════════════════════════════════════════════════
    /// Evaluate an expression
    Expr(Rc<Expression>),

    /// Binary: left done, evaluate right then complete
    BinaryRight { op: BinaryOp, right: Rc<Expression> },

    /// Binary: both done, compute result
    BinaryComplete { op: BinaryOp },

    /// Logical: left done, maybe short-circuit
    LogicalCheck {
        op: LogicalOp,
        right: Rc<Expression>,
    },

    /// Unary: operand done, apply operator
    UnaryComplete { op: UnaryOp },

    /// Conditional: condition done, pick branch
    ConditionalBranch {
        consequent: Rc<Expression>,
        alternate: Rc<Expression>,
    },

    /// Await: promise evaluated, check state
    AwaitCheck,

    // ═══════════════════════════════════════════════════════════════════════
    // Variable Declaration
    // ═══════════════════════════════════════════════════════════════════════
    /// Process variable declarators sequentially
    VarDecl {
        declarators: Rc<[VariableDeclarator]>,
        index: usize,
        mutable: bool,
    },

    /// Bind variable after init expression evaluated
    VarBind { pattern: Rc<Pattern>, mutable: bool },

    // ═══════════════════════════════════════════════════════════════════════
    // Control Flow
    // ═══════════════════════════════════════════════════════════════════════
    /// If statement: condition evaluated, pick branch
    IfBranch {
        consequent: Rc<Statement>,
        alternate: Option<Rc<Statement>>,
    },

    /// While loop: evaluate test, then decide
    WhileLoop {
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: LoopLabel,
    },

    /// While check: test evaluated, execute body or exit
    WhileCheck {
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: LoopLabel,
    },

    /// While after body: body executed, check completion before evaluating test
    WhileAfterBody {
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: LoopLabel,
    },

    /// Do-while loop: execute body first, then check condition
    DoWhileLoop {
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: LoopLabel,
    },

    /// Do-while check: body executed, check completion before evaluating test
    DoWhileCheck {
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: LoopLabel,
    },

    /// Do-while test check: after condition evaluated, decide to loop or exit
    DoWhileTestCheck {
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: LoopLabel,
    },

    /// For loop: full state for iteration
    ForLoop {
        test: Option<Rc<Expression>>,
        update: Option<Rc<Expression>>,
        body: Rc<Statement>,
        label: LoopLabel,
        /// Variables for per-iteration binding (name, mutable)
        loop_vars: Rc<Vec<(JsString, bool)>>,
        /// Saved environment to restore after loop
        saved_env: Gc<JsObject>,
        /// Whether this is the first iteration (need to create per-iteration env)
        first_iteration: bool,
    },

    /// For loop test check: condition evaluated, decide to continue or exit
    ForTestCheck {
        test: Option<Rc<Expression>>,
        update: Option<Rc<Expression>>,
        body: Rc<Statement>,
        label: LoopLabel,
        loop_vars: Rc<Vec<(JsString, bool)>>,
        saved_env: Gc<JsObject>,
    },

    /// For loop after body: handle control flow, create per-iteration env, run update
    ForAfterBody {
        test: Option<Rc<Expression>>,
        update: Option<Rc<Expression>>,
        body: Rc<Statement>,
        label: LoopLabel,
        loop_vars: Rc<Vec<(JsString, bool)>>,
        saved_env: Gc<JsObject>,
    },

    /// For loop cleanup: restore environment after loop exits
    ForCleanup { saved_env: Gc<JsObject> },

    /// For-in loop: iterate over object keys
    ForInLoop {
        /// Keys to iterate over
        keys: Rc<Vec<String>>,
        /// Current index in keys
        index: usize,
        /// Left-hand side binding
        left: Rc<ForInOfLeft>,
        /// Loop body
        body: Rc<Statement>,
        /// Optional label
        label: LoopLabel,
        /// Saved environment to restore after loop
        saved_env: Gc<JsObject>,
    },

    /// For-in iteration: after body, proceed to next key
    ForInAfterBody {
        keys: Rc<Vec<String>>,
        index: usize,
        left: Rc<ForInOfLeft>,
        body: Rc<Statement>,
        label: LoopLabel,
        saved_env: Gc<JsObject>,
    },

    /// For-of loop: iterate over iterable values
    ForOfLoop {
        /// Items to iterate over
        items: Rc<Vec<JsValue>>,
        /// Current index in items
        index: usize,
        /// Left-hand side binding
        left: Rc<ForInOfLeft>,
        /// Loop body
        body: Rc<Statement>,
        /// Optional label
        label: LoopLabel,
        /// Saved environment to restore after loop
        saved_env: Gc<JsObject>,
    },

    /// For-of iteration: after body, proceed to next item
    ForOfAfterBody {
        items: Rc<Vec<JsValue>>,
        index: usize,
        left: Rc<ForInOfLeft>,
        body: Rc<Statement>,
        label: LoopLabel,
        saved_env: Gc<JsObject>,
    },

    /// For-of loop over a generator: call next() each iteration
    /// Boxed to reduce Frame enum size (contains two Gc<JsObject> = 48 bytes)
    ForOfGenerator(Box<ForOfGeneratorData>),

    /// For-of generator: after body, call next() again
    /// Boxed to reduce Frame enum size
    ForOfGeneratorAfterBody(Box<ForOfGeneratorData>),

    /// Discard expression result (for init expressions, update expressions)
    DiscardValue,

    /// Push scope before for loop body
    PushScope {
        /// Continuation frame to push after scope is created
        next: Box<Frame>,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // Throw Statement
    // ═══════════════════════════════════════════════════════════════════════
    /// Throw: expression evaluated, now throw it
    ThrowComplete,

    // ═══════════════════════════════════════════════════════════════════════
    // Switch Statement
    // ═══════════════════════════════════════════════════════════════════════
    /// Switch statement: evaluate discriminant
    SwitchEval { cases: Rc<[crate::ast::SwitchCase]> },

    /// Switch: discriminant evaluated, match cases
    SwitchMatch {
        discriminant: JsValue,
        cases: Rc<[crate::ast::SwitchCase]>,
        index: usize,
        found_match: bool,
    },

    /// Switch: execute case body
    SwitchBody {
        discriminant: JsValue,
        cases: Rc<[crate::ast::SwitchCase]>,
        case_index: usize,
        stmt_index: usize,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // Try/Catch/Finally Statement
    // ═══════════════════════════════════════════════════════════════════════
    /// Try block: mark where to catch errors
    TryBlock {
        handler: Option<Rc<crate::ast::CatchClause>>,
        finalizer: Option<Rc<crate::ast::BlockStatement>>,
        body: Rc<crate::ast::BlockStatement>,
    },

    /// Catch block: execute catch handler
    CatchBlock {
        finalizer: Option<Rc<crate::ast::BlockStatement>>,
        saved_env: Gc<JsObject>,
    },

    /// Finally block: execute finally regardless of outcome
    /// Boxed to reduce Frame enum size (this variant would otherwise be 128 bytes)
    FinallyBlock(Box<FinallyBlockData>),

    // ═══════════════════════════════════════════════════════════════════════
    // Labeled Statement
    // ═══════════════════════════════════════════════════════════════════════
    /// Labeled statement wrapper
    LabeledStmt {
        label: JsString,
        body: Rc<Statement>,
    },

    /// After labeled body executed
    LabeledComplete { label: JsString },
}

// ═══════════════════════════════════════════════════════════════════════════════
// Execution State
// ═══════════════════════════════════════════════════════════════════════════════

/// The execution state for stack-based evaluation
pub struct ExecutionState {
    /// Frame stack (pending operations)
    pub frames: Vec<Frame>,

    /// Value stack (intermediate results)
    pub values: Vec<Guarded>,

    /// Current completion type
    pub completion: StackCompletion,

    /// Promise we're waiting on (when suspended)
    pub waiting_on: Option<Gc<JsObject>>,
}

/// Initial capacity for the value stack.
/// This is sized to avoid reallocations during typical expression evaluation.
/// Complex expressions may exceed this, but loop iterations typically stay under 256.
const VALUE_STACK_INITIAL_CAPACITY: usize = 256;

/// Initial capacity for the frame stack.
/// Frames accumulate for nested expressions, blocks, and control flow.
/// Typical deep nesting rarely exceeds 128 frames.
const FRAME_STACK_INITIAL_CAPACITY: usize = 128;

impl ExecutionState {
    pub fn new() -> Self {
        Self {
            frames: Vec::with_capacity(FRAME_STACK_INITIAL_CAPACITY),
            values: Vec::with_capacity(VALUE_STACK_INITIAL_CAPACITY),
            completion: StackCompletion::Normal,
            waiting_on: None,
        }
    }

    /// Reset state for reuse, keeping allocated capacity
    pub fn reset(&mut self) {
        self.frames.clear();
        self.values.clear();
        self.completion = StackCompletion::Normal;
        self.waiting_on = None;
    }

    /// Create state for executing a program
    pub fn for_program(program: &Program) -> Self {
        let mut state = Self::new();
        state.init_for_program(program);
        state
    }

    /// Initialize this state to execute a program (for pool reuse)
    pub fn init_for_program(&mut self, program: &Program) {
        self.push_frame(Frame::Program {
            statements: Rc::clone(&program.body),
            index: 0,
        });
    }

    /// Create state for executing a single statement
    pub fn for_statement(stmt: &Statement) -> Self {
        let mut state = Self::new();
        state.init_for_statement(stmt);
        state
    }

    /// Initialize this state to execute a statement (for pool reuse)
    pub fn init_for_statement(&mut self, stmt: &Statement) {
        self.push_frame(Frame::Stmt(Rc::new(stmt.clone())));
    }

    /// Push a frame
    pub fn push_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }

    /// Pop a frame
    pub fn pop_frame(&mut self) -> Option<Frame> {
        self.frames.pop()
    }

    /// Push a value
    pub fn push_value(&mut self, value: Guarded) {
        self.values.push(value);
    }

    /// Pop a value
    pub fn pop_value(&mut self) -> Option<Guarded> {
        self.values.pop()
    }
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Stack Execution Implementation
// ═══════════════════════════════════════════════════════════════════════════════

impl Interpreter {
    /// Execute a program using stack-based evaluation
    pub fn eval_with_stack(&mut self, source: &str) -> Result<JsValue, JsError> {
        use crate::parser::Parser;

        let mut parser = Parser::new(source, &mut self.string_dict);
        let program = parser.parse_program()?;

        // Start execution timer
        self.start_execution();

        let mut state = ExecutionState::for_program(&program);

        match self.run(&mut state) {
            StepResult::Done(g) => Ok(g.value),
            StepResult::Error(e) => Err(e),
            StepResult::Suspend(promise) => {
                // For simple eval, suspension means we have pending orders
                // Return undefined for now - full async support comes later
                let _ = promise;
                Ok(JsValue::Undefined)
            }
            StepResult::Continue => {
                // Should never happen after run()
                Ok(JsValue::Undefined)
            }
        }
    }

    /// Execute a program using stack-based evaluation (for module execution)
    ///
    /// This is used internally for executing module code. It runs to completion
    /// without supporting suspension (modules are expected to be synchronous).
    pub fn execute_program_with_stack(
        &mut self,
        program: &crate::ast::Program,
    ) -> Result<JsValue, JsError> {
        // Start execution timer
        self.start_execution();

        let mut state = ExecutionState::for_program(program);

        match self.run(&mut state) {
            StepResult::Done(g) => Ok(g.value),
            StepResult::Error(e) => Err(e),
            StepResult::Suspend(_) => {
                // Modules shouldn't suspend - if they do, treat as error
                Err(JsError::type_error("Module execution cannot be suspended"))
            }
            StepResult::Continue => {
                // Should never happen after run()
                Ok(JsValue::Undefined)
            }
        }
    }

    /// Execute one step of the stack machine
    pub fn step(&mut self, state: &mut ExecutionState) -> StepResult {
        // Check timeout
        if let Err(e) = self.check_timeout() {
            return StepResult::Error(e);
        }

        let Some(frame) = state.pop_frame() else {
            // No more frames - return the top value
            return match state.pop_value() {
                Some(v) => StepResult::Done(v),
                None => StepResult::Done(Guarded::unguarded(JsValue::Undefined)),
            };
        };

        match frame {
            // ═══════════════════════════════════════════════════════════════
            // Program/Statement Execution
            // ═══════════════════════════════════════════════════════════════
            Frame::Program { statements, index } => self.step_program(state, statements, index),

            Frame::Stmt(stmt) => self.step_stmt(state, &stmt),

            Frame::StmtComplete => {
                // Statement done, value on stack represents the result
                // Check for non-normal completion
                match &state.completion {
                    StackCompletion::Normal => StepResult::Continue,
                    StackCompletion::Return => {
                        // Unwind to function boundary
                        StepResult::Continue
                    }
                    StackCompletion::Break(_) | StackCompletion::Continue(_) => {
                        // Loop control - handled by loop frames
                        StepResult::Continue
                    }
                    StackCompletion::Throw => {
                        // Error - propagate up
                        let value = state
                            .pop_value()
                            .map(|g| g.value)
                            .unwrap_or(JsValue::Undefined);
                        StepResult::Error(JsError::thrown(value))
                    }
                }
            }

            Frame::Block { statements, index } => self.step_block(state, statements, index),

            Frame::ExprStmtComplete => {
                // Keep expression value on stack for program/block result
                StepResult::Continue
            }

            // ═══════════════════════════════════════════════════════════════
            // Expression Evaluation
            // ═══════════════════════════════════════════════════════════════
            Frame::Expr(expr) => self.step_expr(state, &expr),

            Frame::BinaryRight { op, right } => {
                // Left value is on stack, evaluate right
                state.push_frame(Frame::BinaryComplete { op });
                state.push_frame(Frame::Expr(right));
                StepResult::Continue
            }

            Frame::BinaryComplete { op } => self.step_binary_complete(state, op),

            Frame::LogicalCheck { op, right } => self.step_logical_check(state, op, right),

            Frame::UnaryComplete { op } => self.step_unary_complete(state, op),

            Frame::ConditionalBranch {
                consequent,
                alternate,
            } => {
                let cond = state
                    .pop_value()
                    .map(|v| v.value)
                    .unwrap_or(JsValue::Undefined);
                if cond.to_boolean() {
                    state.push_frame(Frame::Expr(consequent));
                } else {
                    state.push_frame(Frame::Expr(alternate));
                }
                StepResult::Continue
            }

            Frame::AwaitCheck => self.step_await_check(state),

            // ═══════════════════════════════════════════════════════════════
            // Variable Declaration
            // ═══════════════════════════════════════════════════════════════
            Frame::VarDecl {
                declarators,
                index,
                mutable,
            } => self.step_var_decl(state, declarators, index, mutable),

            Frame::VarBind { pattern, mutable } => self.step_var_bind(state, &pattern, mutable),

            // ═══════════════════════════════════════════════════════════════
            // Control Flow
            // ═══════════════════════════════════════════════════════════════
            Frame::IfBranch {
                consequent,
                alternate,
            } => self.step_if_branch(state, consequent, alternate),

            Frame::WhileLoop { test, body, label } => {
                // Start while loop - evaluate test first
                state.push_frame(Frame::WhileCheck {
                    test: test.clone(),
                    body,
                    label,
                });
                state.push_frame(Frame::Expr(test));
                StepResult::Continue
            }

            Frame::WhileCheck { test, body, label } => {
                self.step_while_check(state, test, body, label)
            }

            Frame::WhileAfterBody { test, body, label } => {
                self.step_while_after_body(state, test, body, label)
            }

            Frame::DoWhileLoop { test, body, label } => {
                // Execute body first, then check condition
                state.push_frame(Frame::DoWhileCheck {
                    test: test.clone(),
                    body: body.clone(),
                    label,
                });
                state.push_frame(Frame::Stmt(body));
                StepResult::Continue
            }

            Frame::DoWhileCheck { test, body, label } => {
                self.step_do_while_check(state, test, body, label)
            }

            Frame::DoWhileTestCheck { test, body, label } => {
                self.step_do_while_test_check(state, test, body, label)
            }

            Frame::ForLoop {
                test,
                update,
                body,
                label,
                loop_vars,
                saved_env,
                first_iteration,
            } => self.step_for_loop(
                state,
                test,
                update,
                body,
                label,
                loop_vars,
                saved_env,
                first_iteration,
            ),

            Frame::ForTestCheck {
                test,
                update,
                body,
                label,
                loop_vars,
                saved_env,
            } => self.step_for_test_check(state, test, update, body, label, loop_vars, saved_env),

            Frame::ForAfterBody {
                test,
                update,
                body,
                label,
                loop_vars,
                saved_env,
            } => self.step_for_after_body(state, test, update, body, label, loop_vars, saved_env),

            Frame::ForCleanup { saved_env } => {
                self.pop_scope(saved_env);
                // Only push undefined if not returning/throwing - preserve the return value
                if !matches!(
                    state.completion,
                    StackCompletion::Return | StackCompletion::Throw
                ) {
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                }
                StepResult::Continue
            }

            Frame::ForInLoop {
                keys,
                index,
                left,
                body,
                label,
                saved_env,
            } => self.step_for_in_loop(state, keys, index, left, body, label, saved_env),

            Frame::ForInAfterBody {
                keys,
                index,
                left,
                body,
                label,
                saved_env,
            } => self.step_for_in_after_body(state, keys, index, left, body, label, saved_env),

            Frame::ForOfLoop {
                items,
                index,
                left,
                body,
                label,
                saved_env,
            } => self.step_for_of_loop(state, items, index, left, body, label, saved_env),

            Frame::ForOfAfterBody {
                items,
                index,
                left,
                body,
                label,
                saved_env,
            } => self.step_for_of_after_body(state, items, index, left, body, label, saved_env),

            Frame::ForOfGenerator(data) => self.step_for_of_generator(
                state,
                data.generator,
                data.gen_state,
                data.left,
                data.body,
                data.label,
                data.saved_env,
            ),

            Frame::ForOfGeneratorAfterBody(data) => self.step_for_of_generator_after_body(
                state,
                data.generator,
                data.gen_state,
                data.left,
                data.body,
                data.label,
                data.saved_env,
            ),

            Frame::DiscardValue => {
                // Pop and discard the value
                let _ = state.pop_value();
                StepResult::Continue
            }

            Frame::PushScope { next } => {
                let _saved = self.push_scope();
                state.push_frame(*next);
                StepResult::Continue
            }

            // ═══════════════════════════════════════════════════════════════
            // Throw Statement
            // ═══════════════════════════════════════════════════════════════
            Frame::ThrowComplete => {
                let guarded = state
                    .pop_value()
                    .unwrap_or_else(|| Guarded::unguarded(JsValue::Undefined));
                // Store the guard to keep the thrown value alive during exception handling
                self.thrown_guard = guarded.guard;
                StepResult::Error(JsError::thrown(guarded.value))
            }

            // ═══════════════════════════════════════════════════════════════
            // Switch Statement
            // ═══════════════════════════════════════════════════════════════
            Frame::SwitchEval { cases } => {
                let discriminant = state
                    .pop_value()
                    .map(|g| g.value)
                    .unwrap_or(JsValue::Undefined);
                state.push_frame(Frame::SwitchMatch {
                    discriminant,
                    cases,
                    index: 0,
                    found_match: false,
                });
                StepResult::Continue
            }

            Frame::SwitchMatch {
                discriminant,
                cases,
                index,
                found_match,
            } => self.step_switch_match(state, discriminant, cases, index, found_match),

            Frame::SwitchBody {
                discriminant,
                cases,
                case_index,
                stmt_index,
            } => self.step_switch_body(state, discriminant, cases, case_index, stmt_index),

            // ═══════════════════════════════════════════════════════════════
            // Try/Catch/Finally Statement
            // ═══════════════════════════════════════════════════════════════
            Frame::TryBlock {
                handler,
                finalizer,
                body,
            } => self.step_try_block(state, handler, finalizer, body),

            Frame::CatchBlock {
                finalizer,
                saved_env,
            } => self.step_catch_block(state, finalizer, saved_env),

            Frame::FinallyBlock(data) => {
                self.step_finally_block(state, data.saved_result, data.saved_error, data.saved_completion)
            }

            // ═══════════════════════════════════════════════════════════════
            // Labeled Statement
            // ═══════════════════════════════════════════════════════════════
            Frame::LabeledStmt { label, body } => {
                state.push_frame(Frame::LabeledComplete {
                    label: label.cheap_clone(),
                });
                state.push_frame(Frame::Stmt(body));
                StepResult::Continue
            }

            Frame::LabeledComplete { label } => {
                // Check if we got a break for this label
                if let StackCompletion::Break(Some(ref break_label)) = state.completion {
                    if break_label == &label {
                        state.completion = StackCompletion::Normal;
                    }
                }
                StepResult::Continue
            }
        }
    }

    /// Run until completion or suspension
    pub fn run(&mut self, state: &mut ExecutionState) -> StepResult {
        loop {
            match self.step(state) {
                StepResult::Continue => continue,
                StepResult::Error(error) => {
                    // Try to find a TryBlock frame to catch the error
                    if let Some(result) = self.handle_error(state, error) {
                        return result;
                    }
                    // Error was handled, continue execution
                    continue;
                }
                result => return result,
            }
        }
    }

    /// Handle an error by unwinding the stack to find a TryBlock
    /// Returns Some(StepResult) if error should propagate, None if handled
    pub fn handle_error(
        &mut self,
        state: &mut ExecutionState,
        error: JsError,
    ) -> Option<StepResult> {
        // Extract error value for catch
        let error_value = match &error {
            JsError::Thrown => self.thrown_value.take().unwrap_or(JsValue::Undefined),
            JsError::ThrownValue { value } => value.clone(),
            _ => JsValue::from(error.to_string()),
        };

        // Search for TryBlock frame
        let mut found_try_idx = None;
        for (idx, frame) in state.frames.iter().enumerate().rev() {
            if matches!(frame, Frame::TryBlock { .. }) {
                found_try_idx = Some(idx);
                break;
            }
        }

        if let Some(idx) = found_try_idx {
            // Remove all frames above the TryBlock (they're being unwound)
            state.frames.truncate(idx + 1);

            // Pop the TryBlock frame to process it
            if let Some(Frame::TryBlock {
                handler,
                finalizer,
                body: _,
            }) = state.pop_frame()
            {
                // Clear value stack (exception unwinds computation)
                state.values.clear();

                // Reset completion - error was caught, so any previous return/break/continue is cancelled
                state.completion = StackCompletion::Normal;

                if let Some(catch_handler) = handler {
                    // Create catch scope with guard
                    let saved_env = self.env.clone();
                    let (catch_env, catch_guard) =
                        create_environment_unrooted(&self.heap, Some(saved_env.clone()));
                    self.env = catch_env;
                    self.push_env_guard(catch_guard);

                    // Bind error parameter if present
                    if let Some(ref param) = catch_handler.param {
                        if let Err(e) = self.bind_pattern(param, error_value, true) {
                            self.pop_env_guard();
                            self.env = saved_env;
                            return Some(StepResult::Error(e));
                        }
                    }

                    // Clear thrown guard - error is now owned by catch environment
                    self.thrown_guard = None;

                    // Push catch block execution
                    state.push_frame(Frame::CatchBlock {
                        finalizer,
                        saved_env,
                    });
                    state.push_frame(Frame::Block {
                        statements: Rc::clone(&catch_handler.body.body),
                        index: 0,
                    });

                    None // Error was handled
                } else if let Some(finally_block) = finalizer {
                    // No catch, but there's finally - run finally then re-throw
                    state.push_frame(Frame::FinallyBlock(Box::new(FinallyBlockData {
                        saved_result: None,
                        saved_error: Some(JsError::thrown(error_value)),
                        saved_completion: StackCompletion::Normal,
                    })));
                    state.push_frame(Frame::Block {
                        statements: Rc::clone(&finally_block.body),
                        index: 0,
                    });

                    None // Error will be re-thrown after finally
                } else {
                    // No catch or finally - propagate error
                    Some(StepResult::Error(JsError::thrown(error_value)))
                }
            } else {
                // TryBlock frame wasn't found (shouldn't happen)
                Some(StepResult::Error(error))
            }
        } else {
            // No TryBlock found - propagate error
            Some(StepResult::Error(error))
        }
    }

    /// Step for program execution
    fn step_program(
        &mut self,
        state: &mut ExecutionState,
        statements: Rc<[Statement]>,
        index: usize,
    ) -> StepResult {
        // Check completion from previous statement FIRST (before checking if done)
        match &state.completion {
            StackCompletion::Return => {
                // Return from program - done, use the value on stack
                if state.values.is_empty() {
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                }
                return StepResult::Continue;
            }
            StackCompletion::Break(_) => {
                return StepResult::Error(JsError::syntax_error_simple("Illegal break statement"));
            }
            StackCompletion::Continue(_) => {
                return StepResult::Error(JsError::syntax_error_simple(
                    "Illegal continue statement",
                ));
            }
            _ => {}
        }

        if index >= statements.len() {
            // Program complete - return last value or undefined
            if state.values.is_empty() {
                state.push_value(Guarded::unguarded(JsValue::Undefined));
            }
            return StepResult::Continue;
        }

        // Push continuation for next statement
        state.push_frame(Frame::Program {
            statements: statements.clone(),
            index: index + 1,
        });

        // Execute current statement
        let stmt = statements.get(index).cloned();
        if let Some(stmt) = stmt {
            state.push_frame(Frame::Stmt(Rc::new(stmt)));
        }

        StepResult::Continue
    }

    /// Step for block execution
    fn step_block(
        &mut self,
        state: &mut ExecutionState,
        statements: Rc<[Statement]>,
        index: usize,
    ) -> StepResult {
        if index >= statements.len() {
            // Block complete
            if state.values.is_empty() {
                state.push_value(Guarded::unguarded(JsValue::Undefined));
            }
            return StepResult::Continue;
        }

        // Check for control flow
        match &state.completion {
            StackCompletion::Return
            | StackCompletion::Break(_)
            | StackCompletion::Continue(_)
            | StackCompletion::Throw => {
                return StepResult::Continue;
            }
            StackCompletion::Normal => {}
        }

        // Push continuation for next statement
        state.push_frame(Frame::Block {
            statements: statements.clone(),
            index: index + 1,
        });

        // Execute current statement
        let stmt = statements.get(index).cloned();
        if let Some(stmt) = stmt {
            state.push_frame(Frame::Stmt(Rc::new(stmt)));
        }

        StepResult::Continue
    }

    /// Step for statement execution
    fn step_stmt(&mut self, state: &mut ExecutionState, stmt: &Statement) -> StepResult {
        match stmt {
            Statement::Expression(expr_stmt) => {
                // Evaluate expression, then keep result
                state.push_frame(Frame::ExprStmtComplete);
                state.push_frame(Frame::Expr(Rc::clone(&expr_stmt.expression)));
                StepResult::Continue
            }

            Statement::Block(block) => {
                // Execute block
                state.push_frame(Frame::Block {
                    statements: Rc::clone(&block.body),
                    index: 0,
                });
                StepResult::Continue
            }

            Statement::Return(ret) => {
                state.completion = StackCompletion::Return;
                if let Some(expr) = &ret.argument {
                    state.push_frame(Frame::Expr(Rc::clone(expr)));
                } else {
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                }
                StepResult::Continue
            }

            Statement::Break(brk) => {
                state.completion =
                    StackCompletion::Break(brk.label.as_ref().map(|l| l.name.cheap_clone()));
                state.push_value(Guarded::unguarded(JsValue::Undefined));
                StepResult::Continue
            }

            Statement::Continue(cont) => {
                state.completion =
                    StackCompletion::Continue(cont.label.as_ref().map(|l| l.name.cheap_clone()));
                state.push_value(Guarded::unguarded(JsValue::Undefined));
                StepResult::Continue
            }

            Statement::Empty => {
                state.push_value(Guarded::unguarded(JsValue::Undefined));
                StepResult::Continue
            }

            Statement::VariableDeclaration(decl) => {
                let mutable = matches!(decl.kind, VariableKind::Let | VariableKind::Var);
                if decl.declarations.is_empty() {
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                    StepResult::Continue
                } else {
                    state.push_frame(Frame::VarDecl {
                        declarators: Rc::clone(&decl.declarations),
                        index: 0,
                        mutable,
                    });
                    StepResult::Continue
                }
            }

            Statement::If(if_stmt) => {
                // Evaluate condition, then branch
                state.push_frame(Frame::IfBranch {
                    consequent: Rc::clone(&if_stmt.consequent),
                    alternate: if_stmt.alternate.as_ref().map(Rc::clone),
                });
                state.push_frame(Frame::Expr(Rc::clone(&if_stmt.test)));
                StepResult::Continue
            }

            Statement::While(while_stmt) => {
                state.push_frame(Frame::WhileLoop {
                    test: Rc::clone(&while_stmt.test),
                    body: Rc::clone(&while_stmt.body),
                    label: None,
                });
                StepResult::Continue
            }

            Statement::DoWhile(do_while) => {
                state.push_frame(Frame::DoWhileLoop {
                    test: Rc::clone(&do_while.test),
                    body: Rc::clone(&do_while.body),
                    label: None,
                });
                StepResult::Continue
            }

            Statement::For(for_stmt) => self.setup_for_loop(state, for_stmt, None),

            Statement::ForIn(for_in) => self.setup_for_in_loop(state, for_in, None),

            Statement::ForOf(for_of) => self.setup_for_of_loop(state, for_of, None),

            Statement::Labeled(labeled) => self.setup_labeled(state, labeled),

            Statement::FunctionDeclaration(func) => {
                // Create the function and bind it to the environment
                match self.stack_execute_function_declaration(func) {
                    Ok(()) => {
                        state.push_value(Guarded::unguarded(JsValue::Undefined));
                        StepResult::Continue
                    }
                    Err(e) => StepResult::Error(e),
                }
            }

            Statement::ClassDeclaration(class) => {
                // Delegate to existing class declaration handler
                match self.execute_class_declaration(class) {
                    Ok(()) => {
                        state.push_value(Guarded::unguarded(JsValue::Undefined));
                        StepResult::Continue
                    }
                    Err(e) => StepResult::Error(e),
                }
            }

            Statement::Switch(switch_stmt) => self.setup_switch(state, switch_stmt),

            Statement::Try(try_stmt) => self.setup_try(state, try_stmt),

            Statement::Throw(throw_stmt) => {
                // Evaluate the argument, then throw
                state.push_frame(Frame::ThrowComplete);
                state.push_frame(Frame::Expr(Rc::clone(&throw_stmt.argument)));
                StepResult::Continue
            }

            Statement::Import(import) => match self.stack_execute_import(import) {
                Ok(()) => {
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                    StepResult::Continue
                }
                Err(e) => StepResult::Error(e),
            },

            Statement::Export(export) => match self.stack_execute_export(export) {
                Ok(()) => {
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                    StepResult::Continue
                }
                Err(e) => StepResult::Error(e),
            },

            // TypeScript declarations - no runtime effect
            Statement::TypeAlias(_) | Statement::InterfaceDeclaration(_) => {
                state.push_value(Guarded::unguarded(JsValue::Undefined));
                StepResult::Continue
            }

            Statement::NamespaceDeclaration(ns_decl) => {
                match self.execute_namespace_declaration(ns_decl) {
                    Ok(()) => {
                        state.push_value(Guarded::unguarded(JsValue::Undefined));
                        StepResult::Continue
                    }
                    Err(e) => StepResult::Error(e),
                }
            }

            Statement::EnumDeclaration(enum_decl) => {
                match self.execute_enum_declaration(enum_decl) {
                    Ok(()) => {
                        state.push_value(Guarded::unguarded(JsValue::Undefined));
                        StepResult::Continue
                    }
                    Err(e) => StepResult::Error(e),
                }
            }

            Statement::Debugger => {
                // Debugger statement is a no-op
                state.push_value(Guarded::unguarded(JsValue::Undefined));
                StepResult::Continue
            }
        }
    }

    /// Step for expression evaluation
    fn step_expr(&mut self, state: &mut ExecutionState, expr: &Expression) -> StepResult {
        match expr {
            Expression::Literal(lit) => {
                // RegExp literals need special handling - delegate to recursive
                if matches!(lit.value, LiteralValue::RegExp { .. }) {
                    return match self.evaluate_expression(expr) {
                        Ok(guarded) => {
                            state.push_value(guarded);
                            StepResult::Continue
                        }
                        Err(e) => StepResult::Error(e),
                    };
                }
                let value = self.stack_literal_to_value(&lit.value);
                state.push_value(Guarded::unguarded(value));
                StepResult::Continue
            }

            Expression::Identifier(id) => match self.env_get(&id.name) {
                Ok(value) => {
                    state.push_value(Guarded::unguarded(value));
                    StepResult::Continue
                }
                Err(e) => StepResult::Error(e),
            },

            Expression::Binary(bin) => {
                // For instanceof/in, delegate to recursive evaluation since they need object access
                if matches!(bin.operator, BinaryOp::Instanceof | BinaryOp::In) {
                    return match self.evaluate_expression(expr) {
                        Ok(guarded) => {
                            state.push_value(guarded);
                            StepResult::Continue
                        }
                        Err(e) => StepResult::Error(e),
                    };
                }
                // Evaluate left first, then right
                state.push_frame(Frame::BinaryRight {
                    op: bin.operator,
                    right: Rc::clone(&bin.right),
                });
                state.push_frame(Frame::Expr(Rc::clone(&bin.left)));
                StepResult::Continue
            }

            Expression::Logical(log) => {
                // Evaluate left, then check for short-circuit
                state.push_frame(Frame::LogicalCheck {
                    op: log.operator,
                    right: Rc::clone(&log.right),
                });
                state.push_frame(Frame::Expr(Rc::clone(&log.left)));
                StepResult::Continue
            }

            Expression::Unary(un) => {
                state.push_frame(Frame::UnaryComplete { op: un.operator });
                state.push_frame(Frame::Expr(Rc::clone(&un.argument)));
                StepResult::Continue
            }

            Expression::Conditional(cond) => {
                state.push_frame(Frame::ConditionalBranch {
                    consequent: Rc::clone(&cond.consequent),
                    alternate: Rc::clone(&cond.alternate),
                });
                state.push_frame(Frame::Expr(Rc::clone(&cond.test)));
                StepResult::Continue
            }

            Expression::Await(await_expr) => {
                // Evaluate the argument, then check if it's a promise
                state.push_frame(Frame::AwaitCheck);
                state.push_frame(Frame::Expr(Rc::clone(&await_expr.argument)));
                StepResult::Continue
            }

            // For complex expressions, fall back to recursive evaluation
            _ => match self.evaluate_expression(expr) {
                Ok(guarded) => {
                    state.push_value(guarded);
                    StepResult::Continue
                }
                Err(e) => StepResult::Error(e),
            },
        }
    }

    /// Convert literal to value (for stack-based evaluation)
    fn stack_literal_to_value(&self, lit: &LiteralValue) -> JsValue {
        match lit {
            LiteralValue::Null => JsValue::Null,
            LiteralValue::Undefined => JsValue::Undefined,
            LiteralValue::Boolean(b) => JsValue::Boolean(*b),
            LiteralValue::Number(n) => JsValue::Number(*n),
            LiteralValue::String(s) => JsValue::String(s.cheap_clone()),
            LiteralValue::BigInt(s) => JsValue::Number(s.parse().unwrap_or(0.0)),
            LiteralValue::RegExp { .. } => JsValue::Undefined,
        }
    }

    /// Step for binary operation completion
    fn step_binary_complete(&mut self, state: &mut ExecutionState, op: BinaryOp) -> StepResult {
        let right = state
            .pop_value()
            .map(|v| v.value)
            .unwrap_or(JsValue::Undefined);
        let left = state
            .pop_value()
            .map(|v| v.value)
            .unwrap_or(JsValue::Undefined);

        let result = self.apply_binary_op_stack(op, &left, &right);
        match result {
            Ok(value) => {
                state.push_value(Guarded::unguarded(value));
                StepResult::Continue
            }
            Err(e) => StepResult::Error(e),
        }
    }

    /// Apply binary operation (stack version)
    fn apply_binary_op_stack(
        &mut self,
        op: BinaryOp,
        left: &JsValue,
        right: &JsValue,
    ) -> Result<JsValue, JsError> {
        Ok(match op {
            // Arithmetic
            BinaryOp::Add => match (left, right) {
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
            BinaryOp::StrictEq => JsValue::Boolean(left.strict_equals(right)),
            BinaryOp::StrictNotEq => JsValue::Boolean(!left.strict_equals(right)),
            BinaryOp::Eq => JsValue::Boolean(self.abstract_equals(left, right)),
            BinaryOp::NotEq => JsValue::Boolean(!self.abstract_equals(left, right)),

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
                let lhs = (left.to_number() as i32) as u32;
                let rhs = ((right.to_number() as i32) as u32) & 0x1f;
                JsValue::Number((lhs >> rhs) as f64)
            }

            // instanceof/in handled in step_expr by delegation
            BinaryOp::Instanceof | BinaryOp::In => {
                return Err(JsError::internal_error(
                    "instanceof/in should be handled by delegation",
                ))
            }
        })
    }

    /// Step for logical check (short-circuit)
    fn step_logical_check(
        &mut self,
        state: &mut ExecutionState,
        op: LogicalOp,
        right: Rc<Expression>,
    ) -> StepResult {
        let left = state
            .pop_value()
            .map(|v| v.value)
            .unwrap_or(JsValue::Undefined);
        let left_bool = left.to_boolean();

        match op {
            LogicalOp::And => {
                if !left_bool {
                    // Short-circuit: return left
                    state.push_value(Guarded::unguarded(left));
                } else {
                    // Evaluate right
                    state.push_frame(Frame::Expr(right));
                }
            }
            LogicalOp::Or => {
                if left_bool {
                    // Short-circuit: return left
                    state.push_value(Guarded::unguarded(left));
                } else {
                    // Evaluate right
                    state.push_frame(Frame::Expr(right));
                }
            }
            LogicalOp::NullishCoalescing => {
                if !matches!(left, JsValue::Null | JsValue::Undefined) {
                    // Short-circuit: return left
                    state.push_value(Guarded::unguarded(left));
                } else {
                    // Evaluate right
                    state.push_frame(Frame::Expr(right));
                }
            }
        }
        StepResult::Continue
    }

    /// Step for unary operation
    fn step_unary_complete(&mut self, state: &mut ExecutionState, op: UnaryOp) -> StepResult {
        let operand = state
            .pop_value()
            .map(|v| v.value)
            .unwrap_or(JsValue::Undefined);

        let result = match op {
            UnaryOp::Minus => JsValue::Number(-operand.to_number()),
            UnaryOp::Plus => JsValue::Number(operand.to_number()),
            UnaryOp::Not => JsValue::Boolean(!operand.to_boolean()),
            UnaryOp::BitNot => JsValue::Number(!(operand.to_number() as i32) as f64),
            UnaryOp::Typeof => JsValue::String(JsString::from(operand.type_of())),
            UnaryOp::Void => JsValue::Undefined,
            UnaryOp::Delete => JsValue::Boolean(true),
        };
        state.push_value(Guarded::unguarded(result));
        StepResult::Continue
    }

    /// Step for await check - this is where suspension happens
    fn step_await_check(&mut self, state: &mut ExecutionState) -> StepResult {
        // Keep the guard alive while we extract the promise's result
        let Guarded {
            value: promise_val,
            guard: _promise_guard,
        } = state
            .pop_value()
            .unwrap_or_else(|| Guarded::unguarded(JsValue::Undefined));

        // If not a promise, just return the value
        let JsValue::Object(obj) = &promise_val else {
            state.push_value(Guarded::unguarded(promise_val));
            return StepResult::Continue;
        };

        let obj_ref = obj.borrow();
        let ExoticObject::Promise(ref promise_state) = obj_ref.exotic else {
            // Not a promise, return as-is
            drop(obj_ref);
            state.push_value(Guarded::unguarded(promise_val));
            return StepResult::Continue;
        };

        let status = promise_state.borrow().status.clone();
        let result = promise_state.borrow().result.clone();
        drop(obj_ref);

        match status {
            PromiseStatus::Fulfilled => {
                let value = result.unwrap_or(JsValue::Undefined);
                // Guard the value to prevent GC during execution
                let guard = self.guard_value(&value);
                state.push_value(Guarded { value, guard });
                StepResult::Continue
            }
            PromiseStatus::Rejected => {
                let reason = result.unwrap_or(JsValue::Undefined);
                // Guard the reason to prevent GC during error propagation
                self.thrown_guard = self.guard_value(&reason);
                StepResult::Error(JsError::thrown(reason))
            }
            PromiseStatus::Pending => {
                // SUSPENSION POINT!
                // Store the promise we're waiting on
                state.waiting_on = Some(obj.clone());
                StepResult::Suspend(obj.clone())
            }
        }
    }

    /// Resume execution after a promise resolves
    pub fn resume_with_value(&mut self, state: &mut ExecutionState, value: JsValue) -> StepResult {
        state.waiting_on = None;
        state.push_value(Guarded::unguarded(value));
        self.run(state)
    }

    /// Resume execution after a promise rejects
    pub fn resume_with_error(&mut self, state: &mut ExecutionState, error: JsError) -> StepResult {
        state.waiting_on = None;
        StepResult::Error(error)
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Variable Declaration
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Step for variable declaration - process declarators sequentially
    fn step_var_decl(
        &mut self,
        state: &mut ExecutionState,
        declarators: Rc<[VariableDeclarator]>,
        index: usize,
        mutable: bool,
    ) -> StepResult {
        if index >= declarators.len() {
            // All declarators processed
            state.push_value(Guarded::unguarded(JsValue::Undefined));
            return StepResult::Continue;
        }

        let declarator = match declarators.get(index) {
            Some(d) => d,
            None => {
                state.push_value(Guarded::unguarded(JsValue::Undefined));
                return StepResult::Continue;
            }
        };

        // Push continuation for remaining declarators
        if index + 1 < declarators.len() {
            state.push_frame(Frame::VarDecl {
                declarators: declarators.clone(),
                index: index + 1,
                mutable,
            });
        }

        // Push bind frame - will be processed after init expression
        state.push_frame(Frame::VarBind {
            pattern: Rc::new(declarator.id.clone()),
            mutable,
        });

        // Evaluate init expression (or undefined)
        match &declarator.init {
            Some(expr) => {
                state.push_frame(Frame::Expr(Rc::clone(expr)));
            }
            None => {
                state.push_value(Guarded::unguarded(JsValue::Undefined));
            }
        }

        StepResult::Continue
    }

    /// Step for variable binding - bind value to pattern
    fn step_var_bind(
        &mut self,
        state: &mut ExecutionState,
        pattern: &Pattern,
        mutable: bool,
    ) -> StepResult {
        let Guarded {
            value: init_value,
            guard: _init_guard,
        } = state
            .pop_value()
            .unwrap_or(Guarded::unguarded(JsValue::Undefined));

        // bind_pattern calls env_define which establishes ownership
        match self.bind_pattern(pattern, init_value, mutable) {
            Ok(()) => StepResult::Continue,
            Err(e) => StepResult::Error(e),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Control Flow
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Step for if branch - condition evaluated, pick and execute branch
    fn step_if_branch(
        &mut self,
        state: &mut ExecutionState,
        consequent: Rc<Statement>,
        alternate: Option<Rc<Statement>>,
    ) -> StepResult {
        let condition = state
            .pop_value()
            .map(|v| v.value)
            .unwrap_or(JsValue::Undefined);

        if condition.to_boolean() {
            state.push_frame(Frame::Stmt(consequent));
        } else if let Some(alt) = alternate {
            state.push_frame(Frame::Stmt(alt));
        } else {
            state.push_value(Guarded::unguarded(JsValue::Undefined));
        }
        StepResult::Continue
    }

    /// Step for while check - test evaluated, execute body or exit
    fn step_while_check(
        &mut self,
        state: &mut ExecutionState,
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: LoopLabel,
    ) -> StepResult {
        // Get condition value (test was already evaluated)
        let condition = state
            .pop_value()
            .map(|v| v.value)
            .unwrap_or(JsValue::Undefined);

        if condition.to_boolean() {
            // Continue loop: push WhileAfterBody to check completion after body, then execute body
            state.push_frame(Frame::WhileAfterBody {
                test,
                body: body.clone(),
                label,
            });
            // Execute body
            state.push_frame(Frame::Stmt(body));
        } else {
            // Exit loop
            state.push_value(Guarded::unguarded(JsValue::Undefined));
        }
        StepResult::Continue
    }

    /// Step for while after body - body executed, check completion before evaluating test
    fn step_while_after_body(
        &mut self,
        state: &mut ExecutionState,
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: LoopLabel,
    ) -> StepResult {
        // Check for break/continue/return/throw from body execution
        match &state.completion {
            StackCompletion::Break(brk_label) => {
                // Check if break targets this loop
                if brk_label.is_none() || brk_label.as_ref() == label_ref(&label) {
                    state.completion = StackCompletion::Normal;
                    let _ = state.pop_value(); // Discard body result
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                    return StepResult::Continue;
                }
                // Break targets outer loop - propagate
                return StepResult::Continue;
            }
            StackCompletion::Continue(cont_label) => {
                // Check if continue targets this loop
                if cont_label.is_none() || cont_label.as_ref() == label_ref(&label) {
                    state.completion = StackCompletion::Normal;
                    let _ = state.pop_value(); // Discard body result
                                               // Continue to next iteration - evaluate test again
                    state.push_frame(Frame::WhileCheck {
                        test: test.clone(),
                        body,
                        label,
                    });
                    state.push_frame(Frame::Expr(test));
                    return StepResult::Continue;
                }
                // Continue targets outer loop - propagate
                return StepResult::Continue;
            }
            StackCompletion::Return | StackCompletion::Throw => {
                // Return/throw - propagate up (don't push undefined, preserve return value)
                return StepResult::Continue;
            }
            StackCompletion::Normal => {}
        }

        // Discard body result
        let _ = state.pop_value();

        // Normal completion - evaluate test for next iteration
        state.push_frame(Frame::WhileCheck {
            test: test.clone(),
            body,
            label,
        });
        state.push_frame(Frame::Expr(test));

        StepResult::Continue
    }

    /// Step for do-while check - body executed, test evaluated, loop or exit
    fn step_do_while_check(
        &mut self,
        state: &mut ExecutionState,
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: LoopLabel,
    ) -> StepResult {
        // Check for break/continue from body execution
        match &state.completion {
            StackCompletion::Break(brk_label) => {
                if brk_label.is_none() || brk_label.as_ref() == label_ref(&label) {
                    state.completion = StackCompletion::Normal;
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                    return StepResult::Continue;
                }
                return StepResult::Continue;
            }
            StackCompletion::Continue(cont_label) => {
                if cont_label.is_none() || cont_label.as_ref() == label_ref(&label) {
                    state.completion = StackCompletion::Normal;
                    // In do-while, continue goes to condition check
                    // Fall through to check the condition
                } else {
                    return StepResult::Continue;
                }
            }
            StackCompletion::Return | StackCompletion::Throw => {
                return StepResult::Continue;
            }
            StackCompletion::Normal => {}
        }

        // Body executed, now check condition - evaluate test
        // Pop any existing value from stack (from body)
        let _ = state.pop_value();

        // We need to evaluate the test expression and then check it
        // Push frame to check result after test is evaluated
        state.push_frame(Frame::DoWhileTestCheck {
            test: test.clone(),
            body,
            label,
        });
        state.push_frame(Frame::Expr(test));
        StepResult::Continue
    }

    /// Step for do-while test check - condition evaluated, loop or exit
    fn step_do_while_test_check(
        &mut self,
        state: &mut ExecutionState,
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: LoopLabel,
    ) -> StepResult {
        let condition = state
            .pop_value()
            .map(|v| v.value)
            .unwrap_or(JsValue::Undefined);

        if condition.to_boolean() {
            // Continue loop - execute body, then check condition again
            state.push_frame(Frame::DoWhileLoop {
                test,
                body: body.clone(),
                label,
            });
        } else {
            // Exit loop
            state.push_value(Guarded::unguarded(JsValue::Undefined));
        }
        StepResult::Continue
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // For Loop
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Set up a for loop - handle var hoisting, create scope, setup init
    fn setup_for_loop(
        &mut self,
        state: &mut ExecutionState,
        for_stmt: &ForStatement,
        label: LoopLabel,
    ) -> StepResult {
        // Handle var declarations BEFORE creating loop scope (var is function-scoped)
        let has_var_init =
            matches!(&for_stmt.init, Some(ForInit::Variable(d)) if d.kind == VariableKind::Var);

        if has_var_init {
            if let Some(ForInit::Variable(decl)) = &for_stmt.init {
                if let Err(e) = self.execute_variable_declaration(decl) {
                    return StepResult::Error(e);
                }
            }
        }

        // Create loop scope
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
                            None
                        }
                    })
                    .collect()
            }
            _ => vec![],
        };

        let test = for_stmt.test.as_ref().map(Rc::clone);
        let update = for_stmt.update.as_ref().map(Rc::clone);
        let body = Rc::clone(&for_stmt.body);
        let loop_vars = Rc::new(loop_vars);

        // Push cleanup frame (will be executed when loop exits)
        state.push_frame(Frame::ForCleanup {
            saved_env: saved_env.clone(),
        });

        // Push the main loop frame (will start after init)
        state.push_frame(Frame::ForLoop {
            test,
            update,
            body,
            label,
            loop_vars: loop_vars.clone(),
            saved_env: saved_env.clone(),
            first_iteration: true, // First iteration needs to create per-iteration env
        });

        // Handle init
        match &for_stmt.init {
            Some(ForInit::Variable(decl)) if decl.kind != VariableKind::Var => {
                // let/const - execute in loop scope
                state.push_frame(Frame::VarDecl {
                    declarators: Rc::clone(&decl.declarations),
                    index: 0,
                    mutable: decl.kind == VariableKind::Let,
                });
            }
            Some(ForInit::Expression(expr)) => {
                // Expression init - discard result
                state.push_frame(Frame::DiscardValue);
                state.push_frame(Frame::Expr(Rc::clone(expr)));
            }
            _ => {
                // No init or var already handled
            }
        }

        StepResult::Continue
    }

    /// Step for ForLoop - set up test evaluation and per-iteration environment
    fn step_for_loop(
        &mut self,
        state: &mut ExecutionState,
        test: Option<Rc<Expression>>,
        update: Option<Rc<Expression>>,
        body: Rc<Statement>,
        label: LoopLabel,
        loop_vars: Rc<Vec<(JsString, bool)>>,
        saved_env: Gc<JsObject>,
        first_iteration: bool,
    ) -> StepResult {
        // Create per-iteration environment if needed
        // On first iteration: create from loop scope (copy loop vars from self.env)
        // On subsequent iterations: step_for_after_body already created it
        if first_iteration && !loop_vars.is_empty() {
            let (iter_env, iter_guard) =
                create_environment_unrooted(&self.heap, Some(saved_env.clone()));
            for (name, mutable) in loop_vars.iter() {
                let value = match self.env_get(name) {
                    Ok(v) => v,
                    Err(e) => return StepResult::Error(e),
                };
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
            }
            self.env = iter_env;
            // Push guard to keep this iteration's environment alive
            self.push_env_guard(iter_guard);
        }

        // Set up test check frame
        state.push_frame(Frame::ForTestCheck {
            test: test.clone(),
            update,
            body,
            label,
            loop_vars,
            saved_env,
        });

        // Evaluate test if present
        if let Some(test_expr) = test {
            state.push_frame(Frame::Expr(test_expr));
        } else {
            // No test means always true
            state.push_value(Guarded::unguarded(JsValue::Boolean(true)));
        }

        StepResult::Continue
    }

    /// Step for ForTestCheck - condition evaluated, decide to continue or exit
    fn step_for_test_check(
        &mut self,
        state: &mut ExecutionState,
        test: Option<Rc<Expression>>,
        update: Option<Rc<Expression>>,
        body: Rc<Statement>,
        label: LoopLabel,
        loop_vars: Rc<Vec<(JsString, bool)>>,
        saved_env: Gc<JsObject>,
    ) -> StepResult {
        let condition = state
            .pop_value()
            .map(|v| v.value)
            .unwrap_or(JsValue::Boolean(true));

        if !condition.to_boolean() {
            // Exit loop - pop iteration guard if we had loop vars
            if !loop_vars.is_empty() {
                self.pop_env_guard();
            }
            // Cleanup frame already on stack will restore loop scope
            state.push_value(Guarded::unguarded(JsValue::Undefined));
            return StepResult::Continue;
        }

        // Continue loop - execute body, then handle post-body
        state.push_frame(Frame::ForAfterBody {
            test,
            update,
            body: body.clone(),
            label,
            loop_vars,
            saved_env,
        });
        state.push_frame(Frame::Stmt(body));

        StepResult::Continue
    }

    /// Step for ForAfterBody - handle control flow, per-iteration env, update
    fn step_for_after_body(
        &mut self,
        state: &mut ExecutionState,
        test: Option<Rc<Expression>>,
        update: Option<Rc<Expression>>,
        body: Rc<Statement>,
        label: LoopLabel,
        loop_vars: Rc<Vec<(JsString, bool)>>,
        saved_env: Gc<JsObject>,
    ) -> StepResult {
        // Check for control flow from body
        match &state.completion {
            StackCompletion::Break(brk_label) => {
                if brk_label.is_none() || brk_label.as_ref() == label_ref(&label) {
                    state.completion = StackCompletion::Normal;
                    // Pop iteration guard if we had loop vars
                    if !loop_vars.is_empty() {
                        self.pop_env_guard();
                    }
                    // Exit loop - cleanup will happen
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                    return StepResult::Continue;
                }
                // Break targets outer loop - pop iteration guard and propagate
                if !loop_vars.is_empty() {
                    self.pop_env_guard();
                }
                return StepResult::Continue;
            }
            StackCompletion::Continue(cont_label) => {
                if cont_label.is_none() || cont_label.as_ref() == label_ref(&label) {
                    state.completion = StackCompletion::Normal;
                    // Continue to update phase (fall through)
                } else {
                    // Continue targets outer loop - pop iteration guard and propagate
                    if !loop_vars.is_empty() {
                        self.pop_env_guard();
                    }
                    return StepResult::Continue;
                }
            }
            StackCompletion::Return | StackCompletion::Throw => {
                // Return/throw - pop iteration guard and propagate up (cleanup will happen)
                if !loop_vars.is_empty() {
                    self.pop_env_guard();
                }
                return StepResult::Continue;
            }
            StackCompletion::Normal => {}
        }

        // Pop body result
        let _ = state.pop_value();

        // Create new per-iteration environment BEFORE update (ES spec)
        if !loop_vars.is_empty() {
            // Create new environment first (may trigger GC)
            let current_env = self.env.clone();
            let (new_iter_env, new_guard) =
                create_environment_unrooted(&self.heap, Some(saved_env.clone()));

            // Copy loop variables from current to new environment
            for (name, mutable) in loop_vars.iter() {
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

            // Push new guard BEFORE popping old one - prevents GC gap
            self.push_env_guard(new_guard);
            // Now safe to pop the old guard
            let _old_guard = self.env_guards.remove(self.env_guards.len() - 2);

            self.env = new_iter_env;
        }

        // Push next iteration
        state.push_frame(Frame::ForLoop {
            test,
            update: update.clone(),
            body,
            label,
            loop_vars,
            saved_env,
            first_iteration: false, // Not first iteration - env already created
        });

        // Execute update if present
        if let Some(upd) = update {
            state.push_frame(Frame::DiscardValue);
            state.push_frame(Frame::Expr(upd));
        }

        StepResult::Continue
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // For-In Loop Implementation
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Setup for-in loop: evaluate right side, collect keys
    fn setup_for_in_loop(
        &mut self,
        state: &mut ExecutionState,
        for_in: &ForInStatement,
        label: LoopLabel,
    ) -> StepResult {
        // Evaluate right side
        let right = match self.evaluate_expression(&for_in.right) {
            Ok(guarded) => guarded.value,
            Err(e) => return StepResult::Error(e),
        };

        // Collect enumerable keys
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

        let saved_env = self.env.clone();

        // Push loop frame
        state.push_frame(Frame::ForInLoop {
            keys: Rc::new(keys),
            index: 0,
            left: Rc::new(for_in.left.clone()),
            body: Rc::clone(&for_in.body),
            label,
            saved_env,
        });

        StepResult::Continue
    }

    /// Step for ForInLoop: bind current key and execute body
    fn step_for_in_loop(
        &mut self,
        state: &mut ExecutionState,
        keys: Rc<Vec<String>>,
        index: usize,
        left: Rc<ForInOfLeft>,
        body: Rc<Statement>,
        label: LoopLabel,
        saved_env: Gc<JsObject>,
    ) -> StepResult {
        // Check if we've iterated through all keys
        if index >= keys.len() {
            // Loop finished - restore env
            self.env = saved_env;
            state.push_value(Guarded::unguarded(JsValue::Undefined));
            return StepResult::Continue;
        }

        // Check for timeout
        if let Err(e) = self.check_timeout() {
            self.env = saved_env;
            return StepResult::Error(e);
        }

        // Get current key
        let key = match keys.get(index) {
            Some(k) => k.clone(),
            None => {
                self.env = saved_env;
                state.push_value(Guarded::unguarded(JsValue::Undefined));
                return StepResult::Continue;
            }
        };
        let key_value = JsValue::String(JsString::from(key));

        // Create per-iteration environment with guard
        let (iter_env, iter_guard) =
            create_environment_unrooted(&self.heap, Some(saved_env.clone()));
        eprintln!(
            "[step_for_in_loop] Created iter_env id={}, exotic={:?}",
            iter_env.id(),
            std::mem::discriminant(&iter_env.borrow().exotic)
        );
        self.env = iter_env.clone(); // Clone to avoid moving
        eprintln!(
            "[step_for_in_loop] After self.env = iter_env: self.env.id={}, exotic={:?}",
            self.env.id(),
            std::mem::discriminant(&self.env.borrow().exotic)
        );
        self.push_env_guard(iter_guard);

        // Bind the key to the left-hand side
        match &*left {
            ForInOfLeft::Variable(decl) => {
                let mutable = decl.kind != VariableKind::Const;
                if let Some(declarator) = decl.declarations.first() {
                    if let Err(e) = self.bind_pattern(&declarator.id, key_value, mutable) {
                        self.pop_env_guard();
                        self.env = saved_env;
                        return StepResult::Error(e);
                    }
                }
            }
            ForInOfLeft::Pattern(pattern) => {
                if let Err(e) = self.assign_pattern(pattern, key_value) {
                    self.pop_env_guard();
                    self.env = saved_env;
                    return StepResult::Error(e);
                }
            }
        }

        // Push after-body frame to handle next iteration
        state.push_frame(Frame::ForInAfterBody {
            keys,
            index,
            left,
            body: body.clone(),
            label,
            saved_env,
        });

        // Push body statement
        state.push_frame(Frame::Stmt(body));

        StepResult::Continue
    }

    /// Step for ForInAfterBody: handle control flow and proceed to next iteration
    fn step_for_in_after_body(
        &mut self,
        state: &mut ExecutionState,
        keys: Rc<Vec<String>>,
        index: usize,
        left: Rc<ForInOfLeft>,
        body: Rc<Statement>,
        label: LoopLabel,
        saved_env: Gc<JsObject>,
    ) -> StepResult {
        // Check completion type
        match &state.completion {
            StackCompletion::Break(break_label) => {
                if break_label.is_none() || break_label.as_ref() == label_ref(&label) {
                    // Break targets this loop - pop iteration guard and restore env
                    state.completion = StackCompletion::Normal;
                    self.pop_env_guard();
                    self.env = saved_env;
                    let _ = state.pop_value();
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                    return StepResult::Continue;
                }
                // Break targets outer loop - pop iteration guard, restore env and propagate
                self.pop_env_guard();
                self.env = saved_env;
                return StepResult::Continue;
            }
            StackCompletion::Continue(cont_label) => {
                if cont_label.is_none() || cont_label.as_ref() == label_ref(&label) {
                    // Continue targets this loop - pop guard and proceed to next iteration
                    state.completion = StackCompletion::Normal;
                    self.pop_env_guard();
                } else {
                    // Continue targets outer loop - pop iteration guard, restore env and propagate
                    self.pop_env_guard();
                    self.env = saved_env;
                    return StepResult::Continue;
                }
            }
            StackCompletion::Return | StackCompletion::Throw => {
                // Return/throw - pop iteration guard, restore env and propagate
                self.pop_env_guard();
                self.env = saved_env;
                return StepResult::Continue;
            }
            StackCompletion::Normal => {
                // Normal completion - pop guard before next iteration creates new one
                self.pop_env_guard();
            }
        }

        // Pop body result
        let _ = state.pop_value();

        // Push next iteration
        state.push_frame(Frame::ForInLoop {
            keys,
            index: index + 1,
            left,
            body,
            label,
            saved_env,
        });

        StepResult::Continue
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // For-Of Loop Implementation
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Setup for-of loop: evaluate right side, collect items or set up generator iteration
    fn setup_for_of_loop(
        &mut self,
        state: &mut ExecutionState,
        for_of: &ForOfStatement,
        label: LoopLabel,
    ) -> StepResult {
        // Evaluate right side
        let right = match self.evaluate_expression(&for_of.right) {
            Ok(guarded) => guarded.value,
            Err(e) => return StepResult::Error(e),
        };

        let saved_env = self.env.clone();

        // Check if it's a generator - handle with special frame
        if let JsValue::Object(ref obj) = right {
            if let ExoticObject::Generator(gen_state) = &obj.borrow().exotic {
                // Use generator-specific iteration
                state.push_frame(Frame::ForOfGenerator(Box::new(ForOfGeneratorData {
                    generator: obj.clone(),
                    gen_state: gen_state.clone(),
                    left: Rc::new(for_of.left.clone()),
                    body: Rc::clone(&for_of.body),
                    label,
                    saved_env,
                })));
                return StepResult::Continue;
            }
        }

        // Collect items to iterate over for non-generators
        let items = match &right {
            JsValue::Object(obj) => {
                let obj_ref = obj.borrow();
                match &obj_ref.exotic {
                    ExoticObject::Array { length } => {
                        let mut items = Vec::with_capacity(*length as usize);
                        for i in 0..*length {
                            items.push(
                                obj_ref
                                    .get_property(&crate::value::PropertyKey::Index(i))
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

        // Push loop frame
        state.push_frame(Frame::ForOfLoop {
            items: Rc::new(items),
            index: 0,
            left: Rc::new(for_of.left.clone()),
            body: Rc::new((*for_of.body).clone()),
            label,
            saved_env,
        });

        StepResult::Continue
    }

    /// Step for ForOfLoop: bind current item and execute body
    fn step_for_of_loop(
        &mut self,
        state: &mut ExecutionState,
        items: Rc<Vec<JsValue>>,
        index: usize,
        left: Rc<ForInOfLeft>,
        body: Rc<Statement>,
        label: LoopLabel,
        saved_env: Gc<JsObject>,
    ) -> StepResult {
        // Check if we've iterated through all items
        if index >= items.len() {
            // Loop finished - restore env
            self.env = saved_env;
            state.push_value(Guarded::unguarded(JsValue::Undefined));
            return StepResult::Continue;
        }

        // Check for timeout
        if let Err(e) = self.check_timeout() {
            self.env = saved_env;
            return StepResult::Error(e);
        }

        // Get current item
        let item = match items.get(index) {
            Some(i) => i.clone(),
            None => {
                self.env = saved_env;
                state.push_value(Guarded::unguarded(JsValue::Undefined));
                return StepResult::Continue;
            }
        };

        // Create per-iteration environment with guard
        let (iter_env, iter_guard) =
            create_environment_unrooted(&self.heap, Some(saved_env.clone()));
        self.env = iter_env;
        self.push_env_guard(iter_guard);

        // Bind the item to the left-hand side
        match &*left {
            ForInOfLeft::Variable(decl) => {
                let mutable = decl.kind != VariableKind::Const;
                if let Some(declarator) = decl.declarations.first() {
                    if let Err(e) = self.bind_pattern(&declarator.id, item, mutable) {
                        self.pop_env_guard();
                        self.env = saved_env;
                        return StepResult::Error(e);
                    }
                }
            }
            ForInOfLeft::Pattern(pattern) => {
                if let Err(e) = self.assign_pattern(pattern, item) {
                    self.pop_env_guard();
                    self.env = saved_env;
                    return StepResult::Error(e);
                }
            }
        }

        // Push after-body frame to handle next iteration
        state.push_frame(Frame::ForOfAfterBody {
            items,
            index,
            left,
            body: body.clone(),
            label,
            saved_env,
        });

        // Push body statement
        state.push_frame(Frame::Stmt(body));

        StepResult::Continue
    }

    /// Step for ForOfAfterBody: handle control flow and proceed to next iteration
    fn step_for_of_after_body(
        &mut self,
        state: &mut ExecutionState,
        items: Rc<Vec<JsValue>>,
        index: usize,
        left: Rc<ForInOfLeft>,
        body: Rc<Statement>,
        label: LoopLabel,
        saved_env: Gc<JsObject>,
    ) -> StepResult {
        // Check completion type
        match &state.completion {
            StackCompletion::Break(break_label) => {
                if break_label.is_none() || break_label.as_ref() == label_ref(&label) {
                    // Break targets this loop - pop iteration guard and restore env
                    state.completion = StackCompletion::Normal;
                    self.pop_env_guard();
                    self.env = saved_env;
                    let _ = state.pop_value();
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                    return StepResult::Continue;
                }
                // Break targets outer loop - pop iteration guard, restore env and propagate
                self.pop_env_guard();
                self.env = saved_env;
                return StepResult::Continue;
            }
            StackCompletion::Continue(cont_label) => {
                if cont_label.is_none() || cont_label.as_ref() == label_ref(&label) {
                    // Continue targets this loop - pop guard and proceed to next iteration
                    state.completion = StackCompletion::Normal;
                    self.pop_env_guard();
                } else {
                    // Continue targets outer loop - pop iteration guard, restore env and propagate
                    self.pop_env_guard();
                    self.env = saved_env;
                    return StepResult::Continue;
                }
            }
            StackCompletion::Return | StackCompletion::Throw => {
                // Return/throw - pop iteration guard, restore env and propagate
                self.pop_env_guard();
                self.env = saved_env;
                return StepResult::Continue;
            }
            StackCompletion::Normal => {
                // Normal completion - pop guard before next iteration creates new one
                self.pop_env_guard();
            }
        }

        // Pop body result
        let _ = state.pop_value();

        // Push next iteration
        state.push_frame(Frame::ForOfLoop {
            items,
            index: index + 1,
            left,
            body,
            label,
            saved_env,
        });

        StepResult::Continue
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // For-Of Generator Loop Implementation
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Step for ForOfGenerator: call next() on generator and iterate
    fn step_for_of_generator(
        &mut self,
        state: &mut ExecutionState,
        generator: Gc<JsObject>,
        gen_state: Rc<std::cell::RefCell<crate::value::GeneratorState>>,
        left: Rc<ForInOfLeft>,
        body: Rc<Statement>,
        label: LoopLabel,
        saved_env: Gc<JsObject>,
    ) -> StepResult {
        // Check for timeout
        if let Err(e) = self.check_timeout() {
            self.env = saved_env;
            return StepResult::Error(e);
        }

        // Call next() on the generator
        let result = match self.resume_generator(&gen_state) {
            Ok(guarded) => guarded.value,
            Err(e) => {
                self.env = saved_env;
                return StepResult::Error(e);
            }
        };

        // Get done and value from result
        let done_key = self.key("done");
        let value_key = self.key("value");

        let (done, value) = match &result {
            JsValue::Object(obj) => {
                let done = obj
                    .borrow()
                    .get_property(&done_key)
                    .map(|v| v.to_boolean())
                    .unwrap_or(false);
                let value = obj
                    .borrow()
                    .get_property(&value_key)
                    .unwrap_or(JsValue::Undefined);
                (done, value)
            }
            _ => (true, JsValue::Undefined),
        };

        // If done, loop is complete
        if done {
            self.env = saved_env;
            state.push_value(Guarded::unguarded(JsValue::Undefined));
            return StepResult::Continue;
        }

        // Create per-iteration environment with guard
        let (iter_env, iter_guard) =
            create_environment_unrooted(&self.heap, Some(saved_env.clone()));
        self.env = iter_env;
        self.push_env_guard(iter_guard);

        // Bind the value to the left-hand side
        match &*left {
            ForInOfLeft::Variable(decl) => {
                let mutable = decl.kind != VariableKind::Const;
                if let Some(declarator) = decl.declarations.first() {
                    if let Err(e) = self.bind_pattern(&declarator.id, value, mutable) {
                        self.pop_env_guard();
                        self.env = saved_env;
                        return StepResult::Error(e);
                    }
                }
            }
            ForInOfLeft::Pattern(pattern) => {
                if let Err(e) = self.bind_pattern(pattern, value, true) {
                    self.pop_env_guard();
                    self.env = saved_env;
                    return StepResult::Error(e);
                }
            }
        }

        // Push continuation frame for after body
        state.push_frame(Frame::ForOfGeneratorAfterBody(Box::new(ForOfGeneratorData {
            generator,
            gen_state,
            left,
            body: body.clone(),
            label,
            saved_env,
        })));

        // Push body statement
        state.push_frame(Frame::Stmt(body));

        StepResult::Continue
    }

    /// Step for ForOfGeneratorAfterBody: handle control flow and proceed to next iteration
    fn step_for_of_generator_after_body(
        &mut self,
        state: &mut ExecutionState,
        generator: Gc<JsObject>,
        gen_state: Rc<std::cell::RefCell<crate::value::GeneratorState>>,
        left: Rc<ForInOfLeft>,
        body: Rc<Statement>,
        label: LoopLabel,
        saved_env: Gc<JsObject>,
    ) -> StepResult {
        // Check completion type
        match &state.completion {
            StackCompletion::Break(break_label) => {
                if break_label.is_none() || break_label.as_ref() == label_ref(&label) {
                    // Break targets this loop - pop iteration guard and restore env
                    state.completion = StackCompletion::Normal;
                    self.pop_env_guard();
                    self.env = saved_env;
                    let _ = state.pop_value();
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                    return StepResult::Continue;
                }
                // Break targets outer loop - pop iteration guard, restore env and propagate
                self.pop_env_guard();
                self.env = saved_env;
                return StepResult::Continue;
            }
            StackCompletion::Continue(cont_label) => {
                if cont_label.is_none() || cont_label.as_ref() == label_ref(&label) {
                    // Continue targets this loop - pop guard and proceed to next iteration
                    state.completion = StackCompletion::Normal;
                    self.pop_env_guard();
                } else {
                    // Continue targets outer loop - pop iteration guard, restore env and propagate
                    self.pop_env_guard();
                    self.env = saved_env;
                    return StepResult::Continue;
                }
            }
            StackCompletion::Return | StackCompletion::Throw => {
                // Return/throw - pop iteration guard, restore env and propagate
                self.pop_env_guard();
                self.env = saved_env;
                return StepResult::Continue;
            }
            StackCompletion::Normal => {
                // Normal completion - pop guard before next iteration creates new one
                self.pop_env_guard();
            }
        }

        // Pop body result
        let _ = state.pop_value();

        // Push next iteration
        state.push_frame(Frame::ForOfGenerator(Box::new(ForOfGeneratorData {
            generator,
            gen_state,
            left,
            body,
            label,
            saved_env,
        })));

        StepResult::Continue
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Labeled Statement Implementation
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Setup labeled statement - push label context and body
    fn setup_labeled(
        &mut self,
        state: &mut ExecutionState,
        labeled: &crate::ast::LabeledStatement,
    ) -> StepResult {
        let label = labeled.label.name.cheap_clone();
        let body = Rc::clone(&labeled.body);

        // Check if this is a labeled loop - if so, pass the label to the loop
        match labeled.body.as_ref() {
            Statement::While(while_stmt) => {
                state.push_frame(Frame::LabeledComplete {
                    label: label.cheap_clone(),
                });
                state.push_frame(Frame::WhileLoop {
                    test: Rc::clone(&while_stmt.test),
                    body: Rc::clone(&while_stmt.body),
                    label: Some(Box::new(label)),
                });
                StepResult::Continue
            }
            Statement::DoWhile(do_while) => {
                state.push_frame(Frame::LabeledComplete {
                    label: label.cheap_clone(),
                });
                state.push_frame(Frame::DoWhileLoop {
                    test: Rc::clone(&do_while.test),
                    body: Rc::clone(&do_while.body),
                    label: Some(Box::new(label)),
                });
                StepResult::Continue
            }
            Statement::For(for_stmt) => {
                state.push_frame(Frame::LabeledComplete {
                    label: label.cheap_clone(),
                });
                self.setup_for_loop(state, for_stmt, Some(Box::new(label)))
            }
            Statement::ForIn(for_in) => {
                state.push_frame(Frame::LabeledComplete {
                    label: label.cheap_clone(),
                });
                self.setup_for_in_loop(state, for_in, Some(Box::new(label)))
            }
            Statement::ForOf(for_of) => {
                state.push_frame(Frame::LabeledComplete {
                    label: label.cheap_clone(),
                });
                self.setup_for_of_loop(state, for_of, Some(Box::new(label)))
            }
            _ => {
                // Non-loop statement with label
                state.push_frame(Frame::LabeledComplete { label });
                state.push_frame(Frame::Stmt(body));
                StepResult::Continue
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Switch Statement Implementation
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Setup switch statement - push discriminant evaluation frame
    fn setup_switch(
        &mut self,
        state: &mut ExecutionState,
        switch_stmt: &crate::ast::SwitchStatement,
    ) -> StepResult {
        // Push frame to handle after discriminant is evaluated
        state.push_frame(Frame::SwitchEval {
            cases: Rc::clone(&switch_stmt.cases),
        });
        // Evaluate discriminant first
        state.push_frame(Frame::Expr(Rc::clone(&switch_stmt.discriminant)));
        StepResult::Continue
    }

    /// Step for switch case matching - find matching case
    fn step_switch_match(
        &mut self,
        state: &mut ExecutionState,
        discriminant: JsValue,
        cases: Rc<[crate::ast::SwitchCase]>,
        index: usize,
        found_match: bool,
    ) -> StepResult {
        // If we already found a match, start executing from here
        if found_match {
            // Start executing from the matched case
            state.push_frame(Frame::SwitchBody {
                discriminant,
                cases,
                case_index: index,
                stmt_index: 0,
            });
            return StepResult::Continue;
        }

        // If we've gone through all cases without a match, look for default
        if index >= cases.len() {
            // Find default case
            let mut default_index = None;
            for (i, case) in cases.iter().enumerate() {
                if case.test.is_none() {
                    default_index = Some(i);
                    break;
                }
            }

            if let Some(idx) = default_index {
                // Execute from default case
                state.push_frame(Frame::SwitchBody {
                    discriminant,
                    cases,
                    case_index: idx,
                    stmt_index: 0,
                });
            } else {
                // No default, switch is done
                state.push_value(Guarded::unguarded(JsValue::Undefined));
            }
            return StepResult::Continue;
        }

        let case = match cases.get(index) {
            Some(c) => c,
            None => {
                state.push_value(Guarded::unguarded(JsValue::Undefined));
                return StepResult::Continue;
            }
        };

        // Skip default case during matching phase
        if case.test.is_none() {
            state.push_frame(Frame::SwitchMatch {
                discriminant,
                cases,
                index: index + 1,
                found_match: false,
            });
            return StepResult::Continue;
        }

        // Evaluate case test
        let test_expr = case.test.as_ref();
        if let Some(test_expr) = test_expr {
            match self.evaluate_expression(test_expr) {
                Ok(guarded) => {
                    if discriminant.strict_equals(&guarded.value) {
                        // Found match - start executing from this case
                        state.push_frame(Frame::SwitchBody {
                            discriminant,
                            cases,
                            case_index: index,
                            stmt_index: 0,
                        });
                    } else {
                        // No match, try next case
                        state.push_frame(Frame::SwitchMatch {
                            discriminant,
                            cases,
                            index: index + 1,
                            found_match: false,
                        });
                    }
                }
                Err(e) => return StepResult::Error(e),
            }
        }

        StepResult::Continue
    }

    /// Step for switch body execution - execute statements with fall-through
    fn step_switch_body(
        &mut self,
        state: &mut ExecutionState,
        discriminant: JsValue,
        cases: Rc<[crate::ast::SwitchCase]>,
        case_index: usize,
        stmt_index: usize,
    ) -> StepResult {
        // Check for break from previous statement
        if let StackCompletion::Break(None) = state.completion {
            state.completion = StackCompletion::Normal;
            state.push_value(Guarded::unguarded(JsValue::Undefined));
            return StepResult::Continue;
        }

        // Check for labeled break, return, or continue - propagate
        match &state.completion {
            StackCompletion::Break(Some(_))
            | StackCompletion::Return
            | StackCompletion::Continue(_)
            | StackCompletion::Throw => {
                return StepResult::Continue;
            }
            StackCompletion::Normal | StackCompletion::Break(None) => {}
        }

        // Done with all cases
        if case_index >= cases.len() {
            state.push_value(Guarded::unguarded(JsValue::Undefined));
            return StepResult::Continue;
        }

        let case = match cases.get(case_index) {
            Some(c) => c,
            None => {
                state.push_value(Guarded::unguarded(JsValue::Undefined));
                return StepResult::Continue;
            }
        };

        // Done with this case's statements, fall through to next case
        if stmt_index >= case.consequent.len() {
            state.push_frame(Frame::SwitchBody {
                discriminant,
                cases,
                case_index: case_index + 1,
                stmt_index: 0,
            });
            return StepResult::Continue;
        }

        // Get current statement
        let stmt = match case.consequent.get(stmt_index) {
            Some(s) => s.clone(),
            None => {
                state.push_frame(Frame::SwitchBody {
                    discriminant,
                    cases,
                    case_index: case_index + 1,
                    stmt_index: 0,
                });
                return StepResult::Continue;
            }
        };

        // Push continuation for next statement
        state.push_frame(Frame::SwitchBody {
            discriminant,
            cases,
            case_index,
            stmt_index: stmt_index + 1,
        });

        // Execute current statement
        state.push_frame(Frame::Stmt(Rc::new(stmt)));

        StepResult::Continue
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Try/Catch/Finally Statement Implementation
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Setup try statement - execute try block with error handling context
    fn setup_try(
        &mut self,
        state: &mut ExecutionState,
        try_stmt: &crate::ast::TryStatement,
    ) -> StepResult {
        let saved_env = self.env.clone();

        // Push the try block frame - it will handle errors
        state.push_frame(Frame::TryBlock {
            handler: try_stmt.handler.as_ref().map(|h| Rc::new(h.clone())),
            finalizer: try_stmt.finalizer.as_ref().map(|f| Rc::new(f.clone())),
            body: Rc::new(try_stmt.block.clone()),
        });

        // Execute try block body
        state.push_frame(Frame::Block {
            statements: Rc::clone(&try_stmt.block.body),
            index: 0,
        });

        // Save environment for potential catch
        // Note: We clone saved_env for potential use in catch
        let _ = saved_env;

        StepResult::Continue
    }

    /// Step for try block completion - handle normal completion (errors are caught in run())
    fn step_try_block(
        &mut self,
        state: &mut ExecutionState,
        _handler: Option<Rc<crate::ast::CatchClause>>,
        finalizer: Option<Rc<crate::ast::BlockStatement>>,
        _body: Rc<crate::ast::BlockStatement>,
    ) -> StepResult {
        // Get result from try block
        let result = state.pop_value().map(|g| g.value);

        // Normal completion (or return/break/continue) - errors are handled in run()
        let saved_completion = state.completion.clone();

        if let Some(finally_block) = finalizer {
            // Run finally block
            state.push_frame(Frame::FinallyBlock(Box::new(FinallyBlockData {
                saved_result: result,
                saved_error: None,
                saved_completion,
            })));
            state.completion = StackCompletion::Normal;
            state.push_frame(Frame::Block {
                statements: Rc::clone(&finally_block.body),
                index: 0,
            });
        } else {
            // No finally, just continue with current completion
            if let Some(val) = result {
                state.push_value(Guarded::unguarded(val));
            } else {
                state.push_value(Guarded::unguarded(JsValue::Undefined));
            }
        }

        StepResult::Continue
    }

    /// Step for catch block completion - restore env and run finally
    fn step_catch_block(
        &mut self,
        state: &mut ExecutionState,
        finalizer: Option<Rc<crate::ast::BlockStatement>>,
        saved_env: Gc<JsObject>,
    ) -> StepResult {
        // Pop catch scope guard and restore environment
        self.pop_env_guard();
        self.env = saved_env;

        // Get catch result
        let result = state.pop_value().map(|g| g.value);
        let saved_completion = state.completion.clone();

        if let Some(finally_block) = finalizer {
            // Run finally
            state.push_frame(Frame::FinallyBlock(Box::new(FinallyBlockData {
                saved_result: result,
                saved_error: None,
                saved_completion,
            })));
            state.completion = StackCompletion::Normal;
            state.push_frame(Frame::Block {
                statements: Rc::clone(&finally_block.body),
                index: 0,
            });
        } else {
            // No finally, continue
            if let Some(val) = result {
                state.push_value(Guarded::unguarded(val));
            } else {
                state.push_value(Guarded::unguarded(JsValue::Undefined));
            }
        }

        StepResult::Continue
    }

    /// Step for finally block completion - restore original completion/error
    fn step_finally_block(
        &mut self,
        state: &mut ExecutionState,
        saved_result: Option<JsValue>,
        saved_error: Option<JsError>,
        saved_completion: StackCompletion,
    ) -> StepResult {
        // Pop finally result (we don't use it unless it threw)
        let _ = state.pop_value();

        // Check if finally threw
        if matches!(state.completion, StackCompletion::Throw) {
            // Finally threw - use its error
            return StepResult::Continue;
        }

        // Restore original completion
        state.completion = saved_completion;

        // Re-throw original error if there was one
        if let Some(error) = saved_error {
            return StepResult::Error(error);
        }

        // Otherwise restore the saved result
        if let Some(val) = saved_result {
            state.push_value(Guarded::unguarded(val));
        } else {
            state.push_value(Guarded::unguarded(JsValue::Undefined));
        }

        StepResult::Continue
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Statement Execution Helpers
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Execute function declaration - creates and binds function to environment
    fn stack_execute_function_declaration(
        &mut self,
        func: &crate::ast::FunctionDeclaration,
    ) -> Result<(), JsError> {
        let name = func.id.as_ref().map(|id| id.name.cheap_clone());
        let params = func.params.clone();
        let body = Rc::new(crate::value::FunctionBody::Block(func.body.clone()));

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

        Ok(())
    }

    /// Execute import declaration - binds imported names to environment
    fn stack_execute_import(
        &mut self,
        import: &crate::ast::ImportDeclaration,
    ) -> Result<(), JsError> {
        let specifier = import.source.value.to_string();

        // Resolve the module
        let module_obj = self.resolve_module(&specifier)?;

        // Bind imported names to current environment
        for spec in &import.specifiers {
            match spec {
                crate::ast::ImportSpecifier::Named {
                    local, imported, ..
                } => {
                    let import_key = self.key(imported.name.as_str());
                    let value = module_obj
                        .borrow()
                        .get_property(&import_key)
                        .unwrap_or(JsValue::Undefined);
                    self.env_define(local.name.cheap_clone(), value, false);
                }
                crate::ast::ImportSpecifier::Default { local, .. } => {
                    let default_key = self.key("default");
                    let value = module_obj
                        .borrow()
                        .get_property(&default_key)
                        .unwrap_or(JsValue::Undefined);
                    self.env_define(local.name.cheap_clone(), value, false);
                }
                crate::ast::ImportSpecifier::Namespace { local, .. } => {
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

    /// Execute export declaration - registers exported values
    fn stack_execute_export(
        &mut self,
        export: &crate::ast::ExportDeclaration,
    ) -> Result<(), JsError> {
        // Handle export declaration (e.g., export function foo() {})
        if let Some(decl) = &export.declaration {
            // Execute the declaration first
            match decl.as_ref() {
                Statement::FunctionDeclaration(func) => {
                    self.stack_execute_function_declaration(func)?;
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
                    self.execute_variable_declaration(var_decl)?;
                    for declarator in var_decl.declarations.iter() {
                        if let Pattern::Identifier(id) = &declarator.id {
                            let value = self.env_get(&id.name)?;
                            self.exports.insert(id.name.cheap_clone(), value);
                        }
                    }
                }
                Statement::ClassDeclaration(class) => {
                    self.execute_class_declaration(class)?;
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
                Statement::EnumDeclaration(enum_decl) => {
                    self.execute_enum_declaration(enum_decl)?;
                    let value = self.env_get(&enum_decl.id.name)?;
                    let export_name = if export.default {
                        JsString::from("default")
                    } else {
                        enum_decl.id.name.cheap_clone()
                    };
                    self.exports.insert(export_name, value);
                }
                Statement::NamespaceDeclaration(ns_decl) => {
                    self.execute_namespace_declaration(ns_decl)?;
                    let value = self.env_get(&ns_decl.id.name)?;
                    let export_name = if export.default {
                        JsString::from("default")
                    } else {
                        ns_decl.id.name.cheap_clone()
                    };
                    self.exports.insert(export_name, value);
                }
                // TypeScript-only declarations - no runtime effect
                Statement::InterfaceDeclaration(_) | Statement::TypeAlias(_) => {}
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
}
