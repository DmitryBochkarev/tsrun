//! Object built-in methods

use crate::error::JsError;
use crate::interpreter::builtins::proxy::{
    is_proxy, proxy_define_property, proxy_get_own_property_descriptor, proxy_get_prototype_of,
    proxy_is_extensible, proxy_own_keys, proxy_prevent_extensions, proxy_set_prototype_of,
};
use crate::interpreter::Interpreter;
use crate::value::{ExoticObject, Guarded, JsObjectRef, JsString, JsValue, Property, PropertyKey};

/// Initialize Object.prototype with hasOwnProperty, toString, valueOf, isPrototypeOf methods.
/// The prototype object must already exist in `interp.object_prototype`.
pub fn init_object_prototype(interp: &mut Interpreter) {
    let proto = interp.object_prototype.clone();

    interp.register_method(&proto, "hasOwnProperty", object_has_own_property, 1);
    interp.register_method(&proto, "isPrototypeOf", object_is_prototype_of, 1);
    interp.register_method(&proto, "toString", object_to_string, 0);
    interp.register_method(&proto, "toLocaleString", object_to_locale_string, 0);
    interp.register_method(&proto, "valueOf", object_value_of, 0);
}

/// Create Object constructor with static methods (keys, values, entries, assign, etc.)
pub fn create_object_constructor(interp: &mut Interpreter) -> JsObjectRef {
    let constructor = interp.create_native_function("Object", object_constructor, 1);

    // Property enumeration
    interp.register_method(&constructor, "keys", object_keys, 1);
    interp.register_method(&constructor, "values", object_values, 1);
    interp.register_method(&constructor, "entries", object_entries, 1);

    // Object manipulation
    interp.register_method(&constructor, "assign", object_assign, 2);
    interp.register_method(&constructor, "fromEntries", object_from_entries, 1);
    interp.register_method(&constructor, "create", object_create, 1);

    // Property checking
    interp.register_method(&constructor, "hasOwn", object_has_own, 2);

    // Freezing/sealing/extensibility
    interp.register_method(&constructor, "freeze", object_freeze, 1);
    interp.register_method(&constructor, "isFrozen", object_is_frozen, 1);
    interp.register_method(&constructor, "seal", object_seal, 1);
    interp.register_method(&constructor, "isSealed", object_is_sealed, 1);
    interp.register_method(
        &constructor,
        "preventExtensions",
        object_prevent_extensions,
        1,
    );
    interp.register_method(&constructor, "isExtensible", object_is_extensible, 1);

    // Comparison
    interp.register_method(&constructor, "is", object_is, 2);

    // Property descriptors
    interp.register_method(
        &constructor,
        "getOwnPropertyDescriptor",
        object_get_own_property_descriptor,
        2,
    );
    interp.register_method(
        &constructor,
        "getOwnPropertyNames",
        object_get_own_property_names,
        1,
    );
    interp.register_method(
        &constructor,
        "getOwnPropertySymbols",
        object_get_own_property_symbols,
        1,
    );
    interp.register_method(
        &constructor,
        "getOwnPropertyDescriptors",
        object_get_own_property_descriptors,
        1,
    );
    interp.register_method(&constructor, "defineProperty", object_define_property, 3);
    interp.register_method(
        &constructor,
        "defineProperties",
        object_define_properties,
        2,
    );

    // Prototype manipulation
    interp.register_method(&constructor, "getPrototypeOf", object_get_prototype_of, 1);
    interp.register_method(&constructor, "setPrototypeOf", object_set_prototype_of, 2);

    // Set constructor.prototype = Object.prototype
    let proto_key = PropertyKey::String(interp.intern("prototype"));
    constructor
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.object_prototype.clone()));

    // Set Object.prototype.constructor = Object
    let constructor_key = PropertyKey::String(interp.intern("constructor"));
    interp
        .object_prototype
        .borrow_mut()
        .set_property(constructor_key, JsValue::Object(constructor.clone()));

    constructor
}

