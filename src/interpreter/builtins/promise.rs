//! Promise built-in methods

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::JsError;
use crate::gc::Space;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object, register_method, CheapClone, ExoticObject, JsFunction,
    JsObject, JsObjectRef, JsValue, NativeFunction, PromiseHandler, PromiseState, PromiseStatus,
    PropertyKey,
};

/// Create Promise.prototype with then, catch, finally methods
pub fn create_promise_prototype(space: &mut Space<JsObject>) -> JsObjectRef {
    let proto = create_object(space);

    register_method(space, &proto, "then", promise_then, 2);
    register_method(space, &proto, "catch", promise_catch, 1);
    register_method(space, &proto, "finally", promise_finally, 1);

    proto
}

/// Create Promise constructor with static methods
pub fn create_promise_constructor(
    space: &mut Space<JsObject>,
    promise_prototype: &JsObjectRef,
) -> JsObjectRef {
    let ctor = create_function(
        space,
        JsFunction::Native(NativeFunction {
            name: "Promise".to_string(),
            func: promise_constructor,
            arity: 1,
        }),
    );

    ctor.borrow_mut().set_property(
        PropertyKey::from("prototype"),
        JsValue::Object(promise_prototype.clone()),
    );

    // Static methods
    register_method(space, &ctor, "resolve", promise_resolve_static, 1);
    register_method(space, &ctor, "reject", promise_reject_static, 1);
    register_method(space, &ctor, "all", promise_all, 1);
    register_method(space, &ctor, "race", promise_race, 1);
    register_method(space, &ctor, "allSettled", promise_allsettled, 1);
    register_method(space, &ctor, "any", promise_any, 1);

    ctor
}

/// Create a new promise object with pending state using the interpreter's GC space
pub fn create_promise_object(interp: &mut Interpreter) -> JsObjectRef {
    let state = Rc::new(RefCell::new(PromiseState {
        status: PromiseStatus::Pending,
        result: None,
        handlers: Vec::new(),
    }));

    let obj = interp.create_object();
    {
        let mut o = obj.borrow_mut();
        o.prototype = Some(interp.promise_prototype.cheap_clone());
        o.exotic = ExoticObject::Promise(state);
    }
    obj
}

/// Create a pending promise using the interpreter's prototype (alias)
pub fn create_promise(interp: &mut Interpreter) -> JsObjectRef {
    create_promise_object(interp)
}

/// Create a fulfilled promise using the interpreter's GC space
pub fn create_fulfilled_promise(interp: &mut Interpreter, value: JsValue) -> JsObjectRef {
    let state = Rc::new(RefCell::new(PromiseState {
        status: PromiseStatus::Fulfilled,
        result: Some(value),
        handlers: Vec::new(),
    }));

    let obj = interp.create_object();
    {
        let mut o = obj.borrow_mut();
        o.prototype = Some(interp.promise_prototype.cheap_clone());
        o.exotic = ExoticObject::Promise(state);
    }
    obj
}

/// Create a rejected promise using the interpreter's GC space
pub fn create_rejected_promise(interp: &mut Interpreter, reason: JsValue) -> JsObjectRef {
    let state = Rc::new(RefCell::new(PromiseState {
        status: PromiseStatus::Rejected,
        result: Some(reason),
        handlers: Vec::new(),
    }));

    let obj = interp.create_object();
    {
        let mut o = obj.borrow_mut();
        o.prototype = Some(interp.promise_prototype.cheap_clone());
        o.exotic = ExoticObject::Promise(state);
    }
    obj
}

/// Public function to resolve a promise (called from PromiseResolve function handling)
pub fn resolve_promise_value(
    interp: &mut Interpreter,
    promise: &JsObjectRef,
    value: JsValue,
) -> Result<(), JsError> {
    resolve_promise(interp, promise, value)
}

/// Public function to reject a promise (called from PromiseReject function handling)
pub fn reject_promise_value(
    interp: &mut Interpreter,
    promise: &JsObjectRef,
    reason: JsValue,
) -> Result<(), JsError> {
    reject_promise(interp, promise, reason)
}

