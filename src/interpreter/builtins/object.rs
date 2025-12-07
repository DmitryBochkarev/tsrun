//! Object built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{create_array, create_function, create_object, ExoticObject, JsFunction, JsObjectRef, JsString, JsValue, NativeFunction, PropertyKey};

/// Create Object.prototype with hasOwnProperty, toString, valueOf
pub fn create_object_prototype() -> JsObjectRef {
    let proto = create_object();
    {
        let mut p = proto.borrow_mut();

        let hasownprop_fn = create_function(JsFunction::Native(NativeFunction {
            name: "hasOwnProperty".to_string(),
            func: object_has_own_property,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("hasOwnProperty"), JsValue::Object(hasownprop_fn));

        let tostring_fn = create_function(JsFunction::Native(NativeFunction {
            name: "toString".to_string(),
            func: object_to_string,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("toString"), JsValue::Object(tostring_fn));

        let valueof_fn = create_function(JsFunction::Native(NativeFunction {
            name: "valueOf".to_string(),
            func: object_value_of,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("valueOf"), JsValue::Object(valueof_fn));
    }
    proto
}

/// Create Object constructor with static methods (keys, values, entries, assign, etc.)
pub fn create_object_constructor() -> JsObjectRef {
    let constructor = create_function(JsFunction::Native(NativeFunction {
        name: "Object".to_string(),
        func: object_constructor,
        arity: 1,
    }));
    {
        let mut obj = constructor.borrow_mut();

        let keys_fn = create_function(JsFunction::Native(NativeFunction {
            name: "keys".to_string(),
            func: object_keys,
            arity: 1,
        }));
        obj.set_property(PropertyKey::from("keys"), JsValue::Object(keys_fn));

        let values_fn = create_function(JsFunction::Native(NativeFunction {
            name: "values".to_string(),
            func: object_values,
            arity: 1,
        }));
        obj.set_property(PropertyKey::from("values"), JsValue::Object(values_fn));

        let entries_fn = create_function(JsFunction::Native(NativeFunction {
            name: "entries".to_string(),
            func: object_entries,
            arity: 1,
        }));
        obj.set_property(PropertyKey::from("entries"), JsValue::Object(entries_fn));

        let assign_fn = create_function(JsFunction::Native(NativeFunction {
            name: "assign".to_string(),
            func: object_assign,
            arity: 2,
        }));
        obj.set_property(PropertyKey::from("assign"), JsValue::Object(assign_fn));

        let fromentries_fn = create_function(JsFunction::Native(NativeFunction {
            name: "fromEntries".to_string(),
            func: object_from_entries,
            arity: 1,
        }));
        obj.set_property(PropertyKey::from("fromEntries"), JsValue::Object(fromentries_fn));

        let hasown_fn = create_function(JsFunction::Native(NativeFunction {
            name: "hasOwn".to_string(),
            func: object_has_own,
            arity: 2,
        }));
        obj.set_property(PropertyKey::from("hasOwn"), JsValue::Object(hasown_fn));

        let create_fn = create_function(JsFunction::Native(NativeFunction {
            name: "create".to_string(),
            func: object_create,
            arity: 1,
        }));
        obj.set_property(PropertyKey::from("create"), JsValue::Object(create_fn));

        let freeze_fn = create_function(JsFunction::Native(NativeFunction {
            name: "freeze".to_string(),
            func: object_freeze,
            arity: 1,
        }));
        obj.set_property(PropertyKey::from("freeze"), JsValue::Object(freeze_fn));

        let isfrozen_fn = create_function(JsFunction::Native(NativeFunction {
            name: "isFrozen".to_string(),
            func: object_is_frozen,
            arity: 1,
        }));
        obj.set_property(PropertyKey::from("isFrozen"), JsValue::Object(isfrozen_fn));

        let seal_fn = create_function(JsFunction::Native(NativeFunction {
            name: "seal".to_string(),
            func: object_seal,
            arity: 1,
        }));
        obj.set_property(PropertyKey::from("seal"), JsValue::Object(seal_fn));

        let issealed_fn = create_function(JsFunction::Native(NativeFunction {
            name: "isSealed".to_string(),
            func: object_is_sealed,
            arity: 1,
        }));
        obj.set_property(PropertyKey::from("isSealed"), JsValue::Object(issealed_fn));
    }
    constructor
}

pub fn object_constructor(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    match value {
        JsValue::Null | JsValue::Undefined => Ok(JsValue::Object(create_object())),
        JsValue::Object(_) => Ok(value),
        _ => Ok(JsValue::Object(create_object())),
    }
}

pub fn object_keys(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_ref) = obj else {
        return Err(JsError::type_error("Object.keys requires an object"));
    };

    let keys: Vec<JsValue> = obj_ref
        .borrow()
        .properties
        .iter()
        .filter(|(_, prop)| prop.enumerable)
        .map(|(key, _)| JsValue::String(JsString::from(key.to_string())))
        .collect();

    Ok(JsValue::Object(create_array(keys)))
}

pub fn object_values(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_ref) = obj else {
        return Err(JsError::type_error("Object.values requires an object"));
    };

    let values: Vec<JsValue> = obj_ref
        .borrow()
        .properties
        .iter()
        .filter(|(_, prop)| prop.enumerable)
        .map(|(_, prop)| prop.value.clone())
        .collect();

    Ok(JsValue::Object(create_array(values)))
}

pub fn object_entries(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_ref) = obj else {
        return Err(JsError::type_error("Object.entries requires an object"));
    };

    let entries: Vec<JsValue> = obj_ref
        .borrow()
        .properties
        .iter()
        .filter(|(_, prop)| prop.enumerable)
        .map(|(key, prop)| {
            JsValue::Object(create_array(vec![
                JsValue::String(JsString::from(key.to_string())),
                prop.value.clone(),
            ]))
        })
        .collect();

    Ok(JsValue::Object(create_array(entries)))
}

pub fn object_assign(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);
    let JsValue::Object(target_ref) = target.clone() else {
        return Err(JsError::type_error("Object.assign requires an object target"));
    };

    for source in args.iter().skip(1) {
        if let JsValue::Object(src_ref) = source {
            let src = src_ref.borrow();
            for (key, prop) in src.properties.iter() {
                if prop.enumerable {
                    target_ref.borrow_mut().set_property(key.clone(), prop.value.clone());
                }
            }
        }
    }

    Ok(target)
}

pub fn object_from_entries(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let iterable = args.first().cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(arr) = iterable else {
        return Err(JsError::type_error("Object.fromEntries requires an iterable"));
    };

    let result = create_object();

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Object.fromEntries requires an array-like")),
        }
    };

    for i in 0..length {
        let entry = arr.borrow().get_property(&PropertyKey::Index(i)).unwrap_or(JsValue::Undefined);
        if let JsValue::Object(entry_ref) = entry {
            let entry_borrow = entry_ref.borrow();
            if let ExoticObject::Array { .. } = entry_borrow.exotic {
                let key = entry_borrow.get_property(&PropertyKey::Index(0)).unwrap_or(JsValue::Undefined);
                let value = entry_borrow.get_property(&PropertyKey::Index(1)).unwrap_or(JsValue::Undefined);
                let key_str = key.to_js_string().to_string();
                drop(entry_borrow);
                result.borrow_mut().set_property(PropertyKey::from(key_str), value);
            }
        }
    }

    Ok(JsValue::Object(result))
}

