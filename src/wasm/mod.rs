//! WebAssembly API for the TypeScript interpreter.
//!
//! This module provides a step-based API for running TypeScript/JavaScript code
//! in the browser via WebAssembly, allowing the host to handle async operations.
//!
//! All values are accessed via handles (u32). Handle 0 is reserved for invalid/not found.
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
//!             // result.value_handle contains the result (inspect with value_as_* methods)
//!             const handle = result.value_handle;
//!             console.log('Result type:', runner.get_value_type(handle));
//!             console.log('Result:', runner.value_as_number(handle));
//!             runner.release_handle(handle);
//!             break;
//!         } else if (result.status === Status.ERROR) {
//!             console.error('Error:', result.error);
//!             break;
//!         } else if (result.status === Status.SUSPENDED) {
//!             // Handle orders using handle-based API
//!             const orderIds = runner.get_pending_order_ids();
//!             for (const orderId of orderIds) {
//!                 const payload = runner.get_order_payload(orderId);
//!                 // Process payload, create result
//!                 const result = runner.create_string("response");
//!                 runner.set_order_result(orderId, result);
//!             }
//!             runner.commit_fulfillments();
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

// ═══════════════════════════════════════════════════════════════════════════════
// Order Fulfillment Types
// ═══════════════════════════════════════════════════════════════════════════════

