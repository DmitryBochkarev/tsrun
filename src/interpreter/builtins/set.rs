//! Set built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::prelude::{Box, Vec, index_set_new, vec};
use crate::value::{CheapClone, ExoticObject, Guarded, JsMapKey, JsValue, PropertyKey};

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

    // Symbol.iterator = Set.prototype.values (Set iterates over values by default)
    let well_known = interp.well_known_symbols;
    let iterator_symbol =
        crate::value::JsSymbol::new(well_known.iterator, Some(interp.intern("Symbol.iterator")));
    let iterator_key = crate::value::PropertyKey::Symbol(Box::new(iterator_symbol));
    let values_fn = interp.create_native_function("[Symbol.iterator]", set_values, 0);
    proto
        .borrow_mut()
        .set_property(iterator_key, JsValue::Object(values_fn));
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
            entries: index_set_new(),
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

    // Create an iterator object
    let guard = interp.heap.create_guard();
    guard.guard(set_obj.clone());

    // Collect values upfront to avoid borrow issues during iteration
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

    // Store values as an array in the iterator
    let values_arr = interp.create_array_from(&guard, values);

    // Create iterator object
    let iter_obj = interp.create_object_raw(&guard);
    let entries_key = interp.property_key("__entries__");
    let index_key = interp.property_key("__index__");
    let next_key = interp.property_key("next");

    iter_obj
        .borrow_mut()
        .set_property(entries_key, JsValue::Object(values_arr));
    iter_obj
        .borrow_mut()
        .set_property(index_key, JsValue::Number(0.0));

    // Add next() method
    let next_fn = interp.create_native_function("next", set_iterator_next, 0);
    guard.guard(next_fn.cheap_clone());
    iter_obj
        .borrow_mut()
        .set_property(next_key, JsValue::Object(next_fn));

    // Add Symbol.iterator that returns the iterator itself (iterator protocol)
    let well_known = interp.well_known_symbols;
    let iterator_symbol =
        crate::value::JsSymbol::new(well_known.iterator, Some(interp.intern("Symbol.iterator")));
    let iterator_key = crate::value::PropertyKey::Symbol(Box::new(iterator_symbol));
    let self_iterator_fn = interp.create_native_function("[Symbol.iterator]", set_iterator_self, 0);
    guard.guard(self_iterator_fn.cheap_clone());
    iter_obj
        .borrow_mut()
        .set_property(iterator_key, JsValue::Object(self_iterator_fn));

    Ok(Guarded::with_guard(JsValue::Object(iter_obj), guard))
}

/// Iterator Symbol.iterator function - returns the iterator itself
fn set_iterator_self(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    // Iterator's Symbol.iterator returns itself
    Ok(Guarded::unguarded(this))
}

/// Iterator next() function for Set values iterator
fn set_iterator_next(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(iter_obj) = this else {
        return Err(JsError::type_error("next called on non-object"));
    };

    let entries_key = interp.property_key("__entries__");
    let index_key = interp.property_key("__index__");
    let value_key = interp.property_key("value");
    let done_key = interp.property_key("done");

    // Get the entries array and current index
    let entries_val = iter_obj.borrow().get_property(&entries_key);
    let index_val = iter_obj.borrow().get_property(&index_key);

    let Some(JsValue::Object(entries_arr)) = entries_val else {
        return Err(JsError::type_error("Invalid set iterator"));
    };

    let index = match index_val {
        Some(JsValue::Number(n)) => n as u32,
        _ => 0,
    };

    // Get length of entries array
    let length = entries_arr.borrow().array_length().unwrap_or(0);

    if index >= length {
        // Done
        let guard = interp.heap.create_guard();
        let result = interp.create_object_raw(&guard);
        result
            .borrow_mut()
            .set_property(value_key, JsValue::Undefined);
        result
            .borrow_mut()
            .set_property(done_key, JsValue::Boolean(true));
        Ok(Guarded::with_guard(JsValue::Object(result), guard))
    } else {
        // Get the value at current index
        let value = entries_arr
            .borrow()
            .get_property(&PropertyKey::Index(index))
            .unwrap_or(JsValue::Undefined);

        // Increment index
        let index_key = interp.property_key("__index__");
        iter_obj
            .borrow_mut()
            .set_property(index_key, JsValue::Number((index + 1) as f64));

        let guard = interp.heap.create_guard();
        let result = interp.create_object_raw(&guard);
        let value_key = interp.property_key("value");
        let done_key = interp.property_key("done");
        result.borrow_mut().set_property(value_key, value);
        result
            .borrow_mut()
            .set_property(done_key, JsValue::Boolean(false));
        Ok(Guarded::with_guard(JsValue::Object(result), guard))
    }
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

    // Create an iterator object
    let guard = interp.heap.create_guard();
    guard.guard(set_obj.clone());

    // Collect entries upfront to avoid borrow issues during iteration
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

    // Store entries as an array in the iterator
    let entries_arr = interp.create_array_from(&guard, entries);

    // Create iterator object
    let iter_obj = interp.create_object_raw(&guard);
    let entries_key = interp.property_key("__entries__");
    let index_key = interp.property_key("__index__");
    let next_key = interp.property_key("next");

    iter_obj
        .borrow_mut()
        .set_property(entries_key, JsValue::Object(entries_arr));
    iter_obj
        .borrow_mut()
        .set_property(index_key, JsValue::Number(0.0));

    // Add next() method (reuse the same iterator next function)
    let next_fn = interp.create_native_function("next", set_iterator_next, 0);
    guard.guard(next_fn.cheap_clone());
    iter_obj
        .borrow_mut()
        .set_property(next_key, JsValue::Object(next_fn));

    // Add Symbol.iterator that returns the iterator itself (iterator protocol)
    let well_known = interp.well_known_symbols;
    let iterator_symbol =
        crate::value::JsSymbol::new(well_known.iterator, Some(interp.intern("Symbol.iterator")));
    let iterator_key = crate::value::PropertyKey::Symbol(Box::new(iterator_symbol));
    let self_iterator_fn = interp.create_native_function("[Symbol.iterator]", set_iterator_self, 0);
    guard.guard(self_iterator_fn.cheap_clone());
    iter_obj
        .borrow_mut()
        .set_property(iterator_key, JsValue::Object(self_iterator_fn));

    Ok(Guarded::with_guard(JsValue::Object(iter_obj), guard))
}
