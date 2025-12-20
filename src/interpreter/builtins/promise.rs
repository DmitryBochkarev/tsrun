//! Promise built-in methods (new GC implementation)
//!
//! This module implements Promise using the new guard-based GC system.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::JsError;
use crate::gc::{Gc, Guard};
use crate::interpreter::Interpreter;
use crate::value::{
    CheapClone, ExoticObject, Guarded, JsFunction, JsObject, JsValue, PromiseAllSharedState,
    PromiseHandler, PromiseState, PromiseStatus, PropertyKey,
};

/// Initialize Promise.prototype with then, catch, finally methods
pub fn init_promise_prototype(interp: &mut Interpreter) {
    let proto = interp.promise_prototype.clone();

    interp.register_method(&proto, "then", promise_then, 2);
    interp.register_method(&proto, "catch", promise_catch, 1);
    interp.register_method(&proto, "finally", promise_finally, 1);
}

/// Create Promise constructor with static methods
pub fn create_promise_constructor(interp: &mut Interpreter) -> Gc<JsObject> {
    let ctor = interp.create_native_function("Promise", promise_constructor, 1);

    // Set constructor.prototype = Promise.prototype
    let proto_key = PropertyKey::String(interp.intern("prototype"));
    ctor.borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.promise_prototype.clone()));

    // Set Promise.prototype.constructor = Promise
    let constructor_key = PropertyKey::String(interp.intern("constructor"));
    interp
        .promise_prototype
        .borrow_mut()
        .set_property(constructor_key, JsValue::Object(ctor.clone()));

    // Static methods
    interp.register_method(&ctor, "resolve", promise_resolve_static, 1);
    interp.register_method(&ctor, "reject", promise_reject_static, 1);
    interp.register_method(&ctor, "all", promise_all, 1);
    interp.register_method(&ctor, "race", promise_race, 1);
    interp.register_method(&ctor, "allSettled", promise_allsettled, 1);
    interp.register_method(&ctor, "any", promise_any, 1);

    ctor
}

/// Create a new pending promise object using the provided guard
pub fn create_promise(interp: &mut Interpreter, guard: &Guard<JsObject>) -> Gc<JsObject> {
    let state = Rc::new(RefCell::new(PromiseState {
        status: PromiseStatus::Pending,
        result: None,
        handlers: Vec::new(),
    }));

    let obj = interp.create_object(guard);
    {
        let mut o = obj.borrow_mut();
        o.prototype = Some(interp.promise_prototype.cheap_clone());
        o.exotic = ExoticObject::Promise(state);
    }
    obj
}

/// Create a fulfilled promise using the provided guard
pub fn create_fulfilled_promise(
    interp: &mut Interpreter,
    guard: &Guard<JsObject>,
    value: JsValue,
) -> Gc<JsObject> {
    // Guard the value BEFORE allocating the promise object
    // This prevents GC from collecting the value during allocation
    let _value_guard = interp.guard_value(&value);

    let state = Rc::new(RefCell::new(PromiseState {
        status: PromiseStatus::Fulfilled,
        result: Some(value),
        handlers: Vec::new(),
    }));

    let obj = interp.create_object(guard);
    {
        let mut o = obj.borrow_mut();
        o.prototype = Some(interp.promise_prototype.cheap_clone());
        o.exotic = ExoticObject::Promise(state);
    }
    obj
}

/// Create a rejected promise using the provided guard
pub fn create_rejected_promise(
    interp: &mut Interpreter,
    guard: &Guard<JsObject>,
    reason: JsValue,
) -> Gc<JsObject> {
    // Guard the reason BEFORE allocating the promise object
    // This prevents GC from collecting the reason during allocation
    let _reason_guard = interp.guard_value(&reason);

    let state = Rc::new(RefCell::new(PromiseState {
        status: PromiseStatus::Rejected,
        result: Some(reason),
        handlers: Vec::new(),
    }));

    let obj = interp.create_object(guard);
    {
        let mut o = obj.borrow_mut();
        o.prototype = Some(interp.promise_prototype.cheap_clone());
        o.exotic = ExoticObject::Promise(state);
    }
    obj
}