pub fn object_has_own(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let key = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_ref) = obj else {
        return Ok(JsValue::Boolean(false));
    };

    let key_str = key.to_js_string().to_string();
    let has = obj_ref.borrow().properties.contains_key(&PropertyKey::from(key_str));
    Ok(JsValue::Boolean(has))
}

pub fn object_create(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let proto = args.first().cloned().unwrap_or(JsValue::Undefined);

    let result = create_object();

    // Set prototype (or null)
    match proto {
        JsValue::Null => {
            // No prototype - object won't have hasOwnProperty etc.
            let mut obj = result.borrow_mut();
            obj.prototype = None;
            obj.null_prototype = true;
        }
        JsValue::Object(proto_ref) => {
            result.borrow_mut().prototype = Some(proto_ref);
        }
        _ => return Err(JsError::type_error("Object prototype may only be an Object or null")),
    }

    Ok(JsValue::Object(result))
}

pub fn object_freeze(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    if let JsValue::Object(obj_ref) = &obj {
        let mut obj_mut = obj_ref.borrow_mut();
        obj_mut.frozen = true;
        // Mark all properties as non-writable and non-configurable
        for (_, prop) in obj_mut.properties.iter_mut() {
            prop.writable = false;
            prop.configurable = false;
        }
    }

    Ok(obj)
}

pub fn object_is_frozen(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    let is_frozen = match obj {
        JsValue::Object(obj_ref) => obj_ref.borrow().frozen,
        _ => true, // Non-objects are considered frozen
    };

    Ok(JsValue::Boolean(is_frozen))
}

pub fn object_seal(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    if let JsValue::Object(obj_ref) = &obj {
        let mut obj_mut = obj_ref.borrow_mut();
        obj_mut.sealed = true;
        // Mark all properties as non-configurable (but still writable)
        for (_, prop) in obj_mut.properties.iter_mut() {
            prop.configurable = false;
        }
    }

    Ok(obj)
}

pub fn object_is_sealed(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    let is_sealed = match obj {
        JsValue::Object(obj_ref) => obj_ref.borrow().sealed,
        _ => true, // Non-objects are considered sealed
    };

    Ok(JsValue::Boolean(is_sealed))
}

// Object.prototype methods

pub fn object_has_own_property(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(obj) = this else {
        return Ok(JsValue::Boolean(false));
    };

    let prop_name = args.first().map(|v| v.to_js_string().to_string()).unwrap_or_default();
    let key = PropertyKey::from(prop_name.as_str());

    let has_prop = obj.borrow().properties.contains_key(&key);
    Ok(JsValue::Boolean(has_prop))
}

pub fn object_to_string(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    match this {
        JsValue::Object(obj) => {
            let obj_ref = obj.borrow();
            match &obj_ref.exotic {
                ExoticObject::Array { length } => {
                    // Array.prototype.toString returns comma-separated values
                    let parts: Vec<String> = (0..*length)
                        .map(|i| {
                            obj_ref
                                .get_property(&PropertyKey::Index(i))
                                .map(|v| v.to_js_string().to_string())
                                .unwrap_or_default()
                        })
                        .collect();
                    Ok(JsValue::String(JsString::from(parts.join(","))))
                }
                ExoticObject::Function(_) => {
                    Ok(JsValue::String(JsString::from("[object Function]")))
                }
                ExoticObject::Ordinary => {
                    Ok(JsValue::String(JsString::from("[object Object]")))
                }
                ExoticObject::Map { .. } => {
                    Ok(JsValue::String(JsString::from("[object Map]")))
                }
                ExoticObject::Set { .. } => {
                    Ok(JsValue::String(JsString::from("[object Set]")))
                }
                ExoticObject::Date { .. } => {
                    Ok(JsValue::String(JsString::from("[object Date]")))
                }
                ExoticObject::RegExp { .. } => {
                    Ok(JsValue::String(JsString::from("[object RegExp]")))
                }
            }
        }
        _ => Ok(JsValue::String(JsString::from("[object Object]"))),
    }
}

pub fn object_value_of(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    Ok(this)
}