/// Object constructor - wraps primitives in their respective wrapper objects
pub fn object_constructor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    match value {
        JsValue::Null | JsValue::Undefined => {
            // Return a new plain object
            let guard = interp.heap.create_guard();
            let obj = interp.create_object(&guard);
            Ok(Guarded::with_guard(JsValue::Object(obj), guard))
        }
        JsValue::Object(_) => {
            // Return the object as-is
            Ok(Guarded::unguarded(value))
        }
        JsValue::Boolean(b) => {
            // Create Boolean wrapper object
            let guard = interp.heap.create_guard();
            let obj = interp.create_object(&guard);
            obj.borrow_mut().prototype = Some(interp.boolean_prototype.clone());
            obj.borrow_mut().exotic = ExoticObject::Boolean(b);
            Ok(Guarded::with_guard(JsValue::Object(obj), guard))
        }
        JsValue::Number(n) => {
            // Create Number wrapper object
            let guard = interp.heap.create_guard();
            let obj = interp.create_object(&guard);
            obj.borrow_mut().prototype = Some(interp.number_prototype.clone());
            obj.borrow_mut().exotic = ExoticObject::Number(n);
            Ok(Guarded::with_guard(JsValue::Object(obj), guard))
        }
        JsValue::String(ref s) => {
            // Create String wrapper object
            let guard = interp.heap.create_guard();
            let obj = interp.create_object(&guard);
            obj.borrow_mut().prototype = Some(interp.string_prototype.clone());
            obj.borrow_mut().exotic = ExoticObject::StringObj(s.clone());
            // Also set length property for string wrappers
            let len = s.len();
            let length_key = PropertyKey::String(interp.intern("length"));
            obj.borrow_mut()
                .set_property(length_key, JsValue::Number(len as f64));
            Ok(Guarded::with_guard(JsValue::Object(obj), guard))
        }
        JsValue::Symbol(_) => {
            // Symbols cannot be wrapped with Object() - this should throw TypeError in strict mode
            // but for now we return an ordinary object
            let guard = interp.heap.create_guard();
            let obj = interp.create_object(&guard);
            Ok(Guarded::with_guard(JsValue::Object(obj), guard))
        }
    }
}

pub fn object_keys(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let arg = args.first().cloned().unwrap_or(JsValue::Undefined);

    // ES2015+: Convert to object (primitives get boxed, null/undefined throw)
    let obj_ref = interp.to_object(arg)?;

    // Use proxy trap if it's a proxy - ownKeys trap returns all keys
    if is_proxy(&obj_ref) {
        // Call ownKeys trap and filter for enumerable string keys
        let Guarded {
            value: keys_result, ..
        } = proxy_own_keys(interp, obj_ref)?;
        // Filter for enumerable string keys only (not symbols)
        if let JsValue::Object(keys_arr) = keys_result {
            let keys_ref = keys_arr.borrow();
            if let Some(elements) = keys_ref.array_elements() {
                let string_keys: Vec<JsValue> = elements
                    .iter()
                    .filter(|k| matches!(k, JsValue::String(_)))
                    .cloned()
                    .collect();
                drop(keys_ref);
                let guard = interp.heap.create_guard();
                let arr = interp.create_array_from(&guard, string_keys);
                return Ok(Guarded::with_guard(JsValue::Object(arr), guard));
            }
        }
        // Fallback to empty array
        let guard = interp.heap.create_guard();
        let arr = interp.create_array_from(&guard, vec![]);
        return Ok(Guarded::with_guard(JsValue::Object(arr), guard));
    }

    let keys: Vec<JsValue> = {
        let obj = obj_ref.borrow();

        // For enums, get keys from EnumData
        if let ExoticObject::Enum(ref data) = obj.exotic {
            data.keys()
                .into_iter()
                .filter(|k| !k.is_symbol())
                .map(|k| JsValue::String(JsString::from(k.to_string())))
                .collect()
        } else {
            // Standard object - get from properties
            // Only include enumerable string keys, not symbols
            obj.properties
                .iter()
                .filter(|(key, prop)| prop.enumerable() && !key.is_symbol())
                .map(|(key, _)| JsValue::String(JsString::from(key.to_string())))
                .collect()
        }
    };

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, keys);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn object_values(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_ref) = obj else {
        return Err(JsError::type_error("Object.values requires an object"));
    };

    let values: Vec<JsValue> = {
        let obj = obj_ref.borrow();

        // For enums, get values from EnumData
        if let ExoticObject::Enum(ref data) = obj.exotic {
            data.values()
        } else {
            // Standard object - get from properties
            // Only include enumerable string keys, not symbols
            obj.properties
                .iter()
                .filter(|(key, prop)| prop.enumerable() && !key.is_symbol())
                .map(|(_, prop)| prop.value.clone())
                .collect()
        }
    };

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, values);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn object_entries(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_ref) = obj else {
        return Err(JsError::type_error("Object.entries requires an object"));
    };

    // Collect key-value pairs first to release the borrow
    let pairs: Vec<(String, JsValue)> = {
        let obj = obj_ref.borrow();

        // For enums, get entries from EnumData
        if let ExoticObject::Enum(ref data) = obj.exotic {
            data.entries()
        } else {
            // Standard object - get from properties
            // Only include enumerable string keys, not symbols
            obj.properties
                .iter()
                .filter(|(key, prop)| prop.enumerable() && !key.is_symbol())
                .map(|(key, prop)| (key.to_string(), prop.value.clone()))
                .collect()
        }
    };

    // Use single guard for all entry arrays
    let guard = interp.heap.create_guard();
    let mut entries: Vec<JsValue> = Vec::with_capacity(pairs.len());
    for (key, value) in pairs {
        let arr =
            interp.create_array_from(&guard, vec![JsValue::String(JsString::from(key)), value]);
        entries.push(JsValue::Object(arr));
    }

    let result = interp.create_array_from(&guard, entries);
    Ok(Guarded::with_guard(JsValue::Object(result), guard))
}

