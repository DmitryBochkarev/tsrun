//! Proxy and Reflect built-in implementations
//!
//! Proxy allows customizing fundamental object operations through handler traps.
//! Reflect provides methods that mirror the proxy trap operations.

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::prelude::{ToString, Vec, vec};
use crate::value::{
    ExoticObject, Guarded, JsFunction, JsObject, JsObjectRef, JsValue, Property, PropertyKey,
    ProxyData,
};

// =============================================================================
// Proxy Constructor
// =============================================================================

/// Initialize the Proxy constructor and Reflect object
pub fn init_proxy(interp: &mut Interpreter) {
    // Create Proxy constructor
    let proxy_ctor = interp.create_native_function("Proxy", proxy_constructor, 2);
    interp.root_guard.guard(proxy_ctor.clone());

    // Add Proxy.revocable static method
    let revocable_fn = interp.create_native_function("revocable", proxy_revocable, 2);
    interp.root_guard.guard(revocable_fn.clone());
    let revocable_key = PropertyKey::String(interp.intern("revocable"));
    proxy_ctor
        .borrow_mut()
        .set_property(revocable_key, JsValue::Object(revocable_fn));

    // Register Proxy globally
    let proxy_key = PropertyKey::String(interp.intern("Proxy"));
    interp
        .global
        .borrow_mut()
        .set_property(proxy_key, JsValue::Object(proxy_ctor));

    // Create Reflect object
    init_reflect(interp);
}

/// Proxy constructor: new Proxy(target, handler)
fn proxy_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // Proxy must be called with 'new'
    // When called with 'new', this is the newly created object
    // When called without 'new', this is undefined or global
    if matches!(this, JsValue::Undefined) {
        return Err(JsError::type_error("Constructor Proxy requires 'new'"));
    }

    let target = args.first().cloned().unwrap_or(JsValue::Undefined);
    let handler = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Both target and handler must be objects
    let JsValue::Object(target_obj) = target else {
        return Err(JsError::type_error(
            "Cannot create proxy with a non-object as target",
        ));
    };
    let JsValue::Object(handler_obj) = handler else {
        return Err(JsError::type_error(
            "Cannot create proxy with a non-object as handler",
        ));
    };

    let guard = interp.heap.create_guard();
    let proxy = create_proxy(&guard, target_obj, handler_obj);

    Ok(Guarded::with_guard(JsValue::Object(proxy), guard))
}

/// Create a proxy object
pub fn create_proxy(
    guard: &crate::gc::Guard<JsObject>,
    target: JsObjectRef,
    handler: JsObjectRef,
) -> JsObjectRef {
    let proxy = guard.alloc();
    {
        let mut proxy_ref = proxy.borrow_mut();
        proxy_ref.exotic = ExoticObject::Proxy(ProxyData {
            target,
            handler,
            revoked: false,
        });
    }
    proxy
}

/// Proxy.revocable(target, handler) - creates a revocable proxy
fn proxy_revocable(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);
    let handler = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Both target and handler must be objects
    let JsValue::Object(target_obj) = target else {
        return Err(JsError::type_error(
            "Cannot create proxy with a non-object as target",
        ));
    };
    let JsValue::Object(handler_obj) = handler else {
        return Err(JsError::type_error(
            "Cannot create proxy with a non-object as handler",
        ));
    };

    let guard = interp.heap.create_guard();

    // Create the proxy
    let proxy = create_proxy(&guard, target_obj, handler_obj);
    guard.guard(proxy.clone());

    // Create the revoke function that captures the proxy reference
    // We need to store the proxy reference in the revoke function's closure
    let revoke_fn = create_revoke_function(interp, &guard, proxy.clone());

    // Create result object { proxy, revoke }
    let result = interp.create_object(&guard);
    let proxy_key = PropertyKey::String(interp.intern("proxy"));
    let revoke_key = PropertyKey::String(interp.intern("revoke"));

    result
        .borrow_mut()
        .set_property(proxy_key, JsValue::Object(proxy));
    result
        .borrow_mut()
        .set_property(revoke_key, JsValue::Object(revoke_fn));

    Ok(Guarded::with_guard(JsValue::Object(result), guard))
}

