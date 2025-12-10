//! Array built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{
    create_array, create_function, create_object, register_method, CheapClone, ExoticObject,
    JsFunction, JsObjectRef, JsString, JsValue, NativeFunction, Property, PropertyKey,
};

/// Create Array.prototype with all array methods
pub fn create_array_prototype() -> JsObjectRef {
    let proto = create_object();
    {
        let mut p = proto.borrow_mut();

        // Mutating methods
        register_method(&mut p, "push", array_push, 1);
        register_method(&mut p, "pop", array_pop, 0);
        register_method(&mut p, "shift", array_shift, 0);
        register_method(&mut p, "unshift", array_unshift, 1);
        register_method(&mut p, "splice", array_splice, 2);
        register_method(&mut p, "reverse", array_reverse, 0);
        register_method(&mut p, "sort", array_sort, 1);
        register_method(&mut p, "fill", array_fill, 3);
        register_method(&mut p, "copyWithin", array_copy_within, 3);

        // Accessor methods
        register_method(&mut p, "at", array_at, 1);
        register_method(&mut p, "concat", array_concat, 1);
        register_method(&mut p, "slice", array_slice, 2);
        register_method(&mut p, "join", array_join, 1);
        register_method(&mut p, "indexOf", array_index_of, 1);
        register_method(&mut p, "lastIndexOf", array_last_index_of, 1);
        register_method(&mut p, "includes", array_includes, 1);

        // Iteration methods
        register_method(&mut p, "forEach", array_foreach, 1);
        register_method(&mut p, "map", array_map, 1);
        register_method(&mut p, "filter", array_filter, 1);
        register_method(&mut p, "reduce", array_reduce, 1);
        register_method(&mut p, "reduceRight", array_reduce_right, 1);
        register_method(&mut p, "find", array_find, 1);
        register_method(&mut p, "findIndex", array_find_index, 1);
        register_method(&mut p, "findLast", array_find_last, 1);
        register_method(&mut p, "findLastIndex", array_find_last_index, 1);
        register_method(&mut p, "every", array_every, 1);
        register_method(&mut p, "some", array_some, 1);
        register_method(&mut p, "flat", array_flat, 1);
        register_method(&mut p, "flatMap", array_flat_map, 1);

        // Non-mutating methods (ES2023+)
        register_method(&mut p, "toReversed", array_to_reversed, 0);
        register_method(&mut p, "toSorted", array_to_sorted, 1);
        register_method(&mut p, "toSpliced", array_to_spliced, 2);
        register_method(&mut p, "with", array_with, 2);

        // Iterator methods
        register_method(&mut p, "keys", array_keys, 0);
        register_method(&mut p, "values", array_values, 0);
        register_method(&mut p, "entries", array_entries, 0);
    }
    proto
}

/// Create Array constructor with static methods (isArray, of, from)
pub fn create_array_constructor(array_prototype: &JsObjectRef) -> JsObjectRef {
    let constructor = create_function(JsFunction::Native(NativeFunction {
        name: "Array".to_string(),
        func: array_constructor_fn,
        arity: 0,
    }));
    {
        let mut arr = constructor.borrow_mut();

        register_method(&mut arr, "isArray", array_is_array, 1);
        register_method(&mut arr, "of", array_of, 0);
        register_method(&mut arr, "from", array_from, 1);

        arr.set_property(
            PropertyKey::from("prototype"),
            JsValue::Object(array_prototype.cheap_clone()),
        );
    }
    constructor
}

pub fn array_constructor_fn(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    if args.len() == 1 {
        if let Some(JsValue::Number(n)) = args.first() {
            let len = *n as u32;
            let mut elements = Vec::with_capacity(len as usize);
            for _ in 0..len {
                elements.push(JsValue::Undefined);
            }
            return Ok(JsValue::Object(create_array(elements)));
        }
    }
    Ok(JsValue::Object(create_array(args.to_vec())))
}

pub fn array_is_array(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    let is_array = match value {
        JsValue::Object(obj) => matches!(obj.borrow().exotic, ExoticObject::Array { .. }),
        _ => false,
    };
    Ok(JsValue::Boolean(is_array))
}

pub fn array_push(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.push called on non-object",
        ));
    };

    let mut arr_ref = arr.borrow_mut();

    let mut current_length = match &arr_ref.exotic {
        ExoticObject::Array { length } => *length,
        _ => {
            return Err(JsError::type_error(
                "Array.prototype.push called on non-array",
            ))
        }
    };

    for arg in args {
        arr_ref.properties.insert(
            PropertyKey::Index(current_length),
            Property::data(arg.clone()),
        );
        current_length += 1;
    }

    if let ExoticObject::Array { ref mut length } = arr_ref.exotic {
        *length = current_length;
    }

    arr_ref.properties.insert(
        PropertyKey::from("length"),
        Property::with_attributes(JsValue::Number(current_length as f64), true, false, false),
    );

    Ok(JsValue::Number(current_length as f64))
}

