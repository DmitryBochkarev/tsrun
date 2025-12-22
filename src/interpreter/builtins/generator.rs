//! Generator built-in methods

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::JsError;
use crate::gc::Gc;
use crate::interpreter::Interpreter;
use crate::value::{
    BytecodeGeneratorState, CheapClone, ExoticObject, GeneratorStatus, Guarded, JsObject, JsString,
    JsSymbol, JsValue, PropertyKey,
};

use super::symbol::get_well_known_symbols;

/// Initialize Generator.prototype
pub fn init_generator_prototype(interp: &mut Interpreter) {
    let proto = interp.generator_prototype.clone();

    // Set Symbol.toStringTag
    let tag_key = PropertyKey::String(interp.intern("@@toStringTag"));
    proto
        .borrow_mut()
        .set_property(tag_key, JsValue::String(JsString::from("Generator")));

    interp.register_method(&proto, "next", generator_next, 1);
    interp.register_method(&proto, "return", generator_return, 1);
    interp.register_method(&proto, "throw", generator_throw, 1);

    let well_known = get_well_known_symbols();

    // Add Symbol.iterator - returns the generator itself
    // This makes generators work with for-of loops
    let iterator_symbol = JsSymbol::new(well_known.iterator, Some("Symbol.iterator".to_string()));
    let iterator_key = PropertyKey::Symbol(Box::new(iterator_symbol));

    // Create the [Symbol.iterator]() method that returns `this`
    let iterator_fn = interp.create_native_function("[Symbol.iterator]", generator_iterator, 0);
    proto
        .borrow_mut()
        .set_property(iterator_key, JsValue::Object(iterator_fn));

    // Add Symbol.asyncIterator - returns the generator itself
    // This makes async generators work with for-await-of
    let async_iterator_symbol = JsSymbol::new(
        well_known.async_iterator,
        Some("Symbol.asyncIterator".to_string()),
    );
    let async_iterator_key = PropertyKey::Symbol(Box::new(async_iterator_symbol));

    // Create the [Symbol.asyncIterator]() method that returns `this`
    let async_iterator_fn =
        interp.create_native_function("[Symbol.asyncIterator]", generator_async_iterator, 0);
    proto
        .borrow_mut()
        .set_property(async_iterator_key, JsValue::Object(async_iterator_fn));
}

/// Generator.prototype[Symbol.iterator]() - returns the generator itself
fn generator_iterator(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    // Simply return `this` - the generator is its own iterator
    Ok(Guarded::unguarded(this))
}

/// Generator.prototype[Symbol.asyncIterator]() - returns the generator itself
fn generator_async_iterator(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    // Simply return `this` - the generator is its own async iterator
    Ok(Guarded::unguarded(this))
}

/// Create a generator result object { value, done }
pub fn create_generator_result(interp: &mut Interpreter, value: JsValue, done: bool) -> Guarded {
    let value_key = PropertyKey::String(interp.intern("value"));
    let done_key = PropertyKey::String(interp.intern("done"));

    let guard = interp.heap.create_guard();
    // Guard the value if it's an object to prevent GC
    if let JsValue::Object(ref val_obj) = value {
        guard.guard(val_obj.clone());
    }
    let obj = interp.create_object(&guard);
    {
        let mut o = obj.borrow_mut();
        o.set_property(value_key, value);
        o.set_property(done_key, JsValue::Boolean(done));
    }
    Guarded::with_guard(JsValue::Object(obj), guard)
}