/// Create a revoke function for a revocable proxy
fn create_revoke_function(
    interp: &mut Interpreter,
    guard: &crate::gc::Guard<JsObject>,
    proxy: JsObjectRef,
) -> JsObjectRef {
    // Use the JsFunction::ProxyRevoke variant which stores the proxy reference
    let revoke_fn = guard.alloc();
    {
        let mut fn_ref = revoke_fn.borrow_mut();
        fn_ref.exotic = ExoticObject::Function(JsFunction::ProxyRevoke(proxy));
        fn_ref.prototype = Some(interp.function_prototype.clone());
    }
    revoke_fn
}

// =============================================================================
// Reflect Object
// =============================================================================

/// Initialize the Reflect object with all its methods
fn init_reflect(interp: &mut Interpreter) {
    let guard = interp.heap.create_guard();
    let reflect = interp.create_object(&guard);
    interp.root_guard.guard(reflect.clone());

    // Register all Reflect methods
    interp.register_method(&reflect, "get", reflect_get, 2);
    interp.register_method(&reflect, "set", reflect_set, 3);
    interp.register_method(&reflect, "has", reflect_has, 2);
    interp.register_method(&reflect, "deleteProperty", reflect_delete_property, 2);
    interp.register_method(&reflect, "apply", reflect_apply, 3);
    interp.register_method(&reflect, "construct", reflect_construct, 2);
    interp.register_method(
        &reflect,
        "getOwnPropertyDescriptor",
        reflect_get_own_property_descriptor,
        2,
    );
    interp.register_method(&reflect, "defineProperty", reflect_define_property, 3);
    interp.register_method(&reflect, "getPrototypeOf", reflect_get_prototype_of, 1);
    interp.register_method(&reflect, "setPrototypeOf", reflect_set_prototype_of, 2);
    interp.register_method(&reflect, "isExtensible", reflect_is_extensible, 1);
    interp.register_method(&reflect, "preventExtensions", reflect_prevent_extensions, 1);
    interp.register_method(&reflect, "ownKeys", reflect_own_keys, 1);

    // Register Reflect globally
    let reflect_key = PropertyKey::String(interp.intern("Reflect"));
    interp
        .global
        .borrow_mut()
        .set_property(reflect_key, JsValue::Object(reflect));
}

/// Reflect.get(target, propertyKey [, receiver])
fn reflect_get(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);
    let property_key = args.get(1).cloned().unwrap_or(JsValue::Undefined);
    let receiver = args.get(2).cloned().unwrap_or_else(|| target.clone());

    let JsValue::Object(obj) = target else {
        return Err(JsError::type_error("Reflect.get called on non-object"));
    };

    let key = PropertyKey::from_value(&property_key);

    // Use proxy_get if it's a proxy, otherwise normal get
    proxy_get(interp, obj, key, receiver)
}

/// Reflect.set(target, propertyKey, value [, receiver])
fn reflect_set(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);
    let property_key = args.get(1).cloned().unwrap_or(JsValue::Undefined);
    let value = args.get(2).cloned().unwrap_or(JsValue::Undefined);
    let receiver = args.get(3).cloned().unwrap_or_else(|| target.clone());

    let JsValue::Object(obj) = target else {
        return Err(JsError::type_error("Reflect.set called on non-object"));
    };

    let key = PropertyKey::from_value(&property_key);

    // Use proxy_set if it's a proxy, otherwise normal set
    let result = proxy_set(interp, obj, key, value, receiver)?;
    Ok(Guarded::unguarded(JsValue::Boolean(result)))
}

/// Reflect.has(target, propertyKey)
fn reflect_has(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);
    let property_key = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj) = target else {
        return Err(JsError::type_error("Reflect.has called on non-object"));
    };

    let key = PropertyKey::from_value(&property_key);

    let result = proxy_has(interp, obj, &key)?;
    Ok(Guarded::unguarded(JsValue::Boolean(result)))
}

/// Reflect.deleteProperty(target, propertyKey)
fn reflect_delete_property(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);
    let property_key = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj) = target else {
        return Err(JsError::type_error(
            "Reflect.deleteProperty called on non-object",
        ));
    };

    let key = PropertyKey::from_value(&property_key);

    let result = proxy_delete_property(interp, obj, &key)?;
    Ok(Guarded::unguarded(JsValue::Boolean(result)))
}

