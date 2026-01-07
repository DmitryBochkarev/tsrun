//! Value inspection, creation, and manipulation functions.

extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::c_char;
use core::ptr;

use crate::value::{CheapClone, ExoticObject, PropertyKey};
use crate::{JsString, JsValue};

use super::{
    TsRunContext, TsRunResult, TsRunType, TsRunValue, TsRunValueResult, c_str_to_str,
    str_to_c_string,
};

// ============================================================================
// Type Inspection
// ============================================================================

/// Get the type of a value.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_typeof(val: *const TsRunValue) -> TsRunType {
    let val = match unsafe { val.as_ref() } {
        Some(v) => v,
        None => return TsRunType::Undefined,
    };

    match val.value() {
        JsValue::Undefined => TsRunType::Undefined,
        JsValue::Null => TsRunType::Null,
        JsValue::Boolean(_) => TsRunType::Boolean,
        JsValue::Number(_) => TsRunType::Number,
        JsValue::String(_) => TsRunType::String,
        JsValue::Object(_) => TsRunType::Object,
        JsValue::Symbol(_) => TsRunType::Symbol,
    }
}

/// Check if value is undefined.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_is_undefined(val: *const TsRunValue) -> bool {
    unsafe { val.as_ref() }
        .map(|v| matches!(v.value(), JsValue::Undefined))
        .unwrap_or(true)
}

/// Check if value is null.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_is_null(val: *const TsRunValue) -> bool {
    unsafe { val.as_ref() }
        .map(|v| matches!(v.value(), JsValue::Null))
        .unwrap_or(false)
}

/// Check if value is null or undefined.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_is_nullish(val: *const TsRunValue) -> bool {
    unsafe { val.as_ref() }
        .map(|v| matches!(v.value(), JsValue::Undefined | JsValue::Null))
        .unwrap_or(true)
}

/// Check if value is a boolean.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_is_boolean(val: *const TsRunValue) -> bool {
    unsafe { val.as_ref() }
        .map(|v| matches!(v.value(), JsValue::Boolean(_)))
        .unwrap_or(false)
}

/// Check if value is a number.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_is_number(val: *const TsRunValue) -> bool {
    unsafe { val.as_ref() }
        .map(|v| matches!(v.value(), JsValue::Number(_)))
        .unwrap_or(false)
}

/// Check if value is a string.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_is_string(val: *const TsRunValue) -> bool {
    unsafe { val.as_ref() }
        .map(|v| matches!(v.value(), JsValue::String(_)))
        .unwrap_or(false)
}

/// Check if value is an object.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_is_object(val: *const TsRunValue) -> bool {
    unsafe { val.as_ref() }
        .map(|v| matches!(v.value(), JsValue::Object(_)))
        .unwrap_or(false)
}

/// Check if value is an array.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_is_array(val: *const TsRunValue) -> bool {
    unsafe { val.as_ref() }
        .map(|v| {
            if let JsValue::Object(obj) = v.value() {
                matches!(obj.borrow().exotic, ExoticObject::Array { .. })
            } else {
                false
            }
        })
        .unwrap_or(false)
}

/// Check if value is a function.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_is_function(val: *const TsRunValue) -> bool {
    unsafe { val.as_ref() }
        .map(|v| {
            if let JsValue::Object(obj) = v.value() {
                matches!(obj.borrow().exotic, ExoticObject::Function(_))
            } else {
                false
            }
        })
        .unwrap_or(false)
}

// ============================================================================
// Value Extraction
// ============================================================================

/// Get boolean value. Returns false if not a boolean.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_get_bool(val: *const TsRunValue) -> bool {
    unsafe { val.as_ref() }
        .and_then(|v| {
            if let JsValue::Boolean(b) = v.value() {
                Some(*b)
            } else {
                None
            }
        })
        .unwrap_or(false)
}

/// Get number value. Returns NaN if not a number.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_get_number(val: *const TsRunValue) -> f64 {
    unsafe { val.as_ref() }
        .and_then(|v| {
            if let JsValue::Number(n) = v.value() {
                Some(*n)
            } else {
                None
            }
        })
        .unwrap_or(f64::NAN)
}