pub fn object_assign(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
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
                if prop.enumerable() {
                    target_ref
                        .borrow_mut()
                        .set_property(key.clone(), prop.value.clone());
                }
            }
        }
    }

    // Target was passed in by caller, so it's already owned - no guard needed
    Ok(Guarded::unguarded(target))
}

pub fn object_from_entries(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let iterable = args.first().cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(arr) = iterable else {
        return Err(JsError::type_error(
            "Object.fromEntries requires an iterable",
        ));
    };

    // Guard the input array to prevent GC from collecting it during iteration
    let _arr_guard = interp.guard_value(&JsValue::Object(arr.clone()));

    // Create result object with guard - key interning may trigger GC
    let result_guard = interp.heap.create_guard();
    let result = interp.create_object(&result_guard);

    let length = arr
        .borrow()
        .array_length()
        .ok_or_else(|| JsError::type_error("Object.fromEntries requires an array-like"))?;

    for i in 0..length {
        let entry = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);
        if let JsValue::Object(entry_ref) = entry {
            let entry_borrow = entry_ref.borrow();
            if entry_borrow.is_array() {
                let key = entry_borrow
                    .get_property(&PropertyKey::Index(0))
                    .unwrap_or(JsValue::Undefined);
                let value = entry_borrow
                    .get_property(&PropertyKey::Index(1))
                    .unwrap_or(JsValue::Undefined);
                let key_str = key.to_js_string().to_string();
                drop(entry_borrow);
                let interned_key = PropertyKey::String(interp.intern(&key_str));
                result.borrow_mut().set_property(interned_key, value);
            }
        }
    }

    Ok(Guarded::with_guard(JsValue::Object(result), result_guard))
}

pub fn object_has_own(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let key = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_ref) = obj else {
        return Ok(Guarded::unguarded(JsValue::Boolean(false)));
    };

    let key_str = key.to_js_string().to_string();
    let interned_key = PropertyKey::String(interp.intern(&key_str));

    let borrowed = obj_ref.borrow();
    let has = if let ExoticObject::Enum(ref data) = borrowed.exotic {
        // For enums, check EnumData
        data.has_property(&interned_key)
    } else {
        // Standard object - check properties
        borrowed.properties.contains_key(&interned_key)
    };
    Ok(Guarded::unguarded(JsValue::Boolean(has)))
}

