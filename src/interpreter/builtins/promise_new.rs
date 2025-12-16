//! Promise built-in methods (new GC implementation)
//!
//! This module implements Promise using the new guard-based GC system.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::JsError;
use crate::gc::{Gc, Guard};
use crate::interpreter::Interpreter;
use crate::value::{
    CheapClone, ExoticObject, Guarded, JsFunction, JsObject, JsValue, PromiseHandler, PromiseState,
    PromiseStatus, PropertyKey,
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

    let proto_key = interp.key("prototype");
    ctor.borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.promise_prototype.clone()));

    // Static methods
    interp.register_method(&ctor, "resolve", promise_resolve_static, 1);
    interp.register_method(&ctor, "reject", promise_reject_static, 1);
    interp.register_method(&ctor, "all", promise_all, 1);
    interp.register_method(&ctor, "race", promise_race, 1);
    interp.register_method(&ctor, "allSettled", promise_allsettled, 1);
    interp.register_method(&ctor, "any", promise_any, 1);

    ctor
}

/// Create a new pending promise object with a guard
pub fn create_promise_with_guard(interp: &mut Interpreter) -> (Gc<JsObject>, Guard<JsObject>) {
    let state = Rc::new(RefCell::new(PromiseState {
        status: PromiseStatus::Pending,
        result: None,
        handlers: Vec::new(),
    }));

    let (obj, guard) = interp.create_object_with_guard();
    {
        let mut o = obj.borrow_mut();
        o.prototype = Some(interp.promise_prototype.cheap_clone());
        o.exotic = ExoticObject::Promise(state);
    }
    (obj, guard)
}

/// Create a fulfilled promise with a guard
pub fn create_fulfilled_promise_with_guard(
    interp: &mut Interpreter,
    value: JsValue,
) -> (Gc<JsObject>, Guard<JsObject>) {
    // Guard the value BEFORE allocating the promise object
    // This prevents GC from collecting the value during allocation
    let _value_guard = interp.guard_value(&value);

    let state = Rc::new(RefCell::new(PromiseState {
        status: PromiseStatus::Fulfilled,
        result: Some(value),
        handlers: Vec::new(),
    }));

    let (obj, guard) = interp.create_object_with_guard();
    {
        let mut o = obj.borrow_mut();
        o.prototype = Some(interp.promise_prototype.cheap_clone());
        o.exotic = ExoticObject::Promise(state);
    }
    (obj, guard)
}

