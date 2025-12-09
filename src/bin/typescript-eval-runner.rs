//! CLI tool for running TypeScript files using typescript-eval
//!
//! Usage: typescript-eval-runner <entry-point.ts>

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use typescript_eval::{JsValue, Runtime, RuntimeResult};

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
    let entry_dir = entry_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let source = fs::read_to_string(&entry_path)?;

    let mut runtime = Runtime::new();
    let mut module_cache: HashMap<PathBuf, JsValue> = HashMap::new();
    let mut result = runtime.eval(&source)?;

    loop {
        match result {
            RuntimeResult::Complete(value) => {
                print_value(&value);
                return Ok(());
            }
            RuntimeResult::ImportAwaited { slot, specifier } => {
                let module_path = resolve_module(&entry_dir, &specifier)?;

                // Check cache first
                if let Some(cached) = module_cache.get(&module_path) {
                    slot.set_success(cached.clone());
                } else {
                    // Load and evaluate module
                    let module = load_module(&mut runtime, &module_path, &mut module_cache)?;
                    slot.set_success(module);
                }
            }
            RuntimeResult::AsyncAwaited { slot, .. } => {
                // For demo purposes: resolve immediately with undefined
                slot.set_success(JsValue::Undefined);
            }
        }
        result = runtime.continue_eval()?;
    }
}

fn resolve_module(base_dir: &Path, specifier: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Handle relative imports
    if specifier.starts_with("./") || specifier.starts_with("../") {
        let mut path = base_dir.join(specifier);

        // Try adding .ts extension if needed
        if !path.exists() {
            let with_ts = path.with_extension("ts");
            if with_ts.exists() {
                path = with_ts;
            } else {
                // Try .ts.ts in case the specifier already had .ts
                let specifier_ts = format!("{}.ts", specifier);
                let alt_path = base_dir.join(&specifier_ts);
                if alt_path.exists() {
                    path = alt_path;
                }
            }
        }

        Ok(path.canonicalize()?)
    } else {
        Err(format!(
            "Unsupported module specifier: {} (only relative imports supported)",
            specifier
        )
        .into())
    }
}

fn load_module(
    main_runtime: &mut Runtime,
    path: &Path,
    cache: &mut HashMap<PathBuf, JsValue>,
) -> Result<JsValue, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let module_dir = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // Evaluate the module with a SEPARATE runtime to avoid corrupting main runtime state
    let mut module_runtime = Runtime::new();
    let mut result = module_runtime.eval(&source)?;

    loop {
        match result {
            RuntimeResult::Complete(_) => {
                // Get module exports from the module runtime
                let exports: Vec<(String, JsValue)> = module_runtime
                    .get_exports()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect();

                // Create module object on the MAIN runtime so it's usable there
                let module = main_runtime.create_module_object(exports);
                cache.insert(path.to_path_buf(), module.clone());
                return Ok(module);
            }
            RuntimeResult::ImportAwaited { slot, specifier } => {
                let nested_path = resolve_module(&module_dir, &specifier)?;

                if let Some(cached) = cache.get(&nested_path) {
                    slot.set_success(cached.clone());
                } else {
                    // For nested imports, use the module_runtime as parent
                    let nested_module =
                        load_module_nested(&mut module_runtime, &nested_path, cache)?;
                    slot.set_success(nested_module);
                }
            }
            RuntimeResult::AsyncAwaited { slot, .. } => {
                slot.set_success(JsValue::Undefined);
            }
        }
        result = module_runtime.continue_eval()?;
    }
}

fn load_module_nested(
    parent_runtime: &mut Runtime,
    path: &Path,
    cache: &mut HashMap<PathBuf, JsValue>,
) -> Result<JsValue, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let module_dir = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // Each nested module also gets its own runtime
    let mut module_runtime = Runtime::new();
    let mut result = module_runtime.eval(&source)?;

    loop {
        match result {
            RuntimeResult::Complete(_) => {
                let exports: Vec<(String, JsValue)> = module_runtime
                    .get_exports()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect();

                let module = parent_runtime.create_module_object(exports);
                cache.insert(path.to_path_buf(), module.clone());
                return Ok(module);
            }
            RuntimeResult::ImportAwaited { slot, specifier } => {
                let nested_path = resolve_module(&module_dir, &specifier)?;

                if let Some(cached) = cache.get(&nested_path) {
                    slot.set_success(cached.clone());
                } else {
                    let nested_module =
                        load_module_nested(&mut module_runtime, &nested_path, cache)?;
                    slot.set_success(nested_module);
                }
            }
            RuntimeResult::AsyncAwaited { slot, .. } => {
                slot.set_success(JsValue::Undefined);
            }
        }
        result = module_runtime.continue_eval()?;
    }
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
            if n.is_nan() {
                Ok(serde_json::Value::Null)
            } else if n.is_infinite() {
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
