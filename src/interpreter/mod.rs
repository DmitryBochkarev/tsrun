//! Interpreter for executing TypeScript AST
//!
//! This module implements a minimal interpreter using the new guard-based GC.

// Old implementation kept for reference in old_mod.rs (not compiled)
// mod old_mod;

// Builtin function implementations (disabled - needs migration to new GC)
// pub mod builtins;

// Evaluation stack for suspendable execution (disabled - needs migration)
// pub mod eval_stack;

use crate::ast::{
    ArrayElement, AssignmentExpression, AssignmentOp, AssignmentTarget, BinaryExpression, BinaryOp,
    BlockStatement, CallExpression, ConditionalExpression, Expression, ForInit, ForStatement,
    FunctionDeclaration, FunctionParam, IfStatement, LiteralValue, LogicalExpression, LogicalOp,
    MemberExpression, MemberProperty, ObjectExpression, ObjectProperty, ObjectPropertyKey, Pattern,
    Program, Statement, UnaryExpression, UnaryOp, VariableDeclaration, VariableKind,
    WhileStatement,
};
use crate::error::JsError;
use crate::gc::{Gc, Guard, Heap};
use crate::lexer::Span;
use crate::parser::Parser;
use crate::string_dict::StringDict;
use crate::value::{
    create_environment_with_guard, Binding, CheapClone, EnvRef, EnvironmentData, ExoticObject,
    FunctionBody, InterpretedFunction, JsFunction, JsObject, JsString, JsValue, PropertyKey,
};
use rustc_hash::FxHashMap;
use std::rc::Rc;

/// Completion record for control flow
/// Control flow completion type
#[derive(Debug)]
pub enum Completion {
    Normal(JsValue),
    Return(JsValue),
    Break(Option<JsString>),
    Continue(Option<JsString>),
}

/// Result of evaluating an expression.
///
/// Contains the value and an optional guard that keeps newly created objects alive
/// until ownership is transferred to an environment or parent object.
///
/// The guard is consumed when:
/// - The value is stored in a variable (env_define establishes ownership)
/// - The value is assigned to an object property (parent.own establishes ownership)
/// - The value is returned from a function (caller takes ownership)
pub struct Guarded {
    pub value: JsValue,
    pub guard: Option<Guard<JsObject>>,
}

impl Guarded {
    /// Create a guarded value with a guard
    pub fn with_guard(value: JsValue, guard: Guard<JsObject>) -> Self {
        Self {
            value,
            guard: Some(guard),
        }
    }

    /// Create an unguarded value (for primitives or already-owned objects)
    pub fn unguarded(value: JsValue) -> Self {
        Self { value, guard: None }
    }

    /// Take just the value, dropping any guard.
    /// Use this only when ownership has been transferred elsewhere.
    pub fn take(self) -> JsValue {
        self.value
    }
}

/// A stack frame for tracking call stack
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Function name (or "<anonymous>" for anonymous functions)
    pub function_name: String,
    /// Source location if available
    pub location: Option<(u32, u32)>, // (line, column)
}

/// GC statistics for debugging and monitoring
#[derive(Debug, Clone)]
pub struct GcStats {
    pub alive_count: usize,
    pub free_count: usize,
}

/// The interpreter state
pub struct Interpreter {
    // ═══════════════════════════════════════════════════════════════════════════
    // GC Infrastructure
    // ═══════════════════════════════════════════════════════════════════════════
    /// The GC heap managing all objects
    pub heap: Heap<JsObject>,

    /// Root guard for permanent objects (prototypes, global, global_env)
    root_guard: Guard<JsObject>,

    // ═══════════════════════════════════════════════════════════════════════════
    // Global State
    // ═══════════════════════════════════════════════════════════════════════════
    /// Global object
    pub global: Gc<JsObject>,

    /// Global environment (variable bindings for global scope)
    pub global_env: EnvRef,

    /// Current execution environment
    pub env: EnvRef,

    /// String dictionary for interning strings
    pub string_dict: StringDict,

    // ═══════════════════════════════════════════════════════════════════════════
    // Prototypes (all rooted via root_guard)
    // ═══════════════════════════════════════════════════════════════════════════
    /// Object.prototype
    pub object_prototype: Gc<JsObject>,

    /// Array.prototype
    pub array_prototype: Gc<JsObject>,

    /// Function.prototype
    pub function_prototype: Gc<JsObject>,

    /// String.prototype (for string primitive methods)
    pub string_prototype: Gc<JsObject>,

    /// Number.prototype (for number primitive methods)
    pub number_prototype: Gc<JsObject>,

    // ═══════════════════════════════════════════════════════════════════════════
    // Execution State
    // ═══════════════════════════════════════════════════════════════════════════
    /// Stores thrown value during exception propagation
    thrown_value: Option<JsValue>,

    /// Exported values from the module
    pub exports: FxHashMap<JsString, JsValue>,

    /// Call stack for stack traces
    pub call_stack: Vec<StackFrame>,
}

