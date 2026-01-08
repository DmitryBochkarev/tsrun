//! WebAssembly API for the TypeScript interpreter.
//!
//! This module provides a step-based API for running TypeScript/JavaScript code
//! in the browser via WebAssembly, allowing the host to handle async operations.
//!
//! # Example (JavaScript)
//!
//! ```javascript
//! import init, { TsRunner, STEP_CONTINUE, STEP_COMPLETE, STEP_ERROR, STEP_SUSPENDED } from './pkg/tsrun.js';
//!
//! async function main() {
//!     await init();
//!     const runner = new TsRunner();
//!
//!     // Status constants (functions returning values)
//!     const Status = {
//!         CONTINUE: STEP_CONTINUE(),
//!         COMPLETE: STEP_COMPLETE(),
//!         ERROR: STEP_ERROR(),
//!         SUSPENDED: STEP_SUSPENDED()
//!     };
//!
//!     runner.prepare('console.log("Hello!"); 42', 'script.ts');
//!
//!     while (true) {
//!         const result = runner.step();
//!
//!         // Display console output from this step
//!         for (const entry of result.console_output) {
//!             console.log(`[${entry.level}] ${entry.message}`);
//!         }
//!
//!         if (result.status === Status.COMPLETE) {
//!             console.log('Result:', result.value);
//!             break;
//!         } else if (result.status === Status.ERROR) {
//!             console.error('Error:', result.error);
//!             break;
//!         } else if (result.status === Status.SUSPENDED) {
//!             const orders = runner.get_pending_orders();
//!             // Handle orders with async operations (e.g., setTimeout, fetch)
//!             const responses = await handleOrders(orders);
//!             runner.fulfill_orders(responses);
//!         }
//!     }
//! }
//! ```

use crate::StepResult;
use crate::interpreter::Interpreter;
use crate::platform::{ConsoleLevel, ConsoleProvider, WasmRegExpProvider};
use crate::prelude::*;
use crate::value::{CheapClone, ExoticObject, PropertyKey};
use crate::{InterpreterConfig, OrderResponse, RuntimeValue, create_eval_internal_module};
use alloc::rc::Rc;
use core::cell::RefCell;
use wasm_bindgen::prelude::*;

// Type alias to disambiguate between tsrun's JsValue and wasm_bindgen's JsValue
type RustJsValue = crate::value::JsValue;
type WasmJsValue = wasm_bindgen::JsValue;

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
// Step Status and Result
// ═══════════════════════════════════════════════════════════════════════════════

/// Status of a step execution.
#[wasm_bindgen]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    /// Execution can continue (call step() again)
    Continue = 0,
    /// Execution completed with a value
    Complete = 1,
    /// Execution needs external modules loaded
    NeedImports = 2,
    /// Execution suspended waiting for orders to be fulfilled
    Suspended = 3,
    /// Execution finished (no return value)
    Done = 4,
    /// Execution encountered an error
    Error = 5,
}

// Export step status constants for JS access as getter functions
// (wasm_bindgen doesn't support const exports, so we use functions)
#[allow(non_snake_case)]
#[wasm_bindgen]
pub fn STEP_CONTINUE() -> u8 {
    StepStatus::Continue as u8
}
#[allow(non_snake_case)]
#[wasm_bindgen]
pub fn STEP_COMPLETE() -> u8 {
    StepStatus::Complete as u8
}
#[allow(non_snake_case)]
#[wasm_bindgen]
pub fn STEP_NEED_IMPORTS() -> u8 {
    StepStatus::NeedImports as u8
}
#[allow(non_snake_case)]
#[wasm_bindgen]
pub fn STEP_SUSPENDED() -> u8 {
    StepStatus::Suspended as u8
}
#[allow(non_snake_case)]
#[wasm_bindgen]
pub fn STEP_DONE() -> u8 {
    StepStatus::Done as u8
}
#[allow(non_snake_case)]
#[wasm_bindgen]
pub fn STEP_ERROR() -> u8 {
    StepStatus::Error as u8
}