/// Reflect.apply(target, thisArgument, argumentsList)
fn reflect_apply(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);
    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);
    let arguments_list = args.get(2).cloned().unwrap_or(JsValue::Undefined);

    if !target.is_callable() {
        return Err(JsError::type_error("Reflect.apply: target is not callable"));
    }

    // Convert arguments list to array
    let call_args = if let JsValue::Object(arr) = arguments_list {
        let arr_ref = arr.borrow();
        if let Some(elements) = arr_ref.array_elements() {
            elements.to_vec()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    interp.call_function(target, this_arg, &call_args)
}

/// Reflect.construct(target, argumentsList [, newTarget])
fn reflect_construct(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);
    let arguments_list = args.get(1).cloned().unwrap_or(JsValue::Undefined);
    let new_target = args.get(2).cloned().unwrap_or_else(|| target.clone());

    if !target.is_callable() {
        return Err(JsError::type_error(
            "Reflect.construct: target is not a constructor",
        ));
    }

    // Convert arguments list to array
    let call_args: Vec<JsValue> = if let JsValue::Object(arr) = arguments_list {
        let arr_ref = arr.borrow();
        if let Some(elements) = arr_ref.array_elements() {
            elements.to_vec()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // Get the target's prototype from newTarget if different
    let prototype = if let JsValue::Object(new_target_obj) = &new_target {
        let proto_key = PropertyKey::String(interp.intern("prototype"));
        new_target_obj.borrow().get_property(&proto_key)
    } else {
        None
    };

    // Create new object with the appropriate prototype
    let guard = interp.heap.create_guard();
    let new_obj = interp.create_object(&guard);
    guard.guard(new_obj.clone());

    if let Some(JsValue::Object(proto)) = prototype {
        new_obj.borrow_mut().prototype = Some(proto);
    } else if let JsValue::Object(target_obj) = &target {
        // Use target's prototype.prototype as the instance prototype
        let proto_key = PropertyKey::String(interp.intern("prototype"));
        if let Some(JsValue::Object(proto)) = target_obj.borrow().get_property(&proto_key) {
            new_obj.borrow_mut().prototype = Some(proto);
        }
    }

    // Call the constructor with the new object as `this`
    let Guarded {
        value: result,
        guard: result_guard,
    } = interp.call_function(target, JsValue::Object(new_obj.clone()), &call_args)?;

    // If constructor returned an object, use that; otherwise use the created object
    match result {
        JsValue::Object(_) => Ok(Guarded {
            value: result,
            guard: result_guard,
        }),
        _ => Ok(Guarded::with_guard(JsValue::Object(new_obj), guard)),
    }
}

/// Reflect.getOwnPropertyDescriptor(target, propertyKey)
fn reflect_get_own_property_descriptor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);
    let property_key = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj) = target else {
        return Err(JsError::type_error(
            "Reflect.getOwnPropertyDescriptor called on non-object",
        ));
    };

    let key = PropertyKey::from_value(&property_key);

    proxy_get_own_property_descriptor(interp, obj, &key)
}

/// Reflect.defineProperty(target, propertyKey, attributes)
fn reflect_define_property(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);
    let property_key = args.get(1).cloned().unwrap_or(JsValue::Undefined);
    let attributes = args.get(2).cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj) = target else {
        return Err(JsError::type_error(
            "Reflect.defineProperty called on non-object",
        ));
    };

    let key = PropertyKey::from_value(&property_key);

    let result = proxy_define_property(interp, obj, key, attributes)?;
    Ok(Guarded::unguarded(JsValue::Boolean(result)))
}

/// Reflect.getPrototypeOf(target)
fn reflect_get_prototype_of(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj) = target else {
        return Err(JsError::type_error(
            "Reflect.getPrototypeOf called on non-object",
        ));
    };

    proxy_get_prototype_of(interp, obj)
}

/// Reflect.setPrototypeOf(target, proto)
fn reflect_set_prototype_of(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);
    let proto = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj) = target else {
        return Err(JsError::type_error(
            "Reflect.setPrototypeOf called on non-object",
        ));
    };

    let result = proxy_set_prototype_of(interp, obj, proto)?;
    Ok(Guarded::unguarded(JsValue::Boolean(result)))
}

/// Reflect.isExtensible(target)
fn reflect_is_extensible(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj) = target else {
        return Err(JsError::type_error(
            "Reflect.isExtensible called on non-object",
        ));
    };

    let result = proxy_is_extensible(interp, obj)?;
    Ok(Guarded::unguarded(JsValue::Boolean(result)))
}

