//! Stack-based evaluation for suspendable execution
//!
//! This module implements a trampolined interpreter that can suspend
//! at await points and resume later with a value.

use crate::ast::{
    BinaryOp, Expression, ForInit, ForStatement, LiteralValue, LogicalOp, Pattern, Program,
    Statement, UnaryOp, VariableDeclarator, VariableKind,
};
use crate::error::JsError;
use crate::gc::Gc;
use crate::value::{
    Binding, CheapClone, ExoticObject, Guarded, JsObject, JsString, JsValue, PromiseStatus,
};
use std::rc::Rc;

use super::{create_environment_with_guard, Completion, Interpreter};

// ═══════════════════════════════════════════════════════════════════════════════
// Stack Types
// ═══════════════════════════════════════════════════════════════════════════════

/// A value on the value stack, wrapped in Guarded to maintain GC safety
pub type StackValue = Guarded;

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
        statements: Rc<Vec<Statement>>,
        index: usize,
    },

    /// Execute a single statement
    Stmt(Rc<Statement>),

    /// Statement completed, check completion type
    StmtComplete,

    /// Execute remaining statements in block
    Block {
        statements: Rc<Vec<Statement>>,
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
        declarators: Rc<Vec<VariableDeclarator>>,
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
        label: Option<JsString>,
    },

    /// While check: test evaluated, execute body or exit
    WhileCheck {
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: Option<JsString>,
    },

    /// Do-while loop: execute body first, then check condition
    DoWhileLoop {
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: Option<JsString>,
    },

    /// Do-while check: test evaluated, loop or exit
    DoWhileCheck {
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: Option<JsString>,
    },

    /// Do-while test check: after condition evaluated, decide to loop or exit
    DoWhileTestCheck {
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: Option<JsString>,
    },

    /// For loop: full state for iteration
    ForLoop {
        test: Option<Rc<Expression>>,
        update: Option<Rc<Expression>>,
        body: Rc<Statement>,
        label: Option<JsString>,
        /// Variables for per-iteration binding (name, mutable)
        loop_vars: Rc<Vec<(JsString, bool)>>,
        /// Saved environment to restore after loop
        saved_env: Gc<JsObject>,
    },

    /// For loop test check: condition evaluated, decide to continue or exit
    ForTestCheck {
        test: Option<Rc<Expression>>,
        update: Option<Rc<Expression>>,
        body: Rc<Statement>,
        label: Option<JsString>,
        loop_vars: Rc<Vec<(JsString, bool)>>,
        saved_env: Gc<JsObject>,
    },

    /// For loop after body: handle control flow, create per-iteration env, run update
    ForAfterBody {
        test: Option<Rc<Expression>>,
        update: Option<Rc<Expression>>,
        body: Rc<Statement>,
        label: Option<JsString>,
        loop_vars: Rc<Vec<(JsString, bool)>>,
        saved_env: Gc<JsObject>,
    },

    /// For loop cleanup: restore environment after loop exits
    ForCleanup { saved_env: Gc<JsObject> },

    /// Discard expression result (for init expressions, update expressions)
    DiscardValue,

    /// Push scope before for loop body
    PushScope {
        /// Continuation frame to push after scope is created
        next: Box<Frame>,
    },
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

