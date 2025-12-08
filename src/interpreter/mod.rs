//! Interpreter for executing TypeScript AST

// Builtin function implementations (split into separate files)
pub mod builtins;

// Import all builtin functions
use builtins::*;

use crate::ast::{
    Argument, ArrayElement, AssignmentExpression, AssignmentOp, AssignmentTarget, BinaryExpression,
    BinaryOp, BlockStatement, CallExpression, ClassConstructor, ClassDeclaration, ClassMember,
    ClassMethod, ClassProperty, ConditionalExpression, DoWhileStatement, EnumDeclaration,
    ExportDeclaration, Expression, ForInOfLeft, ForInStatement, ForInit, ForOfStatement,
    ForStatement, FunctionDeclaration, LiteralValue, LogicalExpression, LogicalOp,
    MemberExpression, MemberProperty, MethodKind, NewExpression, ObjectPatternProperty,
    ObjectProperty, ObjectPropertyKey, Pattern, Program, Statement, UnaryExpression, UnaryOp,
    UpdateExpression, UpdateOp, VariableDeclaration, VariableKind, WhileStatement,
};
use crate::error::JsError;
use crate::value::{
    create_array, create_function, create_object, Environment, ExoticObject, FunctionBody,
    InterpretedFunction, JsFunction, JsObjectRef, JsString, JsValue, Property, PropertyKey,
};

/// Completion record for control flow
#[derive(Debug)]
pub enum Completion {
    Normal(JsValue),
    Return(JsValue),
    Break(Option<String>),
    Continue(Option<String>),
}

/// The interpreter state
pub struct Interpreter {
    /// Global object
    pub global: JsObjectRef,
    /// Current environment
    pub env: Environment,
    /// Object.prototype for all objects
    pub object_prototype: JsObjectRef,
    /// Array.prototype for all array instances
    pub array_prototype: JsObjectRef,
    /// String.prototype for string methods
    pub string_prototype: JsObjectRef,
    /// Number.prototype for number methods
    pub number_prototype: JsObjectRef,
    /// Function.prototype for function methods (call, apply, bind)
    pub function_prototype: JsObjectRef,
    /// Map.prototype for map methods
    pub map_prototype: JsObjectRef,
    /// Set.prototype for set methods
    pub set_prototype: JsObjectRef,
    /// Date.prototype for date methods
    pub date_prototype: JsObjectRef,
    /// RegExp.prototype for regex methods
    pub regexp_prototype: JsObjectRef,
    /// Error.prototype for error methods
    pub error_prototype: JsObjectRef,
    /// Symbol.prototype for symbol methods
    pub symbol_prototype: JsObjectRef,
    /// Stores thrown value during exception propagation
    thrown_value: Option<JsValue>,
    /// Exported values from the module
    pub exports: std::collections::HashMap<String, JsValue>,
}

impl Interpreter {
    /// Create a new interpreter with global environment
    pub fn new() -> Self {
        let global = create_object();
        let mut env = Environment::new();

        // Add basic global values
        env.define("undefined".to_string(), JsValue::Undefined, false);
        env.define("NaN".to_string(), JsValue::Number(f64::NAN), false);
        env.define("Infinity".to_string(), JsValue::Number(f64::INFINITY), false);

        // Create prototypes using builtin module functions
        let object_prototype = create_object_prototype();
        let array_prototype = create_array_prototype();
        let string_prototype = create_string_prototype();
        let number_prototype = create_number_prototype();
        let function_prototype = create_function_prototype();
        let map_prototype = create_map_prototype();
        let set_prototype = create_set_prototype();
        let date_prototype = create_date_prototype();
        let regexp_prototype = create_regexp_prototype();
        let error_prototype = create_error_prototype();
        let symbol_prototype = create_symbol_prototype();

        // Create and register constructors
        let object_constructor = create_object_constructor();
        env.define("Object".to_string(), JsValue::Object(object_constructor), false);

        let array_constructor = create_array_constructor(&array_prototype);
        env.define("Array".to_string(), JsValue::Object(array_constructor), false);

        let string_constructor = create_string_constructor(&string_prototype);
        env.define("String".to_string(), JsValue::Object(string_constructor), false);

        let number_constructor = create_number_constructor(&number_prototype);
        env.define("Number".to_string(), JsValue::Object(number_constructor), false);

        let date_constructor = create_date_constructor(&date_prototype);
        env.define("Date".to_string(), JsValue::Object(date_constructor), false);

        let regexp_constructor = create_regexp_constructor(&regexp_prototype);
        env.define("RegExp".to_string(), JsValue::Object(regexp_constructor), false);

        let map_constructor = create_map_constructor();
        env.define("Map".to_string(), JsValue::Object(map_constructor), false);

        let set_constructor = create_set_constructor();
        env.define("Set".to_string(), JsValue::Object(set_constructor), false);

        // Create and register global objects
        let console = create_console_object();
        env.define("console".to_string(), JsValue::Object(console), false);

        let json = create_json_object();
        env.define("JSON".to_string(), JsValue::Object(json), false);

        let math = create_math_object();
        env.define("Math".to_string(), JsValue::Object(math), false);

        // Register global functions
        register_global_functions(&mut env);

        // Register error constructors
        let (error_fn, type_error_fn, reference_error_fn, syntax_error_fn, range_error_fn) = create_error_constructors(&error_prototype);
        env.define("Error".to_string(), JsValue::Object(error_fn), false);
        env.define("TypeError".to_string(), JsValue::Object(type_error_fn), false);
        env.define("ReferenceError".to_string(), JsValue::Object(reference_error_fn), false);
        env.define("SyntaxError".to_string(), JsValue::Object(syntax_error_fn), false);
        env.define("RangeError".to_string(), JsValue::Object(range_error_fn), false);

        // Register Symbol constructor
        let well_known_symbols = get_well_known_symbols();
        let symbol_constructor = create_symbol_constructor(&symbol_prototype, &well_known_symbols);
        env.define("Symbol".to_string(), JsValue::Object(symbol_constructor), false);

        Self {
            global,
            env,
            object_prototype,
            array_prototype,
            string_prototype,
            number_prototype,
            function_prototype,
            map_prototype,
            set_prototype,
            date_prototype,
            regexp_prototype,
            error_prototype,
            symbol_prototype,
            thrown_value: None,
            exports: std::collections::HashMap::new(),
        }
    }

    /// Create an array with the proper prototype
    pub fn create_array(&self, elements: Vec<JsValue>) -> JsObjectRef {
        let arr = create_array(elements);
        arr.borrow_mut().prototype = Some(self.array_prototype.clone());
        arr
    }

    /// Execute a program
    pub fn execute(&mut self, program: &Program) -> Result<JsValue, JsError> {
        // Hoist var declarations at global scope
        self.hoist_var_declarations(&program.body);

        let mut result = JsValue::Undefined;

        for stmt in &program.body {
            match self.execute_statement(stmt)? {
                Completion::Normal(val) => result = val,
                Completion::Return(val) => return Ok(val),
                Completion::Break(_) => {
                    return Err(JsError::syntax_error("Illegal break statement", 0, 0));
                }
                Completion::Continue(_) => {
                    return Err(JsError::syntax_error("Illegal continue statement", 0, 0));
                }
            }
        }

        Ok(result)
    }

