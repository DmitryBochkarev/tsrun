//! Basic language feature tests: arithmetic, precedence, comparison, variables, conditionals

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_arithmetic() {
    assert_eq!(eval("(1 as number) + (2 as number)"), JsValue::Number(3.0));
    assert_eq!(eval("(10 as number) - (4 as number)"), JsValue::Number(6.0));
    assert_eq!(eval("(3 as number) * (4 as number)"), JsValue::Number(12.0));
    assert_eq!(eval("(15 as number) / (3 as number)"), JsValue::Number(5.0));
    assert_eq!(eval("(2 as number) ** (3 as number)"), JsValue::Number(8.0));
}

#[test]
fn test_precedence() {
    assert_eq!(eval("(1 as number) + (2 as number) * (3 as number)"), JsValue::Number(7.0));
    assert_eq!(eval("((1 as number) + (2 as number)) * (3 as number)"), JsValue::Number(9.0));
}

#[test]
fn test_comparison() {
    assert_eq!(eval("(1 as number) < (2 as number)"), JsValue::Boolean(true));
    assert_eq!(eval("(2 as number) > (1 as number)"), JsValue::Boolean(true));
    assert_eq!(eval("(1 as number) === (1 as number)"), JsValue::Boolean(true));
    assert_eq!(eval("(1 as number) !== (2 as number)"), JsValue::Boolean(true));
}

#[test]
fn test_variables() {
    assert_eq!(eval("let x: number = 5; x"), JsValue::Number(5.0));
    assert_eq!(eval("let x: number = 5; x = 10; x"), JsValue::Number(10.0));
}

#[test]
fn test_conditional() {
    assert_eq!(eval("(true as boolean) ? (1 as number) : (2 as number)"), JsValue::Number(1.0));
    assert_eq!(eval("(false as boolean) ? (1 as number) : (2 as number)"), JsValue::Number(2.0));
}