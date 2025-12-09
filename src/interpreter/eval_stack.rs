//! Evaluation stack for suspendable execution
//!
//! This module provides the explicit evaluation stack that enables
//! suspension at import/await points with true state capture.

use crate::ast::{
    AssignmentOp, AssignmentTarget, BinaryOp, BlockStatement, Expression, LogicalOp, Pattern,
    Statement, UnaryOp, VariableKind,
};
use crate::value::{Environment, JsValue, PropertyKey};

/// A frame on the evaluation stack
///
/// Each frame represents a pending operation. The interpreter processes
/// frames in LIFO order, pushing new frames when sub-expressions need
/// to be evaluated.
#[derive(Debug, Clone)]
pub enum EvalFrame {
    // ═══════════════════════════════════════════════════════════════
    // Program Execution
    // ═══════════════════════════════════════════════════════════════
    /// Execute the program's statements
    ExecuteProgram {
        statements: Vec<Statement>,
        index: usize,
    },

    // ═══════════════════════════════════════════════════════════════
    // Expression Evaluation Frames
    // ═══════════════════════════════════════════════════════════════
    /// Evaluate an expression and push result to value stack
    EvaluateExpr(Box<Expression>),

    /// Binary expression: left evaluated, need right
    BinaryRight {
        op: BinaryOp,
        right: Box<Expression>,
    },

    /// Binary expression: both sides evaluated, compute result
    BinaryComplete { op: BinaryOp },

    /// Unary expression: operand evaluated, apply operator
    UnaryComplete { op: UnaryOp },

    /// Logical expression: left evaluated, may short-circuit
    LogicalRight {
        op: LogicalOp,
        right: Box<Expression>,
    },

    /// Conditional: condition evaluated, pick branch
    ConditionalBranch {
        consequent: Box<Expression>,
        alternate: Box<Expression>,
    },

    /// Member access: object evaluated, access property
    MemberAccess { property: String, optional: bool },

    /// Computed member: object evaluated, need property expression
    ComputedMemberExpr {
        property: Box<Expression>,
        optional: bool,
    },

    /// Computed member: both evaluated
    ComputedMemberComplete { optional: bool },

    /// Call expression: callee evaluated, evaluate args one by one
    CallArgs {
        this_value: Option<JsValue>,
        args_remaining: Vec<Expression>,
        args_done: Vec<JsValue>,
        optional: bool,
    },

    /// Call expression: all args evaluated, execute call
    CallExecute {
        this_value: Option<JsValue>,
        args: Vec<JsValue>,
    },

    /// New expression: constructor evaluated, evaluate args
    NewArgs {
        args_remaining: Vec<Expression>,
        args_done: Vec<JsValue>,
    },

    /// New expression: ready to construct
    NewExecute { args: Vec<JsValue> },

    /// Array literal: evaluate remaining elements
    ArrayElements {
        elements_remaining: Vec<Option<Expression>>,
        elements_done: Vec<JsValue>,
    },

    /// Object literal: evaluate remaining properties
    ObjectProperties {
        properties_remaining: Vec<(PropertyKey, Expression)>,
        properties_done: Vec<(PropertyKey, JsValue)>,
    },

    /// Assignment: evaluate target, then value
    AssignmentValue {
        target: AssignmentTarget,
        op: AssignmentOp,
        value: Box<Expression>,
    },

    /// Assignment: value evaluated, perform assignment
    AssignmentComplete {
        target: AssignmentTarget,
        op: AssignmentOp,
    },

    /// Sequence: evaluate remaining expressions
    SequenceNext { remaining: Vec<Expression> },

    /// Update (++/--): operand evaluated
    UpdateComplete {
        op: crate::ast::UpdateOp,
        prefix: bool,
        target: AssignmentTarget,
    },

    // ═══════════════════════════════════════════════════════════════
    // Statement Execution Frames
    // ═══════════════════════════════════════════════════════════════
    /// Execute a statement
    ExecuteStmt(Box<Statement>),

    /// Execute remaining statements in a block
    ExecuteBlock {
        statements: Vec<Statement>,
        index: usize,
        saved_env: Option<Environment>,
    },

    /// Variable declaration: initializer evaluated, bind pattern
    VariableBind {
        pattern: Pattern,
        kind: VariableKind,
    },

    /// Multiple variable declarators
    VariableDeclarators {
        declarators: Vec<(Pattern, Option<Expression>)>,
        index: usize,
        kind: VariableKind,
    },

    /// If statement: condition evaluated, pick branch
    IfBranch {
        consequent: Box<Statement>,
        alternate: Option<Box<Statement>>,
    },

    /// For loop states
    ForLoopTest {
        test: Option<Box<Expression>>,
        update: Option<Box<Expression>>,
        body: Box<Statement>,
        label: Option<String>,
    },
    ForLoopBody {
        test: Option<Box<Expression>>,
        update: Option<Box<Expression>>,
        body: Box<Statement>,
        label: Option<String>,
    },
    ForLoopUpdate {
        test: Option<Box<Expression>>,
        update: Option<Box<Expression>>,
        body: Box<Statement>,
        label: Option<String>,
    },

    /// While loop
    WhileTest {
        test: Box<Expression>,
        body: Box<Statement>,
        label: Option<String>,
    },
    WhileBody {
        test: Box<Expression>,
        body: Box<Statement>,
        label: Option<String>,
    },

    /// Do-while loop
    DoWhileBody {
        test: Box<Expression>,
        body: Box<Statement>,
        label: Option<String>,
    },
    DoWhileTest {
        test: Box<Expression>,
        body: Box<Statement>,
        label: Option<String>,
    },

    /// Try/catch/finally handling
    TryCatch {
        catch_param: Option<Pattern>,
        catch_body: Option<BlockStatement>,
        finally_block: Option<BlockStatement>,
    },

    /// Finally block execution
    FinallyBlock {
        block: BlockStatement,
        saved_completion: CompletionValue,
    },

    /// Return statement: value evaluated
    ReturnComplete,

    /// Throw statement: value evaluated
    ThrowComplete,

    // ═══════════════════════════════════════════════════════════════
    // Function Execution Frames
    // ═══════════════════════════════════════════════════════════════
    /// Function body execution complete, restore environment
    FunctionTeardown { saved_env: Environment },

    // ═══════════════════════════════════════════════════════════════
    // Import/Await Frames (for future use)
    // ═══════════════════════════════════════════════════════════════
    /// Import: waiting for module, then bind
    ImportBind {
        slot_id: u64,
        bindings: ImportBindings,
    },

    /// Await resume: slot filled, continue with value or throw
    AwaitResume { slot_id: u64 },
}

/// Completion value for control flow tracking
#[derive(Debug, Clone)]
pub enum CompletionValue {
    Normal(JsValue),
    Return(JsValue),
    Throw(JsValue),
    Break(Option<String>),
    Continue(Option<String>),
}

/// Import bindings from an import declaration
#[derive(Debug, Clone)]
pub enum ImportBindings {
    /// import { a, b as c } from "mod"
    Named(Vec<(String, String)>), // (imported, local)
    /// import def from "mod"
    Default(String),
    /// import * as ns from "mod"
    Namespace(String),
    /// import "mod" (side-effect only)
    SideEffect,
}