pub fn object_create(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let proto = args.first().cloned().unwrap_or(JsValue::Undefined);
    let properties = args.get(1).cloned();

    let result_guard = interp.heap.create_guard();
    let result = interp.create_object(&result_guard);

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
            // Establish GC ownership so prototype isn't collected
        }
        _ => {
            return Err(JsError::type_error(
                "Object prototype may only be an Object or null",
            ))
        }
    }

    // If properties argument is provided and not undefined, define properties
    if let Some(props) = properties {
        if !matches!(props, JsValue::Undefined) {
            let JsValue::Object(props_ref) = props else {
                return Err(JsError::type_error(
                    "Property descriptors must be an object",
                ));
            };

            // Pre-intern descriptor property keys
            let value_key = PropertyKey::String(interp.intern("value"));
            let writable_key = PropertyKey::String(interp.intern("writable"));
            let enumerable_key = PropertyKey::String(interp.intern("enumerable"));
            let configurable_key = PropertyKey::String(interp.intern("configurable"));
            let get_key = PropertyKey::String(interp.intern("get"));
            let set_key = PropertyKey::String(interp.intern("set"));

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
                    .get_property(&value_key)
                    .unwrap_or(JsValue::Undefined);
                let writable = desc_borrowed
                    .get_property(&writable_key)
                    .map(|v| v.to_boolean())
                    .unwrap_or(false);
                let enumerable = desc_borrowed
                    .get_property(&enumerable_key)
                    .map(|v| v.to_boolean())
                    .unwrap_or(false);
                let configurable = desc_borrowed
                    .get_property(&configurable_key)
                    .map(|v| v.to_boolean())
                    .unwrap_or(false);

                // Check for getter/setter
                let getter = desc_borrowed.get_property(&get_key);
                let setter = desc_borrowed.get_property(&set_key);
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
                    prop.set_enumerable(enumerable);
                    prop.set_configurable(configurable);
                    result.borrow_mut().define_property(key, prop);
                } else {
                    // Data descriptor
                    let prop = Property::with_attributes(value, writable, enumerable, configurable);
                    result.borrow_mut().define_property(key, prop);
                }
            }
        }
    }

    Ok(Guarded::with_guard(JsValue::Object(result), result_guard))
}

pub fn object_freeze(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    if let JsValue::Object(obj_ref) = &obj {
        let mut obj_mut = obj_ref.borrow_mut();
        obj_mut.frozen = true;
        obj_mut.extensible = false; // Frozen objects are not extensible
                                    // Mark all properties as non-writable and non-configurable
        for (_, prop) in obj_mut.properties.iter_mut() {
            prop.set_writable(false);
            prop.set_configurable(false);
        }
    }

    // Return with guard to protect the object until caller stores it
    // This is necessary because the object might have been created inline
    // (e.g., Object.freeze({a: 1})) and the caller's arg guards will drop
    // before the returned value is used
    let guard = interp.guard_value(&obj);
    Ok(Guarded { value: obj, guard })
}

pub fn object_is_frozen(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    let is_frozen = match obj {
        JsValue::Object(obj_ref) => obj_ref.borrow().frozen,
        _ => true, // Non-objects are considered frozen
    };

    Ok(Guarded::unguarded(JsValue::Boolean(is_frozen)))
}

pub fn object_seal(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    if let JsValue::Object(obj_ref) = &obj {
        let mut obj_mut = obj_ref.borrow_mut();
        obj_mut.sealed = true;
        obj_mut.extensible = false; // Sealed objects are not extensible
                                    // Mark all properties as non-configurable (but still writable)
        for (_, prop) in obj_mut.properties.iter_mut() {
            prop.set_configurable(false);
        }
    }

    // Return with guard to protect the object until caller stores it
    let guard = interp.guard_value(&obj);
    Ok(Guarded { value: obj, guard })
}

pub fn object_is_sealed(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    let is_sealed = match obj {
        JsValue::Object(obj_ref) => obj_ref.borrow().sealed,
        _ => true, // Non-objects are considered sealed
    };

    Ok(Guarded::unguarded(JsValue::Boolean(is_sealed)))
}

// Object.prototype methods

pub fn object_has_own_property(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(obj) = this else {
        return Ok(Guarded::unguarded(JsValue::Boolean(false)));
    };

    let arg = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Handle symbol arguments directly
    let key = if let JsValue::Symbol(ref sym) = arg {
        PropertyKey::Symbol(sym.clone())
    } else {
        let prop_name = arg.to_js_string().to_string();
        PropertyKey::String(interp.intern(&prop_name))
    };

    let obj_ref = obj.borrow();
    let has_prop = if let ExoticObject::Enum(ref data) = obj_ref.exotic {
        // For enums, check EnumData
        data.has_property(&key)
    } else {
        // Standard object - check properties
        obj_ref.properties.contains_key(&key)
    };
    Ok(Guarded::unguarded(JsValue::Boolean(has_prop)))
}

