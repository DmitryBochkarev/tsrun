//! Map built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object, register_method, ExoticObject, JsFunction, JsObjectRef,
    JsValue, NativeFunction, PropertyKey,
};

/// Create Map.prototype with get, set, has, delete, clear, forEach methods
pub fn create_map_prototype() -> JsObjectRef {
    let proto = create_object();
    {
        let mut p = proto.borrow_mut();

        register_method(&mut p, "get", map_get, 1);
        register_method(&mut p, "set", map_set, 2);
        register_method(&mut p, "has", map_has, 1);
        register_method(&mut p, "delete", map_delete, 1);
        register_method(&mut p, "clear", map_clear, 0);
        register_method(&mut p, "forEach", map_foreach, 1);
        register_method(&mut p, "keys", map_keys, 0);
        register_method(&mut p, "values", map_values, 0);
        register_method(&mut p, "entries", map_entries, 0);
    }
    proto
}

/// Create Map constructor
pub fn create_map_constructor() -> JsObjectRef {
    create_function(JsFunction::Native(NativeFunction {
        name: "Map".to_string(),
        func: map_constructor,
        arity: 0,
    }))
}

// Helper to check SameValueZero equality for Map/Set keys
pub fn same_value_zero(a: &JsValue, b: &JsValue) -> bool {
    match (a, b) {
        (JsValue::Number(x), JsValue::Number(y)) => {
            // NaN equals NaN, -0 equals +0
            if x.is_nan() && y.is_nan() {
                return true;
            }
            x == y
        }
        (JsValue::String(x), JsValue::String(y)) => x == y,
        (JsValue::Boolean(x), JsValue::Boolean(y)) => x == y,
        (JsValue::Null, JsValue::Null) => true,
        (JsValue::Undefined, JsValue::Undefined) => true,
        (JsValue::Object(x), JsValue::Object(y)) => std::ptr::eq(x.as_ptr(), y.as_ptr()),
        _ => false,
    }
}

pub fn map_constructor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let map_obj = create_object();
    {
        let mut obj = map_obj.borrow_mut();
        obj.exotic = ExoticObject::Map {
            entries: Vec::new(),
        };
        obj.prototype = Some(interp.map_prototype.clone());
        obj.set_property(PropertyKey::from("size"), JsValue::Number(0.0));
    }

    // If an iterable is passed, add its entries
    // First collect all pairs from the array, then add them to the map
    if let Some(JsValue::Object(arr)) = args.first() {
        let pairs: Vec<(JsValue, JsValue)> = {
            let arr_ref = arr.borrow();
            let mut result = Vec::new();
            if let ExoticObject::Array { length } = arr_ref.exotic {
                for i in 0..length {
                    if let Some(JsValue::Object(pair_arr)) =
                        arr_ref.get_property(&PropertyKey::Index(i))
                    {
                        let pair_ref = pair_arr.borrow();
                        if let ExoticObject::Array { .. } = pair_ref.exotic {
                            let key = pair_ref
                                .get_property(&PropertyKey::Index(0))
                                .unwrap_or(JsValue::Undefined);
                            let value = pair_ref
                                .get_property(&PropertyKey::Index(1))
                                .unwrap_or(JsValue::Undefined);
                            result.push((key, value));
                        }
                    }
                }
            }
            result
        };

        // Now add all pairs to the map
        let mut map = map_obj.borrow_mut();
        if let ExoticObject::Map { ref mut entries } = map.exotic {
            for (key, value) in pairs {
                // Check if key already exists
                let mut found = false;
                for entry in entries.iter_mut() {
                    if same_value_zero(&entry.0, &key) {
                        entry.1 = value.clone();
                        found = true;
                        break;
                    }
                }
                if !found {
                    entries.push((key, value));
                }
            }
            let len = entries.len();
            map.set_property(PropertyKey::from("size"), JsValue::Number(len as f64));
        }
    }

    Ok(JsValue::Object(map_obj))
}

pub fn map_get(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(map_obj) = this else {
        return Err(JsError::type_error(
            "Map.prototype.get called on non-object",
        ));
    };

    let key = args.first().cloned().unwrap_or(JsValue::Undefined);
    let map = map_obj.borrow();

    if let ExoticObject::Map { ref entries } = map.exotic {
        for (k, v) in entries {
            if same_value_zero(k, &key) {
                return Ok(v.clone());
            }
        }
    }

    Ok(JsValue::Undefined)
}

