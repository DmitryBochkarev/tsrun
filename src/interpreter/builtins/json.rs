//! JSON built-in methods

use crate::error::JsError;
use crate::gc::Guard;
use crate::interpreter::Interpreter;
use crate::value::{ExoticObject, Guarded, JsObject, JsString, JsValue, PropertyKey};
use std::collections::HashSet;

/// Initialize JSON object and add it to globals
pub fn init_json(interp: &mut Interpreter) {
    // Use root_guard for permanent objects
    let json = interp.root_guard.alloc();
    json.borrow_mut().prototype = Some(interp.object_prototype.clone());

    interp.register_method(&json, "stringify", json_stringify, 3);
    interp.register_method(&json, "parse", json_parse, 2);
    interp.register_method(&json, "rawJSON", json_raw_json, 1);
    interp.register_method(&json, "isRawJSON", json_is_raw_json, 1);

    let json_key = PropertyKey::String(interp.intern("JSON"));
    interp
        .global
        .borrow_mut()
        .set_property(json_key, JsValue::Object(json));
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

    // Track visited objects for circular reference detection
    let mut visited = HashSet::new();
    let json = js_value_to_json_with_visited(&value, &mut visited)?;

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
    let text_str = interp.to_js_string(&text);

    let json: serde_json::Value = serde_json::from_str(text_str.as_str())
        .map_err(|e| JsError::syntax_error(format!("JSON parse error: {}", e), 0, 0))?;

    // Use a single guard for all objects created during parsing
    let guard = interp.heap.create_guard();
    let value = json_to_js_value_with_guard(interp, &json, &guard)?;

    // Return the result with the guard if it's an object
    if matches!(value, JsValue::Object(_)) {
        return Ok(Guarded::with_guard(value, guard));
    }
    Ok(Guarded::unguarded(value))
}

/// Convert a JsValue to JSON, with public API for external callers (without circular detection)
pub fn js_value_to_json(value: &JsValue) -> Result<serde_json::Value, JsError> {
    let mut visited = HashSet::new();
    js_value_to_json_with_visited(value, &mut visited)
}

/// Convert a JsValue to JSON, tracking visited objects for circular reference detection
fn js_value_to_json_with_visited(
    value: &JsValue,
    visited: &mut HashSet<usize>,
) -> Result<serde_json::Value, JsError> {
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
            // Check for circular reference using object's unique ID
            let obj_id = obj.id();
            if visited.contains(&obj_id) {
                return Err(JsError::type_error(
                    "Converting circular structure to JSON".to_string(),
                ));
            }
            visited.insert(obj_id);

            let result = {
                let obj_ref = obj.borrow();
                if let Some(elements) = obj_ref.array_elements() {
                    let mut arr = Vec::with_capacity(elements.len());
                    for val in elements {
                        arr.push(js_value_to_json_with_visited(val, visited)?);
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
                            let datetime =
                                chrono::DateTime::from_timestamp_millis(*timestamp as i64)
                                    .unwrap_or(chrono::DateTime::UNIX_EPOCH);
                            serde_json::Value::String(
                                datetime.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
                            )
                        }
                        ExoticObject::RegExp { .. } => {
                            serde_json::Value::Object(serde_json::Map::new())
                        }
                        ExoticObject::Generator(_) | ExoticObject::BytecodeGenerator(_) => {
                            serde_json::Value::Null
                        }
                        ExoticObject::Promise(_) => serde_json::Value::Null,
                        ExoticObject::Environment(_) => serde_json::Value::Null, // Internal type
                        ExoticObject::Enum(data) => {
                            // Enums serialize with forward and reverse mappings
                            let mut map = serde_json::Map::new();
                            // Add forward mappings (name -> value)
                            for member in &data.members {
                                let json_val =
                                    js_value_to_json_with_visited(&member.value, visited)?;
                                map.insert(member.name.to_string(), json_val);
                            }
                            // Add reverse mappings (numeric value -> name)
                            for member in &data.members {
                                if let JsValue::Number(n) = &member.value {
                                    map.insert(
                                        n.to_string(),
                                        serde_json::Value::String(member.name.to_string()),
                                    );
                                }
                            }
                            serde_json::Value::Object(map)
                        }
                        ExoticObject::Ordinary => {
                            // Ordinary objects serialize with their properties
                            let mut map = serde_json::Map::new();
                            // First collect keys to avoid borrowing issues
                            let props: Vec<_> = obj_ref
                                .properties
                                .iter()
                                .filter(|(_, prop)| prop.enumerable())
                                .map(|(k, p)| (k.to_string(), p.value.clone()))
                                .collect();
                            drop(obj_ref); // Release borrow before recursive calls

                            for (key, val) in props {
                                let json_val = js_value_to_json_with_visited(&val, visited)?;
                                // Skip undefined values in objects
                                if json_val != serde_json::Value::Null
                                    || !matches!(val, JsValue::Undefined)
                                {
                                    map.insert(key, json_val);
                                }
                            }
                            serde_json::Value::Object(map)
                        }
                        ExoticObject::Proxy(_) => {
                            // Proxies are serialized as their target (or could trap toJSON)
                            // For now, serialize as null to match JSON.stringify behavior
                            serde_json::Value::Null
                        }
                        ExoticObject::Boolean(b) => {
                            // Boolean wrapper objects serialize as their primitive value
                            serde_json::Value::Bool(*b)
                        }
                        ExoticObject::Number(n) => {
                            // Number wrapper objects serialize as their primitive value
                            if n.is_finite() {
                                serde_json::Value::Number(
                                    serde_json::Number::from_f64(*n)
                                        .unwrap_or(serde_json::Number::from(0)),
                                )
                            } else {
                                serde_json::Value::Null
                            }
                        }
                        ExoticObject::StringObj(s) => {
                            // String wrapper objects serialize as their primitive value
                            serde_json::Value::String(s.to_string())
                        }
                        ExoticObject::RawJSON(raw) => {
                            // RawJSON objects are serialized as their raw JSON value
                            // We already validated the JSON when creating the RawJSON object,
                            // so this parse should never fail
                            serde_json::from_str(raw.as_str()).unwrap_or(serde_json::Value::Null)
                        }
                        ExoticObject::Symbol(_) => {
                            // Symbol wrapper objects serialize to undefined (null in JSON)
                            serde_json::Value::Null
                        }
                        ExoticObject::PendingOrder { .. } => {
                            // PendingOrder markers serialize to null
                            serde_json::Value::Null
                        }
                    }
                }
            };

            // Remove from visited set after processing
            visited.remove(&obj_id);
            result
        }
    })
}