/// Create a rejected promise with a guard
pub fn create_rejected_promise_with_guard(
    interp: &mut Interpreter,
    reason: JsValue,
) -> (Gc<JsObject>, Guard<JsObject>) {
    // Guard the reason BEFORE allocating the promise object
    // This prevents GC from collecting the reason during allocation
    let _reason_guard = interp.guard_value(&reason);

    let state = Rc::new(RefCell::new(PromiseState {
        status: PromiseStatus::Rejected,
        result: Some(reason),
        handlers: Vec::new(),
    }));

    let (obj, guard) = interp.create_object_with_guard();
    {
        let mut o = obj.borrow_mut();
        o.prototype = Some(interp.promise_prototype.cheap_clone());
        o.exotic = ExoticObject::Promise(state);
    }
    (obj, guard)
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

    let callback = if is_fulfilled {
        handler.on_fulfilled.clone()
    } else {
        handler.on_rejected.clone()
    };

    match callback {
        Some(cb) => {
            // Call the callback and resolve result_promise with the return value
            match interp.call_function(cb, JsValue::Undefined, std::slice::from_ref(value)) {
                Ok(Guarded { value: result, .. }) => {
                    resolve_promise(interp, &handler.result_promise, result)?;
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

    let (promise, promise_guard) = create_promise_with_guard(interp);

    // Create resolve function using the PromiseResolve variant
    let resolve_fn = interp.create_function(JsFunction::PromiseResolve(promise.cheap_clone()));
    let resolve_guard = interp.heap.create_guard();
    resolve_guard.guard(resolve_fn.clone());

    // Create reject function using the PromiseReject variant
    let reject_fn = interp.create_function(JsFunction::PromiseReject(promise.cheap_clone()));
    let reject_guard = interp.heap.create_guard();
    reject_guard.guard(reject_fn.clone());

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

    Ok(Guarded {
        value: JsValue::Object(promise),
        guard: Some(promise_guard),
    })
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

    // Filter out non-callable values
    let on_fulfilled = on_fulfilled.filter(|v| v.is_callable());
    let on_rejected = on_rejected.filter(|v| v.is_callable());

    // Create the result promise
    let (result_promise, result_guard) = create_promise_with_guard(interp);

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

    Ok(Guarded {
        value: JsValue::Object(result_promise),
        guard: Some(result_guard),
    })
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
            let (result_promise, result_guard) = create_promise_with_guard(interp);

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

            Ok(Guarded {
                value: JsValue::Object(result_promise),
                guard: Some(result_guard),
            })
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

    let (promise, guard) = create_fulfilled_promise_with_guard(interp, value);
    Ok(Guarded {
        value: JsValue::Object(promise),
        guard: Some(guard),
    })
}

/// Promise.reject(reason)
pub fn promise_reject_static(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let reason = args.first().cloned().unwrap_or(JsValue::Undefined);
    let (promise, guard) = create_rejected_promise_with_guard(interp, reason);
    Ok(Guarded {
        value: JsValue::Object(promise),
        guard: Some(guard),
    })
}

/// Extract values from an iterable (for Promise.all/race/etc)
fn extract_iterable(value: &JsValue) -> Result<Vec<JsValue>, JsError> {
    let JsValue::Object(arr) = value else {
        return Ok(vec![]);
    };

    let arr_ref = arr.borrow();
    if let ExoticObject::Array { length } = arr_ref.exotic {
        let mut result = Vec::with_capacity(length as usize);
        for i in 0..length {
            let elem = arr_ref
                .get_property(&PropertyKey::Index(i))
                .unwrap_or(JsValue::Undefined);
            result.push(elem);
        }
        Ok(result)
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

    if promises.is_empty() {
        let (arr, arr_guard) = interp.create_array_with_guard(vec![]);
        let (promise, promise_guard) =
            create_fulfilled_promise_with_guard(interp, JsValue::Object(arr));
        // Keep arr alive through promise_guard
        drop(arr_guard);
        return Ok(Guarded {
            value: JsValue::Object(promise),
            guard: Some(promise_guard),
        });
    }

    // Collect status info
    let mut results: Vec<JsValue> = Vec::with_capacity(promises.len());
    let mut rejected_reason: Option<JsValue> = None;

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
                results.push(result.unwrap_or(JsValue::Undefined));
            }
            PromiseStatus::Rejected => {
                rejected_reason = Some(result.unwrap_or(JsValue::Undefined));
                break;
            }
            PromiseStatus::Pending => {
                results.push(JsValue::Undefined);
            }
        }
    }

    if let Some(reason) = rejected_reason {
        let (promise, guard) = create_rejected_promise_with_guard(interp, reason);
        return Ok(Guarded {
            value: JsValue::Object(promise),
            guard: Some(guard),
        });
    }

    let (arr, arr_guard) = interp.create_array_with_guard(results);
    let (promise, promise_guard) =
        create_fulfilled_promise_with_guard(interp, JsValue::Object(arr));
    drop(arr_guard);
    Ok(Guarded {
        value: JsValue::Object(promise),
        guard: Some(promise_guard),
    })
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

    match first_result {
        Some((PromiseStatus::Fulfilled, value)) => {
            let (promise, guard) = create_fulfilled_promise_with_guard(interp, value);
            Ok(Guarded {
                value: JsValue::Object(promise),
                guard: Some(guard),
            })
        }
        Some((PromiseStatus::Rejected, reason)) => {
            let (promise, guard) = create_rejected_promise_with_guard(interp, reason);
            Ok(Guarded {
                value: JsValue::Object(promise),
                guard: Some(guard),
            })
        }
        _ => {
            // No settled promise found - return pending
            let (promise, guard) = create_promise_with_guard(interp);
            Ok(Guarded {
                value: JsValue::Object(promise),
                guard: Some(guard),
            })
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

    if promises.is_empty() {
        let (arr, arr_guard) = interp.create_array_with_guard(vec![]);
        let (promise, promise_guard) =
            create_fulfilled_promise_with_guard(interp, JsValue::Object(arr));
        drop(arr_guard);
        return Ok(Guarded {
            value: JsValue::Object(promise),
            guard: Some(promise_guard),
        });
    }

    // Pre-intern keys
    let status_key = interp.key("status");
    let value_key = interp.key("value");
    let reason_key = interp.key("reason");

    let mut results: Vec<JsValue> = Vec::with_capacity(promises.len());
    // Keep guards alive until we create the array - prevents GC from collecting result objects
    let mut result_guards: Vec<Guard<JsObject>> = Vec::with_capacity(promises.len());

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

        let (result_obj, obj_guard) = interp.create_object_with_guard();
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
        result_guards.push(obj_guard);
    }

    let (arr, arr_guard) = interp.create_array_with_guard(results);
    // Once array is created, result objects are reachable through it - safe to drop guards
    drop(result_guards);
    let (promise, promise_guard) =
        create_fulfilled_promise_with_guard(interp, JsValue::Object(arr));
    drop(arr_guard);
    Ok(Guarded {
        value: JsValue::Object(promise),
        guard: Some(promise_guard),
    })
}

/// Promise.any(iterable)
pub fn promise_any(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let iterable = args.first().cloned().unwrap_or(JsValue::Undefined);
    let promises = extract_iterable(&iterable)?;

    if promises.is_empty() {
        let (promise, guard) = create_rejected_promise_with_guard(
            interp,
            JsValue::String("All promises were rejected".into()),
        );
        return Ok(Guarded {
            value: JsValue::Object(promise),
            guard: Some(guard),
        });
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
        let (promise, guard) = create_fulfilled_promise_with_guard(interp, value);
        return Ok(Guarded {
            value: JsValue::Object(promise),
            guard: Some(guard),
        });
    }

    if !errors.is_empty() && !any_pending {
        let (errors_arr, arr_guard) = interp.create_array_with_guard(errors);
        let (promise, promise_guard) =
            create_rejected_promise_with_guard(interp, JsValue::Object(errors_arr));
        drop(arr_guard);
        return Ok(Guarded {
            value: JsValue::Object(promise),
            guard: Some(promise_guard),
        });
    }

    // Return pending promise
    let (promise, guard) = create_promise_with_guard(interp);
    Ok(Guarded {
        value: JsValue::Object(promise),
        guard: Some(guard),
    })
}