/// Generator.prototype.next(value)
pub fn generator_next(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(obj) = this else {
        return Err(JsError::type_error(
            "Generator.prototype.next called on non-object",
        ));
    };

    // Get the bytecode generator state
    let obj_ref = obj.borrow();
    match &obj_ref.exotic {
        ExoticObject::BytecodeGenerator(state) => {
            let gen_state = state.clone();
            let is_async = gen_state.borrow().is_async;
            drop(obj_ref); // Release borrow before calling resume_bytecode_generator

            // Check if generator is already completed
            {
                let state = gen_state.borrow();
                if state.status == GeneratorStatus::Completed {
                    let result = create_generator_result(interp, JsValue::Undefined, true);
                    return if is_async {
                        wrap_in_fulfilled_promise(interp, result)
                    } else {
                        Ok(result)
                    };
                }
            }

            // Check if we're delegating to another iterator (yield*)
            let delegated = {
                let state = gen_state.borrow();
                state.delegated_iterator.clone()
            };

            if let Some((iter_obj, next_method)) = delegated {
                // Guard the iterator object during the call to prevent GC collection
                let iter_guard = interp.heap.create_guard();
                iter_guard.guard(iter_obj.cheap_clone());

                // Forward next() to the delegated iterator
                let sent_value = args.first().cloned().unwrap_or(JsValue::Undefined);
                let result = interp.call_function(
                    next_method.clone(),
                    JsValue::Object(iter_obj.cheap_clone()),
                    &[sent_value],
                )?;
                // Keep guard alive until after call_function returns
                let _ = iter_guard;

                // For async generators, the delegated iterator might also be async.
                // In that case, result.value is a Promise. We need to check for this
                // and return a Promise that resolves to the proper iterator result.
                if is_async {
                    if let JsValue::Object(result_obj) = &result.value {
                        if matches!(result_obj.borrow().exotic, ExoticObject::Promise(_)) {
                            // The delegated iterator returned a Promise.
                            // We need to wrap this in handling that:
                            // 1. Awaits the promise
                            // 2. Checks if done
                            // 3. Either yields the value or resumes the outer generator
                            //
                            // For now, we return a Promise that resolves to the inner result
                            // and handle the done/value logic in continuation.
                            // This is complex, so we use a simpler approach:
                            // Store that we need to await and handle on next .next() call
                            //
                            // Actually, the cleanest fix is to return the inner promise
                            // wrapped with .then() to handle the done check.
                            // But that requires promise chaining infrastructure.
                            //
                            // Simplest fix: await the promise now using resolve_promise_sync
                            let promise_result =
                                super::promise::resolve_promise_sync(interp, result_obj)?;
                            let (value, done) = interp.extract_iterator_result(&promise_result);

                            if done {
                                // Clear delegation and resume outer generator with the return value
                                gen_state.borrow_mut().delegated_iterator = None;
                                gen_state.borrow_mut().sent_value = value;
                                let result = interp.resume_bytecode_generator(&gen_state)?;
                                wrap_in_fulfilled_promise(interp, result)
                            } else {
                                // Yield the delegated value
                                let result = create_generator_result(interp, value, false);
                                wrap_in_fulfilled_promise(interp, result)
                            }
                        } else {
                            // Result is not a Promise, extract value/done directly
                            let (value, done) = interp.extract_iterator_result(&result.value);

                            if done {
                                gen_state.borrow_mut().delegated_iterator = None;
                                gen_state.borrow_mut().sent_value = value;
                                let result = interp.resume_bytecode_generator(&gen_state)?;
                                wrap_in_fulfilled_promise(interp, result)
                            } else {
                                let result = create_generator_result(interp, value, false);
                                wrap_in_fulfilled_promise(interp, result)
                            }
                        }
                    } else {
                        // Result is not an object, treat as done
                        gen_state.borrow_mut().delegated_iterator = None;
                        gen_state.borrow_mut().sent_value = JsValue::Undefined;
                        let result = interp.resume_bytecode_generator(&gen_state)?;
                        wrap_in_fulfilled_promise(interp, result)
                    }
                } else {
                    // Sync generator - extract value/done directly
                    let (value, done) = interp.extract_iterator_result(&result.value);

                    if done {
                        // Clear delegation and resume outer generator with the return value
                        gen_state.borrow_mut().delegated_iterator = None;
                        gen_state.borrow_mut().sent_value = value;
                        interp.resume_bytecode_generator(&gen_state)
                    } else {
                        // Yield the delegated value
                        Ok(create_generator_result(interp, value, false))
                    }
                }
            } else {
                // Set the sent value
                {
                    let mut state = gen_state.borrow_mut();
                    state.sent_value = args.first().cloned().unwrap_or(JsValue::Undefined);
                }

                // Resume the bytecode generator
                let result = interp.resume_bytecode_generator(&gen_state)?;
                if is_async {
                    wrap_in_fulfilled_promise(interp, result)
                } else {
                    Ok(result)
                }
            }
        }
        _ => Err(JsError::type_error(
            "Generator.prototype.next called on non-generator",
        )),
    }
}

