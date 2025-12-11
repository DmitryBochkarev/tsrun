//! Error constructor built-in methods

use crate::error::JsError;
use crate::gc::Space;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object, register_method, JsFunction, JsObject, JsObjectRef, JsString,
    JsValue, NativeFunction, PropertyKey,
};

/// Create Error.prototype with toString
pub fn create_error_prototype(space: &mut Space<JsObject>) -> JsObjectRef {
    let proto = create_object(space);

    // Set default name and message
    proto.borrow_mut().set_property(
        PropertyKey::from("name"),
        JsValue::String(JsString::from("Error")),
    );
    proto.borrow_mut().set_property(
        PropertyKey::from("message"),
        JsValue::String(JsString::from("")),
    );

    register_method(space, &proto, "toString", error_to_string, 0);

    proto
}

/// Error constructors tuple type
pub struct ErrorConstructors {
    pub error: JsObjectRef,
    pub type_error: JsObjectRef,
    pub reference_error: JsObjectRef,
    pub syntax_error: JsObjectRef,
    pub range_error: JsObjectRef,
    pub uri_error: JsObjectRef,
    pub eval_error: JsObjectRef,
}

/// Helper to create an error subtype prototype that inherits from Error.prototype
fn create_error_subtype_prototype(
    space: &mut Space<JsObject>,
    error_prototype: &JsObjectRef,
    name: &str,
) -> JsObjectRef {
    let proto = create_object(space);
    {
        let mut p = proto.borrow_mut();
        p.prototype = Some(error_prototype.clone());
        p.set_property(
            PropertyKey::from("name"),
            JsValue::String(JsString::from(name)),
        );
    }
    proto
}

/// Returns error constructors and the Error.prototype
pub fn create_error_constructors(
    space: &mut Space<JsObject>,
    error_prototype: &JsObjectRef,
) -> ErrorConstructors {
    let error_fn = create_function(
        space,
        JsFunction::Native(NativeFunction {
            name: "Error".to_string(),
            func: error_constructor,
            arity: 1,
        }),
    );
    error_fn.borrow_mut().set_property(
        PropertyKey::from("prototype"),
        JsValue::Object(error_prototype.clone()),
    );

    // Create separate prototypes for each error type that inherit from Error.prototype
    let type_error_proto = create_error_subtype_prototype(space, error_prototype, "TypeError");
    let type_error_fn = create_function(
        space,
        JsFunction::Native(NativeFunction {
            name: "TypeError".to_string(),
            func: type_error_constructor,
            arity: 1,
        }),
    );
    type_error_fn.borrow_mut().set_property(
        PropertyKey::from("prototype"),
        JsValue::Object(type_error_proto),
    );

    let reference_error_proto =
        create_error_subtype_prototype(space, error_prototype, "ReferenceError");
    let reference_error_fn = create_function(
        space,
        JsFunction::Native(NativeFunction {
            name: "ReferenceError".to_string(),
            func: reference_error_constructor,
            arity: 1,
        }),
    );
    reference_error_fn.borrow_mut().set_property(
        PropertyKey::from("prototype"),
        JsValue::Object(reference_error_proto),
    );

    let syntax_error_proto = create_error_subtype_prototype(space, error_prototype, "SyntaxError");
    let syntax_error_fn = create_function(
        space,
        JsFunction::Native(NativeFunction {
            name: "SyntaxError".to_string(),
            func: syntax_error_constructor,
            arity: 1,
        }),
    );
    syntax_error_fn.borrow_mut().set_property(
        PropertyKey::from("prototype"),
        JsValue::Object(syntax_error_proto),
    );

    let range_error_proto = create_error_subtype_prototype(space, error_prototype, "RangeError");
    let range_error_fn = create_function(
        space,
        JsFunction::Native(NativeFunction {
            name: "RangeError".to_string(),
            func: range_error_constructor,
            arity: 1,
        }),
    );
    range_error_fn.borrow_mut().set_property(
        PropertyKey::from("prototype"),
        JsValue::Object(range_error_proto),
    );

    let uri_error_proto = create_error_subtype_prototype(space, error_prototype, "URIError");
    let uri_error_fn = create_function(
        space,
        JsFunction::Native(NativeFunction {
            name: "URIError".to_string(),
            func: uri_error_constructor,
            arity: 1,
        }),
    );
    uri_error_fn.borrow_mut().set_property(
        PropertyKey::from("prototype"),
        JsValue::Object(uri_error_proto),
    );

    let eval_error_proto = create_error_subtype_prototype(space, error_prototype, "EvalError");
    let eval_error_fn = create_function(
        space,
        JsFunction::Native(NativeFunction {
            name: "EvalError".to_string(),
            func: eval_error_constructor,
            arity: 1,
        }),
    );
    eval_error_fn.borrow_mut().set_property(
        PropertyKey::from("prototype"),
        JsValue::Object(eval_error_proto),
    );

    ErrorConstructors {
        error: error_fn,
        type_error: type_error_fn,
        reference_error: reference_error_fn,
        syntax_error: syntax_error_fn,
        range_error: range_error_fn,
        uri_error: uri_error_fn,
        eval_error: eval_error_fn,
    }
}

