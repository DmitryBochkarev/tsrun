//! Native function callback system.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::ffi::{CStr, c_char, c_void};
use core::ptr;

use crate::error::JsError;
use crate::value::{CheapClone, Guarded, JsValue};

use super::{
    NativeCallbackWrapper, TsRunContext, TsRunNativeFn, TsRunResult, TsRunValue, TsRunValueResult,
};

// ============================================================================
// Native Function Creation
// ============================================================================

/// Create a native function that can be called from JS.
///
/// The callback will be invoked with:
/// - ctx: The context pointer
/// - this_arg: The 'this' value for the call
/// - args: Array of argument values
/// - argc: Number of arguments
/// - userdata: The userdata pointer passed to this function
/// - error_out: Pointer to set error message on failure
///
/// Return NULL to return undefined. Set *error_out to a static string on error.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_native_function(
    ctx: *mut TsRunContext,
    name: *const c_char,
    func: TsRunNativeFn,
    arity: usize,
    userdata: *mut c_void,
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

    let name_str = if name.is_null() {
        "anonymous"
    } else {
        match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(_) => {
                return TsRunValueResult::err(ctx, "Invalid function name encoding".to_string());
            }
        }
    };

    // Generate a unique FFI ID for this callback
    let ffi_id = ctx.next_ffi_id;
    ctx.next_ffi_id += 1;

    // Create wrapper for the C callback
    let wrapper = NativeCallbackWrapper {
        callback: func,
        userdata,
    };

    // Store the wrapper keyed by the FFI ID
    ctx.native_callbacks.insert(ffi_id, wrapper);

    // Create the native function in the interpreter
    let guard = ctx.interp.heap.create_guard();
    let fn_obj = ctx
        .interp
        .create_native_fn(&guard, name_str, native_callback_trampoline, arity);

    // Set the ffi_id on the NativeFunction so the interpreter can pass it back
    {
        use crate::value::{ExoticObject, JsFunction};
        let mut fn_ref = fn_obj.borrow_mut();
        if let ExoticObject::Function(JsFunction::Native(ref mut native)) = fn_ref.exotic {
            native.ffi_id = ffi_id;
        }
    }

    TsRunValueResult::ok(Box::new(TsRunValue {
        inner: crate::RuntimeValue::with_guard(JsValue::Object(fn_obj), guard),
    }))
}

/// Trampoline function that looks up the C callback and invokes it.
///
/// This is the Rust NativeFn that gets registered with the interpreter.
/// It uses interp.current_ffi_id (set by bytecode_vm before the call) to find the callback.
fn native_callback_trampoline(
    interp: &mut crate::Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // Get the FFI callback ID from the interpreter (set by bytecode_vm before calling)
    let ffi_id = interp.current_ffi_id;
    if ffi_id == 0 {
        return Err(JsError::internal_error(
            "Native callback called without FFI ID",
        ));
    }

    // Get the TsRunContext from interpreter (set by tsrun_step/tsrun_run)
    let ctx_ptr = interp.ffi_context as *mut TsRunContext;
    if ctx_ptr.is_null() {
        return Err(JsError::internal_error(
            "Native callback called without context",
        ));
    }

    let ctx = unsafe { &mut *ctx_ptr };

    // Look up the callback wrapper by FFI ID
    let wrapper = ctx
        .native_callbacks
        .get(&ffi_id)
        .ok_or_else(|| JsError::internal_error("Native callback not found"))?;

    // Create TsRunValue handles for this and args
    let this_handle = Box::into_raw(TsRunValue::from_js_value(&mut ctx.interp, this.clone()));

    let mut arg_handles: Vec<*mut TsRunValue> = args
        .iter()
        .map(|arg| Box::into_raw(TsRunValue::from_js_value(&mut ctx.interp, arg.clone())))
        .collect();

    let args_ptr = if arg_handles.is_empty() {
        ptr::null_mut()
    } else {
        arg_handles.as_mut_ptr()
    };

    // Call the C callback
    let mut error_out: *const c_char = ptr::null();
    let result = (wrapper.callback)(
        ctx_ptr,
        this_handle,
        args_ptr,
        args.len(),
        wrapper.userdata,
        &mut error_out,
    );

    // Clean up argument handles
    unsafe {
        drop(Box::from_raw(this_handle));
        for handle in arg_handles {
            drop(Box::from_raw(handle));
        }
    }

    // Process result
    if !error_out.is_null() {
        let error_str = unsafe { CStr::from_ptr(error_out) }
            .to_str()
            .unwrap_or("Unknown error");
        return Err(JsError::type_error(error_str));
    }

    if result.is_null() {
        Ok(Guarded::unguarded(JsValue::Undefined))
    } else {
        let result_val = unsafe { Box::from_raw(result) };
        // Create a guard for the result if it's an object
        if let JsValue::Object(obj) = result_val.inner.value() {
            let guard = interp.heap.create_guard();
            guard.guard(obj.cheap_clone());
            Ok(Guarded::with_guard(result_val.inner.value().clone(), guard))
        } else {
            Ok(Guarded::unguarded(result_val.inner.value().clone()))
        }
    }
}

// ============================================================================
// Internal Module Builder (Placeholder)
// ============================================================================

/// Opaque internal module builder.
pub struct TsRunInternalModule {
    #[allow(dead_code)]
    specifier: String,
    #[allow(dead_code)]
    exports: Vec<(String, InternalExportKind)>,
}

#[allow(dead_code)]
enum InternalExportKind {
    Function {
        func: TsRunNativeFn,
        arity: usize,
        userdata: *mut c_void,
    },
    Value(*mut TsRunValue),
}

/// Create an internal module builder.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_internal_module_new(specifier: *const c_char) -> *mut TsRunInternalModule {
    let spec_str = match unsafe { super::c_str_to_str(specifier) } {
        Some(s) => s.to_string(),
        None => return ptr::null_mut(),
    };

    Box::into_raw(Box::new(TsRunInternalModule {
        specifier: spec_str,
        exports: Vec::new(),
    }))
}

/// Add a native function export to an internal module.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_internal_module_add_function(
    module: *mut TsRunInternalModule,
    name: *const c_char,
    func: TsRunNativeFn,
    arity: usize,
    userdata: *mut c_void,
) {
    let module = match unsafe { module.as_mut() } {
        Some(m) => m,
        None => return,
    };

    let name_str = match unsafe { super::c_str_to_str(name) } {
        Some(s) => s.to_string(),
        None => return,
    };

    module.exports.push((
        name_str,
        InternalExportKind::Function {
            func,
            arity,
            userdata,
        },
    ));
}

/// Add a value export to an internal module.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_internal_module_add_value(
    module: *mut TsRunInternalModule,
    name: *const c_char,
    value: *mut TsRunValue,
) {
    let module = match unsafe { module.as_mut() } {
        Some(m) => m,
        None => return,
    };

    let name_str = match unsafe { super::c_str_to_str(name) } {
        Some(s) => s.to_string(),
        None => return,
    };

    module
        .exports
        .push((name_str, InternalExportKind::Value(value)));
}

/// Register an internal module with a context.
///
/// Takes ownership of the module.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_register_internal_module(
    ctx: *mut TsRunContext,
    module: *mut TsRunInternalModule,
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

    if module.is_null() {
        return TsRunResult::err(ctx, "NULL module".to_string());
    }

    let _module = unsafe { Box::from_raw(module) };

    // TODO: Actually register the module with the interpreter.
    // This requires converting the C callbacks to Rust InternalFn,
    // which is complex because InternalFn is a function pointer, not a closure.
    //
    // For now, this is a placeholder that frees the module.

    TsRunResult::success()
}
