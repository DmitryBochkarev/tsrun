//! Stack-based evaluation for suspendable execution
//!
//! This module implements a trampolined interpreter that can suspend
//! at await points and resume later with a value.

use crate::ast::{
    Argument, ArrayElement, AssignmentOp, AssignmentTarget, BinaryOp, BlockStatement, Expression,
    ForInOfLeft, LiteralValue, LogicalOp, MemberProperty, ObjectProperty, Pattern, Statement,
    UnaryOp, UpdateOp, VariableKind,
};
use crate::error::JsError;
use crate::gc::Gc;
use crate::value::{CheapClone, ExoticObject, Guarded, JsObject, JsString, JsValue, PromiseStatus};
use std::rc::Rc;

use super::Interpreter;

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
    // Expression Evaluation
    // ═══════════════════════════════════════════════════════════════════════
    /// Evaluate an expression
    Expr(Rc<Expression>),

    /// Binary: left done, evaluate right then complete
    BinaryRight {
        op: BinaryOp,
        right: Rc<Expression>,
    },

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

    /// Member access: object done, access property
    MemberAccess {
        property: MemberProperty,
        computed: bool,
        optional: bool,
    },

    /// Computed member: object done, evaluate property
    ComputedMemberEval {
        property: Rc<Expression>,
        optional: bool,
    },

    /// Computed member: both done, access
    ComputedMemberComplete { optional: bool },

    /// Call: callee done, evaluate args
    CallArgs {
        args: Vec<Argument>,
        args_done: Vec<JsValue>,
        this_value: JsValue,
        optional: bool,
    },

    /// Call: one arg done, continue with rest
    CallArgNext {
        args: Vec<Argument>,
        args_done: Vec<JsValue>,
        this_value: JsValue,
        optional: bool,
    },

    /// Call: all args done, execute
    CallExecute { this_value: JsValue },

    /// New: constructor done, evaluate args
    NewArgs {
        args: Vec<Argument>,
        args_done: Vec<JsValue>,
    },

    /// New: one arg done, continue
    NewArgNext {
        args: Vec<Argument>,
        args_done: Vec<JsValue>,
    },

    /// New: ready to construct
    NewExecute,

    /// Array literal: evaluate elements
    ArrayElements {
        elements: Vec<Option<ArrayElement>>,
        done: Vec<JsValue>,
    },

    /// Array: one element done
    ArrayElementNext {
        elements: Vec<Option<ArrayElement>>,
        done: Vec<JsValue>,
    },

    /// Object literal: evaluate properties
    ObjectProperties {
        properties: Vec<ObjectProperty>,
        obj: Gc<JsObject>,
    },

    /// Object: value done, continue with rest
    ObjectPropertyNext {
        properties: Vec<ObjectProperty>,
        obj: Gc<JsObject>,
    },

    /// Assignment: target reference captured, evaluate value
    AssignmentValue {
        target: AssignmentTarget,
        op: AssignmentOp,
    },

    /// Assignment: value done, perform assignment
    AssignmentComplete {
        target: AssignmentTarget,
        op: AssignmentOp,
    },

    /// Update (++/--): evaluate target
    UpdateComplete {
        op: UpdateOp,
        prefix: bool,
        target: AssignmentTarget,
    },

    /// Sequence: one done, continue with rest
    SequenceNext { remaining: Vec<Expression> },

    /// Await: promise evaluated, check state
    AwaitCheck,

    // ═══════════════════════════════════════════════════════════════════════
    // Statement Execution
    // ═══════════════════════════════════════════════════════════════════════
    /// Execute a statement
    Stmt(Rc<Statement>),

    /// Execute remaining statements in block
    Block {
        statements: Vec<Statement>,
        index: usize,
    },

    /// Expression statement: discard result
    ExprStmtComplete,

    /// Variable declaration: init done, bind
    VarBind {
        pattern: Pattern,
        kind: VariableKind,
    },

    /// Multiple declarators
    VarDeclarators {
        declarators: Vec<(Pattern, Option<Expression>)>,
        index: usize,
        kind: VariableKind,
    },

    /// If: condition done, pick branch
    IfBranch {
        consequent: Box<Statement>,
        alternate: Option<Box<Statement>>,
    },

    /// While: test done, maybe enter body
    WhileBody {
        test: Rc<Expression>,
        body: Box<Statement>,
        label: Option<JsString>,
    },

    /// While: body done, loop back
    WhileLoop {
        test: Rc<Expression>,
        body: Box<Statement>,
        label: Option<JsString>,
    },

    /// Do-while: body done, test
    DoWhileTest {
        test: Rc<Expression>,
        body: Box<Statement>,
        label: Option<JsString>,
    },

    /// Do-while: test done, maybe loop
    DoWhileLoop {
        test: Rc<Expression>,
        body: Box<Statement>,
        label: Option<JsString>,
    },

    /// For: init done, test
    ForTest {
        test: Option<Box<Expression>>,
        update: Option<Box<Expression>>,
        body: Box<Statement>,
        label: Option<JsString>,
    },

    /// For: test done, maybe body
    ForBody {
        test: Option<Box<Expression>>,
        update: Option<Box<Expression>>,
        body: Box<Statement>,
        label: Option<JsString>,
    },

    /// For: body done, update
    ForUpdate {
        test: Option<Box<Expression>>,
        update: Option<Box<Expression>>,
        body: Box<Statement>,
        label: Option<JsString>,
    },

    /// For: update done, loop back
    ForLoop {
        test: Option<Box<Expression>>,
        update: Option<Box<Expression>>,
        body: Box<Statement>,
        label: Option<JsString>,
    },

    /// For-in/of: iterator setup
    ForInOf {
        left: ForInOfLeft,
        body: Box<Statement>,
        label: Option<JsString>,
        items: Vec<JsValue>,
        index: usize,
        is_of: bool,
    },

    /// For-in/of: body done, next iteration
    ForInOfNext {
        left: ForInOfLeft,
        body: Box<Statement>,
        label: Option<JsString>,
        items: Vec<JsValue>,
        index: usize,
        is_of: bool,
    },

    /// Return: value done
    ReturnComplete,

    /// Throw: value done
    ThrowComplete,

    /// Try: body done (normal or error)
    TryCatch {
        catch_param: Option<Pattern>,
        catch_body: Option<BlockStatement>,
        finally_block: Option<BlockStatement>,
    },

    /// Try: catch done, run finally
    TryFinally {
        finally_block: BlockStatement,
        completion: StackCompletion,
    },

    /// Switch: discriminant done, match cases
    SwitchCases {
        cases: Vec<(Option<Expression>, Vec<Statement>)>,
        default_index: Option<usize>,
        label: Option<JsString>,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // Function Execution
    // ═══════════════════════════════════════════════════════════════════════
    /// Async function: wrap result in promise
    AsyncComplete { is_error: bool },
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
    pub fn peek_value(&self) -> Option<&Guarded> {
        self.values.last()
    }

    /// Check if we have more frames
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
    /// Execute one step of the stack machine
    pub fn step(&mut self, state: &mut ExecutionState) -> StepResult {
        let Some(frame) = state.pop_frame() else {
            // No more frames - return the top value
            return match state.pop_value() {
                Some(v) => StepResult::Done(v),
                None => StepResult::Done(Guarded::unguarded(JsValue::Undefined)),
            };
        };

        match frame {
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

            // For unimplemented frames, return error
            _ => StepResult::Error(JsError::internal_error("Unimplemented frame type")),
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

    /// Step for expression evaluation
    fn step_expr(&mut self, state: &mut ExecutionState, expr: &Expression) -> StepResult {
        match expr {
            Expression::Literal(lit) => {
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

            // Complex operations - delegate to evaluate_expression would need the AST
            // For now, return error since these need object access
            BinaryOp::Instanceof | BinaryOp::In => {
                return Err(JsError::internal_error(
                    "instanceof/in not implemented in stack mode",
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
        // TODO: Proper error handling through try/catch frames
        StepResult::Error(error)
    }
}