/// Get string value. Returns NULL if not a string.
///
/// The returned pointer is valid until the value is freed.
/// Note: The returned string is allocated and must be freed with tsrun_free_string.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_get_string(val: *const TsRunValue) -> *const c_char {
    unsafe { val.as_ref() }
        .and_then(|v| {
            if let JsValue::String(s) = v.value() {
                // Allocate a CString for C compatibility
                Some(str_to_c_string(s.as_str()) as *const c_char)
            } else {
                None
            }
        })
        .unwrap_or(ptr::null())
}

/// Get string length in bytes. Returns 0 if not a string.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_get_string_len(val: *const TsRunValue) -> usize {
    unsafe { val.as_ref() }
        .and_then(|v| {
            if let JsValue::String(s) = v.value() {
                Some(s.len())
            } else {
                None
            }
        })
        .unwrap_or(0)
}

// ============================================================================
// Value Creation
// ============================================================================

/// Create an undefined value.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_undefined(ctx: *mut TsRunContext) -> *mut TsRunValue {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => return ptr::null_mut(),
    };
    Box::into_raw(TsRunValue::from_js_value(
        &mut ctx.interp,
        JsValue::Undefined,
    ))
}

/// Create a null value.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_null(ctx: *mut TsRunContext) -> *mut TsRunValue {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => return ptr::null_mut(),
    };
    Box::into_raw(TsRunValue::from_js_value(&mut ctx.interp, JsValue::Null))
}

/// Create a boolean value.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_boolean(ctx: *mut TsRunContext, b: bool) -> *mut TsRunValue {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => return ptr::null_mut(),
    };
    Box::into_raw(TsRunValue::from_js_value(
        &mut ctx.interp,
        JsValue::Boolean(b),
    ))
}

/// Create a number value.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_number(ctx: *mut TsRunContext, n: f64) -> *mut TsRunValue {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => return ptr::null_mut(),
    };
    Box::into_raw(TsRunValue::from_js_value(
        &mut ctx.interp,
        JsValue::Number(n),
    ))
}

/// Create a string value from a null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_string(ctx: *mut TsRunContext, s: *const c_char) -> *mut TsRunValue {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => return ptr::null_mut(),
    };

    let s_str = match unsafe { c_str_to_str(s) } {
        Some(s) => s,
        None => return ptr::null_mut(),
    };

    Box::into_raw(TsRunValue::from_js_value(
        &mut ctx.interp,
        JsValue::String(JsString::from(s_str)),
    ))
}

/// Create a string value with explicit length.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_string_len(
    ctx: *mut TsRunContext,
    s: *const c_char,
    len: usize,
) -> *mut TsRunValue {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => return ptr::null_mut(),
    };

    if s.is_null() {
        return ptr::null_mut();
    }

    let bytes = unsafe { core::slice::from_raw_parts(s as *const u8, len) };
    let s_str = match core::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    Box::into_raw(TsRunValue::from_js_value(
        &mut ctx.interp,
        JsValue::String(JsString::from(s_str)),
    ))
}

// ============================================================================
// Value Memory Management
// ============================================================================

/// Free a value.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_value_free(val: *mut TsRunValue) {
    if !val.is_null() {
        unsafe {
            drop(Box::from_raw(val));
        }
    }
}

/// Duplicate a value handle.
///
/// Both handles must be freed separately.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_value_dup(
    ctx: *mut TsRunContext,
    val: *const TsRunValue,
) -> *mut TsRunValue {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => return ptr::null_mut(),
    };

    let val = match unsafe { val.as_ref() } {
        Some(v) => v,
        None => return ptr::null_mut(),
    };

    // Clone the value and create a new guard if needed
    Box::into_raw(TsRunValue::from_js_value(
        &mut ctx.interp,
        val.value().clone(),
    ))
}

// ============================================================================
// Object/Array Operations
// ============================================================================