pub fn map_set(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(map_obj) = this.clone() else {
        return Err(JsError::type_error(
            "Map.prototype.set called on non-object",
        ));
    };

    let key = args.first().cloned().unwrap_or(JsValue::Undefined);
    let value = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let mut map = map_obj.borrow_mut();

    if let ExoticObject::Map { ref mut entries } = map.exotic {
        // Check if key already exists
        for entry in entries.iter_mut() {
            if same_value_zero(&entry.0, &key) {
                entry.1 = value;
                drop(map);
                return Ok(this); // Return the map for chaining
            }
        }
        entries.push((key, value));
        // Update size property
        let len = entries.len();
        map.set_property(PropertyKey::from("size"), JsValue::Number(len as f64));
    }

    drop(map);
    Ok(this) // Return the map for chaining
}

pub fn map_has(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(map_obj) = this else {
        return Err(JsError::type_error(
            "Map.prototype.has called on non-object",
        ));
    };

    let key = args.first().cloned().unwrap_or(JsValue::Undefined);
    let map = map_obj.borrow();

    if let ExoticObject::Map { ref entries } = map.exotic {
        for (k, _) in entries {
            if same_value_zero(k, &key) {
                return Ok(JsValue::Boolean(true));
            }
        }
    }

    Ok(JsValue::Boolean(false))
}

pub fn map_delete(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(map_obj) = this else {
        return Err(JsError::type_error(
            "Map.prototype.delete called on non-object",
        ));
    };

    let key = args.first().cloned().unwrap_or(JsValue::Undefined);
    let mut map = map_obj.borrow_mut();

    if let ExoticObject::Map { ref mut entries } = map.exotic {
        if let Some(i) = entries.iter().position(|(k, _)| same_value_zero(k, &key)) {
            entries.remove(i);
            let len = entries.len();
            map.set_property(PropertyKey::from("size"), JsValue::Number(len as f64));
            return Ok(JsValue::Boolean(true));
        }
    }

    Ok(JsValue::Boolean(false))
}

pub fn map_clear(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(map_obj) = this else {
        return Err(JsError::type_error(
            "Map.prototype.clear called on non-object",
        ));
    };

    let mut map = map_obj.borrow_mut();

    if let ExoticObject::Map { ref mut entries } = map.exotic {
        entries.clear();
        map.set_property(PropertyKey::from("size"), JsValue::Number(0.0));
    }

    Ok(JsValue::Undefined)
}

pub fn map_foreach(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(map_obj) = this.clone() else {
        return Err(JsError::type_error(
            "Map.prototype.forEach called on non-object",
        ));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Collect entries first to avoid borrow issues
    let entries: Vec<(JsValue, JsValue)>;
    {
        let map = map_obj.borrow();
        if let ExoticObject::Map { entries: ref e } = map.exotic {
            entries = e.clone();
        } else {
            return Err(JsError::type_error(
                "Map.prototype.forEach called on non-Map",
            ));
        }
    }

    for (key, value) in entries {
        interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[value, key, this.clone()],
        )?;
    }

    Ok(JsValue::Undefined)
}

pub fn map_keys(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(map_obj) = this else {
        return Err(JsError::type_error(
            "Map.prototype.keys called on non-object",
        ));
    };

    let keys: Vec<JsValue>;
    {
        let map = map_obj.borrow();
        if let ExoticObject::Map { entries: ref e } = map.exotic {
            keys = e.iter().map(|(k, _)| k.clone()).collect();
        } else {
            return Err(JsError::type_error("Map.prototype.keys called on non-Map"));
        }
    }

    Ok(JsValue::Object(interp.create_array(keys)))
}

pub fn map_values(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(map_obj) = this else {
        return Err(JsError::type_error(
            "Map.prototype.values called on non-object",
        ));
    };

    let values: Vec<JsValue>;
    {
        let map = map_obj.borrow();
        if let ExoticObject::Map { entries: ref e } = map.exotic {
            values = e.iter().map(|(_, v)| v.clone()).collect();
        } else {
            return Err(JsError::type_error(
                "Map.prototype.values called on non-Map",
            ));
        }
    }

    Ok(JsValue::Object(interp.create_array(values)))
}

pub fn map_entries(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(map_obj) = this else {
        return Err(JsError::type_error(
            "Map.prototype.entries called on non-object",
        ));
    };

    let raw_entries: Vec<(JsValue, JsValue)>;
    {
        let map = map_obj.borrow();
        if let ExoticObject::Map { entries: ref e } = map.exotic {
            raw_entries = e.clone();
        } else {
            return Err(JsError::type_error(
                "Map.prototype.entries called on non-Map",
            ));
        }
    }

    let entries: Vec<JsValue> = raw_entries
        .into_iter()
        .map(|(k, v)| JsValue::Object(interp.create_array(vec![k, v])))
        .collect();

    Ok(JsValue::Object(interp.create_array(entries)))
}
