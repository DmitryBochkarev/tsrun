//! Array built-in methods

use crate::error::JsError;
use crate::gc::Gc;
use crate::interpreter::Interpreter;
use crate::prelude::*;
use crate::value::{
    CheapClone, ExoticObject, Guarded, JsObject, JsObjectRef, JsString, JsValue, PropertyKey,
};

/// Convert a number to a length value per ECMAScript ToLength.
/// Clamps to [0, 2^53 - 1] (MAX_SAFE_INTEGER) and truncates.
fn to_length(n: f64) -> u32 {
    if n.is_nan() || n <= 0.0 {
        0
    } else if n > u32::MAX as f64 {
        u32::MAX
    } else {
        n as u32
    }
}

/// Parse a string to a number for ToNumber coercion.
fn string_to_number(s: &str) -> f64 {
    let s = s.trim();
    if s.is_empty() {
        return 0.0;
    }
    s.parse::<f64>().unwrap_or(f64::NAN)
}

/// Get the length of an array-like object with full ToLength coercion.
/// This version properly handles objects with valueOf/toString by calling the interpreter.
fn get_array_like_length(interp: &mut Interpreter, obj: &Gc<JsObject>) -> Result<u32, JsError> {
    let borrowed = obj.borrow();
    // First check if it's a real array
    if let ExoticObject::Array { ref elements } = borrowed.exotic {
        return Ok(elements.len() as u32);
    }
    // Otherwise, get the length property and coerce it
    let length_key = PropertyKey::String(JsString::from("length"));
    let length_val = borrowed
        .get_property(&length_key)
        .unwrap_or(JsValue::Undefined);
    drop(borrowed); // Release borrow before calling interpreter methods

    // Handle simple cases without calling interpreter
    match &length_val {
        JsValue::Number(n) => Ok(to_length(*n)),
        JsValue::Boolean(true) => Ok(1),
        JsValue::Boolean(false) => Ok(0),
        JsValue::Null => Ok(0),
        JsValue::Undefined => Ok(0),
        JsValue::String(s) => Ok(to_length(string_to_number(s.as_str()))),
        JsValue::Symbol(_) => Ok(0), // Symbols can't be converted to number
        JsValue::Object(_) => {
            // Call ToPrimitive with "number" hint, then ToNumber
            let n = interp.coerce_to_number(&length_val)?;
            Ok(to_length(n))
        }
    }
}

/// Get an element from an array-like object by index.
/// Works on both real arrays and array-like objects.
fn get_array_like_element(obj: &Gc<JsObject>, index: u32) -> JsValue {
    let borrowed = obj.borrow();
    // First check if it's a real array with dense storage
    if let ExoticObject::Array { ref elements } = borrowed.exotic {
        return elements
            .get(index as usize)
            .cloned()
            .unwrap_or(JsValue::Undefined);
    }
    // Otherwise, get by property index
    borrowed
        .get_property(&PropertyKey::Index(index))
        .unwrap_or(JsValue::Undefined)
}

/// Check if an array-like object has an element at the given index.
/// Works on both real arrays and array-like objects.
fn has_array_like_element(obj: &Gc<JsObject>, index: u32) -> bool {
    let borrowed = obj.borrow();
    // First check if it's a real array with dense storage
    if let ExoticObject::Array { ref elements } = borrowed.exotic {
        return index < elements.len() as u32;
    }
    // Otherwise, check property existence
    borrowed.has_own_property(&PropertyKey::Index(index))
}

/// Initialize Array.prototype with all array methods.
/// The prototype object must already exist in `interp.array_prototype`.
pub fn init_array_prototype(interp: &mut Interpreter) {
    let proto = interp.array_prototype.clone();

    // Mutating methods
    interp.register_method(&proto, "push", array_push, 1);
    interp.register_method(&proto, "pop", array_pop, 0);
    interp.register_method(&proto, "shift", array_shift, 0);
    interp.register_method(&proto, "unshift", array_unshift, 1);
    interp.register_method(&proto, "splice", array_splice, 2);
    interp.register_method(&proto, "reverse", array_reverse, 0);
    interp.register_method(&proto, "sort", array_sort, 1);
    interp.register_method(&proto, "fill", array_fill, 3);
    interp.register_method(&proto, "copyWithin", array_copy_within, 3);

    // Accessor methods
    interp.register_method(&proto, "at", array_at, 1);
    interp.register_method(&proto, "concat", array_concat, 1);
    interp.register_method(&proto, "slice", array_slice, 2);
    interp.register_method(&proto, "join", array_join, 1);
    interp.register_method(&proto, "toString", array_to_string, 0);
    interp.register_method(&proto, "indexOf", array_index_of, 1);
    interp.register_method(&proto, "lastIndexOf", array_last_index_of, 1);
    interp.register_method(&proto, "includes", array_includes, 1);

    // Iteration methods
    interp.register_method(&proto, "forEach", array_foreach, 1);
    interp.register_method(&proto, "map", array_map, 1);
    interp.register_method(&proto, "filter", array_filter, 1);
    interp.register_method(&proto, "reduce", array_reduce, 1);
    interp.register_method(&proto, "reduceRight", array_reduce_right, 1);
    interp.register_method(&proto, "find", array_find, 1);
    interp.register_method(&proto, "findIndex", array_find_index, 1);
    interp.register_method(&proto, "findLast", array_find_last, 1);
    interp.register_method(&proto, "findLastIndex", array_find_last_index, 1);
    interp.register_method(&proto, "every", array_every, 1);
    interp.register_method(&proto, "some", array_some, 1);
    interp.register_method(&proto, "flat", array_flat, 1);
    interp.register_method(&proto, "flatMap", array_flat_map, 1);

    // Non-mutating methods (ES2023+)
    interp.register_method(&proto, "toReversed", array_to_reversed, 0);
    interp.register_method(&proto, "toSorted", array_to_sorted, 1);
    interp.register_method(&proto, "toSpliced", array_to_spliced, 2);
    interp.register_method(&proto, "with", array_with, 2);

    // Iterator methods
    interp.register_method(&proto, "keys", array_keys, 0);
    interp.register_method(&proto, "values", array_values, 0);
    interp.register_method(&proto, "entries", array_entries, 0);

    // Symbol.iterator = Array.prototype.values
    let well_known = interp.well_known_symbols;
    let iterator_symbol =
        crate::value::JsSymbol::new(well_known.iterator, Some(interp.intern("Symbol.iterator")));
    let iterator_key = crate::value::PropertyKey::Symbol(Box::new(iterator_symbol));
    let values_fn = interp.create_native_function("[Symbol.iterator]", array_values, 0);
    proto
        .borrow_mut()
        .set_property(iterator_key, JsValue::Object(values_fn));
}

