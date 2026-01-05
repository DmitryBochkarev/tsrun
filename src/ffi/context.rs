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
    c_str_to_str, str_to_c_string, TsRunContext, TsRunImportRequest, TsRunOrder, TsRunResult,
    TsRunStepResult, TsRunStepStatus, TsRunValue,
};

// ============================================================================
// Context Lifecycle
// ============================================================================

/// Create a new interpreter context.
///
/// Returns NULL on failure (unlikely).
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_new() -> *mut TsRunContext {
    Box::into_raw(Box::new(TsRunContext::new()))
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
                error: b"NULL context\0".as_ptr() as *const c_char,
            }
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
/// Returns step result. Caller must call tsrun_step_result_free when done.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_step(ctx: *mut TsRunContext) -> TsRunStepResult {
    if ctx.is_null() {
        return TsRunStepResult {
            status: TsRunStepStatus::Error,
            error: b"NULL context\0".as_ptr() as *const c_char,
            ..Default::default()
        };
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

    result
}

/// Run until completion, needing imports, or suspension.
///
/// Equivalent to calling step() in a loop until non-Continue result.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_run(ctx: *mut TsRunContext) -> TsRunStepResult {
    if ctx.is_null() {
        return TsRunStepResult {
            status: TsRunStepStatus::Error,
            error: b"NULL context\0".as_ptr() as *const c_char,
            ..Default::default()
        };
    }

    let ctx_ref = unsafe { &mut *ctx };
    ctx_ref.clear_error();

    // Set FFI context pointer in interpreter for native callbacks
    ctx_ref.interp.ffi_context = ctx as *mut c_void;

    let result = loop {
        match ctx_ref.interp.step() {
            Ok(StepResult::Continue) => continue,
            Ok(step_result) => break convert_step_result(ctx_ref, step_result),
            Err(e) => {
                break TsRunStepResult {
                    status: TsRunStepStatus::Error,
                    error: ctx_ref.set_error(e.to_string()),
                    ..Default::default()
                }
            }
        }
    };

    // Clear FFI context after stepping
    ctx_ref.interp.ffi_context = ptr::null_mut();

    result
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
        if !result.imports.is_null() && result.import_count > 0 {
            let imports =
                Vec::from_raw_parts(result.imports, result.import_count, result.import_count);
            for import in imports {
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
        }
        result.imports = ptr::null_mut();
        result.import_count = 0;

        // Free pending orders array (but NOT the payload values - they're owned by context)
        if !result.pending_orders.is_null() && result.pending_count > 0 {
            drop(Vec::from_raw_parts(
                result.pending_orders,
                result.pending_count,
                result.pending_count,
            ));
        }
        result.pending_orders = ptr::null_mut();
        result.pending_count = 0;

        // Free cancelled orders array
        if !result.cancelled_orders.is_null() && result.cancelled_count > 0 {
            drop(Vec::from_raw_parts(
                result.cancelled_orders,
                result.cancelled_count,
                result.cancelled_count,
            ));
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
            let mut c_imports: Vec<TsRunImportRequest> = imports
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
            let imports_ptr = c_imports.as_mut_ptr();
            core::mem::forget(c_imports);

            TsRunStepResult {
                status: TsRunStepStatus::NeedImports,
                imports: imports_ptr,
                import_count,
                ..Default::default()
            }
        }

        StepResult::Suspended { pending, cancelled } => {
            // Convert pending orders
            let mut c_orders: Vec<TsRunOrder> = pending
                .into_iter()
                .map(|order| TsRunOrder {
                    id: order.id.0,
                    payload: Box::into_raw(TsRunValue::from_runtime_value(order.payload)),
                })
                .collect();

            let pending_count = c_orders.len();
            let orders_ptr = c_orders.as_mut_ptr();
            core::mem::forget(c_orders);

            // Convert cancelled order IDs
            let mut c_cancelled: Vec<u64> = cancelled.into_iter().map(|id| id.0).collect();
            let cancelled_count = c_cancelled.len();
            let cancelled_ptr = c_cancelled.as_mut_ptr();
            core::mem::forget(c_cancelled);

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