pub fn array_pop(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.pop called on non-object",
        ));
    };

    let mut arr_ref = arr.borrow_mut();

    let current_length = match &arr_ref.exotic {
        ExoticObject::Array { length } => *length,
        _ => {
            return Err(JsError::type_error(
                "Array.prototype.pop called on non-array",
            ))
        }
    };

    if current_length == 0 {
        return Ok(JsValue::Undefined);
    }

    let new_length = current_length - 1;

    let value = arr_ref
        .properties
        .remove(&PropertyKey::Index(new_length))
        .map(|p| p.value)
        .unwrap_or(JsValue::Undefined);

    if let ExoticObject::Array { ref mut length } = arr_ref.exotic {
        *length = new_length;
    }

    arr_ref.properties.insert(
        PropertyKey::from("length"),
        Property::with_attributes(JsValue::Number(new_length as f64), true, false, false),
    );

    Ok(value)
}

pub fn array_map(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    let mut result = Vec::with_capacity(length as usize);
    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let mapped = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem, JsValue::Number(i as f64), this.clone()],
        )?;

        result.push(mapped);
    }

    Ok(JsValue::Object(interp.create_array(result)))
}

pub fn array_filter(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    let mut result = Vec::new();
    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let keep = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem.clone(), JsValue::Number(i as f64), this.clone()],
        )?;

        if keep.to_boolean() {
            result.push(elem);
        }
    }

    Ok(JsValue::Object(interp.create_array(result)))
}

pub fn array_foreach(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

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

    Ok(JsValue::Undefined)
}

pub fn array_reduce(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

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

        accumulator = interp.call_function(
            callback.clone(),
            JsValue::Undefined,
            &[accumulator, elem, JsValue::Number(i as f64), this.clone()],
        )?;
    }

    Ok(accumulator)
}

pub fn array_find(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let result = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem.clone(), JsValue::Number(i as f64), this.clone()],
        )?;

        if result.to_boolean() {
            return Ok(elem);
        }
    }

    Ok(JsValue::Undefined)
}

pub fn array_find_index(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let result = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem, JsValue::Number(i as f64), this.clone()],
        )?;

        if result.to_boolean() {
            return Ok(JsValue::Number(i as f64));
        }
    }

    Ok(JsValue::Number(-1.0))
}

pub fn array_index_of(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.indexOf called on non-object",
        ));
    };

    let search_element = args.first().cloned().unwrap_or(JsValue::Undefined);
    let from_index = args.get(1).map(|v| v.to_number() as i64).unwrap_or(0);

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length as i64,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

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
            return Ok(JsValue::Number(i as f64));
        }
    }

    Ok(JsValue::Number(-1.0))
}

pub fn array_includes(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.includes called on non-object",
        ));
    };

    let search_element = args.first().cloned().unwrap_or(JsValue::Undefined);
    let from_index = args.get(1).map(|v| v.to_number() as i64).unwrap_or(0);

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length as i64,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

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
            return Ok(JsValue::Boolean(true));
        }
    }

    Ok(JsValue::Boolean(false))
}

pub fn array_slice(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.slice called on non-object",
        ));
    };

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length as i64,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

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

    Ok(JsValue::Object(interp.create_array(result)))
}

pub fn array_concat(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let mut result = Vec::new();

    fn add_elements(result: &mut Vec<JsValue>, value: JsValue) {
        match &value {
            JsValue::Object(obj) => {
                let obj_ref = obj.borrow();
                if let ExoticObject::Array { length } = &obj_ref.exotic {
                    for i in 0..*length {
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

    Ok(JsValue::Object(interp.create_array(result)))
}

pub fn array_join(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

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

    Ok(JsValue::String(JsString::from(parts.join(&separator))))
}

pub fn array_every(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let result = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem, JsValue::Number(i as f64), this.clone()],
        )?;

        if !result.to_boolean() {
            return Ok(JsValue::Boolean(false));
        }
    }

    Ok(JsValue::Boolean(true))
}

pub fn array_some(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let result = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem, JsValue::Number(i as f64), this.clone()],
        )?;

        if result.to_boolean() {
            return Ok(JsValue::Boolean(true));
        }
    }

    Ok(JsValue::Boolean(false))
}

