//! Generator built-in methods

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{
    create_object, ExoticObject, GeneratorState, GeneratorStatus, JsObjectRef, JsString, JsValue,
};

/// Create Generator.prototype
pub fn create_generator_prototype(interp: &mut Interpreter) -> JsObjectRef {
    let proto = create_object(&mut interp.gc_space);

    // Set Symbol.toStringTag
    let tag_key = interp.key("@@toStringTag");
    proto
        .borrow_mut()
        .set_property(tag_key, JsValue::String(JsString::from("Generator")));

    interp.register_method(&proto, "next", generator_next, 1);
    interp.register_method(&proto, "return", generator_return, 1);
    interp.register_method(&proto, "throw", generator_throw, 1);

    proto
}

/// Create a generator result object { value, done }
pub fn create_generator_result_with_interp(
    interp: &mut Interpreter,
    value: JsValue,
    done: bool,
) -> JsValue {
    // Pre-intern keys
    let value_key = interp.key("value");
    let done_key = interp.key("done");

    let obj = interp.create_object();
    {
        let mut o = obj.borrow_mut();
        o.set_property(value_key, value);
        o.set_property(done_key, JsValue::Boolean(done));
    }
    JsValue::Object(obj)
}

/// Generator.prototype.next(value)
pub fn generator_next(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
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
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
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

    Ok(create_generator_result_with_interp(interp, value, true))
}

/// Generator.prototype.throw(exception)
pub fn generator_throw(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
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
pub fn create_generator_object(interp: &mut Interpreter, state: GeneratorState) -> JsObjectRef {
    let obj = interp.create_object();
    {
        let mut o = obj.borrow_mut();
        o.exotic = ExoticObject::Generator(Rc::new(RefCell::new(state)));
        o.prototype = Some(interp.generator_prototype.clone());
    }
    obj
}
