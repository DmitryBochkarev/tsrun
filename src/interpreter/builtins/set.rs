//! Set built-in methods

use super::map::same_value_zero;
use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object, ExoticObject, JsFunction, JsObjectRef, JsValue, NativeFunction,
    PropertyKey,
};

/// Create Set.prototype with add, has, delete, clear, forEach methods
pub fn create_set_prototype() -> JsObjectRef {
    let proto = create_object();
    {
        let mut p = proto.borrow_mut();

        let add_fn = create_function(JsFunction::Native(NativeFunction {
            name: "add".to_string(),
            func: set_add,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("add"), JsValue::Object(add_fn));

        let has_fn = create_function(JsFunction::Native(NativeFunction {
            name: "has".to_string(),
            func: set_has,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("has"), JsValue::Object(has_fn));

        let delete_fn = create_function(JsFunction::Native(NativeFunction {
            name: "delete".to_string(),
            func: set_delete,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("delete"), JsValue::Object(delete_fn));

        let clear_fn = create_function(JsFunction::Native(NativeFunction {
            name: "clear".to_string(),
            func: set_clear,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("clear"), JsValue::Object(clear_fn));

        let foreach_fn = create_function(JsFunction::Native(NativeFunction {
            name: "forEach".to_string(),
            func: set_foreach,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("forEach"), JsValue::Object(foreach_fn));

        let keys_fn = create_function(JsFunction::Native(NativeFunction {
            name: "keys".to_string(),
            func: set_keys,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("keys"), JsValue::Object(keys_fn));

        let values_fn = create_function(JsFunction::Native(NativeFunction {
            name: "values".to_string(),
            func: set_values,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("values"), JsValue::Object(values_fn));

        let entries_fn = create_function(JsFunction::Native(NativeFunction {
            name: "entries".to_string(),
            func: set_entries,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("entries"), JsValue::Object(entries_fn));
    }
    proto
}

/// Create Set constructor
pub fn create_set_constructor() -> JsObjectRef {
    create_function(JsFunction::Native(NativeFunction {
        name: "Set".to_string(),
        func: set_constructor,
        arity: 0,
    }))
}

pub fn set_constructor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let set_obj = create_object();
    {
        let mut obj = set_obj.borrow_mut();
        obj.exotic = ExoticObject::Set {
            entries: Vec::new(),
        };
        obj.prototype = Some(interp.set_prototype.clone());
        obj.set_property(PropertyKey::from("size"), JsValue::Number(0.0));
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
                set.set_property(PropertyKey::from("size"), JsValue::Number(len as f64));
            }
        }
    }

    Ok(JsValue::Object(set_obj))
}

pub fn set_add(
    _interp: &mut Interpreter,
    this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let JsValue::Object(set_obj) = this.clone() else {
        return Err(JsError::type_error(
            "Set.prototype.add called on non-object",
        ));
    };

    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    let mut set = set_obj.borrow_mut();

    if let ExoticObject::Set { ref mut entries } = set.exotic {
        // Only add if not already present
        let exists = entries.iter().any(|e| same_value_zero(e, &value));
        if !exists {
            entries.push(value);
            let len = entries.len();
            set.set_property(PropertyKey::from("size"), JsValue::Number(len as f64));
        }
    }

    drop(set);
    Ok(this) // Return the set for chaining
}

pub fn set_has(
    _interp: &mut Interpreter,
    this: JsValue,
    args: Vec<JsValue>,
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
    _interp: &mut Interpreter,
    this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let JsValue::Object(set_obj) = this else {
        return Err(JsError::type_error(
            "Set.prototype.delete called on non-object",
        ));
    };

    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    let mut set = set_obj.borrow_mut();

    if let ExoticObject::Set { ref mut entries } = set.exotic {
        for i in 0..entries.len() {
            if same_value_zero(&entries[i], &value) {
                entries.remove(i);
                let len = entries.len();
                set.set_property(PropertyKey::from("size"), JsValue::Number(len as f64));
                return Ok(JsValue::Boolean(true));
            }
        }
    }

    Ok(JsValue::Boolean(false))
}

pub fn set_clear(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let JsValue::Object(set_obj) = this else {
        return Err(JsError::type_error(
            "Set.prototype.clear called on non-object",
        ));
    };

    let mut set = set_obj.borrow_mut();

    if let ExoticObject::Set { ref mut entries } = set.exotic {
        entries.clear();
        set.set_property(PropertyKey::from("size"), JsValue::Number(0.0));
    }

    Ok(JsValue::Undefined)
}

pub fn set_foreach(
    interp: &mut Interpreter,
    this: JsValue,
    args: Vec<JsValue>,
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
            vec![value.clone(), value, this.clone()],
        )?;
    }

    Ok(JsValue::Undefined)
}

pub fn set_keys(
    interp: &mut Interpreter,
    this: JsValue,
    _args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    // For Set, keys() returns the same as values()
    set_values(interp, this, _args)
}

pub fn set_values(
    interp: &mut Interpreter,
    this: JsValue,
    _args: Vec<JsValue>,
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
    _args: Vec<JsValue>,
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
