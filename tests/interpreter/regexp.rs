//! RegExp-related tests

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_regexp_test_basic() {
    assert_eq!(eval("new RegExp('abc').test('abc')"), JsValue::Boolean(true));
    assert_eq!(
        eval("new RegExp('abc').test('def')"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_regexp_test_pattern() {
    assert_eq!(eval("new RegExp('a.c').test('abc')"), JsValue::Boolean(true));
    assert_eq!(eval("new RegExp('a.c').test('adc')"), JsValue::Boolean(true));
}

#[test]
fn test_regexp_case_insensitive() {
    assert_eq!(
        eval("new RegExp('abc', 'i').test('ABC')"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_regexp_source() {
    assert_eq!(eval("new RegExp('abc', 'gi').source"), JsValue::from("abc"));
}

#[test]
fn test_regexp_flags() {
    assert_eq!(eval("new RegExp('abc', 'gi').flags"), JsValue::from("gi"));
}

#[test]
fn test_regexp_exec_match() {
    assert_eq!(
        eval("new RegExp('a(b)c').exec('abc')[0]"),
        JsValue::from("abc")
    );
    assert_eq!(
        eval("new RegExp('a(b)c').exec('abc')[1]"),
        JsValue::from("b")
    );
}

#[test]
fn test_regexp_exec_no_match() {
    assert_eq!(eval("new RegExp('xyz').exec('abc')"), JsValue::Null);
}