/// Object.prototype.isPrototypeOf
/// Returns true if this object is in the prototype chain of the given value.
pub fn object_is_prototype_of(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // Get the this object
    let JsValue::Object(this_obj) = this else {
        // this is not an object - can't be in any prototype chain
        return Ok(Guarded::unguarded(JsValue::Boolean(false)));
    };

    // Get the argument - if not an object, return false
    let arg = args.first().cloned().unwrap_or(JsValue::Undefined);
    let JsValue::Object(mut check_obj) = arg else {
        // Primitives don't have prototype chains (in this context)
        return Ok(Guarded::unguarded(JsValue::Boolean(false)));
    };

    // Walk up the prototype chain of check_obj, looking for this_obj
    loop {
        let proto = check_obj.borrow().prototype.clone();
        match proto {
            Some(p) => {
                // Compare by pointer equality using associated function
                if crate::gc::Gc::ptr_eq(&this_obj, &p) {
                    return Ok(Guarded::unguarded(JsValue::Boolean(true)));
                }
                check_obj = p;
            }
            None => {
                // Reached the end of the chain
                return Ok(Guarded::unguarded(JsValue::Boolean(false)));
            }
        }
    }
}

/// Object.prototype.toString
/// Returns "[object Type]" based on the internal [[Class]] of the value.
/// Per ES spec, this checks for Symbol.toStringTag on objects first.
pub fn object_to_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let tag = match &this {
        JsValue::Undefined => "Undefined",
        JsValue::Null => "Null",
        JsValue::Boolean(_) => "Boolean",
        JsValue::Number(_) => "Number",
        JsValue::String(_) => "String",
        JsValue::Symbol(_) => "Symbol",
        JsValue::Object(obj) => {
            let obj_ref = obj.borrow();
            // TODO: Check for Symbol.toStringTag property first
            match &obj_ref.exotic {
                ExoticObject::Array { .. } => "Array",
                ExoticObject::Function(_) => "Function",
                ExoticObject::Ordinary => "Object",
                ExoticObject::Map { .. } => "Map",
                ExoticObject::Set { .. } => "Set",
                ExoticObject::Date { .. } => "Date",
                ExoticObject::RegExp { .. } => "RegExp",
                ExoticObject::Generator(_) | ExoticObject::BytecodeGenerator(_) => "Generator",
                ExoticObject::Promise(_) => "Promise",
                ExoticObject::Environment(_) => "Object",
                ExoticObject::Enum(_) => "Object",
                ExoticObject::Proxy(_) => "Object",
                ExoticObject::Boolean(_) => "Boolean",
                ExoticObject::Number(_) => "Number",
                ExoticObject::StringObj(_) => "String",
            }
        }
    };

    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        format!("[object {}]", tag),
    ))))
}

pub fn object_value_of(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    // Returns the object itself, which is already owned by caller
    Ok(Guarded::unguarded(this))
}

/// Object.prototype.toLocaleString()
/// Simply calls this.toString() per the spec.
/// For the base Object.prototype, this just calls toString.
pub fn object_to_locale_string(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // The default toLocaleString just calls toString
    // Other types (Number, Date, Array) may override with locale-specific behavior
    object_to_string(interp, this, args)
}

