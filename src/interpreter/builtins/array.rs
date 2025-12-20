//! Array built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{ExoticObject, Guarded, JsObjectRef, JsString, JsValue, PropertyKey};

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

    constructor
}

pub fn array_constructor_fn(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    if args.len() == 1 {
        if let Some(JsValue::Number(n)) = args.first() {
            let len = *n as u32;
            let mut elements = Vec::with_capacity(len as usize);
            for _ in 0..len {
                elements.push(JsValue::Undefined);
            }
            let guard = interp.heap.create_guard();
            let arr = interp.create_array_from(&guard, elements);
            return Ok(Guarded::with_guard(JsValue::Object(arr), guard));
        }
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

    // Guard the callback and this_arg to prevent GC from collecting them
    // during the loop iterations which may trigger allocations.
    let _callback_guard = interp.guard_value(&callback);
    let _this_arg_guard = interp.guard_value(&this_arg);
    let _arr_guard = interp.guard_value(&this);

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    let mut result = Vec::with_capacity(length as usize);
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

        result.push(mapped);
    }

    let guard = interp.heap.create_guard();
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

    // Guard the callback and this_arg to prevent GC from collecting them
    // during the loop iterations which may trigger allocations.
    let _callback_guard = interp.guard_value(&callback);
    let _this_arg_guard = interp.guard_value(&this_arg);
    let _arr_guard = interp.guard_value(&this);

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
            value: keep,
            guard: _keep_guard,
        } = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem.clone(), JsValue::Number(i as f64), this.clone()],
        )?;

        if keep.to_boolean() {
            result.push(elem);
        }
    }

    let guard = interp.heap.create_guard();
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

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem, JsValue::Number(i as f64), this.clone()],
        )?;
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

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    let (mut accumulator, start_index) = if let Some(initial) = args.get(1) {
        (initial.clone(), 0)
    } else {
        if length == 0 {
            return Err(JsError::type_error(
                "Reduce of empty array with no initial value",
            ));
        }
        let first = arr
            .borrow()
            .get_property(&PropertyKey::Index(0))
            .unwrap_or(JsValue::Undefined);
        (first, 1)
    };

    for i in start_index..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

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

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    for i in 0..length {
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

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    for i in 0..length {
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

pub fn array_index_of(
    _interp: &mut Interpreter,
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

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))? as i64;

    let start = if from_index < 0 {
        (length + from_index).max(0) as u32
    } else {
        from_index.min(length) as u32
    };

    for i in start..(length as u32) {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        if elem.strict_equals(&search_element) {
            return Ok(Guarded::unguarded(JsValue::Number(i as f64)));
        }
    }

    Ok(Guarded::unguarded(JsValue::Number(-1.0)))
}

pub fn array_includes(
    _interp: &mut Interpreter,
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

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))? as i64;

    let start = if from_index < 0 {
        (length + from_index).max(0) as u32
    } else {
        from_index.min(length) as u32
    };

    for i in start..(length as u32) {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let found = match (&elem, &search_element) {
            (JsValue::Number(a), JsValue::Number(b)) if a.is_nan() && b.is_nan() => true,
            _ => elem.strict_equals(&search_element),
        };

        if found {
            return Ok(Guarded::unguarded(JsValue::Boolean(true)));
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

    fn add_elements(result: &mut Vec<JsValue>, value: JsValue) {
        match &value {
            JsValue::Object(obj) => {
                let obj_ref = obj.borrow();
                if let Some(length) = obj_ref.array_length() {
                    for i in 0..length {
                        let elem = obj_ref
                            .get_property(&PropertyKey::Index(i))
                            .unwrap_or(JsValue::Undefined);
                        result.push(elem);
                    }
                } else {
                    result.push(value.clone());
                }
            }
            _ => result.push(value),
        }
    }

    add_elements(&mut result, this);

    for arg in args {
        add_elements(&mut result, arg.clone());
    }

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, result);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn array_join(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.join called on non-object",
        ));
    };

    let separator = args
        .first()
        .map(|v| {
            if matches!(v, JsValue::Undefined) {
                ",".to_string()
            } else {
                v.to_js_string().to_string()
            }
        })
        .unwrap_or_else(|| ",".to_string());

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
            _ => elem.to_js_string().to_string(),
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

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    for i in 0..length {
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

        if !result.to_boolean() {
            return Ok(Guarded::unguarded(JsValue::Boolean(false)));
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

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    for i in 0..length {
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
            return Ok(Guarded::unguarded(JsValue::Boolean(true)));
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
        elements.sort_by(|a, b| {
            let a_str = a.to_js_string();
            let b_str = b.to_js_string();
            a_str.as_str().cmp(b_str.as_str())
        });
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
            let source_elements: Vec<JsValue> = {
                let obj_ref = obj.borrow();
                if let Some(elements) = obj_ref.array_elements() {
                    elements.to_vec()
                } else {
                    vec![]
                }
            };

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
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.at called on non-object",
        ));
    };

    let arr_ref = arr.borrow();
    let length = arr_ref
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))? as i64;

    let index = args.first().map(|v| v.to_number() as i64).unwrap_or(0);

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
            if depth > 0 {
                if let JsValue::Object(ref inner) = elem {
                    if inner.borrow().is_array() {
                        result.extend(flatten(inner, depth - 1));
                        continue;
                    }
                }
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

    // Guard values to prevent GC from collecting them during iterations
    let _callback_guard = interp.guard_value(&callback);
    let _this_arg_guard = interp.guard_value(&this_arg);
    let _arr_guard = interp.guard_value(&this);

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
            guard: mapped_guard,
        } = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem, JsValue::Number(i as f64), this.clone()],
        )?;

        let is_array = if let JsValue::Object(ref inner) = mapped {
            let inner_ref = inner.borrow();
            if let Some(elements) = inner_ref.array_elements() {
                result.extend(elements.iter().cloned());
                true
            } else {
                false
            }
            // inner_ref borrow ends here
        } else {
            false
        };

        // Now safe to drop guard since borrows are released
        drop(mapped_guard);

        if !is_array {
            result.push(mapped);
        }
    }

    let guard = interp.heap.create_guard();
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
        elements.sort_by(|a, b| {
            let a_str = a.to_js_string();
            let b_str = b.to_js_string();
            a_str.as_str().cmp(b_str.as_str())
        });
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

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Not an array"))?;

    let values: Vec<JsValue> = (0..length)
        .map(|i| {
            arr.borrow()
                .get_property(&PropertyKey::Index(i))
                .unwrap_or(JsValue::Undefined)
        })
        .collect();
    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, values);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
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