/// Kind of fulfillment for an order (used in builder pattern).
enum FulfillmentKind {
    /// Fulfill with a value (handle to result)
    Value(u32),
    /// Fulfill with an error (handle to error value)
    Error(u32),
}

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
    value_handle: u32,
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

    /// Get the result value handle (for Complete status, 0 if no value).
    #[wasm_bindgen(getter)]
    pub fn value_handle(&self) -> u32 {
        self.value_handle
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
/// 4. For Suspended status: get order IDs, get payloads, set results, commit
#[wasm_bindgen]
pub struct TsRunner {
    /// The interpreter instance (Some when prepared, None otherwise)
    interp: Option<Interpreter>,
    /// Buffer for console output
    console_buffer: Rc<RefCell<Vec<ConsoleEntry>>>,
    /// Cached pending orders from last Suspended state (order_id, payload_handle)
    pending_orders: Vec<(u64, u32)>,
    /// Cached import specifiers from last NeedImports state
    import_specifiers: Vec<String>,
    /// Value handles map: handle_id -> RuntimeValue
    /// Handles keep values alive across WASM boundary calls
    value_handles: alloc::collections::BTreeMap<u32, crate::RuntimeValue>,
    /// Counter for generating unique handle IDs (start at 1, 0 is reserved for invalid/not found)
    next_handle_id: u32,
    /// Handles that are resolvable promises (can call resolve_promise/reject_promise)
    resolvable_promise_handles: alloc::collections::BTreeSet<u32>,
    /// Pending fulfillments accumulated before commit (order_id, fulfillment_kind)
    pending_fulfillments: Vec<(u64, FulfillmentKind)>,
    /// Source modules to register when prepare() is called (specifier, source)
    source_modules: Vec<(String, String)>,
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
            value_handles: alloc::collections::BTreeMap::new(),
            next_handle_id: 1,
            resolvable_promise_handles: alloc::collections::BTreeSet::new(),
            pending_fulfillments: Vec::new(),
            source_modules: Vec::new(),
        }
    }

    /// Register a source module that can be imported by the prepared code.
    ///
    /// Must be called before `prepare()`. The specifier should use a custom
    /// prefix (e.g., "app:config", "mylib:utils") to avoid conflicts with
    /// built-in modules like "tsrun:host".
    ///
    /// # Example
    ///
    /// ```javascript
    /// const runner = new TsRunner();
    /// runner.register_source_module("app:math", `
    ///     export function double(x: number): number {
    ///         return x * 2;
    ///     }
    ///     export const PI = 3.14159;
    /// `);
    /// runner.prepare(`
    ///     import { double, PI } from "app:math";
    ///     console.log(double(PI));
    /// `, "main.ts");
    /// ```
    pub fn register_source_module(&mut self, specifier: &str, source: &str) {
        self.source_modules
            .push((specifier.to_string(), source.to_string()));
    }

    /// Prepare code for execution.
    ///
    /// This compiles the code and prepares the interpreter. Call `step()` after this.
    pub fn prepare(&mut self, code: &str, filename: Option<String>) -> WasmStepResult {
        // Clear previous state
        self.console_buffer.borrow_mut().clear();
        self.pending_orders.clear();
        self.import_specifiers.clear();
        self.value_handles.clear();
        self.next_handle_id = 1;
        self.resolvable_promise_handles.clear();
        self.pending_fulfillments.clear();

        // Create interpreter with tsrun:host module support and user-registered source modules
        let console_provider = BufferedConsoleProvider::new(Rc::clone(&self.console_buffer));
        let regexp_provider = Rc::new(WasmRegExpProvider::new());

        // Build internal modules list: tsrun:host + user-registered source modules
        let mut internal_modules = vec![create_eval_internal_module()];
        for (specifier, source) in self.source_modules.drain(..) {
            internal_modules.push(crate::InternalModule::source(specifier, source));
        }

        let config = InterpreterConfig {
            internal_modules,
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
                    value_handle: 0,
                    error: None,
                    console: Vec::new(),
                }
            }
            Ok(StepResult::NeedImports(imports)) => {
                self.import_specifiers = imports.iter().map(|i| i.specifier.clone()).collect();
                self.interp = Some(interp);
                WasmStepResult {
                    status: StepStatus::NeedImports,
                    value_handle: 0,
                    error: None,
                    console: self.console_buffer.borrow().clone(),
                }
            }
            Ok(_) => {
                // Other results from prepare are unexpected but handle them
                self.interp = Some(interp);
                WasmStepResult {
                    status: StepStatus::Continue,
                    value_handle: 0,
                    error: None,
                    console: Vec::new(),
                }
            }
            Err(e) => WasmStepResult {
                status: StepStatus::Error,
                value_handle: 0,
                error: Some(format!("Parse error: {}", e)),
                console: self.console_buffer.borrow().clone(),
            },
        }
    }

    /// Execute one step.
    ///
    /// Call this in a loop until the status is not Continue.
    /// For Suspended status, get order IDs, payloads, set results, and commit.
    pub fn step(&mut self) -> WasmStepResult {
        let interp = match &mut self.interp {
            Some(i) => i,
            None => {
                return WasmStepResult {
                    status: StepStatus::Error,
                    value_handle: 0,
                    error: Some("No prepared execution. Call prepare() first.".into()),
                    console: Vec::new(),
                };
            }
        };

        match interp.step() {
            Ok(StepResult::Continue) => WasmStepResult {
                status: StepStatus::Continue,
                value_handle: 0,
                error: None,
                console: self.take_console(),
            },
            Ok(StepResult::Complete(runtime_value)) => {
                // Allocate handle for the result value
                let handle = self.allocate_handle(runtime_value);
                // Keep interpreter alive for exports and handle operations
                WasmStepResult {
                    status: StepStatus::Complete,
                    value_handle: handle,
                    error: None,
                    console: self.take_console(),
                }
            }
            Ok(StepResult::Done) => {
                // Keep interpreter alive for exports and handle operations
                WasmStepResult {
                    status: StepStatus::Done,
                    value_handle: 0,
                    error: None,
                    console: self.take_console(),
                }
            }
            Ok(StepResult::NeedImports(imports)) => {
                self.import_specifiers = imports.iter().map(|i| i.specifier.clone()).collect();
                WasmStepResult {
                    status: StepStatus::NeedImports,
                    value_handle: 0,
                    error: None,
                    console: self.take_console(),
                }
            }
            Ok(StepResult::Suspended { pending, .. }) => {
                // Convert pending orders to handle-based format
                self.pending_orders = pending
                    .into_iter()
                    .map(|order| {
                        let payload_handle = self.allocate_handle(order.payload);
                        (order.id.0, payload_handle)
                    })
                    .collect();
                WasmStepResult {
                    status: StepStatus::Suspended,
                    value_handle: 0,
                    error: None,
                    console: self.take_console(),
                }
            }
            Err(e) => {
                self.interp = None;
                WasmStepResult {
                    status: StepStatus::Error,
                    value_handle: 0,
                    error: Some(format!("{}", e)),
                    console: self.take_console(),
                }
            }
        }
    }

    /// Get IDs of pending orders (call after Suspended status).
    pub fn get_pending_order_ids(&self) -> Vec<u64> {
        self.pending_orders.iter().map(|(id, _)| *id).collect()
    }

    /// Get the payload for a pending order as a handle.
    pub fn get_order_payload(&self, order_id: u64) -> u32 {
        self.pending_orders
            .iter()
            .find(|(id, _)| *id == order_id)
            .map(|(_, handle)| *handle)
            .unwrap_or(0)
    }

    /// Get import specifiers (call after NeedImports status).
    pub fn get_import_requests(&self) -> Vec<String> {
        self.import_specifiers.clone()
    }

    /// Queue a result for an order (call commit_fulfillments to process).
    pub fn set_order_result(&mut self, order_id: u64, result_handle: u32) {
        self.pending_fulfillments
            .push((order_id, FulfillmentKind::Value(result_handle)));
    }

    /// Queue an error for an order (call commit_fulfillments to process).
    pub fn set_order_error(&mut self, order_id: u64, error_handle: u32) {
        self.pending_fulfillments
            .push((order_id, FulfillmentKind::Error(error_handle)));
    }

    /// Process all queued fulfillments.
    pub fn commit_fulfillments(&mut self) {
        let interp = match &mut self.interp {
            Some(i) => i,
            None => return,
        };

        let mut order_responses = Vec::new();

        for (order_id, kind) in self.pending_fulfillments.drain(..) {
            match kind {
                FulfillmentKind::Value(result_handle) => {
                    // Get the value from handle
                    let value = self
                        .value_handles
                        .get(&result_handle)
                        .map(|rv| rv.value().clone())
                        .unwrap_or(RustJsValue::Undefined);

                    // Create RuntimeValue with guard if object
                    let runtime_value = if let RustJsValue::Object(ref obj) = value {
                        let guard = interp.heap.create_guard();
                        guard.guard(obj.cheap_clone());
                        RuntimeValue::with_guard(value, guard)
                    } else {
                        RuntimeValue::unguarded(value)
                    };

                    order_responses.push(OrderResponse {
                        id: crate::OrderId(order_id),
                        result: Ok(runtime_value),
                    });
                }
                FulfillmentKind::Error(error_handle) => {
                    // Get the error value from handle
                    let error_value = self
                        .value_handles
                        .get(&error_handle)
                        .map(|rv| rv.value().clone())
                        .unwrap_or(RustJsValue::Undefined);

                    // Convert to JsError
                    let error_msg = match &error_value {
                        RustJsValue::String(s) => s.to_string(),
                        RustJsValue::Object(obj) => {
                            // Try to get message property from Error object
                            let msg_key = PropertyKey::String(crate::JsString::from("message"));
                            obj.borrow()
                                .get_property(&msg_key)
                                .and_then(|v| {
                                    if let RustJsValue::String(s) = v {
                                        Some(s.to_string())
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or_else(|| "Unknown error".into())
                        }
                        _ => "Unknown error".into(),
                    };

                    order_responses.push(OrderResponse {
                        id: crate::OrderId(order_id),
                        result: Err(crate::JsError::type_error(error_msg)),
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

    /// Create an unresolved Promise (returns handle).
    ///
    /// The returned handle can be:
    /// 1. Passed as an order fulfillment result (via `set_order_result`)
    /// 2. Resolved later with `resolve_promise(handle, value_handle)`
    /// 3. Rejected later with `reject_promise(handle, error_handle)`
    ///
    /// This enables parallel async operations:
    /// ```javascript
    /// // When order 1 arrives, create Promise and return it
    /// const promise1 = runner.create_promise();
    /// runner.set_order_result(orderId1, promise1);
    ///
    /// // When order 2 arrives, create another Promise
    /// const promise2 = runner.create_promise();
    /// runner.set_order_result(orderId2, promise2);
    /// runner.commit_fulfillments();
    ///
    /// // JS code now has both Promises and can do Promise.all
    /// // Later, resolve both (host can do async work in parallel):
    /// const result1 = runner.create_string("data1");
    /// runner.resolve_promise(promise1, result1);
    /// const result2 = runner.create_string("data2");
    /// runner.resolve_promise(promise2, result2);
    /// ```
    pub fn create_promise(&mut self) -> u32 {
        let interp = match &mut self.interp {
            Some(i) => i,
            None => return 0,
        };

        let promise = crate::api::create_promise(interp);
        let handle = self.allocate_handle(promise);
        self.resolvable_promise_handles.insert(handle);
        handle
    }

    /// Resolve a Promise created by `create_promise`.
    ///
    /// Both arguments are handles.
    pub fn resolve_promise(&mut self, promise_handle: u32, value_handle: u32) {
        if !self.resolvable_promise_handles.contains(&promise_handle) {
            return;
        }

        let interp = match &mut self.interp {
            Some(i) => i,
            None => return,
        };

        // Get promise RuntimeValue
        let promise_rv = match self.value_handles.get(&promise_handle) {
            Some(p) => p,
            None => return,
        };

        // Get value from handle
        let value = self
            .value_handles
            .get(&value_handle)
            .map(|rv| rv.value().clone())
            .unwrap_or(RustJsValue::Undefined);

        // Create RuntimeValue with guard if object
        let runtime_value = if let RustJsValue::Object(ref obj) = value {
            let guard = interp.heap.create_guard();
            guard.guard(obj.cheap_clone());
            RuntimeValue::with_guard(value, guard)
        } else {
            RuntimeValue::unguarded(value)
        };

        // Resolve the Promise
        if crate::api::resolve_promise(interp, promise_rv, runtime_value).is_ok() {
            self.resolvable_promise_handles.remove(&promise_handle);
        }
    }

    /// Reject a Promise created by `create_promise`.
    ///
    /// Both arguments are handles (error_handle should be an Error object or string).
    pub fn reject_promise(&mut self, promise_handle: u32, error_handle: u32) {
        if !self.resolvable_promise_handles.contains(&promise_handle) {
            return;
        }

        let interp = match &mut self.interp {
            Some(i) => i,
            None => return,
        };

        // Get promise RuntimeValue
        let promise_rv = match self.value_handles.get(&promise_handle) {
            Some(p) => p,
            None => return,
        };

        // Get error value from handle
        let error_value = self
            .value_handles
            .get(&error_handle)
            .map(|rv| rv.value().clone())
            .unwrap_or(RustJsValue::String(crate::JsString::from("Unknown error")));

        // Create RuntimeValue with guard if object
        let runtime_error = if let RustJsValue::Object(ref obj) = error_value {
            let guard = interp.heap.create_guard();
            guard.guard(obj.cheap_clone());
            RuntimeValue::with_guard(error_value, guard)
        } else {
            RuntimeValue::unguarded(error_value)
        };

        // Reject the Promise
        if crate::api::reject_promise(interp, promise_rv, runtime_error).is_ok() {
            self.resolvable_promise_handles.remove(&promise_handle);
        }
    }

    /// Take console output (clears the buffer).
    fn take_console(&mut self) -> Vec<ConsoleEntry> {
        let console = self.console_buffer.borrow().clone();
        self.console_buffer.borrow_mut().clear();
        console
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Internal Handle Helpers
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Allocate a new handle for a RuntimeValue
    fn allocate_handle(&mut self, value: crate::RuntimeValue) -> u32 {
        let id = self.next_handle_id;
        self.next_handle_id += 1;
        self.value_handles.insert(id, value);
        id
    }

    /// Get a reference to a value by handle (returns None if invalid)
    fn get_handle(&self, handle: u32) -> Option<&crate::RuntimeValue> {
        if handle == 0 {
            return None;
        }
        self.value_handles.get(&handle)
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Value Handle API
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Release a handle (free the value and its guard).
    pub fn release_handle(&mut self, handle: u32) {
        if handle != 0 {
            self.value_handles.remove(&handle);
        }
    }

    /// Duplicate a handle (create new handle pointing to same value with new guard).
    pub fn duplicate_handle(&mut self, handle: u32) -> u32 {
        let value = match self.get_handle(handle) {
            Some(v) => v.value().clone(),
            None => return 0,
        };

        // For objects, create a new guard
        if let RustJsValue::Object(ref obj) = value {
            let interp = match &mut self.interp {
                Some(i) => i,
                None => return 0,
            };
            let guard = interp.heap.create_guard();
            guard.guard(obj.cheap_clone());
            self.allocate_handle(crate::RuntimeValue::with_guard(value, guard))
        } else {
            self.allocate_handle(crate::RuntimeValue::unguarded(value))
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Value Creation
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Create a handle for a number value.
    pub fn create_number(&mut self, n: f64) -> u32 {
        self.allocate_handle(crate::RuntimeValue::unguarded(RustJsValue::Number(n)))
    }

    /// Create a handle for a string value.
    pub fn create_string(&mut self, s: &str) -> u32 {
        self.allocate_handle(crate::RuntimeValue::unguarded(RustJsValue::String(
            crate::JsString::from(s),
        )))
    }

    /// Create a handle for a boolean value.
    pub fn create_bool(&mut self, b: bool) -> u32 {
        self.allocate_handle(crate::RuntimeValue::unguarded(RustJsValue::Boolean(b)))
    }

    /// Create a handle for null.
    pub fn create_null(&mut self) -> u32 {
        self.allocate_handle(crate::RuntimeValue::unguarded(RustJsValue::Null))
    }

    /// Create a handle for undefined.
    pub fn create_undefined(&mut self) -> u32 {
        self.allocate_handle(crate::RuntimeValue::unguarded(RustJsValue::Undefined))
    }

    /// Create a handle for an empty object.
    pub fn create_object(&mut self) -> u32 {
        let interp = match &mut self.interp {
            Some(i) => i,
            None => return 0,
        };
        let guard = interp.heap.create_guard();
        let obj = interp.create_object(&guard);
        self.allocate_handle(crate::RuntimeValue::with_guard(
            RustJsValue::Object(obj),
            guard,
        ))
    }

    /// Create a handle for an empty array.
    pub fn create_array(&mut self) -> u32 {
        let interp = match &mut self.interp {
            Some(i) => i,
            None => return 0,
        };
        let guard = interp.heap.create_guard();
        let arr = interp.create_array_from(&guard, vec![]);
        self.allocate_handle(crate::RuntimeValue::with_guard(
            RustJsValue::Object(arr),
            guard,
        ))
    }

    /// Create a handle for an Error object.
    pub fn create_error(&mut self, message: &str) -> u32 {
        let interp = match &mut self.interp {
            Some(i) => i,
            None => return 0,
        };
        let guard = interp.heap.create_guard();
        let error_obj = guard.alloc();

        // Set prototype to Error.prototype
        error_obj.borrow_mut().prototype = Some(interp.error_prototype.cheap_clone());

        // Set name, message, and stack properties
        let name_key = PropertyKey::String(crate::JsString::from("name"));
        let msg_key = PropertyKey::String(crate::JsString::from("message"));
        let stack_key = PropertyKey::String(crate::JsString::from("stack"));

        error_obj.borrow_mut().set_property(
            name_key,
            RustJsValue::String(crate::JsString::from("Error")),
        );
        error_obj
            .borrow_mut()
            .set_property(msg_key, RustJsValue::String(crate::JsString::from(message)));
        let stack = if message.is_empty() {
            "Error".into()
        } else {
            alloc::format!("Error: {}", message)
        };
        error_obj
            .borrow_mut()
            .set_property(stack_key, RustJsValue::String(crate::JsString::from(stack)));

        self.allocate_handle(crate::RuntimeValue::with_guard(
            RustJsValue::Object(error_obj),
            guard,
        ))
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Value Inspection
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Get the type of a value as a string.
    /// Returns: "undefined", "null", "boolean", "number", "string", "object", "symbol"
    pub fn get_value_type(&self, handle: u32) -> String {
        match self.get_handle(handle) {
            Some(rv) => match rv.value() {
                RustJsValue::Undefined => "undefined".into(),
                RustJsValue::Null => "null".into(),
                RustJsValue::Boolean(_) => "boolean".into(),
                RustJsValue::Number(_) => "number".into(),
                RustJsValue::String(_) => "string".into(),
                RustJsValue::Object(_) => "object".into(),
                RustJsValue::Symbol(_) => "symbol".into(),
            },
            None => "undefined".into(),
        }
    }

    /// Get value as number (returns NaN if not a number or invalid handle).
    pub fn value_as_number(&self, handle: u32) -> f64 {
        self.get_handle(handle)
            .and_then(|rv| rv.as_number())
            .unwrap_or(f64::NAN)
    }

    /// Get value as string (returns None/undefined in JS if not a string).
    pub fn value_as_string(&self, handle: u32) -> Option<String> {
        self.get_handle(handle)
            .and_then(|rv| rv.as_str())
            .map(|s| s.to_string())
    }

    /// Get value as boolean (returns None/undefined in JS if not a boolean).
    pub fn value_as_bool(&self, handle: u32) -> Option<bool> {
        self.get_handle(handle).and_then(|rv| rv.as_bool())
    }

    /// Check if value is null.
    pub fn value_is_null(&self, handle: u32) -> bool {
        self.get_handle(handle)
            .map(|rv| rv.is_null())
            .unwrap_or(false)
    }

    /// Check if value is undefined.
    pub fn value_is_undefined(&self, handle: u32) -> bool {
        self.get_handle(handle)
            .map(|rv| rv.is_undefined())
            .unwrap_or(true)
    }

    /// Check if value is an array.
    pub fn value_is_array(&self, handle: u32) -> bool {
        self.get_handle(handle)
            .map(|rv| rv.is_array())
            .unwrap_or(false)
    }

    /// Check if value is a function.
    pub fn value_is_function(&self, handle: u32) -> bool {
        self.get_handle(handle)
            .and_then(|rv| rv.value().as_object())
            .map(|obj| matches!(obj.borrow().exotic, ExoticObject::Function(_)))
            .unwrap_or(false)
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Object Operations
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Get a property from an object (returns new handle, 0 if not found or error).
    pub fn get_property(&mut self, obj_handle: u32, key: &str) -> u32 {
        let value = {
            let rv = match self.get_handle(obj_handle) {
                Some(v) => v,
                None => return 0,
            };
            let obj = match rv.value().as_object() {
                Some(o) => o,
                None => return 0,
            };
            let prop_key = PropertyKey::String(crate::JsString::from(key));
            obj.borrow()
                .get_property(&prop_key)
                .unwrap_or(RustJsValue::Undefined)
        };

        // Create new handle with guard if object
        if let RustJsValue::Object(ref obj) = value {
            let interp = match &mut self.interp {
                Some(i) => i,
                None => return 0,
            };
            let guard = interp.heap.create_guard();
            guard.guard(obj.cheap_clone());
            self.allocate_handle(crate::RuntimeValue::with_guard(value, guard))
        } else {
            self.allocate_handle(crate::RuntimeValue::unguarded(value))
        }
    }

    /// Set a property on an object (returns true on success).
    pub fn set_property(&mut self, obj_handle: u32, key: &str, value_handle: u32) -> bool {
        let value = match self.get_handle(value_handle) {
            Some(v) => v.value().clone(),
            None => RustJsValue::Undefined,
        };

        let obj = match self.get_handle(obj_handle) {
            Some(v) => match v.value().as_object() {
                Some(o) => o.cheap_clone(),
                None => return false,
            },
            None => return false,
        };

        let prop_key = PropertyKey::String(crate::JsString::from(key));
        obj.borrow_mut().set_property(prop_key, value);
        true
    }

    /// Delete a property from an object (returns true on success).
    pub fn delete_property(&mut self, obj_handle: u32, key: &str) -> bool {
        let obj = match self.get_handle(obj_handle) {
            Some(v) => match v.value().as_object() {
                Some(o) => o.cheap_clone(),
                None => return false,
            },
            None => return false,
        };

        let prop_key = PropertyKey::String(crate::JsString::from(key));
        obj.borrow_mut().properties.remove(&prop_key);
        true
    }

    /// Check if an object has a property.
    pub fn has_property(&self, obj_handle: u32, key: &str) -> bool {
        let rv = match self.get_handle(obj_handle) {
            Some(v) => v,
            None => return false,
        };
        let obj = match rv.value().as_object() {
            Some(o) => o,
            None => return false,
        };
        let prop_key = PropertyKey::String(crate::JsString::from(key));
        obj.borrow().get_property(&prop_key).is_some()
    }

    /// Get all property keys of an object.
    pub fn get_keys(&self, obj_handle: u32) -> Vec<String> {
        let rv = match self.get_handle(obj_handle) {
            Some(v) => v,
            None => return Vec::new(),
        };
        let obj = match rv.value().as_object() {
            Some(o) => o,
            None => return Vec::new(),
        };

        obj.borrow()
            .properties
            .keys()
            .filter_map(|k| match k {
                PropertyKey::String(s) => Some(s.to_string()),
                PropertyKey::Index(i) => Some(i.to_string()),
                PropertyKey::Symbol(_) => None,
            })
            .collect()
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Array Operations
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Get an array element by index (returns new handle).
    pub fn get_index(&mut self, arr_handle: u32, index: u32) -> u32 {
        let value = {
            let rv = match self.get_handle(arr_handle) {
                Some(v) => v,
                None => return 0,
            };
            let obj = match rv.value().as_object() {
                Some(o) => o,
                None => return 0,
            };
            let borrowed = obj.borrow();
            match &borrowed.exotic {
                ExoticObject::Array { elements } => elements
                    .get(index as usize)
                    .cloned()
                    .unwrap_or(RustJsValue::Undefined),
                _ => return 0,
            }
        };

        if let RustJsValue::Object(ref obj) = value {
            let interp = match &mut self.interp {
                Some(i) => i,
                None => return 0,
            };
            let guard = interp.heap.create_guard();
            guard.guard(obj.cheap_clone());
            self.allocate_handle(crate::RuntimeValue::with_guard(value, guard))
        } else {
            self.allocate_handle(crate::RuntimeValue::unguarded(value))
        }
    }

    /// Set an array element by index (returns true on success).
    pub fn set_index(&mut self, arr_handle: u32, index: u32, value_handle: u32) -> bool {
        let value = match self.get_handle(value_handle) {
            Some(v) => v.value().clone(),
            None => RustJsValue::Undefined,
        };

        let obj = match self.get_handle(arr_handle) {
            Some(v) => match v.value().as_object() {
                Some(o) => o.cheap_clone(),
                None => return false,
            },
            None => return false,
        };

        let mut borrowed = obj.borrow_mut();
        match &mut borrowed.exotic {
            ExoticObject::Array { elements } => {
                let idx = index as usize;
                while elements.len() <= idx {
                    elements.push(RustJsValue::Undefined);
                }
                if let Some(elem) = elements.get_mut(idx) {
                    *elem = value;
                }
                let new_len = elements.len();
                drop(borrowed);
                obj.borrow_mut().set_property(
                    PropertyKey::String(crate::JsString::from("length")),
                    RustJsValue::Number(new_len as f64),
                );
                true
            }
            _ => false,
        }
    }

    /// Push a value onto an array (returns true on success).
    pub fn push(&mut self, arr_handle: u32, value_handle: u32) -> bool {
        let value = match self.get_handle(value_handle) {
            Some(v) => v.value().clone(),
            None => RustJsValue::Undefined,
        };

        let obj = match self.get_handle(arr_handle) {
            Some(v) => match v.value().as_object() {
                Some(o) => o.cheap_clone(),
                None => return false,
            },
            None => return false,
        };

        let mut borrowed = obj.borrow_mut();
        match &mut borrowed.exotic {
            ExoticObject::Array { elements } => {
                elements.push(value);
                let new_len = elements.len();
                drop(borrowed);
                obj.borrow_mut().set_property(
                    PropertyKey::String(crate::JsString::from("length")),
                    RustJsValue::Number(new_len as f64),
                );
                true
            }
            _ => false,
        }
    }

    /// Get the length of an array (returns 0 if not an array).
    pub fn array_length(&self, arr_handle: u32) -> u32 {
        let rv = match self.get_handle(arr_handle) {
            Some(v) => v,
            None => return 0,
        };
        let obj = match rv.value().as_object() {
            Some(o) => o,
            None => return 0,
        };

        let borrowed = obj.borrow();
        match &borrowed.exotic {
            ExoticObject::Array { elements } => elements.len() as u32,
            _ => 0,
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Export Access
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Get an export from the main module by name (returns handle, 0 if not found).
    pub fn get_export(&mut self, name: &str) -> u32 {
        let value = {
            let interp = match &self.interp {
                Some(i) => i,
                None => return 0,
            };

            match interp.get_export(name) {
                Some(v) => v,
                None => return 0,
            }
        };

        // Create handle with guard if object
        if let RustJsValue::Object(ref obj) = value {
            let interp = match &mut self.interp {
                Some(i) => i,
                None => return 0,
            };
            let guard = interp.heap.create_guard();
            guard.guard(obj.cheap_clone());
            self.allocate_handle(crate::RuntimeValue::with_guard(value, guard))
        } else {
            self.allocate_handle(crate::RuntimeValue::unguarded(value))
        }
    }

    /// Get all export names from the main module.
    pub fn get_export_names(&self) -> Vec<String> {
        match &self.interp {
            Some(i) => i.get_export_names(),
            None => Vec::new(),
        }
    }
}

impl Default for TsRunner {
    fn default() -> Self {
        Self::new()
    }
}