/// Error.prototype.toString()
/// Returns "name: message" or just "name" if message is empty
pub fn error_to_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(obj) = this else {
        return Ok(JsValue::String(JsString::from("Error")));
    };

    let obj_ref = obj.borrow();

    // Get name, default to "Error"
    let name = obj_ref
        .get_property(&PropertyKey::from("name"))
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| "Error".to_string());

    // Get message, default to ""
    let message = obj_ref
        .get_property(&PropertyKey::from("message"))
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();

    if message.is_empty() {
        Ok(JsValue::String(JsString::from(name)))
    } else {
        Ok(JsValue::String(JsString::from(format!(
            "{}: {}",
            name, message
        ))))
    }
}

fn create_error_object_with_stack(
    interp: &mut Interpreter,
    name: &str,
    message: JsValue,
    prototype: Option<JsObjectRef>,
) -> JsValue {
    let msg_str = match &message {
        JsValue::Undefined => JsString::from(""),
        other => other.to_js_string(),
    };

    // Capture stack trace
    let stack_trace = interp.format_stack_trace(name, msg_str.as_ref());

    let obj = interp.create_object();
    {
        let mut obj_ref = obj.borrow_mut();
        obj_ref.set_property(
            PropertyKey::from("name"),
            JsValue::String(JsString::from(name)),
        );
        obj_ref.set_property(PropertyKey::from("message"), JsValue::String(msg_str));
        obj_ref.set_property(
            PropertyKey::from("stack"),
            JsValue::String(JsString::from(stack_trace)),
        );
        if let Some(proto) = prototype {
            obj_ref.prototype = Some(proto);
        }
    }
    JsValue::Object(obj)
}

/// Initialize error properties on an existing object (for subclass support via super())
fn initialize_error_on_this(
    interp: &Interpreter,
    this: &JsObjectRef,
    name: &str,
    message: JsValue,
) {
    let msg_str = match &message {
        JsValue::Undefined => JsString::from(""),
        other => other.to_js_string(),
    };

    // Capture stack trace
    let stack_trace = interp.format_stack_trace(name, msg_str.as_ref());

    let mut obj_ref = this.borrow_mut();
    obj_ref.set_property(
        PropertyKey::from("name"),
        JsValue::String(JsString::from(name)),
    );
    obj_ref.set_property(PropertyKey::from("message"), JsValue::String(msg_str));
    obj_ref.set_property(
        PropertyKey::from("stack"),
        JsValue::String(JsString::from(stack_trace)),
    );
}

pub fn error_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);

    // If called via super() from a subclass, this will be an object
    // In that case, initialize properties on this instead of creating new object
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(interp, this_obj, "Error", message);
        Ok(this)
    } else {
        Ok(create_error_object_with_stack(
            interp,
            "Error",
            message,
            Some(interp.error_prototype.clone()),
        ))
    }
}

pub fn type_error_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(interp, this_obj, "TypeError", message);
        Ok(this)
    } else {
        Ok(create_error_object_with_stack(
            interp,
            "TypeError",
            message,
            Some(interp.error_prototype.clone()),
        ))
    }
}

pub fn reference_error_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(interp, this_obj, "ReferenceError", message);
        Ok(this)
    } else {
        Ok(create_error_object_with_stack(
            interp,
            "ReferenceError",
            message,
            Some(interp.error_prototype.clone()),
        ))
    }
}

pub fn syntax_error_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(interp, this_obj, "SyntaxError", message);
        Ok(this)
    } else {
        Ok(create_error_object_with_stack(
            interp,
            "SyntaxError",
            message,
            Some(interp.error_prototype.clone()),
        ))
    }
}

pub fn range_error_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(interp, this_obj, "RangeError", message);
        Ok(this)
    } else {
        Ok(create_error_object_with_stack(
            interp,
            "RangeError",
            message,
            Some(interp.error_prototype.clone()),
        ))
    }
}

pub fn uri_error_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(interp, this_obj, "URIError", message);
        Ok(this)
    } else {
        Ok(create_error_object_with_stack(
            interp,
            "URIError",
            message,
            Some(interp.error_prototype.clone()),
        ))
    }
}

pub fn eval_error_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let message = args.first().cloned().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(ref this_obj) = this {
        initialize_error_on_this(interp, this_obj, "EvalError", message);
        Ok(this)
    } else {
        Ok(create_error_object_with_stack(
            interp,
            "EvalError",
            message,
            Some(interp.error_prototype.clone()),
        ))
    }
}