/// Get a property from an object.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_get(
    ctx: *mut TsRunContext,
    obj: *mut TsRunValue,
    key: *const c_char,
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

    let obj_val = match unsafe { obj.as_ref() } {
        Some(v) => v,
        None => return TsRunValueResult::err(ctx, "NULL object".to_string()),
    };

    let key_str = match unsafe { c_str_to_str(key) } {
        Some(s) => s,
        None => return TsRunValueResult::err(ctx, "Invalid or NULL key".to_string()),
    };

    let JsValue::Object(obj_ref) = obj_val.value() else {
        return TsRunValueResult::err(ctx, "Value is not an object".to_string());
    };

    let prop_key = PropertyKey::String(JsString::from(key_str));
    let value = obj_ref
        .borrow()
        .get_property(&prop_key)
        .unwrap_or(JsValue::Undefined);

    TsRunValueResult::ok(TsRunValue::from_js_value(&mut ctx.interp, value))
}

/// Set a property on an object.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_set(
    ctx: *mut TsRunContext,
    obj: *mut TsRunValue,
    key: *const c_char,
    val: *mut TsRunValue,
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

    let obj_val = match unsafe { obj.as_ref() } {
        Some(v) => v,
        None => return TsRunResult::err(ctx, "NULL object".to_string()),
    };

    let key_str = match unsafe { c_str_to_str(key) } {
        Some(s) => s,
        None => return TsRunResult::err(ctx, "Invalid or NULL key".to_string()),
    };

    let val_ref = match unsafe { val.as_ref() } {
        Some(v) => v,
        None => return TsRunResult::err(ctx, "NULL value".to_string()),
    };

    let JsValue::Object(obj_ref) = obj_val.value() else {
        return TsRunResult::err(ctx, "Value is not an object".to_string());
    };

    let prop_key = PropertyKey::String(JsString::from(key_str));
    obj_ref
        .borrow_mut()
        .set_property(prop_key, val_ref.value().clone());

    TsRunResult::success()
}

/// Check if an object has a property.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_has(
    ctx: *mut TsRunContext,
    obj: *mut TsRunValue,
    key: *const c_char,
) -> bool {
    let _ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => return false,
    };

    let obj_val = match unsafe { obj.as_ref() } {
        Some(v) => v,
        None => return false,
    };

    let key_str = match unsafe { c_str_to_str(key) } {
        Some(s) => s,
        None => return false,
    };

    let JsValue::Object(obj_ref) = obj_val.value() else {
        return false;
    };

    let prop_key = PropertyKey::String(JsString::from(key_str));
    obj_ref.borrow().get_property(&prop_key).is_some()
}

/// Delete a property from an object.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_delete(
    ctx: *mut TsRunContext,
    obj: *mut TsRunValue,
    key: *const c_char,
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

    let obj_val = match unsafe { obj.as_ref() } {
        Some(v) => v,
        None => return TsRunResult::err(ctx, "NULL object".to_string()),
    };

    let key_str = match unsafe { c_str_to_str(key) } {
        Some(s) => s,
        None => return TsRunResult::err(ctx, "Invalid or NULL key".to_string()),
    };

    let JsValue::Object(obj_ref) = obj_val.value() else {
        return TsRunResult::err(ctx, "Value is not an object".to_string());
    };

    let prop_key = PropertyKey::String(JsString::from(key_str));
    obj_ref.borrow_mut().properties.remove(&prop_key);

    TsRunResult::success()
}

/// Get all property keys of an object.
///
/// Caller must free the returned array with tsrun_free_strings.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_keys(
    ctx: *mut TsRunContext,
    obj: *mut TsRunValue,
    count_out: *mut usize,
) -> *mut *mut c_char {
    let _ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => {
            if !count_out.is_null() {
                unsafe { *count_out = 0 };
            }
            return ptr::null_mut();
        }
    };

    let obj_val = match unsafe { obj.as_ref() } {
        Some(v) => v,
        None => {
            if !count_out.is_null() {
                unsafe { *count_out = 0 };
            }
            return ptr::null_mut();
        }
    };

    let JsValue::Object(obj_ref) = obj_val.value() else {
        if !count_out.is_null() {
            unsafe { *count_out = 0 };
        }
        return ptr::null_mut();
    };

    let borrowed = obj_ref.borrow();
    let keys: Vec<*mut c_char> = borrowed
        .properties
        .keys()
        .filter_map(|k| match k {
            PropertyKey::String(s) => Some(str_to_c_string(s.as_str())),
            PropertyKey::Index(i) => Some(str_to_c_string(&i.to_string())),
            PropertyKey::Symbol(_) => None,
        })
        .collect();

    let count = keys.len();
    if !count_out.is_null() {
        unsafe { *count_out = count };
    }

    if count == 0 {
        return ptr::null_mut();
    }

    let mut boxed = keys.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    core::mem::forget(boxed);
    ptr
}

