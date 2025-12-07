//! Console built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::JsValue;

pub fn console_log(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let output: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
    println!("{}", output.join(" "));
    Ok(JsValue::Undefined)
}

pub fn console_error(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let output: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
    eprintln!("{}", output.join(" "));
    Ok(JsValue::Undefined)
}

pub fn console_warn(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let output: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
    eprintln!("{}", output.join(" "));
    Ok(JsValue::Undefined)
}

pub fn console_info(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let output: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
    println!("{}", output.join(" "));
    Ok(JsValue::Undefined)
}

pub fn console_debug(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let output: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
    println!("{}", output.join(" "));
    Ok(JsValue::Undefined)
}
