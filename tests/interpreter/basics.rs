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

// Bitwise operators
#[test]
fn test_bitwise_shift() {
    // Left shift
    assert_eq!(eval("(8 as number) << (2 as number)"), JsValue::Number(32.0));
    // Right shift (signed)
    assert_eq!(eval("(32 as number) >> (2 as number)"), JsValue::Number(8.0));
    // Right shift preserves sign for negative numbers
    assert_eq!(eval("((-8 as number) >> (2 as number))"), JsValue::Number(-2.0));
}

#[test]
fn test_unsigned_right_shift() {
    // Unsigned right shift (>>>)
    assert_eq!(eval("(32 as number) >>> (2 as number)"), JsValue::Number(8.0));
    // Unsigned right shift converts to unsigned 32-bit first
    assert_eq!(eval("((-1 as number) >>> (0 as number))"), JsValue::Number(4294967295.0));
    // Unsigned right shift on negative numbers
    assert_eq!(eval("((-8 as number) >>> (2 as number))"), JsValue::Number(1073741822.0));
}

#[test]
fn test_unsigned_right_shift_assignment() {
    assert_eq!(
        eval("let x: number = 32; x >>>= 2; x"),
        JsValue::Number(8.0)
    );
    assert_eq!(
        eval("let x: number = -1; x >>>= 0; x"),
        JsValue::Number(4294967295.0)
    );
}

// BigInt literals (parsed and converted to Number for now)
#[test]
fn test_bigint_literal() {
    // BigInt literals are currently converted to Number
    assert_eq!(eval("123n"), JsValue::Number(123.0));
    assert_eq!(eval("0n"), JsValue::Number(0.0));
}

#[test]
fn test_bigint_arithmetic() {
    // BigInt arithmetic works as Number arithmetic for now
    assert_eq!(eval("(100n as number) + (200n as number)"), JsValue::Number(300.0));
}

#[test]
fn test_bigint_variable() {
    assert_eq!(
        eval("const n: bigint = 42n; n"),
        JsValue::Number(42.0)
    );
}