// ============================================================================
// Array Operations
// ============================================================================

/// Get the length of an array.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_array_len(arr: *const TsRunValue) -> usize {
    unsafe { arr.as_ref() }
        .and_then(|v| {
            if let JsValue::Object(obj) = v.value() {
                let borrowed = obj.borrow();
                if let ExoticObject::Array { elements } = &borrowed.exotic {
                    return Some(elements.len());
                }
            }
            None
        })
        .unwrap_or(0)
}

/// Get an element from an array by index.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_array_get(
    ctx: *mut TsRunContext,
    arr: *mut TsRunValue,
    index: usize,
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

    let arr_val = match unsafe { arr.as_ref() } {
        Some(v) => v,
        None => return TsRunValueResult::err(ctx, "NULL array".to_string()),
    };

    let JsValue::Object(obj_ref) = arr_val.value() else {
        return TsRunValueResult::err(ctx, "Value is not an object".to_string());
    };

    let borrowed = obj_ref.borrow();
    let ExoticObject::Array { elements } = &borrowed.exotic else {
        return TsRunValueResult::err(ctx, "Value is not an array".to_string());
    };

    let value = elements.get(index).cloned().unwrap_or(JsValue::Undefined);
    drop(borrowed);

    TsRunValueResult::ok(TsRunValue::from_js_value(&mut ctx.interp, value))
}

/// Set an element in an array by index.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_array_set(
    ctx: *mut TsRunContext,
    arr: *mut TsRunValue,
    index: usize,
    val: *mut TsRunValue,
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

    let arr_val = match unsafe { arr.as_ref() } {
        Some(v) => v,
        None => return TsRunResult::err(ctx, "NULL array".to_string()),
    };

    let val_ref = match unsafe { val.as_ref() } {
        Some(v) => v,
        None => return TsRunResult::err(ctx, "NULL value".to_string()),
    };

    let JsValue::Object(obj_ref) = arr_val.value() else {
        return TsRunResult::err(ctx, "Value is not an object".to_string());
    };

    let mut borrowed = obj_ref.borrow_mut();
    let ExoticObject::Array { elements } = &mut borrowed.exotic else {
        return TsRunResult::err(ctx, "Value is not an array".to_string());
    };

    // Extend array if needed
    while elements.len() <= index {
        elements.push(JsValue::Undefined);
    }

    if let Some(elem) = elements.get_mut(index) {
        *elem = val_ref.value().clone();
    }

    // Update length property
    let new_len = elements.len();
    drop(borrowed);
    obj_ref.borrow_mut().set_property(
        PropertyKey::String(JsString::from("length")),
        JsValue::Number(new_len as f64),
    );

    TsRunResult::success()
}

/// Push a value onto an array.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_array_push(
    ctx: *mut TsRunContext,
    arr: *mut TsRunValue,
    val: *mut TsRunValue,
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

    let arr_val = match unsafe { arr.as_ref() } {
        Some(v) => v,
        None => return TsRunResult::err(ctx, "NULL array".to_string()),
    };

    let val_ref = match unsafe { val.as_ref() } {
        Some(v) => v,
        None => return TsRunResult::err(ctx, "NULL value".to_string()),
    };

    let JsValue::Object(obj_ref) = arr_val.value() else {
        return TsRunResult::err(ctx, "Value is not an object".to_string());
    };

    let mut borrowed = obj_ref.borrow_mut();
    let ExoticObject::Array { elements } = &mut borrowed.exotic else {
        return TsRunResult::err(ctx, "Value is not an array".to_string());
    };

    elements.push(val_ref.value().clone());
    let new_len = elements.len();
    drop(borrowed);

    obj_ref.borrow_mut().set_property(
        PropertyKey::String(JsString::from("length")),
        JsValue::Number(new_len as f64),
    );

    TsRunResult::success()
}

