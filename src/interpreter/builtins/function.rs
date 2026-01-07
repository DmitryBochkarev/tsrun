//! Function.prototype built-in methods (call, apply, bind) and Function constructor

use crate::prelude::{Box, Rc, String, ToString, Vec, format, vec};

use crate::ast::{Expression, Statement};
use crate::compiler::Compiler;
use crate::error::JsError;
use crate::gc::Gc;
use crate::interpreter::Interpreter;
use crate::parser::Parser;
use crate::value::{
    BoundFunctionData, BytecodeFunction, CheapClone, ExoticObject, Guarded, JsFunction, JsObject,
    JsString, JsSymbol, JsValue, NativeFunction, PropertyKey,
};

/// The Function.prototype function itself - accepts any arguments and returns undefined.
/// Per ES spec, "The Function prototype object is itself a built-in function object."
pub fn function_prototype_fn(
    _interp: &mut Interpreter,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    // Function.prototype() returns undefined
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// Initialize Function.prototype with call, apply, bind methods
pub fn init_function_prototype(interp: &mut Interpreter) {
    let proto = interp.function_prototype.clone();

    // Make Function.prototype callable by setting it as a native function
    // This must happen before we register methods on it
    {
        let mut proto_ref = proto.borrow_mut();
        let name = interp.intern("");
        proto_ref.exotic = ExoticObject::Function(JsFunction::Native(NativeFunction {
            name: name.cheap_clone(),
            func: function_prototype_fn,
            arity: 0,
            ffi_id: 0,
        }));
        // Set length = 0 and name = "" as per spec
        let length_key = PropertyKey::String(interp.intern("length"));
        let name_key = PropertyKey::String(interp.intern("name"));
        proto_ref.set_property(length_key, JsValue::Number(0.0));
        proto_ref.set_property(name_key, JsValue::String(name));
    }

    interp.register_method(&proto, "call", function_call, 1);
    interp.register_method(&proto, "apply", function_apply, 2);
    interp.register_method(&proto, "bind", function_bind, 1);

    // Add Symbol.hasInstance method
    let well_known = interp.well_known_symbols;
    let has_instance_symbol = JsSymbol::new(
        well_known.has_instance,
        Some(interp.intern("Symbol.hasInstance")),
    );
    let has_instance_key = PropertyKey::Symbol(Box::new(has_instance_symbol));
    let has_instance_fn =
        interp.create_native_function("[Symbol.hasInstance]", function_has_instance, 1);
    proto
        .borrow_mut()
        .set_property(has_instance_key, JsValue::Object(has_instance_fn));

    // Function.prototype.constructor will be set to the Function constructor
    // after it's created in create_function_constructor
}

/// Function.prototype[Symbol.hasInstance](V)
/// Returns true if V is an instance of this function (i.e., V's prototype chain includes this.prototype)
fn function_has_instance(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);

    // 1. If V is not an object, return false
    let JsValue::Object(obj) = value else {
        return Ok(Guarded::unguarded(JsValue::Boolean(false)));
    };

    // 2. Let F be the this value (the function)
    let JsValue::Object(func) = this else {
        return Err(JsError::type_error(
            "Function.prototype[Symbol.hasInstance] called on non-function",
        ));
    };

    // 3. Return OrdinaryHasInstance(F, V)
    // Get the prototype property of F
    let prototype_key = PropertyKey::String(interp.intern("prototype"));
    let prototype_value = func.borrow().get_property(&prototype_key);

    // If F.prototype is not an object, return false
    let Some(JsValue::Object(prototype)) = prototype_value else {
        // For functions without a prototype property (like arrow functions),
        // we should return false, not throw
        return Ok(Guarded::unguarded(JsValue::Boolean(false)));
    };

    // Walk the prototype chain of V looking for F.prototype
    let mut current = obj.borrow().prototype.clone();
    while let Some(proto) = current {
        if proto == prototype {
            return Ok(Guarded::unguarded(JsValue::Boolean(true)));
        }
        current = proto.borrow().prototype.clone();
    }

    Ok(Guarded::unguarded(JsValue::Boolean(false)))
}

