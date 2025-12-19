//! Object-related tests

use super::eval;
use typescript_eval::value::JsString;
use typescript_eval::JsValue;

#[test]
fn test_object() {
    assert_eq!(
        eval("const obj: { a: number } = { a: 1 }; obj.a"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_object_hasownproperty() {
    assert_eq!(
        eval("({a: 1} as { a: number }).hasOwnProperty('a')"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("({a: 1} as { a: number }).hasOwnProperty('b')"),
        JsValue::Boolean(false)
    );
    assert_eq!(
        eval("let o: { x: number } = {x: 1}; o.hasOwnProperty('x')"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_tostring() {
    assert_eq!(
        eval("({} as object).toString()"),
        JsValue::String(JsString::from("[object Object]"))
    );
    assert_eq!(
        eval("([1,2,3] as number[]).toString()"),
        JsValue::String(JsString::from("1,2,3"))
    );
}

#[test]
fn test_object_fromentries() {
    assert_eq!(
        eval("const entries: Array<[string, number]> = [['a', 1], ['b', 2]]; Object.fromEntries(entries).a"),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval("const entries: Array<[string, number]> = [['a', 1], ['b', 2]]; Object.fromEntries(entries).b"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_object_hasown() {
    assert_eq!(
        eval("Object.hasOwn({a: 1} as { a: number }, 'a')"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("Object.hasOwn({a: 1} as { a: number }, 'b')"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_object_create() {
    assert_eq!(
        eval("Object.create(null).hasOwnProperty"),
        JsValue::Undefined
    );
    assert_eq!(
        eval("let proto: { x: number } = {x: 1}; let o = Object.create(proto); o.x"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_object_freeze() {
    assert_eq!(
        eval("let o: { a: number } = {a: 1}; Object.freeze(o); o.a = 2; o.a"),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval("Object.isFrozen(Object.freeze({a: 1} as { a: number }))"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_seal() {
    assert_eq!(
        eval("Object.isSealed(Object.seal({a: 1} as { a: number }))"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_get_own_property_descriptor() {
    // Basic property descriptor
    assert_eq!(
        eval(
            r#"
            const obj = { x: 42 };
            const desc = Object.getOwnPropertyDescriptor(obj, 'x');
            desc.value
        "#
        ),
        JsValue::Number(42.0)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { x: 42 };
            const desc = Object.getOwnPropertyDescriptor(obj, 'x');
            desc.writable
        "#
        ),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { x: 42 };
            const desc = Object.getOwnPropertyDescriptor(obj, 'x');
            desc.enumerable
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_define_property() {
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 10, writable: true });
            obj.x
        "#
        ),
        JsValue::Number(10.0)
    );
    // Non-writable property
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 10, writable: false });
            obj.x = 20;
            obj.x
        "#
        ),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_object_get_prototype_of() {
    assert_eq!(
        eval(
            r#"
            const arr: number[] = [1, 2, 3];
            Object.getPrototypeOf(arr) === Array.prototype
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_get_own_property_names() {
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2 };
            Object.getOwnPropertyNames(obj).length
        "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_object_define_properties_basic() {
    // Define multiple properties at once
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperties(obj, {
                x: { value: 10, writable: true },
                y: { value: 20, writable: true }
            });
            obj.x + obj.y
        "#
        ),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_object_define_properties_returns_object() {
    // Should return the object
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            const result = Object.defineProperties(obj, {
                x: { value: 10 }
            });
            result === obj
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_define_properties_attributes() {
    // Test non-writable property
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperties(obj, {
                x: { value: 10, writable: false }
            });
            obj.x = 20;
            obj.x
        "#
        ),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_object_define_properties_enumerable() {
    // Test enumerable property
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperties(obj, {
                a: { value: 1, enumerable: true },
                b: { value: 2, enumerable: false }
            });
            Object.keys(obj).length
        "#
        ),
        JsValue::Number(1.0)
    );
}

// __proto__ tests
#[test]
fn test_proto_get() {
    // __proto__ should return the prototype
    assert_eq!(
        eval(
            r#"
            const parent: { x: number } = { x: 42 };
            const child: any = Object.create(parent);
            child.__proto__.x
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_proto_set() {
    // Setting __proto__ should change the prototype
    assert_eq!(
        eval(
            r#"
            const parent: { x: number } = { x: 42 };
            const child: { x?: number } = {};
            child.__proto__ = parent;
            child.x
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_proto_null() {
    // Setting __proto__ to null should work
    // After setting __proto__ to null, accessing it returns null
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            obj.__proto__ = null;
            obj.__proto__
        "#
        ),
        JsValue::Null
    );
}

#[test]
fn test_proto_in_literal() {
    // __proto__ in object literal should set prototype
    assert_eq!(
        eval(
            r#"
            const parent: { x: number } = { x: 42 };
            const child: any = { __proto__: parent, y: 1 };
            child.x
        "#
        ),
        JsValue::Number(42.0)
    );
}

// Object.keys/values/entries should return arrays with proper prototype
#[test]
fn test_object_keys_has_array_methods() {
    // Object.keys should return an array that supports array methods
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2, c: 3 };
            Object.keys(obj).map(k => k.toUpperCase()).length
        "#
        ),
        JsValue::Number(3.0)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2 };
            Object.keys(obj).filter(k => k === "a").length
        "#
        ),
        JsValue::Number(1.0)
    );
    // Test join works (order may vary)
    let result = eval(
        r#"
            const obj = { a: 1, b: 2 };
            Object.keys(obj).sort().join("-")
        "#,
    );
    assert_eq!(result, JsValue::String(JsString::from("a-b")));
}

#[test]
fn test_object_values_has_array_methods() {
    // Object.values should return an array that supports array methods
    assert_eq!(
        eval(
            r#"
            const obj = { a: 10, b: 20, c: 30 };
            Object.values(obj).reduce((sum, v) => sum + v, 0)
        "#
        ),
        JsValue::Number(60.0)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2, c: 3 };
            Object.values(obj).map(v => v * 2).length
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_object_entries_has_array_methods() {
    // Object.entries should return an array that supports array methods
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2 };
            Object.entries(obj).map(([k, v]) => k + ":" + v).length
        "#
        ),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { x: 10, y: 20 };
            Object.entries(obj).filter(([k, v]) => v > 15).length
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_object_entries_inner_arrays_have_methods() {
    // The inner [key, value] arrays should also have array methods
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1 };
            const entries = Object.entries(obj);
            entries[0].join("=")
        "#
        ),
        JsValue::String(JsString::from("a=1"))
    );
}

// Object.is tests
#[test]
fn test_object_is_basic() {
    // Same values
    assert_eq!(eval("Object.is(1, 1)"), JsValue::Boolean(true));
    assert_eq!(eval("Object.is('foo', 'foo')"), JsValue::Boolean(true));
    assert_eq!(eval("Object.is(true, true)"), JsValue::Boolean(true));
    assert_eq!(eval("Object.is(null, null)"), JsValue::Boolean(true));
    assert_eq!(
        eval("Object.is(undefined, undefined)"),
        JsValue::Boolean(true)
    );

    // Different values
    assert_eq!(eval("Object.is(1, 2)"), JsValue::Boolean(false));
    assert_eq!(eval("Object.is('foo', 'bar')"), JsValue::Boolean(false));
    assert_eq!(eval("Object.is(true, false)"), JsValue::Boolean(false));
}

#[test]
fn test_object_is_nan() {
    // Object.is(NaN, NaN) should be true (unlike === which is false)
    assert_eq!(eval("Object.is(NaN, NaN)"), JsValue::Boolean(true));
    assert_eq!(eval("NaN === NaN"), JsValue::Boolean(false));
}

#[test]
fn test_object_is_zero() {
    // Object.is distinguishes +0 and -0 (unlike === which treats them as equal)
    assert_eq!(eval("Object.is(0, 0)"), JsValue::Boolean(true));
    assert_eq!(eval("Object.is(-0, -0)"), JsValue::Boolean(true));
    assert_eq!(eval("Object.is(0, -0)"), JsValue::Boolean(false));
    assert_eq!(eval("Object.is(-0, 0)"), JsValue::Boolean(false));
    // Verify === treats them as equal
    assert_eq!(eval("0 === -0"), JsValue::Boolean(true));
}

#[test]
fn test_object_is_objects() {
    // Same object reference
    assert_eq!(
        eval("const obj = {}; Object.is(obj, obj)"),
        JsValue::Boolean(true)
    );
    // Different object references
    assert_eq!(eval("Object.is({}, {})"), JsValue::Boolean(false));
}

#[test]
fn test_object_is_symbols() {
    // Same symbol
    assert_eq!(
        eval("const s = Symbol('test'); Object.is(s, s)"),
        JsValue::Boolean(true)
    );
    // Different symbols with same description
    assert_eq!(
        eval("Object.is(Symbol('test'), Symbol('test'))"),
        JsValue::Boolean(false)
    );
}

// Object.preventExtensions/isExtensible tests
#[test]
fn test_object_prevent_extensions() {
    // After preventExtensions, new properties cannot be added
    assert_eq!(
        eval(
            r#"
            const obj: any = { a: 1 };
            Object.preventExtensions(obj);
            obj.b = 2;
            obj.b
        "#
        ),
        JsValue::Undefined
    );
}

#[test]
fn test_object_is_extensible() {
    // Objects are extensible by default
    assert_eq!(eval("Object.isExtensible({})"), JsValue::Boolean(true));
    // After preventExtensions, object is not extensible
    assert_eq!(
        eval(
            r#"
            const obj = {};
            Object.preventExtensions(obj);
            Object.isExtensible(obj)
        "#
        ),
        JsValue::Boolean(false)
    );
    // Non-objects are not extensible
    assert_eq!(eval("Object.isExtensible(1)"), JsValue::Boolean(false));
    assert_eq!(eval("Object.isExtensible('str')"), JsValue::Boolean(false));
}

#[test]
fn test_object_prevent_extensions_existing_props() {
    // Existing properties can still be modified
    assert_eq!(
        eval(
            r#"
            const obj: any = { a: 1 };
            Object.preventExtensions(obj);
            obj.a = 42;
            obj.a
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_object_freeze_prevents_extension() {
    // Object.freeze also makes object non-extensible
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1 };
            Object.freeze(obj);
            Object.isExtensible(obj)
        "#
        ),
        JsValue::Boolean(false)
    );
}

// Object.getOwnPropertyDescriptors tests
#[test]
fn test_object_get_own_property_descriptors_basic() {
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2 };
            const descs = Object.getOwnPropertyDescriptors(obj);
            descs.a.value
        "#
        ),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2 };
            const descs = Object.getOwnPropertyDescriptors(obj);
            descs.b.value
        "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_object_get_own_property_descriptors_attributes() {
    assert_eq!(
        eval(
            r#"
            const obj = { x: 42 };
            const descs = Object.getOwnPropertyDescriptors(obj);
            descs.x.writable
        "#
        ),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { x: 42 };
            const descs = Object.getOwnPropertyDescriptors(obj);
            descs.x.enumerable
        "#
        ),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { x: 42 };
            const descs = Object.getOwnPropertyDescriptors(obj);
            descs.x.configurable
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_get_own_property_descriptors_with_define() {
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 10, writable: false, enumerable: false, configurable: false });
            const descs = Object.getOwnPropertyDescriptors(obj);
            descs.x.writable
        "#
        ),
        JsValue::Boolean(false)
    );
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 10, writable: false, enumerable: false, configurable: false });
            const descs = Object.getOwnPropertyDescriptors(obj);
            descs.x.enumerable
        "#
        ),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_object_get_own_property_descriptors_count() {
    // Test that all properties are returned
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2, c: 3 };
            Object.keys(Object.getOwnPropertyDescriptors(obj)).length
        "#
        ),
        JsValue::Number(3.0)
    );
}

// =============================================================================
// Object Literal Getter/Setter Tests
// =============================================================================

#[test]
fn test_object_literal_getter_basic() {
    // Basic getter in object literal
    assert_eq!(
        eval(
            r#"
            let obj = {
                get value() { return 42; }
            };
            obj.value
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_object_literal_getter_with_this() {
    // Getter that uses this
    assert_eq!(
        eval(
            r#"
            let obj = {
                x: 10,
                get double() { return this.x * 2; }
            };
            obj.double
        "#
        ),
        JsValue::Number(20.0)
    );
}

#[test]
fn test_object_literal_setter_basic() {
    // Basic setter in object literal
    assert_eq!(
        eval(
            r#"
            let obj = {
                _value: 0,
                set value(v: number) { this._value = v; }
            };
            obj.value = 42;
            obj._value
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_object_literal_getter_setter_pair() {
    // Getter/setter pair
    assert_eq!(
        eval(
            r#"
            let obj = {
                _count: 0,
                get count() { return this._count; },
                set count(v: number) { this._count = v; }
            };
            obj.count = 5;
            obj.count
        "#
        ),
        JsValue::Number(5.0)
    );
}
