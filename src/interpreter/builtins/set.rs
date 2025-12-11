//! Set built-in methods

use super::map::same_value_zero;
use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object, ExoticObject, JsFunction, JsObjectRef, JsValue, NativeFunction,
    PropertyKey,
};

/// Create Set.prototype with add, has, delete, clear, forEach methods
pub fn create_set_prototype(interp: &mut Interpreter) -> JsObjectRef {
    let proto = create_object(&mut interp.gc_space);

    interp.register_method(&proto, "add", set_add, 1);
    interp.register_method(&proto, "has", set_has, 1);
    interp.register_method(&proto, "delete", set_delete, 1);
    interp.register_method(&proto, "clear", set_clear, 0);
    interp.register_method(&proto, "forEach", set_foreach, 1);
    interp.register_method(&proto, "keys", set_keys, 0);
    interp.register_method(&proto, "values", set_values, 0);
    interp.register_method(&proto, "entries", set_entries, 0);

    proto
}

/// Create Set constructor
pub fn create_set_constructor(interp: &mut Interpreter) -> JsObjectRef {
    let name = interp.intern("Set");
    create_function(
        &mut interp.gc_space,
        &mut interp.string_dict,
        JsFunction::Native(NativeFunction {
            name,
            func: set_constructor,
            arity: 0,
        }),
    )
}

pub fn set_constructor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let size_key = interp.key("size");

    let set_obj = interp.create_object();
    {
        let mut obj = set_obj.borrow_mut();
        obj.exotic = ExoticObject::Set {
            entries: Vec::new(),
        };
        obj.prototype = Some(interp.set_prototype.clone());
        obj.set_property(size_key.clone(), JsValue::Number(0.0));
    }

    // If an iterable (array) is passed, add its elements
    if let Some(JsValue::Object(arr)) = args.first() {
        let arr_ref = arr.borrow();
        if let ExoticObject::Array { length } = arr_ref.exotic {
            let mut items = Vec::new();
            for i in 0..length {
                if let Some(value) = arr_ref.get_property(&PropertyKey::Index(i)) {
                    items.push(value);
                }
            }
            drop(arr_ref);

            let mut set = set_obj.borrow_mut();
            if let ExoticObject::Set { ref mut entries } = set.exotic {
                for value in items {
                    // Only add if not already present
                    let exists = entries.iter().any(|e| same_value_zero(e, &value));
                    if !exists {
                        entries.push(value);
                    }
                }
                let len = entries.len();
                set.set_property(size_key, JsValue::Number(len as f64));
            }
        }
    }

    Ok(JsValue::Object(set_obj))
}

pub fn set_add(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(set_obj) = this.clone() else {
        return Err(JsError::type_error(
            "Set.prototype.add called on non-object",
        ));
    };

    let size_key = interp.key("size");

    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    let mut set = set_obj.borrow_mut();

    if let ExoticObject::Set { ref mut entries } = set.exotic {
        // Only add if not already present
        let exists = entries.iter().any(|e| same_value_zero(e, &value));
        if !exists {
            entries.push(value);
            let len = entries.len();
            set.set_property(size_key, JsValue::Number(len as f64));
        }
    }

    drop(set);
    Ok(this) // Return the set for chaining
}

pub fn set_has(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(set_obj) = this else {
        return Err(JsError::type_error(
            "Set.prototype.has called on non-object",
        ));
    };

    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    let set = set_obj.borrow();

    if let ExoticObject::Set { ref entries } = set.exotic {
        for e in entries {
            if same_value_zero(e, &value) {
                return Ok(JsValue::Boolean(true));
            }
        }
    }

    Ok(JsValue::Boolean(false))
}

pub fn set_delete(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(set_obj) = this else {
        return Err(JsError::type_error(
            "Set.prototype.delete called on non-object",
        ));
    };

    let size_key = interp.key("size");

    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    let mut set = set_obj.borrow_mut();

    if let ExoticObject::Set { ref mut entries } = set.exotic {
        if let Some(i) = entries.iter().position(|v| same_value_zero(v, &value)) {
            entries.remove(i);
            let len = entries.len();
            set.set_property(size_key, JsValue::Number(len as f64));
            return Ok(JsValue::Boolean(true));
        }
    }

    Ok(JsValue::Boolean(false))
}

pub fn set_clear(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(set_obj) = this else {
        return Err(JsError::type_error(
            "Set.prototype.clear called on non-object",
        ));
    };

    let size_key = interp.key("size");

    let mut set = set_obj.borrow_mut();

    if let ExoticObject::Set { ref mut entries } = set.exotic {
        entries.clear();
        set.set_property(size_key, JsValue::Number(0.0));
    }

    Ok(JsValue::Undefined)
}

pub fn set_foreach(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(set_obj) = this.clone() else {
        return Err(JsError::type_error(
            "Set.prototype.forEach called on non-object",
        ));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Collect entries first to avoid borrow issues
    let entries: Vec<JsValue>;
    {
        let set = set_obj.borrow();
        if let ExoticObject::Set { entries: ref e } = set.exotic {
            entries = e.clone();
        } else {
            return Err(JsError::type_error(
                "Set.prototype.forEach called on non-Set",
            ));
        }
    }

    for value in entries {
        // Set.forEach passes (value, value, set) - value is passed twice
        interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[value.clone(), value, this.clone()],
        )?;
    }

    Ok(JsValue::Undefined)
}

pub fn set_keys(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    // For Set, keys() returns the same as values()
    set_values(interp, this, _args)
}

pub fn set_values(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(set_obj) = this else {
        return Err(JsError::type_error(
            "Set.prototype.values called on non-object",
        ));
    };

    let values: Vec<JsValue>;
    {
        let set = set_obj.borrow();
        if let ExoticObject::Set { entries: ref e } = set.exotic {
            values = e.clone();
        } else {
            return Err(JsError::type_error(
                "Set.prototype.values called on non-Set",
            ));
        }
    }

    Ok(JsValue::Object(interp.create_array(values)))
}

pub fn set_entries(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(set_obj) = this else {
        return Err(JsError::type_error(
            "Set.prototype.entries called on non-object",
        ));
    };

    let raw_entries: Vec<JsValue>;
    {
        let set = set_obj.borrow();
        if let ExoticObject::Set { entries: ref e } = set.exotic {
            raw_entries = e.clone();
        } else {
            return Err(JsError::type_error(
                "Set.prototype.entries called on non-Set",
            ));
        }
    }

    // For Set, entries returns [value, value] pairs
    let entries: Vec<JsValue> = raw_entries
        .into_iter()
        .map(|v| JsValue::Object(interp.create_array(vec![v.clone(), v])))
        .collect();

    Ok(JsValue::Object(interp.create_array(entries)))
}