impl ExecutionState {
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            values: Vec::new(),
            completion: StackCompletion::Normal,
            waiting_on: None,
        }
    }

    /// Create state for executing a program
    pub fn for_program(program: &Program) -> Self {
        let mut state = Self::new();
        state.push_frame(Frame::Program {
            statements: Rc::new(program.body.clone()),
            index: 0,
        });
        state
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

    /// Peek at top value
    #[allow(dead_code)]
    pub fn peek_value(&self) -> Option<&Guarded> {
        self.values.last()
    }

    /// Check if we have more frames
    #[allow(dead_code)]
    pub fn has_frames(&self) -> bool {
        !self.frames.is_empty()
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
            } => self.step_for_loop(state, test, update, body, label, loop_vars, saved_env),

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
                state.push_value(Guarded::unguarded(JsValue::Undefined));
                StepResult::Continue
            }

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
        }
    }

    /// Run until completion or suspension
    pub fn run(&mut self, state: &mut ExecutionState) -> StepResult {
        loop {
            match self.step(state) {
                StepResult::Continue => continue,
                result => return result,
            }
        }
    }

    /// Step for program execution
    fn step_program(
        &mut self,
        state: &mut ExecutionState,
        statements: Rc<Vec<Statement>>,
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
        statements: Rc<Vec<Statement>>,
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
                state.push_frame(Frame::Expr(Rc::new(expr_stmt.expression.clone())));
                StepResult::Continue
            }

            Statement::Block(block) => {
                // Execute block
                state.push_frame(Frame::Block {
                    statements: Rc::new(block.body.clone()),
                    index: 0,
                });
                StepResult::Continue
            }

            Statement::Return(ret) => {
                state.completion = StackCompletion::Return;
                if let Some(expr) = &ret.argument {
                    state.push_frame(Frame::Expr(Rc::new(expr.clone())));
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
                        declarators: Rc::new(decl.declarations.clone()),
                        index: 0,
                        mutable,
                    });
                    StepResult::Continue
                }
            }

            Statement::If(if_stmt) => {
                // Evaluate condition, then branch
                state.push_frame(Frame::IfBranch {
                    consequent: Rc::new((*if_stmt.consequent).clone()),
                    alternate: if_stmt.alternate.as_ref().map(|a| Rc::new((**a).clone())),
                });
                state.push_frame(Frame::Expr(Rc::new(if_stmt.test.clone())));
                StepResult::Continue
            }

            Statement::While(while_stmt) => {
                state.push_frame(Frame::WhileLoop {
                    test: Rc::new(while_stmt.test.clone()),
                    body: Rc::new((*while_stmt.body).clone()),
                    label: None,
                });
                StepResult::Continue
            }

            Statement::DoWhile(do_while) => {
                state.push_frame(Frame::DoWhileLoop {
                    test: Rc::new(do_while.test.clone()),
                    body: Rc::new((*do_while.body).clone()),
                    label: None,
                });
                StepResult::Continue
            }

            Statement::For(for_stmt) => self.setup_for_loop(state, for_stmt, None),

            // For complex statements, delegate to recursive execution
            _ => match self.execute_statement(stmt) {
                Ok(completion) => {
                    match completion {
                        Completion::Normal(val) => {
                            state.push_value(Guarded::unguarded(val));
                            state.completion = StackCompletion::Normal;
                        }
                        Completion::Return(val) => {
                            state.push_value(Guarded::unguarded(val));
                            state.completion = StackCompletion::Return;
                        }
                        Completion::Break(label) => {
                            state.push_value(Guarded::unguarded(JsValue::Undefined));
                            state.completion = StackCompletion::Break(label);
                        }
                        Completion::Continue(label) => {
                            state.push_value(Guarded::unguarded(JsValue::Undefined));
                            state.completion = StackCompletion::Continue(label);
                        }
                    }
                    StepResult::Continue
                }
                Err(e) => StepResult::Error(e),
            },
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
                    right: Rc::new((*bin.right).clone()),
                });
                state.push_frame(Frame::Expr(Rc::new((*bin.left).clone())));
                StepResult::Continue
            }

            Expression::Logical(log) => {
                // Evaluate left, then check for short-circuit
                state.push_frame(Frame::LogicalCheck {
                    op: log.operator,
                    right: Rc::new((*log.right).clone()),
                });
                state.push_frame(Frame::Expr(Rc::new((*log.left).clone())));
                StepResult::Continue
            }

            Expression::Unary(un) => {
                state.push_frame(Frame::UnaryComplete { op: un.operator });
                state.push_frame(Frame::Expr(Rc::new((*un.argument).clone())));
                StepResult::Continue
            }

            Expression::Conditional(cond) => {
                state.push_frame(Frame::ConditionalBranch {
                    consequent: Rc::new((*cond.consequent).clone()),
                    alternate: Rc::new((*cond.alternate).clone()),
                });
                state.push_frame(Frame::Expr(Rc::new((*cond.test).clone())));
                StepResult::Continue
            }

            Expression::Await(await_expr) => {
                // Evaluate the argument, then check if it's a promise
                state.push_frame(Frame::AwaitCheck);
                state.push_frame(Frame::Expr(Rc::new((*await_expr.argument).clone())));
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
        let promise_val = state
            .pop_value()
            .map(|v| v.value)
            .unwrap_or(JsValue::Undefined);

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
                state.push_value(Guarded::unguarded(value));
                StepResult::Continue
            }
            PromiseStatus::Rejected => {
                let reason = result.unwrap_or(JsValue::Undefined);
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
        declarators: Rc<Vec<VariableDeclarator>>,
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
                state.push_frame(Frame::Expr(Rc::new(expr.clone())));
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
        label: Option<JsString>,
    ) -> StepResult {
        // Check for break/continue from previous body execution
        match &state.completion {
            StackCompletion::Break(brk_label) => {
                // Check if break targets this loop
                if brk_label.is_none() || brk_label.as_ref() == label.as_ref() {
                    state.completion = StackCompletion::Normal;
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                    return StepResult::Continue;
                }
                // Break targets outer loop - propagate
                return StepResult::Continue;
            }
            StackCompletion::Continue(cont_label) => {
                // Check if continue targets this loop
                if cont_label.is_none() || cont_label.as_ref() == label.as_ref() {
                    state.completion = StackCompletion::Normal;
                    // Continue to next iteration - don't check condition value, restart loop
                    state.push_frame(Frame::WhileLoop { test, body, label });
                    return StepResult::Continue;
                }
                // Continue targets outer loop - propagate
                return StepResult::Continue;
            }
            StackCompletion::Return | StackCompletion::Throw => {
                // Return/throw - propagate up
                return StepResult::Continue;
            }
            StackCompletion::Normal => {}
        }

        // Get condition value
        let condition = state
            .pop_value()
            .map(|v| v.value)
            .unwrap_or(JsValue::Undefined);

        if condition.to_boolean() {
            // Continue loop: check condition again after body
            state.push_frame(Frame::WhileCheck {
                test: test.clone(),
                body: body.clone(),
                label: label.clone(),
            });
            state.push_frame(Frame::Expr(test));
            // Execute body
            state.push_frame(Frame::Stmt(body));
        } else {
            // Exit loop
            state.push_value(Guarded::unguarded(JsValue::Undefined));
        }
        StepResult::Continue
    }

    /// Step for do-while check - body executed, test evaluated, loop or exit
    fn step_do_while_check(
        &mut self,
        state: &mut ExecutionState,
        test: Rc<Expression>,
        body: Rc<Statement>,
        label: Option<JsString>,
    ) -> StepResult {
        // Check for break/continue from body execution
        match &state.completion {
            StackCompletion::Break(brk_label) => {
                if brk_label.is_none() || brk_label.as_ref() == label.as_ref() {
                    state.completion = StackCompletion::Normal;
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                    return StepResult::Continue;
                }
                return StepResult::Continue;
            }
            StackCompletion::Continue(cont_label) => {
                if cont_label.is_none() || cont_label.as_ref() == label.as_ref() {
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
        label: Option<JsString>,
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
        label: Option<JsString>,
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

        let test = for_stmt.test.as_ref().map(|t| Rc::new(t.clone()));
        let update = for_stmt.update.as_ref().map(|u| Rc::new(u.clone()));
        let body = Rc::new((*for_stmt.body).clone());
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
        });

        // Handle init
        match &for_stmt.init {
            Some(ForInit::Variable(decl)) if decl.kind != VariableKind::Var => {
                // let/const - execute in loop scope
                state.push_frame(Frame::VarDecl {
                    declarators: Rc::new(decl.declarations.clone()),
                    index: 0,
                    mutable: decl.kind == VariableKind::Let,
                });
            }
            Some(ForInit::Expression(expr)) => {
                // Expression init - discard result
                state.push_frame(Frame::DiscardValue);
                state.push_frame(Frame::Expr(Rc::new(expr.clone())));
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
        label: Option<JsString>,
        loop_vars: Rc<Vec<(JsString, bool)>>,
        saved_env: Gc<JsObject>,
    ) -> StepResult {
        // Create per-iteration environment if needed
        if !loop_vars.is_empty() {
            let iter_env = create_environment_with_guard(&self.root_guard, Some(saved_env.clone()));
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
        label: Option<JsString>,
        loop_vars: Rc<Vec<(JsString, bool)>>,
        saved_env: Gc<JsObject>,
    ) -> StepResult {
        let condition = state
            .pop_value()
            .map(|v| v.value)
            .unwrap_or(JsValue::Boolean(true));

        if !condition.to_boolean() {
            // Exit loop - cleanup frame already on stack
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
        label: Option<JsString>,
        loop_vars: Rc<Vec<(JsString, bool)>>,
        saved_env: Gc<JsObject>,
    ) -> StepResult {
        // Check for control flow from body
        match &state.completion {
            StackCompletion::Break(brk_label) => {
                if brk_label.is_none() || brk_label.as_ref() == label.as_ref() {
                    state.completion = StackCompletion::Normal;
                    // Exit loop - cleanup will happen
                    state.push_value(Guarded::unguarded(JsValue::Undefined));
                    return StepResult::Continue;
                }
                // Break targets outer loop - propagate
                return StepResult::Continue;
            }
            StackCompletion::Continue(cont_label) => {
                if cont_label.is_none() || cont_label.as_ref() == label.as_ref() {
                    state.completion = StackCompletion::Normal;
                    // Continue to update phase (fall through)
                } else {
                    // Continue targets outer loop - propagate
                    return StepResult::Continue;
                }
            }
            StackCompletion::Return | StackCompletion::Throw => {
                // Return/throw - propagate up (cleanup will happen)
                return StepResult::Continue;
            }
            StackCompletion::Normal => {}
        }

        // Pop body result
        let _ = state.pop_value();

        // Create new per-iteration environment BEFORE update (ES spec)
        if !loop_vars.is_empty() {
            let current_env = self.env.clone();
            let new_iter_env =
                create_environment_with_guard(&self.root_guard, Some(saved_env.clone()));
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
        });

        // Execute update if present
        if let Some(upd) = update {
            state.push_frame(Frame::DiscardValue);
            state.push_frame(Frame::Expr(upd));
        }

        StepResult::Continue
    }
}
