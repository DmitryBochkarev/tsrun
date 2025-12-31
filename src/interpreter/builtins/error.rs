//! Error constructor built-in methods

use crate::error::JsError;
use crate::gc::{Gc, Guard};
use crate::interpreter::Interpreter;
use crate::value::{Guarded, JsObject, JsString, JsValue, PropertyKey};

/// Initialize Error and all derived error constructors and add them to globals
pub fn init_error(interp: &mut Interpreter) {
    // Use the pre-created error prototypes from the Interpreter
    let error_proto = interp.error_prototype.clone();
    let type_error_proto = interp.type_error_prototype.clone();
    let reference_error_proto = interp.reference_error_prototype.clone();
    let range_error_proto = interp.range_error_prototype.clone();
    let syntax_error_proto = interp.syntax_error_prototype.clone();

    // Pre-intern all property keys
    let name_key = PropertyKey::String(interp.intern("name"));
    let message_key = PropertyKey::String(interp.intern("message"));
    let proto_key = PropertyKey::String(interp.intern("prototype"));
    let constructor_key = PropertyKey::String(interp.intern("constructor"));
    let error_key = PropertyKey::String(interp.intern("Error"));
    let type_error_key = PropertyKey::String(interp.intern("TypeError"));
    let reference_error_key = PropertyKey::String(interp.intern("ReferenceError"));
    let range_error_key = PropertyKey::String(interp.intern("RangeError"));
    let syntax_error_key = PropertyKey::String(interp.intern("SyntaxError"));
    let uri_error_key = PropertyKey::String(interp.intern("URIError"));
    let eval_error_key = PropertyKey::String(interp.intern("EvalError"));
    {
        let mut p = error_proto.borrow_mut();
        p.set_property(name_key.clone(), JsValue::String(JsString::from("Error")));
        p.set_property(message_key.clone(), JsValue::String(JsString::from("")));
    }

    // Add toString method to Error.prototype
    interp.register_method(&error_proto, "toString", error_to_string, 0);

    // Create Error constructor function
    let error_fn = interp.create_native_function("Error", error_constructor, 1);
    interp.root_guard.guard(error_fn.clone());

    // Set up prototype chain
    error_fn
        .borrow_mut()
        .set_property(proto_key.clone(), JsValue::Object(error_proto.clone()));

    // Set constructor property on Error.prototype
    error_proto
        .borrow_mut()
        .set_property(constructor_key.clone(), JsValue::Object(error_fn.clone()));

    // Register Error globally
    interp
        .global
        .borrow_mut()
        .set_property(error_key, JsValue::Object(error_fn));

    // Initialize derived error types using pre-created prototypes
    // TypeError
    {
        let mut p = type_error_proto.borrow_mut();
        p.set_property(
            name_key.clone(),
            JsValue::String(JsString::from("TypeError")),
        );
        p.set_property(message_key.clone(), JsValue::String(JsString::from("")));
    }
    let type_error_fn = interp.create_native_function("TypeError", type_error_constructor, 1);
    interp.root_guard.guard(type_error_fn.clone());
    type_error_fn
        .borrow_mut()
        .set_property(proto_key.clone(), JsValue::Object(type_error_proto.clone()));
    type_error_proto.borrow_mut().set_property(
        constructor_key.clone(),
        JsValue::Object(type_error_fn.clone()),
    );
    interp
        .global
        .borrow_mut()
        .set_property(type_error_key, JsValue::Object(type_error_fn));

    // ReferenceError
    {
        let mut p = reference_error_proto.borrow_mut();
        p.set_property(
            name_key.clone(),
            JsValue::String(JsString::from("ReferenceError")),
        );
        p.set_property(message_key.clone(), JsValue::String(JsString::from("")));
    }
    let reference_error_fn =
        interp.create_native_function("ReferenceError", reference_error_constructor, 1);
    interp.root_guard.guard(reference_error_fn.clone());
    reference_error_fn.borrow_mut().set_property(
        proto_key.clone(),
        JsValue::Object(reference_error_proto.clone()),
    );
    reference_error_proto.borrow_mut().set_property(
        constructor_key.clone(),
        JsValue::Object(reference_error_fn.clone()),
    );
    interp
        .global
        .borrow_mut()
        .set_property(reference_error_key, JsValue::Object(reference_error_fn));

    // RangeError
    {
        let mut p = range_error_proto.borrow_mut();
        p.set_property(
            name_key.clone(),
            JsValue::String(JsString::from("RangeError")),
        );
        p.set_property(message_key.clone(), JsValue::String(JsString::from("")));
    }
    let range_error_fn = interp.create_native_function("RangeError", range_error_constructor, 1);
    interp.root_guard.guard(range_error_fn.clone());
    range_error_fn.borrow_mut().set_property(
        proto_key.clone(),
        JsValue::Object(range_error_proto.clone()),
    );
    range_error_proto.borrow_mut().set_property(
        constructor_key.clone(),
        JsValue::Object(range_error_fn.clone()),
    );
    interp
        .global
        .borrow_mut()
        .set_property(range_error_key, JsValue::Object(range_error_fn));

    // SyntaxError
    {
        let mut p = syntax_error_proto.borrow_mut();
        p.set_property(
            name_key.clone(),
            JsValue::String(JsString::from("SyntaxError")),
        );
        p.set_property(message_key.clone(), JsValue::String(JsString::from("")));
    }
    let syntax_error_fn = interp.create_native_function("SyntaxError", syntax_error_constructor, 1);
    interp.root_guard.guard(syntax_error_fn.clone());
    syntax_error_fn.borrow_mut().set_property(
        proto_key.clone(),
        JsValue::Object(syntax_error_proto.clone()),
    );
    syntax_error_proto.borrow_mut().set_property(
        constructor_key.clone(),
        JsValue::Object(syntax_error_fn.clone()),
    );
    interp
        .global
        .borrow_mut()
        .set_property(syntax_error_key, JsValue::Object(syntax_error_fn));

    // URIError (uses error_prototype since we don't have a dedicated prototype)
    let uri_error_proto = interp.root_guard.alloc();
    uri_error_proto.borrow_mut().prototype = Some(interp.error_prototype.clone());
    {
        let mut p = uri_error_proto.borrow_mut();
        p.set_property(
            name_key.clone(),
            JsValue::String(JsString::from("URIError")),
        );
        p.set_property(message_key.clone(), JsValue::String(JsString::from("")));
    }
    let uri_error_fn = interp.create_native_function("URIError", uri_error_constructor, 1);
    interp.root_guard.guard(uri_error_fn.clone());
    uri_error_fn
        .borrow_mut()
        .set_property(proto_key.clone(), JsValue::Object(uri_error_proto.clone()));
    uri_error_proto.borrow_mut().set_property(
        constructor_key.clone(),
        JsValue::Object(uri_error_fn.clone()),
    );
    interp
        .global
        .borrow_mut()
        .set_property(uri_error_key, JsValue::Object(uri_error_fn));

    // EvalError (uses error_prototype since we don't have a dedicated prototype)
    let eval_error_proto = interp.root_guard.alloc();
    eval_error_proto.borrow_mut().prototype = Some(interp.error_prototype.clone());
    {
        let mut p = eval_error_proto.borrow_mut();
        p.set_property(name_key, JsValue::String(JsString::from("EvalError")));
        p.set_property(message_key, JsValue::String(JsString::from("")));
    }
    let eval_error_fn = interp.create_native_function("EvalError", eval_error_constructor, 1);
    interp.root_guard.guard(eval_error_fn.clone());
    eval_error_fn
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(eval_error_proto.clone()));
    eval_error_proto
        .borrow_mut()
        .set_property(constructor_key, JsValue::Object(eval_error_fn.clone()));
    interp
        .global
        .borrow_mut()
        .set_property(eval_error_key, JsValue::Object(eval_error_fn));
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
    let name_key = interp.property_key("name");
    let message_key = interp.property_key("message");

    let obj_ref = obj.borrow();
    let name_val = obj_ref.get_property(&name_key);
    let message_val = obj_ref.get_property(&message_key);
    drop(obj_ref);

    // Get name, default to "Error"
    let name = match name_val {
        Some(v) => interp.to_js_string(&v).to_string(),
        None => "Error".to_string(),
    };

    // Get message, default to ""
    let message = match message_val {
        Some(v) => interp.to_js_string(&v).to_string(),
        None => String::new(),
    };

    if message.is_empty() {
        Ok(Guarded::unguarded(JsValue::String(JsString::from(name))))
    } else {
        Ok(Guarded::unguarded(JsValue::String(JsString::from(
            format!("{}: {}", name, message),
        ))))
    }
}

