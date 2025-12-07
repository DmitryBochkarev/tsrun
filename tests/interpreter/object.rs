//! Object-related tests

use super::eval;
use typescript_eval::value::JsString;
use typescript_eval::JsValue;

#[test]
fn test_object() {
    assert_eq!(eval("const obj: { a: number } = { a: 1 }; obj.a"), JsValue::Number(1.0));
}

#[test]
fn test_object_hasownproperty() {
    assert_eq!(eval("({a: 1} as { a: number }).hasOwnProperty('a')"), JsValue::Boolean(true));
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
        eval(r#"
            const obj = { x: 42 };
            const desc = Object.getOwnPropertyDescriptor(obj, 'x');
            desc.value
        "#),
        JsValue::Number(42.0)
    );
    assert_eq!(
        eval(r#"
            const obj = { x: 42 };
            const desc = Object.getOwnPropertyDescriptor(obj, 'x');
            desc.writable
        "#),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval(r#"
            const obj = { x: 42 };
            const desc = Object.getOwnPropertyDescriptor(obj, 'x');
            desc.enumerable
        "#),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_define_property() {
    assert_eq!(
        eval(r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 10, writable: true });
            obj.x
        "#),
        JsValue::Number(10.0)
    );
    // Non-writable property
    assert_eq!(
        eval(r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 10, writable: false });
            obj.x = 20;
            obj.x
        "#),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_object_get_prototype_of() {
    assert_eq!(
        eval(r#"
            const arr: number[] = [1, 2, 3];
            Object.getPrototypeOf(arr) === Array.prototype
        "#),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_get_own_property_names() {
    assert_eq!(
        eval(r#"
            const obj = { a: 1, b: 2 };
            Object.getOwnPropertyNames(obj).length
        "#),
        JsValue::Number(2.0)
    );
}