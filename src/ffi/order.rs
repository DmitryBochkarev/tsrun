//! Order/async system functions.

use std::ffi::c_char;
use std::ptr;

use crate::value::{CheapClone, PropertyKey};
use crate::{JsError, JsString, JsValue, OrderId, OrderResponse, RuntimeValue};

use super::{c_str_to_str, TsRunContext, TsRunOrderResponse, TsRunResult, TsRunValue, TsRunValueResult};

// ============================================================================
// Order Fulfillment
// ============================================================================

/// Fulfill one or more orders.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_fulfill_orders(
    ctx: *mut TsRunContext,
    responses: *const TsRunOrderResponse,
    count: usize,
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

    if responses.is_null() && count > 0 {
        return TsRunResult::err(ctx, "NULL responses array".to_string());
    }

    // Convert C responses to Rust
    let rust_responses: Vec<OrderResponse> = (0..count)
        .map(|i| unsafe {
            let resp = &*responses.add(i);
            let result = if resp.error.is_null() {
                // Success case
                if let Some(val) = resp.value.as_ref() {
                    Ok(RuntimeValue::unguarded(val.value().clone()))
                } else {
                    Ok(RuntimeValue::unguarded(JsValue::Undefined))
                }
            } else {
                // Error case
                let error_str = c_str_to_str(resp.error).unwrap_or("Unknown error");
                Err(JsError::type_error(error_str))
            };

            OrderResponse {
                id: OrderId(resp.id),
                result,
            }
        })
        .collect();

    ctx.interp.fulfill_orders(rust_responses);
    TsRunResult::success()
}

// ============================================================================
// Pending Order Creation
// ============================================================================

/// Create a pending order that will cause the interpreter to suspend.
///
/// Use this in native C callbacks when you need to perform async operations.
/// When a native function returns a pending order value, the interpreter suspends
/// and reports the order to the host via TSRUN_STEP_SUSPENDED.
///
/// The payload is the value that will be reported to the host (accessible via order.payload).
/// The returned order_id_out is the ID the host will use to fulfill the order.
///
/// Returns a TsRunValue that MUST be returned from the native callback.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_create_pending_order(
    ctx: *mut TsRunContext,
    payload: *mut TsRunValue,
    order_id_out: *mut u64,
) -> TsRunValueResult {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => {
            return TsRunValueResult {
                value: ptr::null_mut(),
                error: b"NULL context\0".as_ptr() as *const c_char,
            }
        }
    };

    // Get payload value (or undefined if NULL)
    let payload_value = match unsafe { payload.as_ref() } {
        Some(v) => v.value().clone(),
        None => JsValue::Undefined,
    };

    // Generate unique order ID
    let id = crate::OrderId(ctx.interp.next_order_id);
    ctx.interp.next_order_id += 1;

    // Write order ID to output parameter
    if !order_id_out.is_null() {
        unsafe { *order_id_out = id.0 };
    }

    // Create payload RuntimeValue with guard if it's an object
    let payload_rv = if let JsValue::Object(ref obj) = payload_value {
        let payload_guard = ctx.interp.heap.create_guard();
        payload_guard.guard(obj.clone());
        RuntimeValue::with_guard(payload_value.clone(), payload_guard)
    } else {
        RuntimeValue::unguarded(payload_value)
    };

    // Record the pending order
    ctx.interp.pending_orders.push(crate::Order {
        id,
        payload: payload_rv,
    });

    // Create PendingOrder marker - VM will suspend when this is returned
    let marker_guard = ctx.interp.heap.create_guard();
    let marker = marker_guard.alloc();
    marker.borrow_mut().exotic = crate::value::ExoticObject::PendingOrder { id: id.0 };

    TsRunValueResult::ok(Box::new(TsRunValue {
        inner: RuntimeValue::with_guard(JsValue::Object(marker), marker_guard),
    }))
}

// ============================================================================
// Promise Operations
// ============================================================================

/// Create a promise for deferred order fulfillment.
///
/// Use when you want to return a promise that will be resolved later.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_create_order_promise(
    ctx: *mut TsRunContext,
    order_id: u64,
) -> TsRunValueResult {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => {
            return TsRunValueResult {
                value: ptr::null_mut(),
                error: b"NULL context\0".as_ptr() as *const c_char,
            }
        }
    };

    let promise = crate::api::create_order_promise(&mut ctx.interp, OrderId(order_id));
    TsRunValueResult::ok(TsRunValue::from_runtime_value(promise))
}

/// Resolve a promise created with tsrun_create_order_promise.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_resolve_promise(
    ctx: *mut TsRunContext,
    promise: *mut TsRunValue,
    value: *mut TsRunValue,
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

    let promise_val = match unsafe { promise.as_ref() } {
        Some(v) => v,
        None => return TsRunResult::err(ctx, "NULL promise".to_string()),
    };

    let value_val = match unsafe { value.as_ref() } {
        Some(v) => RuntimeValue::unguarded(v.value().clone()),
        None => RuntimeValue::unguarded(JsValue::Undefined),
    };

    let promise_rv = RuntimeValue::unguarded(promise_val.value().clone());
    match crate::api::resolve_promise(&mut ctx.interp, &promise_rv, value_val) {
        Ok(()) => TsRunResult::success(),
        Err(e) => TsRunResult::err(ctx, e.to_string()),
    }
}

/// Reject a promise created with tsrun_create_order_promise.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_reject_promise(
    ctx: *mut TsRunContext,
    promise: *mut TsRunValue,
    error: *const c_char,
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

    let promise_val = match unsafe { promise.as_ref() } {
        Some(v) => v,
        None => return TsRunResult::err(ctx, "NULL promise".to_string()),
    };

    let error_str = unsafe { c_str_to_str(error) }.unwrap_or("Unknown error");

    // Create an error object as the rejection reason
    let guard = ctx.interp.heap.create_guard();
    let error_obj = guard.alloc();
    {
        let mut obj_ref = error_obj.borrow_mut();
        obj_ref.prototype = Some(ctx.interp.error_prototype.cheap_clone());
        obj_ref.set_property(
            PropertyKey::String(JsString::from("message")),
            JsValue::String(JsString::from(error_str)),
        );
        obj_ref.set_property(
            PropertyKey::String(JsString::from("name")),
            JsValue::String(JsString::from("Error")),
        );
    }
    let error_rv = RuntimeValue::with_guard(JsValue::Object(error_obj), guard);

    let promise_rv = RuntimeValue::unguarded(promise_val.value().clone());
    match crate::api::reject_promise(&mut ctx.interp, &promise_rv, error_rv) {
        Ok(()) => TsRunResult::success(),
        Err(e) => TsRunResult::err(ctx, e.to_string()),
    }
}