// ============================================================================
// JSON Operations
// ============================================================================

/// Parse a JSON string into a value.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_json_parse(
    ctx: *mut TsRunContext,
    json: *const c_char,
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

    let json_str = match unsafe { c_str_to_str(json) } {
        Some(s) => s,
        None => return TsRunValueResult::err(ctx, "Invalid or NULL JSON string".to_string()),
    };

    let json_value: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => return TsRunValueResult::err(ctx, format!("JSON parse error: {}", e)),
    };

    let guard = ctx.interp.heap.create_guard();
    match crate::json_to_js_value_with_guard(&mut ctx.interp, &json_value, &guard) {
        Ok(value) => {
            // Guard the value if it's an object
            if let JsValue::Object(ref obj) = value {
                guard.guard(obj.cheap_clone());
            }
            TsRunValueResult::ok(Box::new(TsRunValue {
                inner: crate::RuntimeValue::with_guard(value, guard),
            }))
        }
        Err(e) => TsRunValueResult::err(ctx, e.to_string()),
    }
}

/// Serialize a value to a JSON string.
///
/// Caller must free the returned string with tsrun_free_string.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_json_stringify(
    ctx: *mut TsRunContext,
    val: *mut TsRunValue,
) -> *mut c_char {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => return ptr::null_mut(),
    };

    let val_ref = match unsafe { val.as_ref() } {
        Some(v) => v,
        None => return ptr::null_mut(),
    };

    match crate::js_value_to_json(val_ref.value()) {
        Ok(json_value) => match serde_json::to_string(&json_value) {
            Ok(s) => str_to_c_string(&s),
            Err(_) => {
                ctx.set_error("JSON stringify error".to_string());
                ptr::null_mut()
            }
        },
        Err(e) => {
            ctx.set_error(e.to_string());
            ptr::null_mut()
        }
    }
}

// ============================================================================
// Object/Array Creation
// ============================================================================

/// Create an empty object.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_object_new(ctx: *mut TsRunContext) -> TsRunValueResult {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => {
            return TsRunValueResult {
                value: ptr::null_mut(),
                error: b"NULL context\0".as_ptr() as *const c_char,
            };
        }
    };

    let guard = ctx.interp.heap.create_guard();
    let obj = ctx.interp.create_object(&guard);
    TsRunValueResult::ok(Box::new(TsRunValue {
        inner: crate::RuntimeValue::with_guard(JsValue::Object(obj), guard),
    }))
}

/// Create an empty array.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_array_new(ctx: *mut TsRunContext) -> TsRunValueResult {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => {
            return TsRunValueResult {
                value: ptr::null_mut(),
                error: b"NULL context\0".as_ptr() as *const c_char,
            };
        }
    };

    let guard = ctx.interp.heap.create_guard();
    let arr = ctx.interp.create_array_from(&guard, vec![]);
    TsRunValueResult::ok(Box::new(TsRunValue {
        inner: crate::RuntimeValue::with_guard(JsValue::Object(arr), guard),
    }))
}

// ============================================================================
// Function Calls
// ============================================================================

/// Call a function with arguments.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_call(
    ctx: *mut TsRunContext,
    func: *mut TsRunValue,
    this_arg: *mut TsRunValue,
    args: *mut *mut TsRunValue,
    argc: usize,
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

    let func_val = match unsafe { func.as_ref() } {
        Some(v) => v.value().clone(),
        None => return TsRunValueResult::err(ctx, "NULL function".to_string()),
    };

    let this_val = unsafe { this_arg.as_ref() }
        .map(|v| v.value().clone())
        .unwrap_or(JsValue::Undefined);

    // Convert args array
    let args_vec: Vec<JsValue> = if args.is_null() || argc == 0 {
        Vec::new()
    } else {
        (0..argc)
            .filter_map(|i| unsafe {
                let arg_ptr = *args.add(i);
                arg_ptr.as_ref().map(|v| v.value().clone())
            })
            .collect()
    };

    match ctx.interp.call_function(func_val, this_val, &args_vec) {
        Ok(guarded) => TsRunValueResult::ok(TsRunValue::from_runtime_value(
            crate::RuntimeValue::from_guarded(guarded),
        )),
        Err(e) => TsRunValueResult::err(ctx, e.to_string()),
    }
}

