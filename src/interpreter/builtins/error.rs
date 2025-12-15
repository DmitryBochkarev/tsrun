//! Error constructor built-in methods

use crate::error::JsError;
use crate::gc::Gc;
use crate::interpreter::Interpreter;
use crate::value::{CheapClone, Guarded, JsObject, JsString, JsValue, NativeFn, PropertyKey};

/// Initialize Error and all derived error constructors and add them to globals
pub fn init_error(interp: &mut Interpreter) {
    // Create Error.prototype first (base for all error prototypes)
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

    // Create Error constructor function
    let error_fn = interp.create_native_function("Error", error_constructor, 1);
    interp.root_guard.guard(&error_fn);

    // Set up prototype chain
    let proto_key = interp.key("prototype");
    error_fn
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(error_proto.clone()));

    // Register Error globally
    let error_key = interp.key("Error");
    interp
        .global
        .borrow_mut()
        .set_property(error_key, JsValue::Object(error_fn));

    // Create derived error types
    let derived_errors: &[(&str, NativeFn)] = &[
        ("TypeError", type_error_constructor),
        ("RangeError", range_error_constructor),
        ("ReferenceError", reference_error_constructor),
        ("SyntaxError", syntax_error_constructor),
        ("URIError", uri_error_constructor),
        ("EvalError", eval_error_constructor),
    ];

    for (name, constructor_fn) in derived_errors {
        create_derived_error(interp, name, *constructor_fn, &error_proto);
    }
}

/// Create a derived error type (TypeError, RangeError, etc.)
fn create_derived_error(
    interp: &mut Interpreter,
    name: &str,
    constructor_fn: NativeFn,
    error_proto: &Gc<JsObject>,
) {
    // Create prototype that inherits from Error.prototype
    let (derived_proto, _proto_guard) = interp.create_object_with_guard();
    interp.root_guard.guard(&derived_proto);

    // Set prototype chain to Error.prototype
    derived_proto.borrow_mut().prototype = Some(error_proto.clone());

    // Set name on prototype
    let name_key = interp.key("name");
    let message_key = interp.key("message");
    {
        let mut p = derived_proto.borrow_mut();
        p.set_property(name_key, JsValue::String(JsString::from(name)));
        p.set_property(message_key, JsValue::String(JsString::from("")));
    }

    // Create constructor function
    let constructor = interp.create_native_function(name, constructor_fn, 1);
    interp.root_guard.guard(&constructor);

    // Set up prototype chain
    let proto_key = interp.key("prototype");
    constructor
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(derived_proto));

    // Register globally
    let key = interp.key(name);
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(constructor));
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

    // Build a simple stack trace
    let stack = if msg_str.is_empty() {
        JsString::from(name)
    } else {
        JsString::from(format!("{}: {}", name, msg_str))
    };

    let mut obj_ref = obj.borrow_mut();
    obj_ref.set_property(
        PropertyKey::from("name"),
        JsValue::String(JsString::from(name)),
    );
    obj_ref.set_property(
        PropertyKey::from("message"),
        JsValue::String(msg_str.cheap_clone()),
    );
    obj_ref.set_property(PropertyKey::from("stack"), JsValue::String(stack));
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

/// TypeError constructor
pub fn type_error_constructor(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(this_obj, "TypeError", message);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// RangeError constructor
pub fn range_error_constructor(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(this_obj, "RangeError", message);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// ReferenceError constructor
pub fn reference_error_constructor(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(this_obj, "ReferenceError", message);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// SyntaxError constructor
pub fn syntax_error_constructor(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(this_obj, "SyntaxError", message);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// URIError constructor
pub fn uri_error_constructor(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(this_obj, "URIError", message);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// EvalError constructor
pub fn eval_error_constructor(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(this_obj, "EvalError", message);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}
