//! Context lifecycle and execution functions.

extern crate alloc;

use alloc::boxed::Box;
use alloc::ffi::CString;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};
use core::ptr;

use crate::{ModulePath, StepResult};

use super::{
    TsRunContext, TsRunImportRequest, TsRunOrder, TsRunResult, TsRunStepResult, TsRunStepStatus,
    TsRunValue, c_str_to_str, console::FfiConsoleProvider, str_to_c_string,
};

// ============================================================================
// Context Lifecycle
// ============================================================================

/// Create a new interpreter context.
///
/// Returns NULL on failure (unlikely).
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_new() -> *mut TsRunContext {
    let ctx = Box::new(TsRunContext::new());
    let ctx_ptr = Box::into_raw(ctx);

    // Set up FFI console provider with context pointer.
    // The provider will look up the callback from TsRunContext.console_callback
    // (which is initially None, meaning output is discarded).
    let ctx_ref = unsafe { &mut *ctx_ptr };
    let provider = FfiConsoleProvider::new(ctx_ptr as *mut c_void);
    ctx_ref.interp.set_console(Box::new(provider));

    ctx_ptr
}

/// Free an interpreter context.
///
/// Also frees all associated values that haven't been explicitly freed.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_free(ctx: *mut TsRunContext) {
    if !ctx.is_null() {
        unsafe {
            drop(Box::from_raw(ctx));
        }
    }
}

// ============================================================================
// Execution
// ============================================================================

/// Prepare code for execution.
///
/// `path` is optional - NULL for anonymous scripts, or a path like "/main.ts" for modules.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_prepare(
    ctx: *mut TsRunContext,
    code: *const c_char,
    path: *const c_char,
) -> TsRunResult {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => {
            return TsRunResult {
                ok: false,
                error: c"NULL context".as_ptr(),
            };
        }
    };

    ctx.clear_error();

    let code_str = match unsafe { c_str_to_str(code) } {
        Some(s) => s,
        None => return TsRunResult::err(ctx, "Invalid or NULL code string".to_string()),
    };

    let module_path = unsafe { c_str_to_str(path) }.map(|p| ModulePath::new(p.to_string()));

    match ctx.interp.prepare(code_str, module_path) {
        Ok(_) => TsRunResult::success(),
        Err(e) => TsRunResult::err(ctx, e.to_string()),
    }
}

/// Execute one step.
///
/// The result is written to `out` which must point to valid memory for TsRunStepResult.
/// Caller must call tsrun_step_result_free when done.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_step(out: *mut TsRunStepResult, ctx: *mut TsRunContext) {
    if out.is_null() {
        return;
    }

    if ctx.is_null() {
        unsafe {
            ptr::write(
                out,
                TsRunStepResult {
                    status: TsRunStepStatus::Error,
                    error: c"NULL context".as_ptr(),
                    ..Default::default()
                },
            );
        }
        return;
    }

    let ctx_ref = unsafe { &mut *ctx };
    ctx_ref.clear_error();

    // Set FFI context pointer in interpreter for native callbacks
    ctx_ref.interp.ffi_context = ctx as *mut c_void;

    let result = match ctx_ref.interp.step() {
        Ok(step_result) => convert_step_result(ctx_ref, step_result),
        Err(e) => TsRunStepResult {
            status: TsRunStepStatus::Error,
            error: ctx_ref.set_error(e.to_string()),
            ..Default::default()
        },
    };

    // Clear FFI context after stepping
    ctx_ref.interp.ffi_context = ptr::null_mut();

    // Write result to output pointer
    unsafe {
        ptr::write(out, result);
    }
}

/// Run until completion, needing imports, or suspension.
///
/// Equivalent to calling step() in a loop until non-Continue result.
/// The result is written to `out` which must point to valid memory for TsRunStepResult.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_run(out: *mut TsRunStepResult, ctx: *mut TsRunContext) {
    if out.is_null() {
        return;
    }

    if ctx.is_null() {
        unsafe {
            ptr::write(
                out,
                TsRunStepResult {
                    status: TsRunStepStatus::Error,
                    error: c"NULL context".as_ptr(),
                    ..Default::default()
                },
            );
        }
        return;
    }

    let ctx_ref = unsafe { &mut *ctx };
    ctx_ref.clear_error();

    // Set FFI context pointer in interpreter for native callbacks
    ctx_ref.interp.ffi_context = ctx as *mut c_void;

    let result = loop {
        match ctx_ref.interp.step() {
            Ok(StepResult::Continue) => continue,
            Ok(step_result) => {
                break convert_step_result(ctx_ref, step_result);
            }
            Err(e) => {
                break TsRunStepResult {
                    status: TsRunStepStatus::Error,
                    error: ctx_ref.set_error(e.to_string()),
                    ..Default::default()
                };
            }
        }
    };

    // Clear FFI context after stepping
    ctx_ref.interp.ffi_context = ptr::null_mut();

    // Write result to output pointer
    unsafe {
        ptr::write(out, result);
    }
}

