//! Console built-in methods

use std::sync::Mutex;
use std::time::Instant;

use rustc_hash::FxHashMap;

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{Guarded, JsValue, PropertyKey};

/// Format a JsValue for console output (strings without quotes)
fn format_for_console(value: &JsValue) -> String {
    match value {
        JsValue::String(s) => s.to_string(),
        other => format!("{:?}", other),
    }
}

// Thread-local storage for console timers and counters
// FIXME: store this in interpreter
lazy_static::lazy_static! {
    static ref CONSOLE_TIMERS: Mutex<FxHashMap<String, Instant>> = Mutex::new(FxHashMap::default());
    static ref CONSOLE_COUNTERS: Mutex<FxHashMap<String, u64>> = Mutex::new(FxHashMap::default());
}

/// Initialize console global object
pub fn init_console(interp: &mut Interpreter) {
    // Use root_guard for permanent global objects
    let console = interp.root_guard.alloc();
    console.borrow_mut().prototype = Some(interp.object_prototype.clone());

    // Logging methods
    interp.register_method(&console, "log", console_log, 0);
    interp.register_method(&console, "error", console_error, 0);
    interp.register_method(&console, "warn", console_warn, 0);
    interp.register_method(&console, "info", console_info, 0);
    interp.register_method(&console, "debug", console_debug, 0);

    // Display methods
    interp.register_method(&console, "table", console_table, 1);
    interp.register_method(&console, "dir", console_dir, 1);

    // Timing methods
    interp.register_method(&console, "time", console_time, 1);
    interp.register_method(&console, "timeEnd", console_time_end, 1);

    // Counting methods
    interp.register_method(&console, "count", console_count, 1);
    interp.register_method(&console, "countReset", console_count_reset, 1);

    // Other methods
    interp.register_method(&console, "clear", console_clear, 0);
    interp.register_method(&console, "group", console_group, 0);
    interp.register_method(&console, "groupEnd", console_group_end, 0);

    let console_key = PropertyKey::String(interp.intern("console"));
    interp
        .global
        .borrow_mut()
        .set_property(console_key, JsValue::Object(console));
}

pub fn console_log(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let output: Vec<String> = args.iter().map(format_for_console).collect();
    println!("{}", output.join(" "));
    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn console_error(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let output: Vec<String> = args.iter().map(format_for_console).collect();
    eprintln!("{}", output.join(" "));
    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn console_warn(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let output: Vec<String> = args.iter().map(format_for_console).collect();
    eprintln!("{}", output.join(" "));
    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn console_info(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let output: Vec<String> = args.iter().map(format_for_console).collect();
    println!("{}", output.join(" "));
    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn console_debug(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let output: Vec<String> = args.iter().map(format_for_console).collect();
    println!("{}", output.join(" "));
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.table(data, columns?)
/// Displays tabular data as a table
pub fn console_table(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let data = args.first().cloned().unwrap_or(JsValue::Undefined);

    match &data {
        JsValue::Object(obj) => {
            let obj_ref = obj.borrow();
            if let Some(length) = obj_ref.array_length() {
                println!("┌───────┬───────────┐");
                println!("│ index │   value   │");
                println!("├───────┼───────────┤");
                for i in 0..length {
                    let val = obj_ref
                        .get_property(&PropertyKey::Index(i))
                        .unwrap_or(JsValue::Undefined);
                    println!("│ {:5} │ {:9} │", i, format!("{:?}", val));
                }
                println!("└───────┴───────────┘");
            } else {
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
        _ => println!("{:?}", data),
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.dir(obj, options?)
/// Displays an interactive listing of the properties of a specified JavaScript object
pub fn console_dir(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
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
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.time(label)
/// Starts a timer with a specified label
pub fn console_time(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let label = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| "default".to_string());

    if let Ok(mut timers) = CONSOLE_TIMERS.lock() {
        timers.insert(label, Instant::now());
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.timeEnd(label)
/// Stops a timer and logs the elapsed time
pub fn console_time_end(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
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
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.count(label)
/// Logs the number of times this particular call to count() has been called
pub fn console_count(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let label = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| "default".to_string());

    if let Ok(mut counters) = CONSOLE_COUNTERS.lock() {
        let count = counters.entry(label.clone()).or_insert(0);
        *count += 1;
        println!("{}: {}", label, count);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.countReset(label)
/// Resets the counter for the given label
pub fn console_count_reset(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let label = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| "default".to_string());

    if let Ok(mut counters) = CONSOLE_COUNTERS.lock() {
        counters.remove(&label);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.clear()
/// Clears the console
pub fn console_clear(
    _interp: &mut Interpreter,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    // In a real terminal, we'd clear the screen
    // For now, just print some newlines
    println!("\n\n--- Console cleared ---\n\n");
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.group(label)
/// Creates a new inline group, indenting subsequent console messages
pub fn console_group(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let label = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();

    if label.is_empty() {
        println!("▼");
    } else {
        println!("▼ {}", label);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.groupEnd()
/// Exits the current inline group
pub fn console_group_end(
    _interp: &mut Interpreter,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    // In a real implementation, this would decrease indentation
    Ok(Guarded::unguarded(JsValue::Undefined))
}