/// Create Array constructor with static methods (isArray, of, from)
pub fn create_array_constructor(interp: &mut Interpreter) -> JsObjectRef {
    let constructor = interp.create_native_function("Array", array_constructor_fn, 0);

    interp.register_method(&constructor, "isArray", array_is_array, 1);
    interp.register_method(&constructor, "of", array_of, 0);
    interp.register_method(&constructor, "from", array_from, 1);

    // Set constructor.prototype = Array.prototype
    let proto_key = PropertyKey::String(interp.intern("prototype"));
    constructor
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.array_prototype.clone()));

    // Set Array.prototype.constructor = Array
    let constructor_key = PropertyKey::String(interp.intern("constructor"));
    interp
        .array_prototype
        .borrow_mut()
        .set_property(constructor_key, JsValue::Object(constructor.clone()));

    // Add Symbol.species getter
    interp.register_species_getter(&constructor);

    constructor
}

pub fn array_constructor_fn(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    if args.len() == 1
        && let Some(JsValue::Number(n)) = args.first()
    {
        let len = *n as u32;
        let mut elements = Vec::with_capacity(len as usize);
        for _ in 0..len {
            elements.push(JsValue::Undefined);
        }
        let guard = interp.heap.create_guard();
        let arr = interp.create_array_from(&guard, elements);
        return Ok(Guarded::with_guard(JsValue::Object(arr), guard));
    }
    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, args.to_vec());
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_is_array(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    let is_array = match value {
        JsValue::Object(obj) => matches!(obj.borrow().exotic, ExoticObject::Array { .. }),
        _ => false,
    };
    Ok(Guarded::unguarded(JsValue::Boolean(is_array)))
}

pub fn array_push(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.push called on non-object",
        ));
    };

    let mut arr_ref = arr.borrow_mut();

    let elements = arr_ref
        .array_elements_mut()
        .ok_or_else(|| JsError::type_error("Array.prototype.push called on non-array"))?;

    for arg in args {
        elements.push(arg.clone());
    }

    let new_length = elements.len();
    Ok(Guarded::unguarded(JsValue::Number(new_length as f64)))
}

pub fn array_pop(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.pop called on non-object",
        ));
    };

    let mut arr_ref = arr.borrow_mut();

    let elements = arr_ref
        .array_elements_mut()
        .ok_or_else(|| JsError::type_error("Array.prototype.pop called on non-array"))?;

    let value = elements.pop().unwrap_or(JsValue::Undefined);
    Ok(Guarded::unguarded(value))
}

pub fn array_map(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.map called on non-object",
        ));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error(
            "Array.prototype.map callback is not a function",
        ));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Create a single guard that will protect all values throughout the function.
    // This guard protects: the callback, this_arg, arr, and all mapped result values.
    let guard = interp.heap.create_guard();

    // Guard the callback and this_arg to prevent GC from collecting them
    // during the loop iterations which may trigger allocations.
    if let JsValue::Object(obj) = &callback {
        guard.guard(obj.cheap_clone());
    }
    if let JsValue::Object(obj) = &this_arg {
        guard.guard(obj.cheap_clone());
    }
    guard.guard(arr.cheap_clone());

    // Use array-like length with full ToLength coercion (works on both arrays and array-like objects)
    let length = get_array_like_length(interp, &arr)?;

    let mut result = Vec::with_capacity(length as usize);
    for i in 0..length {
        // Check if property exists (sparse arrays / array-likes may have holes)
        if has_array_like_element(&arr, i) {
            let elem = get_array_like_element(&arr, i);

            let Guarded {
                value: mapped,
                guard: _mapped_guard,
            } = interp.call_function(
                callback.clone(),
                this_arg.clone(),
                &[elem, JsValue::Number(i as f64), this.clone()],
            )?;

            // Guard the mapped value to keep it alive until we create the result array.
            // Without this, GC could collect mapped objects during subsequent iterations.
            if let JsValue::Object(obj) = &mapped {
                guard.guard(obj.cheap_clone());
            }

            result.push(mapped);
        } else {
            // Preserve holes as undefined in the result
            result.push(JsValue::Undefined);
        }
    }

    let arr = interp.create_array_from(&guard, result);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_filter(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.filter called on non-object",
        ));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error(
            "Array.prototype.filter callback is not a function",
        ));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Create a single guard that will protect all values throughout the function.
    // This guard protects: the callback, this_arg, arr, and all result elements.
    let guard = interp.heap.create_guard();

    // Guard the callback and this_arg to prevent GC from collecting them
    // during the loop iterations which may trigger allocations.
    if let JsValue::Object(obj) = &callback {
        guard.guard(obj.cheap_clone());
    }
    if let JsValue::Object(obj) = &this_arg {
        guard.guard(obj.cheap_clone());
    }
    guard.guard(arr.cheap_clone());

    // Use array-like length with full ToLength coercion (works on both arrays and array-like objects)
    let length = get_array_like_length(interp, &arr)?;

    let mut result = Vec::new();
    for i in 0..length {
        if has_array_like_element(&arr, i) {
            let elem = get_array_like_element(&arr, i);

            let Guarded {
                value: keep,
                guard: _keep_guard,
            } = interp.call_function(
                callback.clone(),
                this_arg.clone(),
                &[elem.clone(), JsValue::Number(i as f64), this.clone()],
            )?;

            if keep.to_boolean() {
                // Guard the element to keep it alive until we create the result array.
                if let JsValue::Object(obj) = &elem {
                    guard.guard(obj.cheap_clone());
                }
                result.push(elem);
            }
        }
    }

    let arr = interp.create_array_from(&guard, result);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_foreach(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.forEach called on non-object",
        ));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error(
            "Array.prototype.forEach callback is not a function",
        ));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Guard values to prevent GC from collecting them during iterations
    let _callback_guard = interp.guard_value(&callback);
    let _this_arg_guard = interp.guard_value(&this_arg);
    let _arr_guard = interp.guard_value(&this);

    // Use array-like length with full ToLength coercion (works on both arrays and array-like objects)
    let length = get_array_like_length(interp, &arr)?;

    for i in 0..length {
        if has_array_like_element(&arr, i) {
            let elem = get_array_like_element(&arr, i);

            interp.call_function(
                callback.clone(),
                this_arg.clone(),
                &[elem, JsValue::Number(i as f64), this.clone()],
            )?;
        }
    }

    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn array_reduce(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.reduce called on non-object",
        ));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error(
            "Array.prototype.reduce callback is not a function",
        ));
    }

    // Guard values to prevent GC from collecting them during iterations
    let _callback_guard = interp.guard_value(&callback);
    let _arr_guard = interp.guard_value(&this);

    // Use array-like length with full ToLength coercion (works on both arrays and array-like objects)
    let length = get_array_like_length(interp, &arr)?;

    let (mut accumulator, start_index) = if let Some(initial) = args.get(1) {
        (initial.clone(), 0)
    } else {
        if length == 0 {
            return Err(JsError::type_error(
                "Reduce of empty array with no initial value",
            ));
        }
        let first = get_array_like_element(&arr, 0);
        (first, 1)
    };

    for i in start_index..length {
        if has_array_like_element(&arr, i) {
            let elem = get_array_like_element(&arr, i);

            let Guarded {
                value: acc,
                guard: _acc_guard,
            } = interp.call_function(
                callback.clone(),
                JsValue::Undefined,
                &[accumulator, elem, JsValue::Number(i as f64), this.clone()],
            )?;
            accumulator = acc;
        }
    }

    // Accumulator is a derived value - no guard needed as it's either a primitive
    // or an object from the array/callback which is already owned
    Ok(Guarded::unguarded(accumulator))
}

