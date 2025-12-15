//! Error constructor built-in methods

use crate::error::JsError;
use crate::gc::Gc;
use crate::interpreter::Interpreter;
use crate::value::{Guarded, JsObject, JsString, JsValue, PropertyKey};

/// Initialize Error constructor and add it to globals
pub fn init_error(interp: &mut Interpreter) {
    // Create Error constructor function
    let error_fn = interp.create_native_function("Error", error_constructor, 1);
    interp.root_guard.guard(&error_fn);

    // Create Error.prototype
    let (error_proto, _proto_guard) = interp.create_object_with_guard();
    interp.root_guard.guard(&error_proto);

    // Set default name and message on prototype
    let name_key = interp.key("name");
    let message_key = interp.key("message");
    {
        let mut p = error_proto.borrow_mut();
        p.set_property(name_key, JsValue::String(JsString::from("Error")));
        p.set_property(message_key, JsValue::String(JsString::from("")));
    }

    // Add toString method
    interp.register_method(&error_proto, "toString", error_to_string, 0);

    // Set up prototype chain
    let proto_key = interp.key("prototype");
    error_fn.own(&error_proto, &interp.heap);
    error_fn
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(error_proto));

    // Register Error globally
    let error_key = interp.key("Error");
    interp.global.own(&error_fn, &interp.heap);
    interp
        .global
        .borrow_mut()
        .set_property(error_key, JsValue::Object(error_fn));
}

/// Error.prototype.toString()
/// Returns "name: message" or just "name" if message is empty
pub fn error_to_string(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(obj) = this else {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from("Error"))));
    };

    // Pre-intern keys
    let name_key = interp.key("name");
    let message_key = interp.key("message");

    let obj_ref = obj.borrow();

    // Get name, default to "Error"
    let name = obj_ref
        .get_property(&name_key)
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| "Error".to_string());

    // Get message, default to ""
    let message = obj_ref
        .get_property(&message_key)
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();

    if message.is_empty() {
        Ok(Guarded::unguarded(JsValue::String(JsString::from(name))))
    } else {
        Ok(Guarded::unguarded(JsValue::String(JsString::from(
            format!("{}: {}", name, message),
        ))))
    }
}

/// Initialize error properties on an existing object
fn initialize_error_on_this(obj: &Gc<JsObject>, name: &str, message: JsValue) {
    let msg_str = match &message {
        JsValue::Undefined => JsString::from(""),
        other => other.to_js_string(),
    };

    let mut obj_ref = obj.borrow_mut();
    obj_ref.set_property(
        PropertyKey::from("name"),
        JsValue::String(JsString::from(name)),
    );
    obj_ref.set_property(PropertyKey::from("message"), JsValue::String(msg_str));
}

/// Error constructor - sets name and message on `this`
pub fn error_constructor(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);

    // When called via `new Error()`, this is the newly created object
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(this_obj, "Error", message);
    }

    // Return undefined - new handler will return the created object
    Ok(Guarded::unguarded(JsValue::Undefined))
}
