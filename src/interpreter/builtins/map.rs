//! Map built-in methods

use crate::error::JsError;
use crate::gc::Gc;
use crate::interpreter::Interpreter;
use crate::value::{ExoticObject, Guarded, JsObject, JsValue, PropertyKey};

/// Initialize Map.prototype with get, set, has, delete, clear, forEach methods
pub fn init_map_prototype(interp: &mut Interpreter) {
    let proto = interp.map_prototype;

    interp.register_method(&proto, "get", map_get, 1);
    interp.register_method(&proto, "set", map_set, 2);
    interp.register_method(&proto, "has", map_has, 1);
    interp.register_method(&proto, "delete", map_delete, 1);
    interp.register_method(&proto, "clear", map_clear, 0);
    interp.register_method(&proto, "forEach", map_foreach, 1);
    interp.register_method(&proto, "keys", map_keys, 0);
    interp.register_method(&proto, "values", map_values, 0);
    interp.register_method(&proto, "entries", map_entries, 0);
}

/// Create Map constructor and register it globally
pub fn init_map(interp: &mut Interpreter) {
    init_map_prototype(interp);

    let constructor = interp.create_native_function("Map", map_constructor, 0);
    interp.root_guard.guard(&constructor);

    // Set prototype property on constructor
    let proto_key = interp.key("prototype");
    constructor.own(&interp.map_prototype, &interp.heap);
    constructor
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.map_prototype));

    // Register globally
    let map_key = interp.key("Map");
    interp.global.own(&constructor, &interp.heap);
    interp
        .global
        .borrow_mut()
        .set_property(map_key, JsValue::Object(constructor));
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
        (JsValue::Object(x), JsValue::Object(y)) => x.id() == y.id(),
        _ => false,
    }
}

pub fn map_constructor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let size_key = interp.key("size");

    let (map_obj, map_guard) = interp.create_object_with_guard();
    {
        let mut obj = map_obj.borrow_mut();
        obj.exotic = ExoticObject::Map {
            entries: Vec::new(),
        };
        obj.prototype = Some(interp.map_prototype);
        obj.set_property(size_key, JsValue::Number(0.0));
    }
    map_obj.own(&interp.map_prototype, &interp.heap);

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
        let size_key = interp.key("size");
        let mut map = map_obj.borrow_mut();
        if let ExoticObject::Map { ref mut entries } = map.exotic {
            for (key, value) in pairs {
                // Own any object keys/values
                if let JsValue::Object(ref obj) = key {
                    map_obj.own(obj, &interp.heap);
                }
                if let JsValue::Object(ref obj) = value {
                    map_obj.own(obj, &interp.heap);
                }
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
            map.set_property(size_key, JsValue::Number(len as f64));
        }
    }

    Ok(Guarded::with_guard(JsValue::Object(map_obj), map_guard))
}

pub fn map_get(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
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
                return Ok(Guarded::unguarded(v.clone()));
            }
        }
    }

    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn map_set(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(map_obj) = this.clone() else {
        return Err(JsError::type_error(
            "Map.prototype.set called on non-object",
        ));
    };

    let size_key = interp.key("size");

    let key = args.first().cloned().unwrap_or(JsValue::Undefined);
    let value = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Own any object keys/values
    if let JsValue::Object(ref obj) = key {
        map_obj.own(obj, &interp.heap);
    }
    if let JsValue::Object(ref obj) = value {
        map_obj.own(obj, &interp.heap);
    }

    let mut map = map_obj.borrow_mut();

    if let ExoticObject::Map { ref mut entries } = map.exotic {
        // Check if key already exists
        for entry in entries.iter_mut() {
            if same_value_zero(&entry.0, &key) {
                entry.1 = value;
                drop(map);
                return Ok(Guarded::unguarded(this)); // Return the map for chaining
            }
        }
        entries.push((key, value));
        // Update size property
        let len = entries.len();
        map.set_property(size_key, JsValue::Number(len as f64));
    }

    drop(map);
    Ok(Guarded::unguarded(this)) // Return the map for chaining
}

pub fn map_has(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
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
                return Ok(Guarded::unguarded(JsValue::Boolean(true)));
            }
        }
    }

    Ok(Guarded::unguarded(JsValue::Boolean(false)))
}

pub fn map_delete(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(map_obj) = this else {
        return Err(JsError::type_error(
            "Map.prototype.delete called on non-object",
        ));
    };

    let size_key = interp.key("size");

    let key = args.first().cloned().unwrap_or(JsValue::Undefined);
    let mut map = map_obj.borrow_mut();

    if let ExoticObject::Map { ref mut entries } = map.exotic {
        if let Some(i) = entries.iter().position(|(k, _)| same_value_zero(k, &key)) {
            entries.remove(i);
            let len = entries.len();
            map.set_property(size_key, JsValue::Number(len as f64));
            return Ok(Guarded::unguarded(JsValue::Boolean(true)));
        }
    }

    Ok(Guarded::unguarded(JsValue::Boolean(false)))
}

pub fn map_clear(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(map_obj) = this else {
        return Err(JsError::type_error(
            "Map.prototype.clear called on non-object",
        ));
    };

    let size_key = interp.key("size");

    let mut map = map_obj.borrow_mut();

    if let ExoticObject::Map { ref mut entries } = map.exotic {
        entries.clear();
        map.set_property(size_key, JsValue::Number(0.0));
    }

    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn map_foreach(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
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

    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn map_keys(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
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

    let (arr, guard) = interp.create_array(keys);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn map_values(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
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

    let (arr, guard) = interp.create_array(values);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn map_entries(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
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

    // Build entry arrays and collect nested objects for ownership
    let mut entries = Vec::with_capacity(raw_entries.len());
    let mut nested_arrays: Vec<Gc<JsObject>> = Vec::new();
    for (k, v) in raw_entries {
        let (arr, _guard) = interp.create_array(vec![k, v]);
        interp.root_guard.guard(&arr);
        nested_arrays.push(arr);
        entries.push(JsValue::Object(arr));
    }

    let (result, guard) = interp.create_array(entries);
    // Own all nested arrays
    for nested in nested_arrays {
        result.own(&nested, &interp.heap);
    }
    Ok(Guarded::with_guard(JsValue::Object(result), guard))
}
