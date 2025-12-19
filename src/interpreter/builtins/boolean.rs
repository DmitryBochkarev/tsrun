//! Boolean built-in constructor and prototype methods

use crate::error::JsError;
use crate::gc::Gc;
use crate::interpreter::Interpreter;
use crate::value::{ExoticObject, Guarded, JsObject, JsString, JsValue, PropertyKey};

/// Initialize Boolean.prototype with toString, valueOf
pub fn init_boolean_prototype(interp: &mut Interpreter) {
    let proto = interp.boolean_prototype.clone();

    interp.register_method(&proto, "toString", boolean_to_string, 0);
    interp.register_method(&proto, "valueOf", boolean_value_of, 0);
}

/// Boolean constructor function - Boolean(value) converts value to boolean
/// When called without `new`, returns a primitive boolean
/// When called with `new`, returns a Boolean wrapper object
pub fn boolean_constructor_fn(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // Get the boolean value from argument
    let bool_val = args
        .first()
        .cloned()
        .unwrap_or(JsValue::Undefined)
        .to_boolean();

    // Check if called with `new` (this will be a fresh object with Boolean.prototype)
    // When called as a function, `this` is undefined or the global object
    // When called with `new`, `this` is a new object created by evaluate_new
    if let JsValue::Object(obj) = &this {
        // Check if this object was created by the `new` operator
        // by checking if it has boolean_prototype as its prototype
        let is_new_call = {
            let borrowed = obj.borrow();
            if let Some(ref proto) = borrowed.prototype {
                std::ptr::eq(
                    &*proto.borrow() as *const _,
                    &*interp.boolean_prototype.borrow() as *const _,
                )
            } else {
                false
            }
        };

        if is_new_call {
            // Called with `new` - set the internal boolean value to make it a Boolean wrapper
            obj.borrow_mut().exotic = ExoticObject::Boolean(bool_val);
            return Ok(Guarded::unguarded(this));
        }
    }

    // Called as function - return primitive boolean
    Ok(Guarded::unguarded(JsValue::Boolean(bool_val)))
}

/// Create Boolean constructor with prototype property
pub fn create_boolean_constructor(interp: &mut Interpreter) -> Gc<JsObject> {
    let constructor = interp.create_native_function("Boolean", boolean_constructor_fn, 1);

    // Set prototype property
    let proto_key = PropertyKey::String(interp.intern("prototype"));
    constructor
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.boolean_prototype.clone()));

    // Set constructor property on prototype
    let ctor_key = PropertyKey::String(interp.intern("constructor"));
    interp
        .boolean_prototype
        .borrow_mut()
        .set_property(ctor_key, JsValue::Object(constructor.clone()));

    constructor
}

/// Boolean.prototype.toString()
/// Returns "true" or "false" based on the boolean value
pub fn boolean_to_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let bool_val = get_boolean_value(&this)?;
    let result = if bool_val { "true" } else { "false" };
    Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
}

/// Boolean.prototype.valueOf()
/// Returns the primitive boolean value
pub fn boolean_value_of(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let bool_val = get_boolean_value(&this)?;
    Ok(Guarded::unguarded(JsValue::Boolean(bool_val)))
}

/// Helper to extract boolean value from `this`
/// Works for both primitive booleans and Boolean wrapper objects
fn get_boolean_value(this: &JsValue) -> Result<bool, JsError> {
    match this {
        JsValue::Boolean(b) => Ok(*b),
        JsValue::Object(obj) => {
            let borrowed = obj.borrow();
            match borrowed.exotic {
                ExoticObject::Boolean(b) => Ok(b),
                _ => Err(JsError::type_error(
                    "Boolean.prototype method called on incompatible receiver",
                )),
            }
        }
        _ => Err(JsError::type_error(
            "Boolean.prototype method called on incompatible receiver",
        )),
    }
}
