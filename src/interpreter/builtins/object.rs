//! Object built-in methods

use crate::error::JsError;
use crate::gc::Space;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object_with_capacity, register_method, ExoticObject, JsFunction,
    JsObject, JsObjectRef, JsString, JsValue, NativeFunction, Property, PropertyKey,
};

/// Create Object.prototype with hasOwnProperty, toString, valueOf
pub fn create_object_prototype(space: &mut Space<JsObject>) -> JsObjectRef {
    let proto = create_object_with_capacity(space, 4);

    register_method(space, &proto, "hasOwnProperty", object_has_own_property, 1);
    register_method(space, &proto, "toString", object_to_string, 0);
    register_method(space, &proto, "valueOf", object_value_of, 0);

    debug_assert_eq!(
        proto.borrow().properties.len(),
        3,
        "Object.prototype capacity mismatch: expected 3, got {}",
        proto.borrow().properties.len()
    );

    proto
}

/// Create Object constructor with static methods (keys, values, entries, assign, etc.)
pub fn create_object_constructor(space: &mut Space<JsObject>) -> JsObjectRef {
    let constructor = create_function(
        space,
        JsFunction::Native(NativeFunction {
            name: "Object".to_string(),
            func: object_constructor,
            arity: 1,
        }),
    );

    // Property enumeration
    register_method(space, &constructor, "keys", object_keys, 1);
    register_method(space, &constructor, "values", object_values, 1);
    register_method(space, &constructor, "entries", object_entries, 1);

    // Object manipulation
    register_method(space, &constructor, "assign", object_assign, 2);
    register_method(space, &constructor, "fromEntries", object_from_entries, 1);
    register_method(space, &constructor, "create", object_create, 1);

    // Property checking
    register_method(space, &constructor, "hasOwn", object_has_own, 2);

    // Freezing/sealing
    register_method(space, &constructor, "freeze", object_freeze, 1);
    register_method(space, &constructor, "isFrozen", object_is_frozen, 1);
    register_method(space, &constructor, "seal", object_seal, 1);
    register_method(space, &constructor, "isSealed", object_is_sealed, 1);

    // Property descriptors
    register_method(
        space,
        &constructor,
        "getOwnPropertyDescriptor",
        object_get_own_property_descriptor,
        2,
    );
    register_method(
        space,
        &constructor,
        "getOwnPropertyNames",
        object_get_own_property_names,
        1,
    );
    register_method(
        space,
        &constructor,
        "getOwnPropertySymbols",
        object_get_own_property_symbols,
        1,
    );
    register_method(
        space,
        &constructor,
        "defineProperty",
        object_define_property,
        3,
    );
    register_method(
        space,
        &constructor,
        "defineProperties",
        object_define_properties,
        2,
    );

    // Prototype manipulation
    register_method(
        space,
        &constructor,
        "getPrototypeOf",
        object_get_prototype_of,
        1,
    );
    register_method(
        space,
        &constructor,
        "setPrototypeOf",
        object_set_prototype_of,
        2,
    );

    constructor
}

pub fn object_constructor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    match value {
        JsValue::Null | JsValue::Undefined => Ok(JsValue::Object(interp.create_object())),
        JsValue::Object(_) => Ok(value),
        _ => Ok(JsValue::Object(interp.create_object())),
    }
}

pub fn object_keys(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    Ok(JsValue::Object(interp.create_array(keys)))
}

pub fn object_values(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    Ok(JsValue::Object(interp.create_array(values)))
}

pub fn object_entries(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_ref) = obj else {
        return Err(JsError::type_error("Object.entries requires an object"));
    };

    // Collect key-value pairs first to release the borrow
    let pairs: Vec<(String, JsValue)> = obj_ref
        .borrow()
        .properties
        .iter()
        .filter(|(_, prop)| prop.enumerable)
        .map(|(key, prop)| (key.to_string(), prop.value.clone()))
        .collect();

    // Create entry arrays with proper prototype
    let entries: Vec<JsValue> = pairs
        .into_iter()
        .map(|(key, value)| {
            JsValue::Object(interp.create_array(vec![JsValue::String(JsString::from(key)), value]))
        })
        .collect();

    Ok(JsValue::Object(interp.create_array(entries)))
}