/// Resolve a promise (fulfill or reject based on value)
fn resolve_promise(
    interp: &mut Interpreter,
    promise: &Gc<JsObject>,
    value: JsValue,
) -> Result<(), JsError> {
    // Check if value is a thenable (another promise)
    if let JsValue::Object(obj) = &value {
        if let ExoticObject::Promise(state) = &obj.borrow().exotic {
            // If the value is a promise, adopt its state
            let state_ref = state.borrow();
            match state_ref.status {
                PromiseStatus::Pending => {
                    // Chain this promise to the other
                    drop(state_ref);
                    let promise_clone = promise.cheap_clone();
                    let mut state_mut = state.borrow_mut();
                    state_mut.handlers.push(PromiseHandler {
                        on_fulfilled: None,
                        on_rejected: None,
                        result_promise: promise_clone,
                    });
                    return Ok(());
                }
                PromiseStatus::Fulfilled => {
                    let result = state_ref.result.clone().unwrap_or(JsValue::Undefined);
                    drop(state_ref);
                    return fulfill_promise(interp, promise, result);
                }
                PromiseStatus::Rejected => {
                    let result = state_ref.result.clone().unwrap_or(JsValue::Undefined);
                    drop(state_ref);
                    return reject_promise(interp, promise, result);
                }
            }
        }
    }

    // Not a promise, fulfill with the value
    fulfill_promise(interp, promise, value)
}

/// Fulfill a promise with a value
fn fulfill_promise(
    interp: &mut Interpreter,
    promise: &Gc<JsObject>,
    value: JsValue,
) -> Result<(), JsError> {
    let handlers = {
        let obj = promise.borrow();
        let ExoticObject::Promise(ref state) = obj.exotic else {
            return Err(JsError::type_error("Not a promise"));
        };

        let mut state_mut = state.borrow_mut();
        if state_mut.status != PromiseStatus::Pending {
            return Ok(()); // Already settled, ignore
        }

        state_mut.status = PromiseStatus::Fulfilled;
        state_mut.result = Some(value.clone());
        std::mem::take(&mut state_mut.handlers)
    };

    // Trigger handlers synchronously
    for handler in handlers {
        trigger_handler(interp, handler, &value, true)?;
    }

    Ok(())
}

/// Reject a promise with a reason
fn reject_promise(
    interp: &mut Interpreter,
    promise: &Gc<JsObject>,
    reason: JsValue,
) -> Result<(), JsError> {
    let handlers = {
        let obj = promise.borrow();
        let ExoticObject::Promise(ref state) = obj.exotic else {
            return Err(JsError::type_error("Not a promise"));
        };

        let mut state_mut = state.borrow_mut();
        if state_mut.status != PromiseStatus::Pending {
            return Ok(()); // Already settled, ignore
        }

        state_mut.status = PromiseStatus::Rejected;
        state_mut.result = Some(reason.clone());
        std::mem::take(&mut state_mut.handlers)
    };

    // Trigger handlers synchronously
    for handler in handlers {
        trigger_handler(interp, handler, &reason, false)?;
    }

    Ok(())
}

/// Public function to resolve a promise
pub fn resolve_promise_value(
    interp: &mut Interpreter,
    promise: &Gc<JsObject>,
    value: JsValue,
) -> Result<(), JsError> {
    resolve_promise(interp, promise, value)
}

/// Public function to reject a promise
pub fn reject_promise_value(
    interp: &mut Interpreter,
    promise: &Gc<JsObject>,
    reason: JsValue,
) -> Result<(), JsError> {
    reject_promise(interp, promise, reason)
}