pub fn array_find(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.find called on non-object",
        ));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error(
            "Array.prototype.find callback is not a function",
        ));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Guard values to prevent GC from collecting them during iterations
    let _callback_guard = interp.guard_value(&callback);
    let _this_arg_guard = interp.guard_value(&this_arg);
    let _arr_guard = interp.guard_value(&this);

    // Use array-like length with full ToLength coercion (works on both arrays and array-like objects)
    let length = get_array_like_length(interp, &arr)?;

    for i in 0..length {
        let elem = get_array_like_element(&arr, i);

        let Guarded {
            value: result,
            guard: _result_guard,
        } = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem.clone(), JsValue::Number(i as f64), this.clone()],
        )?;

        if result.to_boolean() {
            // Element came from array which is owned by caller - no guard needed
            return Ok(Guarded::unguarded(elem));
        }
    }

    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn array_find_index(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.findIndex called on non-object",
        ));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error(
            "Array.prototype.findIndex callback is not a function",
        ));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Guard values to prevent GC from collecting them during iterations
    let _callback_guard = interp.guard_value(&callback);
    let _this_arg_guard = interp.guard_value(&this_arg);
    let _arr_guard = interp.guard_value(&this);

    // Use array-like length with full ToLength coercion (works on both arrays and array-like objects)
    let length = get_array_like_length(interp, &arr)?;

    for i in 0..length {
        let elem = get_array_like_element(&arr, i);

        let Guarded {
            value: result,
            guard: _result_guard,
        } = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem, JsValue::Number(i as f64), this.clone()],
        )?;

        if result.to_boolean() {
            return Ok(Guarded::unguarded(JsValue::Number(i as f64)));
        }
    }

    Ok(Guarded::unguarded(JsValue::Number(-1.0)))
}

pub fn array_index_of(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.indexOf called on non-object",
        ));
    };

    let search_element = args.first().cloned().unwrap_or(JsValue::Undefined);
    let from_index = args.get(1).map(|v| v.to_number() as i64).unwrap_or(0);

    // Use array-like length with full ToLength coercion (works on both arrays and array-like objects)
    let length = get_array_like_length(interp, &arr)? as i64;

    let start = if from_index < 0 {
        (length + from_index).max(0) as u32
    } else {
        from_index.min(length) as u32
    };

    for i in start..(length as u32) {
        if has_array_like_element(&arr, i) {
            let elem = get_array_like_element(&arr, i);

            if elem.strict_equals(&search_element) {
                return Ok(Guarded::unguarded(JsValue::Number(i as f64)));
            }
        }
    }

    Ok(Guarded::unguarded(JsValue::Number(-1.0)))
}

pub fn array_includes(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.includes called on non-object",
        ));
    };

    let search_element = args.first().cloned().unwrap_or(JsValue::Undefined);
    let from_index = args.get(1).map(|v| v.to_number() as i64).unwrap_or(0);

    // Use array-like length with full ToLength coercion (works on both arrays and array-like objects)
    let length = get_array_like_length(interp, &arr)? as i64;

    let start = if from_index < 0 {
        (length + from_index).max(0) as u32
    } else {
        from_index.min(length) as u32
    };

    for i in start..(length as u32) {
        if has_array_like_element(&arr, i) {
            let elem = get_array_like_element(&arr, i);

            let found = match (&elem, &search_element) {
                (JsValue::Number(a), JsValue::Number(b)) if a.is_nan() && b.is_nan() => true,
                _ => elem.strict_equals(&search_element),
            };

            if found {
                return Ok(Guarded::unguarded(JsValue::Boolean(true)));
            }
        }
    }

    Ok(Guarded::unguarded(JsValue::Boolean(false)))
}

