//! Console built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::platform::ConsoleLevel;
use crate::prelude::*;
use crate::value::{ExoticObject, Guarded, JsValue, PropertyKey};

/// Format a JsValue for console output (strings without quotes)
fn format_for_console(value: &JsValue) -> String {
    format_value_with_depth(value, 0, &mut Vec::new())
}

/// Format a JsValue with depth tracking to handle nested structures.
fn format_value_with_depth(value: &JsValue, depth: usize, seen: &mut Vec<usize>) -> String {
    const MAX_DEPTH: usize = 10;
    const MAX_ITEMS: usize = 100;

    match value {
        JsValue::Undefined => String::from("undefined"),
        JsValue::Null => String::from("null"),
        JsValue::Boolean(b) => {
            if *b {
                String::from("true")
            } else {
                String::from("false")
            }
        }
        JsValue::Number(n) => {
            if n.is_nan() {
                String::from("NaN")
            } else if n.is_infinite() {
                if *n > 0.0 {
                    String::from("Infinity")
                } else {
                    String::from("-Infinity")
                }
            } else if crate::prelude::math::fract(*n) == 0.0 && n.abs() < 1e15 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        JsValue::String(s) => s.to_string(), // No quotes for console output
        JsValue::Symbol(sym) => match &sym.description {
            Some(desc) => format!("Symbol({})", desc),
            None => String::from("Symbol()"),
        },
        JsValue::Object(obj) => {
            // Check for circular reference using object id
            let obj_id = obj.id();
            if seen.contains(&obj_id) {
                return String::from("[Circular]");
            }

            // Check depth limit
            if depth >= MAX_DEPTH {
                let obj_ref = obj.borrow();
                return match &obj_ref.exotic {
                    ExoticObject::Array { elements } => format!("[Array({})]", elements.len()),
                    ExoticObject::Function(_) => String::from("[Function]"),
                    _ => String::from("{...}"),
                };
            }

            seen.push(obj_id);
            let result = format_object_for_console(obj, depth, seen, MAX_ITEMS);
            seen.pop();
            result
        }
    }
}

/// Format the contents of an object or array for console output.
fn format_object_for_console(
    obj: &crate::gc::Gc<crate::value::JsObject>,
    depth: usize,
    seen: &mut Vec<usize>,
    max_items: usize,
) -> String {
    let obj_ref = obj.borrow();

    match &obj_ref.exotic {
        ExoticObject::Array { elements } => {
            let mut items = Vec::new();
            let length = elements.len();
            let display_len = length.min(max_items);

            for elem in elements.iter().take(display_len) {
                items.push(format_value_with_depth(elem, depth + 1, seen));
            }

            if length > max_items {
                items.push(format!("... {} more items", length - max_items));
            }

            format!("[{}]", items.join(", "))
        }
        ExoticObject::Function(func_info) => {
            let name = func_info.name().unwrap_or("anonymous");
            format!("[Function: {}]", name)
        }
        ExoticObject::Date { timestamp } => {
            format!("Date({})", timestamp)
        }
        ExoticObject::RegExp { pattern, flags, .. } => {
            format!("/{}/{}", pattern, flags)
        }
        ExoticObject::Map { entries } => {
            let mut items = Vec::new();
            let display_len = entries.len().min(max_items);

            for (i, (k, v)) in entries.iter().enumerate() {
                if i >= display_len {
                    break;
                }
                let key_str = format_value_with_depth(&k.0, depth + 1, seen);
                let val_str = format_value_with_depth(v, depth + 1, seen);
                items.push(format!("{} => {}", key_str, val_str));
            }

            if entries.len() > max_items {
                items.push(format!("... {} more entries", entries.len() - max_items));
            }

            format!(
                "Map({}){{{}}}",
                entries.len(),
                if items.is_empty() {
                    String::new()
                } else {
                    format!(" {} ", items.join(", "))
                }
            )
        }
        ExoticObject::Set { entries } => {
            let mut items = Vec::new();
            let display_len = entries.len().min(max_items);

            for (i, v) in entries.iter().enumerate() {
                if i >= display_len {
                    break;
                }
                items.push(format_value_with_depth(&v.0, depth + 1, seen));
            }

            if entries.len() > max_items {
                items.push(format!("... {} more items", entries.len() - max_items));
            }

            format!(
                "Set({}){{{}}}",
                entries.len(),
                if items.is_empty() {
                    String::new()
                } else {
                    format!(" {} ", items.join(", "))
                }
            )
        }
        ExoticObject::Promise(_) => String::from("Promise { <pending> }"),
        ExoticObject::Generator(_) | ExoticObject::BytecodeGenerator(_) => {
            String::from("Generator { <suspended> }")
        }
        ExoticObject::Proxy(_) => String::from("Proxy {}"),
        ExoticObject::Boolean(b) => format!("[Boolean: {}]", b),
        ExoticObject::Number(n) => format!("[Number: {}]", n),
        ExoticObject::StringObj(s) => format!("[String: \"{}\"]", s),
        ExoticObject::Symbol(sym) => match &sym.description {
            Some(desc) => format!("[Symbol: Symbol({})]", desc),
            None => String::from("[Symbol: Symbol()]"),
        },
        ExoticObject::Environment(_) => String::from("[Environment]"),
        ExoticObject::Enum(_) => String::from("[Enum]"),
        ExoticObject::RawJSON(s) => s.to_string(),
        ExoticObject::PendingOrder { id } => format!("[PendingOrder: {}]", id),
        ExoticObject::Ordinary => {
            // Regular object - format as { key: value, ... }
            let mut items = Vec::new();
            let mut count = 0;

            for (key, prop) in obj_ref.properties.iter() {
                if count >= max_items {
                    break;
                }
                // Skip internal properties (symbols)
                let key_str = match key {
                    PropertyKey::String(s) => s.to_string(),
                    PropertyKey::Symbol(_) => continue, // Skip symbols in output
                    PropertyKey::Index(i) => i.to_string(),
                };
                let val_str = format_value_with_depth(&prop.value, depth + 1, seen);
                items.push(format!("{}: {}", key_str, val_str));
                count += 1;
            }

            let total = obj_ref.properties.len();
            if total > max_items {
                items.push(format!("... {} more properties", total - max_items));
            }

            if items.is_empty() {
                String::from("{}")
            } else {
                format!("{{ {} }}", items.join(", "))
            }
        }
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