/// Reflect.preventExtensions(target)
fn reflect_prevent_extensions(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj) = target else {
        return Err(JsError::type_error(
            "Reflect.preventExtensions called on non-object",
        ));
    };

    let result = proxy_prevent_extensions(interp, obj)?;
    Ok(Guarded::unguarded(JsValue::Boolean(result)))
}

/// Reflect.ownKeys(target)
fn reflect_own_keys(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj) = target else {
        return Err(JsError::type_error("Reflect.ownKeys called on non-object"));
    };

    proxy_own_keys(interp, obj)
}

// =============================================================================
// Proxy Trap Implementations
// =============================================================================

/// Get a trap function from the handler, or None if not defined/undefined
fn get_trap(interp: &mut Interpreter, handler: &JsObjectRef, trap_name: &str) -> Option<JsValue> {
    let key = PropertyKey::String(interp.intern(trap_name));
    let trap = handler.borrow().get_property(&key)?;
    if trap.is_null_or_undefined() {
        None
    } else {
        Some(trap)
    }
}

/// Convert a PropertyKey to a JsValue for passing to trap handlers
fn property_key_to_value(key: &PropertyKey) -> JsValue {
    match key {
        PropertyKey::String(s) => JsValue::String(s.clone()),
        PropertyKey::Index(i) => JsValue::String(crate::value::JsString::from(i.to_string())),
        PropertyKey::Symbol(s) => JsValue::Symbol(s.clone()),
    }
}

/// Proxy [[Get]] internal method
pub fn proxy_get(
    interp: &mut Interpreter,
    obj: JsObjectRef,
    key: PropertyKey,
    receiver: JsValue,
) -> Result<Guarded, JsError> {
    // Check if this is actually a proxy
    let (target, handler) = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Proxy(data) => {
                if data.revoked {
                    return Err(JsError::type_error(
                        "Cannot perform 'get' on a revoked proxy",
                    ));
                }
                (data.target.clone(), data.handler.clone())
            }
            _ => {
                // Not a proxy, do normal property access
                drop(obj_ref);
                let prop = obj.borrow().get_property_descriptor(&key);
                match prop {
                    Some((p, _)) if p.is_accessor() => {
                        if let Some(getter) = p.getter() {
                            return interp.call_function(
                                JsValue::Object(getter.clone()),
                                receiver,
                                &[],
                            );
                        }
                        return Ok(Guarded::unguarded(JsValue::Undefined));
                    }
                    Some((p, _)) => return Ok(Guarded::unguarded(p.value)),
                    None => return Ok(Guarded::unguarded(JsValue::Undefined)),
                }
            }
        }
    };

    // Check for get trap
    if let Some(trap) = get_trap(interp, &handler, "get") {
        let key_val = property_key_to_value(&key);
        return interp.call_function(
            trap,
            JsValue::Object(handler),
            &[JsValue::Object(target), key_val, receiver],
        );
    }

    // No trap, forward to target
    proxy_get(interp, target, key, receiver)
}

/// Proxy [[Set]] internal method
pub fn proxy_set(
    interp: &mut Interpreter,
    obj: JsObjectRef,
    key: PropertyKey,
    value: JsValue,
    receiver: JsValue,
) -> Result<bool, JsError> {
    // Check if this is actually a proxy
    let (target, handler) = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Proxy(data) => {
                if data.revoked {
                    return Err(JsError::type_error(
                        "Cannot perform 'set' on a revoked proxy",
                    ));
                }
                (data.target.clone(), data.handler.clone())
            }
            _ => {
                // Not a proxy, do normal property set
                drop(obj_ref);
                let prop_desc = obj.borrow().get_property_descriptor(&key);
                if let Some((p, _)) = prop_desc
                    && p.is_accessor()
                {
                    if let Some(setter) = p.setter() {
                        interp.call_function(
                            JsValue::Object(setter.clone()),
                            receiver,
                            &[value],
                        )?;
                        return Ok(true);
                    }
                    return Ok(false);
                }
                obj.borrow_mut().set_property(key, value);
                return Ok(true);
            }
        }
    };

    // Check for set trap
    if let Some(trap) = get_trap(interp, &handler, "set") {
        let key_val = property_key_to_value(&key);
        let Guarded { value: result, .. } = interp.call_function(
            trap,
            JsValue::Object(handler),
            &[JsValue::Object(target), key_val, value, receiver],
        )?;
        return Ok(result.to_boolean());
    }

    // No trap, forward to target
    proxy_set(interp, target, key, value, receiver)
}