/// Result of a step operation.
#[wasm_bindgen]
pub struct WasmStepResult {
    status: StepStatus,
    value: Option<String>,
    error: Option<String>,
    console: Vec<ConsoleEntry>,
}

#[wasm_bindgen]
impl WasmStepResult {
    /// Get the execution status.
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> StepStatus {
        self.status
    }

    /// Get the result value as a string (for Complete status).
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> Option<String> {
        self.value.clone()
    }

    /// Get the error message (for Error status).
    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }

    /// Get the console output captured during this step.
    #[wasm_bindgen(getter)]
    pub fn console_output(&self) -> Vec<ConsoleEntry> {
        self.console.clone()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TypeScript Runner
// ═══════════════════════════════════════════════════════════════════════════════

/// TypeScript/JavaScript interpreter for WebAssembly with step-based execution.
///
/// Usage:
/// 1. Create with `new TsRunner()`
/// 2. Call `prepare(code, filename)` to compile code
/// 3. Call `step()` in a loop until non-Continue status
/// 4. For Suspended status: get orders, handle them, call `fulfill_orders()`
#[wasm_bindgen]
pub struct TsRunner {
    /// The interpreter instance (Some when prepared, None otherwise)
    interp: Option<Interpreter>,
    /// Buffer for console output
    console_buffer: Rc<RefCell<Vec<ConsoleEntry>>>,
    /// Cached pending orders from last Suspended state (id, payload as WasmJsValue)
    pending_orders: Vec<(u64, WasmJsValue)>,
    /// Cached import specifiers from last NeedImports state
    import_specifiers: Vec<String>,
    /// Host-created Promises awaiting resolution (promise_id -> RuntimeValue)
    host_promises: alloc::collections::BTreeMap<u32, crate::RuntimeValue>,
    /// Counter for generating unique promise IDs
    next_promise_id: u32,
}

#[wasm_bindgen]
impl TsRunner {
    /// Create a new TypeScript runner.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            interp: None,
            console_buffer: Rc::new(RefCell::new(Vec::new())),
            pending_orders: Vec::new(),
            import_specifiers: Vec::new(),
            host_promises: alloc::collections::BTreeMap::new(),
            next_promise_id: 1,
        }
    }

    /// Prepare code for execution.
    ///
    /// This compiles the code and prepares the interpreter. Call `step()` after this.
    pub fn prepare(&mut self, code: &str, filename: Option<String>) -> WasmStepResult {
        // Clear previous state
        self.console_buffer.borrow_mut().clear();
        self.pending_orders.clear();
        self.import_specifiers.clear();
        self.host_promises.clear();

        // Create interpreter with tsrun:host module support
        let console_provider = BufferedConsoleProvider::new(Rc::clone(&self.console_buffer));
        let regexp_provider = Rc::new(WasmRegExpProvider::new());
        let config = InterpreterConfig {
            internal_modules: vec![create_eval_internal_module()],
            regexp_provider: Some(regexp_provider),
        };
        let mut interp = Interpreter::with_config(config);
        interp.set_console(Box::new(console_provider));

        // Prepare execution
        let module_path = filename.map(crate::ModulePath::new);
        match interp.prepare(code, module_path) {
            Ok(StepResult::Continue) => {
                self.interp = Some(interp);
                WasmStepResult {
                    status: StepStatus::Continue,
                    value: None,
                    error: None,
                    console: Vec::new(),
                }
            }
            Ok(StepResult::NeedImports(imports)) => {
                self.import_specifiers = imports.iter().map(|i| i.specifier.clone()).collect();
                self.interp = Some(interp);
                WasmStepResult {
                    status: StepStatus::NeedImports,
                    value: None,
                    error: None,
                    console: self.console_buffer.borrow().clone(),
                }
            }
            Ok(_) => {
                // Other results from prepare are unexpected but handle them
                self.interp = Some(interp);
                WasmStepResult {
                    status: StepStatus::Continue,
                    value: None,
                    error: None,
                    console: Vec::new(),
                }
            }
            Err(e) => WasmStepResult {
                status: StepStatus::Error,
                value: None,
                error: Some(format!("Parse error: {}", e)),
                console: self.console_buffer.borrow().clone(),
            },
        }
    }

    /// Execute one step.
    ///
    /// Call this in a loop until the status is not Continue.
    /// For Suspended status, handle orders and call `fulfill_orders()` before continuing.
    pub fn step(&mut self) -> WasmStepResult {
        let interp = match &mut self.interp {
            Some(i) => i,
            None => {
                return WasmStepResult {
                    status: StepStatus::Error,
                    value: None,
                    error: Some("No prepared execution. Call prepare() first.".into()),
                    console: Vec::new(),
                };
            }
        };

        match interp.step() {
            Ok(StepResult::Continue) => WasmStepResult {
                status: StepStatus::Continue,
                value: None,
                error: None,
                console: self.take_console(),
            },
            Ok(StepResult::Complete(runtime_value)) => {
                let value_str = format_js_value(runtime_value.value());
                self.interp = None;
                WasmStepResult {
                    status: StepStatus::Complete,
                    value: Some(value_str),
                    error: None,
                    console: self.take_console(),
                }
            }
            Ok(StepResult::Done) => {
                self.interp = None;
                WasmStepResult {
                    status: StepStatus::Done,
                    value: None,
                    error: None,
                    console: self.take_console(),
                }
            }
            Ok(StepResult::NeedImports(imports)) => {
                self.import_specifiers = imports.iter().map(|i| i.specifier.clone()).collect();
                WasmStepResult {
                    status: StepStatus::NeedImports,
                    value: None,
                    error: None,
                    console: self.take_console(),
                }
            }
            Ok(StepResult::Suspended { pending, .. }) => {
                // Convert pending orders to JS-accessible format
                self.pending_orders = pending
                    .iter()
                    .map(|order| {
                        let payload = rust_value_to_wasm_value(order.payload.value());
                        (order.id.0, payload)
                    })
                    .collect();
                WasmStepResult {
                    status: StepStatus::Suspended,
                    value: None,
                    error: None,
                    console: self.take_console(),
                }
            }
            Err(e) => {
                self.interp = None;
                WasmStepResult {
                    status: StepStatus::Error,
                    value: None,
                    error: Some(format!("{}", e)),
                    console: self.take_console(),
                }
            }
        }
    }

    /// Get pending orders (call after Suspended status).
    ///
    /// Returns an array of `{ id: number, payload: any }` objects.
    pub fn get_pending_orders(&self) -> WasmJsValue {
        let array = js_sys::Array::new();
        for (id, payload) in &self.pending_orders {
            let obj = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&obj, &"id".into(), &(*id as f64).into());
            let _ = js_sys::Reflect::set(&obj, &"payload".into(), payload);
            array.push(&obj);
        }
        array.into()
    }

    /// Get import specifiers (call after NeedImports status).
    pub fn get_import_requests(&self) -> Vec<String> {
        self.import_specifiers.clone()
    }

    /// Fulfill pending orders and continue execution.
    ///
    /// `responses` should be a JS array of objects with one of these formats:
    /// - `{ id: number, result: any }` - fulfill with a direct value
    /// - `{ id: number, promise_id: number }` - fulfill with a host-created Promise
    /// - `{ id: number, error: string }` - fulfill with an error
    pub fn fulfill_orders(&mut self, responses: WasmJsValue) {
        let interp = match &mut self.interp {
            Some(i) => i,
            None => return,
        };

        // Parse responses from JS
        let responses_array: js_sys::Array = match responses.dyn_into() {
            Ok(arr) => arr,
            Err(_) => return,
        };

        let mut order_responses = Vec::new();

        for i in 0..responses_array.length() {
            let resp = responses_array.get(i);

            // Get id
            let id_val =
                js_sys::Reflect::get(&resp, &"id".into()).unwrap_or(WasmJsValue::UNDEFINED);
            let id = id_val.as_f64().unwrap_or(0.0) as u64;

            // Check for error first
            let error_val =
                js_sys::Reflect::get(&resp, &"error".into()).unwrap_or(WasmJsValue::UNDEFINED);

            if error_val.is_truthy() {
                let error_str = error_val
                    .as_string()
                    .unwrap_or_else(|| "Unknown error".into());
                order_responses.push(OrderResponse {
                    id: crate::OrderId(id),
                    result: Err(crate::JsError::type_error(error_str)),
                });
                continue;
            }

            // Check for promise_id (host-created Promise reference)
            let promise_id_val =
                js_sys::Reflect::get(&resp, &"promise_id".into()).unwrap_or(WasmJsValue::UNDEFINED);

            if let Some(promise_id) = promise_id_val.as_f64() {
                let promise_id = promise_id as u32;
                if let Some(promise_rv) = self.host_promises.get(&promise_id) {
                    // Create a new guard for the Promise value
                    if let RustJsValue::Object(obj) = promise_rv.value() {
                        let guard = interp.heap.create_guard();
                        guard.guard(obj.cheap_clone());
                        order_responses.push(OrderResponse {
                            id: crate::OrderId(id),
                            result: Ok(RuntimeValue::with_guard(
                                RustJsValue::Object(obj.cheap_clone()),
                                guard,
                            )),
                        });
                        continue;
                    }
                }
            }

            // Otherwise, use direct result value
            let result_val =
                js_sys::Reflect::get(&resp, &"result".into()).unwrap_or(WasmJsValue::UNDEFINED);
            match wasm_value_to_runtime_value(interp, &result_val) {
                Ok(runtime_value) => {
                    order_responses.push(OrderResponse {
                        id: crate::OrderId(id),
                        result: Ok(runtime_value),
                    });
                }
                Err(e) => {
                    order_responses.push(OrderResponse {
                        id: crate::OrderId(id),
                        result: Err(e),
                    });
                }
            }
        }

        interp.fulfill_orders(order_responses);
        self.pending_orders.clear();
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Promise API for Parallel Operations
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Create an unresolved Promise.
    ///
    /// Returns a promise ID that can be:
    /// 1. Passed as an order fulfillment result (via `fulfill_orders`)
    /// 2. Resolved later with `resolve_promise(id, value)`
    /// 3. Rejected later with `reject_promise(id, error)`
    ///
    /// This enables parallel async operations:
    /// ```javascript
    /// // When order 1 arrives, create Promise and return it
    /// const promiseId1 = runner.create_promise();
    /// runner.fulfill_orders([{ id: orderId1, promise_id: promiseId1 }]);
    ///
    /// // When order 2 arrives, create another Promise
    /// const promiseId2 = runner.create_promise();
    /// runner.fulfill_orders([{ id: orderId2, promise_id: promiseId2 }]);
    ///
    /// // JS code now has both Promises and can do Promise.all
    /// // Later, resolve both (host can do async work in parallel):
    /// runner.resolve_promise(promiseId1, { data: "result1" });
    /// runner.resolve_promise(promiseId2, { data: "result2" });
    /// ```
    pub fn create_promise(&mut self) -> u32 {
        let interp = match &mut self.interp {
            Some(i) => i,
            None => return 0,
        };

        let promise = crate::api::create_promise(interp);
        let id = self.next_promise_id;
        self.next_promise_id += 1;
        self.host_promises.insert(id, promise);
        id
    }

    /// Get the Promise value for a given promise ID.
    ///
    /// Used when fulfilling orders with a Promise. The returned value
    /// should be passed as the `result` in `fulfill_orders`.
    pub fn get_promise_value(&self, promise_id: u32) -> WasmJsValue {
        match self.host_promises.get(&promise_id) {
            Some(_) => {
                // Return a special object that indicates this is a Promise reference
                let obj = js_sys::Object::new();
                js_sys::Reflect::set(&obj, &"__promise_id__".into(), &(promise_id as f64).into())
                    .ok();
                obj.into()
            }
            None => WasmJsValue::UNDEFINED,
        }
    }

    /// Resolve a Promise created by `create_promise`.
    ///
    /// The value can be any JS value (object, string, number, etc.).
    pub fn resolve_promise(&mut self, promise_id: u32, value: WasmJsValue) {
        let interp = match &mut self.interp {
            Some(i) => i,
            None => return,
        };

        let promise_rv = match self.host_promises.get(&promise_id) {
            Some(p) => p,
            None => return,
        };

        // Convert JS value to RuntimeValue with proper guard
        let runtime_value = match wasm_value_to_runtime_value(interp, &value) {
            Ok(v) => v,
            Err(_) => RuntimeValue::unguarded(RustJsValue::Undefined),
        };

        // Resolve the Promise
        if crate::api::resolve_promise(interp, promise_rv, runtime_value).is_ok() {
            // Remove from our tracking (Promise is now resolved)
            self.host_promises.remove(&promise_id);
        }
    }

    /// Reject a Promise created by `create_promise`.
    ///
    /// The error can be a string or any JS value.
    pub fn reject_promise(&mut self, promise_id: u32, error: WasmJsValue) {
        let interp = match &mut self.interp {
            Some(i) => i,
            None => return,
        };

        let promise_rv = match self.host_promises.get(&promise_id) {
            Some(p) => p,
            None => return,
        };

        // Convert JS error to RuntimeValue with proper guard
        let runtime_error = match wasm_value_to_runtime_value(interp, &error) {
            Ok(v) => v,
            Err(_) => {
                RuntimeValue::unguarded(RustJsValue::String(crate::JsString::from("Unknown error")))
            }
        };

        // Reject the Promise
        if crate::api::reject_promise(interp, promise_rv, runtime_error).is_ok() {
            // Remove from our tracking (Promise is now rejected)
            self.host_promises.remove(&promise_id);
        }
    }

    /// Take console output (clears the buffer).
    fn take_console(&mut self) -> Vec<ConsoleEntry> {
        let console = self.console_buffer.borrow().clone();
        self.console_buffer.borrow_mut().clear();
        console
    }
}