    /// Execute a statement
    pub fn execute_statement(&mut self, stmt: &Statement) -> Result<Completion, JsError> {
        match stmt {
            Statement::Expression(expr) => {
                let value = self.evaluate(&expr.expression)?;
                Ok(Completion::Normal(value))
            }

            Statement::VariableDeclaration(decl) => {
                self.execute_variable_declaration(decl)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::FunctionDeclaration(decl) => {
                self.execute_function_declaration(decl)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Block(block) => self.execute_block(block),

            Statement::If(if_stmt) => {
                let test = self.evaluate(&if_stmt.test)?;
                if test.to_boolean() {
                    self.execute_statement(&if_stmt.consequent)
                } else if let Some(alt) = &if_stmt.alternate {
                    self.execute_statement(alt)
                } else {
                    Ok(Completion::Normal(JsValue::Undefined))
                }
            }

            Statement::While(while_stmt) => {
                loop {
                    let test = self.evaluate(&while_stmt.test)?;
                    if !test.to_boolean() {
                        break;
                    }

                    match self.execute_statement(&while_stmt.body)? {
                        Completion::Break(None) => break,
                        Completion::Break(label) => return Ok(Completion::Break(label)),
                        Completion::Continue(None) => continue,
                        Completion::Continue(label) => return Ok(Completion::Continue(label)),
                        Completion::Return(val) => return Ok(Completion::Return(val)),
                        Completion::Normal(_) => {}
                    }
                }
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::DoWhile(do_while) => {
                loop {
                    match self.execute_statement(&do_while.body)? {
                        Completion::Break(None) => break,
                        Completion::Break(label) => return Ok(Completion::Break(label)),
                        Completion::Continue(None) => {}
                        Completion::Continue(label) => return Ok(Completion::Continue(label)),
                        Completion::Return(val) => return Ok(Completion::Return(val)),
                        Completion::Normal(_) => {}
                    }

                    let test = self.evaluate(&do_while.test)?;
                    if !test.to_boolean() {
                        break;
                    }
                }
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::For(for_stmt) => self.execute_for(for_stmt),

            Statement::ForIn(for_in) => self.execute_for_in(for_in),

            Statement::ForOf(for_of) => self.execute_for_of(for_of),

            Statement::Return(ret) => {
                let value = if let Some(arg) = &ret.argument {
                    self.evaluate(arg)?
                } else {
                    JsValue::Undefined
                };
                Ok(Completion::Return(value))
            }

            Statement::Break(brk) => {
                Ok(Completion::Break(brk.label.as_ref().map(|l| l.name.clone())))
            }

            Statement::Continue(cont) => {
                Ok(Completion::Continue(cont.label.as_ref().map(|l| l.name.clone())))
            }

            Statement::Throw(throw) => {
                let value = self.evaluate(&throw.argument)?;
                self.thrown_value = Some(value);
                Err(JsError::Thrown)
            }

            Statement::Try(try_stmt) => {
                let result = self.execute_block(&try_stmt.block);

                match result {
                    Ok(completion) => {
                        if let Some(finalizer) = &try_stmt.finalizer {
                            self.execute_block(finalizer)?;
                        }
                        Ok(completion)
                    }
                    Err(err) => {
                        if let Some(handler) = &try_stmt.handler {
                            // Get the error value - either from thrown_value or create from error
                            let error_value = if matches!(err, JsError::Thrown) {
                                self.thrown_value.take().unwrap_or(JsValue::Undefined)
                            } else {
                                JsValue::from(err.to_string())
                            };

                            // Bind catch parameter
                            let prev_env = self.env.clone();
                            self.env = Environment::with_outer(self.env.clone());

                            if let Some(param) = &handler.param {
                                self.bind_pattern(param, error_value, true)?;
                            }

                            let result = self.execute_block(&handler.body);
                            self.env = prev_env;

                            if let Some(finalizer) = &try_stmt.finalizer {
                                self.execute_block(finalizer)?;
                            }

                            result
                        } else if let Some(finalizer) = &try_stmt.finalizer {
                            self.execute_block(finalizer)?;
                            // Re-throw if not caught
                            if matches!(err, JsError::Thrown) {
                                Err(JsError::Thrown)
                            } else {
                                Err(err)
                            }
                        } else {
                            Err(err)
                        }
                    }
                }
            }

            Statement::Switch(switch) => {
                let discriminant = self.evaluate(&switch.discriminant)?;
                let mut matched = false;
                let mut default_index = None;

                // Find matching case or default
                for (i, case) in switch.cases.iter().enumerate() {
                    if case.test.is_none() {
                        default_index = Some(i);
                        continue;
                    }

                    if !matched {
                        let test = self.evaluate(case.test.as_ref().unwrap())?;
                        if discriminant.strict_equals(&test) {
                            matched = true;
                        }
                    }

                    if matched {
                        for stmt in &case.consequent {
                            match self.execute_statement(stmt)? {
                                Completion::Break(_) => return Ok(Completion::Normal(JsValue::Undefined)),
                                Completion::Return(val) => return Ok(Completion::Return(val)),
                                Completion::Continue(label) => return Ok(Completion::Continue(label)),
                                Completion::Normal(_) => {}
                            }
                        }
                    }
                }

                // Fall through to default if no match
                if !matched {
                    if let Some(idx) = default_index {
                        for case in switch.cases.iter().skip(idx) {
                            for stmt in &case.consequent {
                                match self.execute_statement(stmt)? {
                                    Completion::Break(_) => return Ok(Completion::Normal(JsValue::Undefined)),
                                    Completion::Return(val) => return Ok(Completion::Return(val)),
                                    Completion::Continue(label) => return Ok(Completion::Continue(label)),
                                    Completion::Normal(_) => {}
                                }
                            }
                        }
                    }
                }

                Ok(Completion::Normal(JsValue::Undefined))
            }

            // TypeScript declarations - no-ops at runtime
            Statement::TypeAlias(_) | Statement::InterfaceDeclaration(_) => {
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::EnumDeclaration(enum_decl) => {
                self.execute_enum(enum_decl)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::ClassDeclaration(class) => {
                self.execute_class_declaration(class)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Import(_) => {
                // Import handling would require module resolution
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Export(export_decl) => {
                self.execute_export(export_decl)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Empty | Statement::Debugger => {
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Labeled(labeled) => {
                let label_name = labeled.label.name.clone();
                // Execute loop statements with the label so they can handle labeled break/continue
                match labeled.body.as_ref() {
                    Statement::For(for_stmt) => {
                        self.execute_for_labeled(for_stmt, Some(&label_name))
                    }
                    Statement::ForIn(for_in) => {
                        self.execute_for_in_labeled(for_in, Some(&label_name))
                    }
                    Statement::ForOf(for_of) => {
                        self.execute_for_of_labeled(for_of, Some(&label_name))
                    }
                    Statement::While(while_stmt) => {
                        self.execute_while_labeled(while_stmt, Some(&label_name))
                    }
                    Statement::DoWhile(do_while) => {
                        self.execute_do_while_labeled(do_while, Some(&label_name))
                    }
                    _ => {
                        // Non-loop statements - just handle break with matching label
                        match self.execute_statement(&labeled.body)? {
                            Completion::Break(Some(ref l)) if l == &label_name => {
                                Ok(Completion::Normal(JsValue::Undefined))
                            }
                            other => Ok(other)
                        }
                    }
                }
            }
        }
    }

    fn execute_variable_declaration(&mut self, decl: &VariableDeclaration) -> Result<(), JsError> {
        let mutable = decl.kind != VariableKind::Const;
        let is_var = decl.kind == VariableKind::Var;

        for declarator in &decl.declarations {
            let value = if let Some(init) = &declarator.init {
                self.evaluate(init)?
            } else {
                JsValue::Undefined
            };

            if is_var {
                // For var, use set to update the hoisted binding in outer scope
                self.bind_pattern_var(&declarator.id, value)?;
            } else {
                // For let/const, define in current scope
                self.bind_pattern(&declarator.id, value, mutable)?;
            }
        }

        Ok(())
    }

    /// Hoist var declarations to the current scope (function-scoped hoisting)
    /// This defines all var-declared variables as undefined before execution
    fn hoist_var_declarations(&mut self, statements: &[Statement]) {
        for stmt in statements {
            self.hoist_var_in_statement(stmt);
        }
    }

    fn hoist_var_in_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VariableDeclaration(decl) if decl.kind == VariableKind::Var => {
                for declarator in &decl.declarations {
                    self.hoist_pattern_names(&declarator.id);
                }
            }
            Statement::Block(block) => {
                // Var declarations inside blocks are still hoisted to function scope
                self.hoist_var_declarations(&block.body);
            }
            Statement::If(if_stmt) => {
                self.hoist_var_in_statement(&if_stmt.consequent);
                if let Some(alt) = &if_stmt.alternate {
                    self.hoist_var_in_statement(alt);
                }
            }
            Statement::For(for_stmt) => {
                if let Some(ForInit::Variable(decl)) = &for_stmt.init {
                    if decl.kind == VariableKind::Var {
                        for declarator in &decl.declarations {
                            self.hoist_pattern_names(&declarator.id);
                        }
                    }
                }
                self.hoist_var_in_statement(&for_stmt.body);
            }
            Statement::ForIn(for_in) => {
                if let ForInOfLeft::Variable(decl) = &for_in.left {
                    if decl.kind == VariableKind::Var {
                        for declarator in &decl.declarations {
                            self.hoist_pattern_names(&declarator.id);
                        }
                    }
                }
                self.hoist_var_in_statement(&for_in.body);
            }
            Statement::ForOf(for_of) => {
                if let ForInOfLeft::Variable(decl) = &for_of.left {
                    if decl.kind == VariableKind::Var {
                        for declarator in &decl.declarations {
                            self.hoist_pattern_names(&declarator.id);
                        }
                    }
                }
                self.hoist_var_in_statement(&for_of.body);
            }
            Statement::While(while_stmt) => {
                self.hoist_var_in_statement(&while_stmt.body);
            }
            Statement::DoWhile(do_while) => {
                self.hoist_var_in_statement(&do_while.body);
            }
            Statement::Try(try_stmt) => {
                self.hoist_var_declarations(&try_stmt.block.body);
                if let Some(catch) = &try_stmt.handler {
                    self.hoist_var_declarations(&catch.body.body);
                }
                if let Some(finally) = &try_stmt.finalizer {
                    self.hoist_var_declarations(&finally.body);
                }
            }
            Statement::Switch(switch_stmt) => {
                for case in &switch_stmt.cases {
                    self.hoist_var_declarations(&case.consequent);
                }
            }
            Statement::Labeled(labeled) => {
                self.hoist_var_in_statement(&labeled.body);
            }
            _ => {}
        }
    }

    /// Extract variable names from a pattern and define them as undefined (hoisted)
    fn hoist_pattern_names(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Identifier(id) => {
                // Only hoist if not already defined in this scope
                if !self.env.has_own(&id.name) {
                    self.env.define(id.name.clone(), JsValue::Undefined, true);
                }
            }
            Pattern::Object(obj_pat) => {
                for prop in &obj_pat.properties {
                    match prop {
                        ObjectPatternProperty::KeyValue { value, .. } => {
                            self.hoist_pattern_names(value);
                        }
                        ObjectPatternProperty::Rest(rest) => {
                            self.hoist_pattern_names(&rest.argument);
                        }
                    }
                }
            }
            Pattern::Array(arr_pat) => {
                for elem in &arr_pat.elements {
                    if let Some(pat) = elem {
                        self.hoist_pattern_names(pat);
                    }
                }
            }
            Pattern::Rest(rest) => {
                self.hoist_pattern_names(&rest.argument);
            }
            Pattern::Assignment(assign) => {
                self.hoist_pattern_names(&assign.left);
            }
        }
    }

    fn execute_function_declaration(&mut self, decl: &FunctionDeclaration) -> Result<(), JsError> {
        let func = InterpretedFunction {
            name: decl.id.as_ref().map(|id| id.name.clone()),
            params: decl.params.clone(),
            body: FunctionBody::Block(decl.body.clone()),
            closure: self.env.clone(),
            source_location: decl.span,
        };

        let func_obj = create_function(JsFunction::Interpreted(func));

        if let Some(id) = &decl.id {
            self.env.define(id.name.clone(), JsValue::Object(func_obj), true);
        }

        Ok(())
    }

    fn execute_class_declaration(&mut self, class: &ClassDeclaration) -> Result<(), JsError> {
        let constructor_fn = self.create_class_constructor(class)?;

        // Bind the class name first (so static blocks can reference it)
        if let Some(id) = &class.id {
            self.env.define(id.name.clone(), JsValue::Object(constructor_fn.clone()), false);
        }

        // Now execute static blocks - they can reference the class name
        for member in &class.body.members {
            if let ClassMember::StaticBlock(block) = member {
                // Execute the static block's statements
                for stmt in &block.body {
                    if let Completion::Return(_) = self.execute_statement(stmt)? {
                        // Static blocks shouldn't have returns, but handle it gracefully
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn create_class_constructor(&mut self, class: &ClassDeclaration) -> Result<JsObjectRef, JsError> {
        // Handle extends - evaluate superclass first
        let super_constructor: Option<JsObjectRef> = if let Some(super_class_expr) = &class.super_class {
            let super_val = self.evaluate(super_class_expr)?;
            if let JsValue::Object(sc) = super_val {
                Some(sc)
            } else {
                return Err(JsError::type_error("Class extends value is not a constructor"));
            }
        } else {
            None
        };

        // Create prototype object
        let prototype = create_object();

        // If we have a superclass, set up prototype chain
        if let Some(ref super_ctor) = super_constructor {
            let super_proto = super_ctor.borrow()
                .get_property(&PropertyKey::from("prototype"));
            if let Some(JsValue::Object(sp)) = super_proto {
                prototype.borrow_mut().prototype = Some(sp);
            }
        }

        // Find constructor and collect methods/properties
        let mut constructor: Option<&ClassConstructor> = None;
        let mut instance_fields: Vec<&ClassProperty> = Vec::new();
        let mut static_fields: Vec<&ClassProperty> = Vec::new();
        let mut instance_methods: Vec<&ClassMethod> = Vec::new();
        let mut static_methods: Vec<&ClassMethod> = Vec::new();

        for member in &class.body.members {
            match member {
                ClassMember::Constructor(ctor) => {
                    constructor = Some(ctor);
                }
                ClassMember::Method(method) => {
                    if method.static_ {
                        static_methods.push(method);
                    } else {
                        instance_methods.push(method);
                    }
                }
                ClassMember::Property(prop) => {
                    if prop.static_ {
                        static_fields.push(prop);
                    } else {
                        instance_fields.push(prop);
                    }
                }
                ClassMember::StaticBlock(_) => {
                    // Static blocks are collected and executed later
                }
            }
        }

        // Collect getters, setters, and regular methods separately
        // We need to combine getters and setters with the same name into one accessor property
        let mut accessors: std::collections::HashMap<String, (Option<JsObjectRef>, Option<JsObjectRef>)> = std::collections::HashMap::new();
        let mut regular_methods: Vec<(String, JsObjectRef)> = Vec::new();

        for method in &instance_methods {
            let method_name = match &method.key {
                ObjectPropertyKey::Identifier(id) => id.name.clone(),
                ObjectPropertyKey::String(s) => s.value.clone(),
                ObjectPropertyKey::Number(lit) => match &lit.value {
                    LiteralValue::Number(n) => n.to_string(),
                    _ => continue,
                },
                ObjectPropertyKey::Computed(_) => continue, // Skip computed for now
                ObjectPropertyKey::PrivateIdentifier(id) => format!("#{}", id.name),
            };

            let func = &method.value;
            let interpreted = InterpretedFunction {
                name: Some(method_name.clone()),
                params: func.params.clone(),
                body: FunctionBody::Block(func.body.clone()),
                closure: self.env.clone(),
                source_location: func.span,
            };

            let func_obj = create_function(JsFunction::Interpreted(interpreted));

            // If we have a superclass, store __super__ on the method so super.method() works
            if let Some(ref super_ctor) = super_constructor {
                func_obj.borrow_mut().set_property(
                    PropertyKey::from("__super__"),
                    JsValue::Object(super_ctor.clone()),
                );
            }

            match method.kind {
                MethodKind::Get => {
                    let entry = accessors.entry(method_name).or_insert((None, None));
                    entry.0 = Some(func_obj);
                }
                MethodKind::Set => {
                    let entry = accessors.entry(method_name).or_insert((None, None));
                    entry.1 = Some(func_obj);
                }
                MethodKind::Method => {
                    regular_methods.push((method_name, func_obj));
                }
            }
        }

        // Add accessor properties
        for (name, (getter, setter)) in accessors {
            prototype.borrow_mut().define_property(
                PropertyKey::from(name),
                Property::accessor(getter, setter),
            );
        }

        // Add regular methods
        for (name, func_obj) in regular_methods {
            prototype.borrow_mut().set_property(
                PropertyKey::from(name),
                JsValue::Object(func_obj),
            );
        }

        // Build constructor body that initializes instance fields then runs user constructor
        // We store instance fields info in the constructor function
        let field_initializers: Vec<(String, Option<Expression>)> = instance_fields
            .iter()
            .filter_map(|prop| {
                let name = match &prop.key {
                    ObjectPropertyKey::Identifier(id) => id.name.clone(),
                    ObjectPropertyKey::String(s) => s.value.clone(),
                    ObjectPropertyKey::PrivateIdentifier(id) => format!("#{}", id.name),
                    _ => return None,
                };
                Some((name, prop.value.clone()))
            })
            .collect();

        // Create the constructor function with field initializers stored in closure
        let ctor_body = if let Some(ctor) = constructor {
            ctor.body.clone()
        } else {
            // Default constructor - empty body
            BlockStatement {
                body: vec![],
                span: class.span,
            }
        };

        let ctor_params = if let Some(ctor) = constructor {
            ctor.params.clone()
        } else {
            vec![]
        };

        // Store field initializers in a special property so evaluate_new can access them
        let constructor_fn = create_function(JsFunction::Interpreted(InterpretedFunction {
            name: class.id.as_ref().map(|id| id.name.clone()),
            params: ctor_params,
            body: FunctionBody::Block(ctor_body),
            closure: self.env.clone(),
            source_location: class.span,
        }));

        // Store field initializers as a property on the constructor
        // We'll use a special internal format
        {
            let mut ctor = constructor_fn.borrow_mut();
            ctor.set_property(PropertyKey::from("prototype"), JsValue::Object(prototype.clone()));

            // Store field initializers as internal data
            // For now, we'll evaluate them at class definition time and store as default values
        }

        // Store field info that will be evaluated at construction time
        // We need a way to pass this to the new operator
        // For now, let's store the field expressions in a special way
        if !field_initializers.is_empty() {
            // First, evaluate all field values
            let mut field_values: Vec<(String, JsValue)> = Vec::new();
            for (name, value_expr) in field_initializers {
                let value = if let Some(expr) = value_expr {
                    self.evaluate(&expr).unwrap_or(JsValue::Undefined)
                } else {
                    JsValue::Undefined
                };
                field_values.push((name, value));
            }

            // Then create the fields array
            let mut field_pairs: Vec<JsValue> = Vec::new();
            for (name, value) in field_values {
                let pair = self.create_array(vec![
                    JsValue::String(JsString::from(name)),
                    value,
                ]);
                field_pairs.push(JsValue::Object(pair));
            }

            let fields_array = self.create_array(field_pairs);
            constructor_fn.borrow_mut().set_property(
                PropertyKey::from("__fields__"),
                JsValue::Object(fields_array),
            );
        }

        // Store super constructor if we have one
        if let Some(ref super_ctor) = super_constructor {
            constructor_fn.borrow_mut().set_property(
                PropertyKey::from("__super__"),
                JsValue::Object(super_ctor.clone()),
            );
        }

        // Collect static getters, setters, and regular methods separately
        let mut static_accessors: std::collections::HashMap<String, (Option<JsObjectRef>, Option<JsObjectRef>)> = std::collections::HashMap::new();
        let mut static_regular_methods: Vec<(String, JsObjectRef)> = Vec::new();

        for method in &static_methods {
            let method_name = match &method.key {
                ObjectPropertyKey::Identifier(id) => id.name.clone(),
                ObjectPropertyKey::String(s) => s.value.clone(),
                ObjectPropertyKey::Number(lit) => match &lit.value {
                    LiteralValue::Number(n) => n.to_string(),
                    _ => continue,
                },
                ObjectPropertyKey::Computed(_) => continue,
                ObjectPropertyKey::PrivateIdentifier(id) => format!("#{}", id.name),
            };

            let func = &method.value;
            let interpreted = InterpretedFunction {
                name: Some(method_name.clone()),
                params: func.params.clone(),
                body: FunctionBody::Block(func.body.clone()),
                closure: self.env.clone(),
                source_location: func.span,
            };

            let func_obj = create_function(JsFunction::Interpreted(interpreted));

            match method.kind {
                MethodKind::Get => {
                    let entry = static_accessors.entry(method_name).or_insert((None, None));
                    entry.0 = Some(func_obj);
                }
                MethodKind::Set => {
                    let entry = static_accessors.entry(method_name).or_insert((None, None));
                    entry.1 = Some(func_obj);
                }
                MethodKind::Method => {
                    static_regular_methods.push((method_name, func_obj));
                }
            }
        }

        // Add static accessor properties
        for (name, (getter, setter)) in static_accessors {
            constructor_fn.borrow_mut().define_property(
                PropertyKey::from(name),
                Property::accessor(getter, setter),
            );
        }

        // Add static regular methods
        for (name, func_obj) in static_regular_methods {
            constructor_fn.borrow_mut().set_property(
                PropertyKey::from(name),
                JsValue::Object(func_obj),
            );
        }

        // Initialize static fields
        for prop in &static_fields {
            let name = match &prop.key {
                ObjectPropertyKey::Identifier(id) => id.name.clone(),
                ObjectPropertyKey::String(s) => s.value.clone(),
                _ => continue,
            };

            let value = if let Some(expr) = &prop.value {
                self.evaluate(expr)?
            } else {
                JsValue::Undefined
            };

            constructor_fn.borrow_mut().set_property(PropertyKey::from(name), value);
        }

        // Set prototype.constructor = constructor
        prototype.borrow_mut().set_property(
            PropertyKey::from("constructor"),
            JsValue::Object(constructor_fn.clone()),
        );

        Ok(constructor_fn)
    }

    fn execute_enum(&mut self, enum_decl: &EnumDeclaration) -> Result<(), JsError> {
        let obj = create_object();
        let mut next_value = 0i32;

        for member in &enum_decl.members {
            let value = if let Some(init) = &member.initializer {
                let val = self.evaluate(init)?;
                if let JsValue::Number(n) = val {
                    next_value = n as i32 + 1;
                }
                val
            } else {
                let val = JsValue::Number(next_value as f64);
                next_value += 1;
                val
            };

            // Forward mapping: name -> value
            obj.borrow_mut().set_property(
                PropertyKey::from(member.id.name.as_str()),
                value.clone(),
            );

            // Reverse mapping for numeric enums: value -> name
            if let JsValue::Number(n) = &value {
                obj.borrow_mut().set_property(
                    PropertyKey::from(n.to_string()),
                    JsValue::String(JsString::from(member.id.name.clone())),
                );
            }
        }

        self.env.define(enum_decl.id.name.clone(), JsValue::Object(obj), false);
        Ok(())
    }

    fn execute_export(&mut self, export_decl: &ExportDeclaration) -> Result<(), JsError> {
        // Handle export with declaration: export function foo() {}, export const x = 1
        if let Some(declaration) = &export_decl.declaration {
            match declaration.as_ref() {
                Statement::FunctionDeclaration(func_decl) => {
                    self.execute_function_declaration(func_decl)?;
                    if let Some(id) = &func_decl.id {
                        let name = id.name.clone();
                        if let Ok(value) = self.env.get(&name) {
                            self.exports.insert(name, value);
                        }
                    }
                }
                Statement::VariableDeclaration(var_decl) => {
                    self.execute_variable_declaration(var_decl)?;
                    // Export each declared variable
                    for declarator in &var_decl.declarations {
                        let names = self.get_pattern_names(&declarator.id);
                        for name in names {
                            if let Ok(value) = self.env.get(&name) {
                                self.exports.insert(name, value);
                            }
                        }
                    }
                }
                Statement::ClassDeclaration(class_decl) => {
                    self.execute_class_declaration(class_decl)?;
                    if let Some(id) = &class_decl.id {
                        let name = id.name.clone();
                        if let Ok(value) = self.env.get(&name) {
                            self.exports.insert(name, value);
                        }
                    }
                }
                Statement::TypeAlias(_) | Statement::InterfaceDeclaration(_) => {
                    // Type-only exports - no runtime effect
                }
                _ => {
                    // Other declarations that we may not support yet
                }
            }
        }

        // Handle export specifiers: export { foo, bar }
        for spec in &export_decl.specifiers {
            let local_name = &spec.local.name;
            let exported_name = &spec.exported.name;
            if let Ok(value) = self.env.get(local_name) {
                self.exports.insert(exported_name.clone(), value);
            }
        }

        // Handle export default
        if export_decl.default {
            // TODO: Handle export default expression
        }

        Ok(())
    }

    /// Get variable names from a pattern for export tracking
    fn get_pattern_names(&self, pattern: &Pattern) -> Vec<String> {
        let mut names = Vec::new();
        self.collect_pattern_names(pattern, &mut names);
        names
    }

    fn execute_block(&mut self, block: &BlockStatement) -> Result<Completion, JsError> {
        let prev_env = self.env.clone();
        self.env = Environment::with_outer(self.env.clone());

        let mut result = Completion::Normal(JsValue::Undefined);

        for stmt in &block.body {
            result = self.execute_statement(stmt)?;
            match &result {
                Completion::Normal(_) => {}
                _ => break,
            }
        }

        self.env = prev_env;
        Ok(result)
    }

    fn execute_for(&mut self, for_stmt: &ForStatement) -> Result<Completion, JsError> {
        self.execute_for_labeled(for_stmt, None)
    }

    fn execute_for_labeled(&mut self, for_stmt: &ForStatement, label: Option<&str>) -> Result<Completion, JsError> {
        let prev_env = self.env.clone();
        self.env = Environment::with_outer(self.env.clone());

        // Track let-declared loop variables for per-iteration binding
        let mut let_var_names: Vec<String> = Vec::new();
        let is_let_loop = if let Some(ForInit::Variable(decl)) = &for_stmt.init {
            decl.kind == VariableKind::Let || decl.kind == VariableKind::Const
        } else {
            false
        };

        // Init
        if let Some(init) = &for_stmt.init {
            match init {
                ForInit::Variable(decl) => {
                    // Collect let/const variable names for per-iteration binding
                    if is_let_loop {
                        for declarator in &decl.declarations {
                            self.collect_pattern_names(&declarator.id, &mut let_var_names);
                        }
                    }
                    self.execute_variable_declaration(decl)?;
                }
                ForInit::Expression(expr) => {
                    self.evaluate(expr)?;
                }
            }
        }

        let loop_env = self.env.clone();

        // Loop
        loop {
            // Test
            if let Some(test) = &for_stmt.test {
                let test_val = self.evaluate(test)?;
                if !test_val.to_boolean() {
                    break;
                }
            }

            // For let/const loops, create per-iteration scope
            if is_let_loop && !let_var_names.is_empty() {
                let mut iter_env = Environment::with_outer(loop_env.clone());
                // Copy current values into the per-iteration scope
                for name in &let_var_names {
                    if let Ok(val) = self.env.get(name) {
                        iter_env.define(name.clone(), val, true);
                    }
                }
                self.env = iter_env;
            }

            // Body
            match self.execute_statement(&for_stmt.body)? {
                Completion::Break(None) => {
                    self.env = loop_env.clone();
                    break;
                }
                Completion::Break(Some(ref l)) if label == Some(l.as_str()) => {
                    self.env = prev_env;
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => {
                    self.env = prev_env;
                    return Ok(Completion::Break(lbl));
                }
                Completion::Continue(None) => {}
                Completion::Continue(Some(ref l)) if label == Some(l.as_str()) => {
                    // Continue with matching label - continue this loop
                }
                Completion::Continue(lbl) => {
                    self.env = prev_env;
                    return Ok(Completion::Continue(lbl));
                }
                Completion::Return(val) => {
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {}
            }

            // Update - copy values back to loop env, update, then continue
            if is_let_loop && !let_var_names.is_empty() {
                // Copy updated values back to loop env before update
                for name in &let_var_names {
                    if let Ok(val) = self.env.get(name) {
                        let _ = loop_env.set(name, val);
                    }
                }
                self.env = loop_env.clone();
            }

            if let Some(update) = &for_stmt.update {
                self.evaluate(update)?;
            }
        }

        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    /// Collect all variable names from a pattern
    fn collect_pattern_names(&self, pattern: &Pattern, names: &mut Vec<String>) {
        match pattern {
            Pattern::Identifier(id) => names.push(id.name.clone()),
            Pattern::Object(obj) => {
                for prop in &obj.properties {
                    match prop {
                        ObjectPatternProperty::KeyValue { value, .. } => {
                            self.collect_pattern_names(value, names);
                        }
                        ObjectPatternProperty::Rest(rest) => {
                            self.collect_pattern_names(&rest.argument, names);
                        }
                    }
                }
            }
            Pattern::Array(arr) => {
                for elem in &arr.elements {
                    if let Some(p) = elem {
                        self.collect_pattern_names(p, names);
                    }
                }
            }
            Pattern::Assignment(assign) => {
                self.collect_pattern_names(&assign.left, names);
            }
            Pattern::Rest(rest) => {
                self.collect_pattern_names(&rest.argument, names);
            }
        }
    }

    fn execute_for_in(&mut self, for_in: &ForInStatement) -> Result<Completion, JsError> {
        self.execute_for_in_labeled(for_in, None)
    }

    fn execute_for_in_labeled(&mut self, for_in: &ForInStatement, label: Option<&str>) -> Result<Completion, JsError> {
        let right = self.evaluate(&for_in.right)?;

        let keys = match &right {
            JsValue::Object(obj) => {
                obj.borrow()
                    .properties
                    .iter()
                    .filter(|(key, prop)| prop.enumerable && !key.is_symbol())
                    .map(|(key, _)| key.to_string())
                    .collect::<Vec<_>>()
            }
            _ => vec![],
        };

        let prev_env = self.env.clone();

        for key in keys {
            self.env = Environment::with_outer(prev_env.clone());

            let key_value = JsValue::String(JsString::from(key));

            match &for_in.left {
                ForInOfLeft::Variable(decl) => {
                    let mutable = decl.kind != VariableKind::Const;
                    if let Some(declarator) = decl.declarations.first() {
                        self.bind_pattern(&declarator.id, key_value, mutable)?;
                    }
                }
                ForInOfLeft::Pattern(pattern) => {
                    self.bind_pattern(pattern, key_value, true)?;
                }
            }

            match self.execute_statement(&for_in.body)? {
                Completion::Break(None) => break,
                Completion::Break(Some(ref l)) if label == Some(l.as_str()) => {
                    self.env = prev_env;
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => {
                    self.env = prev_env;
                    return Ok(Completion::Break(lbl));
                }
                Completion::Continue(None) => continue,
                Completion::Continue(Some(ref l)) if label == Some(l.as_str()) => {
                    // Continue with matching label - continue this loop
                    continue;
                }
                Completion::Continue(lbl) => {
                    self.env = prev_env;
                    return Ok(Completion::Continue(lbl));
                }
                Completion::Return(val) => {
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {}
            }
        }

        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_for_of(&mut self, for_of: &ForOfStatement) -> Result<Completion, JsError> {
        self.execute_for_of_labeled(for_of, None)
    }

    fn execute_for_of_labeled(&mut self, for_of: &ForOfStatement, label: Option<&str>) -> Result<Completion, JsError> {
        let right = self.evaluate(&for_of.right)?;

        let items = match &right {
            JsValue::Object(obj) => {
                let obj_ref = obj.borrow();
                match &obj_ref.exotic {
                    ExoticObject::Array { length } => {
                        let mut items = Vec::with_capacity(*length as usize);
                        for i in 0..*length {
                            if let Some(val) = obj_ref.get_property(&PropertyKey::Index(i)) {
                                items.push(val);
                            } else {
                                items.push(JsValue::Undefined);
                            }
                        }
                        items
                    }
                    _ => vec![],
                }
            }
            JsValue::String(s) => {
                s.as_str().chars().map(|c| JsValue::from(c.to_string())).collect()
            }
            _ => vec![],
        };

        let prev_env = self.env.clone();

        for item in items {
            self.env = Environment::with_outer(prev_env.clone());

            match &for_of.left {
                ForInOfLeft::Variable(decl) => {
                    let mutable = decl.kind != VariableKind::Const;
                    if let Some(declarator) = decl.declarations.first() {
                        self.bind_pattern(&declarator.id, item, mutable)?;
                    }
                }
                ForInOfLeft::Pattern(pattern) => {
                    self.bind_pattern(pattern, item, true)?;
                }
            }

            match self.execute_statement(&for_of.body)? {
                Completion::Break(None) => break,
                Completion::Break(Some(ref l)) if label == Some(l.as_str()) => {
                    self.env = prev_env;
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => {
                    self.env = prev_env;
                    return Ok(Completion::Break(lbl));
                }
                Completion::Continue(None) => continue,
                Completion::Continue(Some(ref l)) if label == Some(l.as_str()) => {
                    // Continue with matching label - continue this loop
                    continue;
                }
                Completion::Continue(lbl) => {
                    self.env = prev_env;
                    return Ok(Completion::Continue(lbl));
                }
                Completion::Return(val) => {
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {}
            }
        }

        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_while_labeled(&mut self, while_stmt: &WhileStatement, label: Option<&str>) -> Result<Completion, JsError> {
        loop {
            let test = self.evaluate(&while_stmt.test)?;
            if !test.to_boolean() {
                break;
            }

            match self.execute_statement(&while_stmt.body)? {
                Completion::Break(None) => break,
                Completion::Break(Some(ref l)) if label == Some(l.as_str()) => {
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => return Ok(Completion::Break(lbl)),
                Completion::Continue(None) => continue,
                Completion::Continue(Some(ref l)) if label == Some(l.as_str()) => {
                    // Continue with matching label - continue this loop
                    continue;
                }
                Completion::Continue(lbl) => return Ok(Completion::Continue(lbl)),
                Completion::Return(val) => return Ok(Completion::Return(val)),
                Completion::Normal(_) => {}
            }
        }
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_do_while_labeled(&mut self, do_while: &DoWhileStatement, label: Option<&str>) -> Result<Completion, JsError> {
        loop {
            match self.execute_statement(&do_while.body)? {
                Completion::Break(None) => break,
                Completion::Break(Some(ref l)) if label == Some(l.as_str()) => {
                    return Ok(Completion::Normal(JsValue::Undefined));
                }
                Completion::Break(lbl) => return Ok(Completion::Break(lbl)),
                Completion::Continue(None) => {}
                Completion::Continue(Some(ref l)) if label == Some(l.as_str()) => {
                    // Continue with matching label - skip to test
                }
                Completion::Continue(lbl) => return Ok(Completion::Continue(lbl)),
                Completion::Return(val) => return Ok(Completion::Return(val)),
                Completion::Normal(_) => {}
            }

            let test = self.evaluate(&do_while.test)?;
            if !test.to_boolean() {
                break;
            }
        }
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn bind_pattern(&mut self, pattern: &Pattern, value: JsValue, mutable: bool) -> Result<(), JsError> {
        match pattern {
            Pattern::Identifier(id) => {
                self.env.define(id.name.clone(), value, mutable);
                Ok(())
            }

            Pattern::Object(obj_pattern) => {
                let obj = match &value {
                    JsValue::Object(o) => o.clone(),
                    _ => return Err(JsError::type_error("Cannot destructure non-object")),
                };

                for prop in &obj_pattern.properties {
                    match prop {
                        ObjectPatternProperty::KeyValue { key, value: pattern, .. } => {
                            let key_str = match key {
                                ObjectPropertyKey::Identifier(id) => id.name.clone(),
                                ObjectPropertyKey::String(s) => s.value.clone(),
                                ObjectPropertyKey::Number(l) => {
                                    if let LiteralValue::Number(n) = &l.value {
                                        n.to_string()
                                    } else {
                                        continue;
                                    }
                                }
                                ObjectPropertyKey::Computed(_) => continue,
                                ObjectPropertyKey::PrivateIdentifier(id) => format!("#{}", id.name),
                            };

                            let prop_value = obj
                                .borrow()
                                .get_property(&PropertyKey::from(key_str.as_str()))
                                .unwrap_or(JsValue::Undefined);

                            self.bind_pattern(pattern, prop_value, mutable)?;
                        }
                        ObjectPatternProperty::Rest(rest) => {
                            // Collect remaining properties
                            let rest_obj = create_object();
                            // Simplified - would need to track which keys were already destructured
                            self.bind_pattern(&rest.argument, JsValue::Object(rest_obj), mutable)?;
                        }
                    }
                }

                Ok(())
            }

            Pattern::Array(arr_pattern) => {
                let items: Vec<JsValue> = match &value {
                    JsValue::Object(obj) => {
                        let obj_ref = obj.borrow();
                        if let ExoticObject::Array { length } = &obj_ref.exotic {
                            let mut items = Vec::with_capacity(*length as usize);
                            for i in 0..*length {
                                items.push(
                                    obj_ref
                                        .get_property(&PropertyKey::Index(i))
                                        .unwrap_or(JsValue::Undefined),
                                );
                            }
                            items
                        } else {
                            vec![]
                        }
                    }
                    _ => vec![],
                };

                for (i, elem) in arr_pattern.elements.iter().enumerate() {
                    if let Some(pattern) = elem {
                        match pattern {
                            Pattern::Rest(rest) => {
                                let remaining: Vec<JsValue> = items.iter().skip(i).cloned().collect();
                                self.bind_pattern(
                                    &rest.argument,
                                    JsValue::Object(create_array(remaining)),
                                    mutable,
                                )?;
                                break;
                            }
                            _ => {
                                let val = items.get(i).cloned().unwrap_or(JsValue::Undefined);
                                self.bind_pattern(pattern, val, mutable)?;
                            }
                        }
                    }
                }

                Ok(())
            }

            Pattern::Assignment(assign) => {
                let val = if value == JsValue::Undefined {
                    self.evaluate(&assign.right)?
                } else {
                    value
                };
                self.bind_pattern(&assign.left, val, mutable)
            }

            Pattern::Rest(rest) => {
                self.bind_pattern(&rest.argument, value, mutable)
            }
        }
    }

    /// Bind a pattern using var semantics (set existing hoisted binding)
    fn bind_pattern_var(&mut self, pattern: &Pattern, value: JsValue) -> Result<(), JsError> {
        match pattern {
            Pattern::Identifier(id) => {
                // For var, the binding was hoisted, so we need to set it
                // Try to set in existing scope chain; if not found, define in current
                if self.env.has(&id.name) {
                    self.env.set(&id.name, value)?;
                } else {
                    // Fallback: define if somehow not hoisted
                    self.env.define(id.name.clone(), value, true);
                }
                Ok(())
            }

            Pattern::Object(obj_pattern) => {
                let obj = match &value {
                    JsValue::Object(o) => o.clone(),
                    _ => return Err(JsError::type_error("Cannot destructure non-object")),
                };

                for prop in &obj_pattern.properties {
                    match prop {
                        ObjectPatternProperty::KeyValue { key, value: pattern, .. } => {
                            let key_str = match key {
                                ObjectPropertyKey::Identifier(id) => id.name.clone(),
                                ObjectPropertyKey::String(s) => s.value.clone(),
                                ObjectPropertyKey::Number(l) => {
                                    if let LiteralValue::Number(n) = &l.value {
                                        n.to_string()
                                    } else {
                                        continue;
                                    }
                                }
                                ObjectPropertyKey::Computed(_) => continue,
                                ObjectPropertyKey::PrivateIdentifier(id) => format!("#{}", id.name),
                            };

                            let prop_value = obj
                                .borrow()
                                .get_property(&PropertyKey::from(key_str.as_str()))
                                .unwrap_or(JsValue::Undefined);

                            self.bind_pattern_var(pattern, prop_value)?;
                        }
                        ObjectPatternProperty::Rest(rest) => {
                            let rest_obj = create_object();
                            self.bind_pattern_var(&rest.argument, JsValue::Object(rest_obj))?;
                        }
                    }
                }

                Ok(())
            }

            Pattern::Array(arr_pattern) => {
                let items: Vec<JsValue> = match &value {
                    JsValue::Object(obj) => {
                        let obj_ref = obj.borrow();
                        if let ExoticObject::Array { length } = &obj_ref.exotic {
                            let mut items = Vec::with_capacity(*length as usize);
                            for i in 0..*length {
                                items.push(
                                    obj_ref
                                        .get_property(&PropertyKey::Index(i))
                                        .unwrap_or(JsValue::Undefined),
                                );
                            }
                            items
                        } else {
                            vec![]
                        }
                    }
                    _ => vec![],
                };

                for (i, elem) in arr_pattern.elements.iter().enumerate() {
                    if let Some(pattern) = elem {
                        match pattern {
                            Pattern::Rest(rest) => {
                                let remaining: Vec<JsValue> = items.iter().skip(i).cloned().collect();
                                self.bind_pattern_var(
                                    &rest.argument,
                                    JsValue::Object(create_array(remaining)),
                                )?;
                                break;
                            }
                            _ => {
                                let val = items.get(i).cloned().unwrap_or(JsValue::Undefined);
                                self.bind_pattern_var(pattern, val)?;
                            }
                        }
                    }
                }

                Ok(())
            }

            Pattern::Assignment(assign) => {
                let val = if value == JsValue::Undefined {
                    self.evaluate(&assign.right)?
                } else {
                    value
                };
                self.bind_pattern_var(&assign.left, val)
            }

            Pattern::Rest(rest) => {
                self.bind_pattern_var(&rest.argument, value)
            }
        }
    }

    /// Evaluate an expression
    pub fn evaluate(&mut self, expr: &Expression) -> Result<JsValue, JsError> {
        match expr {
            Expression::Literal(lit) => self.evaluate_literal(&lit.value),

            Expression::Identifier(id) => {
                self.env.get(&id.name)
            }

            Expression::This(_) => {
                // Look up 'this' from the environment
                Ok(self.env.get("this").unwrap_or_else(|_| JsValue::Undefined))
            }

            Expression::Array(arr) => {
                let mut elements = vec![];
                for elem in &arr.elements {
                    match elem {
                        Some(ArrayElement::Expression(e)) => {
                            elements.push(self.evaluate(e)?);
                        }
                        Some(ArrayElement::Spread(spread)) => {
                            let val = self.evaluate(&spread.argument)?;
                            if let JsValue::Object(obj) = val {
                                let obj_ref = obj.borrow();
                                if let ExoticObject::Array { length } = &obj_ref.exotic {
                                    for i in 0..*length {
                                        if let Some(v) = obj_ref.get_property(&PropertyKey::Index(i)) {
                                            elements.push(v);
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            elements.push(JsValue::Undefined);
                        }
                    }
                }
                Ok(JsValue::Object(self.create_array(elements)))
            }

            Expression::Object(obj) => {
                let result = create_object();
                for prop in &obj.properties {
                    match prop {
                        ObjectProperty::Property(p) => {
                            let key = self.evaluate_property_key(&p.key)?;
                            let value = if p.method {
                                // Method shorthand - would need to handle this specially
                                self.evaluate(&p.value)?
                            } else {
                                self.evaluate(&p.value)?
                            };
                            result.borrow_mut().set_property(key, value);
                        }
                        ObjectProperty::Spread(spread) => {
                            let val = self.evaluate(&spread.argument)?;
                            if let JsValue::Object(src) = val {
                                let src_ref = src.borrow();
                                for (key, prop) in src_ref.properties.iter() {
                                    if prop.enumerable {
                                        result.borrow_mut().set_property(key.clone(), prop.value.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(JsValue::Object(result))
            }

            Expression::Function(func) => {
                let interpreted = InterpretedFunction {
                    name: func.id.as_ref().map(|id| id.name.clone()),
                    params: func.params.clone(),
                    body: FunctionBody::Block(func.body.clone()),
                    closure: self.env.clone(),
                    source_location: func.span,
                };
                Ok(JsValue::Object(create_function(JsFunction::Interpreted(interpreted))))
            }

            Expression::ArrowFunction(arrow) => {
                let interpreted = InterpretedFunction {
                    name: None,
                    params: arrow.params.clone(),
                    body: arrow.body.clone().into(),
                    closure: self.env.clone(),
                    source_location: arrow.span,
                };
                Ok(JsValue::Object(create_function(JsFunction::Interpreted(interpreted))))
            }

            Expression::Unary(unary) => self.evaluate_unary(unary),
            Expression::Binary(binary) => self.evaluate_binary(binary),
            Expression::Logical(logical) => self.evaluate_logical(logical),
            Expression::Conditional(cond) => self.evaluate_conditional(cond),
            Expression::Assignment(assign) => self.evaluate_assignment(assign),
            Expression::Update(update) => self.evaluate_update(update),
            Expression::Member(member) => self.evaluate_member(member),
            Expression::Call(call) => self.evaluate_call(call),
            Expression::New(new) => self.evaluate_new(new),

            Expression::Sequence(seq) => {
                let mut result = JsValue::Undefined;
                for expr in &seq.expressions {
                    result = self.evaluate(expr)?;
                }
                Ok(result)
            }

            Expression::Template(template) => {
                let mut result = String::new();
                for (i, quasi) in template.quasis.iter().enumerate() {
                    result.push_str(&quasi.value);
                    if i < template.expressions.len() {
                        let val = self.evaluate(&template.expressions[i])?;
                        result.push_str(&val.to_js_string().to_string());
                    }
                }
                Ok(JsValue::String(JsString::from(result)))
            }

            Expression::TaggedTemplate(tagged) => {
                // Evaluate the tag function
                let tag_fn = self.evaluate(&tagged.tag)?;

                // Build the strings array (first argument)
                let strings: Vec<JsValue> = tagged
                    .quasi
                    .quasis
                    .iter()
                    .map(|q| JsValue::String(JsString::from(q.value.clone())))
                    .collect();
                let strings_array = JsValue::Object(self.create_array(strings));

                // Add 'raw' property to strings array (same as cooked for now)
                // TODO: properly handle raw strings with escape sequences
                if let JsValue::Object(ref arr) = strings_array {
                    let raw: Vec<JsValue> = tagged
                        .quasi
                        .quasis
                        .iter()
                        .map(|q| JsValue::String(JsString::from(q.value.clone())))
                        .collect();
                    let raw_array = JsValue::Object(self.create_array(raw));
                    arr.borrow_mut()
                        .set_property(PropertyKey::from("raw"), raw_array);
                }

                // Evaluate all interpolated expressions (remaining arguments)
                let mut args = vec![strings_array];
                for expr in &tagged.quasi.expressions {
                    args.push(self.evaluate(expr)?);
                }

                // Call the tag function
                self.call_function(tag_fn, JsValue::Undefined, args)
            }

            Expression::Parenthesized(inner, _) => self.evaluate(inner),

            // TypeScript expressions - evaluate the inner expression
            Expression::TypeAssertion(ta) => self.evaluate(&ta.expression),
            Expression::NonNull(nn) => self.evaluate(&nn.expression),

            Expression::Spread(spread) => self.evaluate(&spread.argument),

            Expression::Await(_) | Expression::Yield(_) => {
                Err(JsError::type_error("Async/generators not supported"))
            }

            Expression::Super(_) => {
                // Return __super__ from environment so it can be called or have properties accessed
                // super() calls the parent constructor with current this
                // super.method() accesses parent prototype method
                self.env.get("__super__").map_err(|_| {
                    JsError::reference_error("'super' keyword is not available in this context")
                })
            }

            Expression::Class(class_expr) => {
                // Convert ClassExpression to ClassDeclaration-like structure and create constructor
                let class_decl = ClassDeclaration {
                    id: class_expr.id.clone(),
                    type_parameters: class_expr.type_parameters.clone(),
                    super_class: class_expr.super_class.clone(),
                    implements: class_expr.implements.clone(),
                    body: class_expr.body.clone(),
                    decorators: class_expr.decorators.clone(),
                    abstract_: false,
                    span: class_expr.span,
                };
                let constructor_fn = self.create_class_constructor(&class_decl)?;
                Ok(JsValue::Object(constructor_fn))
            }

            Expression::OptionalChain(chain) => {
                // Simplified optional chain handling
                self.evaluate(&chain.base)
            }
        }
    }

    fn evaluate_literal(&self, value: &LiteralValue) -> Result<JsValue, JsError> {
        Ok(match value {
            LiteralValue::Null => JsValue::Null,
            LiteralValue::Undefined => JsValue::Undefined,
            LiteralValue::Boolean(b) => JsValue::Boolean(*b),
            LiteralValue::Number(n) => JsValue::Number(*n),
            LiteralValue::String(s) => JsValue::String(JsString::from(s.clone())),
            LiteralValue::BigInt(s) => {
                // TODO: Implement proper BigInt type
                // For now, convert to Number (loses precision for large values)
                JsValue::Number(s.parse::<f64>().unwrap_or(f64::NAN))
            }
            LiteralValue::RegExp { pattern, flags } => {
                // Create RegExp object with proper prototype and properties
                let regexp_obj = create_object();
                {
                    let mut obj = regexp_obj.borrow_mut();
                    obj.exotic = ExoticObject::RegExp {
                        pattern: pattern.clone(),
                        flags: flags.clone(),
                    };
                    obj.prototype = Some(self.regexp_prototype.clone());
                    obj.set_property(
                        PropertyKey::from("source"),
                        JsValue::String(JsString::from(pattern.clone())),
                    );
                    obj.set_property(
                        PropertyKey::from("flags"),
                        JsValue::String(JsString::from(flags.clone())),
                    );
                    obj.set_property(
                        PropertyKey::from("global"),
                        JsValue::Boolean(flags.contains('g')),
                    );
                    obj.set_property(
                        PropertyKey::from("ignoreCase"),
                        JsValue::Boolean(flags.contains('i')),
                    );
                    obj.set_property(
                        PropertyKey::from("multiline"),
                        JsValue::Boolean(flags.contains('m')),
                    );
                }
                JsValue::Object(regexp_obj)
            }
        })
    }

    fn evaluate_property_key(&mut self, key: &ObjectPropertyKey) -> Result<PropertyKey, JsError> {
        Ok(match key {
            ObjectPropertyKey::Identifier(id) => PropertyKey::from(id.name.as_str()),
            ObjectPropertyKey::String(s) => PropertyKey::from(s.value.as_str()),
            ObjectPropertyKey::Number(lit) => {
                if let LiteralValue::Number(n) = &lit.value {
                    PropertyKey::from_value(&JsValue::Number(*n))
                } else {
                    PropertyKey::from("undefined")
                }
            }
            ObjectPropertyKey::Computed(expr) => {
                let val = self.evaluate(expr)?;
                PropertyKey::from_value(&val)
            }
            ObjectPropertyKey::PrivateIdentifier(id) => {
                // Private fields are stored with # prefix
                PropertyKey::from(format!("#{}", id.name))
            }
        })
    }

    fn evaluate_unary(&mut self, unary: &UnaryExpression) -> Result<JsValue, JsError> {
        let arg = self.evaluate(&unary.argument)?;

        Ok(match unary.operator {
            UnaryOp::Minus => JsValue::Number(-arg.to_number()),
            UnaryOp::Plus => JsValue::Number(arg.to_number()),
            UnaryOp::Not => JsValue::Boolean(!arg.to_boolean()),
            UnaryOp::BitNot => JsValue::Number(!(arg.to_number() as i32) as f64),
            UnaryOp::Typeof => JsValue::String(JsString::from(arg.type_of())),
            UnaryOp::Void => JsValue::Undefined,
            UnaryOp::Delete => {
                // Simplified - would need to actually delete property
                JsValue::Boolean(true)
            }
        })
    }

    fn evaluate_binary(&mut self, binary: &BinaryExpression) -> Result<JsValue, JsError> {
        let left = self.evaluate(&binary.left)?;
        let right = self.evaluate(&binary.right)?;

        Ok(match binary.operator {
            // Arithmetic
            BinaryOp::Add => {
                if left.is_string() || right.is_string() {
                    let ls = left.to_js_string();
                    let rs = right.to_js_string();
                    JsValue::String(ls + &rs)
                } else {
                    JsValue::Number(left.to_number() + right.to_number())
                }
            }
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
            BinaryOp::Eq => {
                // Abstract equality - simplified
                JsValue::Boolean(left.strict_equals(&right))
            }
            BinaryOp::NotEq => JsValue::Boolean(!left.strict_equals(&right)),
            BinaryOp::StrictEq => JsValue::Boolean(left.strict_equals(&right)),
            BinaryOp::StrictNotEq => JsValue::Boolean(!left.strict_equals(&right)),

            // Bitwise
            BinaryOp::BitAnd => JsValue::Number((left.to_number() as i32 & right.to_number() as i32) as f64),
            BinaryOp::BitOr => JsValue::Number((left.to_number() as i32 | right.to_number() as i32) as f64),
            BinaryOp::BitXor => JsValue::Number((left.to_number() as i32 ^ right.to_number() as i32) as f64),
            BinaryOp::LShift => JsValue::Number(((left.to_number() as i32) << (right.to_number() as u32 & 0x1f)) as f64),
            BinaryOp::RShift => JsValue::Number(((left.to_number() as i32) >> (right.to_number() as u32 & 0x1f)) as f64),
            BinaryOp::URShift => JsValue::Number((((left.to_number() as i32) as u32) >> (right.to_number() as u32 & 0x1f)) as f64),

            // Other
            BinaryOp::In => {
                if let JsValue::Object(obj) = right {
                    let key = crate::value::PropertyKey::from_value(&left);
                    JsValue::Boolean(obj.borrow().has_own_property(&key))
                } else {
                    return Err(JsError::type_error("Cannot use 'in' operator on non-object"));
                }
            }
            BinaryOp::Instanceof => {
                // Simplified
                JsValue::Boolean(false)
            }
        })
    }

    fn evaluate_logical(&mut self, logical: &LogicalExpression) -> Result<JsValue, JsError> {
        let left = self.evaluate(&logical.left)?;

        match logical.operator {
            LogicalOp::And => {
                if !left.to_boolean() {
                    Ok(left)
                } else {
                    self.evaluate(&logical.right)
                }
            }
            LogicalOp::Or => {
                if left.to_boolean() {
                    Ok(left)
                } else {
                    self.evaluate(&logical.right)
                }
            }
            LogicalOp::NullishCoalescing => {
                if left.is_null_or_undefined() {
                    self.evaluate(&logical.right)
                } else {
                    Ok(left)
                }
            }
        }
    }

    fn evaluate_conditional(&mut self, cond: &ConditionalExpression) -> Result<JsValue, JsError> {
        let test = self.evaluate(&cond.test)?;
        if test.to_boolean() {
            self.evaluate(&cond.consequent)
        } else {
            self.evaluate(&cond.alternate)
        }
    }

    fn evaluate_assignment(&mut self, assign: &AssignmentExpression) -> Result<JsValue, JsError> {
        let right = self.evaluate(&assign.right)?;

        let value = if assign.operator != AssignmentOp::Assign {
            let left = match &assign.left {
                AssignmentTarget::Identifier(id) => self.env.get(&id.name).unwrap_or(JsValue::Undefined),
                AssignmentTarget::Member(member) => self.evaluate_member(member)?,
                AssignmentTarget::Pattern(_) => {
                    return Err(JsError::syntax_error("Invalid assignment target", 0, 0));
                }
            };

            match assign.operator {
                AssignmentOp::AddAssign => {
                    if left.is_string() || right.is_string() {
                        JsValue::String(left.to_js_string() + &right.to_js_string())
                    } else {
                        JsValue::Number(left.to_number() + right.to_number())
                    }
                }
                AssignmentOp::SubAssign => JsValue::Number(left.to_number() - right.to_number()),
                AssignmentOp::MulAssign => JsValue::Number(left.to_number() * right.to_number()),
                AssignmentOp::DivAssign => JsValue::Number(left.to_number() / right.to_number()),
                AssignmentOp::ModAssign => JsValue::Number(left.to_number() % right.to_number()),
                AssignmentOp::ExpAssign => JsValue::Number(left.to_number().powf(right.to_number())),
                AssignmentOp::BitAndAssign => JsValue::Number((left.to_number() as i32 & right.to_number() as i32) as f64),
                AssignmentOp::BitOrAssign => JsValue::Number((left.to_number() as i32 | right.to_number() as i32) as f64),
                AssignmentOp::BitXorAssign => JsValue::Number((left.to_number() as i32 ^ right.to_number() as i32) as f64),
                AssignmentOp::LShiftAssign => JsValue::Number(((left.to_number() as i32) << (right.to_number() as u32 & 0x1f)) as f64),
                AssignmentOp::RShiftAssign => JsValue::Number(((left.to_number() as i32) >> (right.to_number() as u32 & 0x1f)) as f64),
                AssignmentOp::URShiftAssign => JsValue::Number((((left.to_number() as i32) as u32) >> (right.to_number() as u32 & 0x1f)) as f64),
                AssignmentOp::AndAssign => {
                    if !left.to_boolean() {
                        left
                    } else {
                        right
                    }
                }
                AssignmentOp::OrAssign => {
                    if left.to_boolean() {
                        left
                    } else {
                        right
                    }
                }
                AssignmentOp::NullishAssign => {
                    if left.is_null_or_undefined() {
                        right
                    } else {
                        left
                    }
                }
                AssignmentOp::Assign => unreachable!(),
            }
        } else {
            right
        };

        match &assign.left {
            AssignmentTarget::Identifier(id) => {
                self.env.set(&id.name, value.clone())?;
            }
            AssignmentTarget::Member(member) => {
                self.set_member(member, value.clone())?;
            }
            AssignmentTarget::Pattern(pattern) => {
                self.bind_pattern(pattern, value.clone(), true)?;
            }
        }

        Ok(value)
    }

    fn evaluate_update(&mut self, update: &UpdateExpression) -> Result<JsValue, JsError> {
        let old_value = self.evaluate(&update.argument)?;
        let old_num = old_value.to_number();

        let new_value = match update.operator {
            UpdateOp::Increment => JsValue::Number(old_num + 1.0),
            UpdateOp::Decrement => JsValue::Number(old_num - 1.0),
        };

        // Set the new value
        match update.argument.as_ref() {
            Expression::Identifier(id) => {
                self.env.set(&id.name, new_value.clone())?;
            }
            Expression::Member(member) => {
                self.set_member(member, new_value.clone())?;
            }
            _ => return Err(JsError::syntax_error("Invalid update target", 0, 0)),
        }

        Ok(if update.prefix { new_value } else { JsValue::Number(old_num) })
    }

    fn evaluate_member(&mut self, member: &MemberExpression) -> Result<JsValue, JsError> {
        let object = self.evaluate(&member.object)?;

        let key = match &member.property {
            MemberProperty::Identifier(id) => crate::value::PropertyKey::from(id.name.as_str()),
            MemberProperty::Expression(expr) => {
                let val = self.evaluate(expr)?;
                crate::value::PropertyKey::from_value(&val)
            }
            MemberProperty::PrivateIdentifier(id) => {
                // Private fields are stored with # prefix
                crate::value::PropertyKey::from(format!("#{}", id.name))
            }
        };

        match object.clone() {
            JsValue::Object(obj) => {
                // First, try own properties and prototype chain with accessor support
                // We need to drop the borrow before calling the getter
                let property_result = {
                    if let Some((prop, _)) = obj.borrow().get_property_descriptor(&key) {
                        // If this is an accessor property with a getter, return the getter
                        if let Some(ref getter) = prop.getter {
                            Some((true, Some(getter.clone()), JsValue::Undefined))
                        } else if prop.is_accessor() {
                            // Getter-less accessor property returns undefined
                            Some((false, None, JsValue::Undefined))
                        } else {
                            Some((false, None, prop.value.clone()))
                        }
                    } else {
                        None
                    }
                };

                if let Some((is_getter, getter, value)) = property_result {
                    if is_getter {
                        if let Some(getter_fn) = getter {
                            return self.call_function(JsValue::Object(getter_fn), object, vec![]);
                        }
                    }
                    return Ok(value);
                }

                // For functions, check Function.prototype
                if obj.borrow().is_callable() {
                    if let Some(method) = self.function_prototype.borrow().get_property(&key) {
                        return Ok(method);
                    }
                }
                // Fall back to Object.prototype for ordinary objects
                // (but not for objects created with Object.create(null))
                if !obj.borrow().null_prototype {
                    if let Some(method) = self.object_prototype.borrow().get_property(&key) {
                        return Ok(method);
                    }
                }
                Ok(JsValue::Undefined)
            }
            JsValue::String(s) => {
                // String indexing
                if let crate::value::PropertyKey::Index(i) = key {
                    if let Some(ch) = s.as_str().chars().nth(i as usize) {
                        return Ok(JsValue::String(JsString::from(ch.to_string())));
                    }
                }
                if key.to_string() == "length" {
                    return Ok(JsValue::Number(s.len() as f64));
                }
                // Look up on String.prototype
                if let Some(method) = self.string_prototype.borrow().get_property(&key) {
                    return Ok(method);
                }
                Ok(JsValue::Undefined)
            }
            JsValue::Number(_) => {
                // Look up on Number.prototype
                if let Some(method) = self.number_prototype.borrow().get_property(&key) {
                    return Ok(method);
                }
                Ok(JsValue::Undefined)
            }
            JsValue::Symbol(ref s) => {
                // Handle special symbol properties
                if key.to_string() == "description" {
                    return Ok(match &s.description {
                        Some(desc) => JsValue::String(JsString::from(desc.as_str())),
                        None => JsValue::Undefined,
                    });
                }
                // Look up on Symbol.prototype
                if let Some(method) = self.symbol_prototype.borrow().get_property(&key) {
                    return Ok(method);
                }
                Ok(JsValue::Undefined)
            }
            _ => Ok(JsValue::Undefined),
        }
    }

    fn set_member(&mut self, member: &MemberExpression, value: JsValue) -> Result<(), JsError> {
        let object = self.evaluate(&member.object)?;

        let key = match &member.property {
            MemberProperty::Identifier(id) => crate::value::PropertyKey::from(id.name.as_str()),
            MemberProperty::Expression(expr) => {
                let val = self.evaluate(expr)?;
                crate::value::PropertyKey::from_value(&val)
            }
            MemberProperty::PrivateIdentifier(id) => {
                // Private fields are stored with # prefix
                crate::value::PropertyKey::from(format!("#{}", id.name))
            }
        };

        match object.clone() {
            JsValue::Object(obj) => {
                // Check if there's an accessor property with a setter
                // We need to drop the borrow before calling the setter
                let setter_fn = {
                    if let Some((prop, _)) = obj.borrow().get_property_descriptor(&key) {
                        if prop.is_accessor() {
                            if let Some(ref setter) = prop.setter {
                                Some(setter.clone())
                            } else {
                                // Accessor property without setter - silently ignore in non-strict mode
                                return Ok(());
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                if let Some(setter) = setter_fn {
                    self.call_function(JsValue::Object(setter), object, vec![value])?;
                    return Ok(());
                }

                obj.borrow_mut().set_property(key, value);
                Ok(())
            }
            _ => Err(JsError::type_error("Cannot set property on non-object")),
        }
    }

    fn evaluate_call(&mut self, call: &CallExpression) -> Result<JsValue, JsError> {
        // Determine 'this' binding
        // For super() calls, use the current this value
        // For super.method() calls, also use the current this value
        let this_value = if let Expression::Super(_) = call.callee.as_ref() {
            // super() - call parent constructor with current this
            self.env.get("this").unwrap_or(JsValue::Undefined)
        } else if let Expression::Member(member) = call.callee.as_ref() {
            if let Expression::Super(_) = member.object.as_ref() {
                // super.method() - call with current this
                self.env.get("this").unwrap_or(JsValue::Undefined)
            } else {
                self.evaluate(&member.object)?
            }
        } else {
            JsValue::Undefined
        };

        // For super.method(), we need to look up the method on the super prototype
        let callee = if let Expression::Member(member) = call.callee.as_ref() {
            if let Expression::Super(_) = member.object.as_ref() {
                // Get super constructor
                let super_ctor = self.env.get("__super__").map_err(|_| {
                    JsError::reference_error("'super' keyword is not available in this context")
                })?;
                // Get super prototype
                if let JsValue::Object(ctor) = super_ctor {
                    let proto = ctor.borrow().get_property(&PropertyKey::from("prototype"));
                    if let Some(JsValue::Object(proto_obj)) = proto {
                        // Get the method from prototype
                        let key = match &member.property {
                            MemberProperty::Identifier(id) => PropertyKey::from(id.name.as_str()),
                            MemberProperty::Expression(expr) => {
                                let val = self.evaluate(expr)?;
                                PropertyKey::from_value(&val)
                            }
                            MemberProperty::PrivateIdentifier(id) => {
                                PropertyKey::from(format!("#{}", id.name))
                            }
                        };
                        proto_obj.borrow().get_property(&key).unwrap_or(JsValue::Undefined)
                    } else {
                        return Err(JsError::type_error("Super has no prototype"));
                    }
                } else {
                    return Err(JsError::type_error("Super is not an object"));
                }
            } else {
                self.evaluate(&call.callee)?
            }
        } else {
            self.evaluate(&call.callee)?
        };

        // Evaluate arguments
        let mut args = vec![];
        for arg in &call.arguments {
            match arg {
                Argument::Expression(expr) => {
                    args.push(self.evaluate(expr)?);
                }
                Argument::Spread(spread) => {
                    let val = self.evaluate(&spread.argument)?;
                    if let JsValue::Object(obj) = val {
                        let obj_ref = obj.borrow();
                        if let ExoticObject::Array { length } = &obj_ref.exotic {
                            for i in 0..*length {
                                if let Some(v) = obj_ref.get_property(&PropertyKey::Index(i)) {
                                    args.push(v);
                                }
                            }
                        }
                    }
                }
            }
        }

        self.call_function(callee, this_value, args)
    }

    fn evaluate_new(&mut self, new_expr: &NewExpression) -> Result<JsValue, JsError> {
        let callee = self.evaluate(&new_expr.callee)?;

        let mut args = vec![];
        for arg in &new_expr.arguments {
            match arg {
                Argument::Expression(expr) => {
                    args.push(self.evaluate(expr)?);
                }
                Argument::Spread(spread) => {
                    let val = self.evaluate(&spread.argument)?;
                    if let JsValue::Object(obj) = val {
                        let obj_ref = obj.borrow();
                        if let ExoticObject::Array { length } = &obj_ref.exotic {
                            for i in 0..*length {
                                if let Some(v) = obj_ref.get_property(&PropertyKey::Index(i)) {
                                    args.push(v);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Create new object with prototype from constructor
        let new_obj = create_object();

        // Get prototype from constructor.prototype and set it on the new object
        if let JsValue::Object(ctor_obj) = &callee {
            let ctor_ref = ctor_obj.borrow();
            if let Some(JsValue::Object(proto)) = ctor_ref.get_property(&PropertyKey::from("prototype")) {
                drop(ctor_ref);
                new_obj.borrow_mut().prototype = Some(proto);
            } else {
                drop(ctor_ref);
            }

            // Initialize instance fields from __fields__
            let fields = ctor_obj.borrow().get_property(&PropertyKey::from("__fields__"));
            if let Some(JsValue::Object(fields_arr)) = fields {
                let fields_ref = fields_arr.borrow();
                if let ExoticObject::Array { length } = fields_ref.exotic {
                    for i in 0..length {
                        if let Some(JsValue::Object(pair)) = fields_ref.get_property(&PropertyKey::Index(i)) {
                            let pair_ref = pair.borrow();
                            if let Some(JsValue::String(name)) = pair_ref.get_property(&PropertyKey::Index(0)) {
                                let value = pair_ref.get_property(&PropertyKey::Index(1))
                                    .unwrap_or(JsValue::Undefined);
                                drop(pair_ref);
                                new_obj.borrow_mut().set_property(
                                    PropertyKey::from(name.to_string()),
                                    value,
                                );
                            }
                        }
                    }
                }
            }
        }

        // Call constructor
        let result = self.call_function(callee, JsValue::Object(new_obj.clone()), args)?;

        // Return result if it's an object, otherwise return new_obj
        match result {
            JsValue::Object(_) => Ok(result),
            _ => Ok(JsValue::Object(new_obj)),
        }
    }

    pub fn call_function(
        &mut self,
        callee: JsValue,
        this_value: JsValue,
        args: Vec<JsValue>,
    ) -> Result<JsValue, JsError> {
        let JsValue::Object(obj) = callee else {
            return Err(JsError::type_error("Not a function"));
        };

        let func = {
            let obj_ref = obj.borrow();
            match &obj_ref.exotic {
                ExoticObject::Function(f) => f.clone(),
                _ => return Err(JsError::type_error("Not a function")),
            }
        };

        match func {
            JsFunction::Interpreted(interpreted) => {
                let prev_env = self.env.clone();
                self.env = Environment::with_outer(interpreted.closure.clone());

                // Bind 'this' value
                self.env.define("this".to_string(), this_value.clone(), false);

                // Create and bind 'arguments' object (array-like object with all args)
                let arguments_obj = self.create_array(args.clone());
                self.env.define("arguments".to_string(), JsValue::Object(arguments_obj), false);

                // Check if function has __super__ (for class constructors/methods)
                let super_ctor = obj.borrow().get_property(&PropertyKey::from("__super__"));
                if let Some(super_val) = super_ctor {
                    self.env.define("__super__".to_string(), super_val, false);
                }

                // Hoist var declarations before anything else
                if let FunctionBody::Block(block) = &interpreted.body {
                    self.hoist_var_declarations(&block.body);
                }

                // Bind parameters
                for (i, param) in interpreted.params.iter().enumerate() {
                    // Check if this is a rest parameter
                    if let Pattern::Rest(rest) = &param.pattern {
                        // Collect remaining arguments into an array
                        let rest_args: Vec<JsValue> = args[i..].to_vec();
                        let rest_array = JsValue::Object(self.create_array(rest_args));
                        self.bind_pattern(&rest.argument, rest_array, true)?;
                        break; // Rest param must be last
                    } else {
                        let arg = args.get(i).cloned().unwrap_or(JsValue::Undefined);
                        self.bind_pattern(&param.pattern, arg, true)?;
                    }
                }

                // Execute body
                let result = match &interpreted.body {
                    FunctionBody::Block(block) => {
                        match self.execute_block(block)? {
                            Completion::Return(val) => val,
                            Completion::Normal(val) => val,
                            _ => JsValue::Undefined,
                        }
                    }
                    FunctionBody::Expression(expr) => self.evaluate(expr)?,
                };

                self.env = prev_env;
                Ok(result)
            }

            JsFunction::Native(native) => {
                (native.func)(self, this_value, args)
            }

            JsFunction::Bound(bound_data) => {
                // For bound functions:
                // - Use the bound this value (ignore the passed this_value)
                // - Prepend bound args to the call args
                let bound_this = bound_data.this_arg.clone();
                let mut full_args = bound_data.bound_args.clone();
                full_args.extend(args);

                // Call the target function with bound this and combined args
                self.call_function(
                    JsValue::Object(bound_data.target.clone()),
                    bound_this,
                    full_args,
                )
            }
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}