impl Interpreter {
    /// Create a new interpreter instance
    pub fn new() -> Self {
        let heap: Heap<JsObject> = Heap::new();
        let root_guard = heap.create_guard();

        // Create prototypes (all rooted)
        let object_prototype = root_guard.alloc();
        let array_prototype = root_guard.alloc();
        let function_prototype = root_guard.alloc();
        let string_prototype = root_guard.alloc();
        let number_prototype = root_guard.alloc();

        // Set up prototype chain
        {
            let proto = object_prototype;
            array_prototype.borrow_mut().prototype = Some(proto);
            array_prototype.own(&proto, &heap);
        }
        {
            let proto = object_prototype;
            function_prototype.borrow_mut().prototype = Some(proto);
            function_prototype.own(&proto, &heap);
        }
        {
            let proto = object_prototype;
            string_prototype.borrow_mut().prototype = Some(proto);
            string_prototype.own(&proto, &heap);
        }
        {
            let proto = object_prototype;
            number_prototype.borrow_mut().prototype = Some(proto);
            number_prototype.own(&proto, &heap);
        }

        // Create global object (rooted)
        let global = root_guard.alloc();
        global.borrow_mut().prototype = Some(object_prototype);
        global.own(&object_prototype, &heap);

        // Create global environment (rooted, owned by global)
        let global_env = root_guard.alloc();
        {
            let mut env_ref = global_env.borrow_mut();
            env_ref.null_prototype = true;
            env_ref.exotic = ExoticObject::Environment(EnvironmentData::new());
        }
        global.own(&global_env, &heap);

        let string_dict = StringDict::new();

        let mut interp = Self {
            heap,
            root_guard,
            global,
            global_env,
            env: global_env,
            string_dict,
            object_prototype,
            array_prototype,
            function_prototype,
            string_prototype,
            number_prototype,
            thrown_value: None,
            exports: FxHashMap::default(),
            call_stack: Vec::new(),
        };

        // Initialize built-in globals
        interp.init_globals();

        interp
    }