/// Resolve a promise (fulfill or reject based on value)
fn resolve_promise(
    interp: &mut Interpreter,
    promise: &JsObjectRef,
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
    promise: &JsObjectRef,
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
    promise: &JsObjectRef,
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

/// Trigger a promise handler
fn trigger_handler(
    interp: &mut Interpreter,
    handler: PromiseHandler,
    value: &JsValue,
    is_fulfilled: bool,
) -> Result<(), JsError> {
    let callback = if is_fulfilled {
        handler.on_fulfilled.clone()
    } else {
        handler.on_rejected.clone()
    };

    match callback {
        Some(cb) => {
            // Call the callback and resolve result_promise with the return value
            match interp.call_function(cb, JsValue::Undefined, std::slice::from_ref(value)) {
                Ok(result) => {
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
) -> Result<JsValue, JsError> {
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

    let promise = create_promise_object(interp);

    // Create resolve function using the new PromiseResolve variant
    let resolve_fn = interp.create_function(JsFunction::PromiseResolve(promise.cheap_clone()));

    // Create reject function using the new PromiseReject variant
    let reject_fn = interp.create_function(JsFunction::PromiseReject(promise.cheap_clone()));

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

    Ok(JsValue::Object(promise))
}

/// Promise.prototype.then(onFulfilled, onRejected)
pub fn promise_then(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(promise) = this else {
        return Err(JsError::type_error(
            "Promise.prototype.then called on non-object",
        ));
    };

    let on_fulfilled = args.first().cloned();
    let on_rejected = args.get(1).cloned();

    // Filter out non-callable values
    let on_fulfilled = on_fulfilled.filter(|v| v.is_callable());
    let on_rejected = on_rejected.filter(|v| v.is_callable());

    // Create the result promise
    let result_promise = create_promise_object(interp);

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

    Ok(JsValue::Object(result_promise))
}

/// Promise.prototype.catch(onRejected)
pub fn promise_catch(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    // catch(onRejected) is equivalent to then(undefined, onRejected)
    let on_rejected = args.first().cloned().unwrap_or(JsValue::Undefined);
    promise_then(interp, this, &[JsValue::Undefined, on_rejected])
}

/// Promise.prototype.finally(onFinally)
pub fn promise_finally(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(promise) = this.clone() else {
        return Err(JsError::type_error(
            "Promise.prototype.finally called on non-object",
        ));
    };

    let on_finally = args.first().cloned();
    let on_finally = on_finally.filter(|v| v.is_callable());

    match on_finally {
        Some(callback) => {
            // For finally, we need to call the callback but preserve the original value/reason
            // This is tricky without closures. We'll use a simpler approach:
            // Create wrapper functions that call the callback then return/re-throw

            let result_promise = create_promise_object(interp);

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
                    // We'll handle finally specially in trigger_handler_finally
                    let obj = promise.borrow();
                    let ExoticObject::Promise(ref state) = obj.exotic else {
                        return Err(JsError::type_error("Not a promise"));
                    };
                    // Store callback in both slots - we'll call it regardless
                    // JsValue clone for callback - may be cheap or expensive
                    state.borrow_mut().handlers.push(PromiseHandler {
                        on_fulfilled: Some(callback.clone()),
                        on_rejected: Some(callback),
                        result_promise: result_promise.cheap_clone(),
                    });
                    // Mark this as a finally handler somehow... for now, use a simpler approach
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

            Ok(JsValue::Object(result_promise))
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
) -> Result<JsValue, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);

    // If value is already a promise, return it as-is
    if let JsValue::Object(obj) = &value {
        if matches!(obj.borrow().exotic, ExoticObject::Promise(_)) {
            return Ok(value);
        }
    }

    Ok(JsValue::Object(create_fulfilled_promise(interp, value)))
}

/// Promise.reject(reason)
pub fn promise_reject_static(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let reason = args.first().cloned().unwrap_or(JsValue::Undefined);
    Ok(JsValue::Object(create_rejected_promise(interp, reason)))
}

/// Promise.all(iterable)
pub fn promise_all(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let iterable = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Extract array elements
    let promises = extract_iterable(&iterable)?;

    if promises.is_empty() {
        let empty_array = interp.create_array(vec![]);
        return Ok(JsValue::Object(create_fulfilled_promise(
            interp,
            JsValue::Object(empty_array),
        )));
    }

    // For Promise.all, we need to track all results.
    // Since we process synchronously, we can collect results directly.
    let mut results: Vec<JsValue> = Vec::with_capacity(promises.len());
    let mut rejected_reason: Option<JsValue> = None;

    for promise_value in &promises {
        // Convert to promise if not already - for non-promise values, just extract directly
        let (status, result) = if let JsValue::Object(obj) = promise_value {
            let obj_ref = obj.borrow();
            if let ExoticObject::Promise(ref state) = obj_ref.exotic {
                let state_ref = state.borrow();
                (state_ref.status.clone(), state_ref.result.clone())
            } else {
                // Non-promise object - treat as fulfilled with that value
                (PromiseStatus::Fulfilled, Some(promise_value.clone()))
            }
        } else {
            // Primitive value - treat as fulfilled with that value
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
                // In a sync context, pending promises won't resolve
                // For now, treat as undefined
                results.push(JsValue::Undefined);
            }
        }
    }

    if let Some(reason) = rejected_reason {
        return Ok(JsValue::Object(create_rejected_promise(interp, reason)));
    }

    let array = JsValue::Object(interp.create_array(results));
    Ok(JsValue::Object(create_fulfilled_promise(interp, array)))
}

/// Promise.race(iterable)
pub fn promise_race(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let iterable = args.first().cloned().unwrap_or(JsValue::Undefined);
    let promises = extract_iterable(&iterable)?;

    // Race returns the first settled promise
    // First, scan for the first settled promise without creating new promises
    let mut first_result: Option<(PromiseStatus, JsValue)> = None;

    for promise_value in &promises {
        let (status, result) = if let JsValue::Object(obj) = promise_value {
            let obj_ref = obj.borrow();
            if let ExoticObject::Promise(ref state) = obj_ref.exotic {
                let state_ref = state.borrow();
                (state_ref.status.clone(), state_ref.result.clone())
            } else {
                // Non-promise object - treat as fulfilled with that value
                (PromiseStatus::Fulfilled, Some(promise_value.clone()))
            }
        } else {
            // Primitive value - treat as fulfilled with that value
            (PromiseStatus::Fulfilled, Some(promise_value.clone()))
        };

        match status {
            PromiseStatus::Fulfilled | PromiseStatus::Rejected => {
                first_result = Some((status, result.unwrap_or(JsValue::Undefined)));
                break;
            }
            PromiseStatus::Pending => {
                // Continue to next promise
            }
        }
    }

    // Now create the result promise
    match first_result {
        Some((PromiseStatus::Fulfilled, value)) => {
            Ok(JsValue::Object(create_fulfilled_promise(interp, value)))
        }
        Some((PromiseStatus::Rejected, reason)) => {
            Ok(JsValue::Object(create_rejected_promise(interp, reason)))
        }
        _ => {
            // If no promise is settled yet, return a pending promise
            Ok(JsValue::Object(create_promise_object(interp)))
        }
    }
}

/// Promise.allSettled(iterable)
pub fn promise_allsettled(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let iterable = args.first().cloned().unwrap_or(JsValue::Undefined);
    let promises = extract_iterable(&iterable)?;

    if promises.is_empty() {
        let empty_array = interp.create_array(vec![]);
        return Ok(JsValue::Object(create_fulfilled_promise(
            interp,
            JsValue::Object(empty_array),
        )));
    }

    // Collect status info first without borrowing interp
    let mut status_info: Vec<(PromiseStatus, Option<JsValue>)> = Vec::with_capacity(promises.len());

    for promise_value in &promises {
        let (status, result) = if let JsValue::Object(obj) = promise_value {
            let obj_ref = obj.borrow();
            if let ExoticObject::Promise(ref state) = obj_ref.exotic {
                let state_ref = state.borrow();
                (state_ref.status.clone(), state_ref.result.clone())
            } else {
                // Non-promise object - treat as fulfilled with that value
                (PromiseStatus::Fulfilled, Some(promise_value.clone()))
            }
        } else {
            // Primitive value - treat as fulfilled with that value
            (PromiseStatus::Fulfilled, Some(promise_value.clone()))
        };
        status_info.push((status, result));
    }

    // Now create result objects using interp
    let mut results: Vec<JsValue> = Vec::with_capacity(status_info.len());

    for (status, result) in status_info {
        let result_obj = interp.create_object();
        {
            let mut result_ref = result_obj.borrow_mut();
            result_ref.prototype = Some(interp.object_prototype.cheap_clone());

            match status {
                PromiseStatus::Fulfilled => {
                    result_ref.set_property(
                        PropertyKey::from("status"),
                        JsValue::String("fulfilled".into()),
                    );
                    result_ref.set_property(
                        PropertyKey::from("value"),
                        result.unwrap_or(JsValue::Undefined),
                    );
                }
                PromiseStatus::Rejected => {
                    result_ref.set_property(
                        PropertyKey::from("status"),
                        JsValue::String("rejected".into()),
                    );
                    result_ref.set_property(
                        PropertyKey::from("reason"),
                        result.unwrap_or(JsValue::Undefined),
                    );
                }
                PromiseStatus::Pending => {
                    result_ref.set_property(
                        PropertyKey::from("status"),
                        JsValue::String("pending".into()),
                    );
                }
            }
        }
        results.push(JsValue::Object(result_obj));
    }

    let array = JsValue::Object(interp.create_array(results));
    Ok(JsValue::Object(create_fulfilled_promise(interp, array)))
}

/// Promise.any(iterable)
pub fn promise_any(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let iterable = args.first().cloned().unwrap_or(JsValue::Undefined);
    let promises = extract_iterable(&iterable)?;

    if promises.is_empty() {
        // AggregateError with no errors
        return Ok(JsValue::Object(create_rejected_promise(
            interp,
            JsValue::String("All promises were rejected".into()),
        )));
    }

    // Scan for first fulfilled or collect errors
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
                // Non-promise object - treat as fulfilled with that value
                (PromiseStatus::Fulfilled, Some(promise_value.clone()))
            }
        } else {
            // Primitive value - treat as fulfilled with that value
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

    // Now create the result using interp
    if let Some(value) = fulfilled_value {
        return Ok(JsValue::Object(create_fulfilled_promise(interp, value)));
    }

    // All rejected - reject with array of errors
    if !errors.is_empty() && !any_pending {
        let errors_array = JsValue::Object(interp.create_array(errors));
        return Ok(JsValue::Object(create_rejected_promise(
            interp,
            errors_array,
        )));
    }

    // No promise is settled yet (or some pending)
    Ok(JsValue::Object(create_promise_object(interp)))
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
