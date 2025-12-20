//! Tests for bytecode VM execution
//!
//! These tests verify that the bytecode VM produces the same results
//! as the stack-based interpreter.

use typescript_eval::{Interpreter, JsValue};

/// Helper to evaluate using bytecode VM
fn eval_bytecode(source: &str) -> JsValue {
    let mut interp = Interpreter::new();
    interp.eval_bytecode(source).expect("bytecode eval failed")
}

// ═══════════════════════════════════════════════════════════════════════════
// Basic Literals
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_number_literal() {
    assert_eq!(eval_bytecode("42"), JsValue::Number(42.0));
    assert_eq!(eval_bytecode("3.14"), JsValue::Number(3.14));
    assert_eq!(eval_bytecode("-17"), JsValue::Number(-17.0));
}

#[test]
fn test_bytecode_string_literal() {
    assert_eq!(eval_bytecode("'hello'"), JsValue::String("hello".into()));
    assert_eq!(eval_bytecode("\"world\""), JsValue::String("world".into()));
}

#[test]
fn test_bytecode_boolean_literal() {
    assert_eq!(eval_bytecode("true"), JsValue::Boolean(true));
    assert_eq!(eval_bytecode("false"), JsValue::Boolean(false));
}

#[test]
fn test_bytecode_null_undefined() {
    assert_eq!(eval_bytecode("null"), JsValue::Null);
    assert_eq!(eval_bytecode("void 0"), JsValue::Undefined);
}

// ═══════════════════════════════════════════════════════════════════════════
// Arithmetic Operations
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_addition() {
    assert_eq!(eval_bytecode("1 + 2"), JsValue::Number(3.0));
    assert_eq!(eval_bytecode("10 + 20 + 30"), JsValue::Number(60.0));
}

#[test]
fn test_bytecode_subtraction() {
    assert_eq!(eval_bytecode("10 - 3"), JsValue::Number(7.0));
    assert_eq!(eval_bytecode("100 - 50 - 25"), JsValue::Number(25.0));
}

#[test]
fn test_bytecode_multiplication() {
    assert_eq!(eval_bytecode("3 * 4"), JsValue::Number(12.0));
    assert_eq!(eval_bytecode("2 * 3 * 5"), JsValue::Number(30.0));
}

#[test]
fn test_bytecode_division() {
    assert_eq!(eval_bytecode("10 / 2"), JsValue::Number(5.0));
    assert_eq!(eval_bytecode("100 / 10 / 2"), JsValue::Number(5.0));
}

#[test]
fn test_bytecode_modulo() {
    assert_eq!(eval_bytecode("10 % 3"), JsValue::Number(1.0));
    assert_eq!(eval_bytecode("17 % 5"), JsValue::Number(2.0));
}

#[test]
fn test_bytecode_exponentiation() {
    assert_eq!(eval_bytecode("2 ** 3"), JsValue::Number(8.0));
    assert_eq!(eval_bytecode("2 ** 10"), JsValue::Number(1024.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Comparison Operations
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_equality() {
    assert_eq!(eval_bytecode("1 === 1"), JsValue::Boolean(true));
    assert_eq!(eval_bytecode("1 === 2"), JsValue::Boolean(false));
    assert_eq!(eval_bytecode("'a' === 'a'"), JsValue::Boolean(true));
}

#[test]
fn test_bytecode_inequality() {
    assert_eq!(eval_bytecode("1 !== 2"), JsValue::Boolean(true));
    assert_eq!(eval_bytecode("1 !== 1"), JsValue::Boolean(false));
}

#[test]
fn test_bytecode_relational() {
    assert_eq!(eval_bytecode("1 < 2"), JsValue::Boolean(true));
    assert_eq!(eval_bytecode("2 > 1"), JsValue::Boolean(true));
    assert_eq!(eval_bytecode("2 <= 2"), JsValue::Boolean(true));
    assert_eq!(eval_bytecode("3 >= 3"), JsValue::Boolean(true));
}

// ═══════════════════════════════════════════════════════════════════════════
// Unary Operations
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_unary_minus() {
    assert_eq!(eval_bytecode("-5"), JsValue::Number(-5.0));
    assert_eq!(eval_bytecode("-(3 + 2)"), JsValue::Number(-5.0));
}

#[test]
fn test_bytecode_unary_not() {
    assert_eq!(eval_bytecode("!true"), JsValue::Boolean(false));
    assert_eq!(eval_bytecode("!false"), JsValue::Boolean(true));
    assert_eq!(eval_bytecode("!!true"), JsValue::Boolean(true));
}

#[test]
fn test_bytecode_typeof() {
    assert_eq!(eval_bytecode("typeof 42"), JsValue::String("number".into()));
    assert_eq!(eval_bytecode("typeof 'hello'"), JsValue::String("string".into()));
    assert_eq!(eval_bytecode("typeof true"), JsValue::String("boolean".into()));
    assert_eq!(eval_bytecode("typeof null"), JsValue::String("object".into()));
}

// ═══════════════════════════════════════════════════════════════════════════
// Bitwise Operations
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_bitwise() {
    assert_eq!(eval_bytecode("5 & 3"), JsValue::Number(1.0));
    assert_eq!(eval_bytecode("5 | 3"), JsValue::Number(7.0));
    assert_eq!(eval_bytecode("5 ^ 3"), JsValue::Number(6.0));
    assert_eq!(eval_bytecode("~0"), JsValue::Number(-1.0));
}

#[test]
fn test_bytecode_shifts() {
    assert_eq!(eval_bytecode("8 << 2"), JsValue::Number(32.0));
    assert_eq!(eval_bytecode("32 >> 2"), JsValue::Number(8.0));
    assert_eq!(eval_bytecode("-1 >>> 0"), JsValue::Number(4294967295.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Logical Operations
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_logical_and() {
    assert_eq!(eval_bytecode("true && true"), JsValue::Boolean(true));
    assert_eq!(eval_bytecode("true && false"), JsValue::Boolean(false));
    assert_eq!(eval_bytecode("1 && 2"), JsValue::Number(2.0));
    assert_eq!(eval_bytecode("0 && 2"), JsValue::Number(0.0));
}

#[test]
fn test_bytecode_logical_or() {
    assert_eq!(eval_bytecode("true || false"), JsValue::Boolean(true));
    assert_eq!(eval_bytecode("false || true"), JsValue::Boolean(true));
    assert_eq!(eval_bytecode("1 || 2"), JsValue::Number(1.0));
    assert_eq!(eval_bytecode("0 || 2"), JsValue::Number(2.0));
}

#[test]
fn test_bytecode_nullish_coalescing() {
    assert_eq!(eval_bytecode("null ?? 'default'"), JsValue::String("default".into()));
    assert_eq!(eval_bytecode("'value' ?? 'default'"), JsValue::String("value".into()));
}

// ═══════════════════════════════════════════════════════════════════════════
// Conditional Expression
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_ternary() {
    assert_eq!(eval_bytecode("true ? 1 : 2"), JsValue::Number(1.0));
    assert_eq!(eval_bytecode("false ? 1 : 2"), JsValue::Number(2.0));
    assert_eq!(eval_bytecode("1 > 0 ? 'yes' : 'no'"), JsValue::String("yes".into()));
}