pub fn array_slice(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.slice called on non-object",
        ));
    };

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))? as i64;

    let start_arg = args.first().map(|v| v.to_number() as i64).unwrap_or(0);
    let end_arg = args.get(1).map(|v| v.to_number() as i64).unwrap_or(length);

    let start = if start_arg < 0 {
        (length + start_arg).max(0)
    } else {
        start_arg.min(length)
    };

    let end = if end_arg < 0 {
        (length + end_arg).max(0)
    } else {
        end_arg.min(length)
    };

    let mut result = Vec::new();
    for i in start..end {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i as u32))
            .unwrap_or(JsValue::Undefined);
        result.push(elem);
    }

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, result);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_concat(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let mut result = Vec::new();
    let well_known = interp.well_known_symbols;
    let spreadable_key = PropertyKey::Symbol(Box::new(crate::value::JsSymbol::new(
        well_known.is_concat_spreadable,
        None,
    )));

    // Helper to determine if a value should be spread
    // Returns (should_spread, length_if_spread)
    fn should_spread_value(value: &JsValue, spreadable_key: &PropertyKey) -> (bool, Option<u32>) {
        match value {
            JsValue::Object(obj) => {
                let obj_ref = obj.borrow();

                // Check Symbol.isConcatSpreadable property
                if let Some(spreadable) = obj_ref.get_property(spreadable_key) {
                    // If explicitly set, use its boolean value
                    let spread = spreadable.to_boolean();
                    if spread {
                        // Get length for spreading
                        let length = if let Some(len) = obj_ref.array_length() {
                            len
                        } else {
                            // For non-arrays with spreadable=true, use length property
                            let length_key = PropertyKey::String(JsString::from("length"));
                            obj_ref
                                .get_property(&length_key)
                                .and_then(|v| match v {
                                    JsValue::Number(n) => Some(n as u32),
                                    _ => None,
                                })
                                .unwrap_or(0)
                        };
                        (true, Some(length))
                    } else {
                        (false, None)
                    }
                } else {
                    // If not set, spread only if it's an array
                    if let Some(length) = obj_ref.array_length() {
                        (true, Some(length))
                    } else {
                        (false, None)
                    }
                }
            }
            _ => (false, None),
        }
    }

    fn add_elements(result: &mut Vec<JsValue>, value: JsValue, spreadable_key: &PropertyKey) {
        let (should_spread, length) = should_spread_value(&value, spreadable_key);

        if should_spread {
            if let JsValue::Object(obj) = &value {
                let obj_ref = obj.borrow();
                for i in 0..length.unwrap_or(0) {
                    let elem = obj_ref
                        .get_property(&PropertyKey::Index(i))
                        .unwrap_or(JsValue::Undefined);
                    result.push(elem);
                }
            }
        } else {
            result.push(value);
        }
    }

    add_elements(&mut result, this, &spreadable_key);

    for arg in args {
        add_elements(&mut result, arg.clone(), &spreadable_key);
    }

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, result);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_join(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.join called on non-object",
        ));
    };

    let separator = match args.first() {
        Some(v) if !matches!(v, JsValue::Undefined) => interp.to_js_string(v).to_string(),
        _ => ",".to_string(),
    };

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    let mut parts = Vec::with_capacity(length as usize);
    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let part = match elem {
            JsValue::Undefined | JsValue::Null => String::new(),
            _ => interp.to_js_string(&elem).to_string(),
        };
        parts.push(part);
    }

    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        parts.join(&separator),
    ))))
}

/// Array.prototype.toString()
/// Returns a string of comma-separated elements (equivalent to join()).
pub fn array_to_string(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    // toString() is equivalent to join() with default separator
    array_join(interp, this, &[])
}

pub fn array_every(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.every called on non-object",
        ));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error(
            "Array.prototype.every callback is not a function",
        ));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Guard values to prevent GC from collecting them during iterations
    let _callback_guard = interp.guard_value(&callback);
    let _this_arg_guard = interp.guard_value(&this_arg);
    let _arr_guard = interp.guard_value(&this);

    // Use array-like length with full ToLength coercion (works on both arrays and array-like objects)
    let length = get_array_like_length(interp, &arr)?;

    for i in 0..length {
        if has_array_like_element(&arr, i) {
            let elem = get_array_like_element(&arr, i);

            let Guarded {
                value: result,
                guard: _result_guard,
            } = interp.call_function(
                callback.clone(),
                this_arg.clone(),
                &[elem, JsValue::Number(i as f64), this.clone()],
            )?;

            if !result.to_boolean() {
                return Ok(Guarded::unguarded(JsValue::Boolean(false)));
            }
        }
    }

    Ok(Guarded::unguarded(JsValue::Boolean(true)))
}

pub fn array_some(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.some called on non-object",
        ));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error(
            "Array.prototype.some callback is not a function",
        ));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Guard values to prevent GC from collecting them during iterations
    let _callback_guard = interp.guard_value(&callback);
    let _this_arg_guard = interp.guard_value(&this_arg);
    let _arr_guard = interp.guard_value(&this);

    // Use array-like length with full ToLength coercion (works on both arrays and array-like objects)
    let length = get_array_like_length(interp, &arr)?;

    for i in 0..length {
        if has_array_like_element(&arr, i) {
            let elem = get_array_like_element(&arr, i);

            let Guarded {
                value: result,
                guard: _result_guard,
            } = interp.call_function(
                callback.clone(),
                this_arg.clone(),
                &[elem, JsValue::Number(i as f64), this.clone()],
            )?;

            if result.to_boolean() {
                return Ok(Guarded::unguarded(JsValue::Boolean(true)));
            }
        }
    }

    Ok(Guarded::unguarded(JsValue::Boolean(false)))
}

pub fn array_shift(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.shift called on non-object",
        ));
    };

    let mut arr_ref = arr.borrow_mut();
    let elements = arr_ref
        .array_elements_mut()
        .ok_or_else(|| JsError::type_error("Array.prototype.shift called on non-array"))?;

    if elements.is_empty() {
        return Ok(Guarded::unguarded(JsValue::Undefined));
    }

    let first = elements.remove(0);
    Ok(Guarded::unguarded(first))
}

pub fn array_unshift(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.unshift called on non-object",
        ));
    };

    let mut arr_ref = arr.borrow_mut();
    let elements = arr_ref
        .array_elements_mut()
        .ok_or_else(|| JsError::type_error("Array.prototype.unshift called on non-array"))?;

    if args.is_empty() {
        return Ok(Guarded::unguarded(JsValue::Number(elements.len() as f64)));
    }

    // Insert args at the beginning
    for (i, val) in args.iter().enumerate() {
        elements.insert(i, val.clone());
    }

    Ok(Guarded::unguarded(JsValue::Number(elements.len() as f64)))
}