/// Proxy [[Has]] internal method (in operator)
pub fn proxy_has(
    interp: &mut Interpreter,
    obj: JsObjectRef,
    key: &PropertyKey,
) -> Result<bool, JsError> {
    // Check if this is actually a proxy
    let (target, handler) = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Proxy(data) => {
                if data.revoked {
                    return Err(JsError::type_error(
                        "Cannot perform 'has' on a revoked proxy",
                    ));
                }
                (data.target.clone(), data.handler.clone())
            }
            _ => {
                // Not a proxy, do normal has check
                drop(obj_ref);
                return Ok(obj.borrow().get_property(key).is_some());
            }
        }
    };

    // Check for has trap
    if let Some(trap) = get_trap(interp, &handler, "has") {
        let key_val = property_key_to_value(key);
        let Guarded { value: result, .. } = interp.call_function(
            trap,
            JsValue::Object(handler),
            &[JsValue::Object(target), key_val],
        )?;
        return Ok(result.to_boolean());
    }

    // No trap, forward to target
    proxy_has(interp, target, key)
}

/// Proxy [[Delete]] internal method
pub fn proxy_delete_property(
    interp: &mut Interpreter,
    obj: JsObjectRef,
    key: &PropertyKey,
) -> Result<bool, JsError> {
    // Check if this is actually a proxy
    let (target, handler) = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Proxy(data) => {
                if data.revoked {
                    return Err(JsError::type_error(
                        "Cannot perform 'deleteProperty' on a revoked proxy",
                    ));
                }
                (data.target.clone(), data.handler.clone())
            }
            _ => {
                // Not a proxy, do normal delete
                drop(obj_ref);
                obj.borrow_mut().properties.remove(key);
                return Ok(true);
            }
        }
    };

    // Check for deleteProperty trap
    if let Some(trap) = get_trap(interp, &handler, "deleteProperty") {
        let key_val = property_key_to_value(key);
        let Guarded { value: result, .. } = interp.call_function(
            trap,
            JsValue::Object(handler),
            &[JsValue::Object(target), key_val],
        )?;
        return Ok(result.to_boolean());
    }

    // No trap, forward to target
    proxy_delete_property(interp, target, key)
}

/// Proxy [[GetOwnProperty]] internal method
pub fn proxy_get_own_property_descriptor(
    interp: &mut Interpreter,
    obj: JsObjectRef,
    key: &PropertyKey,
) -> Result<Guarded, JsError> {
    // Check if this is actually a proxy
    let (target, handler) = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Proxy(data) => {
                if data.revoked {
                    return Err(JsError::type_error(
                        "Cannot perform 'getOwnPropertyDescriptor' on a revoked proxy",
                    ));
                }
                (data.target.clone(), data.handler.clone())
            }
            _ => {
                // Not a proxy, return normal descriptor
                drop(obj_ref);
                return create_property_descriptor(interp, &obj, key);
            }
        }
    };

    // Check for getOwnPropertyDescriptor trap
    if let Some(trap) = get_trap(interp, &handler, "getOwnPropertyDescriptor") {
        let key_val = property_key_to_value(key);
        return interp.call_function(
            trap,
            JsValue::Object(handler),
            &[JsValue::Object(target), key_val],
        );
    }

    // No trap, forward to target
    proxy_get_own_property_descriptor(interp, target, key)
}

/// Proxy [[DefineOwnProperty]] internal method
pub fn proxy_define_property(
    interp: &mut Interpreter,
    obj: JsObjectRef,
    key: PropertyKey,
    descriptor: JsValue,
) -> Result<bool, JsError> {
    // Check if this is actually a proxy
    let (target, handler) = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Proxy(data) => {
                if data.revoked {
                    return Err(JsError::type_error(
                        "Cannot perform 'defineProperty' on a revoked proxy",
                    ));
                }
                (data.target.clone(), data.handler.clone())
            }
            _ => {
                // Not a proxy, do normal define
                drop(obj_ref);
                define_property_from_descriptor(interp, &obj, key, descriptor)?;
                return Ok(true);
            }
        }
    };

    // Check for defineProperty trap
    if let Some(trap) = get_trap(interp, &handler, "defineProperty") {
        let key_val = property_key_to_value(&key);
        let Guarded { value: result, .. } = interp.call_function(
            trap,
            JsValue::Object(handler),
            &[JsValue::Object(target), key_val, descriptor],
        )?;
        return Ok(result.to_boolean());
    }

    // No trap, forward to target
    proxy_define_property(interp, target, key, descriptor)
}

