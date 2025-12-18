//! Function.prototype built-in methods (call, apply, bind)

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{BoundFunctionData, Guarded, JsFunction, JsValue};

/// Initialize Function.prototype with call, apply, bind methods
pub fn init_function_prototype(interp: &mut Interpreter) {
    let proto = interp.function_prototype.clone();

    interp.register_method(&proto, "call", function_call, 1);
    interp.register_method(&proto, "apply", function_apply, 2);
    interp.register_method(&proto, "bind", function_bind, 1);
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
            ))
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
