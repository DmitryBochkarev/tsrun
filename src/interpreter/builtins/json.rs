//! JSON built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{
    create_array, create_object, register_method, ExoticObject, JsObjectRef, JsString, JsValue,
    PropertyKey,
};

/// Create JSON object with stringify and parse methods
pub fn create_json_object() -> JsObjectRef {
    let json = create_object();
    {
        let mut j = json.borrow_mut();

        register_method(&mut j, "stringify", json_stringify, 1);
        register_method(&mut j, "parse", json_parse, 1);
    }
    json
}

pub fn json_stringify(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    Ok(JsValue::String(JsString::from(output)))
}

pub fn json_parse(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let text = args.first().cloned().unwrap_or(JsValue::Undefined);
    let text_str = text.to_js_string();

    let json: serde_json::Value = serde_json::from_str(text_str.as_str())
        .map_err(|e| JsError::syntax_error(format!("JSON parse error: {}", e), 0, 0))?;

    json_to_js_value(&json)
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
            match &obj_ref.exotic {
                ExoticObject::Array { length } => {
                    let mut arr = Vec::with_capacity(*length as usize);
                    for i in 0..*length {
                        let val = obj_ref
                            .get_property(&PropertyKey::Index(i))
                            .unwrap_or(JsValue::Undefined);
                        arr.push(js_value_to_json(&val)?);
                    }
                    serde_json::Value::Array(arr)
                }
                ExoticObject::Function(_) => serde_json::Value::Null,
                ExoticObject::Map { .. } => serde_json::Value::Null,
                ExoticObject::Set { .. } => serde_json::Value::Null,
                ExoticObject::Date { timestamp } => {
                    // Dates serialize as their ISO string
                    let datetime = chrono::DateTime::from_timestamp_millis(*timestamp as i64)
                        .unwrap_or(chrono::DateTime::UNIX_EPOCH);
                    serde_json::Value::String(datetime.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
                }
                ExoticObject::RegExp { .. } => serde_json::Value::Object(serde_json::Map::new()),
                ExoticObject::Generator(_) => serde_json::Value::Null,
                ExoticObject::Promise(_) => serde_json::Value::Null,
                ExoticObject::Ordinary => {
                    let mut map = serde_json::Map::new();
                    for (key, prop) in obj_ref.properties.iter() {
                        if prop.enumerable {
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
    })
}

pub fn json_to_js_value(json: &serde_json::Value) -> Result<JsValue, JsError> {
    Ok(match json {
        serde_json::Value::Null => JsValue::Null,
        serde_json::Value::Bool(b) => JsValue::Boolean(*b),
        serde_json::Value::Number(n) => JsValue::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => JsValue::String(JsString::from(s.clone())),
        serde_json::Value::Array(arr) => {
            let elements: Result<Vec<_>, _> = arr.iter().map(json_to_js_value).collect();
            JsValue::Object(create_array(elements?))
        }
        serde_json::Value::Object(map) => {
            let obj = create_object();
            for (key, value) in map {
                obj.borrow_mut()
                    .set_property(PropertyKey::from(key.as_str()), json_to_js_value(value)?);
            }
            JsValue::Object(obj)
        }
    })
}
