//! Console built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::platform::ConsoleLevel;
use crate::prelude::*;
use crate::value::{Guarded, JsValue, PropertyKey};

/// Format a JsValue for console output (strings without quotes)
fn format_for_console(value: &JsValue) -> String {
    match value {
        JsValue::String(s) => s.to_string(),
        other => format!("{:?}", other),
    }
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
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let output: Vec<String> = args.iter().map(format_for_console).collect();
    interp.console_write(ConsoleLevel::Log, &output.join(" "));
    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn console_error(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let output: Vec<String> = args.iter().map(format_for_console).collect();
    interp.console_write(ConsoleLevel::Error, &output.join(" "));
    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn console_warn(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let output: Vec<String> = args.iter().map(format_for_console).collect();
    interp.console_write(ConsoleLevel::Warn, &output.join(" "));
    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn console_info(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let output: Vec<String> = args.iter().map(format_for_console).collect();
    interp.console_write(ConsoleLevel::Info, &output.join(" "));
    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn console_debug(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let output: Vec<String> = args.iter().map(format_for_console).collect();
    interp.console_write(ConsoleLevel::Debug, &output.join(" "));
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.table(data, columns?)
/// Displays tabular data as a table
pub fn console_table(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let data = args.first().cloned().unwrap_or(JsValue::Undefined);

    match &data {
        JsValue::Object(obj) => {
            let obj_ref = obj.borrow();
            if let Some(length) = obj_ref.array_length() {
                interp.console_write(ConsoleLevel::Log, "┌───────┬───────────┐");
                interp.console_write(ConsoleLevel::Log, "│ index │   value   │");
                interp.console_write(ConsoleLevel::Log, "├───────┼───────────┤");
                for i in 0..length {
                    let val = obj_ref
                        .get_property(&PropertyKey::Index(i))
                        .unwrap_or(JsValue::Undefined);
                    interp.console_write(
                        ConsoleLevel::Log,
                        &format!("│ {:5} │ {:9} │", i, format!("{:?}", val)),
                    );
                }
                interp.console_write(ConsoleLevel::Log, "└───────┴───────────┘");
            } else {
                // Regular object - display properties
                interp.console_write(ConsoleLevel::Log, "┌─────────────┬───────────┐");
                interp.console_write(ConsoleLevel::Log, "│     key     │   value   │");
                interp.console_write(ConsoleLevel::Log, "├─────────────┼───────────┤");
                for (key, prop) in obj_ref.properties.iter() {
                    interp.console_write(
                        ConsoleLevel::Log,
                        &format!(
                            "│ {:11} │ {:9} │",
                            key.to_string(),
                            format!("{:?}", prop.value)
                        ),
                    );
                }
                interp.console_write(ConsoleLevel::Log, "└─────────────┴───────────┘");
            }
        }
        _ => interp.console_write(ConsoleLevel::Log, &format!("{:?}", data)),
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.dir(obj, options?)
/// Displays an interactive listing of the properties of a specified JavaScript object
pub fn console_dir(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    match &obj {
        JsValue::Object(o) => {
            let o_ref = o.borrow();
            interp.console_write(ConsoleLevel::Log, "Object {");
            for (key, prop) in o_ref.properties.iter() {
                interp.console_write(ConsoleLevel::Log, &format!("  {}: {:?}", key, prop.value));
            }
            interp.console_write(ConsoleLevel::Log, "}");
        }
        other => interp.console_write(ConsoleLevel::Log, &format!("{:?}", other)),
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.time(label)
/// Starts a timer with a specified label
pub fn console_time(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let label = match args.first() {
        Some(v) => interp.to_js_string(v).to_string(),
        None => "default".to_string(),
    };

    interp.console_timer_start(label);
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.timeEnd(label)
/// Stops a timer and logs the elapsed time
pub fn console_time_end(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let label = match args.first() {
        Some(v) => interp.to_js_string(v).to_string(),
        None => "default".to_string(),
    };

    match interp.console_timer_end(&label) {
        Some(elapsed_ms) => {
            interp.console_write(ConsoleLevel::Log, &format!("{}: {}ms", label, elapsed_ms))
        }
        None => interp.console_write(
            ConsoleLevel::Warn,
            &format!("Timer '{}' does not exist", label),
        ),
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.count(label)
/// Logs the number of times this particular call to count() has been called
pub fn console_count(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let label = match args.first() {
        Some(v) => interp.to_js_string(v).to_string(),
        None => "default".to_string(),
    };

    let count = interp.console_counter_increment(label.clone());
    interp.console_write(ConsoleLevel::Log, &format!("{}: {}", label, count));
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.countReset(label)
/// Resets the counter for the given label
pub fn console_count_reset(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let label = match args.first() {
        Some(v) => interp.to_js_string(v).to_string(),
        None => "default".to_string(),
    };

    interp.console_counter_reset(&label);
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.clear()
/// Clears the console
pub fn console_clear(
    interp: &mut Interpreter,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    interp.console_clear();
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// console.group(label)
/// Creates a new inline group, indenting subsequent console messages
pub fn console_group(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let label = match args.first() {
        Some(v) => interp.to_js_string(v).to_string(),
        None => String::new(),
    };

    if label.is_empty() {
        interp.console_write(ConsoleLevel::Log, "▼");
    } else {
        interp.console_write(ConsoleLevel::Log, &format!("▼ {}", label));
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