/// Proxy [[GetPrototypeOf]] internal method
pub fn proxy_get_prototype_of(
    interp: &mut Interpreter,
    obj: JsObjectRef,
) -> Result<Guarded, JsError> {
    // Check if this is actually a proxy
    let (target, handler) = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Proxy(data) => {
                if data.revoked {
                    return Err(JsError::type_error(
                        "Cannot perform 'getPrototypeOf' on a revoked proxy",
                    ));
                }
                (data.target.clone(), data.handler.clone())
            }
            _ => {
                // Not a proxy, return normal prototype
                drop(obj_ref);
                let proto = obj.borrow().prototype.clone();
                return Ok(Guarded::unguarded(match proto {
                    Some(p) => JsValue::Object(p),
                    None => JsValue::Null,
                }));
            }
        }
    };

    // Check for getPrototypeOf trap
    if let Some(trap) = get_trap(interp, &handler, "getPrototypeOf") {
        return interp.call_function(trap, JsValue::Object(handler), &[JsValue::Object(target)]);
    }

    // No trap, forward to target
    proxy_get_prototype_of(interp, target)
}

/// Proxy [[SetPrototypeOf]] internal method
pub fn proxy_set_prototype_of(
    interp: &mut Interpreter,
    obj: JsObjectRef,
    proto: JsValue,
) -> Result<bool, JsError> {
    // Check if this is actually a proxy
    let (target, handler) = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Proxy(data) => {
                if data.revoked {
                    return Err(JsError::type_error(
                        "Cannot perform 'setPrototypeOf' on a revoked proxy",
                    ));
                }
                (data.target.clone(), data.handler.clone())
            }
            _ => {
                // Not a proxy, do normal set
                drop(obj_ref);
                let new_proto = match proto {
                    JsValue::Null => None,
                    JsValue::Object(p) => Some(p),
                    _ => return Err(JsError::type_error("Prototype must be object or null")),
                };
                obj.borrow_mut().prototype = new_proto;
                return Ok(true);
            }
        }
    };

    // Check for setPrototypeOf trap
    if let Some(trap) = get_trap(interp, &handler, "setPrototypeOf") {
        let Guarded { value: result, .. } = interp.call_function(
            trap,
            JsValue::Object(handler),
            &[JsValue::Object(target), proto],
        )?;
        return Ok(result.to_boolean());
    }

    // No trap, forward to target
    proxy_set_prototype_of(interp, target, proto)
}

/// Proxy [[IsExtensible]] internal method
pub fn proxy_is_extensible(interp: &mut Interpreter, obj: JsObjectRef) -> Result<bool, JsError> {
    // Check if this is actually a proxy
    let (target, handler) = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Proxy(data) => {
                if data.revoked {
                    return Err(JsError::type_error(
                        "Cannot perform 'isExtensible' on a revoked proxy",
                    ));
                }
                (data.target.clone(), data.handler.clone())
            }
            _ => {
                // Not a proxy, return normal extensible
                drop(obj_ref);
                return Ok(obj.borrow().extensible);
            }
        }
    };

    // Check for isExtensible trap
    if let Some(trap) = get_trap(interp, &handler, "isExtensible") {
        let Guarded { value: result, .. } =
            interp.call_function(trap, JsValue::Object(handler), &[JsValue::Object(target)])?;
        return Ok(result.to_boolean());
    }

    // No trap, forward to target
    proxy_is_extensible(interp, target)
}

/// Proxy [[PreventExtensions]] internal method
pub fn proxy_prevent_extensions(
    interp: &mut Interpreter,
    obj: JsObjectRef,
) -> Result<bool, JsError> {
    // Check if this is actually a proxy
    let (target, handler) = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Proxy(data) => {
                if data.revoked {
                    return Err(JsError::type_error(
                        "Cannot perform 'preventExtensions' on a revoked proxy",
                    ));
                }
                (data.target.clone(), data.handler.clone())
            }
            _ => {
                // Not a proxy, do normal prevent
                drop(obj_ref);
                obj.borrow_mut().extensible = false;
                return Ok(true);
            }
        }
    };

    // Check for preventExtensions trap
    if let Some(trap) = get_trap(interp, &handler, "preventExtensions") {
        let Guarded { value: result, .. } =
            interp.call_function(trap, JsValue::Object(handler), &[JsValue::Object(target)])?;
        return Ok(result.to_boolean());
    }

    // No trap, forward to target
    proxy_prevent_extensions(interp, target)
}