/// Call a method on an object.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_call_method(
    ctx: *mut TsRunContext,
    obj: *mut TsRunValue,
    method: *const c_char,
    args: *mut *mut TsRunValue,
    argc: usize,
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

    let obj_val = match unsafe { obj.as_ref() } {
        Some(v) => v,
        None => return TsRunValueResult::err(ctx, "NULL object".to_string()),
    };

    let method_str = match unsafe { c_str_to_str(method) } {
        Some(s) => s,
        None => return TsRunValueResult::err(ctx, "Invalid or NULL method name".to_string()),
    };

    let JsValue::Object(obj_ref) = obj_val.value() else {
        return TsRunValueResult::err(ctx, "Value is not an object".to_string());
    };

    // Look up method
    let prop_key = PropertyKey::String(JsString::from(method_str));
    let method_val = obj_ref
        .borrow()
        .get_property(&prop_key)
        .ok_or_else(|| format!("{} is not a function", method_str));

    let method_val = match method_val {
        Ok(v) => v,
        Err(e) => return TsRunValueResult::err(ctx, e),
    };

    if !method_val.is_callable() {
        return TsRunValueResult::err(ctx, format!("{} is not a function", method_str));
    }

    // Convert args array
    let args_vec: Vec<JsValue> = if args.is_null() || argc == 0 {
        Vec::new()
    } else {
        (0..argc)
            .filter_map(|i| unsafe {
                let arg_ptr = *args.add(i);
                arg_ptr.as_ref().map(|v| v.value().clone())
            })
            .collect()
    };

    let this_val = JsValue::Object(obj_ref.cheap_clone());
    match ctx.interp.call_function(method_val, this_val, &args_vec) {
        Ok(guarded) => TsRunValueResult::ok(TsRunValue::from_runtime_value(
            crate::RuntimeValue::from_guarded(guarded),
        )),
        Err(e) => TsRunValueResult::err(ctx, e.to_string()),
    }
}

// ============================================================================
// Global Access
// ============================================================================

/// Get a global variable.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_get_global(
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
        None => return TsRunValueResult::err(ctx, "Invalid or NULL name".to_string()),
    };

    let prop_key = PropertyKey::String(JsString::from(name_str));
    let value = ctx
        .interp
        .global
        .borrow()
        .get_property(&prop_key)
        .unwrap_or(JsValue::Undefined);

    TsRunValueResult::ok(TsRunValue::from_js_value(&mut ctx.interp, value))
}

/// Set a global variable.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_set_global(
    ctx: *mut TsRunContext,
    name: *const c_char,
    val: *mut TsRunValue,
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

    let name_str = match unsafe { c_str_to_str(name) } {
        Some(s) => s,
        None => return TsRunResult::err(ctx, "Invalid or NULL name".to_string()),
    };

    let val_ref = match unsafe { val.as_ref() } {
        Some(v) => v,
        None => return TsRunResult::err(ctx, "NULL value".to_string()),
    };

    let prop_key = PropertyKey::String(JsString::from(name_str));
    ctx.interp
        .global
        .borrow_mut()
        .set_property(prop_key, val_ref.value().clone());

    TsRunResult::success()
}

// ============================================================================
// GC Statistics
// ============================================================================

/// Get garbage collector statistics.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_gc_stats(ctx: *mut TsRunContext) -> super::TsRunGcStats {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => {
            return super::TsRunGcStats {
                total_objects: 0,
                pooled_objects: 0,
                live_objects: 0,
            };
        }
    };

    let stats = ctx.interp.heap.stats();
    super::TsRunGcStats {
        total_objects: stats.total_objects,
        pooled_objects: stats.pooled_objects,
        live_objects: stats.live_objects,
    }
}
