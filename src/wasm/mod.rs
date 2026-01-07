//! WebAssembly API for the TypeScript interpreter.
//!
//! This module provides a simple API for running TypeScript/JavaScript code
//! in the browser via WebAssembly.
//!
//! # Example (JavaScript)
//!
//! ```javascript
//! import init, { TsRunner } from './pkg/tsrun.js';
//!
//! async function main() {
//!     await init();
//!     const runner = new TsRunner();
//!     const result = runner.run('console.log("Hello from TypeScript!")');
//!
//!     // Display console output
//!     for (const entry of result.console_output) {
//!         console.log(`[${entry.level}] ${entry.message}`);
//!     }
//!
//!     if (result.error) {
//!         console.error('Error:', result.error);
//!     } else {
//!         console.log('Result:', result.value);
//!     }
//! }
//! ```

use crate::interpreter::Interpreter;
use crate::platform::{ConsoleLevel, ConsoleProvider, WasmRegExpProvider};
use crate::prelude::*;
use crate::value::{ExoticObject, JsValue};
use crate::StepResult;
use alloc::rc::Rc;
use core::cell::RefCell;
use wasm_bindgen::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════════
// Console Output Entry
// ═══════════════════════════════════════════════════════════════════════════════

/// A single console output entry.
#[wasm_bindgen]
#[derive(Clone)]
pub struct ConsoleEntry {
    level: String,
    message: String,
}

#[wasm_bindgen]
impl ConsoleEntry {
    /// Get the log level (log, info, debug, warn, error).
    #[wasm_bindgen(getter)]
    pub fn level(&self) -> String {
        self.level.clone()
    }

    /// Get the message.
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Buffered Console Provider
// ═══════════════════════════════════════════════════════════════════════════════

/// Console provider that buffers all output for later retrieval.
struct BufferedConsoleProvider {
    buffer: Rc<RefCell<Vec<ConsoleEntry>>>,
}

impl BufferedConsoleProvider {
    fn new(buffer: Rc<RefCell<Vec<ConsoleEntry>>>) -> Self {
        Self { buffer }
    }
}

impl ConsoleProvider for BufferedConsoleProvider {
    fn write(&self, level: ConsoleLevel, message: &str) {
        let level_str = match level {
            ConsoleLevel::Log => "log",
            ConsoleLevel::Info => "info",
            ConsoleLevel::Debug => "debug",
            ConsoleLevel::Warn => "warn",
            ConsoleLevel::Error => "error",
        };
        self.buffer.borrow_mut().push(ConsoleEntry {
            level: level_str.into(),
            message: message.into(),
        });
    }

