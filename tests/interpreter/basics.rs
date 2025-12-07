//! Basic language feature tests: arithmetic, precedence, comparison, variables, conditionals

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_arithmetic() {
    assert_eq!(eval("1 + 2"), JsValue::Number(3.0));
    assert_eq!(eval("10 - 4"), JsValue::Number(6.0));
    assert_eq!(eval("3 * 4"), JsValue::Number(12.0));
    assert_eq!(eval("15 / 3"), JsValue::Number(5.0));
    assert_eq!(eval("2 ** 3"), JsValue::Number(8.0));
}

#[test]
fn test_precedence() {
    assert_eq!(eval("1 + 2 * 3"), JsValue::Number(7.0));
    assert_eq!(eval("(1 + 2) * 3"), JsValue::Number(9.0));
}

#[test]
fn test_comparison() {
    assert_eq!(eval("1 < 2"), JsValue::Boolean(true));
    assert_eq!(eval("2 > 1"), JsValue::Boolean(true));
    assert_eq!(eval("1 === 1"), JsValue::Boolean(true));
    assert_eq!(eval("1 !== 2"), JsValue::Boolean(true));
}

#[test]
fn test_variables() {
    assert_eq!(eval("let x = 5; x"), JsValue::Number(5.0));
    assert_eq!(eval("let x = 5; x = 10; x"), JsValue::Number(10.0));
}

#[test]
fn test_conditional() {
    assert_eq!(eval("true ? 1 : 2"), JsValue::Number(1.0));
    assert_eq!(eval("false ? 1 : 2"), JsValue::Number(2.0));
}