/// Object.getOwnPropertyDescriptor(obj, prop)
pub fn object_get_own_property_descriptor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let prop = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // ES2015+: Convert to object (primitives get boxed, null/undefined throw)
    let obj_ref = interp.to_object(obj)?;

    let key = PropertyKey::from_value(&prop);

    // Use proxy trap if it's a proxy
    if is_proxy(&obj_ref) {
        return proxy_get_own_property_descriptor(interp, obj_ref, &key);
    }

    let obj_borrowed = obj_ref.borrow();

    if let Some(property) = obj_borrowed.get_own_property(&key) {
        // Pre-intern all descriptor property keys
        let get_key = PropertyKey::String(interp.intern("get"));
        let set_key = PropertyKey::String(interp.intern("set"));
        let value_key = PropertyKey::String(interp.intern("value"));
        let writable_key = PropertyKey::String(interp.intern("writable"));
        let enumerable_key = PropertyKey::String(interp.intern("enumerable"));
        let configurable_key = PropertyKey::String(interp.intern("configurable"));

        // Create a descriptor object
        let desc_guard = interp.heap.create_guard();
        let desc = interp.create_object(&desc_guard);
        {
            let mut desc_ref = desc.borrow_mut();

            if property.is_accessor() {
                // Accessor descriptor
                if let Some(getter) = property.getter() {
                    desc_ref.set_property(get_key, JsValue::Object(getter.clone()));
                } else {
                    desc_ref.set_property(get_key, JsValue::Undefined);
                }
                if let Some(setter) = property.setter() {
                    desc_ref.set_property(set_key, JsValue::Object(setter.clone()));
                } else {
                    desc_ref.set_property(set_key, JsValue::Undefined);
                }
            } else {
                // Data descriptor
                desc_ref.set_property(value_key, property.value.clone());
                desc_ref.set_property(writable_key, JsValue::Boolean(property.writable()));
            }

            desc_ref.set_property(enumerable_key, JsValue::Boolean(property.enumerable()));
            desc_ref.set_property(configurable_key, JsValue::Boolean(property.configurable()));
        }
        Ok(Guarded::with_guard(JsValue::Object(desc), desc_guard))
    } else {
        Ok(Guarded::unguarded(JsValue::Undefined))
    }
}

/// Object.getOwnPropertyNames(obj)
pub fn object_get_own_property_names(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    // ES2015+: Convert to object (primitives get boxed, null/undefined throw)
    let obj_ref = interp.to_object(obj)?;

    // Filter out symbol keys - getOwnPropertyNames only returns string keys
    let names: Vec<JsValue> = obj_ref
        .borrow()
        .properties
        .keys()
        .filter(|key| !key.is_symbol())
        .map(|key| JsValue::String(JsString::from(key.to_string())))
        .collect();

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, names);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

/// Object.getOwnPropertySymbols(obj)
pub fn object_get_own_property_symbols(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    // ES2015+: Convert to object (primitives get boxed, null/undefined throw)
    let obj_ref = interp.to_object(obj)?;

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

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, symbols);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

/// Object.defineProperty(obj, prop, descriptor)
pub fn object_define_property(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let prop = args.get(1).cloned().unwrap_or(JsValue::Undefined);
    let descriptor = args.get(2).cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_ref) = obj.clone() else {
        return Err(JsError::type_error(
            "Object.defineProperty requires an object",
        ));
    };

    let JsValue::Object(ref desc_ref) = descriptor else {
        return Err(JsError::type_error("Property descriptor must be an object"));
    };

    let key = PropertyKey::from_value(&prop);

    // Use proxy trap if it's a proxy
    if is_proxy(&obj_ref) {
        proxy_define_property(interp, obj_ref, key, descriptor)?;
        return Ok(Guarded::unguarded(obj));
    }

    // Pre-intern descriptor property keys
    let value_key = PropertyKey::String(interp.intern("value"));
    let writable_key = PropertyKey::String(interp.intern("writable"));
    let enumerable_key = PropertyKey::String(interp.intern("enumerable"));
    let configurable_key = PropertyKey::String(interp.intern("configurable"));
    let get_key = PropertyKey::String(interp.intern("get"));
    let set_key = PropertyKey::String(interp.intern("set"));

    // Get descriptor properties
    let desc_borrowed = desc_ref.borrow();
    let value = desc_borrowed
        .get_property(&value_key)
        .unwrap_or(JsValue::Undefined);
    let writable = desc_borrowed
        .get_property(&writable_key)
        .map(|v| v.to_boolean())
        .unwrap_or(false);
    let enumerable = desc_borrowed
        .get_property(&enumerable_key)
        .map(|v| v.to_boolean())
        .unwrap_or(false);
    let configurable = desc_borrowed
        .get_property(&configurable_key)
        .map(|v| v.to_boolean())
        .unwrap_or(false);

    // Check for getter/setter
    let getter = desc_borrowed.get_property(&get_key);
    let setter = desc_borrowed.get_property(&set_key);
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
        prop.set_enumerable(enumerable);
        prop.set_configurable(configurable);
        obj_ref.borrow_mut().define_property(key, prop);
    } else {
        // Data descriptor
        let prop = Property::with_attributes(value, writable, enumerable, configurable);
        obj_ref.borrow_mut().define_property(key, prop);
    }

    // Object was passed in by caller, already owned - no guard needed
    Ok(Guarded::unguarded(obj))
}

