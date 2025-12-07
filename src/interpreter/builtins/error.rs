//! Error constructor built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{create_function, create_object, JsFunction, JsObjectRef, JsString, JsValue, NativeFunction, PropertyKey};

/// Returns (Error, TypeError, ReferenceError, SyntaxError, RangeError) constructors
pub fn create_error_constructors() -> (JsObjectRef, JsObjectRef, JsObjectRef, JsObjectRef, JsObjectRef) {
    let error_fn = create_function(JsFunction::Native(NativeFunction {
        name: "Error".to_string(),
        func: error_constructor,
        arity: 1,
    }));

    let type_error_fn = create_function(JsFunction::Native(NativeFunction {
        name: "TypeError".to_string(),
        func: type_error_constructor,
        arity: 1,
    }));

    let reference_error_fn = create_function(JsFunction::Native(NativeFunction {
        name: "ReferenceError".to_string(),
        func: reference_error_constructor,
        arity: 1,
    }));

    let syntax_error_fn = create_function(JsFunction::Native(NativeFunction {
        name: "SyntaxError".to_string(),
        func: syntax_error_constructor,
        arity: 1,
    }));

    let range_error_fn = create_function(JsFunction::Native(NativeFunction {
        name: "RangeError".to_string(),
        func: range_error_constructor,
        arity: 1,
    }));

    (error_fn, type_error_fn, reference_error_fn, syntax_error_fn, range_error_fn)
}

fn create_error_object(name: &str, message: JsValue) -> JsValue {
    let msg_str = match message {
        JsValue::Undefined => JsString::from(""),
        other => other.to_js_string(),
    };

    let obj = create_object();
    obj.borrow_mut().set_property(PropertyKey::from("name"), JsValue::String(JsString::from(name)));
    obj.borrow_mut().set_property(PropertyKey::from("message"), JsValue::String(msg_str));
    JsValue::Object(obj)
}

pub fn error_constructor(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    Ok(create_error_object("Error", message))
}

pub fn type_error_constructor(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    Ok(create_error_object("TypeError", message))
}

pub fn reference_error_constructor(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    Ok(create_error_object("ReferenceError", message))
}

pub fn syntax_error_constructor(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    Ok(create_error_object("SyntaxError", message))
}

pub fn range_error_constructor(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    Ok(create_error_object("RangeError", message))
}
