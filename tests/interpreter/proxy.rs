//! Proxy and Reflect tests
//!
//! Tests for the Proxy constructor, all proxy traps, Reflect methods,
//! and Proxy.revocable().

use super::{eval, eval_result, throws_error};
use typescript_eval::JsValue;

// =============================================================================
// Proxy Basic Tests
// =============================================================================

#[test]
fn test_proxy_basic_creation() {
    // Basic proxy creation with empty handler
    assert_eq!(
        eval("let target = { x: 1 }; let p = new Proxy(target, {}); p.x"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_proxy_typeof() {
    // typeof proxy should be 'object'
    assert_eq!(
        eval("let p = new Proxy({}, {}); typeof p"),
        JsValue::from("object")
    );
}

#[test]
fn test_proxy_target_passthrough() {
    // Without traps, proxy should pass through to target
    assert_eq!(
        eval(
            r#"
            let target = { a: 1, b: 2, c: 3 };
            let p = new Proxy(target, {});
            p.a + p.b + p.c
        "#
        ),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_proxy_array_target() {
    // Proxy can wrap arrays
    assert_eq!(
        eval(
            r#"
            let arr = [1, 2, 3];
            let p = new Proxy(arr, {});
            p[0] + p[1] + p[2]
        "#
        ),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_proxy_function_target() {
    // Proxy can wrap functions
    assert_eq!(
        eval(
            r#"
            let fn = function(x: number) { return x * 2; };
            let p = new Proxy(fn, {});
            p(5)
        "#
        ),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_proxy_constructor_requires_object_target() {
    // Target must be an object
    assert!(throws_error(
        "new Proxy(42, {})",
        "Cannot create proxy with a non-object"
    ));
}

#[test]
fn test_proxy_constructor_requires_object_handler() {
    // Handler must be an object
    assert!(throws_error(
        "new Proxy({}, null)",
        "Cannot create proxy with a non-object"
    ));
}

// =============================================================================
// get trap
// =============================================================================

#[test]
fn test_proxy_get_trap() {
    assert_eq!(
        eval(
            r#"
            let target = { x: 10 };
            let handler = {
                get(target: any, prop: string) {
                    return prop === 'x' ? target[prop] * 2 : target[prop];
                }
            };
            let p = new Proxy(target, handler);
            p.x
        "#
        ),
        JsValue::Number(20.0)
    );
}

#[test]
fn test_proxy_get_trap_receiver() {
    // The receiver should be the proxy itself
    assert_eq!(
        eval(
            r#"
            let target = { x: 1 };
            let receiverLog: any;
            let handler = {
                get(target: any, prop: string, receiver: any) {
                    receiverLog = receiver;
                    return target[prop];
                }
            };
            let p = new Proxy(target, handler);
            p.x;
            receiverLog === p
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_proxy_get_trap_intercepts_missing() {
    // get trap is called even for non-existent properties
    assert_eq!(
        eval(
            r#"
            let handler = {
                get(target: any, prop: string) {
                    return prop === 'missing' ? 'intercepted' : target[prop];
                }
            };
            let p = new Proxy({}, handler);
            p.missing
        "#
        ),
        JsValue::from("intercepted")
    );
}

#[test]
fn test_proxy_get_trap_symbol_property() {
    // get trap should work with Symbol properties
    assert_eq!(
        eval(
            r#"
            let sym = Symbol('test');
            let target = { [sym]: 'symbol value' };
            let propLog: any;
            let handler = {
                get(target: any, prop: any) {
                    propLog = prop;
                    return target[prop];
                }
            };
            let p = new Proxy(target, handler);
            p[sym];
            typeof propLog === 'symbol'
        "#
        ),
        JsValue::Boolean(true)
    );
}

// =============================================================================
// set trap
// =============================================================================

#[test]
fn test_proxy_set_trap() {
    assert_eq!(
        eval(
            r#"
            let target = { x: 0 };
            let handler = {
                set(target: any, prop: string, value: any) {
                    target[prop] = value * 2;
                    return true;
                }
            };
            let p = new Proxy(target, handler);
            p.x = 5;
            target.x
        "#
        ),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_proxy_set_trap_returns_false() {
    // If set returns false, assignment should fail in strict mode
    // For now, we just test that it can return false
    assert_eq!(
        eval(
            r#"
            let setResult: boolean;
            let handler = {
                set(target: any, prop: string, value: any) {
                    setResult = false;
                    return false;
                }
            };
            let p = new Proxy({}, handler);
            try {
                p.x = 5;
            } catch (e) {}
            setResult
        "#
        ),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_proxy_set_trap_receiver() {
    assert_eq!(
        eval(
            r#"
            let target = {};
            let receiverLog: any;
            let handler = {
                set(target: any, prop: string, value: any, receiver: any) {
                    receiverLog = receiver;
                    target[prop] = value;
                    return true;
                }
            };
            let p = new Proxy(target, handler);
            p.x = 1;
            receiverLog === p
        "#
        ),
        JsValue::Boolean(true)
    );
}

// =============================================================================
// has trap (in operator)
// =============================================================================

#[test]
fn test_proxy_has_trap() {
    assert_eq!(
        eval(
            r#"
            let target = { x: 1 };
            let handler = {
                has(target: any, prop: string) {
                    return prop === 'hidden' ? false : prop in target;
                }
            };
            let p = new Proxy(target, handler);
            'x' in p
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_proxy_has_trap_hides_property() {
    assert_eq!(
        eval(
            r#"
            let target = { secret: 'hidden' };
            let handler = {
                has(target: any, prop: string) {
                    return prop !== 'secret' && prop in target;
                }
            };
            let p = new Proxy(target, handler);
            'secret' in p
        "#
        ),
        JsValue::Boolean(false)
    );
}

// =============================================================================
// deleteProperty trap
// =============================================================================

#[test]
fn test_proxy_delete_property_trap() {
    assert_eq!(
        eval(
            r#"
            let target = { x: 1, y: 2 };
            let deletedProps: string[] = [];
            let handler = {
                deleteProperty(target: any, prop: string) {
                    deletedProps.push(prop);
                    delete target[prop];
                    return true;
                }
            };
            let p = new Proxy(target, handler);
            delete p.x;
            deletedProps[0]
        "#
        ),
        JsValue::from("x")
    );
}

#[test]
fn test_proxy_delete_property_prevents_delete() {
    assert_eq!(
        eval(
            r#"
            let target = { secret: 'value' };
            let handler = {
                deleteProperty(target: any, prop: string) {
                    if (prop === 'secret') return false;
                    delete target[prop];
                    return true;
                }
            };
            let p = new Proxy(target, handler);
            let result = delete p.secret;
            target.secret
        "#
        ),
        JsValue::from("value")
    );
}

// =============================================================================
// apply trap (function call)
// =============================================================================

#[test]
fn test_proxy_apply_trap() {
    assert_eq!(
        eval(
            r#"
            let target = function(a: number, b: number) { return a + b; };
            let handler = {
                apply(target: any, thisArg: any, args: any[]) {
                    return target.apply(thisArg, args) * 2;
                }
            };
            let p = new Proxy(target, handler);
            p(3, 4)
        "#
        ),
        JsValue::Number(14.0)
    );
}

#[test]
fn test_proxy_apply_trap_this_binding() {
    assert_eq!(
        eval(
            r#"
            let target = function() { return this.value; };
            let thisArgLog: any;
            let handler = {
                apply(target: any, thisArg: any, args: any[]) {
                    thisArgLog = thisArg;
                    return target.apply(thisArg, args);
                }
            };
            let p = new Proxy(target, handler);
            let obj = { value: 42 };
            p.call(obj);
            thisArgLog.value
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_proxy_apply_trap_modifies_args() {
    assert_eq!(
        eval(
            r#"
            let target = function(x: number) { return x; };
            let handler = {
                apply(target: any, thisArg: any, args: any[]) {
                    return target.apply(thisArg, [args[0] + 100]);
                }
            };
            let p = new Proxy(target, handler);
            p(5)
        "#
        ),
        JsValue::Number(105.0)
    );
}

// =============================================================================
// construct trap (new operator)
// =============================================================================

#[test]
fn test_proxy_construct_trap() {
    assert_eq!(
        eval(
            r#"
            function Target(x: number) {
                this.x = x;
            }
            let handler = {
                construct(target: any, args: any[], newTarget: any) {
                    let obj = new target(...args);
                    obj.x *= 2;
                    return obj;
                }
            };
            let P = new Proxy(Target, handler);
            let instance = new P(5);
            instance.x
        "#
        ),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_proxy_construct_trap_must_return_object() {
    // construct trap must return an object
    assert!(throws_error(
        r#"
            function Target() {}
            let handler = {
                construct(target: any, args: any[]) {
                    return 42; // Not an object!
                }
            };
            let P = new Proxy(Target, handler);
            new P();
        "#,
        "must return an object"
    ));
}

// =============================================================================
// getOwnPropertyDescriptor trap
// =============================================================================

#[test]
fn test_proxy_get_own_property_descriptor_trap() {
    assert_eq!(
        eval(
            r#"
            let target = { x: 1 };
            let handler = {
                getOwnPropertyDescriptor(target: any, prop: string) {
                    return {
                        value: target[prop],
                        writable: true,
                        enumerable: true,
                        configurable: true
                    };
                }
            };
            let p = new Proxy(target, handler);
            let desc = Object.getOwnPropertyDescriptor(p, 'x');
            desc.value
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_proxy_get_own_property_descriptor_virtual_property() {
    assert_eq!(
        eval(
            r#"
            let target = {};
            let handler = {
                getOwnPropertyDescriptor(target: any, prop: string) {
                    if (prop === 'virtual') {
                        return { value: 'I exist!', writable: true, enumerable: true, configurable: true };
                    }
                    return Object.getOwnPropertyDescriptor(target, prop);
                }
            };
            let p = new Proxy(target, handler);
            let desc = Object.getOwnPropertyDescriptor(p, 'virtual');
            desc.value
        "#
        ),
        JsValue::from("I exist!")
    );
}

// =============================================================================
// defineProperty trap
// =============================================================================

#[test]
fn test_proxy_define_property_trap() {
    assert_eq!(
        eval(
            r#"
            let target = {};
            let definedProps: string[] = [];
            let handler = {
                defineProperty(target: any, prop: string, descriptor: any) {
                    definedProps.push(prop);
                    Object.defineProperty(target, prop, descriptor);
                    return true;
                }
            };
            let p = new Proxy(target, handler);
            Object.defineProperty(p, 'x', { value: 1 });
            definedProps[0]
        "#
        ),
        JsValue::from("x")
    );
}

// =============================================================================
// getPrototypeOf trap
// =============================================================================

#[test]
fn test_proxy_get_prototype_of_trap() {
    assert_eq!(
        eval(
            r#"
            let proto = { protoMethod: function() { return 'from proto'; } };
            let target = Object.create(proto);
            let customProto = { custom: true };
            let handler = {
                getPrototypeOf(target: any) {
                    return customProto;
                }
            };
            let p = new Proxy(target, handler);
            Object.getPrototypeOf(p).custom
        "#
        ),
        JsValue::Boolean(true)
    );
}

// =============================================================================
// setPrototypeOf trap
// =============================================================================

#[test]
fn test_proxy_set_prototype_of_trap() {
    assert_eq!(
        eval(
            r#"
            let target = {};
            let newProto = { x: 42 };
            let protoSet: any;
            let handler = {
                setPrototypeOf(target: any, proto: any) {
                    protoSet = proto;
                    Object.setPrototypeOf(target, proto);
                    return true;
                }
            };
            let p = new Proxy(target, handler);
            Object.setPrototypeOf(p, newProto);
            protoSet.x
        "#
        ),
        JsValue::Number(42.0)
    );
}

// =============================================================================
// isExtensible trap
// =============================================================================

#[test]
fn test_proxy_is_extensible_trap() {
    assert_eq!(
        eval(
            r#"
            let target = {};
            let handler = {
                isExtensible(target: any) {
                    return true;
                }
            };
            let p = new Proxy(target, handler);
            Object.isExtensible(p)
        "#
        ),
        JsValue::Boolean(true)
    );
}

// =============================================================================
// preventExtensions trap
// =============================================================================

#[test]
fn test_proxy_prevent_extensions_trap() {
    assert_eq!(
        eval(
            r#"
            let target = {};
            let prevented = false;
            let handler = {
                preventExtensions(target: any) {
                    prevented = true;
                    Object.preventExtensions(target);
                    return true;
                }
            };
            let p = new Proxy(target, handler);
            Object.preventExtensions(p);
            prevented
        "#
        ),
        JsValue::Boolean(true)
    );
}

// =============================================================================
// ownKeys trap
// =============================================================================

#[test]
fn test_proxy_own_keys_trap() {
    assert_eq!(
        eval(
            r#"
            let target = { a: 1, b: 2, c: 3 };
            let handler = {
                ownKeys(target: any) {
                    return ['a', 'c']; // Hide 'b'
                }
            };
            let p = new Proxy(target, handler);
            Object.keys(p).join(',')
        "#
        ),
        JsValue::from("a,c")
    );
}

#[test]
fn test_proxy_own_keys_add_virtual_keys() {
    assert_eq!(
        eval(
            r#"
            let target = { real: 1 };
            let handler = {
                ownKeys(target: any) {
                    return ['real', 'virtual1', 'virtual2'];
                },
                getOwnPropertyDescriptor(target: any, prop: string) {
                    if (prop.startsWith('virtual')) {
                        return { value: prop, enumerable: true, configurable: true };
                    }
                    return Object.getOwnPropertyDescriptor(target, prop);
                }
            };
            let p = new Proxy(target, handler);
            Object.keys(p).length
        "#
        ),
        JsValue::Number(3.0)
    );
}

// =============================================================================
// Proxy.revocable()
// =============================================================================

#[test]
fn test_proxy_revocable() {
    assert_eq!(
        eval(
            r#"
            let target = { x: 1 };
            let { proxy, revoke } = Proxy.revocable(target, {});
            proxy.x
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_proxy_revocable_after_revoke() {
    // After revoke(), any operation on proxy should throw
    assert!(throws_error(
        r#"
            let target = { x: 1 };
            let { proxy, revoke } = Proxy.revocable(target, {});
            revoke();
            proxy.x; // Should throw
        "#,
        "revoked"
    ));
}

#[test]
fn test_proxy_revocable_revoke_multiple_times() {
    // Calling revoke() multiple times is a no-op
    assert_eq!(
        eval(
            r#"
            let target = { x: 1 };
            let { proxy, revoke } = Proxy.revocable(target, {});
            revoke();
            revoke();
            revoke();
            'no error'
        "#
        ),
        JsValue::from("no error")
    );
}

// =============================================================================
// Reflect object
// =============================================================================

#[test]
fn test_reflect_get() {
    assert_eq!(
        eval("let obj = { x: 42 }; Reflect.get(obj, 'x')"),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_reflect_get_with_receiver() {
    assert_eq!(
        eval(
            r#"
            let obj = {
                get prop() { return this.value; }
            };
            let receiver = { value: 100 };
            Reflect.get(obj, 'prop', receiver)
        "#
        ),
        JsValue::Number(100.0)
    );
}

#[test]
fn test_reflect_set() {
    assert_eq!(
        eval(
            r#"
            let obj = { x: 1 };
            Reflect.set(obj, 'x', 42);
            obj.x
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_reflect_set_returns_boolean() {
    assert_eq!(
        eval("let obj = {}; Reflect.set(obj, 'x', 1)"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_reflect_has() {
    assert_eq!(
        eval("let obj = { x: 1 }; Reflect.has(obj, 'x')"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("let obj = { x: 1 }; Reflect.has(obj, 'y')"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_reflect_delete_property() {
    assert_eq!(
        eval(
            r#"
            let obj = { x: 1 };
            let result = Reflect.deleteProperty(obj, 'x');
            result && !('x' in obj)
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_reflect_apply() {
    assert_eq!(
        eval(
            r#"
            function greet(greeting: string) {
                return greeting + ', ' + this.name;
            }
            Reflect.apply(greet, { name: 'World' }, ['Hello'])
        "#
        ),
        JsValue::from("Hello, World")
    );
}

#[test]
fn test_reflect_construct() {
    assert_eq!(
        eval(
            r#"
            function Person(name: string) {
                this.name = name;
            }
            let p = Reflect.construct(Person, ['Alice']);
            p.name
        "#
        ),
        JsValue::from("Alice")
    );
}

#[test]
fn test_reflect_construct_with_new_target() {
    assert_eq!(
        eval(
            r#"
            function Base() {
                this.type = 'base';
            }
            function Derived() {
                this.type = 'derived';
            }
            let obj = Reflect.construct(Base, [], Derived);
            obj instanceof Derived
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_reflect_get_own_property_descriptor() {
    assert_eq!(
        eval(
            r#"
            let obj = { x: 42 };
            let desc = Reflect.getOwnPropertyDescriptor(obj, 'x');
            desc.value
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_reflect_define_property() {
    assert_eq!(
        eval(
            r#"
            let obj = {};
            Reflect.defineProperty(obj, 'x', { value: 42, writable: true });
            obj.x
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_reflect_get_prototype_of() {
    assert_eq!(
        eval(
            r#"
            let proto = { x: 1 };
            let obj = Object.create(proto);
            Reflect.getPrototypeOf(obj) === proto
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_reflect_set_prototype_of() {
    assert_eq!(
        eval(
            r#"
            let obj = {};
            let proto = { inherited: true };
            Reflect.setPrototypeOf(obj, proto);
            obj.inherited
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_reflect_is_extensible() {
    assert_eq!(
        eval("let obj = {}; Reflect.isExtensible(obj)"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("let obj = Object.freeze({}); Reflect.isExtensible(obj)"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_reflect_prevent_extensions() {
    assert_eq!(
        eval(
            r#"
            let obj = {};
            Reflect.preventExtensions(obj);
            Reflect.isExtensible(obj)
        "#
        ),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_reflect_own_keys() {
    assert_eq!(
        eval(
            r#"
            let obj = { a: 1, b: 2 };
            Reflect.ownKeys(obj).join(',')
        "#
        ),
        JsValue::from("a,b")
    );
}

#[test]
fn test_reflect_own_keys_includes_symbols() {
    assert_eq!(
        eval(
            r#"
            let sym = Symbol('test');
            let obj = { a: 1, [sym]: 2 };
            Reflect.ownKeys(obj).length
        "#
        ),
        JsValue::Number(2.0)
    );
}

// =============================================================================
// Complex Proxy Patterns
// =============================================================================

#[test]
fn test_proxy_logging() {
    // Common use case: logging proxy
    assert_eq!(
        eval(
            r#"
            let log: string[] = [];
            let target = { x: 1, y: 2 };
            let handler = {
                get(target: any, prop: string) {
                    log.push('get:' + prop);
                    return target[prop];
                },
                set(target: any, prop: string, value: any) {
                    log.push('set:' + prop + '=' + value);
                    target[prop] = value;
                    return true;
                }
            };
            let p = new Proxy(target, handler);
            p.x;
            p.y = 10;
            log.join(',')
        "#
        ),
        JsValue::from("get:x,set:y=10")
    );
}

#[test]
fn test_proxy_validation() {
    // Validation proxy
    assert!(throws_error(
        r#"
            let target = { age: 25 };
            let handler = {
                set(target: any, prop: string, value: any) {
                    if (prop === 'age' && (typeof value !== 'number' || value < 0)) {
                        throw new Error('Invalid age');
                    }
                    target[prop] = value;
                    return true;
                }
            };
            let p = new Proxy(target, handler);
            p.age = -5; // Should throw
        "#,
        "Invalid age"
    ));
}

#[test]
fn test_proxy_default_values() {
    // Default value proxy
    assert_eq!(
        eval(
            r#"
            let target = { x: 1 };
            let handler = {
                get(target: any, prop: string) {
                    return prop in target ? target[prop] : 'default';
                }
            };
            let p = new Proxy(target, handler);
            p.missing
        "#
        ),
        JsValue::from("default")
    );
}

#[test]
fn test_proxy_negative_array_indices() {
    // Negative array index support
    assert_eq!(
        eval(
            r#"
            let arr = [1, 2, 3, 4, 5];
            let handler = {
                get(target: any[], prop: string) {
                    let index = Number(prop);
                    if (!isNaN(index) && index < 0) {
                        return target[target.length + index];
                    }
                    return target[prop];
                }
            };
            let p = new Proxy(arr, handler);
            p[-1]
        "#
        ),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_proxy_private_properties() {
    // Hide properties starting with underscore
    assert_eq!(
        eval(
            r#"
            let target = { _private: 'secret', visible: 'shown' };
            let handler = {
                get(target: any, prop: string) {
                    if (prop.startsWith('_')) return undefined;
                    return target[prop];
                },
                has(target: any, prop: string) {
                    if (prop.startsWith('_')) return false;
                    return prop in target;
                },
                ownKeys(target: any) {
                    return Object.keys(target).filter(k => !k.startsWith('_'));
                }
            };
            let p = new Proxy(target, handler);
            // Note: undefined in array.join becomes empty string (JS behavior)
            [p._private === undefined, '_private' in p, Object.keys(p).length].join(',')
        "#
        ),
        JsValue::from("true,false,1")
    );
}

#[test]
fn test_proxy_observable() {
    // Observable pattern
    assert_eq!(
        eval(
            r#"
            let changes: string[] = [];
            function createObservable(target: any) {
                return new Proxy(target, {
                    set(target: any, prop: string, value: any) {
                        let old = target[prop];
                        target[prop] = value;
                        changes.push(prop + ':' + old + '->' + value);
                        return true;
                    }
                });
            }
            let obj = createObservable({ x: 1 });
            obj.x = 2;
            obj.x = 3;
            changes.join(';')
        "#
        ),
        JsValue::from("x:1->2;x:2->3")
    );
}

#[test]
fn test_proxy_readonly() {
    // Read-only proxy
    assert!(throws_error(
        r#"
            let target = { x: 1 };
            let handler = {
                set(target: any, prop: string, value: any) {
                    throw new Error('Cannot modify read-only object');
                },
                deleteProperty(target: any, prop: string) {
                    throw new Error('Cannot delete from read-only object');
                }
            };
            let p = new Proxy(target, handler);
            p.x = 2;
        "#,
        "Cannot modify read-only object"
    ));
}

#[test]
fn test_nested_proxies() {
    // Proxy wrapping another proxy
    assert_eq!(
        eval(
            r#"
            let target = { x: 1 };
            let p1 = new Proxy(target, {
                get(t: any, p: string) { return t[p] * 2; }
            });
            let p2 = new Proxy(p1, {
                get(t: any, p: string) { return t[p] + 10; }
            });
            p2.x
        "#
        ),
        JsValue::Number(12.0) // (1 * 2) + 10
    );
}

#[test]
fn test_proxy_with_class_instance() {
    // Proxy wrapping an object with method
    assert_eq!(
        eval(
            r#"
            let obj = {
                count: 0,
                increment() { this.count++; }
            };
            let p = new Proxy(obj, {});
            p.increment();
            p.increment();
            obj.count
        "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_proxy_increment_through_target() {
    // Verify increment through proxy updates target
    assert_eq!(
        eval(
            r#"
            let target = { count: 10 };
            let p = new Proxy(target, {});
            p.count++;
            p.count++;
            target.count
        "#
        ),
        JsValue::Number(12.0)
    );
}

#[test]
fn test_proxy_decrement_through_target() {
    // Verify decrement through proxy updates target
    assert_eq!(
        eval(
            r#"
            let target = { count: 10 };
            let p = new Proxy(target, {});
            --p.count;
            --p.count;
            target.count
        "#
        ),
        JsValue::Number(8.0)
    );
}

// =============================================================================
// Edge Cases and Error Conditions
// =============================================================================

#[test]
fn test_proxy_trap_throws() {
    // Trap throwing an error should propagate
    let source = r#"
            let handler = {
                get() { throw new Error('trap error'); }
            };
            let p = new Proxy({}, handler);
            p.x;
        "#;
    let result = eval_result(source);
    eprintln!("Result: {:?}", result);
    assert!(throws_error(source, "trap error"));
}

#[test]
fn test_proxy_null_target() {
    assert!(throws_error(
        "new Proxy(null, {})",
        "Cannot create proxy with a non-object"
    ));
}

#[test]
fn test_proxy_undefined_target() {
    assert!(throws_error(
        "new Proxy(undefined, {})",
        "Cannot create proxy with a non-object"
    ));
}

#[test]
fn test_proxy_without_new() {
    assert!(throws_error(
        "Proxy({}, {})",
        "Constructor Proxy requires 'new'"
    ));
}

#[test]
fn test_proxy_handler_undefined_trap() {
    // Handler with undefined trap should fall through to target
    assert_eq!(
        eval(
            r#"
            let handler = { get: undefined };
            let p = new Proxy({ x: 42 }, handler);
            p.x
        "#
        ),
        JsValue::Number(42.0)
    );
}

// =============================================================================
// Proxy with for...in and for...of
// =============================================================================

#[test]
fn test_proxy_for_in() {
    assert_eq!(
        eval(
            r#"
            let target = { a: 1, b: 2, c: 3 };
            let handler = {
                ownKeys(target: any) {
                    return ['a', 'b']; // Hide 'c'
                },
                getOwnPropertyDescriptor(target: any, prop: string) {
                    return { enumerable: true, configurable: true, value: target[prop] };
                }
            };
            let p = new Proxy(target, handler);
            let keys: string[] = [];
            for (let k in p) keys.push(k);
            keys.join(',')
        "#
        ),
        JsValue::from("a,b")
    );
}

#[test]
fn test_proxy_array_for_of() {
    assert_eq!(
        eval(
            r#"
            let arr = [10, 20, 30];
            let handler = {
                get(target: any, prop: string) {
                    let value = target[prop];
                    if (typeof value === 'number') return value * 2;
                    return value;
                }
            };
            let p = new Proxy(arr, handler);
            let result: number[] = [];
            for (let x of p) result.push(x);
            result.join(',')
        "#
        ),
        JsValue::from("20,40,60")
    );
}

// =============================================================================
// Invariant Enforcement (Optional - depends on implementation)
// =============================================================================

#[test]
fn test_proxy_invariant_get_non_configurable() {
    // For non-configurable, non-writable properties, get must return same value
    // This is an optional invariant check
    assert_eq!(
        eval(
            r#"
            let target = {};
            Object.defineProperty(target, 'x', { value: 1, writable: false, configurable: false });
            let handler = {
                get(target: any, prop: string) {
                    return target[prop]; // Must return same value
                }
            };
            let p = new Proxy(target, handler);
            p.x
        "#
        ),
        JsValue::Number(1.0)
    );
}

// =============================================================================
// Reflect and Proxy Integration
// =============================================================================

#[test]
fn test_reflect_in_proxy_handler() {
    // Using Reflect in proxy handlers is idiomatic
    assert_eq!(
        eval(
            r#"
            let target = { x: 1, y: 2 };
            let handler = {
                get(target: any, prop: string, receiver: any) {
                    let value = Reflect.get(target, prop, receiver);
                    return typeof value === 'number' ? value * 10 : value;
                },
                set(target: any, prop: string, value: any, receiver: any) {
                    return Reflect.set(target, prop, value, receiver);
                }
            };
            let p = new Proxy(target, handler);
            p.x + p.y
        "#
        ),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_reflect_forwarding() {
    // Forward all operations using Reflect
    assert_eq!(
        eval(
            r#"
            let target = { a: 1 };
            let handler = {
                get: Reflect.get,
                set: Reflect.set,
                has: Reflect.has,
                deleteProperty: Reflect.deleteProperty
            };
            let p = new Proxy(target, handler);
            p.b = 2;
            p.a + p.b
        "#
        ),
        JsValue::Number(3.0)
    );
}