/// Proxy [[OwnPropertyKeys]] internal method
pub fn proxy_own_keys(interp: &mut Interpreter, obj: JsObjectRef) -> Result<Guarded, JsError> {
    // Check if this is actually a proxy
    let (target, handler) = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Proxy(data) => {
                if data.revoked {
                    return Err(JsError::type_error(
                        "Cannot perform 'ownKeys' on a revoked proxy",
                    ));
                }
                (data.target.clone(), data.handler.clone())
            }
            _ => {
                // Not a proxy, return normal keys
                drop(obj_ref);
                return get_own_keys(interp, &obj);
            }
        }
    };

    // Check for ownKeys trap
    if let Some(trap) = get_trap(interp, &handler, "ownKeys") {
        return interp.call_function(trap, JsValue::Object(handler), &[JsValue::Object(target)]);
    }

    // No trap, forward to target
    proxy_own_keys(interp, target)
}

/// Proxy [[Call]] internal method (for function proxies)
///
/// Implements the [[Call]] internal method for Proxy exotic objects.
/// Creates its own guard internally for the arguments array.
pub fn proxy_apply(
    interp: &mut Interpreter,
    obj: JsObjectRef,
    this_arg: JsValue,
    args: Vec<JsValue>,
) -> Result<Guarded, JsError> {
    // Check if this is actually a proxy
    let (target, handler) = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Proxy(data) => {
                if data.revoked {
                    return Err(JsError::type_error(
                        "Cannot perform 'apply' on a revoked proxy",
                    ));
                }
                (data.target.clone(), data.handler.clone())
            }
            _ => {
                // Not a proxy, should not be called
                return Err(JsError::type_error("proxy_apply called on non-proxy"));
            }
        }
    };

    // Check for apply trap
    if let Some(trap) = get_trap(interp, &handler, "apply") {
        // Create args array
        let guard = interp.heap.create_guard();
        let args_array = interp.create_array_from(&guard, args);

        return interp.call_function(
            trap,
            JsValue::Object(handler),
            &[
                JsValue::Object(target),
                this_arg,
                JsValue::Object(args_array),
            ],
        );
    }

    // No trap, forward to target
    interp.call_function(JsValue::Object(target), this_arg, &args)
}

