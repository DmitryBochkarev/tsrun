//! CLI tool for running TypeScript files using typescript-eval
//!
//! Usage: typescript-eval-runner <entry-point.ts>

use std::env;
use std::fs;
use std::path::PathBuf;
use typescript_eval::{JsValue, Runtime};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let entry_arg = match args.get(1) {
        Some(arg) => arg,
        None => {
            eprintln!(
                "Usage: {} <entry-point.ts>",
                args.first()
                    .map_or("typescript-eval-runner", |s| s.as_str())
            );
            std::process::exit(1);
        }
    };

    let entry_path = PathBuf::from(entry_arg);
    let source = fs::read_to_string(&entry_path)?;

    let mut runtime = Runtime::new();
    runtime.set_gc_threshold(100);
    runtime.set_timeout_ms(300 * 1000);
    let result = runtime.eval_simple(&source)?;

    print_value(&result);
    Ok(())
}

fn print_value(value: &JsValue) {
    match value {
        JsValue::Undefined => println!("undefined"),
        JsValue::Null => println!("null"),
        JsValue::Boolean(b) => println!("{}", b),
        JsValue::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 1e15 {
                println!("{}", *n as i64);
            } else {
                println!("{}", n);
            }
        }
        JsValue::String(s) => println!("{}", s),
        JsValue::Object(_) => {
            // Try to convert to JSON for pretty printing
            if let Ok(json) = value_to_json(value) {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json).unwrap_or_else(|_| format!("{:?}", value))
                );
            } else {
                println!("{:?}", value);
            }
        }
        JsValue::Symbol(_) => println!("[Symbol]"),
    }
}

fn value_to_json(value: &JsValue) -> Result<serde_json::Value, &'static str> {
    match value {
        JsValue::Undefined => Ok(serde_json::Value::Null),
        JsValue::Null => Ok(serde_json::Value::Null),
        JsValue::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
        JsValue::Number(n) => {
            if n.is_nan() || n.is_infinite() {
                Ok(serde_json::Value::Null)
            } else {
                Ok(serde_json::json!(*n))
            }
        }
        JsValue::String(s) => Ok(serde_json::Value::String(s.to_string())),
        JsValue::Object(obj) => {
            let borrowed = obj.borrow();

            // Check if it's an array
            if let typescript_eval::value::ExoticObject::Array { length } = &borrowed.exotic {
                let mut arr = Vec::with_capacity(*length as usize);
                for i in 0..*length {
                    let elem = borrowed
                        .get_property(&typescript_eval::value::PropertyKey::Index(i))
                        .unwrap_or(JsValue::Undefined);
                    arr.push(value_to_json(&elem)?);
                }
                return Ok(serde_json::Value::Array(arr));
            }

            // Regular object
            let mut map = serde_json::Map::new();
            for (key, prop) in borrowed.properties.iter() {
                if let typescript_eval::value::PropertyKey::String(s) = key {
                    map.insert(s.to_string(), value_to_json(&prop.value)?);
                }
            }
            Ok(serde_json::Value::Object(map))
        }
        JsValue::Symbol(_) => Err("Cannot convert symbol to JSON"),
    }
}
