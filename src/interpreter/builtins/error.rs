//! Error constructor built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{create_object, JsString, JsValue, PropertyKey};

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