/// Trigger a promise handler
fn trigger_handler(
    interp: &mut Interpreter,
    handler: PromiseHandler,
    value: &JsValue,
    is_fulfilled: bool,
) -> Result<(), JsError> {
    // Guard the result_promise since it's been removed from the traced promise state
    let guard = interp.heap.create_guard();
    guard.guard(handler.result_promise.clone());

    // Guard the callbacks too - they've been removed from the promise and need protection
    if let Some(JsValue::Object(ref cb)) = handler.on_fulfilled {
        guard.guard(cb.clone());
    }
    if let Some(JsValue::Object(ref cb)) = handler.on_rejected {
        guard.guard(cb.clone());
    }

    let callback = if is_fulfilled {
        handler.on_fulfilled.clone()
    } else {
        handler.on_rejected.clone()
    };

    match callback {
        Some(cb) => {
            // Check if this is a Promise.all handler - these manage their own result promise
            let is_promise_all_handler = if let JsValue::Object(ref cb_obj) = cb {
                let cb_ref = cb_obj.borrow();
                matches!(
                    cb_ref.exotic,
                    ExoticObject::Function(JsFunction::PromiseAllFulfill { .. })
                        | ExoticObject::Function(JsFunction::PromiseAllReject(_))
                )
            } else {
                false
            };

            // Call the callback
            match interp.call_function(cb, JsValue::Undefined, std::slice::from_ref(value)) {
                Ok(Guarded { value: result, .. }) => {
                    // Promise.all handlers manage their own result promise resolution
                    if !is_promise_all_handler {
                        resolve_promise(interp, &handler.result_promise, result)?;
                    }
                }
                Err(e) => {
                    // If callback throws, reject the result promise
                    let error_value = e.to_value();
                    reject_promise(interp, &handler.result_promise, error_value)?;
                }
            }
        }
        None => {
            // No callback - propagate the value/reason to result_promise
            if is_fulfilled {
                fulfill_promise(interp, &handler.result_promise, value.clone())?;
            } else {
                reject_promise(interp, &handler.result_promise, value.clone())?;
            }
        }
    }

    Ok(())
}

/// Promise constructor: new Promise((resolve, reject) => { ... })
pub fn promise_constructor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let executor = args
        .first()
        .cloned()
        .ok_or_else(|| JsError::type_error("Promise resolver undefined is not a function"))?;

    if !executor.is_callable() {
        return Err(JsError::type_error(format!(
            "Promise resolver {} is not a function",
            executor.type_of()
        )));
    }

    let guard = interp.heap.create_guard();
    let promise = create_promise(interp, &guard);

    // Create resolve function using the PromiseResolve variant
    let resolve_fn =
        interp.create_js_function(&guard, JsFunction::PromiseResolve(promise.cheap_clone()));

    // Create reject function using the PromiseReject variant
    let reject_fn =
        interp.create_js_function(&guard, JsFunction::PromiseReject(promise.cheap_clone()));

    // Call executor(resolve, reject)
    match interp.call_function(
        executor,
        JsValue::Undefined,
        &[JsValue::Object(resolve_fn), JsValue::Object(reject_fn)],
    ) {
        Ok(_) => {}
        Err(e) => {
            // If executor throws, reject the promise
            let error_value = e.to_value();
            reject_promise(interp, &promise, error_value)?;
        }
    }

    Ok(Guarded::with_guard(JsValue::Object(promise), guard))
}

/// Promise.prototype.then(onFulfilled, onRejected)
pub fn promise_then(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(promise) = this else {
        return Err(JsError::type_error(
            "Promise.prototype.then called on non-object",
        ));
    };

    // Guard the promise
    let guard = interp.heap.create_guard();
    guard.guard(promise.clone());

    let on_fulfilled = args.first().cloned();
    let on_rejected = args.get(1).cloned();

    // Guard the callbacks BEFORE creating the result promise
    // This ensures they (and their closures) survive GC during allocation
    if let Some(JsValue::Object(ref cb)) = on_fulfilled {
        guard.guard(cb.clone());
    }
    if let Some(JsValue::Object(ref cb)) = on_rejected {
        guard.guard(cb.clone());
    }

    // Filter out non-callable values
    let on_fulfilled = on_fulfilled.filter(|v| v.is_callable());
    let on_rejected = on_rejected.filter(|v| v.is_callable());

    // Create the result promise
    let result_promise = create_promise(interp, &guard);

    let (status, result) = {
        let obj = promise.borrow();
        let ExoticObject::Promise(ref state) = obj.exotic else {
            return Err(JsError::type_error("Not a promise"));
        };
        let state_ref = state.borrow();
        (state_ref.status.clone(), state_ref.result.clone())
    };

    match status {
        PromiseStatus::Pending => {
            // Add handler to pending promise
            let obj = promise.borrow();
            let ExoticObject::Promise(ref state) = obj.exotic else {
                return Err(JsError::type_error("Not a promise"));
            };
            state.borrow_mut().handlers.push(PromiseHandler {
                on_fulfilled,
                on_rejected,
                result_promise: result_promise.cheap_clone(),
            });
        }
        PromiseStatus::Fulfilled => {
            // Already fulfilled - trigger handler immediately
            let value = result.unwrap_or(JsValue::Undefined);
            let handler = PromiseHandler {
                on_fulfilled,
                on_rejected,
                result_promise: result_promise.cheap_clone(),
            };
            trigger_handler(interp, handler, &value, true)?;
        }
        PromiseStatus::Rejected => {
            // Already rejected - trigger handler immediately
            let reason = result.unwrap_or(JsValue::Undefined);
            let handler = PromiseHandler {
                on_fulfilled,
                on_rejected,
                result_promise: result_promise.cheap_clone(),
            };
            trigger_handler(interp, handler, &reason, false)?;
        }
    }

    Ok(Guarded::with_guard(JsValue::Object(result_promise), guard))
}