/// Create the global Function constructor
pub fn create_function_constructor(interp: &mut Interpreter) -> Gc<JsObject> {
    let constructor = interp.create_native_function("Function", function_constructor_fn, 1);

    // Set Function.prototype
    let proto_key = PropertyKey::String(interp.intern("prototype"));
    constructor.borrow_mut().set_property(
        proto_key,
        JsValue::Object(interp.function_prototype.clone()),
    );

    // Set Function.prototype.constructor = Function
    let ctor_key = PropertyKey::String(interp.intern("constructor"));
    interp
        .function_prototype
        .borrow_mut()
        .set_property(ctor_key, JsValue::Object(constructor.clone()));

    constructor
}

/// The Function constructor: new Function([p1[, p2[, ...pN]],] body)
///
/// Creates a new function from strings. The last argument is the function body,
/// all preceding arguments are parameter names. Parameter strings can contain
/// comma-separated parameter names.
///
/// The function is created with global scope (no access to local variables).
fn function_constructor_fn(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // Collect parameter strings and body string
    let (param_strings, body_string) = if args.is_empty() {
        // new Function() - empty function
        (Vec::new(), String::new())
    } else if args.len() == 1 {
        // new Function(body) - no parameters
        let body = args.first().map(js_value_to_string).unwrap_or_default();
        (Vec::new(), body)
    } else {
        // new Function(p1, p2, ..., body)
        let body = args.last().map(js_value_to_string).unwrap_or_default();
        let params: Vec<String> = args
            .iter()
            .take(args.len() - 1)
            .map(js_value_to_string)
            .collect();
        (params, body)
    };

    // Parse parameter strings
    // Each param string can be "x" or "x, y, z" (comma-separated)
    let param_source = build_param_string(&param_strings);

    // Build the source code as a function expression
    // We wrap it as: (function anonymous(params) { body })
    let source = format!(
        "(function anonymous({}) {{ {} }})",
        param_source, body_string
    );

    // Parse the function
    let mut parser = Parser::new(&source, &mut interp.string_dict);
    let program = parser
        .parse_program()
        .map_err(|e| JsError::syntax_error_simple(format!("Invalid function body: {}", e)))?;

    // Extract the function expression from the parsed program
    // The program should contain one expression statement with a parenthesized function
    let func_expr = extract_function_expression(&program)?;

    // Compile the function body to bytecode
    let chunk = Compiler::compile_function_body_direct(
        &func_expr.params,
        &func_expr.body.body,
        Some(JsString::from("anonymous")),
        false, // not a generator
        false, // not async
    )
    .map_err(|e| JsError::syntax_error_simple(format!("Failed to compile function: {}", e)))?;

    // Create bytecode function with global scope as closure
    // This is what makes Function() different from regular function declarations
    // (which capture the local scope)
    let bc_func = BytecodeFunction {
        chunk: Rc::new(chunk),
        closure: interp.global_env.clone(),
        captured_this: None,
    };

    let guard = interp.heap.create_guard();
    let func_obj = interp.create_bytecode_function(&guard, bc_func);

    Ok(Guarded::with_guard(JsValue::Object(func_obj), guard))
}

/// Convert a JsValue to a string for parameter/body parsing
fn js_value_to_string(value: &JsValue) -> String {
    match value {
        JsValue::String(s) => s.to_string(),
        JsValue::Number(n) => {
            if n.is_nan() {
                "NaN".to_string()
            } else if n.is_infinite() {
                if *n > 0.0 {
                    "Infinity".to_string()
                } else {
                    "-Infinity".to_string()
                }
            } else {
                n.to_string()
            }
        }
        JsValue::Boolean(b) => b.to_string(),
        JsValue::Undefined => "undefined".to_string(),
        JsValue::Null => "null".to_string(),
        JsValue::Object(_) => "[object Object]".to_string(),
        JsValue::Symbol(_) => {
            // Symbols can't be converted to string in this context
            "".to_string()
        }
    }
}