pub fn array_reverse(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.reverse called on non-object",
        ));
    };

    let mut arr_ref = arr.borrow_mut();
    let elements = arr_ref
        .array_elements_mut()
        .ok_or_else(|| JsError::type_error("Array.prototype.reverse called on non-array"))?;

    elements.reverse();

    drop(arr_ref);
    // Array was passed in by caller, already owned - no guard needed
    Ok(Guarded::unguarded(this))
}

pub fn array_sort(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.sort called on non-object",
        ));
    };

    let compare_fn = args.first().cloned();

    // Guard the compare function and array to prevent GC from collecting them
    let _cmp_guard = compare_fn.as_ref().and_then(|c| interp.guard_value(c));
    let _arr_guard = interp.guard_value(&this);

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    let mut elements: Vec<JsValue> = {
        let arr_ref = arr.borrow();
        (0..length)
            .map(|i| {
                arr_ref
                    .get_property(&PropertyKey::Index(i))
                    .unwrap_or(JsValue::Undefined)
            })
            .collect()
    };

    if let Some(cmp) = compare_fn {
        if cmp.is_callable() {
            for i in 0..elements.len() {
                let limit = elements.len().saturating_sub(1 + i);
                for j in 0..limit {
                    // j and j+1 are guaranteed in bounds due to limit calculation
                    let (left, right) = match (elements.get(j), elements.get(j + 1)) {
                        (Some(l), Some(r)) => (l.clone(), r.clone()),
                        _ => continue,
                    };
                    let Guarded {
                        value: result,
                        guard: _result_guard,
                    } = interp.call_function(cmp.clone(), JsValue::Undefined, &[left, right])?;
                    if result.to_number() > 0.0 {
                        elements.swap(j, j + 1);
                    }
                }
            }
        }
    } else {
        // Pre-compute string representations for sorting
        let mut pairs: Vec<(JsString, JsValue)> = elements
            .into_iter()
            .map(|v| {
                let s = interp.to_js_string(&v);
                (s, v)
            })
            .collect();
        pairs.sort_by(|(a_str, _), (b_str, _)| a_str.as_str().cmp(b_str.as_str()));
        elements = pairs.into_iter().map(|(_, v)| v).collect();
    }

    {
        let mut arr_ref = arr.borrow_mut();
        for (i, val) in elements.into_iter().enumerate() {
            arr_ref.set_property(PropertyKey::Index(i as u32), val);
        }
    }

    // Return with guard to protect the array until caller stores it
    // This is necessary because the array might have been created inline in a chain
    let guard = interp.guard_value(&this);
    Ok(Guarded { value: this, guard })
}

pub fn array_fill(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.fill called on non-object",
        ));
    };

    let value = args.first().cloned().unwrap_or(JsValue::Undefined);

    let mut arr_ref = arr.borrow_mut();
    let elements = arr_ref
        .array_elements_mut()
        .ok_or_else(|| JsError::type_error("Array.prototype.fill called on non-array"))?;
    let length = elements.len() as i64;

    let start = args
        .get(1)
        .map(|v| {
            let n = v.to_number() as i64;
            if n < 0 {
                (length + n).max(0)
            } else {
                n.min(length)
            }
        })
        .unwrap_or(0) as usize;

    let end = args
        .get(2)
        .map(|v| {
            let n = v.to_number() as i64;
            if n < 0 {
                (length + n).max(0)
            } else {
                n.min(length)
            }
        })
        .unwrap_or(length) as usize;

    for i in start..end {
        if let Some(slot) = elements.get_mut(i) {
            *slot = value.clone();
        }
    }

    drop(arr_ref);
    // Array was passed in by caller, already owned - no guard needed
    Ok(Guarded::unguarded(this))
}

pub fn array_copy_within(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.copyWithin called on non-object",
        ));
    };

    let mut arr_ref = arr.borrow_mut();
    let elements = arr_ref
        .array_elements_mut()
        .ok_or_else(|| JsError::type_error("Array.prototype.copyWithin called on non-array"))?;
    let length = elements.len() as i64;

    let target = args
        .first()
        .map(|v| {
            let n = v.to_number() as i64;
            if n < 0 {
                (length + n).max(0)
            } else {
                n.min(length)
            }
        })
        .unwrap_or(0) as usize;

    let start = args
        .get(1)
        .map(|v| {
            let n = v.to_number() as i64;
            if n < 0 {
                (length + n).max(0)
            } else {
                n.min(length)
            }
        })
        .unwrap_or(0) as usize;

    let end = args
        .get(2)
        .map(|v| {
            let n = v.to_number() as i64;
            if n < 0 {
                (length + n).max(0)
            } else {
                n.min(length)
            }
        })
        .unwrap_or(length) as usize;

    // Copy elements to temporary Vec first to avoid borrow issues
    let copied: Vec<JsValue> = elements.get(start..end).unwrap_or_default().to_vec();

    for (i, val) in copied.into_iter().enumerate() {
        let target_idx = target + i;
        if let Some(slot) = elements.get_mut(target_idx) {
            *slot = val;
        }
    }

    drop(arr_ref);
    // Array was passed in by caller, already owned - no guard needed
    Ok(Guarded::unguarded(this))
}

