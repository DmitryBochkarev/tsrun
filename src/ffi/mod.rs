//! C FFI for the tsrun TypeScript interpreter.
//!
//! This module provides a C-compatible API for embedding tsrun in C/C++ applications.
//! All types are opaque pointers, and all functions use C calling conventions.
//!
//! # Thread Safety
//!
//! This library is NOT thread-safe. Use one `TsRunContext` per thread.
//!
//! # Memory Management
//!
//! - `TsRunContext`: Created by `tsrun_new()`, freed by `tsrun_free()`
//! - `TsRunValue`: Created by various functions, freed by `tsrun_value_free()`
//! - Error strings: Valid until the next tsrun_* call on the same context
//! - Allocated strings (from `tsrun_json_stringify`): Freed by `tsrun_free_string()`

extern crate alloc;

pub(crate) mod console;
mod context;
mod module;
mod native;
mod order;
mod regexp;
mod value;

use alloc::boxed::Box;
use alloc::ffi::CString;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{CStr, c_char, c_void};
use core::ptr;

use crate::prelude::FxHashMap;

use crate::value::CheapClone;
use crate::{Interpreter, JsValue, RuntimeValue};

// ============================================================================
// Version
// ============================================================================

/// Library version string
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");

/// Returns the library version string.
///
/// The returned string is valid for the lifetime of the library.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_version() -> *const c_char {
    VERSION.as_ptr() as *const c_char
}

// ============================================================================
// Opaque Types
// ============================================================================

/// Opaque interpreter context.
///
/// Contains the interpreter state, error storage, and native callback registry.
pub struct TsRunContext {
    pub(crate) interp: Interpreter,
    pub(crate) last_error: Option<CString>,
    pub(crate) native_callbacks: FxHashMap<usize, NativeCallbackWrapper>,
    /// Counter for generating unique FFI callback IDs
    pub(crate) next_ffi_id: usize,
    /// Console callback (None = no-op)
    pub(crate) console_callback: Option<ConsoleCallbackWrapper>,
}

impl TsRunContext {
    pub(crate) fn new() -> Self {
        Self {
            interp: Interpreter::new(),
            last_error: None,
            native_callbacks: FxHashMap::default(),
            next_ffi_id: 1, // Start at 1 so 0 means "not an FFI callback"
            console_callback: None,
        }
    }

    /// Set the last error and return a pointer to it.
    /// The pointer is valid until the next call to this function.
    pub(crate) fn set_error(&mut self, error: String) -> *const c_char {
        match CString::new(error) {
            Ok(c_str) => {
                self.last_error = Some(c_str);
                self.last_error.as_ref().map_or(ptr::null(), |s| s.as_ptr())
            }
            Err(_) => {
                // If error contains null bytes, use a fallback message
                // SAFETY: Static string has no null bytes
                self.last_error = Some(unsafe {
                    CString::from_vec_unchecked(b"Error: null byte in message".to_vec())
                });
                self.last_error.as_ref().map_or(ptr::null(), |s| s.as_ptr())
            }
        }
    }

    /// Clear the last error.
    pub(crate) fn clear_error(&mut self) {
        self.last_error = None;
    }
}

/// Opaque value handle.
///
/// Wraps a JavaScript value with a guard to prevent garbage collection.
pub struct TsRunValue {
    pub(crate) inner: RuntimeValue,
}

impl TsRunValue {
    pub(crate) fn from_runtime_value(rv: RuntimeValue) -> Box<Self> {
        Box::new(Self { inner: rv })
    }

    pub(crate) fn from_js_value(interp: &mut Interpreter, value: JsValue) -> Box<Self> {
        // For objects, create guard to keep alive
        if let JsValue::Object(ref obj) = value {
            let guard = interp.heap.create_guard();
            guard.guard(obj.cheap_clone());
            Box::new(Self {
                inner: RuntimeValue::with_guard(value, guard),
            })
        } else {
            Box::new(Self {
                inner: RuntimeValue::unguarded(value),
            })
        }
    }