/// Object.defineProperties(obj, props)
/// Define multiple properties at once
pub fn object_define_properties(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
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

    // Pre-intern descriptor property keys
    let value_key = PropertyKey::String(interp.intern("value"));
    let writable_key = PropertyKey::String(interp.intern("writable"));
    let enumerable_key = PropertyKey::String(interp.intern("enumerable"));
    let configurable_key = PropertyKey::String(interp.intern("configurable"));
    let get_key = PropertyKey::String(interp.intern("get"));
    let set_key = PropertyKey::String(interp.intern("set"));

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
            .get_property(&value_key)
            .unwrap_or(JsValue::Undefined);
        let writable = desc_borrowed
            .get_property(&writable_key)
            .map(|v| v.to_boolean())
            .unwrap_or(false);
        let enumerable = desc_borrowed
            .get_property(&enumerable_key)
            .map(|v| v.to_boolean())
            .unwrap_or(false);
        let configurable = desc_borrowed
            .get_property(&configurable_key)
            .map(|v| v.to_boolean())
            .unwrap_or(false);

        // Check for getter/setter
        let getter = desc_borrowed.get_property(&get_key);
        let setter = desc_borrowed.get_property(&set_key);
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
            prop.set_enumerable(enumerable);
            prop.set_configurable(configurable);
            obj_ref.borrow_mut().define_property(key, prop);
        } else {
            // Data descriptor
            let prop = Property::with_attributes(value, writable, enumerable, configurable);
            obj_ref.borrow_mut().define_property(key, prop);
        }
    }

    // Object was passed in by caller, already owned - no guard needed
    Ok(Guarded::unguarded(obj))
}

/// Object.getPrototypeOf(obj)
pub fn object_get_prototype_of(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_ref) = obj else {
        return Err(JsError::type_error(
            "Object.getPrototypeOf requires an object",
        ));
    };

    // Use proxy trap if it's a proxy
    if is_proxy(&obj_ref) {
        return proxy_get_prototype_of(interp, obj_ref);
    }

    let obj_borrowed = obj_ref.borrow();
    match &obj_borrowed.prototype {
        Some(proto) => Ok(Guarded::unguarded(JsValue::Object(proto.clone()))),
        None => Ok(Guarded::unguarded(JsValue::Null)),
    }
}

/// Object.setPrototypeOf(obj, proto)
pub fn object_set_prototype_of(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let proto = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_ref) = obj.clone() else {
        return Err(JsError::type_error(
            "Object.setPrototypeOf requires an object",
        ));
    };

    // Use proxy trap if it's a proxy
    if is_proxy(&obj_ref) {
        proxy_set_prototype_of(interp, obj_ref, proto)?;
        return Ok(Guarded::unguarded(obj));
    }

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
    // Object was passed in by caller, already owned - no guard needed
    Ok(Guarded::unguarded(obj))
}

/// Object.is(value1, value2)
/// Uses SameValue algorithm which differs from === in handling of NaN and -0
pub fn object_is(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let value1 = args.first().cloned().unwrap_or(JsValue::Undefined);
    let value2 = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let result = same_value(&value1, &value2);
    Ok(Guarded::unguarded(JsValue::Boolean(result)))
}