pub fn array_splice(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.splice called on non-object",
        ));
    };

    let mut arr_ref = arr.borrow_mut();
    let elements = arr_ref
        .array_elements_mut()
        .ok_or_else(|| JsError::type_error("Array.prototype.splice called on non-array"))?;
    let length = elements.len() as i64;

    let start = args
        .first()
        .map(|v| {
            let n = v.to_number() as i64;
            if n < 0 {
                (length + n).max(0)
            } else {
                n.min(length)
            }
        })
        .unwrap_or(0) as usize;

    let delete_count = args
        .get(1)
        .map(|v| {
            let n = v.to_number() as i64;
            n.max(0).min(length - start as i64) as usize
        })
        .unwrap_or((length - start as i64) as usize);

    // Remove elements and collect them
    let removed: Vec<JsValue> = elements.drain(start..start + delete_count).collect();

    // Insert new items
    let insert_items: Vec<JsValue> = args.iter().skip(2).cloned().collect();
    for (i, val) in insert_items.into_iter().enumerate() {
        elements.insert(start + i, val);
    }

    drop(arr_ref);
    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, removed);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_of(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, args.to_vec());
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_from(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let source = args.first().cloned().unwrap_or(JsValue::Undefined);
    let map_fn = args.get(1).cloned();

    // Guard the source and map_fn to prevent GC from collecting them
    // during the loop iterations which may trigger allocations.
    let _source_guard = interp.guard_value(&source);
    let _map_fn_guard = map_fn.as_ref().and_then(|m| interp.guard_value(m));

    let mut elements = Vec::new();

    match source {
        JsValue::Object(obj) => {
            // First check if object is an array (fast path)
            let is_array = obj.borrow().array_elements().is_some();
            if is_array {
                let source_elements: Vec<JsValue> = obj
                    .borrow()
                    .array_elements()
                    .map(|e| e.to_vec())
                    .unwrap_or_default();
                for (i, elem) in source_elements.into_iter().enumerate() {
                    let mapped = if let Some(ref map) = map_fn {
                        if map.is_callable() {
                            let Guarded {
                                value: mapped_val,
                                guard: _mapped_guard,
                            } = interp.call_function(
                                map.clone(),
                                JsValue::Undefined,
                                &[elem, JsValue::Number(i as f64)],
                            )?;
                            mapped_val
                        } else {
                            elem
                        }
                    } else {
                        elem
                    };
                    elements.push(mapped);
                }
            } else {
                // Check for Symbol.iterator (handles Map, Set, and other iterables)
                let well_known = interp.well_known_symbols;
                let iterator_symbol = crate::value::JsSymbol::new(
                    well_known.iterator,
                    Some(interp.intern("Symbol.iterator")),
                );
                let iterator_key = PropertyKey::Symbol(Box::new(iterator_symbol));
                let iterator_method = obj.borrow().get_property(&iterator_key);

                if let Some(JsValue::Object(method_obj)) = iterator_method {
                    // Call the iterator method to get the iterator object
                    let Guarded {
                        value: iterator_val,
                        guard: _iter_guard,
                    } = interp.call_function(
                        JsValue::Object(method_obj),
                        JsValue::Object(obj.cheap_clone()),
                        &[],
                    )?;

                    if let JsValue::Object(iterator_obj) = iterator_val {
                        // Get the next method
                        let next_key = interp.property_key("next");
                        let next_method = iterator_obj.borrow().get_property(&next_key);

                        if let Some(JsValue::Object(next_fn)) = next_method {
                            // Iterate until done
                            let mut i = 0usize;
                            loop {
                                let Guarded {
                                    value: result_val,
                                    guard: _result_guard,
                                } = interp.call_function(
                                    JsValue::Object(next_fn.cheap_clone()),
                                    JsValue::Object(iterator_obj.cheap_clone()),
                                    &[],
                                )?;

                                if let JsValue::Object(result_obj) = result_val {
                                    let done_key = interp.property_key("done");
                                    let value_key = interp.property_key("value");

                                    let done = result_obj
                                        .borrow()
                                        .get_property(&done_key)
                                        .map(|v| v.to_boolean())
                                        .unwrap_or(false);

                                    if done {
                                        break;
                                    }

                                    let elem = result_obj
                                        .borrow()
                                        .get_property(&value_key)
                                        .unwrap_or(JsValue::Undefined);

                                    let mapped = if let Some(ref map) = map_fn {
                                        if map.is_callable() {
                                            let Guarded {
                                                value: mapped_val,
                                                guard: _mapped_guard,
                                            } = interp.call_function(
                                                map.clone(),
                                                JsValue::Undefined,
                                                &[elem, JsValue::Number(i as f64)],
                                            )?;
                                            mapped_val
                                        } else {
                                            elem
                                        }
                                    } else {
                                        elem
                                    };
                                    elements.push(mapped);
                                    i += 1;
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }
                // If no iterator, elements remains empty (array-like objects would need length property)
            }
        }
        JsValue::String(s) => {
            for (i, ch) in s.as_str().chars().enumerate() {
                let elem = JsValue::String(JsString::from(ch.to_string()));
                let mapped = if let Some(ref map) = map_fn {
                    if map.is_callable() {
                        let Guarded {
                            value: mapped_val,
                            guard: _mapped_guard,
                        } = interp.call_function(
                            map.clone(),
                            JsValue::Undefined,
                            &[elem, JsValue::Number(i as f64)],
                        )?;
                        mapped_val
                    } else {
                        elem
                    }
                } else {
                    elem
                };
                elements.push(mapped);
            }
        }
        _ => {}
    }

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, elements);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_at(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.at called on non-object",
        ));
    };

    // ToIntegerOrInfinity on the index argument (calls valueOf if object)
    let index = match args.first() {
        Some(v) => interp.coerce_to_number(v)? as i64,
        None => 0,
    };

    let arr_ref = arr.borrow();
    let length = arr_ref
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))? as i64;

    let actual_index = if index < 0 { length + index } else { index };

    if actual_index < 0 || actual_index >= length {
        return Ok(Guarded::unguarded(JsValue::Undefined));
    }

    // Value came from array which is owned by caller - no guard needed
    Ok(Guarded::unguarded(
        arr_ref
            .get_property(&PropertyKey::Index(actual_index as u32))
            .unwrap_or(JsValue::Undefined),
    ))
}

pub fn array_last_index_of(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.lastIndexOf called on non-object",
        ));
    };

    let search_elem = args.first().cloned().unwrap_or(JsValue::Undefined);

    let arr_ref = arr.borrow();
    let length = arr_ref
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    let from_index = args
        .get(1)
        .map(|v| {
            let n = v.to_number() as i64;
            if n < 0 {
                (length as i64 + n).max(-1)
            } else {
                n.min(length as i64 - 1)
            }
        })
        .unwrap_or(length as i64 - 1);

    if from_index < 0 {
        return Ok(Guarded::unguarded(JsValue::Number(-1.0)));
    }

    for i in (0..=from_index as u32).rev() {
        let elem = arr_ref
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);
        if elem.strict_equals(&search_elem) {
            return Ok(Guarded::unguarded(JsValue::Number(i as f64)));
        }
    }

    Ok(Guarded::unguarded(JsValue::Number(-1.0)))
}