    /// Initialize built-in global values
    fn init_globals(&mut self) {
        // For now, minimal globals - just define undefined and NaN
        let undefined_name = self.string_dict.get_or_insert("undefined");
        self.env_define(undefined_name, JsValue::Undefined, false);

        let nan_name = self.string_dict.get_or_insert("NaN");
        self.env_define(nan_name, JsValue::Number(f64::NAN), false);

        let infinity_name = self.string_dict.get_or_insert("Infinity");
        self.env_define(infinity_name, JsValue::Number(f64::INFINITY), false);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Environment Operations
    // ═══════════════════════════════════════════════════════════════════════════

    /// Define a variable in the current environment
    pub fn env_define(&mut self, name: JsString, value: JsValue, mutable: bool) {
        // If value is an object, env owns it
        if let JsValue::Object(ref obj) = value {
            self.env.own(obj, &self.heap);
        }

        let mut env_ref = self.env.borrow_mut();
        if let Some(data) = env_ref.as_environment_mut() {
            data.bindings.insert(
                name,
                Binding {
                    value,
                    mutable,
                    initialized: true,
                },
            );
        }
    }

    /// Get a variable from the environment chain
    pub fn env_get(&self, name: &JsString) -> Result<JsValue, JsError> {
        let mut current = Some(self.env);

        while let Some(env) = current {
            let env_ref = env.borrow();
            if let Some(data) = env_ref.as_environment() {
                if let Some(binding) = data.bindings.get(name) {
                    if !binding.initialized {
                        return Err(JsError::reference_error(format!(
                            "Cannot access '{}' before initialization",
                            name
                        )));
                    }
                    return Ok(binding.value.clone());
                }
                current = data.outer;
            } else {
                break;
            }
        }

        // Check global object properties
        let global = self.global.borrow();
        if let Some(prop) = global.get_property(&PropertyKey::String(name.cheap_clone())) {
            return Ok(prop);
        }

        Err(JsError::reference_error(format!("{} is not defined", name)))
    }

    /// Set a variable in the environment chain
    pub fn env_set(&mut self, name: &JsString, value: JsValue) -> Result<(), JsError> {
        let mut current = Some(self.env);

        while let Some(env) = current {
            let mut env_ref = env.borrow_mut();
            if let Some(data) = env_ref.as_environment_mut() {
                if let Some(binding) = data.bindings.get_mut(name) {
                    if !binding.mutable {
                        return Err(JsError::type_error(format!(
                            "Assignment to constant variable '{}'",
                            name
                        )));
                    }
                    // Update ownership
                    if let JsValue::Object(ref old_obj) = binding.value {
                        env.disown(old_obj, &self.heap);
                    }
                    if let JsValue::Object(ref new_obj) = value {
                        env.own(new_obj, &self.heap);
                    }
                    binding.value = value;
                    return Ok(());
                }
                let outer = data.outer;
                drop(env_ref);
                current = outer;
            } else {
                break;
            }
        }

        Err(JsError::reference_error(format!("{} is not defined", name)))
    }

    /// Push a new scope and return the saved environment
    pub fn push_scope(&mut self) -> EnvRef {
        let new_env = create_environment_with_guard(&self.root_guard, &self.heap, Some(self.env));

        let old_env = self.env;
        self.env = new_env;
        old_env
    }

    /// Pop scope by restoring saved environment
    pub fn pop_scope(&mut self, saved_env: EnvRef) {
        self.env = saved_env;
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Object Creation Helpers (Temporary Guard Pattern)
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // Objects are created with a temporary guard that keeps them alive until
    // ownership is transferred to a permanent owner (environment, parent object).
    // The caller MUST transfer ownership before the temp guard is dropped.
    //
    // Pattern:
    //   let (obj, _temp) = self.create_object_with_guard();
    //   parent.own(&obj, &self.heap);  // Transfer ownership
    //   // _temp is dropped here, but obj is still alive via parent
    // ═══════════════════════════════════════════════════════════════════════════

    /// Create a new plain object with a temporary guard.
    /// Returns (object, temp_guard). Caller must transfer ownership before guard is dropped.
    pub fn create_object_with_guard(&mut self) -> (Gc<JsObject>, Guard<JsObject>) {
        let temp = self.heap.create_guard();
        let obj = temp.alloc();
        obj.borrow_mut().prototype = Some(self.object_prototype);
        obj.own(&self.object_prototype, &self.heap);
        (obj, temp)
    }

    /// Create a new array with elements and a temporary guard.
    /// Returns (array, temp_guard). Caller must transfer ownership before guard is dropped.
    pub fn create_array_with_guard(
        &mut self,
        elements: Vec<JsValue>,
    ) -> (Gc<JsObject>, Guard<JsObject>) {
        let len = elements.len() as u32;
        let temp = self.heap.create_guard();
        let arr = temp.alloc();
        {
            let mut arr_ref = arr.borrow_mut();
            arr_ref.prototype = Some(self.array_prototype);
            arr_ref.exotic = ExoticObject::Array { length: len };

            for (i, elem) in elements.iter().enumerate() {
                arr_ref.set_property(PropertyKey::Index(i as u32), elem.clone());
            }

            arr_ref.set_property(
                PropertyKey::String(self.string_dict.get_or_insert("length")),
                JsValue::Number(len as f64),
            );
        }
        arr.own(&self.array_prototype, &self.heap);

        // Array owns its element objects
        for elem in &elements {
            if let JsValue::Object(elem_obj) = elem {
                arr.own(elem_obj, &self.heap);
            }
        }

        (arr, temp)
    }

    /// Create a function object with a temporary guard.
    /// Returns (function, temp_guard). Caller must transfer ownership before guard is dropped.
    fn create_function_with_guard(
        &mut self,
        name: Option<JsString>,
        params: Rc<[FunctionParam]>,
        body: Rc<FunctionBody>,
        closure: EnvRef,
        span: Span,
        generator: bool,
        async_: bool,
    ) -> (Gc<JsObject>, Guard<JsObject>) {
        let temp = self.heap.create_guard();
        let func_obj = temp.alloc();
        {
            let mut f_ref = func_obj.borrow_mut();
            f_ref.prototype = Some(self.function_prototype);
            f_ref.exotic = ExoticObject::Function(JsFunction::Interpreted(InterpretedFunction {
                name,
                params,
                body,
                closure,
                source_location: span,
                generator,
                async_,
            }));
        }
        func_obj.own(&self.function_prototype, &self.heap);
        func_obj.own(&closure, &self.heap);
        (func_obj, temp)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Evaluation Entry Point
    // ═══════════════════════════════════════════════════════════════════════════

    /// Evaluate source code and return the result
    pub fn eval_simple(&mut self, source: &str) -> Result<JsValue, JsError> {
        let mut parser = Parser::new(source, &mut self.string_dict);
        let program = parser.parse_program()?;
        self.execute_program(&program)
    }

    /// Execute a program
    fn execute_program(&mut self, program: &Program) -> Result<JsValue, JsError> {
        let mut result = JsValue::Undefined;

        for stmt in &program.body {
            match self.execute_statement(stmt)? {
                Completion::Normal(val) => result = val,
                Completion::Return(val) => return Ok(val),
                Completion::Break(_) | Completion::Continue(_) => {
                    return Err(JsError::syntax_error_simple(
                        "Illegal break/continue statement",
                    ));
                }
            }
        }

        Ok(result)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Statement Execution
    // ═══════════════════════════════════════════════════════════════════════════

    fn execute_statement(&mut self, stmt: &Statement) -> Result<Completion, JsError> {
        match stmt {
            Statement::Expression(expr_stmt) => {
                // Expression statement - value is discarded, guard dropped
                let val = self.evaluate_expression(&expr_stmt.expression)?.take();
                Ok(Completion::Normal(val))
            }

            Statement::VariableDeclaration(decl) => {
                self.execute_variable_declaration(decl)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Block(block) => self.execute_block(block),

            Statement::If(if_stmt) => self.execute_if(if_stmt),

            Statement::Return(ret) => {
                // Return value - guard passed to caller via Completion::Return
                let Guarded {
                    value,
                    guard: _return_guard,
                } = match &ret.argument {
                    Some(expr) => self.evaluate_expression(expr)?,
                    None => Guarded::unguarded(JsValue::Undefined),
                };
                // Note: _return_guard keeps the value alive; caller should establish ownership
                Ok(Completion::Return(value))
            }

            Statement::While(while_stmt) => self.execute_while(while_stmt),

            Statement::For(for_stmt) => self.execute_for(for_stmt),

            Statement::FunctionDeclaration(func) => {
                self.execute_function_declaration(func)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Empty => Ok(Completion::Normal(JsValue::Undefined)),

            // For now, skip unimplemented statements
            _ => Ok(Completion::Normal(JsValue::Undefined)),
        }
    }

    fn execute_variable_declaration(&mut self, decl: &VariableDeclaration) -> Result<(), JsError> {
        let mutable = matches!(decl.kind, VariableKind::Let | VariableKind::Var);

        for declarator in &decl.declarations {
            // Keep guard alive until bind_pattern transfers ownership to env
            let Guarded {
                value: init_value,
                guard: _init_guard,
            } = match &declarator.init {
                Some(expr) => self.evaluate_expression(expr)?,
                None => Guarded::unguarded(JsValue::Undefined),
            };

            // bind_pattern calls env_define which establishes ownership
            self.bind_pattern(&declarator.id, init_value, mutable)?;
            // _init_guard dropped here after ownership transferred
        }

        Ok(())
    }

    fn bind_pattern(
        &mut self,
        pattern: &Pattern,
        value: JsValue,
        mutable: bool,
    ) -> Result<(), JsError> {
        match pattern {
            Pattern::Identifier(id) => {
                let name = id.name.cheap_clone();
                // env_define establishes ownership for object values
                self.env_define(name, value, mutable);
                Ok(())
            }
            _ => Err(JsError::internal_error(
                "Destructuring patterns not yet implemented",
            )),
        }
    }

    fn execute_block(&mut self, block: &BlockStatement) -> Result<Completion, JsError> {
        let saved_env = self.push_scope();

        let mut result = Completion::Normal(JsValue::Undefined);
        for stmt in &block.body {
            result = self.execute_statement(stmt)?;
            match &result {
                Completion::Normal(_) => {}
                _ => break,
            }
        }

        self.pop_scope(saved_env);
        Ok(result)
    }

    fn execute_if(&mut self, if_stmt: &IfStatement) -> Result<Completion, JsError> {
        let condition = self.evaluate_expression(&if_stmt.test)?.take();

        if condition.to_boolean() {
            self.execute_statement(&if_stmt.consequent)
        } else if let Some(ref alternate) = if_stmt.alternate {
            self.execute_statement(alternate)
        } else {
            Ok(Completion::Normal(JsValue::Undefined))
        }
    }

    fn execute_while(&mut self, while_stmt: &WhileStatement) -> Result<Completion, JsError> {
        loop {
            let condition = self.evaluate_expression(&while_stmt.test)?.take();
            if !condition.to_boolean() {
                break;
            }

            match self.execute_statement(&while_stmt.body)? {
                Completion::Normal(_) | Completion::Continue(None) => {}
                Completion::Break(None) => break,
                Completion::Return(v) => return Ok(Completion::Return(v)),
                c @ (Completion::Break(_) | Completion::Continue(_)) => return Ok(c),
            }
        }

        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_for(&mut self, for_stmt: &ForStatement) -> Result<Completion, JsError> {
        let saved_env = self.push_scope();

        // Init
        if let Some(ref init) = for_stmt.init {
            match init {
                ForInit::Variable(decl) => {
                    self.execute_variable_declaration(decl)?;
                }
                ForInit::Expression(expr) => {
                    // Init expression - value discarded
                    self.evaluate_expression(expr)?.take();
                }
            }
        }

        // Loop
        loop {
            // Test
            if let Some(ref test) = for_stmt.test {
                let condition = self.evaluate_expression(test)?.take();
                if !condition.to_boolean() {
                    break;
                }
            }

            // Body
            match self.execute_statement(&for_stmt.body)? {
                Completion::Normal(_) | Completion::Continue(None) => {}
                Completion::Break(None) => break,
                Completion::Return(v) => {
                    self.pop_scope(saved_env);
                    return Ok(Completion::Return(v));
                }
                c @ (Completion::Break(_) | Completion::Continue(_)) => {
                    self.pop_scope(saved_env);
                    return Ok(c);
                }
            }

            // Update
            if let Some(ref update) = for_stmt.update {
                // Update expression - value discarded
                self.evaluate_expression(update)?.take();
            }
        }

        self.pop_scope(saved_env);
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_function_declaration(&mut self, func: &FunctionDeclaration) -> Result<(), JsError> {
        let name = func.id.as_ref().map(|id| id.name.cheap_clone());
        let params: Rc<[_]> = func.params.clone().into();
        let body = Rc::new(FunctionBody::Block(func.body.clone()));

        // Create function with temp guard
        let (func_obj, _temp) = self.create_function_with_guard(
            name.clone(),
            params,
            body,
            self.env,
            func.span,
            func.generator,
            func.async_,
        );

        // Transfer ownership to environment before temp guard is dropped
        if let Some(js_name) = name {
            // env_define will establish ownership via env.own()
            self.env_define(js_name, JsValue::Object(func_obj), true);
        }
        // _temp is dropped here, but func_obj is alive via env ownership

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Expression Evaluation
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // Returns Guarded { value, guard } - the guard keeps newly created objects alive
    // until ownership is transferred to an environment or parent object.
    // ═══════════════════════════════════════════════════════════════════════════

    fn evaluate_expression(&mut self, expr: &Expression) -> Result<Guarded, JsError> {
        match expr {
            Expression::Literal(lit) => Ok(Guarded::unguarded(self.evaluate_literal(&lit.value)?)),

            Expression::Identifier(id) => Ok(Guarded::unguarded(self.env_get(&id.name)?)),

            Expression::Binary(bin) => self.evaluate_binary(bin),

            Expression::Unary(un) => self.evaluate_unary(un),

            Expression::Logical(log) => self.evaluate_logical(log),

            Expression::Conditional(cond) => self.evaluate_conditional(cond),

            Expression::Assignment(assign) => self.evaluate_assignment(assign),

            Expression::Call(call) => self.evaluate_call(call),

            Expression::Member(member) => Ok(Guarded::unguarded(self.evaluate_member(member)?)),

            Expression::Array(arr) => self.evaluate_array(arr),

            Expression::Object(obj) => self.evaluate_object(obj),

            Expression::ArrowFunction(arrow) => {
                let params: Rc<[_]> = arrow.params.clone().into();
                let body = Rc::new(FunctionBody::from(arrow.body.clone()));

                let (func_obj, guard) = self.create_function_with_guard(
                    None,
                    params,
                    body,
                    self.env,
                    arrow.span,
                    false,
                    arrow.async_,
                );

                Ok(Guarded::with_guard(JsValue::Object(func_obj), guard))
            }

            Expression::Function(func) => {
                let name = func.id.as_ref().map(|id| id.name.cheap_clone());
                let params: Rc<[_]> = func.params.clone().into();
                let body = Rc::new(FunctionBody::Block(func.body.clone()));

                let (func_obj, guard) = self.create_function_with_guard(
                    name,
                    params,
                    body,
                    self.env,
                    func.span,
                    func.generator,
                    func.async_,
                );

                Ok(Guarded::with_guard(JsValue::Object(func_obj), guard))
            }

            Expression::Parenthesized(inner, _) => self.evaluate_expression(inner),

            // TypeScript type assertions - just evaluate the expression, ignore the type
            Expression::TypeAssertion(ta) => self.evaluate_expression(&ta.expression),
            Expression::NonNull(nn) => self.evaluate_expression(&nn.expression),

            _ => Ok(Guarded::unguarded(JsValue::Undefined)),
        }
    }

    fn evaluate_literal(&self, lit: &LiteralValue) -> Result<JsValue, JsError> {
        Ok(match lit {
            LiteralValue::Null => JsValue::Null,
            LiteralValue::Undefined => JsValue::Undefined,
            LiteralValue::Boolean(b) => JsValue::Boolean(*b),
            LiteralValue::Number(n) => JsValue::Number(*n),
            LiteralValue::String(s) => JsValue::String(s.cheap_clone()),
            LiteralValue::BigInt(s) => {
                // Parse BigInt string to number (loses precision for large values)
                JsValue::Number(s.parse().unwrap_or(0.0))
            }
            LiteralValue::RegExp { .. } => JsValue::Undefined,
        })
    }

    fn evaluate_binary(&mut self, bin: &BinaryExpression) -> Result<Guarded, JsError> {
        let left = self.evaluate_expression(&bin.left)?.take();
        let right = self.evaluate_expression(&bin.right)?.take();

        Ok(Guarded::unguarded(match bin.operator {
            // Arithmetic
            BinaryOp::Add => match (&left, &right) {
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
            BinaryOp::StrictEq => JsValue::Boolean(left.strict_equals(&right)),
            BinaryOp::StrictNotEq => JsValue::Boolean(!left.strict_equals(&right)),
            BinaryOp::Eq => JsValue::Boolean(self.abstract_equals(&left, &right)),
            BinaryOp::NotEq => JsValue::Boolean(!self.abstract_equals(&left, &right)),

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
                // Must convert through i32 first for proper handling of negative numbers
                // -1.0 as u32 = 0 (wrong), but -1.0 as i32 as u32 = 4294967295 (correct)
                let lhs = (left.to_number() as i32) as u32;
                let rhs = ((right.to_number() as i32) as u32) & 0x1f;
                JsValue::Number((lhs >> rhs) as f64)
            }

            _ => JsValue::Undefined,
        }))
    }

    fn abstract_equals(&self, left: &JsValue, right: &JsValue) -> bool {
        match (left, right) {
            (JsValue::Undefined, JsValue::Null) | (JsValue::Null, JsValue::Undefined) => true,
            (JsValue::Number(a), JsValue::String(b)) => *a == b.parse().unwrap_or(f64::NAN),
            (JsValue::String(a), JsValue::Number(b)) => a.parse().unwrap_or(f64::NAN) == *b,
            _ => left.strict_equals(right),
        }
    }

    fn evaluate_unary(&mut self, un: &UnaryExpression) -> Result<Guarded, JsError> {
        let operand = self.evaluate_expression(&un.argument)?.take();

        Ok(Guarded::unguarded(match un.operator {
            UnaryOp::Minus => JsValue::Number(-operand.to_number()),
            UnaryOp::Plus => JsValue::Number(operand.to_number()),
            UnaryOp::Not => JsValue::Boolean(!operand.to_boolean()),
            UnaryOp::BitNot => JsValue::Number(!(operand.to_number() as i32) as f64),
            UnaryOp::Typeof => JsValue::String(JsString::from(operand.type_of())),
            UnaryOp::Void => JsValue::Undefined,
            UnaryOp::Delete => JsValue::Boolean(true),
        }))
    }

    fn evaluate_logical(&mut self, log: &LogicalExpression) -> Result<Guarded, JsError> {
        let left = self.evaluate_expression(&log.left)?;

        match log.operator {
            LogicalOp::And => {
                if !left.value.to_boolean() {
                    Ok(left)
                } else {
                    self.evaluate_expression(&log.right)
                }
            }
            LogicalOp::Or => {
                if left.value.to_boolean() {
                    Ok(left)
                } else {
                    self.evaluate_expression(&log.right)
                }
            }
            LogicalOp::NullishCoalescing => {
                if left.value.is_null_or_undefined() {
                    self.evaluate_expression(&log.right)
                } else {
                    Ok(left)
                }
            }
        }
    }

    fn evaluate_conditional(&mut self, cond: &ConditionalExpression) -> Result<Guarded, JsError> {
        let test = self.evaluate_expression(&cond.test)?.take();

        if test.to_boolean() {
            self.evaluate_expression(&cond.consequent)
        } else {
            self.evaluate_expression(&cond.alternate)
        }
    }

    fn evaluate_assignment(&mut self, assign: &AssignmentExpression) -> Result<Guarded, JsError> {
        // Evaluate RHS and keep the guard alive until ownership is transferred
        let Guarded {
            value,
            guard: _rhs_guard,
        } = self.evaluate_expression(&assign.right)?;

        match &assign.left {
            AssignmentTarget::Identifier(id) => {
                let name = id.name.cheap_clone();
                let final_value = match assign.operator {
                    AssignmentOp::Assign => value,
                    AssignmentOp::AddAssign => {
                        let current = self.env_get(&name)?;
                        match (&current, &value) {
                            (JsValue::String(a), _) => {
                                JsValue::String(a.cheap_clone() + &value.to_js_string())
                            }
                            _ => JsValue::Number(current.to_number() + value.to_number()),
                        }
                    }
                    AssignmentOp::SubAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(current.to_number() - value.to_number())
                    }
                    AssignmentOp::MulAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(current.to_number() * value.to_number())
                    }
                    AssignmentOp::DivAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(current.to_number() / value.to_number())
                    }
                    AssignmentOp::ModAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(current.to_number() % value.to_number())
                    }
                    AssignmentOp::ExpAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(current.to_number().powf(value.to_number()))
                    }
                    AssignmentOp::BitAndAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(
                            (current.to_number() as i32 & value.to_number() as i32) as f64,
                        )
                    }
                    AssignmentOp::BitOrAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(
                            (current.to_number() as i32 | value.to_number() as i32) as f64,
                        )
                    }
                    AssignmentOp::BitXorAssign => {
                        let current = self.env_get(&name)?;
                        JsValue::Number(
                            (current.to_number() as i32 ^ value.to_number() as i32) as f64,
                        )
                    }
                    AssignmentOp::LShiftAssign => {
                        let current = self.env_get(&name)?;
                        let lhs = current.to_number() as i32;
                        let rhs = (value.to_number() as u32) & 0x1f;
                        JsValue::Number((lhs << rhs) as f64)
                    }
                    AssignmentOp::RShiftAssign => {
                        let current = self.env_get(&name)?;
                        let lhs = current.to_number() as i32;
                        let rhs = (value.to_number() as u32) & 0x1f;
                        JsValue::Number((lhs >> rhs) as f64)
                    }
                    AssignmentOp::URShiftAssign => {
                        let current = self.env_get(&name)?;
                        let lhs = (current.to_number() as i32) as u32;
                        let rhs = ((value.to_number() as i32) as u32) & 0x1f;
                        JsValue::Number((lhs >> rhs) as f64)
                    }
                    _ => value,
                };
                // env_set establishes ownership, so _rhs_guard can be dropped after this
                self.env_set(&name, final_value.clone())?;
                Ok(Guarded::unguarded(final_value))
            }
            AssignmentTarget::Member(member) => {
                let obj_val = self.evaluate_expression(&member.object)?.take();
                let JsValue::Object(obj) = obj_val else {
                    return Err(JsError::type_error("Cannot set property of non-object"));
                };

                let key = self.get_member_key(&member.property)?;

                // Establish ownership before setting property
                if let JsValue::Object(ref val_obj) = value {
                    obj.own(val_obj, &self.heap);
                }
                obj.borrow_mut().set_property(key, value.clone());
                // _rhs_guard dropped here, but ownership transferred to obj

                Ok(Guarded::unguarded(value))
            }
            _ => Err(JsError::internal_error("Unsupported assignment target")),
        }
    }

    fn evaluate_call(&mut self, call: &CallExpression) -> Result<Guarded, JsError> {
        let (callee, this_value) = match &*call.callee {
            Expression::Member(member) => {
                let obj = self.evaluate_expression(&member.object)?.take();
                let key = self.get_member_key(&member.property)?;

                let func = match &obj {
                    JsValue::Object(o) => o.borrow().get_property(&key),
                    _ => None,
                };

                match func {
                    Some(f) => (f, obj),
                    None => return Err(JsError::type_error(format!("{} is not a function", key))),
                }
            }
            _ => {
                let callee = self.evaluate_expression(&call.callee)?.take();
                (callee, JsValue::Undefined)
            }
        };

        // Evaluate arguments, keeping guards alive until call completes
        let mut args = Vec::new();
        let mut _arg_guards = Vec::new();
        for arg in &call.arguments {
            match arg {
                crate::ast::Argument::Expression(expr) => {
                    let Guarded { value, guard } = self.evaluate_expression(expr)?;
                    args.push(value);
                    if let Some(g) = guard {
                        _arg_guards.push(g);
                    }
                }
                crate::ast::Argument::Spread(_) => {
                    return Err(JsError::internal_error("Spread not implemented"));
                }
            }
        }

        // Call function - result ownership will be handled by caller
        let result = self.call_function(callee, this_value, args)?;
        Ok(Guarded::unguarded(result))
    }

    fn call_function(
        &mut self,
        callee: JsValue,
        _this_value: JsValue,
        args: Vec<JsValue>,
    ) -> Result<JsValue, JsError> {
        let JsValue::Object(func_obj) = callee else {
            return Err(JsError::type_error("Not a function"));
        };

        let func = {
            let obj_ref = func_obj.borrow();
            match &obj_ref.exotic {
                ExoticObject::Function(f) => f.clone(),
                _ => return Err(JsError::type_error("Not a function")),
            }
        };

        match func {
            JsFunction::Interpreted(interp) => {
                // Create new environment
                let func_env = create_environment_with_guard(
                    &self.root_guard,
                    &self.heap,
                    Some(interp.closure),
                );

                // Bind parameters
                for (i, param) in interp.params.iter().enumerate() {
                    let arg_val = args.get(i).cloned().unwrap_or(JsValue::Undefined);
                    let name = match &param.pattern {
                        Pattern::Identifier(id) => id.name.cheap_clone(),
                        _ => continue,
                    };

                    if let JsValue::Object(ref obj) = arg_val {
                        func_env.own(obj, &self.heap);
                    }

                    if let Some(data) = func_env.borrow_mut().as_environment_mut() {
                        data.bindings.insert(
                            name,
                            Binding {
                                value: arg_val,
                                mutable: true,
                                initialized: true,
                            },
                        );
                    }
                }

                // Execute function body
                let saved_env = self.env;
                self.env = func_env;

                let result = match &*interp.body {
                    FunctionBody::Block(block) => match self.execute_block(block)? {
                        Completion::Return(v) => v,
                        _ => JsValue::Undefined,
                    },
                    FunctionBody::Expression(expr) => self.evaluate_expression(expr)?.take(),
                };

                self.env = saved_env;
                Ok(result)
            }

            JsFunction::Native(_native) => {
                // Native functions are not implemented yet
                Ok(JsValue::Undefined)
            }

            _ => Err(JsError::internal_error("Unsupported function type")),
        }
    }

    fn evaluate_member(&mut self, member: &MemberExpression) -> Result<JsValue, JsError> {
        let obj = self.evaluate_expression(&member.object)?.take();
        let key = self.get_member_key(&member.property)?;

        match &obj {
            JsValue::Object(o) => Ok(o.borrow().get_property(&key).unwrap_or(JsValue::Undefined)),
            JsValue::String(s) => {
                if let PropertyKey::Index(i) = key {
                    let chars: Vec<char> = s.as_str().chars().collect();
                    if let Some(c) = chars.get(i as usize) {
                        return Ok(JsValue::String(JsString::from(c.to_string())));
                    }
                }
                if let PropertyKey::String(ref k) = key {
                    if k.as_str() == "length" {
                        return Ok(JsValue::Number(s.as_str().chars().count() as f64));
                    }
                }
                Ok(JsValue::Undefined)
            }
            _ => Ok(JsValue::Undefined),
        }
    }

    fn get_member_key(&mut self, property: &MemberProperty) -> Result<PropertyKey, JsError> {
        match property {
            MemberProperty::Identifier(id) => Ok(PropertyKey::String(id.name.cheap_clone())),
            MemberProperty::Expression(expr) => {
                let val = self.evaluate_expression(expr)?.take();
                Ok(PropertyKey::from_value(&val))
            }
            MemberProperty::PrivateIdentifier(id) => {
                Ok(PropertyKey::String(JsString::from(format!("#{}", id.name))))
            }
        }
    }

    fn evaluate_array(&mut self, arr: &crate::ast::ArrayExpression) -> Result<Guarded, JsError> {
        // Collect elements, keeping guards alive until array is created
        let mut elements = Vec::new();
        let mut _elem_guards = Vec::new();

        for elem in &arr.elements {
            match elem {
                Some(ArrayElement::Expression(expr)) => {
                    let Guarded { value, guard } = self.evaluate_expression(expr)?;
                    elements.push(value);
                    if let Some(g) = guard {
                        _elem_guards.push(g);
                    }
                }
                Some(ArrayElement::Spread(_)) => {
                    return Err(JsError::internal_error("Spread not implemented"));
                }
                None => elements.push(JsValue::Undefined),
            }
        }

        // Create array with guard - elements ownership transferred to array
        let (arr_obj, guard) = self.create_array_with_guard(elements);
        Ok(Guarded::with_guard(JsValue::Object(arr_obj), guard))
    }

    fn evaluate_object(&mut self, obj_expr: &ObjectExpression) -> Result<Guarded, JsError> {
        let (obj, obj_guard) = self.create_object_with_guard();

        // Keep property value guards alive until ownership is transferred to obj
        let mut _prop_guards = Vec::new();

        for prop in &obj_expr.properties {
            match prop {
                ObjectProperty::Property(p) => {
                    let prop_key = match &p.key {
                        ObjectPropertyKey::Identifier(id) => {
                            PropertyKey::String(id.name.cheap_clone())
                        }
                        ObjectPropertyKey::String(s) => PropertyKey::String(s.value.cheap_clone()),
                        ObjectPropertyKey::Number(lit) => {
                            if let LiteralValue::Number(n) = &lit.value {
                                PropertyKey::from_value(&JsValue::Number(*n))
                            } else {
                                continue;
                            }
                        }
                        ObjectPropertyKey::Computed(expr) => {
                            let k = self.evaluate_expression(expr)?.take();
                            PropertyKey::from_value(&k)
                        }
                        ObjectPropertyKey::PrivateIdentifier(id) => {
                            PropertyKey::String(JsString::from(format!("#{}", id.name)))
                        }
                    };

                    let Guarded {
                        value: prop_val,
                        guard: prop_guard,
                    } = if p.shorthand {
                        // Shorthand: { x } means { x: x }
                        if let ObjectPropertyKey::Identifier(id) = &p.key {
                            Guarded::unguarded(self.env_get(&id.name)?)
                        } else {
                            self.evaluate_expression(&p.value)?
                        }
                    } else {
                        self.evaluate_expression(&p.value)?
                    };

                    // Transfer ownership from prop_guard to obj
                    if let JsValue::Object(ref val_obj) = prop_val {
                        obj.own(val_obj, &self.heap);
                    }
                    // Keep prop_guard alive until after own() call
                    if let Some(g) = prop_guard {
                        _prop_guards.push(g);
                    }

                    obj.borrow_mut().set_property(prop_key, prop_val);
                }
                ObjectProperty::Spread(_) => {
                    return Err(JsError::internal_error("Spread not implemented"));
                }
            }
        }

        Ok(Guarded::with_guard(JsValue::Object(obj), obj_guard))
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unsigned_right_shift_basic() {
        let mut interp = Interpreter::new();
        let result = interp.eval_simple("32 >>> 2").unwrap();
        assert_eq!(result, JsValue::Number(8.0));
    }

    #[test]
    fn test_unsigned_right_shift_negative() {
        let mut interp = Interpreter::new();
        assert_eq!(
            interp.eval_simple("-1 >>> 0").unwrap(),
            JsValue::Number(4294967295.0)
        );
    }

    #[test]
    fn test_basic_arithmetic() {
        let mut interp = Interpreter::new();
        assert_eq!(interp.eval_simple("1 + 2").unwrap(), JsValue::Number(3.0));
        assert_eq!(interp.eval_simple("3 * 4").unwrap(), JsValue::Number(12.0));
        assert_eq!(interp.eval_simple("10 / 2").unwrap(), JsValue::Number(5.0));
    }
}
