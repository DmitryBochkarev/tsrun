//! eval:internal built-in module
//!
//! Provides the core order system functions for blocking host operations.

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{ExoticObject, Guarded, JsValue};
use crate::{InternalModule, Order, OrderId, RuntimeValue};

/// Create the eval:internal module
pub fn create_eval_internal_module() -> InternalModule {
    InternalModule::native("eval:internal")
        .with_function("__order__", order_syscall, 1)
        .with_function("__cancelOrder__", cancel_order_syscall, 1)
        .with_function("__getOrderId__", get_order_id_syscall, 0)
        .build()
}

/// Native implementation of __order__
///
/// Suspends the VM immediately and creates a pending order. The host provides
/// a response value (any JsValue) via fulfill_orders(). The VM resumes and
/// __order__() returns that value directly.
///
/// This is a blocking operation - the VM suspends at the call site.
/// If host wants to defer the actual value, host can return a Promise
/// and the script can await it.
///
/// Usage:
///   const result = __order__({ type: "readFile", path: "/foo" });
///   // If host returns a Promise that needs unwrapping:
///   const result = await __order__({ type: "getAsyncValue" });
fn order_syscall(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let payload = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Generate unique order ID
    let id = OrderId(interp.next_order_id);
    interp.next_order_id += 1;

    // Create payload RuntimeValue with guard if it's an object
    let payload_rv = if let JsValue::Object(ref obj) = payload {
        let payload_guard = interp.heap.create_guard();
        payload_guard.guard(obj.clone());
        RuntimeValue::with_guard(payload, payload_guard)
    } else {
        RuntimeValue::unguarded(payload)
    };

    // Record the pending order
    interp.pending_orders.push(Order {
        id,
        payload: payload_rv,
    });

    // Return PendingOrder marker - VM will suspend when this is detected
    let marker_guard = interp.heap.create_guard();
    let marker = marker_guard.alloc();
    marker.borrow_mut().exotic = ExoticObject::PendingOrder { id: id.0 };

    Ok(Guarded::with_guard(JsValue::Object(marker), marker_guard))
}

/// Native implementation of __cancelOrder__
///
/// Cancels a pending order.
///
/// Usage: __cancelOrder__(orderId);
fn cancel_order_syscall(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let id = match args.first() {
        Some(JsValue::Number(n)) => OrderId(*n as u64),
        _ => return Err(JsError::type_error("__cancelOrder__ requires order ID")),
    };

    // Mark as cancelled
    interp.cancelled_orders.push(id);

    // Remove from pending
    interp.pending_orders.retain(|o| o.id != id);

    // Remove any pending response (in case host already provided one)
    interp.order_responses.remove(&id);

    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// Native implementation of __getOrderId__
///
/// Returns a new unique order ID without creating an order.
/// Useful for tracking purposes.
///
/// Usage: const id = __getOrderId__();
fn get_order_id_syscall(
    interp: &mut Interpreter,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let id = interp.next_order_id;
    interp.next_order_id += 1;
    Ok(Guarded::unguarded(JsValue::Number(id as f64)))
}