/// Convert a serde_json value to a JsValue using a provided guard.
/// The guard keeps any created objects alive until it is dropped.
/// This is the preferred method when you need to control the lifetime of the result.
pub fn json_to_js_value_with_guard(
    interp: &mut Interpreter,
    json: &serde_json::Value,
    guard: &Guard<JsObject>,
) -> Result<JsValue, JsError> {
    Ok(match json {
        serde_json::Value::Null => JsValue::Null,
        serde_json::Value::Bool(b) => JsValue::Boolean(*b),
        serde_json::Value::Number(n) => JsValue::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => JsValue::String(JsString::from(s.clone())),
        serde_json::Value::Array(arr) => {
            // First build all elements
            let mut elements = Vec::with_capacity(arr.len());
            for item in arr {
                let val = json_to_js_value_with_guard(interp, item, guard)?;
                elements.push(val);
            }
            let result = interp.create_array_from(guard, elements);
            JsValue::Object(result)
        }
        serde_json::Value::Object(map) => {
            let obj = interp.create_object(guard);
            for (key, value) in map {
                let js_value = json_to_js_value_with_guard(interp, value, guard)?;
                let interned_key = PropertyKey::String(interp.intern(key));
                obj.borrow_mut().set_property(interned_key, js_value);
            }
            JsValue::Object(obj)
        }
    })
}

/// Convert a serde_json value to a JsValue using the interpreter's GC space
pub fn json_to_js_value_with_interp(
    interp: &mut Interpreter,
    json: &serde_json::Value,
) -> Result<JsValue, JsError> {
    // Create a temporary guard - caller should guard the result if needed
    let guard = interp.heap.create_guard();
    json_to_js_value_with_guard(interp, json, &guard)
}

/// JSON.rawJSON(string) - Creates a raw JSON object
///
/// The raw JSON object contains a JSON string that will be inserted literally
/// when passed to JSON.stringify, without additional escaping or conversion.
/// This is useful for inserting pre-serialized JSON.
pub fn json_raw_json(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Per spec: must be a string
    let JsValue::String(json_string) = value else {
        return Err(JsError::type_error(
            "JSON.rawJSON argument must be a string".to_string(),
        ));
    };

    // Per spec: must be valid JSON (parse it to verify)
    let json_str = json_string.as_str();
    serde_json::from_str::<serde_json::Value>(json_str)
        .map_err(|e| JsError::syntax_error(format!("JSON.rawJSON: invalid JSON: {}", e), 0, 0))?;

    // Create a RawJSON exotic object
    let guard = interp.heap.create_guard();
    let obj = interp.create_object(&guard);
    {
        let mut obj_ref = obj.borrow_mut();
        obj_ref.exotic = ExoticObject::RawJSON(json_string);
        // RawJSON objects have null prototype per spec
        obj_ref.prototype = None;
        obj_ref.null_prototype = true;
    }

    Ok(Guarded::with_guard(JsValue::Object(obj), guard))
}

/// JSON.isRawJSON(value) - Checks if a value is a raw JSON object
///
/// Returns true if the value was created by JSON.rawJSON.
pub fn json_is_raw_json(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);

    let is_raw_json = match value {
        JsValue::Object(obj) => matches!(obj.borrow().exotic, ExoticObject::RawJSON(_)),
        _ => false,
    };

    Ok(Guarded::unguarded(JsValue::Boolean(is_raw_json)))
}
