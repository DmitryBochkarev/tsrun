//! JSON built-in methods

use crate::error::JsError;
use crate::gc::{Gc, Guard};
use crate::interpreter::Interpreter;
use crate::value::{ExoticObject, Guarded, JsObject, JsString, JsValue};

/// Initialize JSON object and add it to globals
pub fn init_json(interp: &mut Interpreter) {
    let (json, _json_guard) = interp.create_object_with_guard();
    interp.root_guard.guard(json.clone());

    interp.register_method(&json, "stringify", json_stringify, 1);
    interp.register_method(&json, "parse", json_parse, 1);

    let json_key = interp.key("JSON");
    interp
        .global
        .borrow_mut()
        .set_property(json_key, JsValue::Object(json));
}

/// Create JSON object with stringify and parse methods (for compatibility)
pub fn create_json_object(interp: &mut Interpreter) -> Gc<JsObject> {
    let (json, _json_guard) = interp.create_object_with_guard();
    interp.root_guard.guard(json.clone());

    interp.register_method(&json, "stringify", json_stringify, 1);
    interp.register_method(&json, "parse", json_parse, 1);

    json
}

pub fn json_stringify(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    // Second argument is replacer (not implemented, ignored)
    // Third argument is space/indent
    let indent = args.get(2).cloned().unwrap_or(JsValue::Undefined);

    let json = js_value_to_json(&value)?;

    let output = match indent {
        JsValue::Number(n) if n > 0.0 => {
            // Use pretty printing with indentation
            let indent_size = n.min(10.0) as usize;
            serde_json::to_string_pretty(&json)
                .map(|s| {
                    // serde_json uses 2 spaces by default, adjust if needed
                    if indent_size == 2 {
                        s
                    } else {
                        // Re-indent with the requested size
                        let indent_str = " ".repeat(indent_size);
                        s.lines()
                            .map(|line| {
                                let stripped = line.trim_start();
                                let leading_spaces = line.len() - stripped.len();
                                let indent_level = leading_spaces / 2;
                                format!("{}{}", indent_str.repeat(indent_level), stripped)
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                })
                .unwrap_or_else(|_| json.to_string())
        }
        JsValue::String(s) if !s.is_empty() => {
            // Use string as indent
            let indent_str = s.as_str();
            serde_json::to_string_pretty(&json)
                .map(|s| {
                    s.lines()
                        .map(|line| {
                            let stripped = line.trim_start();
                            let leading_spaces = line.len() - stripped.len();
                            let indent_level = leading_spaces / 2;
                            format!(
                                "{}{}",
                                indent_str
                                    .chars()
                                    .take(10)
                                    .collect::<String>()
                                    .repeat(indent_level),
                                stripped
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_else(|_| json.to_string())
        }
        _ => json.to_string(),
    };

    Ok(Guarded::unguarded(JsValue::String(JsString::from(output))))
}

pub fn json_parse(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let text = args.first().cloned().unwrap_or(JsValue::Undefined);
    let text_str = text.to_js_string();

    let json: serde_json::Value = serde_json::from_str(text_str.as_str())
        .map_err(|e| JsError::syntax_error(format!("JSON parse error: {}", e), 0, 0))?;

    // Collect all guards to keep objects alive during construction
    let mut guards: Vec<Guard<JsObject>> = Vec::new();
    let value = json_to_js_value_guarded(interp, &json, &mut guards)?;

    // Return the result with a guard if it's an object
    if let JsValue::Object(ref obj) = value {
        // Move the first guard (the top-level object's guard) into Guarded
        // The rest of the guards will be dropped, but that's fine because
        // nested objects are now owned by their parents
        if let Some(guard) = guards.into_iter().next() {
            return Ok(Guarded::with_guard(value, guard));
        }
        // If no guard, use the root guard to protect it
        interp.root_guard.guard(obj.clone());
    }
    Ok(Guarded::unguarded(value))
}

pub fn js_value_to_json(value: &JsValue) -> Result<serde_json::Value, JsError> {
    Ok(match value {
        JsValue::Undefined => serde_json::Value::Null,
        JsValue::Null => serde_json::Value::Null,
        JsValue::Boolean(b) => serde_json::Value::Bool(*b),
        JsValue::Number(n) => {
            if n.is_finite() {
                // Check if the number is a whole integer that fits in i64
                if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 {
                    serde_json::Value::Number(serde_json::Number::from(*n as i64))
                } else {
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(*n).unwrap_or(serde_json::Number::from(0)),
                    )
                }
            } else {
                serde_json::Value::Null
            }
        }
        JsValue::String(s) => serde_json::Value::String(s.to_string()),
        JsValue::Symbol(_) => serde_json::Value::Null, // Symbols are ignored in JSON
        JsValue::Object(obj) => {
            let obj_ref = obj.borrow();
            if let Some(elements) = obj_ref.array_elements() {
                let mut arr = Vec::with_capacity(elements.len());
                for val in elements {
                    arr.push(js_value_to_json(val)?);
                }
                serde_json::Value::Array(arr)
            } else {
                match &obj_ref.exotic {
                    // Array is handled above by array_elements() check
                    ExoticObject::Array { .. } | ExoticObject::Function(_) => {
                        serde_json::Value::Null
                    }
                    ExoticObject::Map { .. } => serde_json::Value::Null,
                    ExoticObject::Set { .. } => serde_json::Value::Null,
                    ExoticObject::Date { timestamp } => {
                        // Dates serialize as their ISO string
                        let datetime = chrono::DateTime::from_timestamp_millis(*timestamp as i64)
                            .unwrap_or(chrono::DateTime::UNIX_EPOCH);
                        serde_json::Value::String(
                            datetime.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
                        )
                    }
                    ExoticObject::RegExp { .. } => {
                        serde_json::Value::Object(serde_json::Map::new())
                    }
                    ExoticObject::Generator(_) => serde_json::Value::Null,
                    ExoticObject::Promise(_) => serde_json::Value::Null,
                    ExoticObject::Environment(_) => serde_json::Value::Null, // Internal type
                    ExoticObject::Ordinary => {
                        let mut map = serde_json::Map::new();
                        for (key, prop) in obj_ref.properties.iter() {
                            if prop.enumerable() {
                                let json_val = js_value_to_json(&prop.value)?;
                                // Skip undefined values in objects
                                if json_val != serde_json::Value::Null
                                    || !matches!(prop.value, JsValue::Undefined)
                                {
                                    map.insert(key.to_string(), json_val);
                                }
                            }
                        }
                        serde_json::Value::Object(map)
                    }
                }
            }
        }
    })
}

/// Convert a serde_json value to a JsValue, collecting guards to keep objects alive
fn json_to_js_value_guarded(
    interp: &mut Interpreter,
    json: &serde_json::Value,
    guards: &mut Vec<Guard<JsObject>>,
) -> Result<JsValue, JsError> {
    Ok(match json {
        serde_json::Value::Null => JsValue::Null,
        serde_json::Value::Bool(b) => JsValue::Boolean(*b),
        serde_json::Value::Number(n) => JsValue::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => JsValue::String(JsString::from(s.clone())),
        serde_json::Value::Array(arr) => {
            // First build all elements with guards collected
            let mut elements = Vec::with_capacity(arr.len());
            for item in arr {
                let val = json_to_js_value_guarded(interp, item, guards)?;
                elements.push(val);
            }
            let (result, guard) = interp.create_array(elements);
            guards.push(guard);
            JsValue::Object(result)
        }
        serde_json::Value::Object(map) => {
            let (obj, guard) = interp.create_object_with_guard();
            for (key, value) in map {
                let js_value = json_to_js_value_guarded(interp, value, guards)?;
                let interned_key = interp.key(key);
                obj.borrow_mut().set_property(interned_key, js_value);
            }
            guards.push(guard);
            JsValue::Object(obj)
        }
    })
}

/// Convert a serde_json value to a JsValue using the interpreter's GC space
pub fn json_to_js_value_with_interp(
    interp: &mut Interpreter,
    json: &serde_json::Value,
) -> Result<JsValue, JsError> {
    let mut guards = Vec::new();
    let value = json_to_js_value_guarded(interp, json, &mut guards)?;
    // Note: guards will be dropped after this, but objects should be owned by their parents
    Ok(value)
}
