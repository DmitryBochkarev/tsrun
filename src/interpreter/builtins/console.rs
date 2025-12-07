//! Console built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{create_function, create_object, JsFunction, JsObjectRef, JsValue, NativeFunction, PropertyKey};

/// Create console object with log, error, warn, info, debug methods
pub fn create_console_object() -> JsObjectRef {
    let console = create_object();
    {
        let mut con = console.borrow_mut();

        let log_fn = create_function(JsFunction::Native(NativeFunction {
            name: "log".to_string(),
            func: console_log,
            arity: 0,
        }));
        con.set_property(PropertyKey::from("log"), JsValue::Object(log_fn));

        let error_fn = create_function(JsFunction::Native(NativeFunction {
            name: "error".to_string(),
            func: console_error,
            arity: 0,
        }));
        con.set_property(PropertyKey::from("error"), JsValue::Object(error_fn));

        let warn_fn = create_function(JsFunction::Native(NativeFunction {
            name: "warn".to_string(),
            func: console_warn,
            arity: 0,
        }));
        con.set_property(PropertyKey::from("warn"), JsValue::Object(warn_fn));

        let info_fn = create_function(JsFunction::Native(NativeFunction {
            name: "info".to_string(),
            func: console_info,
            arity: 0,
        }));
        con.set_property(PropertyKey::from("info"), JsValue::Object(info_fn));

        let debug_fn = create_function(JsFunction::Native(NativeFunction {
            name: "debug".to_string(),
            func: console_debug,
            arity: 0,
        }));
        con.set_property(PropertyKey::from("debug"), JsValue::Object(debug_fn));
    }
    console
}

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