    pub(crate) fn value(&self) -> &JsValue {
        self.inner.value()
    }
}

/// Native callback wrapper storing C function pointer and userdata.
pub(crate) struct NativeCallbackWrapper {
    pub callback: TsRunNativeFn,
    pub userdata: *mut c_void,
}

// ============================================================================
// Result Types
// ============================================================================

/// Result for operations returning a value.
#[repr(C)]
pub struct TsRunValueResult {
    /// The value, or NULL on error.
    pub value: *mut TsRunValue,
    /// Error message, or NULL on success. Valid until next tsrun_* call.
    pub error: *const c_char,
}

impl TsRunValueResult {
    pub(crate) fn ok(value: Box<TsRunValue>) -> Self {
        Self {
            value: Box::into_raw(value),
            error: ptr::null(),
        }
    }

    pub(crate) fn err(ctx: &mut TsRunContext, error: String) -> Self {
        Self {
            value: ptr::null_mut(),
            error: ctx.set_error(error),
        }
    }
}

/// Result for operations returning nothing.
#[repr(C)]
pub struct TsRunResult {
    /// True on success, false on error.
    pub ok: bool,
    /// Error message, or NULL on success. Valid until next tsrun_* call.
    pub error: *const c_char,
}

impl TsRunResult {
    pub(crate) fn success() -> Self {
        Self {
            ok: true,
            error: ptr::null(),
        }
    }

    pub(crate) fn err(ctx: &mut TsRunContext, error: String) -> Self {
        Self {
            ok: false,
            error: ctx.set_error(error),
        }
    }
}

// ============================================================================
// Value Types
// ============================================================================

/// JavaScript value types.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsRunType {
    Undefined = 0,
    Null = 1,
    Boolean = 2,
    Number = 3,
    String = 4,
    Object = 5,
    Symbol = 6,
}

// ============================================================================
// Step Status
// ============================================================================

/// Status of a step result.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsRunStepStatus {
    /// More instructions to execute.
    Continue = 0,
    /// Execution finished.
    Complete = 1,
    /// Waiting for modules.
    NeedImports = 2,
    /// Waiting for order fulfillment.
    Suspended = 3,
    /// No active execution.
    Done = 4,
    /// Execution error.
    Error = 5,
}

// ============================================================================
// Step Result
// ============================================================================

/// Import request data.
#[repr(C)]
pub struct TsRunImportRequest {
    /// Original import specifier (e.g., "./foo").
    pub specifier: *const c_char,
    /// Resolved absolute path.
    pub resolved_path: *const c_char,
    /// Module that requested this (NULL for main).
    pub importer: *const c_char,
}

/// Order from JS to host.
#[repr(C)]
pub struct TsRunOrder {
    /// Unique order ID.
    pub id: u64,
    /// Order payload value (owned by context).
    pub payload: *mut TsRunValue,
}

/// Step result with all possible data.
#[repr(C)]
pub struct TsRunStepResult {
    /// Status of this step.
    pub status: TsRunStepStatus,

    /// For TSRUN_STEP_COMPLETE: the result value.
    pub value: *mut TsRunValue,

    /// For TSRUN_STEP_NEED_IMPORTS: import requests.
    pub imports: *mut TsRunImportRequest,
    /// Number of import requests.
    pub import_count: usize,

    /// For TSRUN_STEP_SUSPENDED: pending orders.
    pub pending_orders: *mut TsRunOrder,
    /// Number of pending orders.
    pub pending_count: usize,

    /// For TSRUN_STEP_SUSPENDED: cancelled order IDs.
    pub cancelled_orders: *mut u64,
    /// Number of cancelled orders.
    pub cancelled_count: usize,

    /// For TSRUN_STEP_ERROR: error message.
    pub error: *const c_char,
}

