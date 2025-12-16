//! CLI tool for running TypeScript files using typescript-eval
//!
//! Usage: typescript-eval-runner <entry-point.ts>
//!
//! Supports static imports - modules are resolved relative to the importing file.
//! Nested imports are supported.

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

    // Rewrite imports in entry file to use canonical paths
    let source = fs::read_to_string(&entry_path)?;
    let rewritten_source = rewrite_imports(&source, &entry_dir)?;

    let mut runtime = Runtime::new();
    runtime.set_gc_threshold(100);
    runtime.set_timeout_ms(300 * 1000);

    // Track provided modules to avoid reloading
    let mut provided: HashMap<String, PathBuf> = HashMap::new();

    // Start evaluation - may return NeedImports
    let mut result = runtime.eval(&rewritten_source)?;

    // Module loading loop
    loop {
        match result {
            RuntimeResult::Complete(value) => {
                print_value(&value);
                return Ok(());
            }
            RuntimeResult::NeedImports(specifiers) => {
                for specifier in specifiers {
                    if provided.contains_key(&specifier) {
                        continue;
                    }

                    // Load module from filesystem
                    let (module_source, module_dir) = load_module(&specifier)?;

                    // Rewrite imports in this module to use canonical paths
                    let rewritten_module = rewrite_imports(&module_source, &module_dir)?;

                    // Provide to runtime (doesn't execute yet, just stores it)
                    runtime.provide_module(&specifier, &rewritten_module)?;
                    provided.insert(specifier, module_dir);
                }
                // continue_eval will check for nested imports and return NeedImports again if needed
                result = runtime.continue_eval()?;
            }
            RuntimeResult::Suspended { pending, .. } => {
                // For now, we don't support async operations in the CLI
                return Err(format!(
                    "Async operations not supported in CLI (pending orders: {})",
                    pending.len()
                )
                .into());
            }
        }
    }
}

/// Rewrite import specifiers in source to use canonical paths
fn rewrite_imports(source: &str, base_dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    use regex::Regex;

    // Match import statements: import ... from "specifier" or import ... from 'specifier'
    // Also match export ... from "specifier"
    // Handle double quotes
    let double_quote_re = Regex::new(r#"((?:import|export)\s+(?:[^;]*?\s+)?from\s+)"([^"]+)""#)?;
    // Handle single quotes
    let single_quote_re = Regex::new(r#"((?:import|export)\s+(?:[^;]*?\s+)?from\s+)'([^']+)'"#)?;

    // First pass: double quotes
    let result = double_quote_re.replace_all(source, |caps: &regex::Captures| {
        let prefix = caps.get(1).map_or("", |m| m.as_str());
        let specifier = caps.get(2).map_or("", |m| m.as_str());

        if specifier.starts_with("./") || specifier.starts_with("../") {
            let canonical = resolve_to_canonical(base_dir, specifier);
            format!("{prefix}\"{canonical}\"")
        } else {
            caps.get(0)
                .map_or(String::new(), |m| m.as_str().to_string())
        }
    });

    // Second pass: single quotes
    let result = single_quote_re.replace_all(&result, |caps: &regex::Captures| {
        let prefix = caps.get(1).map_or("", |m| m.as_str());
        let specifier = caps.get(2).map_or("", |m| m.as_str());

        if specifier.starts_with("./") || specifier.starts_with("../") {
            let canonical = resolve_to_canonical(base_dir, specifier);
            format!("{prefix}'{canonical}'")
        } else {
            caps.get(0)
                .map_or(String::new(), |m| m.as_str().to_string())
        }
    });

    Ok(result.into_owned())
}

/// Resolve a specifier to a canonical path string
fn resolve_to_canonical(base_dir: &Path, specifier: &str) -> String {
    let module_path = base_dir.join(specifier);

    // Resolve the path (handles ./ and ../)
    let resolved = resolve_path(&module_path);

    // Add extension if needed
    let with_ext = if resolved.extension().is_some() {
        resolved
    } else {
        let ts_path = resolved.with_extension("ts");
        if ts_path.exists() {
            ts_path
        } else {
            let js_path = resolved.with_extension("js");
            if js_path.exists() {
                js_path
            } else {
                let index_path = resolved.join("index.ts");
                if index_path.exists() {
                    index_path
                } else {
                    ts_path // Default to .ts for error handling
                }
            }
        }
    };

    with_ext.to_string_lossy().into_owned()
}

/// Resolve a path, handling . and .. components without requiring the path to exist
fn resolve_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {
                // Skip
            }
            c => {
                result.push(c);
            }
        }
    }

    result
}

/// Load a module from a canonical path
fn load_module(canonical_path: &str) -> Result<(String, PathBuf), Box<dyn std::error::Error>> {
    let path = PathBuf::from(canonical_path);

    let source = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to load module '{}': {}", canonical_path, e))?;

    let module_dir = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    Ok((source, module_dir))
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