pub fn object_assign(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);
    let JsValue::Object(target_ref) = target.clone() else {
        return Err(JsError::type_error(
            "Object.assign requires an object target",
        ));
    };

    for source in args.iter().skip(1) {
        if let JsValue::Object(src_ref) = source {
            let src = src_ref.borrow();
            for (key, prop) in src.properties.iter() {
                if prop.enumerable {
                    target_ref
                        .borrow_mut()
                        .set_property(key.clone(), prop.value.clone());
                }
            }
        }
    }

    Ok(target)
}

pub fn object_from_entries(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let iterable = args.first().cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(arr) = iterable else {
        return Err(JsError::type_error(
            "Object.fromEntries requires an iterable",
        ));
    };

    let result = interp.create_object();

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => {
                return Err(JsError::type_error(
                    "Object.fromEntries requires an array-like",
                ))
            }
        }
    };

    for i in 0..length {
        let entry = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);
        if let JsValue::Object(entry_ref) = entry {
            let entry_borrow = entry_ref.borrow();
            if let ExoticObject::Array { .. } = entry_borrow.exotic {
                let key = entry_borrow
                    .get_property(&PropertyKey::Index(0))
                    .unwrap_or(JsValue::Undefined);
                let value = entry_borrow
                    .get_property(&PropertyKey::Index(1))
                    .unwrap_or(JsValue::Undefined);
                let key_str = key.to_js_string().to_string();
                drop(entry_borrow);
                result
                    .borrow_mut()
                    .set_property(PropertyKey::from(key_str), value);
            }
        }
    }

    Ok(JsValue::Object(result))
}

pub fn object_has_own(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let key = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_ref) = obj else {
        return Ok(JsValue::Boolean(false));
    };

    let key_str = key.to_js_string().to_string();
    let has = obj_ref
        .borrow()
        .properties
        .contains_key(&PropertyKey::from(key_str));
    Ok(JsValue::Boolean(has))
}

pub fn object_create(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let proto = args.first().cloned().unwrap_or(JsValue::Undefined);

    let result = interp.create_object();

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
        _ => {
            return Err(JsError::type_error(
                "Object prototype may only be an Object or null",
            ))
        }
    }

    Ok(JsValue::Object(result))
}

pub fn object_freeze(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

pub fn object_is_frozen(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    let is_frozen = match obj {
        JsValue::Object(obj_ref) => obj_ref.borrow().frozen,
        _ => true, // Non-objects are considered frozen
    };

    Ok(JsValue::Boolean(is_frozen))
}

pub fn object_seal(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

pub fn object_is_sealed(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    let is_sealed = match obj {
        JsValue::Object(obj_ref) => obj_ref.borrow().sealed,
        _ => true, // Non-objects are considered sealed
    };

    Ok(JsValue::Boolean(is_sealed))
}

// Object.prototype methods

pub fn object_has_own_property(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(obj) = this else {
        return Ok(JsValue::Boolean(false));
    };

    let prop_name = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();
    let key = PropertyKey::from(prop_name.as_str());

    let has_prop = obj.borrow().properties.contains_key(&key);
    Ok(JsValue::Boolean(has_prop))
}

pub fn object_to_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
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
                ExoticObject::Ordinary => Ok(JsValue::String(JsString::from("[object Object]"))),
                ExoticObject::Map { .. } => Ok(JsValue::String(JsString::from("[object Map]"))),
                ExoticObject::Set { .. } => Ok(JsValue::String(JsString::from("[object Set]"))),
                ExoticObject::Date { .. } => Ok(JsValue::String(JsString::from("[object Date]"))),
                ExoticObject::RegExp { .. } => {
                    Ok(JsValue::String(JsString::from("[object RegExp]")))
                }
                ExoticObject::Generator(_) => {
                    Ok(JsValue::String(JsString::from("[object Generator]")))
                }
                ExoticObject::Promise(_) => Ok(JsValue::String(JsString::from("[object Promise]"))),
            }
        }
        _ => Ok(JsValue::String(JsString::from("[object Object]"))),
    }
}

pub fn object_value_of(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    Ok(this)
}

