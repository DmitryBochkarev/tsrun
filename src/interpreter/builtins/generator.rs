//! Generator built-in methods

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::JsError;
use crate::gc::Gc;
use crate::interpreter::Interpreter;
use crate::value::{
    BytecodeGeneratorState, CheapClone, ExoticObject, GeneratorState, GeneratorStatus, Guarded,
    JsObject, JsString, JsSymbol, JsValue, PropertyKey,
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

    // Get the generator state (either AST-based or bytecode-based)
    let obj_ref = obj.borrow();
    match &obj_ref.exotic {
        ExoticObject::Generator(state) => {
            let gen_state = state.clone();
            drop(obj_ref); // Release borrow before calling resume_generator

            // Check if generator is already completed
            {
                let state = gen_state.borrow();
                if state.status == GeneratorStatus::Completed {
                    return Ok(create_generator_result(interp, JsValue::Undefined, true));
                }
            }

            // Set the sent value
            {
                let mut state = gen_state.borrow_mut();
                state.sent_value = args.first().cloned().unwrap_or(JsValue::Undefined);
            }

            // Resume the generator
            interp.resume_generator(&gen_state)
        }
        ExoticObject::BytecodeGenerator(state) => {
            let gen_state = state.clone();
            drop(obj_ref); // Release borrow before calling resume_bytecode_generator

            // Check if generator is already completed
            {
                let state = gen_state.borrow();
                if state.status == GeneratorStatus::Completed {
                    return Ok(create_generator_result(interp, JsValue::Undefined, true));
                }
            }

            // Check if we're delegating to another iterator (yield*)
            let delegated = {
                let state = gen_state.borrow();
                state.delegated_iterator.clone()
            };

            if let Some((iter_obj, next_method)) = delegated {
                // Forward next() to the delegated iterator
                let sent_value = args.first().cloned().unwrap_or(JsValue::Undefined);
                let result = interp.call_function(
                    next_method.clone(),
                    JsValue::Object(iter_obj.cheap_clone()),
                    &[sent_value],
                )?;

                // Check if delegated iterator is done
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
            } else {
                // Set the sent value
                {
                    let mut state = gen_state.borrow_mut();
                    state.sent_value = args.first().cloned().unwrap_or(JsValue::Undefined);
                }

                // Resume the bytecode generator
                interp.resume_bytecode_generator(&gen_state)
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

    // Get the generator state (either AST-based or bytecode-based)
    let obj_ref = obj.borrow();
    match &obj_ref.exotic {
        ExoticObject::Generator(state) => {
            let gen_id = state.borrow().id;
            // Clean up saved execution state
            drop(obj_ref);
            interp.saved_generator_states.remove(&gen_id);
            let obj_ref = obj.borrow();
            if let ExoticObject::Generator(state) = &obj_ref.exotic {
                state.borrow_mut().status = GeneratorStatus::Completed;
            }
            drop(obj_ref);
            Ok(create_generator_result(interp, value, true))
        }
        ExoticObject::BytecodeGenerator(state) => {
            state.borrow_mut().status = GeneratorStatus::Completed;
            drop(obj_ref);
            Ok(create_generator_result(interp, value, true))
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

    // Get the generator state (either AST-based or bytecode-based)
    let obj_ref = obj.borrow();
    match &obj_ref.exotic {
        ExoticObject::Generator(state) => {
            let gen_state = state.clone();
            drop(obj_ref);

            // Check if generator is completed
            {
                let state = gen_state.borrow();
                if state.status == GeneratorStatus::Completed {
                    return Err(JsError::ThrownValue { value: exception });
                }
            }

            // Set exception as sent value and mark for throwing
            {
                let mut state = gen_state.borrow_mut();
                state.sent_value = exception;
            }

            // Resume with throw semantics
            interp.resume_generator_with_throw(&gen_state)
        }
        ExoticObject::BytecodeGenerator(state) => {
            // Check if generator is completed
            if state.borrow().status == GeneratorStatus::Completed {
                return Err(JsError::ThrownValue { value: exception });
            }

            // For bytecode generators, throw the exception directly for now
            // Full throw semantics require resuming with an exception
            Err(JsError::ThrownValue { value: exception })
        }
        _ => Err(JsError::type_error(
            "Generator.prototype.throw called on non-generator",
        )),
    }
}

/// Create a new generator object from a generator function
pub fn create_generator_object(interp: &mut Interpreter, state: GeneratorState) -> Gc<JsObject> {
    // Use root_guard for longer-lived generator objects
    let obj = interp.root_guard.alloc();
    {
        let mut o = obj.borrow_mut();
        o.exotic = ExoticObject::Generator(Rc::new(RefCell::new(state)));
        o.prototype = Some(interp.generator_prototype.clone());
    }
    obj
}

/// Create a new bytecode generator object
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