pub fn array_reduce_right(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.reduceRight called on non-object",
        ));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error(
            "Array.prototype.reduceRight callback is not a function",
        ));
    }

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    if length == 0 && args.get(1).is_none() {
        return Err(JsError::type_error(
            "Reduce of empty array with no initial value",
        ));
    }

    let (mut accumulator, start_index) = if let Some(initial) = args.get(1) {
        (initial.clone(), length as i64 - 1)
    } else {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(length - 1))
            .unwrap_or(JsValue::Undefined);
        (elem, length as i64 - 2)
    };

    for i in (0..=start_index).rev() {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i as u32))
            .unwrap_or(JsValue::Undefined);
        let Guarded {
            value: result,
            guard: _result_guard,
        } = interp.call_function(
            callback.clone(),
            JsValue::Undefined,
            &[
                accumulator.clone(),
                elem,
                JsValue::Number(i as f64),
                this.clone(),
            ],
        )?;
        accumulator = result;
    }

    // Accumulator is a derived value - no guard needed
    Ok(Guarded::unguarded(accumulator))
}

pub fn array_flat(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.flat called on non-object",
        ));
    };

    let depth = args.first().map(|v| v.to_number() as i32).unwrap_or(1);

    fn flatten(arr: &JsObjectRef, depth: i32) -> Vec<JsValue> {
        let elements: Vec<JsValue> = {
            let arr_ref = arr.borrow();
            if let Some(elements) = arr_ref.array_elements() {
                elements.to_vec()
            } else {
                return vec![];
            }
        };

        let mut result = Vec::new();
        for elem in elements {
            if depth > 0
                && let JsValue::Object(ref inner) = elem
                && inner.borrow().is_array()
            {
                result.extend(flatten(inner, depth - 1));
                continue;
            }
            result.push(elem);
        }
        result
    }

    let elements = flatten(&arr, depth);
    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, elements);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_flat_map(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.flatMap called on non-object",
        ));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error(
            "Array.prototype.flatMap callback is not a function",
        ));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Create a single guard that will protect all values throughout the function.
    let guard = interp.heap.create_guard();

    // Guard values to prevent GC from collecting them during iterations
    if let JsValue::Object(obj) = &callback {
        guard.guard(obj.cheap_clone());
    }
    if let JsValue::Object(obj) = &this_arg {
        guard.guard(obj.cheap_clone());
    }
    guard.guard(arr.cheap_clone());

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    let mut result = Vec::new();

    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let Guarded {
            value: mapped,
            guard: _mapped_guard,
        } = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem, JsValue::Number(i as f64), this.clone()],
        )?;

        let is_array = if let JsValue::Object(ref inner) = mapped {
            let inner_ref = inner.borrow();
            if let Some(elements) = inner_ref.array_elements() {
                // Guard each element being added to result
                for el in elements.iter() {
                    if let JsValue::Object(obj) = el {
                        guard.guard(obj.cheap_clone());
                    }
                }
                result.extend(elements.iter().cloned());
                true
            } else {
                false
            }
            // inner_ref borrow ends here
        } else {
            false
        };

        if !is_array {
            // Guard the mapped value to keep it alive until we create the result array.
            if let JsValue::Object(obj) = &mapped {
                guard.guard(obj.cheap_clone());
            }
            result.push(mapped);
        }
    }

    let arr = interp.create_array_from(&guard, result);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_find_last(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.findLast called on non-object",
        ));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error(
            "Array.prototype.findLast callback is not a function",
        ));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    for i in (0..length).rev() {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let Guarded {
            value: result,
            guard: _result_guard,
        } = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem.clone(), JsValue::Number(i as f64), this.clone()],
        )?;

        if result.to_boolean() {
            // Element came from array which is owned by caller - no guard needed
            return Ok(Guarded::unguarded(elem));
        }
    }

    Ok(Guarded::unguarded(JsValue::Undefined))
}

pub fn array_find_last_index(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.findLastIndex called on non-object",
        ));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error(
            "Array.prototype.findLastIndex callback is not a function",
        ));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    for i in (0..length).rev() {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let Guarded {
            value: result,
            guard: _result_guard,
        } = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem, JsValue::Number(i as f64), this.clone()],
        )?;

        if result.to_boolean() {
            return Ok(Guarded::unguarded(JsValue::Number(i as f64)));
        }
    }

    Ok(Guarded::unguarded(JsValue::Number(-1.0)))
}

