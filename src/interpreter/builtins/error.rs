//! Error constructor built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{create_function, create_object, JsFunction, JsObjectRef, JsString, JsValue, NativeFunction, PropertyKey};

/// Create Error.prototype with toString
pub fn create_error_prototype() -> JsObjectRef {
    let proto = create_object();
    {
        let mut p = proto.borrow_mut();

        // Set default name and message
        p.set_property(PropertyKey::from("name"), JsValue::String(JsString::from("Error")));
        p.set_property(PropertyKey::from("message"), JsValue::String(JsString::from("")));

        let tostring_fn = create_function(JsFunction::Native(NativeFunction {
            name: "toString".to_string(),
            func: error_to_string,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("toString"), JsValue::Object(tostring_fn));
    }
    proto
}

/// Returns (Error, TypeError, ReferenceError, SyntaxError, RangeError) constructors and the Error.prototype
pub fn create_error_constructors(error_prototype: &JsObjectRef) -> (JsObjectRef, JsObjectRef, JsObjectRef, JsObjectRef, JsObjectRef) {
    let error_fn = create_function(JsFunction::Native(NativeFunction {
        name: "Error".to_string(),
        func: error_constructor,
        arity: 1,
    }));
    error_fn.borrow_mut().set_property(PropertyKey::from("prototype"), JsValue::Object(error_prototype.clone()));

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

/// Error.prototype.toString()
/// Returns "name: message" or just "name" if message is empty
pub fn error_to_string(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(obj) = this else {
        return Ok(JsValue::String(JsString::from("Error")));
    };

    let obj_ref = obj.borrow();

    // Get name, default to "Error"
    let name = obj_ref.get_property(&PropertyKey::from("name"))
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| "Error".to_string());

    // Get message, default to ""
    let message = obj_ref.get_property(&PropertyKey::from("message"))
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();

    if message.is_empty() {
        Ok(JsValue::String(JsString::from(name)))
    } else {
        Ok(JsValue::String(JsString::from(format!("{}: {}", name, message))))
    }
}

fn create_error_object_with_stack(
    interp: &Interpreter,
    name: &str,
    message: JsValue,
    prototype: Option<JsObjectRef>,
) -> JsValue {
    let msg_str = match &message {
        JsValue::Undefined => JsString::from(""),
        other => other.to_js_string(),
    };

    // Capture stack trace
    let stack_trace = interp.format_stack_trace(name, &msg_str.to_string());

    let obj = create_object();
    {
        let mut obj_ref = obj.borrow_mut();
        obj_ref.set_property(PropertyKey::from("name"), JsValue::String(JsString::from(name)));
        obj_ref.set_property(PropertyKey::from("message"), JsValue::String(msg_str));
        obj_ref.set_property(PropertyKey::from("stack"), JsValue::String(JsString::from(stack_trace)));
        if let Some(proto) = prototype {
            obj_ref.prototype = Some(proto);
        }
    }
    JsValue::Object(obj)
}

pub fn error_constructor(interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    Ok(create_error_object_with_stack(interp, "Error", message, Some(interp.error_prototype.clone())))
}

pub fn type_error_constructor(interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    Ok(create_error_object_with_stack(interp, "TypeError", message, Some(interp.error_prototype.clone())))
}

pub fn reference_error_constructor(interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    Ok(create_error_object_with_stack(interp, "ReferenceError", message, Some(interp.error_prototype.clone())))
}

pub fn syntax_error_constructor(interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    Ok(create_error_object_with_stack(interp, "SyntaxError", message, Some(interp.error_prototype.clone())))
}

pub fn range_error_constructor(interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    Ok(create_error_object_with_stack(interp, "RangeError", message, Some(interp.error_prototype.clone())))
}