impl Default for TsRunner {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Value Conversion Helpers
// ═══════════════════════════════════════════════════════════════════════════════

/// Convert tsrun JsValue to wasm_bindgen JsValue for JS access.
fn rust_value_to_wasm_value(value: &RustJsValue) -> WasmJsValue {
    match value {
        RustJsValue::Undefined => WasmJsValue::UNDEFINED,
        RustJsValue::Null => WasmJsValue::NULL,
        RustJsValue::Boolean(b) => (*b).into(),
        RustJsValue::Number(n) => (*n).into(),
        RustJsValue::String(s) => s.as_str().into(),
        RustJsValue::Object(_) => {
            // Convert to valid JSON string then parse in JS
            let json_str = rust_value_to_json(value);
            js_sys::JSON::parse(&json_str).unwrap_or(WasmJsValue::UNDEFINED)
        }
        RustJsValue::Symbol(sym) => {
            let desc = sym.description.as_ref().map(|s| s.as_str()).unwrap_or("");
            format!("Symbol({})", desc).into()
        }
    }
}

/// Convert tsrun JsValue to valid JSON string.
fn rust_value_to_json(value: &RustJsValue) -> String {
    match value {
        RustJsValue::Undefined => String::from("null"),
        RustJsValue::Null => String::from("null"),
        RustJsValue::Boolean(b) => {
            if *b {
                String::from("true")
            } else {
                String::from("false")
            }
        }
        RustJsValue::Number(n) => {
            if n.is_nan() || n.is_infinite() {
                String::from("null")
            } else {
                format!("{}", n)
            }
        }
        RustJsValue::String(s) => {
            // Escape special characters for JSON
            let escaped: String = s
                .as_str()
                .chars()
                .map(|c| match c {
                    '"' => "\\\"".to_string(),
                    '\\' => "\\\\".to_string(),
                    '\n' => "\\n".to_string(),
                    '\r' => "\\r".to_string(),
                    '\t' => "\\t".to_string(),
                    c if c.is_control() => format!("\\u{:04x}", c as u32),
                    c => c.to_string(),
                })
                .collect();
            format!("\"{}\"", escaped)
        }
        RustJsValue::Object(obj) => {
            let obj_ref = obj.borrow();
            match &obj_ref.exotic {
                ExoticObject::Array { elements } => {
                    let items: Vec<String> =
                        elements.iter().map(|e| rust_value_to_json(e)).collect();
                    format!("[{}]", items.join(","))
                }
                _ => {
                    // Plain object - serialize properties as JSON
                    let mut items = Vec::new();
                    for (key, prop) in obj_ref.properties.iter() {
                        let key_str = match key {
                            PropertyKey::String(s) => s.to_string(),
                            PropertyKey::Symbol(_) => continue,
                            PropertyKey::Index(i) => i.to_string(),
                        };
                        let val_json = rust_value_to_json(&prop.value);
                        items.push(format!("\"{}\":{}", key_str, val_json));
                    }
                    format!("{{{}}}", items.join(","))
                }
            }
        }
        RustJsValue::Symbol(_) => String::from("null"),
    }
}

/// Convert wasm_bindgen JsValue to tsrun RuntimeValue with proper GC guard.
fn wasm_value_to_runtime_value(
    interp: &mut Interpreter,
    js: &WasmJsValue,
) -> Result<RuntimeValue, crate::JsError> {
    // Primitives don't need guards
    if js.is_undefined() {
        return Ok(RuntimeValue::unguarded(RustJsValue::Undefined));
    }
    if js.is_null() {
        return Ok(RuntimeValue::unguarded(RustJsValue::Null));
    }
    if let Some(b) = js.as_bool() {
        return Ok(RuntimeValue::unguarded(RustJsValue::Boolean(b)));
    }
    if let Some(n) = js.as_f64() {
        return Ok(RuntimeValue::unguarded(RustJsValue::Number(n)));
    }
    if let Some(s) = js.as_string() {
        return Ok(RuntimeValue::unguarded(RustJsValue::String(s.into())));
    }

    // For objects/arrays, serialize to JSON and parse
    let json_str = js_sys::JSON::stringify(js)
        .map_err(|_| crate::JsError::type_error("Cannot serialize value"))?
        .as_string()
        .ok_or_else(|| crate::JsError::type_error("Cannot serialize value"))?;

    let json_value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| crate::JsError::type_error(format!("JSON parse error: {}", e)))?;