/// Promise.prototype.catch(onRejected)
pub fn promise_catch(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // catch(onRejected) is equivalent to then(undefined, onRejected)
    let on_rejected = args.first().cloned().unwrap_or(JsValue::Undefined);
    promise_then(interp, this, &[JsValue::Undefined, on_rejected])
}

/// Promise.prototype.finally(onFinally)
pub fn promise_finally(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(promise) = this.clone() else {
        return Err(JsError::type_error(
            "Promise.prototype.finally called on non-object",
        ));
    };

    let on_finally = args.first().cloned();
    let on_finally = on_finally.filter(|v| v.is_callable());

    match on_finally {
        Some(callback) => {
            let guard = interp.heap.create_guard();
            let result_promise = create_promise(interp, &guard);

            let (status, result) = {
                let obj = promise.borrow();
                let ExoticObject::Promise(ref state) = obj.exotic else {
                    return Err(JsError::type_error("Not a promise"));
                };
                let state_ref = state.borrow();
                (state_ref.status.clone(), state_ref.result.clone())
            };

            match status {
                PromiseStatus::Pending => {
                    // Store callback reference in the handler
                    let obj = promise.borrow();
                    let ExoticObject::Promise(ref state) = obj.exotic else {
                        return Err(JsError::type_error("Not a promise"));
                    };
                    // Store callback in both slots - we'll call it regardless
                    state.borrow_mut().handlers.push(PromiseHandler {
                        on_fulfilled: Some(callback.clone()),
                        on_rejected: Some(callback),
                        result_promise: result_promise.cheap_clone(),
                    });
                }
                PromiseStatus::Fulfilled => {
                    let value = result.unwrap_or(JsValue::Undefined);
                    // Call the finally callback
                    let _ = interp.call_function(callback, JsValue::Undefined, &[]);
                    // Fulfill result promise with original value
                    fulfill_promise(interp, &result_promise, value)?;
                }
                PromiseStatus::Rejected => {
                    let reason = result.unwrap_or(JsValue::Undefined);
                    // Call the finally callback
                    let _ = interp.call_function(callback, JsValue::Undefined, &[]);
                    // Reject result promise with original reason
                    reject_promise(interp, &result_promise, reason)?;
                }
            }

            Ok(Guarded::with_guard(JsValue::Object(result_promise), guard))
        }
        None => {
            // No callback - just return a then with no handlers
            promise_then(interp, this, &[JsValue::Undefined, JsValue::Undefined])
        }
    }
}

/// Promise.resolve(value)
pub fn promise_resolve_static(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);

    // If value is already a promise, return it as-is
    if let JsValue::Object(obj) = &value {
        if matches!(obj.borrow().exotic, ExoticObject::Promise(_)) {
            return Ok(Guarded::unguarded(value));
        }
    }

    let guard = interp.heap.create_guard();
    let promise = create_fulfilled_promise(interp, &guard, value);
    Ok(Guarded::with_guard(JsValue::Object(promise), guard))
}