/// Initialize error properties on an existing object
fn initialize_error_on_this(
    interp: &mut Interpreter,
    obj: &Gc<JsObject>,
    name: &str,
    message: JsValue,
) {
    let msg_str = match &message {
        JsValue::Undefined => interp.intern(""),
        other => interp.to_js_string(other),
    };

    // Build a simple stack trace
    let stack = if msg_str.is_empty() {
        JsString::from(name)
    } else {
        JsString::from(format!("{}: {}", name, msg_str))
    };

    let name_key = interp.property_key("name");
    let message_key = interp.property_key("message");
    let stack_key = interp.property_key("stack");

    let mut obj_ref = obj.borrow_mut();
    obj_ref.set_property(name_key, JsValue::String(JsString::from(name)));
    obj_ref.set_property(message_key, JsValue::String(msg_str.clone()));
    obj_ref.set_property(stack_key, JsValue::String(stack));
}

/// Error constructor - sets name and message on `this`
pub fn error_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);

    // When called via `new Error()`, this is the newly created object
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(interp, this_obj, "Error", message);
    }

    // Return undefined - new handler will return the created object
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// TypeError constructor
pub fn type_error_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(interp, this_obj, "TypeError", message);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// RangeError constructor
pub fn range_error_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(interp, this_obj, "RangeError", message);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// ReferenceError constructor
pub fn reference_error_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(interp, this_obj, "ReferenceError", message);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// SyntaxError constructor
pub fn syntax_error_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(interp, this_obj, "SyntaxError", message);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// URIError constructor
pub fn uri_error_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(interp, this_obj, "URIError", message);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// EvalError constructor
pub fn eval_error_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(interp, this_obj, "EvalError", message);
    }
    Ok(Guarded::unguarded(JsValue::Undefined))
}