pub fn array_shift(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.shift called on non-object",
        ));
    };

    let mut arr_ref = arr.borrow_mut();
    let length = match &arr_ref.exotic {
        ExoticObject::Array { length } => *length,
        _ => return Err(JsError::type_error("Not an array")),
    };

    if length == 0 {
        return Ok(JsValue::Undefined);
    }

    let first = arr_ref
        .get_property(&PropertyKey::Index(0))
        .unwrap_or(JsValue::Undefined);

    for i in 1..length {
        let val = arr_ref
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);
        arr_ref.set_property(PropertyKey::Index(i - 1), val);
    }

    arr_ref.properties.remove(&PropertyKey::Index(length - 1));
    let new_len = length - 1;
    if let ExoticObject::Array { ref mut length } = arr_ref.exotic {
        *length = new_len;
    }
    arr_ref.set_property(PropertyKey::from("length"), JsValue::Number(new_len as f64));

    Ok(first)
}

pub fn array_unshift(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.unshift called on non-object",
        ));
    };

    let mut arr_ref = arr.borrow_mut();
    let current_length = match &arr_ref.exotic {
        ExoticObject::Array { length } => *length,
        _ => return Err(JsError::type_error("Not an array")),
    };

    let arg_count = args.len() as u32;
    if arg_count == 0 {
        return Ok(JsValue::Number(current_length as f64));
    }

    for i in (0..current_length).rev() {
        let val = arr_ref
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);
        arr_ref.set_property(PropertyKey::Index(i + arg_count), val);
    }

    for (i, val) in args.iter().enumerate() {
        arr_ref.set_property(PropertyKey::Index(i as u32), val.clone());
    }

    let new_length = current_length + arg_count;
    if let ExoticObject::Array { ref mut length } = arr_ref.exotic {
        *length = new_length;
    }
    arr_ref.set_property(
        PropertyKey::from("length"),
        JsValue::Number(new_length as f64),
    );

    Ok(JsValue::Number(new_length as f64))
}

pub fn array_reverse(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.reverse called on non-object",
        ));
    };

    let mut arr_ref = arr.borrow_mut();
    let length = match &arr_ref.exotic {
        ExoticObject::Array { length } => *length,
        _ => return Err(JsError::type_error("Not an array")),
    };

    if length <= 1 {
        return Ok(this);
    }

    let mut elements: Vec<JsValue> = (0..length)
        .map(|i| {
            arr_ref
                .get_property(&PropertyKey::Index(i))
                .unwrap_or(JsValue::Undefined)
        })
        .collect();

    elements.reverse();
    for (i, val) in elements.into_iter().enumerate() {
        arr_ref.set_property(PropertyKey::Index(i as u32), val);
    }

    drop(arr_ref);
    Ok(this)
}

pub fn array_sort(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.sort called on non-object",
        ));
    };

    let compare_fn = args.first().cloned();

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

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
                    let result =
                        interp.call_function(cmp.clone(), JsValue::Undefined, &[left, right])?;
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

    Ok(this)
}

pub fn array_fill(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.fill called on non-object",
        ));
    };

    let value = args.first().cloned().unwrap_or(JsValue::Undefined);

    let mut arr_ref = arr.borrow_mut();
    let length = match &arr_ref.exotic {
        ExoticObject::Array { length } => *length as i64,
        _ => return Err(JsError::type_error("Not an array")),
    };

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
        .unwrap_or(0) as u32;

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
        .unwrap_or(length) as u32;

    for i in start..end {
        arr_ref.set_property(PropertyKey::Index(i), value.clone());
    }

    drop(arr_ref);
    Ok(this)
}

pub fn array_copy_within(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.copyWithin called on non-object",
        ));
    };

    let mut arr_ref = arr.borrow_mut();
    let length = match &arr_ref.exotic {
        ExoticObject::Array { length } => *length as i64,
        _ => return Err(JsError::type_error("Not an array")),
    };

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
        .unwrap_or(0) as u32;

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
        .unwrap_or(0) as u32;

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
        .unwrap_or(length) as u32;

    let elements: Vec<JsValue> = (start..end)
        .map(|i| {
            arr_ref
                .get_property(&PropertyKey::Index(i))
                .unwrap_or(JsValue::Undefined)
        })
        .collect();

    for (i, val) in elements.into_iter().enumerate() {
        let target_idx = target + i as u32;
        if target_idx < length as u32 {
            arr_ref.set_property(PropertyKey::Index(target_idx), val);
        }
    }

    drop(arr_ref);
    Ok(this)
}