    fn clear(&self) {
        self.buffer.borrow_mut().push(ConsoleEntry {
            level: "clear".into(),
            message: "--- Console cleared ---".into(),
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Run Result
// ═══════════════════════════════════════════════════════════════════════════════

/// Result of running TypeScript/JavaScript code.
#[wasm_bindgen]
pub struct RunResult {
    /// The result value as a string (if successful)
    value: Option<String>,
    /// The error message (if failed)
    error: Option<String>,
    /// Console output captured during execution
    console: Vec<ConsoleEntry>,
}

#[wasm_bindgen]
impl RunResult {
    /// Get the result value as a string, or null if there was an error.
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> Option<String> {
        self.value.clone()
    }

    /// Get the error message, or null if successful.
    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }

    /// Check if the execution was successful.
    #[wasm_bindgen(getter)]
    pub fn success(&self) -> bool {
        self.error.is_none()
    }

    /// Get the console output as an array of ConsoleEntry objects.
    #[wasm_bindgen(getter)]
    pub fn console_output(&self) -> Vec<ConsoleEntry> {
        self.console.clone()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TypeScript Runner
// ═══════════════════════════════════════════════════════════════════════════════

/// TypeScript/JavaScript interpreter for WebAssembly.
///
/// Create an instance with `new TsRunner()` and run code with `runner.run(code)`.
#[wasm_bindgen]
pub struct TsRunner {
    // We don't store the interpreter - we create a fresh one for each run
    // to ensure clean state and proper console buffering
    _private: (),
}

#[wasm_bindgen]
impl TsRunner {
    /// Create a new TypeScript runner.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Run TypeScript/JavaScript code and return the result.
    ///
    /// Console output is captured and returned in the result's `console_output` property.
    /// Each run starts with a fresh interpreter state.
    pub fn run(&mut self, code: &str) -> RunResult {
        // Create a buffer for console output
        let console_buffer: Rc<RefCell<Vec<ConsoleEntry>>> = Rc::new(RefCell::new(Vec::new()));

        // Create a fresh interpreter with buffered console and WASM regex provider
        let console_provider = BufferedConsoleProvider::new(Rc::clone(&console_buffer));
        let regexp_provider = Rc::new(WasmRegExpProvider::new());
        let mut interp = Interpreter::with_console(Box::new(console_provider));
        interp.set_regexp_provider(regexp_provider);

        // Prepare execution
        if let Err(e) = interp.prepare(code, Some("playground.ts".into())) {
            return RunResult {
                value: None,
                error: Some(format!("Parse error: {}", e)),
                console: console_buffer.borrow().clone(),
            };
        }

        // Execute until completion
        loop {
            match interp.step() {
                Ok(StepResult::Continue) => continue,
                Ok(StepResult::Complete(runtime_value)) => {
                    let value_str = format_js_value(&runtime_value.value);
                    return RunResult {
                        value: Some(value_str),
                        error: None,
                        console: console_buffer.borrow().clone(),
                    };
                }
                Ok(StepResult::Done) => {
                    return RunResult {
                        value: Some("undefined".to_string()),
                        error: None,
                        console: console_buffer.borrow().clone(),
                    };
                }
                Ok(StepResult::NeedImports(imports)) => {
                    let import_names: Vec<String> =
                        imports.iter().map(|i| i.specifier.clone()).collect();
                    return RunResult {
                        value: None,
                        error: Some(format!(
                            "Module imports not supported in playground: {:?}",
                            import_names
                        )),
                        console: console_buffer.borrow().clone(),
                    };
                }
                Ok(StepResult::Suspended { .. }) => {
                    return RunResult {
                        value: None,
                        error: Some("Async operations not supported in playground".to_string()),
                        console: console_buffer.borrow().clone(),
                    };
                }
                Err(e) => {
                    return RunResult {
                        value: None,
                        error: Some(format!("{}", e)),
                        console: console_buffer.borrow().clone(),
                    };
                }
            }
        }
    }

    /// Reset the interpreter state.
    ///
    /// This is a no-op since each run already uses a fresh interpreter.
    pub fn reset(&mut self) {
        // No-op: each run creates a fresh interpreter
    }
}

impl Default for TsRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a JsValue as a string for display.
fn format_js_value(value: &JsValue) -> String {
    format_js_value_with_depth(value, 0, &mut Vec::new())
}

/// Format a JsValue with depth tracking to handle nested structures.
/// `seen` tracks object addresses to detect circular references.
fn format_js_value_with_depth(
    value: &JsValue,
    depth: usize,
    seen: &mut Vec<usize>,
) -> String {
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
            } else if n.fract() == 0.0 && n.abs() < 1e15 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        JsValue::String(s) => format!("\"{}\"", s),
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
            let result = format_object_contents(obj, depth, seen, MAX_ITEMS);
            seen.pop();
            result
        }
        JsValue::Symbol(sym) => match &sym.description {
            Some(desc) => format!("Symbol({})", desc),
            None => String::from("Symbol()"),
        },
    }
}

/// Format the contents of an object or array.
fn format_object_contents(
    obj: &crate::gc::Gc<crate::value::JsObject>,
    depth: usize,
    seen: &mut Vec<usize>,
    max_items: usize,
) -> String {
    use crate::value::PropertyKey;

    let obj_ref = obj.borrow();

    match &obj_ref.exotic {
        ExoticObject::Array { elements } => {
            let mut items = Vec::new();
            let length = elements.len();
            let display_len = length.min(max_items);

            for elem in elements.iter().take(display_len) {
                items.push(format_js_value_with_depth(elem, depth + 1, seen));
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
                // JsMapKey wraps JsValue, access via .0
                let key_str = format_js_value_with_depth(&k.0, depth + 1, seen);
                let val_str = format_js_value_with_depth(v, depth + 1, seen);
                items.push(format!("{} => {}", key_str, val_str));
            }

            if entries.len() > max_items {
                items.push(format!("... {} more entries", entries.len() - max_items));
            }

            format!("Map({}){{{}}}", entries.len(),
                if items.is_empty() { String::new() } else { format!(" {} ", items.join(", ")) })
        }
        ExoticObject::Set { entries } => {
            let mut items = Vec::new();
            let display_len = entries.len().min(max_items);

            for (i, v) in entries.iter().enumerate() {
                if i >= display_len {
                    break;
                }
                // JsMapKey wraps JsValue, access via .0
                items.push(format_js_value_with_depth(&v.0, depth + 1, seen));
            }

            if entries.len() > max_items {
                items.push(format!("... {} more items", entries.len() - max_items));
            }

            format!("Set({}){{{}}}", entries.len(),
                if items.is_empty() { String::new() } else { format!(" {} ", items.join(", ")) })
        }
        ExoticObject::Promise(_) => String::from("Promise { <pending> }"),
        ExoticObject::Generator(_) | ExoticObject::BytecodeGenerator(_) => {
            String::from("Generator { <suspended> }")
        }
        ExoticObject::Proxy(_) => String::from("Proxy {}"),
        ExoticObject::Boolean(b) => format!("[Boolean: {}]", b),
        ExoticObject::Number(n) => format!("[Number: {}]", n),
        ExoticObject::StringObj(s) => format!("[String: \"{}\"]", s),
        ExoticObject::Symbol(sym) => {
            match &sym.description {
                Some(desc) => format!("[Symbol: Symbol({})]", desc),
                None => String::from("[Symbol: Symbol()]"),
            }
        }
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
                let val_str = format_js_value_with_depth(&prop.value, depth + 1, seen);
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
