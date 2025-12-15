//! eval:internal built-in module
//!
//! Provides the core order system functions for async operations.

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{Guarded, JsValue};
use crate::{InternalModule, Order, OrderId};

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
/// Creates an order and returns the order ID.
/// The host will fulfill the order and the value will be available
/// when the runtime resumes.
///
/// Usage: const orderId = __order__({ type: "readFile", path: "/foo" });
fn order_syscall(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let payload = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Generate unique order ID
    let id = OrderId(interp.next_order_id);
    interp.next_order_id += 1;

    // Record the pending order
    interp.pending_orders.push(Order {
        id,
        payload: payload.clone(),
    });

    // If payload is an object, guard it to keep it alive until fulfilled
    if let JsValue::Object(ref obj) = payload {
        let guard = interp.heap.create_guard();
        guard.guard(obj);
        interp.pending_order_guards.push(guard);
    }

    // Return the order ID as a number
    Ok(Guarded::unguarded(JsValue::Number(id.0 as f64)))
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

    // Remove from callbacks if registered
    interp.order_callbacks.remove(&id);

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