    // Create guard and keep it with the value
    let guard = interp.heap.create_guard();
    let js_value = crate::json_to_js_value_with_guard(interp, &json_value, &guard)?;

    // Return RuntimeValue with guard to keep objects alive
    if let RustJsValue::Object(ref obj) = js_value {
        guard.guard(obj.cheap_clone());
        Ok(RuntimeValue::with_guard(js_value, guard))
    } else {
        Ok(RuntimeValue::unguarded(js_value))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Value Formatting
// ═══════════════════════════════════════════════════════════════════════════════

/// Format a JsValue as a string for display.
fn format_js_value(value: &RustJsValue) -> String {
    format_js_value_with_depth(value, 0, &mut Vec::new())
}

/// Format a JsValue with depth tracking to handle nested structures.
fn format_js_value_with_depth(value: &RustJsValue, depth: usize, seen: &mut Vec<usize>) -> String {
    const MAX_DEPTH: usize = 10;
    const MAX_ITEMS: usize = 100;

    match value {
        RustJsValue::Undefined => String::from("undefined"),
        RustJsValue::Null => String::from("null"),
        RustJsValue::Boolean(b) => {
            if *b {
                String::from("true")
            } else {
                String::from("false")
            }
        }
        RustJsValue::Number(n) => {
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
        RustJsValue::String(s) => format!("\"{}\"", s),
        RustJsValue::Object(obj) => {
            let obj_id = obj.id();
            if seen.contains(&obj_id) {
                return String::from("[Circular]");
            }

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
        RustJsValue::Symbol(sym) => match &sym.description {
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
                let key_str = format_js_value_with_depth(&k.0, depth + 1, seen);
                let val_str = format_js_value_with_depth(v, depth + 1, seen);
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
                items.push(format_js_value_with_depth(&v.0, depth + 1, seen));
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
            let mut items = Vec::new();
            let mut count = 0;

            for (key, prop) in obj_ref.properties.iter() {
                if count >= max_items {
                    break;
                }
                let key_str = match key {
                    PropertyKey::String(s) => s.to_string(),
                    PropertyKey::Symbol(_) => continue,
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