impl Default for TsRunStepResult {
    fn default() -> Self {
        Self {
            status: TsRunStepStatus::Done,
            value: ptr::null_mut(),
            imports: ptr::null_mut(),
            import_count: 0,
            pending_orders: ptr::null_mut(),
            pending_count: 0,
            cancelled_orders: ptr::null_mut(),
            cancelled_count: 0,
            error: ptr::null(),
        }
    }
}

// ============================================================================
// Order Response
// ============================================================================

/// Order response from host to JS.
#[repr(C)]
pub struct TsRunOrderResponse {
    /// The order ID this response is for.
    pub id: u64,
    /// Result value (NULL if error).
    pub value: *mut TsRunValue,
    /// Error message (NULL if success).
    pub error: *const c_char,
}

// ============================================================================
// Console Callback
// ============================================================================

/// Console log level.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsRunConsoleLevel {
    Log = 0,
    Info = 1,
    Debug = 2,
    Warn = 3,
    Error = 4,
    Clear = 5,
}

/// Console callback signature.
///
/// Called synchronously during step/run when JS calls console methods.
/// For Clear level, message will be empty (message_len = 0).
pub type TsRunConsoleFn = extern "C" fn(
    level: TsRunConsoleLevel,
    message: *const c_char,
    message_len: usize,
    userdata: *mut c_void,
);

/// Console callback wrapper storing C function pointer and userdata.
pub(crate) struct ConsoleCallbackWrapper {
    pub callback: TsRunConsoleFn,
    pub userdata: *mut c_void,
}

// ============================================================================
// Native Function Callback
// ============================================================================

/// Native function callback signature.
///
/// Return NULL to return undefined. Set *error_out to a static string on error.
pub type TsRunNativeFn = extern "C" fn(
    ctx: *mut TsRunContext,
    this_arg: *mut TsRunValue,
    args: *mut *mut TsRunValue,
    argc: usize,
    userdata: *mut c_void,
    error_out: *mut *const c_char,
) -> *mut TsRunValue;

// ============================================================================
// GC Statistics
// ============================================================================

/// Garbage collector statistics.
#[repr(C)]
pub struct TsRunGcStats {
    /// Total number of GcBox slots (including pooled).
    pub total_objects: usize,
    /// Number of objects in the pool (available for reuse).
    pub pooled_objects: usize,
    /// Number of live objects.
    pub live_objects: usize,
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Free a string allocated by tsrun (e.g., from tsrun_json_stringify).
///
/// # Safety
/// `s` must be a pointer returned by a tsrun function (or NULL).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tsrun_free_string(s: *mut c_char) {
    if !s.is_null() {
        // SAFETY: s was allocated by a tsrun function using CString::into_raw
        unsafe { drop(CString::from_raw(s)) };
    }
}

/// Free a string array allocated by tsrun (e.g., from tsrun_keys).
///
/// # Safety
/// `strings` must be a pointer returned by a tsrun function (or NULL), and
/// `count` must match the count returned by that function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tsrun_free_strings(strings: *mut *mut c_char, count: usize) {
    if strings.is_null() {
        return;
    }
    // SAFETY: strings was allocated by a tsrun function, count matches original allocation
    unsafe {
        for i in 0..count {
            let s = *strings.add(i);
            if !s.is_null() {
                drop(CString::from_raw(s));
            }
        }
        // Free the array itself
        drop(Vec::from_raw_parts(strings, count, count));
    }
}

/// Helper to convert C string to Rust &str.
pub(crate) unsafe fn c_str_to_str<'a>(s: *const c_char) -> Option<&'a str> {
    if s.is_null() {
        None
    } else {
        // SAFETY: Caller guarantees s is a valid C string
        unsafe { CStr::from_ptr(s) }.to_str().ok()
    }
}

/// Helper to allocate a C string from Rust &str.
pub(crate) fn str_to_c_string(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}