pub fn array_splice(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.splice called on non-object",
        ));
    };

    let mut arr_ref = arr.borrow_mut();
    let length = match &arr_ref.exotic {
        ExoticObject::Array { length } => *length as i64,
        _ => return Err(JsError::type_error("Not an array")),
    };

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
        .unwrap_or(0) as u32;

    let delete_count = args
        .get(1)
        .map(|v| {
            let n = v.to_number() as i64;
            n.max(0).min(length - start as i64) as u32
        })
        .unwrap_or((length - start as i64) as u32);

    let removed: Vec<JsValue> = (start..start + delete_count)
        .map(|i| {
            arr_ref
                .get_property(&PropertyKey::Index(i))
                .unwrap_or(JsValue::Undefined)
        })
        .collect();

    let insert_items: Vec<JsValue> = args.iter().skip(2).cloned().collect();
    let insert_count = insert_items.len() as u32;

    let new_length = length as u32 - delete_count + insert_count;

    if insert_count > delete_count {
        let shift = insert_count - delete_count;
        for i in (start + delete_count..length as u32).rev() {
            let val = arr_ref
                .get_property(&PropertyKey::Index(i))
                .unwrap_or(JsValue::Undefined);
            arr_ref.set_property(PropertyKey::Index(i + shift), val);
        }
    } else if insert_count < delete_count {
        let shift = delete_count - insert_count;
        for i in start + delete_count..length as u32 {
            let val = arr_ref
                .get_property(&PropertyKey::Index(i))
                .unwrap_or(JsValue::Undefined);
            arr_ref.set_property(PropertyKey::Index(i - shift), val);
        }
        for i in new_length..length as u32 {
            arr_ref.properties.remove(&PropertyKey::Index(i));
        }
    }

    for (i, val) in insert_items.into_iter().enumerate() {
        arr_ref.set_property(PropertyKey::Index(start + i as u32), val);
    }

    if let ExoticObject::Array { ref mut length } = arr_ref.exotic {
        *length = new_length;
    }
    arr_ref.set_property(
        PropertyKey::from("length"),
        JsValue::Number(new_length as f64),
    );

    drop(arr_ref);
    Ok(JsValue::Object(interp.create_array(removed)))
}

pub fn array_of(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    Ok(JsValue::Object(interp.create_array(args.to_vec())))
}

pub fn array_from(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let source = args.first().cloned().unwrap_or(JsValue::Undefined);
    let map_fn = args.get(1).cloned();

    let mut elements = Vec::new();

    match source {
        JsValue::Object(obj) => {
            let source_elements: Vec<JsValue> = {
                let obj_ref = obj.borrow();
                if let ExoticObject::Array { length } = obj_ref.exotic {
                    (0..length)
                        .map(|i| {
                            obj_ref
                                .get_property(&PropertyKey::Index(i))
                                .unwrap_or(JsValue::Undefined)
                        })
                        .collect()
                } else {
                    vec![]
                }
            };

            for (i, elem) in source_elements.into_iter().enumerate() {
                let mapped = if let Some(ref map) = map_fn {
                    if map.is_callable() {
                        interp.call_function(
                            map.clone(),
                            JsValue::Undefined,
                            &[elem, JsValue::Number(i as f64)],
                        )?
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
                        interp.call_function(
                            map.clone(),
                            JsValue::Undefined,
                            &[elem, JsValue::Number(i as f64)],
                        )?
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

    Ok(JsValue::Object(interp.create_array(elements)))
}

pub fn array_at(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.at called on non-object",
        ));
    };

    let arr_ref = arr.borrow();
    let length = match &arr_ref.exotic {
        ExoticObject::Array { length } => *length as i64,
        _ => return Err(JsError::type_error("Not an array")),
    };

    let index = args.first().map(|v| v.to_number() as i64).unwrap_or(0);

    let actual_index = if index < 0 { length + index } else { index };

    if actual_index < 0 || actual_index >= length {
        return Ok(JsValue::Undefined);
    }

    Ok(arr_ref
        .get_property(&PropertyKey::Index(actual_index as u32))
        .unwrap_or(JsValue::Undefined))
}

pub fn array_last_index_of(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.lastIndexOf called on non-object",
        ));
    };

    let search_elem = args.first().cloned().unwrap_or(JsValue::Undefined);

    let arr_ref = arr.borrow();
    let length = match &arr_ref.exotic {
        ExoticObject::Array { length } => *length,
        _ => return Err(JsError::type_error("Not an array")),
    };

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
        return Ok(JsValue::Number(-1.0));
    }

    for i in (0..=from_index as u32).rev() {
        let elem = arr_ref
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);
        if elem.strict_equals(&search_elem) {
            return Ok(JsValue::Number(i as f64));
        }
    }

    Ok(JsValue::Number(-1.0))
}

