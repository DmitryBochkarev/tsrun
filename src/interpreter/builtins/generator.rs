//! Generator built-in methods

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object, ExoticObject, GeneratorState, GeneratorStatus, JsFunction,
    JsObjectRef, JsString, JsValue, NativeFunction, PropertyKey,
};

/// Create Generator.prototype
pub fn create_generator_prototype() -> JsObjectRef {
    let proto = create_object();
    {
        let mut p = proto.borrow_mut();

        // Set Symbol.toStringTag
        p.set_property(
            PropertyKey::from("@@toStringTag"),
            JsValue::String(JsString::from("Generator")),
        );

        // Generator.prototype.next
        let next_fn = create_function(JsFunction::Native(NativeFunction {
            name: "next".to_string(),
            func: generator_next,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("next"), JsValue::Object(next_fn));

        // Generator.prototype.return
        let return_fn = create_function(JsFunction::Native(NativeFunction {
            name: "return".to_string(),
            func: generator_return,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("return"), JsValue::Object(return_fn));

        // Generator.prototype.throw
        let throw_fn = create_function(JsFunction::Native(NativeFunction {
            name: "throw".to_string(),
            func: generator_throw,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("throw"), JsValue::Object(throw_fn));
    }
    proto
}

/// Create a generator result object { value, done }
pub fn create_generator_result(value: JsValue, done: bool) -> JsValue {
    let obj = create_object();
    {
        let mut o = obj.borrow_mut();
        o.set_property(PropertyKey::from("value"), value);
        o.set_property(PropertyKey::from("done"), JsValue::Boolean(done));
    }
    JsValue::Object(obj)
}

/// Generator.prototype.next(value)
pub fn generator_next(
    interp: &mut Interpreter,
    this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let JsValue::Object(obj) = this else {
        return Err(JsError::type_error(
            "Generator.prototype.next called on non-object",
        ));
    };

    // Get the generator state
    let gen_state = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Generator(state) => state.clone(),
            _ => {
                return Err(JsError::type_error(
                    "Generator.prototype.next called on non-generator",
                ))
            }
        }
    };

    // Set the sent value
    {
        let mut state = gen_state.borrow_mut();
        state.sent_value = args.first().cloned().unwrap_or(JsValue::Undefined);
    }

    // Resume the generator
    interp.resume_generator(&gen_state)
}

/// Generator.prototype.return(value)
pub fn generator_return(
    _interp: &mut Interpreter,
    this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let JsValue::Object(obj) = this else {
        return Err(JsError::type_error(
            "Generator.prototype.return called on non-object",
        ));
    };

    // Get the generator state
    let gen_state = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Generator(state) => state.clone(),
            _ => {
                return Err(JsError::type_error(
                    "Generator.prototype.return called on non-generator",
                ))
            }
        }
    };

    // Mark as completed and return the value
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    {
        let mut state = gen_state.borrow_mut();
        state.state = GeneratorStatus::Completed;
    }

    Ok(create_generator_result(value, true))
}

/// Generator.prototype.throw(exception)
pub fn generator_throw(
    interp: &mut Interpreter,
    this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let JsValue::Object(obj) = this else {
        return Err(JsError::type_error(
            "Generator.prototype.throw called on non-object",
        ));
    };

    // Get the generator state
    let gen_state = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Generator(state) => state.clone(),
            _ => {
                return Err(JsError::type_error(
                    "Generator.prototype.throw called on non-generator",
                ))
            }
        }
    };

    let exception = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Set exception as sent value and mark for throwing
    {
        let mut state = gen_state.borrow_mut();
        if state.state == GeneratorStatus::Completed {
            // If generator is completed, throw the exception directly
            return Err(JsError::type_error("Generator is already completed"));
        }
        state.sent_value = exception;
    }

    // Resume with throw semantics
    interp.resume_generator_with_throw(&gen_state)
}

/// Create a new generator object from a generator function
pub fn create_generator_object(interp: &Interpreter, state: GeneratorState) -> JsObjectRef {
    let obj = create_object();
    {
        let mut o = obj.borrow_mut();
        o.exotic = ExoticObject::Generator(Rc::new(RefCell::new(state)));
        o.prototype = Some(interp.generator_prototype.clone());
    }
    obj
}
