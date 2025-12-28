//! Set built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{ExoticObject, Guarded, JsMapKey, JsValue, PropertyKey};
use indexmap::IndexSet;

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
    interp.root_guard.guard(constructor.clone());

    // Set constructor.prototype = Set.prototype
    let proto_key = PropertyKey::String(interp.intern("prototype"));
    constructor
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.set_prototype.clone()));

    // Set Set.prototype.constructor = Set
    let constructor_key = PropertyKey::String(interp.intern("constructor"));
    interp
        .set_prototype
        .borrow_mut()
        .set_property(constructor_key, JsValue::Object(constructor.clone()));

    // Add Symbol.species getter
    interp.register_species_getter(&constructor);

    // Register globally
    let set_key = PropertyKey::String(interp.intern("Set"));
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
    let size_key = PropertyKey::String(interp.intern("size"));

    let guard = interp.heap.create_guard();
    let set_obj = interp.create_object(&guard);
    {
        let mut obj = set_obj.borrow_mut();
        obj.exotic = ExoticObject::Set {
            entries: IndexSet::new(),
        };
        obj.prototype = Some(interp.set_prototype.clone());
        obj.set_property(size_key, JsValue::Number(0.0));
    }

    // If an iterable (array) is passed, add its elements
    if let Some(JsValue::Object(arr)) = args.first() {
        let arr_ref = arr.borrow();
        if let Some(elements) = arr_ref.array_elements() {
            let items: Vec<JsValue> = elements.to_vec();
            drop(arr_ref);

            let size_key = PropertyKey::String(interp.intern("size"));
            let mut set = set_obj.borrow_mut();
            if let ExoticObject::Set { ref mut entries } = set.exotic {
                for value in items {
                    entries.insert(JsMapKey(value));
                }
                let len = entries.len();
                set.set_property(size_key, JsValue::Number(len as f64));
            }
        }
    }

    Ok(Guarded::with_guard(JsValue::Object(set_obj), guard))
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

    let size_key = PropertyKey::String(interp.intern("size"));

    let value = args.first().cloned().unwrap_or(JsValue::Undefined);

    let mut set = set_obj.borrow_mut();

    if let ExoticObject::Set { ref mut entries } = set.exotic {
        entries.insert(JsMapKey(value));
        let len = entries.len();
        set.set_property(size_key, JsValue::Number(len as f64));
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
        return Ok(Guarded::unguarded(JsValue::Boolean(
            entries.contains(&JsMapKey(value)),
        )));
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

    let size_key = PropertyKey::String(interp.intern("size"));

    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    let mut set = set_obj.borrow_mut();

    if let ExoticObject::Set { ref mut entries } = set.exotic
        && entries.shift_remove(&JsMapKey(value))
    {
        let len = entries.len();
        set.set_property(size_key, JsValue::Number(len as f64));
        return Ok(Guarded::unguarded(JsValue::Boolean(true)));
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

    let size_key = PropertyKey::String(interp.intern("size"));

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
            entries = e.iter().map(|k| k.0.clone()).collect();
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
            values = e.iter().map(|k| k.0.clone()).collect();
        } else {
            return Err(JsError::type_error(
                "Set.prototype.values called on non-Set",
            ));
        }
    }

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, values);
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

    // Guard the set and collect entries
    let guard = interp.heap.create_guard();
    guard.guard(set_obj.clone());

    let raw_entries: Vec<JsValue>;
    {
        let set = set_obj.borrow();
        if let ExoticObject::Set { entries: ref e } = set.exotic {
            raw_entries = e.iter().map(|k| k.0.clone()).collect();
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
        let arr = interp.create_array_from(&guard, vec![v.clone(), v]);
        entries.push(JsValue::Object(arr));
    }

    let result = interp.create_array_from(&guard, entries);
    Ok(Guarded::with_guard(JsValue::Object(result), guard))
}
