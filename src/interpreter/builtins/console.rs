//! Console built-in methods

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object, ExoticObject, JsFunction, JsObjectRef, JsValue, NativeFunction,
    PropertyKey,
};

// Thread-local storage for console timers and counters
lazy_static::lazy_static! {
    static ref CONSOLE_TIMERS: Mutex<HashMap<String, Instant>> = Mutex::new(HashMap::new());
    static ref CONSOLE_COUNTERS: Mutex<HashMap<String, u64>> = Mutex::new(HashMap::new());
}

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

        let table_fn = create_function(JsFunction::Native(NativeFunction {
            name: "table".to_string(),
            func: console_table,
            arity: 1,
        }));
        con.set_property(PropertyKey::from("table"), JsValue::Object(table_fn));

        let dir_fn = create_function(JsFunction::Native(NativeFunction {
            name: "dir".to_string(),
            func: console_dir,
            arity: 1,
        }));
        con.set_property(PropertyKey::from("dir"), JsValue::Object(dir_fn));

        let time_fn = create_function(JsFunction::Native(NativeFunction {
            name: "time".to_string(),
            func: console_time,
            arity: 1,
        }));
        con.set_property(PropertyKey::from("time"), JsValue::Object(time_fn));

        let time_end_fn = create_function(JsFunction::Native(NativeFunction {
            name: "timeEnd".to_string(),
            func: console_time_end,
            arity: 1,
        }));
        con.set_property(PropertyKey::from("timeEnd"), JsValue::Object(time_end_fn));

        let count_fn = create_function(JsFunction::Native(NativeFunction {
            name: "count".to_string(),
            func: console_count,
            arity: 1,
        }));
        con.set_property(PropertyKey::from("count"), JsValue::Object(count_fn));

        let count_reset_fn = create_function(JsFunction::Native(NativeFunction {
            name: "countReset".to_string(),
            func: console_count_reset,
            arity: 1,
        }));
        con.set_property(
            PropertyKey::from("countReset"),
            JsValue::Object(count_reset_fn),
        );

        let clear_fn = create_function(JsFunction::Native(NativeFunction {
            name: "clear".to_string(),
            func: console_clear,
            arity: 0,
        }));
        con.set_property(PropertyKey::from("clear"), JsValue::Object(clear_fn));

        let group_fn = create_function(JsFunction::Native(NativeFunction {
            name: "group".to_string(),
            func: console_group,
            arity: 0,
        }));
        con.set_property(PropertyKey::from("group"), JsValue::Object(group_fn));

        let group_end_fn = create_function(JsFunction::Native(NativeFunction {
            name: "groupEnd".to_string(),
            func: console_group_end,
            arity: 0,
        }));
        con.set_property(PropertyKey::from("groupEnd"), JsValue::Object(group_end_fn));
    }
    console
}

pub fn console_log(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let output: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
    println!("{}", output.join(" "));
    Ok(JsValue::Undefined)
}

pub fn console_error(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let output: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
    eprintln!("{}", output.join(" "));
    Ok(JsValue::Undefined)
}

pub fn console_warn(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let output: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
    eprintln!("{}", output.join(" "));
    Ok(JsValue::Undefined)
}

pub fn console_info(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let output: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
    println!("{}", output.join(" "));
    Ok(JsValue::Undefined)
}

pub fn console_debug(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let output: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
    println!("{}", output.join(" "));
    Ok(JsValue::Undefined)
}

/// console.table(data, columns?)
/// Displays tabular data as a table
pub fn console_table(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let data = args.first().cloned().unwrap_or(JsValue::Undefined);

    match &data {
        JsValue::Object(obj) => {
            let obj_ref = obj.borrow();
            match &obj_ref.exotic {
                ExoticObject::Array { length } => {
                    println!("┌───────┬───────────┐");
                    println!("│ index │   value   │");
                    println!("├───────┼───────────┤");
                    for i in 0..*length {
                        let val = obj_ref
                            .get_property(&PropertyKey::Index(i))
                            .unwrap_or(JsValue::Undefined);
                        println!("│ {:5} │ {:9} │", i, format!("{:?}", val));
                    }
                    println!("└───────┴───────────┘");
                }
                _ => {
                    // Regular object - display properties
                    println!("┌─────────────┬───────────┐");
                    println!("│     key     │   value   │");
                    println!("├─────────────┼───────────┤");
                    for (key, prop) in obj_ref.properties.iter() {
                        println!(
                            "│ {:11} │ {:9} │",
                            key.to_string(),
                            format!("{:?}", prop.value)
                        );
                    }
                    println!("└─────────────┴───────────┘");
                }
            }
        }
        _ => println!("{:?}", data),
    }
    Ok(JsValue::Undefined)
}

/// console.dir(obj, options?)
/// Displays an interactive listing of the properties of a specified JavaScript object
pub fn console_dir(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    match &obj {
        JsValue::Object(o) => {
            let o_ref = o.borrow();
            println!("Object {{");
            for (key, prop) in o_ref.properties.iter() {
                println!("  {}: {:?}", key, prop.value);
            }
            println!("}}");
        }
        other => println!("{:?}", other),
    }
    Ok(JsValue::Undefined)
}

/// console.time(label)
/// Starts a timer with a specified label
pub fn console_time(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let label = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| "default".to_string());

    if let Ok(mut timers) = CONSOLE_TIMERS.lock() {
        timers.insert(label, Instant::now());
    }
    Ok(JsValue::Undefined)
}

/// console.timeEnd(label)
/// Stops a timer and logs the elapsed time
pub fn console_time_end(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let label = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| "default".to_string());

    if let Ok(mut timers) = CONSOLE_TIMERS.lock() {
        if let Some(start) = timers.remove(&label) {
            let elapsed = start.elapsed();
            println!("{}: {}ms", label, elapsed.as_millis());
        } else {
            println!("Timer '{}' does not exist", label);
        }
    }
    Ok(JsValue::Undefined)
}

/// console.count(label)
/// Logs the number of times this particular call to count() has been called
pub fn console_count(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let label = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| "default".to_string());

    if let Ok(mut counters) = CONSOLE_COUNTERS.lock() {
        let count = counters.entry(label.clone()).or_insert(0);
        *count += 1;
        println!("{}: {}", label, count);
    }
    Ok(JsValue::Undefined)
}

/// console.countReset(label)
/// Resets the counter for the given label
pub fn console_count_reset(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let label = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| "default".to_string());

    if let Ok(mut counters) = CONSOLE_COUNTERS.lock() {
        counters.remove(&label);
    }
    Ok(JsValue::Undefined)
}

/// console.clear()
/// Clears the console
pub fn console_clear(
    _interp: &mut Interpreter,
    _this: JsValue,
    _args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    // In a real terminal, we'd clear the screen
    // For now, just print some newlines
    println!("\n\n--- Console cleared ---\n\n");
    Ok(JsValue::Undefined)
}

/// console.group(label)
/// Creates a new inline group, indenting subsequent console messages
pub fn console_group(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let label = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();

    if label.is_empty() {
        println!("▼");
    } else {
        println!("▼ {}", label);
    }
    Ok(JsValue::Undefined)
}

/// console.groupEnd()
/// Exits the current inline group
pub fn console_group_end(
    _interp: &mut Interpreter,
    _this: JsValue,
    _args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    // In a real implementation, this would decrease indentation
    Ok(JsValue::Undefined)
}