pub fn array_to_reversed(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.toReversed called on non-object",
        ));
    };

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    let elements: Vec<JsValue> = (0..length)
        .rev()
        .map(|i| {
            arr.borrow()
                .get_property(&PropertyKey::Index(i))
                .unwrap_or(JsValue::Undefined)
        })
        .collect();

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, elements);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_to_sorted(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.toSorted called on non-object",
        ));
    };

    let comparator = args.first().cloned();

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    let mut elements: Vec<JsValue> = (0..length)
        .map(|i| {
            arr.borrow()
                .get_property(&PropertyKey::Index(i))
                .unwrap_or(JsValue::Undefined)
        })
        .collect();

    if let Some(ref cmp_fn) = comparator {
        if cmp_fn.is_callable() {
            let cmp_fn = cmp_fn.clone();
            let mut i = 0;
            while i < elements.len() {
                let mut j = i;
                while j > 0 {
                    // j > 0 guarantees j-1 is valid, and j < elements.len() from outer loop
                    let (left, right) = match (elements.get(j - 1), elements.get(j)) {
                        (Some(l), Some(r)) => (l.clone(), r.clone()),
                        _ => break,
                    };
                    let Guarded {
                        value: cmp_result,
                        guard: _cmp_guard,
                    } = interp.call_function(cmp_fn.clone(), JsValue::Undefined, &[left, right])?;
                    let cmp = cmp_result.to_number();
                    if cmp > 0.0 {
                        elements.swap(j - 1, j);
                        j -= 1;
                    } else {
                        break;
                    }
                }
                i += 1;
            }
        }
    } else {
        // Pre-compute string representations for sorting
        let mut pairs: Vec<(JsString, JsValue)> = elements
            .into_iter()
            .map(|v| {
                let s = interp.to_js_string(&v);
                (s, v)
            })
            .collect();
        pairs.sort_by(|(a_str, _), (b_str, _)| a_str.as_str().cmp(b_str.as_str()));
        elements = pairs.into_iter().map(|(_, v)| v).collect();
    }

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, elements);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_to_spliced(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.toSpliced called on non-object",
        ));
    };

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))? as i32;

    let start_arg = args.first().map(|v| v.to_number() as i32).unwrap_or(0);
    let start = if start_arg < 0 {
        (length + start_arg).max(0) as u32
    } else {
        (start_arg as u32).min(length as u32)
    };

    let delete_count = args
        .get(1)
        .map(|v| (v.to_number() as i32).max(0) as u32)
        .unwrap_or((length as u32).saturating_sub(start));
    let delete_count = delete_count.min(length as u32 - start);

    let mut result: Vec<JsValue> = (0..start)
        .map(|i| {
            arr.borrow()
                .get_property(&PropertyKey::Index(i))
                .unwrap_or(JsValue::Undefined)
        })
        .collect();

    for arg in args.iter().skip(2) {
        result.push(arg.clone());
    }

    for i in (start + delete_count)..(length as u32) {
        result.push(
            arr.borrow()
                .get_property(&PropertyKey::Index(i))
                .unwrap_or(JsValue::Undefined),
        );
    }

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, result);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_with(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.with called on non-object",
        ));
    };

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))? as i32;

    let index_arg = args.first().map(|v| v.to_number() as i32).unwrap_or(0);
    let index = if index_arg < 0 {
        length + index_arg
    } else {
        index_arg
    };

    if index < 0 || index >= length {
        return Err(JsError::range_error("Invalid index"));
    }

    let value = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let elements: Vec<JsValue> = (0..length as u32)
        .map(|i| {
            if i == index as u32 {
                value.clone()
            } else {
                arr.borrow()
                    .get_property(&PropertyKey::Index(i))
                    .unwrap_or(JsValue::Undefined)
            }
        })
        .collect();

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, elements);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_keys(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.keys called on non-object",
        ));
    };

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    let keys: Vec<JsValue> = (0..length).map(|i| JsValue::Number(i as f64)).collect();
    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, keys);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_values(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.values called on non-object",
        ));
    };

    // Create an iterator object that has the array and current index
    let guard = interp.heap.create_guard();
    guard.guard(arr.cheap_clone());
    let iter_obj = interp.create_object_raw(&guard);

    // Store the array and current index on the iterator
    let array_key = interp.property_key("__array__");
    let index_key = interp.property_key("__index__");
    let next_key = interp.property_key("next");

    iter_obj
        .borrow_mut()
        .set_property(array_key, JsValue::Object(arr));
    iter_obj
        .borrow_mut()
        .set_property(index_key, JsValue::Number(0.0));

    // Add next() method
    let next_fn = interp.create_native_function("next", array_iterator_next, 0);
    guard.guard(next_fn.cheap_clone());
    iter_obj
        .borrow_mut()
        .set_property(next_key, JsValue::Object(next_fn));

    // Make it its own iterator (for use in for-of)
    let well_known = interp.well_known_symbols;
    let iterator_symbol =
        crate::value::JsSymbol::new(well_known.iterator, Some(interp.intern("Symbol.iterator")));
    let iterator_key = PropertyKey::Symbol(Box::new(iterator_symbol));
    let self_fn = interp.create_native_function("[Symbol.iterator]", return_this, 0);
    guard.guard(self_fn.cheap_clone());
    iter_obj
        .borrow_mut()
        .set_property(iterator_key, JsValue::Object(self_fn));

    Ok(Guarded::with_guard(JsValue::Object(iter_obj), guard))
}

/// Helper function that returns `this` - used for iterator[Symbol.iterator]()
fn return_this(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    Ok(Guarded::unguarded(this))
}

/// next() method for array iterators
fn array_iterator_next(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(iter_obj) = this else {
        return Err(JsError::type_error("next called on non-object"));
    };

    // Pre-create property keys
    let array_key = interp.property_key("__array__");
    let index_key = interp.property_key("__index__");
    let value_key = interp.property_key("value");
    let done_key = interp.property_key("done");

    // Get the array and current index
    let arr = iter_obj.borrow().get_property(&array_key);
    let index = iter_obj.borrow().get_property(&index_key);

    let Some(JsValue::Object(arr)) = arr else {
        return Err(JsError::type_error("Invalid array iterator"));
    };

    let index = match index {
        Some(JsValue::Number(n)) => n as u32,
        _ => 0,
    };

    // Check if this is a proxy - if so, get length through proxy
    let is_proxy = matches!(arr.borrow().exotic, crate::value::ExoticObject::Proxy(_));

    let length = if is_proxy {
        // Get length through proxy trap
        let length_key = interp.property_key("length");
        let length_result = super::proxy::proxy_get(
            interp,
            arr.cheap_clone(),
            length_key,
            JsValue::Object(arr.cheap_clone()),
        )?;
        match length_result.value {
            JsValue::Number(n) => n as u32,
            _ => 0,
        }
    } else {
        arr.borrow().array_length().unwrap_or(0)
    };

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
        // Get value through proxy if needed
        let value = if is_proxy {
            let result = super::proxy::proxy_get(
                interp,
                arr.cheap_clone(),
                PropertyKey::Index(index),
                JsValue::Object(arr.cheap_clone()),
            )?;
            result.value
        } else {
            arr.borrow()
                .get_property(&PropertyKey::Index(index))
                .unwrap_or(JsValue::Undefined)
        };

        // Re-create the key since it was consumed above
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

pub fn array_entries(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.entries called on non-object",
        ));
    };

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    // Use single guard for all entry arrays - keeps them alive until result array is created
    let guard = interp.heap.create_guard();
    let mut entries = Vec::with_capacity(length as usize);
    for i in 0..length {
        let value = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);
        let pair = vec![JsValue::Number(i as f64), value];
        let entry_arr = interp.create_array_from(&guard, pair);
        entries.push(JsValue::Object(entry_arr));
    }

    let result = interp.create_array_from(&guard, entries);
    Ok(Guarded::with_guard(JsValue::Object(result), guard))
}
