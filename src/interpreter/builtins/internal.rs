//! eval:internal built-in module
//!
//! Provides the core order system functions for async operations.

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{CheapClone, Guarded, JsFunction, JsValue};
use crate::{InternalModule, Order, OrderId};

use super::promise::create_promise;

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
/// Creates an order and returns a Promise that will be resolved when the host
/// fulfills the order.
///
/// Usage: const result = await __order__({ type: "readFile", path: "/foo" });
fn order_syscall(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let payload = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Generate unique order ID
    let id = OrderId(interp.next_order_id);
    interp.next_order_id += 1;

    // Create a pending promise
    let promise_guard = interp.heap.create_guard();
    let promise = create_promise(interp, &promise_guard);

    // Create resolve/reject functions for this promise
    let resolve_fn = interp.create_js_function(
        &promise_guard,
        JsFunction::PromiseResolve(promise.cheap_clone()),
    );
    let reject_fn = interp.create_js_function(
        &promise_guard,
        JsFunction::PromiseReject(promise.cheap_clone()),
    );

    // Store callbacks for order fulfillment
    interp.order_callbacks.insert(id, (resolve_fn, reject_fn));

    // Record the pending order
    interp.pending_orders.push(Order {
        id,
        payload: payload.clone(),
    });

    // If payload is an object, guard it to keep it alive until fulfilled
    if let JsValue::Object(ref obj) = payload {
        let guard = interp.heap.create_guard();
        guard.guard(obj.clone());
        interp.pending_order_guards.push(guard);
    }

    // Keep the promise alive
    interp.pending_order_guards.push(promise_guard);

    // Return the promise
    Ok(Guarded::unguarded(JsValue::Object(promise)))
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