/// Promise.reject(reason)
pub fn promise_reject_static(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let reason = args.first().cloned().unwrap_or(JsValue::Undefined);
    let guard = interp.heap.create_guard();
    let promise = create_rejected_promise(interp, &guard, reason);
    Ok(Guarded::with_guard(JsValue::Object(promise), guard))
}

/// Extract values from an iterable (for Promise.all/race/etc)
fn extract_iterable(value: &JsValue) -> Result<Vec<JsValue>, JsError> {
    let JsValue::Object(arr) = value else {
        return Ok(vec![]);
    };

    let arr_ref = arr.borrow();
    if let Some(elements) = arr_ref.array_elements() {
        Ok(elements.to_vec())
    } else {
        Ok(vec![])
    }
}

/// Promise.all(iterable)
pub fn promise_all(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let iterable = args.first().cloned().unwrap_or(JsValue::Undefined);
    let promises = extract_iterable(&iterable)?;

    let guard = interp.heap.create_guard();

    if promises.is_empty() {
        let arr = interp.create_empty_array(&guard);
        let promise = create_fulfilled_promise(interp, &guard, JsValue::Object(arr));
        return Ok(Guarded::with_guard(JsValue::Object(promise), guard));
    }

    // First pass: check for already-rejected promises and count pending
    let mut results: Vec<JsValue> = vec![JsValue::Undefined; promises.len()];
    let mut pending_count = 0;
    let mut pending_indices: Vec<usize> = Vec::new();

    for (i, promise_value) in promises.iter().enumerate() {
        let (status, result) = if let JsValue::Object(obj) = promise_value {
            let obj_ref = obj.borrow();
            if let ExoticObject::Promise(ref state) = obj_ref.exotic {
                let state_ref = state.borrow();
                (state_ref.status.clone(), state_ref.result.clone())
            } else {
                // Non-promise object is treated as fulfilled with that value
                (PromiseStatus::Fulfilled, Some(promise_value.clone()))
            }
        } else {
            // Non-object value is treated as fulfilled with that value
            (PromiseStatus::Fulfilled, Some(promise_value.clone()))
        };

        match status {
            PromiseStatus::Fulfilled => {
                if let Some(idx) = results.get_mut(i) {
                    *idx = result.unwrap_or(JsValue::Undefined);
                }
            }
            PromiseStatus::Rejected => {
                // Short-circuit: reject immediately
                let reason = result.unwrap_or(JsValue::Undefined);
                let promise = create_rejected_promise(interp, &guard, reason);
                return Ok(Guarded::with_guard(JsValue::Object(promise), guard));
            }
            PromiseStatus::Pending => {
                pending_count += 1;
                pending_indices.push(i);
            }
        }
    }

    // If no pending promises, all are fulfilled - return fulfilled promise
    if pending_count == 0 {
        let arr = interp.create_array_from(&guard, results);
        let promise = create_fulfilled_promise(interp, &guard, JsValue::Object(arr));
        return Ok(Guarded::with_guard(JsValue::Object(promise), guard));
    }

    // Create the result promise (pending)
    let result_promise = create_promise(interp, &guard);

    // Create shared state for tracking - shared by all handlers
    let shared_state = Rc::new(PromiseAllSharedState {
        remaining: std::cell::Cell::new(pending_count),
        results: RefCell::new(results),
        result_promise: result_promise.cheap_clone(),
        rejected: std::cell::Cell::new(false),
    });

    // Attach handlers to each pending promise
    for &idx in &pending_indices {
        if let Some(JsValue::Object(promise_obj)) = promises.get(idx) {
            let promise_obj_ref = promise_obj.borrow();
            if let ExoticObject::Promise(ref state) = promise_obj_ref.exotic {
                // Create on_fulfilled callback using PromiseAllFulfill variant
                // Each handler shares the same state but has its own index
                let on_fulfilled = interp.create_object(&guard);
                {
                    let mut f = on_fulfilled.borrow_mut();
                    f.prototype = Some(interp.function_prototype.cheap_clone());
                    f.exotic = ExoticObject::Function(JsFunction::PromiseAllFulfill {
                        state: shared_state.clone(),
                        index: idx,
                    });
                }

                // Create on_rejected callback using PromiseAllReject variant
                let on_rejected = interp.create_object(&guard);
                {
                    let mut f = on_rejected.borrow_mut();
                    f.prototype = Some(interp.function_prototype.cheap_clone());
                    f.exotic =
                        ExoticObject::Function(JsFunction::PromiseAllReject(shared_state.clone()));
                }

                // Add handler to the pending promise
                let mut state_mut = state.borrow_mut();
                state_mut.handlers.push(PromiseHandler {
                    on_fulfilled: Some(JsValue::Object(on_fulfilled)),
                    on_rejected: Some(JsValue::Object(on_rejected)),
                    result_promise: result_promise.cheap_clone(),
                });
            }
        }
    }

    Ok(Guarded::with_guard(JsValue::Object(result_promise), guard))
}