/// SameValue algorithm (used by Object.is)
/// Different from strict equality (===) in that:
/// - NaN is equal to NaN
/// - +0 is NOT equal to -0
fn same_value(x: &JsValue, y: &JsValue) -> bool {
    match (x, y) {
        (JsValue::Undefined, JsValue::Undefined) => true,
        (JsValue::Null, JsValue::Null) => true,
        (JsValue::Boolean(a), JsValue::Boolean(b)) => a == b,
        (JsValue::Number(a), JsValue::Number(b)) => {
            // Handle NaN: NaN is equal to NaN
            if a.is_nan() && b.is_nan() {
                return true;
            }
            // Handle -0 vs +0: they are NOT equal
            if *a == 0.0 && *b == 0.0 {
                // Check sign bit: 1/0.0 = Infinity, 1/-0.0 = -Infinity
                return a.signum() == b.signum() || (a.is_nan() && b.is_nan());
            }
            a == b
        }
        (JsValue::String(a), JsValue::String(b)) => a == b,
        (JsValue::Symbol(a), JsValue::Symbol(b)) => a == b,
        (JsValue::Object(a), JsValue::Object(b)) => a == b,
        _ => false,
    }
}

/// Object.preventExtensions(obj)
/// Prevents new properties from being added to an object
pub fn object_prevent_extensions(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    if let JsValue::Object(obj_ref) = &obj {
        // Use proxy trap if it's a proxy
        if is_proxy(obj_ref) {
            proxy_prevent_extensions(interp, obj_ref.clone())?;
        } else {
            obj_ref.borrow_mut().extensible = false;
        }
    }

    // Return with guard to protect the object until caller stores it
    let guard = interp.guard_value(&obj);
    Ok(Guarded { value: obj, guard })
}

/// Object.isExtensible(obj)
/// Returns true if new properties can be added to the object
pub fn object_is_extensible(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    let is_extensible = match obj {
        JsValue::Object(ref obj_ref) if is_proxy(obj_ref) => {
            proxy_is_extensible(interp, obj_ref.clone())?
        }
        JsValue::Object(obj_ref) => obj_ref.borrow().extensible,
        _ => false, // Non-objects are not extensible
    };

    Ok(Guarded::unguarded(JsValue::Boolean(is_extensible)))
}

/// Object.getOwnPropertyDescriptors(obj)
/// Returns an object containing all own property descriptors
pub fn object_get_own_property_descriptors(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);

    // ES2015+: Convert to object (primitives get boxed, null/undefined throw)
    let obj_ref = interp.to_object(obj)?;

    // Pre-intern descriptor property keys
    let get_key = PropertyKey::String(interp.intern("get"));
    let set_key = PropertyKey::String(interp.intern("set"));
    let value_key = PropertyKey::String(interp.intern("value"));
    let writable_key = PropertyKey::String(interp.intern("writable"));
    let enumerable_key = PropertyKey::String(interp.intern("enumerable"));
    let configurable_key = PropertyKey::String(interp.intern("configurable"));

    // Collect all property keys first
    let prop_keys: Vec<PropertyKey> = {
        let obj_borrowed = obj_ref.borrow();
        obj_borrowed.properties.keys().cloned().collect()
    };

    // Create result object
    let result_guard = interp.heap.create_guard();
    let result = interp.create_object(&result_guard);

    for key in prop_keys {
        let property = {
            let obj_borrowed = obj_ref.borrow();
            obj_borrowed.get_own_property(&key).cloned()
        };

        if let Some(property) = property {
            // Create descriptor object for this property
            let desc = interp.create_object(&result_guard);
            {
                let mut desc_ref = desc.borrow_mut();

                if property.is_accessor() {
                    // Accessor descriptor
                    if let Some(getter) = property.getter() {
                        desc_ref.set_property(get_key.clone(), JsValue::Object(getter.clone()));
                    } else {
                        desc_ref.set_property(get_key.clone(), JsValue::Undefined);
                    }
                    if let Some(setter) = property.setter() {
                        desc_ref.set_property(set_key.clone(), JsValue::Object(setter.clone()));
                    } else {
                        desc_ref.set_property(set_key.clone(), JsValue::Undefined);
                    }
                } else {
                    // Data descriptor
                    desc_ref.set_property(value_key.clone(), property.value.clone());
                    desc_ref
                        .set_property(writable_key.clone(), JsValue::Boolean(property.writable()));
                }

                desc_ref.set_property(
                    enumerable_key.clone(),
                    JsValue::Boolean(property.enumerable()),
                );
                desc_ref.set_property(
                    configurable_key.clone(),
                    JsValue::Boolean(property.configurable()),
                );
            }

            result.borrow_mut().set_property(key, JsValue::Object(desc));
        }
    }

    Ok(Guarded::with_guard(JsValue::Object(result), result_guard))
}