/// Object.getOwnPropertyDescriptor(obj, prop)
pub fn object_get_own_property_descriptor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let prop = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_ref) = obj else {
        return Err(JsError::type_error(
            "Object.getOwnPropertyDescriptor requires an object",
        ));
    };

    let key = PropertyKey::from_value(&prop);
    let obj_borrowed = obj_ref.borrow();

    if let Some(property) = obj_borrowed.get_own_property(&key) {
        // Create a descriptor object
        let desc = interp.create_object();
        {
            let mut desc_ref = desc.borrow_mut();

            if property.is_accessor() {
                // Accessor descriptor
                if let Some(ref getter) = property.getter {
                    desc_ref
                        .set_property(PropertyKey::from("get"), JsValue::Object(getter.clone()));
                } else {
                    desc_ref.set_property(PropertyKey::from("get"), JsValue::Undefined);
                }
                if let Some(ref setter) = property.setter {
                    desc_ref
                        .set_property(PropertyKey::from("set"), JsValue::Object(setter.clone()));
                } else {
                    desc_ref.set_property(PropertyKey::from("set"), JsValue::Undefined);
                }
            } else {
                // Data descriptor
                desc_ref.set_property(PropertyKey::from("value"), property.value.clone());
                desc_ref.set_property(
                    PropertyKey::from("writable"),
                    JsValue::Boolean(property.writable),
                );
            }

            desc_ref.set_property(
                PropertyKey::from("enumerable"),
                JsValue::Boolean(property.enumerable),
            );
            desc_ref.set_property(
                PropertyKey::from("configurable"),
                JsValue::Boolean(property.configurable),
            );
        }
        Ok(JsValue::Object(desc))
    } else {
        Ok(JsValue::Undefined)
    }
}

/// Object.getOwnPropertyNames(obj)
pub fn object_get_own_property_names(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_ref) = obj else {
        return Err(JsError::type_error(
            "Object.getOwnPropertyNames requires an object",
        ));
    };

    // Filter out symbol keys - getOwnPropertyNames only returns string keys
    let names: Vec<JsValue> = obj_ref
        .borrow()
        .properties
        .keys()
        .filter(|key| !key.is_symbol())
        .map(|key| JsValue::String(JsString::from(key.to_string())))
        .collect();

    Ok(JsValue::Object(interp.create_array(names)))
}

/// Object.getOwnPropertySymbols(obj)
pub fn object_get_own_property_symbols(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_ref) = obj else {
        return Err(JsError::type_error(
            "Object.getOwnPropertySymbols requires an object",
        ));
    };

    // Return only symbol keys
    let symbols: Vec<JsValue> = obj_ref
        .borrow()
        .properties
        .keys()
        .filter_map(|key| {
            if let PropertyKey::Symbol(s) = key {
                Some(JsValue::Symbol(s.clone()))
            } else {
                None
            }
        })
        .collect();

    Ok(JsValue::Object(interp.create_array(symbols)))
}

/// Object.defineProperty(obj, prop, descriptor)
pub fn object_define_property(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let prop = args.get(1).cloned().unwrap_or(JsValue::Undefined);
    let descriptor = args.get(2).cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_ref) = obj.clone() else {
        return Err(JsError::type_error(
            "Object.defineProperty requires an object",
        ));
    };

    let JsValue::Object(desc_ref) = descriptor else {
        return Err(JsError::type_error("Property descriptor must be an object"));
    };

    let key = PropertyKey::from_value(&prop);

    // Get descriptor properties
    let desc_borrowed = desc_ref.borrow();
    let value = desc_borrowed
        .get_property(&PropertyKey::from("value"))
        .unwrap_or(JsValue::Undefined);
    let writable = desc_borrowed
        .get_property(&PropertyKey::from("writable"))
        .map(|v| v.to_boolean())
        .unwrap_or(false);
    let enumerable = desc_borrowed
        .get_property(&PropertyKey::from("enumerable"))
        .map(|v| v.to_boolean())
        .unwrap_or(false);
    let configurable = desc_borrowed
        .get_property(&PropertyKey::from("configurable"))
        .map(|v| v.to_boolean())
        .unwrap_or(false);

    // Check for getter/setter
    let getter = desc_borrowed.get_property(&PropertyKey::from("get"));
    let setter = desc_borrowed.get_property(&PropertyKey::from("set"));
    drop(desc_borrowed);

    let is_accessor = getter.is_some() || setter.is_some();

    if is_accessor {
        // Accessor descriptor
        let getter_ref = match getter {
            Some(JsValue::Object(g)) => Some(g),
            _ => None,
        };
        let setter_ref = match setter {
            Some(JsValue::Object(s)) => Some(s),
            _ => None,
        };
        let mut prop = Property::accessor(getter_ref, setter_ref);
        prop.enumerable = enumerable;
        prop.configurable = configurable;
        obj_ref.borrow_mut().define_property(key, prop);
    } else {
        // Data descriptor
        let prop = Property::with_attributes(value, writable, enumerable, configurable);
        obj_ref.borrow_mut().define_property(key, prop);
    }

    Ok(obj)
}