/// Free a step result's internal arrays.
///
/// Does NOT free the value - caller must free that separately with tsrun_value_free.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_step_result_free(result: *mut TsRunStepResult) {
    if result.is_null() {
        return;
    }

    unsafe {
        let result = &mut *result;

        // Free imports array
        // Use Box::from_raw with slice to match how we allocated (via into_boxed_slice)
        if !result.imports.is_null() && result.import_count > 0 {
            let slice_ptr =
                core::ptr::slice_from_raw_parts_mut(result.imports, result.import_count);
            let imports = Box::from_raw(slice_ptr);
            for import in imports.iter() {
                // Free the strings inside each import request
                if !import.specifier.is_null() {
                    drop(CString::from_raw(import.specifier as *mut c_char));
                }
                if !import.resolved_path.is_null() {
                    drop(CString::from_raw(import.resolved_path as *mut c_char));
                }
                if !import.importer.is_null() {
                    drop(CString::from_raw(import.importer as *mut c_char));
                }
            }
            // Box is dropped here, freeing the array memory
        }
        result.imports = ptr::null_mut();
        result.import_count = 0;

        // Free pending orders array (but NOT the payload values - they're owned by context)
        // Use Box::from_raw with slice to match how we allocated (via into_boxed_slice)
        if !result.pending_orders.is_null() && result.pending_count > 0 {
            let slice_ptr =
                core::ptr::slice_from_raw_parts_mut(result.pending_orders, result.pending_count);
            drop(Box::from_raw(slice_ptr));
        }
        result.pending_orders = ptr::null_mut();
        result.pending_count = 0;

        // Free cancelled orders array
        // Use Box::from_raw with slice to match how we allocated (via into_boxed_slice)
        if !result.cancelled_orders.is_null() && result.cancelled_count > 0 {
            let slice_ptr = core::ptr::slice_from_raw_parts_mut(
                result.cancelled_orders,
                result.cancelled_count,
            );
            drop(Box::from_raw(slice_ptr));
        }
        result.cancelled_orders = ptr::null_mut();
        result.cancelled_count = 0;
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn convert_step_result(_ctx: &mut TsRunContext, result: StepResult) -> TsRunStepResult {
    match result {
        StepResult::Continue => TsRunStepResult {
            status: TsRunStepStatus::Continue,
            ..Default::default()
        },

        StepResult::Complete(rv) => TsRunStepResult {
            status: TsRunStepStatus::Complete,
            value: Box::into_raw(TsRunValue::from_runtime_value(rv)),
            ..Default::default()
        },

        StepResult::Done => TsRunStepResult {
            status: TsRunStepStatus::Done,
            ..Default::default()
        },

        StepResult::NeedImports(imports) => {
            // Use into_boxed_slice to ensure capacity == length for correct deallocation
            let c_imports: Vec<TsRunImportRequest> = imports
                .iter()
                .map(|req| TsRunImportRequest {
                    specifier: str_to_c_string(&req.specifier),
                    resolved_path: str_to_c_string(req.resolved_path.as_str()),
                    importer: req
                        .importer
                        .as_ref()
                        .map(|p| str_to_c_string(p.as_str()))
                        .unwrap_or(ptr::null_mut()),
                })
                .collect();

            let import_count = c_imports.len();
            let boxed = c_imports.into_boxed_slice();
            let imports_ptr = Box::into_raw(boxed) as *mut TsRunImportRequest;

            TsRunStepResult {
                status: TsRunStepStatus::NeedImports,
                imports: imports_ptr,
                import_count,
                ..Default::default()
            }
        }

        StepResult::Suspended { pending, cancelled } => {
            // Convert pending orders - use null pointer if empty
            // Use into_boxed_slice to ensure capacity == length for correct deallocation
            let (orders_ptr, pending_count) = if pending.is_empty() {
                (ptr::null_mut(), 0)
            } else {
                let c_orders: Vec<TsRunOrder> = pending
                    .into_iter()
                    .map(|order| TsRunOrder {
                        id: order.id.0,
                        payload: Box::into_raw(TsRunValue::from_runtime_value(order.payload)),
                    })
                    .collect();
                let count = c_orders.len();
                let boxed = c_orders.into_boxed_slice();
                let ptr = Box::into_raw(boxed) as *mut TsRunOrder;
                (ptr, count)
            };

            // Convert cancelled order IDs - use null pointer if empty
            // Use into_boxed_slice to ensure capacity == length for correct deallocation
            let (cancelled_ptr, cancelled_count) = if cancelled.is_empty() {
                (ptr::null_mut(), 0)
            } else {
                let c_cancelled: Vec<u64> = cancelled.into_iter().map(|id| id.0).collect();
                let count = c_cancelled.len();
                let boxed = c_cancelled.into_boxed_slice();
                let ptr = Box::into_raw(boxed) as *mut u64;
                (ptr, count)
            };

            TsRunStepResult {
                status: TsRunStepStatus::Suspended,
                pending_orders: orders_ptr,
                pending_count,
                cancelled_orders: cancelled_ptr,
                cancelled_count,
                ..Default::default()
            }
        }
    }
}
