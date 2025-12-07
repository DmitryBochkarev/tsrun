//! Interpreter for executing TypeScript AST

// Builtin function implementations (split into separate files)
pub mod builtins;

// Import all builtin functions
use builtins::*;

use crate::ast::{
    Argument, ArrayElement, AssignmentExpression, AssignmentOp, AssignmentTarget, BinaryExpression,
    BinaryOp, BlockStatement, CallExpression, ClassConstructor, ClassDeclaration, ClassMember,
    ClassMethod, ClassProperty, ConditionalExpression, EnumDeclaration, Expression, ForInOfLeft,
    ForInStatement, ForInit, ForOfStatement, ForStatement, FunctionDeclaration, LiteralValue,
    LogicalExpression, LogicalOp, MemberExpression, MemberProperty, NewExpression,
    ObjectPatternProperty, ObjectProperty, ObjectPropertyKey, Pattern, Program, Statement,
    UnaryExpression, UnaryOp, UpdateExpression, UpdateOp, VariableDeclaration, VariableKind,
};
use crate::error::JsError;
use crate::value::{
    create_array, create_function, create_object, Environment, ExoticObject, FunctionBody,
    InterpretedFunction, JsFunction, JsObjectRef, JsString, JsValue, PropertyKey,
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
        let (error_fn, type_error_fn, reference_error_fn, syntax_error_fn, range_error_fn) = create_error_constructors();
        env.define("Error".to_string(), JsValue::Object(error_fn), false);
        env.define("TypeError".to_string(), JsValue::Object(type_error_fn), false);
        env.define("ReferenceError".to_string(), JsValue::Object(reference_error_fn), false);
        env.define("SyntaxError".to_string(), JsValue::Object(syntax_error_fn), false);
        env.define("RangeError".to_string(), JsValue::Object(range_error_fn), false);

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
                        Completion::Break(_) => break,
                        Completion::Continue(_) => continue,
                        Completion::Return(val) => return Ok(Completion::Return(val)),
                        Completion::Normal(_) => {}
                    }
                }
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::DoWhile(do_while) => {
                loop {
                    match self.execute_statement(&do_while.body)? {
                        Completion::Break(_) => break,
                        Completion::Continue(_) => {}
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
                Err(JsError::RuntimeError {
                    kind: "Error".to_string(),
                    message: value.to_js_string().to_string(),
                    stack: vec![],
                })
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
                            // Create error value
                            let error_value = JsValue::from(err.to_string());

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
                            Err(err)
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

            Statement::Import(_) | Statement::Export(_) => {
                // Module handling would go here
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Empty | Statement::Debugger => {
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Labeled(labeled) => {
                self.execute_statement(&labeled.body)
            }
        }
    }

    fn execute_variable_declaration(&mut self, decl: &VariableDeclaration) -> Result<(), JsError> {
        let mutable = decl.kind != VariableKind::Const;

        for declarator in &decl.declarations {
            let value = if let Some(init) = &declarator.init {
                self.evaluate(init)?
            } else {
                JsValue::Undefined
            };

            self.bind_pattern(&declarator.id, value, mutable)?;
        }

        Ok(())
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

        if let Some(id) = &class.id {
            self.env.define(id.name.clone(), JsValue::Object(constructor_fn), false);
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
                    // TODO: implement static initialization blocks
                }
            }
        }

        // Add instance methods to prototype
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

            // For now, treat all methods as regular methods
            // TODO: implement proper getter/setter support
            prototype.borrow_mut().set_property(
                PropertyKey::from(method_name),
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

        // Add static methods to constructor function
        for method in &static_methods {
            let method_name = match &method.key {
                ObjectPropertyKey::Identifier(id) => id.name.clone(),
                ObjectPropertyKey::String(s) => s.value.clone(),
                ObjectPropertyKey::Number(lit) => match &lit.value {
                    LiteralValue::Number(n) => n.to_string(),
                    _ => continue,
                },
                ObjectPropertyKey::Computed(_) => continue,
                ObjectPropertyKey::PrivateIdentifier(_) => continue,
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
            constructor_fn.borrow_mut().set_property(
                PropertyKey::from(method_name),
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
        let prev_env = self.env.clone();
        self.env = Environment::with_outer(self.env.clone());

        // Init
        if let Some(init) = &for_stmt.init {
            match init {
                ForInit::Variable(decl) => {
                    self.execute_variable_declaration(decl)?;
                }
                ForInit::Expression(expr) => {
                    self.evaluate(expr)?;
                }
            }
        }

        // Loop
        loop {
            // Test
            if let Some(test) = &for_stmt.test {
                let test_val = self.evaluate(test)?;
                if !test_val.to_boolean() {
                    break;
                }
            }

            // Body
            match self.execute_statement(&for_stmt.body)? {
                Completion::Break(_) => break,
                Completion::Continue(_) => {}
                Completion::Return(val) => {
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {}
            }

            // Update
            if let Some(update) = &for_stmt.update {
                self.evaluate(update)?;
            }
        }

        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_for_in(&mut self, for_in: &ForInStatement) -> Result<Completion, JsError> {
        let right = self.evaluate(&for_in.right)?;

        let keys = match &right {
            JsValue::Object(obj) => {
                obj.borrow()
                    .properties
                    .iter()
                    .filter(|(_, prop)| prop.enumerable)
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
                Completion::Break(_) => break,
                Completion::Continue(_) => continue,
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
                Completion::Break(_) => break,
                Completion::Continue(_) => continue,
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

    /// Evaluate an expression
    pub fn evaluate(&mut self, expr: &Expression) -> Result<JsValue, JsError> {
        match expr {
            Expression::Literal(lit) => self.evaluate_literal(&lit.value),

            Expression::Identifier(id) => {
                self.env
                    .get(&id.name)
                    .ok_or_else(|| JsError::reference_error(&id.name))
            }

            Expression::This(_) => {
                // Look up 'this' from the environment
                Ok(self.env.get("this").unwrap_or(JsValue::Undefined))
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
                match self.env.get("__super__") {
                    Some(val) => Ok(val),
                    None => Err(JsError::reference_error("'super' keyword is not available in this context")),
                }
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

        match object {
            JsValue::Object(obj) => {
                // First, try own properties and prototype chain
                if let Some(val) = obj.borrow().get_property(&key) {
                    return Ok(val);
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

        match object {
            JsValue::Object(obj) => {
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
                let super_ctor = self.env.get("__super__").ok_or_else(|| {
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