/// Object.defineProperties(obj, props)
/// Define multiple properties at once
pub fn object_define_properties(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let props = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_ref) = obj.clone() else {
        return Err(JsError::type_error(
            "Object.defineProperties requires an object",
        ));
    };

    let JsValue::Object(props_ref) = props else {
        return Err(JsError::type_error(
            "Property descriptors must be an object",
        ));
    };

    // Iterate over all properties in the descriptor object
    let prop_keys: Vec<PropertyKey> = {
        let props_borrowed = props_ref.borrow();
        props_borrowed.properties.keys().cloned().collect()
    };

    for key in prop_keys {
        let descriptor = {
            let props_borrowed = props_ref.borrow();
            props_borrowed
                .get_property(&key)
                .unwrap_or(JsValue::Undefined)
        };

        let JsValue::Object(desc_ref) = descriptor else {
            continue; // Skip non-object descriptors
        };

        // Get descriptor properties
        let desc_borrowed = desc_ref.borrow();
        let value = desc_borrowed
            .get_property(&PropertyKey::from("value"))
            .unwrap_or(JsValue::Undefined);
        let writable = desc_borrowed
            .get_property(&PropertyKey::from("writable"))
            .map(|v| v.to_boolean())
            .unwrap_or(false);
        let enumerable = desc_borrowed
            .get_property(&PropertyKey::from("enumerable"))
            .map(|v| v.to_boolean())
            .unwrap_or(false);
        let configurable = desc_borrowed
            .get_property(&PropertyKey::from("configurable"))
            .map(|v| v.to_boolean())
            .unwrap_or(false);

        // Check for getter/setter
        let getter = desc_borrowed.get_property(&PropertyKey::from("get"));
        let setter = desc_borrowed.get_property(&PropertyKey::from("set"));
        drop(desc_borrowed);

        let is_accessor = getter.is_some() || setter.is_some();

        if is_accessor {
            // Accessor descriptor
            let getter_ref = match getter {
                Some(JsValue::Object(g)) => Some(g),
                _ => None,
            };
            let setter_ref = match setter {
                Some(JsValue::Object(s)) => Some(s),
                _ => None,
            };
            let mut prop = Property::accessor(getter_ref, setter_ref);
            prop.enumerable = enumerable;
            prop.configurable = configurable;
            obj_ref.borrow_mut().define_property(key, prop);
        } else {
            // Data descriptor
            let prop = Property::with_attributes(value, writable, enumerable, configurable);
            obj_ref.borrow_mut().define_property(key, prop);
        }
    }

    Ok(obj)
}

/// Object.getPrototypeOf(obj)
pub fn object_get_prototype_of(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_ref) = obj else {
        return Err(JsError::type_error(
            "Object.getPrototypeOf requires an object",
        ));
    };

    let obj_borrowed = obj_ref.borrow();
    match &obj_borrowed.prototype {
        Some(proto) => Ok(JsValue::Object(proto.clone())),
        None => Ok(JsValue::Null),
    }
}

/// Object.setPrototypeOf(obj, proto)
pub fn object_set_prototype_of(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let proto = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_ref) = obj.clone() else {
        return Err(JsError::type_error(
            "Object.setPrototypeOf requires an object",
        ));
    };

    let new_proto = match proto {
        JsValue::Object(p) => Some(p),
        JsValue::Null => None,
        _ => {
            return Err(JsError::type_error(
                "Object prototype may only be an Object or null",
            ))
        }
    };

    obj_ref.borrow_mut().prototype = new_proto;
    Ok(obj)
}