/// Proxy [[Construct]] internal method (for constructor proxies)
pub fn proxy_construct(
    interp: &mut Interpreter,
    obj: JsObjectRef,
    args: Vec<JsValue>,
    new_target: JsValue,
) -> Result<Guarded, JsError> {
    // Check if this is actually a proxy
    let (target, handler) = {
        let obj_ref = obj.borrow();
        match &obj_ref.exotic {
            ExoticObject::Proxy(data) => {
                if data.revoked {
                    return Err(JsError::type_error(
                        "Cannot perform 'construct' on a revoked proxy",
                    ));
                }
                (data.target.clone(), data.handler.clone())
            }
            _ => {
                // Not a proxy, should not be called
                return Err(JsError::type_error("proxy_construct called on non-proxy"));
            }
        }
    };

    // Check for construct trap
    if let Some(trap) = get_trap(interp, &handler, "construct") {
        // Create args array
        let guard = interp.heap.create_guard();
        let args_array = interp.create_array_from(&guard, args.clone());

        let Guarded {
            value: result,
            guard: result_guard,
        } = interp.call_function(
            trap,
            JsValue::Object(handler),
            &[
                JsValue::Object(target),
                JsValue::Object(args_array),
                new_target,
            ],
        )?;

        // Construct trap must return an object
        if !matches!(result, JsValue::Object(_)) {
            return Err(JsError::type_error(
                "'construct' trap must return an object",
            ));
        }

        return Ok(Guarded {
            value: result,
            guard: result_guard,
        });
    }

    // No trap, forward to target as constructor
    // Create new object and call target
    let guard = interp.heap.create_guard();
    let new_obj = interp.create_object(&guard);
    guard.guard(new_obj.clone());

    // Set prototype from target
    let proto_key = PropertyKey::String(interp.intern("prototype"));
    if let Some(JsValue::Object(proto)) = target.borrow().get_property(&proto_key) {
        new_obj.borrow_mut().prototype = Some(proto);
    }

    let Guarded {
        value: result,
        guard: result_guard,
    } = interp.call_function(
        JsValue::Object(target),
        JsValue::Object(new_obj.clone()),
        &args,
    )?;

    // If constructor returned an object, use that; otherwise use new_obj
    match result {
        JsValue::Object(_) => Ok(Guarded {
            value: result,
            guard: result_guard,
        }),
        _ => Ok(Guarded::with_guard(JsValue::Object(new_obj), guard)),
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a property descriptor object from an object's property
fn create_property_descriptor(
    interp: &mut Interpreter,
    obj: &JsObjectRef,
    key: &PropertyKey,
) -> Result<Guarded, JsError> {
    let prop = obj.borrow().get_own_property(key).cloned();

    match prop {
        Some(p) => {
            let guard = interp.heap.create_guard();
            let desc = interp.create_object(&guard);

            let value_key = PropertyKey::String(interp.intern("value"));
            let writable_key = PropertyKey::String(interp.intern("writable"));
            let enumerable_key = PropertyKey::String(interp.intern("enumerable"));
            let configurable_key = PropertyKey::String(interp.intern("configurable"));

            // Extract attributes before moving p.value
            let writable = p.writable();
            let enumerable = p.enumerable();
            let configurable = p.configurable();

            {
                let mut desc_ref = desc.borrow_mut();
                desc_ref.set_property(value_key, p.value);
                desc_ref.set_property(writable_key, JsValue::Boolean(writable));
                desc_ref.set_property(enumerable_key, JsValue::Boolean(enumerable));
                desc_ref.set_property(configurable_key, JsValue::Boolean(configurable));
            }

            Ok(Guarded::with_guard(JsValue::Object(desc), guard))
        }
        None => Ok(Guarded::unguarded(JsValue::Undefined)),
    }
}

/// Define a property from a descriptor object
fn define_property_from_descriptor(
    interp: &mut Interpreter,
    obj: &JsObjectRef,
    key: PropertyKey,
    descriptor: JsValue,
) -> Result<(), JsError> {
    let JsValue::Object(desc_obj) = descriptor else {
        return Err(JsError::type_error("Property descriptor must be an object"));
    };

    let desc = desc_obj.borrow();
    let value_key = PropertyKey::String(interp.intern("value"));
    let writable_key = PropertyKey::String(interp.intern("writable"));
    let enumerable_key = PropertyKey::String(interp.intern("enumerable"));
    let configurable_key = PropertyKey::String(interp.intern("configurable"));

    let value = desc.get_property(&value_key).unwrap_or(JsValue::Undefined);
    let writable = desc
        .get_property(&writable_key)
        .is_none_or(|v| v.to_boolean());
    let enumerable = desc
        .get_property(&enumerable_key)
        .is_none_or(|v| v.to_boolean());
    let configurable = desc
        .get_property(&configurable_key)
        .is_none_or(|v| v.to_boolean());

    drop(desc);

    let prop = Property::with_attributes(value, writable, enumerable, configurable);
    obj.borrow_mut().define_property(key, prop);

    Ok(())
}

/// Get own property keys as an array
fn get_own_keys(interp: &mut Interpreter, obj: &JsObjectRef) -> Result<Guarded, JsError> {
    let keys: Vec<JsValue> = {
        let obj_ref = obj.borrow();

        // For arrays, include indices
        if let ExoticObject::Array { ref elements } = obj_ref.exotic {
            let mut keys: Vec<JsValue> = (0..elements.len())
                .map(|i| JsValue::String(crate::value::JsString::from(i.to_string())))
                .collect();

            // Add other string keys
            for key in obj_ref.properties.keys() {
                if let PropertyKey::String(s) = key {
                    keys.push(JsValue::String(s.clone()));
                }
            }

            keys
        } else {
            obj_ref
                .properties
                .keys()
                .map(property_key_to_value)
                .collect()
        }
    };

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, keys);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

/// Check if an object is a proxy
pub fn is_proxy(obj: &JsObjectRef) -> bool {
    matches!(obj.borrow().exotic, ExoticObject::Proxy(_))
}