/// Generator.prototype.return(value)
pub fn generator_return(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(obj) = this else {
        return Err(JsError::type_error(
            "Generator.prototype.return called on non-object",
        ));
    };

    let value = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Get the bytecode generator state
    let obj_ref = obj.borrow();
    match &obj_ref.exotic {
        ExoticObject::BytecodeGenerator(state) => {
            let is_async = state.borrow().is_async;
            state.borrow_mut().status = GeneratorStatus::Completed;
            drop(obj_ref);
            let result = create_generator_result(interp, value, true);
            if is_async {
                wrap_in_fulfilled_promise(interp, result)
            } else {
                Ok(result)
            }
        }
        _ => Err(JsError::type_error(
            "Generator.prototype.return called on non-generator",
        )),
    }
}

/// Generator.prototype.throw(exception)
pub fn generator_throw(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(obj) = this else {
        return Err(JsError::type_error(
            "Generator.prototype.throw called on non-object",
        ));
    };

    let exception = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Get the bytecode generator state
    let obj_ref = obj.borrow();
    match &obj_ref.exotic {
        ExoticObject::BytecodeGenerator(state) => {
            let gen_state = state.clone();
            let is_async = gen_state.borrow().is_async;
            drop(obj_ref); // Release borrow before resuming

            // Check if generator is completed or not started
            {
                let state_ref = gen_state.borrow();
                if state_ref.status == GeneratorStatus::Completed {
                    return Err(JsError::ThrownValue { value: exception });
                }
                if !state_ref.started {
                    // Generator hasn't started, just throw
                    drop(state_ref);
                    gen_state.borrow_mut().status = GeneratorStatus::Completed;
                    return Err(JsError::ThrownValue { value: exception });
                }
            }

            // Set the throw value and resume the generator
            // The generator will throw this exception at the current yield point
            gen_state.borrow_mut().throw_value = Some(exception);

            // Resume the generator - it will throw the exception inside
            let result = interp.resume_bytecode_generator(&gen_state)?;
            if is_async {
                wrap_in_fulfilled_promise(interp, result)
            } else {
                Ok(result)
            }
        }
        _ => Err(JsError::type_error(
            "Generator.prototype.throw called on non-generator",
        )),
    }
}

/// Create a new bytecode generator object
// FIXME: accept guard to avoid rooting every time
pub fn create_bytecode_generator_object(
    interp: &mut Interpreter,
    state: BytecodeGeneratorState,
) -> Gc<JsObject> {
    // Use root_guard for longer-lived generator objects
    let obj = interp.root_guard.alloc();
    {
        let mut o = obj.borrow_mut();
        o.exotic = ExoticObject::BytecodeGenerator(Rc::new(RefCell::new(state)));
        o.prototype = Some(interp.generator_prototype.clone());
    }
    obj
}

/// Wrap a generator result (Guarded) in a fulfilled Promise.
/// Used by async generators to return Promise<{value, done}>.
fn wrap_in_fulfilled_promise(
    interp: &mut Interpreter,
    result: Guarded,
) -> Result<Guarded, JsError> {
    use super::promise::create_fulfilled_promise;

    let guard = interp.heap.create_guard();
    // Guard the result value before allocating the promise
    if let JsValue::Object(ref obj) = result.value {
        guard.guard(obj.cheap_clone());
    }

    let promise = create_fulfilled_promise(interp, &guard, result.value);
    Ok(Guarded::with_guard(JsValue::Object(promise), guard))
}