/// Handle Promise.all fulfill - called when one of the input promises resolves
pub fn handle_promise_all_fulfill(
    interp: &mut Interpreter,
    state: &Rc<PromiseAllSharedState>,
    index: usize,
    value: JsValue,
) -> Result<(), JsError> {
    if state.rejected.get() {
        // Already rejected, ignore
        return Ok(());
    }

    // Store result at the correct index
    {
        let mut results = state.results.borrow_mut();
        if let Some(slot) = results.get_mut(index) {
            *slot = value;
        }
    }

    let remaining = state.remaining.get();
    state.remaining.set(remaining - 1);

    if remaining - 1 == 0 {
        // All promises fulfilled - fulfill the result promise
        let results = std::mem::take(&mut *state.results.borrow_mut());
        let result_promise = state.result_promise.cheap_clone();

        let guard = interp.heap.create_guard();
        let arr = interp.create_array_from(&guard, results);
        fulfill_promise(interp, &result_promise, JsValue::Object(arr))?;
    }

    Ok(())
}

/// Handle Promise.all reject - called when any of the input promises rejects
pub fn handle_promise_all_reject(
    interp: &mut Interpreter,
    state: &Rc<PromiseAllSharedState>,
    reason: JsValue,
) -> Result<(), JsError> {
    if state.rejected.get() {
        // Already rejected, ignore
        return Ok(());
    }

    state.rejected.set(true);
    let result_promise = state.result_promise.cheap_clone();

    reject_promise(interp, &result_promise, reason)?;

    Ok(())
}

/// Promise.race(iterable)
pub fn promise_race(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let iterable = args.first().cloned().unwrap_or(JsValue::Undefined);
    let promises = extract_iterable(&iterable)?;

    let mut first_result: Option<(PromiseStatus, JsValue)> = None;

    for promise_value in &promises {
        let (status, result) = if let JsValue::Object(obj) = promise_value {
            let obj_ref = obj.borrow();
            if let ExoticObject::Promise(ref state) = obj_ref.exotic {
                let state_ref = state.borrow();
                (state_ref.status.clone(), state_ref.result.clone())
            } else {
                (PromiseStatus::Fulfilled, Some(promise_value.clone()))
            }
        } else {
            (PromiseStatus::Fulfilled, Some(promise_value.clone()))
        };

        match status {
            PromiseStatus::Fulfilled | PromiseStatus::Rejected => {
                first_result = Some((status, result.unwrap_or(JsValue::Undefined)));
                break;
            }
            PromiseStatus::Pending => {}
        }
    }

    let guard = interp.heap.create_guard();
    match first_result {
        Some((PromiseStatus::Fulfilled, value)) => {
            let promise = create_fulfilled_promise(interp, &guard, value);
            Ok(Guarded::with_guard(JsValue::Object(promise), guard))
        }
        Some((PromiseStatus::Rejected, reason)) => {
            let promise = create_rejected_promise(interp, &guard, reason);
            Ok(Guarded::with_guard(JsValue::Object(promise), guard))
        }
        _ => {
            // No settled promise found - return pending
            let promise = create_promise(interp, &guard);
            Ok(Guarded::with_guard(JsValue::Object(promise), guard))
        }
    }
}