/// Build a parameter string from multiple parameter strings
/// Each string can be "x" or "x, y, z"
fn build_param_string(param_strings: &[String]) -> String {
    param_strings
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Extract the FunctionExpression from a parsed program
/// Expects the program to be: (function anonymous(...) { ... })
fn extract_function_expression(
    program: &crate::ast::Program,
) -> Result<&crate::ast::FunctionExpression, JsError> {
    // Should have exactly one statement
    let stmt = program
        .body
        .first()
        .ok_or_else(|| JsError::syntax_error_simple("Failed to parse function"))?;

    // Should be an expression statement
    let expr_stmt = match stmt {
        Statement::Expression(expr_stmt) => expr_stmt,
        _ => return Err(JsError::syntax_error_simple("Expected function expression")),
    };

    // Should be a parenthesized expression containing a function
    let inner = match &*expr_stmt.expression {
        Expression::Parenthesized(inner, _) => inner,
        Expression::Function(f) => return Ok(f),
        _ => return Err(JsError::syntax_error_simple("Expected function expression")),
    };

    // The inner expression should be a function
    match &**inner {
        Expression::Function(f) => Ok(f),
        _ => Err(JsError::syntax_error_simple("Expected function expression")),
    }
}

// Function.prototype.call - call function with specified this value and arguments
pub fn function_call(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // `this` is the function to call
    // args[0] is the thisArg for the call
    // args[1..] are the arguments
    let this_arg = args.first().cloned().unwrap_or(JsValue::Undefined);
    let call_args: Vec<JsValue> = args.iter().skip(1).cloned().collect();
    // Propagate the Guarded from call_function
    interp.call_function(this, this_arg, &call_args)
}

// Function.prototype.apply - call function with specified this value and array of arguments
pub fn function_apply(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // `this` is the function to call
    // args[0] is the thisArg for the call
    // args[1] is an array of arguments
    let this_arg = args.first().cloned().unwrap_or(JsValue::Undefined);
    let args_array = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let call_args: Vec<JsValue> = match args_array {
        JsValue::Object(arr_ref) => {
            let arr = arr_ref.borrow();
            if let Some(elements) = arr.array_elements() {
                elements.to_vec()
            } else {
                vec![]
            }
        }
        JsValue::Undefined | JsValue::Null => vec![],
        _ => {
            return Err(JsError::type_error(
                "Second argument to apply must be an array",
            ));
        }
    };

    // Propagate the Guarded from call_function
    interp.call_function(this, this_arg, &call_args)
}

// Function.prototype.bind - create a new function with bound this value and pre-filled arguments
pub fn function_bind(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // `this` is the function to bind
    // args[0] is the thisArg to bind
    // args[1..] are pre-filled arguments
    let JsValue::Object(target_fn) = this else {
        return Err(JsError::type_error("Bind must be called on a function"));
    };

    // Verify it's actually a function
    if !target_fn.borrow().is_callable() {
        return Err(JsError::type_error("Bind must be called on a function"));
    }

    let this_arg = args.first().cloned().unwrap_or(JsValue::Undefined);
    let bound_args: Vec<JsValue> = args.iter().skip(1).cloned().collect();

    // Create a bound function using JsFunction::Bound
    let guard = interp.heap.create_guard();
    let bound_fn = interp.create_js_function(
        &guard,
        JsFunction::Bound(Box::new(BoundFunctionData {
            target: target_fn,
            this_arg: this_arg.clone(),
            bound_args: bound_args.clone(),
        })),
    );

    Ok(Guarded::with_guard(JsValue::Object(bound_fn), guard))
}
