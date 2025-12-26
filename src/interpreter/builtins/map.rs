//! Map built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{ExoticObject, Guarded, JsMapKey, JsValue, PropertyKey};
use indexmap::IndexMap;

/// Initialize Map.prototype with get, set, has, delete, clear, forEach methods
pub fn init_map_prototype(interp: &mut Interpreter) {
    let proto = interp.map_prototype.clone();

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
    interp.root_guard.guard(constructor.clone());

    // Add static methods to constructor
    interp.register_method(&constructor, "groupBy", map_group_by, 2);

    // Set constructor.prototype = Map.prototype
    let proto_key = PropertyKey::String(interp.intern("prototype"));
    constructor
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.map_prototype.clone()));

    // Set Map.prototype.constructor = Map
    let constructor_key = PropertyKey::String(interp.intern("constructor"));
    interp
        .map_prototype
        .borrow_mut()
        .set_property(constructor_key, JsValue::Object(constructor.clone()));

    // Add Symbol.species getter
    interp.register_species_getter(&constructor);

    // Register globally
    let map_key = PropertyKey::String(interp.intern("Map"));
    interp
        .global
        .borrow_mut()
        .set_property(map_key, JsValue::Object(constructor));
}

pub fn map_constructor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let size_key = PropertyKey::String(interp.intern("size"));

    let guard = interp.heap.create_guard();
    let map_obj = interp.create_object(&guard);
    {
        let mut obj = map_obj.borrow_mut();
        obj.exotic = ExoticObject::Map {
            entries: IndexMap::new(),
        };
        obj.prototype = Some(interp.map_prototype.clone());
        obj.set_property(size_key, JsValue::Number(0.0));
    }

    // If an iterable is passed, add its entries
    // First collect all pairs from the array, then add them to the map
    if let Some(JsValue::Object(arr)) = args.first() {
        let pairs: Vec<(JsValue, JsValue)> = {
            let arr_ref = arr.borrow();
            let mut result = Vec::new();
            if let Some(elements) = arr_ref.array_elements() {
                for elem in elements {
                    if let JsValue::Object(pair_arr) = elem {
                        let pair_ref = pair_arr.borrow();
                        if pair_ref.is_array() {
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
        let size_key = PropertyKey::String(interp.intern("size"));
        let mut map = map_obj.borrow_mut();
        if let ExoticObject::Map { ref mut entries } = map.exotic {
            for (key, value) in pairs {
                entries.insert(JsMapKey(key), value);
            }
            let len = entries.len();
            map.set_property(size_key, JsValue::Number(len as f64));
        }
    }

    Ok(Guarded::with_guard(JsValue::Object(map_obj), guard))
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
        if let Some(value) = entries.get(&JsMapKey(key)) {
            return Ok(Guarded::unguarded(value.clone()));
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

    let size_key = PropertyKey::String(interp.intern("size"));

    let key = args.first().cloned().unwrap_or(JsValue::Undefined);
    let value = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let mut map = map_obj.borrow_mut();

    if let ExoticObject::Map { ref mut entries } = map.exotic {
        entries.insert(JsMapKey(key), value);
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
        return Ok(Guarded::unguarded(JsValue::Boolean(
            entries.contains_key(&JsMapKey(key)),
        )));
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

    let size_key = PropertyKey::String(interp.intern("size"));

    let key = args.first().cloned().unwrap_or(JsValue::Undefined);
    let mut map = map_obj.borrow_mut();

    if let ExoticObject::Map { ref mut entries } = map.exotic {
        if entries.shift_remove(&JsMapKey(key)).is_some() {
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

    let size_key = PropertyKey::String(interp.intern("size"));

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
            entries = e.iter().map(|(k, v)| (k.0.clone(), v.clone())).collect();
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
            keys = e.keys().map(|k| k.0.clone()).collect();
        } else {
            return Err(JsError::type_error("Map.prototype.keys called on non-Map"));
        }
    }

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, keys);
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

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, values);
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

    // Guard the map and collect entries
    let guard = interp.heap.create_guard();
    guard.guard(map_obj.clone());

    let raw_entries: Vec<(JsValue, JsValue)>;
    {
        let map = map_obj.borrow();
        if let ExoticObject::Map { entries: ref e } = map.exotic {
            raw_entries = e.iter().map(|(k, v)| (k.0.clone(), v.clone())).collect();
        } else {
            return Err(JsError::type_error(
                "Map.prototype.entries called on non-Map",
            ));
        }
    }

    // Build entry arrays
    let mut entries = Vec::with_capacity(raw_entries.len());
    for (k, v) in raw_entries {
        let arr = interp.create_array_from(&guard, vec![k, v]);
        entries.push(JsValue::Object(arr));
    }

    let result = interp.create_array_from(&guard, entries);
    Ok(Guarded::with_guard(JsValue::Object(result), guard))
}

/// Map.groupBy(items, callbackFn)
/// Groups elements of an iterable using a callback function.
/// Returns a Map where keys are group values and values are arrays.
/// Unlike Object.groupBy, keys can be any value (not just strings).
pub fn map_group_by(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let items = args.first().cloned().unwrap_or(JsValue::Undefined);
    let callback = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Items must be iterable - for now we support arrays
    let JsValue::Object(items_ref) = items else {
        return Err(JsError::type_error("Map.groupBy requires an iterable"));
    };

    // Guard the inputs
    let guard = interp.heap.create_guard();
    guard.guard(items_ref.clone());
    if let JsValue::Object(cb_obj) = &callback {
        guard.guard(cb_obj.clone());
    }

    // Get array elements
    let elements: Vec<JsValue> = {
        let items_borrowed = items_ref.borrow();
        if let Some(elems) = items_borrowed.array_elements() {
            elems.to_vec()
        } else {
            return Err(JsError::type_error(
                "Map.groupBy requires an array-like object",
            ));
        }
    };

    // Create a new Map for the result
    let size_key = PropertyKey::String(interp.intern("size"));
    let map_obj = interp.create_object(&guard);
    {
        let mut obj = map_obj.borrow_mut();
        obj.exotic = ExoticObject::Map {
            entries: IndexMap::new(),
        };
        obj.prototype = Some(interp.map_prototype.clone());
        obj.set_property(size_key.clone(), JsValue::Number(0.0));
    }

    // Track groups using IndexMap for O(1) lookup with SameValueZero semantics
    let mut groups: IndexMap<JsMapKey, Vec<JsValue>> = IndexMap::new();

    // Iterate and group
    for (index, item) in elements.into_iter().enumerate() {
        // Guard the item in case callback triggers GC
        if let JsValue::Object(item_obj) = &item {
            guard.guard(item_obj.clone());
        }

        // Call the callback with (item, index)
        let key_result = interp.call_function(
            callback.clone(),
            JsValue::Undefined,
            &[item.clone(), JsValue::Number(index as f64)],
        )?;

        let key = key_result.value;

        // Add to existing group or create new one
        groups.entry(JsMapKey(key)).or_default().push(item);
    }

    // Now build the Map from the groups
    // First, create all the arrays (which may trigger GC)
    let mut built_entries: IndexMap<JsMapKey, JsValue> = IndexMap::with_capacity(groups.len());
    for (key, items) in groups {
        let arr = interp.create_array_from(&guard, items);
        built_entries.insert(key, JsValue::Object(arr));
    }

    // Then add entries to the map
    {
        let mut map = map_obj.borrow_mut();
        if let ExoticObject::Map { ref mut entries } = map.exotic {
            *entries = built_entries;
            let len = entries.len();
            map.set_property(size_key, JsValue::Number(len as f64));
        }
    }

    Ok(Guarded::with_guard(JsValue::Object(map_obj), guard))
}