/// Promise.allSettled(iterable)
pub fn promise_allsettled(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let iterable = args.first().cloned().unwrap_or(JsValue::Undefined);
    let promises = extract_iterable(&iterable)?;

    let guard = interp.heap.create_guard();

    if promises.is_empty() {
        let arr = interp.create_empty_array(&guard);
        let promise = create_fulfilled_promise(interp, &guard, JsValue::Object(arr));
        return Ok(Guarded::with_guard(JsValue::Object(promise), guard));
    }

    // Pre-intern keys
    let status_key = PropertyKey::String(interp.intern("status"));
    let value_key = PropertyKey::String(interp.intern("value"));
    let reason_key = PropertyKey::String(interp.intern("reason"));

    let mut results: Vec<JsValue> = Vec::with_capacity(promises.len());

    for promise_value in &promises {
        let (status, result) = if let JsValue::Object(obj) = promise_value {
            let obj_ref = obj.borrow();
            if let ExoticObject::Promise(ref state) = obj_ref.exotic {
                let state_ref = state.borrow();
                (state_ref.status.clone(), state_ref.result.clone())
            } else {
                (PromiseStatus::Fulfilled, Some(promise_value.clone()))
            }
        } else {
            (PromiseStatus::Fulfilled, Some(promise_value.clone()))
        };

        let result_obj = interp.create_object(&guard);
        {
            let mut result_ref = result_obj.borrow_mut();
            result_ref.prototype = Some(interp.object_prototype.cheap_clone());

            match status {
                PromiseStatus::Fulfilled => {
                    result_ref
                        .set_property(status_key.clone(), JsValue::String("fulfilled".into()));
                    result_ref
                        .set_property(value_key.clone(), result.unwrap_or(JsValue::Undefined));
                }
                PromiseStatus::Rejected => {
                    result_ref.set_property(status_key.clone(), JsValue::String("rejected".into()));
                    result_ref
                        .set_property(reason_key.clone(), result.unwrap_or(JsValue::Undefined));
                }
                PromiseStatus::Pending => {
                    result_ref.set_property(status_key.clone(), JsValue::String("pending".into()));
                }
            }
        }
        results.push(JsValue::Object(result_obj));
    }

    let arr = interp.create_array_from(&guard, results);
    let promise = create_fulfilled_promise(interp, &guard, JsValue::Object(arr));
    Ok(Guarded::with_guard(JsValue::Object(promise), guard))
}

/// Promise.any(iterable)
pub fn promise_any(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let iterable = args.first().cloned().unwrap_or(JsValue::Undefined);
    let promises = extract_iterable(&iterable)?;

    let guard = interp.heap.create_guard();

    if promises.is_empty() {
        let promise = create_rejected_promise(
            interp,
            &guard,
            JsValue::String("All promises were rejected".into()),
        );
        return Ok(Guarded::with_guard(JsValue::Object(promise), guard));
    }

    let mut errors: Vec<JsValue> = Vec::new();
    let mut fulfilled_value: Option<JsValue> = None;
    let mut any_pending = false;

    for promise_value in &promises {
        let (status, result) = if let JsValue::Object(obj) = promise_value {
            let obj_ref = obj.borrow();
            if let ExoticObject::Promise(ref state) = obj_ref.exotic {
                let state_ref = state.borrow();
                (state_ref.status.clone(), state_ref.result.clone())
            } else {
                (PromiseStatus::Fulfilled, Some(promise_value.clone()))
            }
        } else {
            (PromiseStatus::Fulfilled, Some(promise_value.clone()))
        };

        match status {
            PromiseStatus::Fulfilled => {
                fulfilled_value = Some(result.unwrap_or(JsValue::Undefined));
                break;
            }
            PromiseStatus::Rejected => {
                errors.push(result.unwrap_or(JsValue::Undefined));
            }
            PromiseStatus::Pending => {
                any_pending = true;
            }
        }
    }

    if let Some(value) = fulfilled_value {
        let promise = create_fulfilled_promise(interp, &guard, value);
        return Ok(Guarded::with_guard(JsValue::Object(promise), guard));
    }

    if !errors.is_empty() && !any_pending {
        let errors_arr = interp.create_array_from(&guard, errors);
        let promise = create_rejected_promise(interp, &guard, JsValue::Object(errors_arr));
        return Ok(Guarded::with_guard(JsValue::Object(promise), guard));
    }

    // Return pending promise
    let promise = create_promise(interp, &guard);
    Ok(Guarded::with_guard(JsValue::Object(promise), guard))
}
