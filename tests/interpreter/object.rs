//! Object-related tests

use super::eval;
use typescript_eval::value::JsString;
use typescript_eval::JsValue;

#[test]
fn test_object() {
    assert_eq!(eval("const obj = { a: 1 }; obj.a"), JsValue::Number(1.0));
}

#[test]
fn test_object_hasownproperty() {
    assert_eq!(eval("({a: 1}).hasOwnProperty('a')"), JsValue::Boolean(true));
    assert_eq!(
        eval("({a: 1}).hasOwnProperty('b')"),
        JsValue::Boolean(false)
    );
    assert_eq!(
        eval("let o = {x: 1}; o.hasOwnProperty('x')"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_tostring() {
    assert_eq!(
        eval("({}).toString()"),
        JsValue::String(JsString::from("[object Object]"))
    );
    assert_eq!(
        eval("[1,2,3].toString()"),
        JsValue::String(JsString::from("1,2,3"))
    );
}

#[test]
fn test_object_fromentries() {
    assert_eq!(
        eval("Object.fromEntries([['a', 1], ['b', 2]]).a"),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval("Object.fromEntries([['a', 1], ['b', 2]]).b"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_object_hasown() {
    assert_eq!(
        eval("Object.hasOwn({a: 1}, 'a')"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("Object.hasOwn({a: 1}, 'b')"),
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
        eval("let proto = {x: 1}; let o = Object.create(proto); o.x"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_object_freeze() {
    assert_eq!(
        eval("let o = {a: 1}; Object.freeze(o); o.a = 2; o.a"),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval("Object.isFrozen(Object.freeze({a: 1}))"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_seal() {
    assert_eq!(
        eval("Object.isSealed(Object.seal({a: 1}))"),
        JsValue::Boolean(true)
    );
}