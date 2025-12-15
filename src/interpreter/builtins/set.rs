//! Set built-in methods

use super::map::same_value_zero;
use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{ExoticObject, Guarded, JsValue, PropertyKey};

/// Initialize Set.prototype with add, has, delete, clear, forEach methods
pub fn init_set_prototype(interp: &mut Interpreter) {
    let proto = interp.set_prototype.clone();

    interp.register_method(&proto, "add", set_add, 1);
    interp.register_method(&proto, "has", set_has, 1);
    interp.register_method(&proto, "delete", set_delete, 1);
    interp.register_method(&proto, "clear", set_clear, 0);
    interp.register_method(&proto, "forEach", set_foreach, 1);
    interp.register_method(&proto, "keys", set_keys, 0);
    interp.register_method(&proto, "values", set_values, 0);
    interp.register_method(&proto, "entries", set_entries, 0);
}

/// Create Set constructor and register it globally
pub fn init_set(interp: &mut Interpreter) {
    init_set_prototype(interp);

    let constructor = interp.create_native_function("Set", set_constructor, 0);
    interp.root_guard.guard(&constructor);

    // Set prototype property on constructor
    let proto_key = interp.key("prototype");
    constructor
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.set_prototype.clone()));

    // Register globally
    let set_key = interp.key("Set");
    interp
        .global
        .borrow_mut()
        .set_property(set_key, JsValue::Object(constructor));
}

pub fn set_constructor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let size_key = interp.key("size");

    let (set_obj, set_guard) = interp.create_object_with_guard();
    {
        let mut obj = set_obj.borrow_mut();
        obj.exotic = ExoticObject::Set {
            entries: Vec::new(),
        };
        obj.prototype = Some(interp.set_prototype.clone());
        obj.set_property(size_key, JsValue::Number(0.0));
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

            let size_key = interp.key("size");
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

    Ok(Guarded::with_guard(JsValue::Object(set_obj), set_guard))
}

pub fn set_add(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
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
    Ok(Guarded::unguarded(this)) // Return the set for chaining
}

pub fn set_has(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
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
                return Ok(Guarded::unguarded(JsValue::Boolean(true)));
            }
        }
    }

    Ok(Guarded::unguarded(JsValue::Boolean(false)))
}

pub fn set_delete(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
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
            return Ok(Guarded::unguarded(JsValue::Boolean(true)));
        }
    }

    Ok(Guarded::unguarded(JsValue::Boolean(false)))
}

pub fn set_clear(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
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

    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn set_foreach(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
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

    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn set_keys(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // For Set, keys() returns the same as values()
    set_values(interp, this, args)
}

pub fn set_values(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
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

    let (arr, guard) = interp.create_array(values);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn set_entries(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
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
    // Build entry arrays
    let mut entries = Vec::with_capacity(raw_entries.len());
    for v in raw_entries {
        let (arr, _guard) = interp.create_array(vec![v.clone(), v]);
        interp.root_guard.guard(&arr);
        entries.push(JsValue::Object(arr));
    }

    let (result, guard) = interp.create_array(entries);
    Ok(Guarded::with_guard(JsValue::Object(result), guard))
}
