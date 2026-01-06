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
use crate::platform::{ConsoleLevel, ConsoleProvider};
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

        // Create a fresh interpreter with buffered console provider
        let console_provider = BufferedConsoleProvider::new(Rc::clone(&console_buffer));
        let mut interp = Interpreter::with_console(Box::new(console_provider));

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
            let obj_ref = obj.borrow();
            match &obj_ref.exotic {
                ExoticObject::Array { .. } => String::from("[Array]"),
                ExoticObject::Function(_) => String::from("[Function]"),
                _ => String::from("[Object]"),
            }
        }
        JsValue::Symbol(sym) => match &sym.description {
            Some(desc) => format!("Symbol({})", desc),
            None => String::from("Symbol()"),
        },
    }
}