pub fn array_reduce_right(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

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
        let result = interp.call_function(
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

    Ok(accumulator)
}

pub fn array_flat(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.flat called on non-object",
        ));
    };

    let depth = args.first().map(|v| v.to_number() as i32).unwrap_or(1);

    fn flatten(arr: &JsObjectRef, depth: i32) -> Vec<JsValue> {
        let elements: Vec<JsValue> = {
            let arr_ref = arr.borrow();
            let length = match &arr_ref.exotic {
                ExoticObject::Array { length } => *length,
                _ => return vec![],
            };
            (0..length)
                .map(|i| {
                    arr_ref
                        .get_property(&PropertyKey::Index(i))
                        .unwrap_or(JsValue::Undefined)
                })
                .collect()
        };

        let mut result = Vec::new();
        for elem in elements {
            if depth > 0 {
                if let JsValue::Object(ref inner) = elem {
                    if matches!(inner.borrow().exotic, ExoticObject::Array { .. }) {
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
    Ok(JsValue::Object(interp.create_array(elements)))
}

pub fn array_flat_map(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    let mut result = Vec::new();

    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let mapped = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem, JsValue::Number(i as f64), this.clone()],
        )?;

        if let JsValue::Object(ref inner) = mapped {
            let inner_ref = inner.borrow();
            if let ExoticObject::Array { length: inner_len } = inner_ref.exotic {
                for j in 0..inner_len {
                    let inner_elem = inner_ref
                        .get_property(&PropertyKey::Index(j))
                        .unwrap_or(JsValue::Undefined);
                    result.push(inner_elem);
                }
                continue;
            }
        }
        result.push(mapped);
    }

    Ok(JsValue::Object(interp.create_array(result)))
}

pub fn array_find_last(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    for i in (0..length).rev() {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let result = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem.clone(), JsValue::Number(i as f64), this.clone()],
        )?;

        if result.to_boolean() {
            return Ok(elem);
        }
    }

    Ok(JsValue::Undefined)
}

pub fn array_find_last_index(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    for i in (0..length).rev() {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let result = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            &[elem, JsValue::Number(i as f64), this.clone()],
        )?;

        if result.to_boolean() {
            return Ok(JsValue::Number(i as f64));
        }
    }

    Ok(JsValue::Number(-1.0))
}

pub fn array_to_reversed(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.toReversed called on non-object",
        ));
    };

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    let elements: Vec<JsValue> = (0..length)
        .rev()
        .map(|i| {
            arr.borrow()
                .get_property(&PropertyKey::Index(i))
                .unwrap_or(JsValue::Undefined)
        })
        .collect();

    Ok(JsValue::Object(interp.create_array(elements)))
}

pub fn array_to_sorted(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error(
            "Array.prototype.toSorted called on non-object",
        ));
    };

    let comparator = args.first().cloned();

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

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
                    let cmp_result =
                        interp.call_function(cmp_fn.clone(), JsValue::Undefined, &[left, right])?;
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

    Ok(JsValue::Object(interp.create_array(elements)))
}

pub fn array_to_spliced(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.toSpliced called on non-object",
        ));
    };

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length as i32,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

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

    Ok(JsValue::Object(interp.create_array(result)))
}

pub fn array_with(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.with called on non-object",
        ));
    };

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length as i32,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

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

    Ok(JsValue::Object(interp.create_array(elements)))
}

pub fn array_keys(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.keys called on non-object",
        ));
    };

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    let keys: Vec<JsValue> = (0..length).map(|i| JsValue::Number(i as f64)).collect();
    Ok(JsValue::Object(interp.create_array(keys)))
}

pub fn array_values(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.values called on non-object",
        ));
    };

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    let values: Vec<JsValue> = (0..length)
        .map(|i| {
            arr.borrow()
                .get_property(&PropertyKey::Index(i))
                .unwrap_or(JsValue::Undefined)
        })
        .collect();
    Ok(JsValue::Object(interp.create_array(values)))
}

pub fn array_entries(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error(
            "Array.prototype.entries called on non-object",
        ));
    };

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    let entries: Vec<JsValue> = (0..length)
        .map(|i| {
            let value = arr
                .borrow()
                .get_property(&PropertyKey::Index(i))
                .unwrap_or(JsValue::Undefined);
            let pair = vec![JsValue::Number(i as f64), value];
            JsValue::Object(interp.create_array(pair))
        })
        .collect();
    Ok(JsValue::Object(interp.create_array(entries)))
}
