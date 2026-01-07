//! Module system functions.

extern crate alloc;

use alloc::string::ToString;
use alloc::vec::Vec;
use core::ffi::c_char;
use core::ptr;

use crate::ModulePath;

use super::{TsRunContext, TsRunResult, TsRunValueResult, c_str_to_str, str_to_c_string};

// ============================================================================
// Module Loading
// ============================================================================

/// Provide module source code in response to TSRUN_STEP_NEED_IMPORTS.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_provide_module(
    ctx: *mut TsRunContext,
    path: *const c_char,
    code: *const c_char,
) -> TsRunResult {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => {
            return TsRunResult {
                ok: false,
                error: b"NULL context\0".as_ptr() as *const c_char,
            };
        }
    };

    let path_str = match unsafe { c_str_to_str(path) } {
        Some(s) => s,
        None => return TsRunResult::err(ctx, "Invalid or NULL path".to_string()),
    };

    let code_str = match unsafe { c_str_to_str(code) } {
        Some(s) => s,
        None => return TsRunResult::err(ctx, "Invalid or NULL code".to_string()),
    };

    let module_path = ModulePath::new(path_str.to_string());
    match ctx.interp.provide_module(module_path, code_str) {
        Ok(()) => TsRunResult::success(),
        Err(e) => TsRunResult::err(ctx, e.to_string()),
    }
}

// ============================================================================
// Module Exports
// ============================================================================

/// Get an export from the main module (after execution completes).
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_get_export(
    ctx: *mut TsRunContext,
    name: *const c_char,
) -> TsRunValueResult {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => {
            return TsRunValueResult {
                value: ptr::null_mut(),
                error: b"NULL context\0".as_ptr() as *const c_char,
            };
        }
    };

    let name_str = match unsafe { c_str_to_str(name) } {
        Some(s) => s,
        None => return TsRunValueResult::err(ctx, "Invalid or NULL export name".to_string()),
    };

    match ctx.interp.get_export(name_str) {
        Some(value) => {
            TsRunValueResult::ok(super::TsRunValue::from_js_value(&mut ctx.interp, value))
        }
        None => TsRunValueResult::ok(super::TsRunValue::from_js_value(
            &mut ctx.interp,
            crate::JsValue::Undefined,
        )),
    }
}

/// Get all export names from the main module.
///
/// Caller must free the returned array with tsrun_free_strings.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_get_export_names(
    ctx: *mut TsRunContext,
    count_out: *mut usize,
) -> *mut *mut c_char {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => {
            if !count_out.is_null() {
                unsafe { *count_out = 0 };
            }
            return ptr::null_mut();
        }
    };

    let names = ctx.interp.get_export_names();
    let c_names: Vec<*mut c_char> = names.iter().map(|s| str_to_c_string(s)).collect();

    let count = c_names.len();
    if !count_out.is_null() {
        unsafe { *count_out = count };
    }

    if count == 0 {
        return ptr::null_mut();
    }

    let mut boxed = c_names.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    core::mem::forget(boxed);
    ptr
}