/// Create an error object from a JsError
/// Returns the error object and a guard to keep it alive
pub fn create_error_object(
    interp: &mut Interpreter,
    error: &JsError,
) -> (JsValue, Option<Guard<JsObject>>) {
    let guard = interp.heap.create_guard();

    let (prototype, name, message) = match error {
        JsError::TypeError { message, .. } => (
            interp.type_error_prototype.clone(),
            "TypeError",
            message.clone(),
        ),
        JsError::ReferenceError { name } => (
            interp.reference_error_prototype.clone(),
            "ReferenceError",
            format!("{} is not defined", name),
        ),
        JsError::RangeError { message } => (
            interp.range_error_prototype.clone(),
            "RangeError",
            message.clone(),
        ),
        JsError::SyntaxError { message, location } => (
            interp.syntax_error_prototype.clone(),
            "SyntaxError",
            format!("{} at {}", message, location),
        ),
        JsError::RuntimeError { kind, message, .. } => {
            // Map to appropriate prototype based on kind
            let proto = match kind.as_str() {
                "TypeError" => interp.type_error_prototype.clone(),
                "ReferenceError" => interp.reference_error_prototype.clone(),
                "RangeError" => interp.range_error_prototype.clone(),
                "SyntaxError" => interp.syntax_error_prototype.clone(),
                _ => interp.error_prototype.clone(),
            };
            (proto, kind.as_str(), message.clone())
        }
        JsError::ModuleError { message } => {
            (interp.error_prototype.clone(), "Error", message.clone())
        }
        JsError::Internal(msg) => (interp.error_prototype.clone(), "Error", msg.clone()),
        // These should not reach here, but handle them anyway
        JsError::Thrown
        | JsError::ThrownValue { .. }
        | JsError::GeneratorYield { .. }
        | JsError::OptionalChainShortCircuit => {
            return (JsValue::Undefined, None);
        }
    };

    // Create the error object
    let error_obj = guard.alloc();
    error_obj.borrow_mut().prototype = Some(prototype);

    // Set name, message, and stack properties
    let msg_str = JsString::from(message.as_str());
    let stack_str = if msg_str.is_empty() {
        JsString::from(name)
    } else {
        JsString::from(format!("{}: {}", name, msg_str))
    };

    let name_key = interp.property_key("name");
    let message_key = interp.property_key("message");
    let stack_key = interp.property_key("stack");

    {
        let mut obj = error_obj.borrow_mut();
        obj.set_property(name_key, JsValue::String(JsString::from(name)));
        obj.set_property(message_key, JsValue::String(msg_str));
        obj.set_property(stack_key, JsValue::String(stack_str));
    }

    (JsValue::Object(error_obj), Some(guard))
}
