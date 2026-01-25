//! Tests for bytecode VM execution
//!
//! These tests verify that the bytecode VM produces the same results
//! as the stack-based interpreter.

use super::{create_test_runtime, run};
use tsrun::{JsValue, StepResult};

/// Helper to evaluate using bytecode VM
#[allow(clippy::expect_used, clippy::panic)]
fn eval_bytecode(source: &str) -> JsValue {
    let mut interp = create_test_runtime();
    let result = run(&mut interp, source, None).expect("eval failed");
    match result {
        StepResult::Complete(rv) => rv.value().clone(),
        _ => panic!("Expected Complete result"),
    }
}

/// Helper to evaluate and return error message if any
#[allow(clippy::expect_used, clippy::panic)]
fn eval_bytecode_result(source: &str) -> Result<JsValue, String> {
    let mut interp = create_test_runtime();
    match run(&mut interp, source, None) {
        Ok(StepResult::Complete(rv)) => Ok(rv.value().clone()),
        Ok(other) => Err(format!("Unexpected result: {:?}", other)),
        Err(e) => Err(format!("{:?}", e)),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Basic Literals
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_number_literal() {
    assert_eq!(eval_bytecode("42"), JsValue::Number(42.0));
    assert_eq!(eval_bytecode("3.15"), JsValue::Number(3.15));
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
    assert_eq!(
        eval_bytecode("typeof 'hello'"),
        JsValue::String("string".into())
    );
    assert_eq!(
        eval_bytecode("typeof true"),
        JsValue::String("boolean".into())
    );
    assert_eq!(
        eval_bytecode("typeof null"),
        JsValue::String("object".into())
    );
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
    assert_eq!(
        eval_bytecode("null ?? 'default'"),
        JsValue::String("default".into())
    );
    assert_eq!(
        eval_bytecode("'value' ?? 'default'"),
        JsValue::String("value".into())
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Conditional Expression
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_ternary() {
    assert_eq!(eval_bytecode("true ? 1 : 2"), JsValue::Number(1.0));
    assert_eq!(eval_bytecode("false ? 1 : 2"), JsValue::Number(2.0));
    assert_eq!(
        eval_bytecode("1 > 0 ? 'yes' : 'no'"),
        JsValue::String("yes".into())
    );
}

#[test]
fn test_bytecode_ternary_with_member_access() {
    // Ternary with member access in parentheses - from data-pipeline example
    assert_eq!(
        eval_bytecode(
            r#"
            const product = { price: 10 };
            const quantity = 2;
            const result = (product ? product.price * quantity : 0);
            result
            "#
        ),
        JsValue::Number(20.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Variable Declarations and Access
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_let_declaration() {
    assert_eq!(
        eval_bytecode("let x: number = 42; x"),
        JsValue::Number(42.0)
    );
    assert_eq!(
        eval_bytecode("let a: number = 1; let b: number = 2; a + b"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_bytecode_const_declaration() {
    assert_eq!(
        eval_bytecode("const x: number = 10; x"),
        JsValue::Number(10.0)
    );
    assert_eq!(
        eval_bytecode("const s: string = 'hello'; s"),
        JsValue::String("hello".into())
    );
}

#[test]
fn test_bytecode_var_declaration() {
    assert_eq!(eval_bytecode("var x: number = 5; x"), JsValue::Number(5.0));
}

#[test]
fn test_bytecode_variable_assignment() {
    assert_eq!(
        eval_bytecode("let x: number = 1; x = 42; x"),
        JsValue::Number(42.0)
    );
    assert_eq!(
        eval_bytecode("let a: number = 5; a = a + 10; a"),
        JsValue::Number(15.0)
    );
}

#[test]
fn test_bytecode_multiple_variables() {
    assert_eq!(
        eval_bytecode("let x: number = 1; let y: number = 2; let z: number = x + y; z"),
        JsValue::Number(3.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Object Literals and Member Access
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_object_literal() {
    // Simple object creation
    let result = eval_bytecode("({ a: 1, b: 2 })");
    assert!(matches!(result, JsValue::Object(_)));
}

#[test]
fn test_bytecode_object_property_access() {
    assert_eq!(
        eval_bytecode("let obj = { x: 42 }; obj.x"),
        JsValue::Number(42.0)
    );
    assert_eq!(
        eval_bytecode("let obj = { name: 'test' }; obj.name"),
        JsValue::String("test".into())
    );
}

#[test]
fn test_bytecode_computed_property_access() {
    assert_eq!(
        eval_bytecode("let obj = { a: 1, b: 2 }; obj['a']"),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval_bytecode("let obj = { a: 1 }; let key: string = 'a'; obj[key]"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_bytecode_property_assignment() {
    assert_eq!(
        eval_bytecode("let obj: { x?: number } = {}; obj.x = 42; obj.x"),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_bytecode_nested_property_access() {
    assert_eq!(
        eval_bytecode("let obj = { inner: { value: 100 } }; obj.inner.value"),
        JsValue::Number(100.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Array Literals and Access
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_array_literal() {
    let result = eval_bytecode("[1, 2, 3]");
    assert!(matches!(result, JsValue::Object(_)));
}

#[test]
fn test_bytecode_array_element_access() {
    assert_eq!(
        eval_bytecode("let arr: number[] = [10, 20, 30]; arr[0]"),
        JsValue::Number(10.0)
    );
    assert_eq!(
        eval_bytecode("let arr: number[] = [10, 20, 30]; arr[2]"),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_bytecode_array_length() {
    assert_eq!(
        eval_bytecode("let arr: number[] = [1, 2, 3, 4, 5]; arr.length"),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_bytecode_array_element_assignment() {
    assert_eq!(
        eval_bytecode("let arr: number[] = [1, 2, 3]; arr[1] = 99; arr[1]"),
        JsValue::Number(99.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// String Concatenation
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_string_concat() {
    assert_eq!(
        eval_bytecode("'hello' + ' ' + 'world'"),
        JsValue::String("hello world".into())
    );
    assert_eq!(eval_bytecode("'x' + 1"), JsValue::String("x1".into()));
    assert_eq!(eval_bytecode("1 + 'x'"), JsValue::String("1x".into()));
}

// ═══════════════════════════════════════════════════════════════════════════
// Control Flow - If/Else
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_if_true() {
    assert_eq!(
        eval_bytecode("let x: number = 0; if (true) { x = 1; } x"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_bytecode_if_false() {
    assert_eq!(
        eval_bytecode("let x: number = 0; if (false) { x = 1; } x"),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_bytecode_if_else() {
    assert_eq!(
        eval_bytecode("let x: number = 0; if (true) { x = 1; } else { x = 2; } x"),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval_bytecode("let x: number = 0; if (false) { x = 1; } else { x = 2; } x"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_bytecode_if_else_if() {
    assert_eq!(
        eval_bytecode("let x: number = 0; if (false) { x = 1; } else if (true) { x = 2; } x"),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval_bytecode(
            "let x: number = 0; if (false) { x = 1; } else if (false) { x = 2; } else { x = 3; } x"
        ),
        JsValue::Number(3.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Control Flow - While Loop
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_while_loop() {
    assert_eq!(
        eval_bytecode("let i: number = 0; while (i < 5) { i = i + 1; } i"),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_bytecode_while_false() {
    assert_eq!(
        eval_bytecode("let i: number = 0; while (false) { i = i + 1; } i"),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_bytecode_while_sum() {
    assert_eq!(
        eval_bytecode(
            "let sum: number = 0; let i: number = 1; while (i <= 10) { sum = sum + i; i = i + 1; } sum"
        ),
        JsValue::Number(55.0) // 1+2+3+4+5+6+7+8+9+10 = 55
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Control Flow - Do/While Loop
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_do_while() {
    assert_eq!(
        eval_bytecode("let i: number = 0; do { i = i + 1; } while (i < 3); i"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_bytecode_do_while_once() {
    // Do-while always runs at least once
    assert_eq!(
        eval_bytecode("let i: number = 0; do { i = i + 1; } while (false); i"),
        JsValue::Number(1.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Control Flow - For Loop
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_for_loop() {
    assert_eq!(
        eval_bytecode(
            "let sum: number = 0; for (let i: number = 0; i < 5; i = i + 1) { sum = sum + i; } sum"
        ),
        JsValue::Number(10.0) // 0+1+2+3+4 = 10
    );
}

#[test]
fn test_bytecode_for_no_init() {
    assert_eq!(
        eval_bytecode("let i: number = 0; for (; i < 3; i = i + 1) {} i"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_bytecode_for_no_update() {
    assert_eq!(
        eval_bytecode("let i: number = 0; for (; i < 3;) { i = i + 1; } i"),
        JsValue::Number(3.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Control Flow - Break/Continue
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_break() {
    assert_eq!(
        eval_bytecode("let i: number = 0; while (true) { i = i + 1; if (i >= 5) break; } i"),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_bytecode_continue() {
    // Sum only even numbers up to 10
    assert_eq!(
        eval_bytecode(
            "let sum: number = 0; for (let i: number = 0; i < 10; i = i + 1) { if (i % 2 !== 0) continue; sum = sum + i; } sum"
        ),
        JsValue::Number(20.0) // 0+2+4+6+8 = 20
    );
}

#[test]
fn test_bytecode_for_break() {
    assert_eq!(
        eval_bytecode(
            "let sum: number = 0; for (let i: number = 0; i < 100; i = i + 1) { if (i >= 5) break; sum = sum + i; } sum"
        ),
        JsValue::Number(10.0) // 0+1+2+3+4 = 10
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Control Flow - Switch Statement
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_switch_match() {
    assert_eq!(
        eval_bytecode(
            "let x: number = 2; let result: string = ''; switch (x) { case 1: result = 'one'; break; case 2: result = 'two'; break; case 3: result = 'three'; break; } result"
        ),
        JsValue::String("two".into())
    );
}

#[test]
fn test_bytecode_switch_default() {
    assert_eq!(
        eval_bytecode(
            "let x: number = 99; let result: string = ''; switch (x) { case 1: result = 'one'; break; default: result = 'other'; break; } result"
        ),
        JsValue::String("other".into())
    );
}

#[test]
fn test_bytecode_switch_fallthrough() {
    assert_eq!(
        eval_bytecode(
            "let x: number = 1; let result: number = 0; switch (x) { case 1: result = result + 1; case 2: result = result + 2; break; case 3: result = result + 3; break; } result"
        ),
        JsValue::Number(3.0) // Falls through from case 1 to case 2
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Exception Handling - Try/Catch/Finally
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_try_no_error() {
    assert_eq!(
        eval_bytecode(
            "let result: number = 0; try { result = 1; } catch (e) { result = 2; } result"
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_bytecode_try_catch() {
    assert_eq!(
        eval_bytecode(
            "let result: string = ''; try { throw 'error'; result = 'try'; } catch (e) { result = 'catch'; } result"
        ),
        JsValue::String("catch".into())
    );
}

#[test]
fn test_bytecode_throw_and_catch() {
    assert_eq!(
        eval_bytecode(
            "let result: string = ''; try { throw 'test error'; } catch (e) { result = e as string; } result"
        ),
        JsValue::String("test error".into())
    );
}

#[test]
fn test_bytecode_try_finally() {
    assert_eq!(
        eval_bytecode(
            "let result: number = 0; try { result = 1; } finally { result = result + 10; } result"
        ),
        JsValue::Number(11.0)
    );
}

#[test]
fn test_bytecode_try_catch_finally() {
    assert_eq!(
        eval_bytecode(
            "let result: number = 0; try { throw 'error'; } catch (e) { result = 1; } finally { result = result + 10; } result"
        ),
        JsValue::Number(11.0)
    );
}

#[test]
fn test_bytecode_nested_try() {
    assert_eq!(
        eval_bytecode(
            "
            let result: string = '';
            try {
                try {
                    throw 'inner';
                } catch (e) {
                    result = result + 'caught-inner ';
                }
                throw 'outer';
            } catch (e) {
                result = result + 'caught-outer';
            }
            result
        "
        ),
        JsValue::String("caught-inner caught-outer".into())
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Update Expressions (++/--)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_prefix_increment() {
    assert_eq!(
        eval_bytecode("let x: number = 5; ++x"),
        JsValue::Number(6.0)
    );
    assert_eq!(
        eval_bytecode("let x: number = 5; ++x; x"),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_bytecode_postfix_increment() {
    assert_eq!(
        eval_bytecode("let x: number = 5; x++"),
        JsValue::Number(5.0) // Returns old value
    );
    assert_eq!(
        eval_bytecode("let x: number = 5; x++; x"),
        JsValue::Number(6.0) // Variable is updated
    );
}

#[test]
fn test_bytecode_prefix_decrement() {
    assert_eq!(
        eval_bytecode("let x: number = 5; --x"),
        JsValue::Number(4.0)
    );
}

#[test]
fn test_bytecode_postfix_decrement() {
    assert_eq!(
        eval_bytecode("let x: number = 5; x--"),
        JsValue::Number(5.0) // Returns old value
    );
    assert_eq!(
        eval_bytecode("let x: number = 5; x--; x"),
        JsValue::Number(4.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Compound Assignment Operators
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_add_assign() {
    assert_eq!(
        eval_bytecode("let x: number = 10; x += 5; x"),
        JsValue::Number(15.0)
    );
}

#[test]
fn test_bytecode_sub_assign() {
    assert_eq!(
        eval_bytecode("let x: number = 10; x -= 3; x"),
        JsValue::Number(7.0)
    );
}

#[test]
fn test_bytecode_mul_assign() {
    assert_eq!(
        eval_bytecode("let x: number = 10; x *= 2; x"),
        JsValue::Number(20.0)
    );
}

#[test]
fn test_bytecode_div_assign() {
    assert_eq!(
        eval_bytecode("let x: number = 10; x /= 2; x"),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_bytecode_mod_assign() {
    assert_eq!(
        eval_bytecode("let x: number = 10; x %= 3; x"),
        JsValue::Number(1.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Template Literals
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_template_literal() {
    assert_eq!(
        eval_bytecode("`hello world`"),
        JsValue::String("hello world".into())
    );
}

#[test]
fn test_bytecode_template_with_expr() {
    assert_eq!(
        eval_bytecode("let name: string = 'Alice'; `Hello, ${name}!`"),
        JsValue::String("Hello, Alice!".into())
    );
}

#[test]
fn test_bytecode_template_with_number() {
    assert_eq!(
        eval_bytecode("let x: number = 42; `The answer is ${x}`"),
        JsValue::String("The answer is 42".into())
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// typeof and instanceof
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_typeof_values() {
    assert_eq!(
        eval_bytecode("typeof undefined"),
        JsValue::String("undefined".into())
    );
    assert_eq!(
        eval_bytecode("typeof null"),
        JsValue::String("object".into())
    );
    assert_eq!(
        eval_bytecode("typeof true"),
        JsValue::String("boolean".into())
    );
    assert_eq!(eval_bytecode("typeof 42"), JsValue::String("number".into()));
    assert_eq!(
        eval_bytecode("typeof 'hello'"),
        JsValue::String("string".into())
    );
    assert_eq!(
        eval_bytecode("typeof ({})"),
        JsValue::String("object".into())
    );
    assert_eq!(eval_bytecode("typeof []"), JsValue::String("object".into()));
}

// ═══════════════════════════════════════════════════════════════════════════
// Function Expressions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_function_expression() {
    // Simple function that returns a value
    assert_eq!(
        eval_bytecode(
            "
            let add = function(a: number, b: number): number { return a + b; };
            add(3, 4)
        "
        ),
        JsValue::Number(7.0)
    );
}

#[test]
fn test_bytecode_function_no_return() {
    // Function with no explicit return returns undefined
    assert_eq!(
        eval_bytecode(
            "
            let noReturn = function(): void {};
            noReturn()
        "
        ),
        JsValue::Undefined
    );
}

#[test]
fn test_bytecode_function_multiple_params() {
    assert_eq!(
        eval_bytecode(
            "
            let sum = function(a: number, b: number, c: number): number { return a + b + c; };
            sum(1, 2, 3)
        "
        ),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_bytecode_function_no_params() {
    assert_eq!(
        eval_bytecode(
            "
            let getFortyTwo = function(): number { return 42; };
            getFortyTwo()
        "
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_bytecode_function_local_vars() {
    assert_eq!(
        eval_bytecode(
            "
            let compute = function(x: number): number {
                let y: number = x * 2;
                let z: number = y + 1;
                return z;
            };
            compute(5)
        "
        ),
        JsValue::Number(11.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Arrow Functions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_arrow_function_expression() {
    // Arrow with expression body
    assert_eq!(
        eval_bytecode(
            "
            let double = (x: number): number => x * 2;
            double(5)
        "
        ),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_bytecode_arrow_function_block() {
    // Arrow with block body
    assert_eq!(
        eval_bytecode(
            "
            let triple = (x: number): number => { return x * 3; };
            triple(4)
        "
        ),
        JsValue::Number(12.0)
    );
}

#[test]
fn test_bytecode_arrow_no_params() {
    assert_eq!(
        eval_bytecode(
            "
            let getNumber = (): number => 100;
            getNumber()
        "
        ),
        JsValue::Number(100.0)
    );
}

#[test]
fn test_bytecode_arrow_multiple_params() {
    assert_eq!(
        eval_bytecode(
            "
            let add = (a: number, b: number, c: number): number => a + b + c;
            add(10, 20, 30)
        "
        ),
        JsValue::Number(60.0)
    );
}

#[test]
fn test_bytecode_arrow_default_param_nested_closure() {
    // Default parameter must be captured in nested arrow function
    assert_eq!(
        eval_bytecode(
            "
            const sortByPrice = (arr: number[], ascending: boolean = true) =>
                arr.sort((a, b) => ascending ? a - b : b - a);
            sortByPrice([3, 1, 2]).join(',')
            "
        ),
        JsValue::from("1,2,3")
    );
    // With explicit false
    assert_eq!(
        eval_bytecode(
            "
            const sortByPrice = (arr: number[], ascending: boolean = true) =>
                arr.sort((a, b) => ascending ? a - b : b - a);
            sortByPrice([3, 1, 2], false).join(',')
            "
        ),
        JsValue::from("3,2,1")
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Closures
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_closure_capture() {
    // Capture variable from outer scope
    assert_eq!(
        eval_bytecode(
            "
            let x: number = 10;
            let addX = (y: number): number => x + y;
            addX(5)
        "
        ),
        JsValue::Number(15.0)
    );
}

#[test]
fn test_bytecode_closure_factory() {
    // Function that returns a function (closure factory)
    assert_eq!(
        eval_bytecode(
            "
            let makeAdder = function(x: number): (y: number) => number {
                return (y: number): number => x + y;
            };
            let add5 = makeAdder(5);
            add5(3)
        "
        ),
        JsValue::Number(8.0)
    );
}

#[test]
fn test_bytecode_closure_counter() {
    assert_eq!(
        eval_bytecode(
            "
            let makeCounter = function(): () => number {
                let count: number = 0;
                return (): number => {
                    count = count + 1;
                    return count;
                };
            };
            let counter = makeCounter();
            counter();
            counter();
            counter()
        "
        ),
        JsValue::Number(3.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Constructor (new) with Functions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_new_function() {
    // Constructor function
    assert_eq!(
        eval_bytecode(
            "
            function Point(x: number, y: number): void {
                this.x = x;
                this.y = y;
            }
            let p = new Point(3, 4);
            p.x + p.y
        "
        ),
        JsValue::Number(7.0)
    );
}

#[test]
fn test_bytecode_new_with_method() {
    assert_eq!(
        eval_bytecode(
            "
            function Rectangle(w: number, h: number): void {
                this.width = w;
                this.height = h;
                this.area = function(): number { return this.width * this.height; };
            }
            let rect = new Rectangle(5, 3);
            rect.area()
        "
        ),
        JsValue::Number(15.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Function typeof
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_typeof_function() {
    assert_eq!(
        eval_bytecode("typeof function(): void {}"),
        JsValue::String("function".into())
    );
    assert_eq!(
        eval_bytecode("typeof ((): void => {})"),
        JsValue::String("function".into())
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Object Destructuring
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_object_destructure_basic() {
    assert_eq!(
        eval_bytecode(
            "
            const obj = { a: 1, b: 2 };
            const { a, b } = obj;
            a + b
        "
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_bytecode_object_destructure_rename() {
    assert_eq!(
        eval_bytecode(
            "
            const obj = { x: 10, y: 20 };
            const { x: first, y: second } = obj;
            first + second
        "
        ),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_bytecode_object_destructure_default() {
    assert_eq!(
        eval_bytecode(
            "
            const obj = { a: 1 };
            const { a, b = 5 } = obj;
            a + b
        "
        ),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_bytecode_object_destructure_nested() {
    assert_eq!(
        eval_bytecode(
            "
            const obj = { outer: { inner: 42 } };
            const { outer: { inner } } = obj;
            inner
        "
        ),
        JsValue::Number(42.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Array Destructuring
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_array_destructure_basic() {
    assert_eq!(
        eval_bytecode(
            "
            const arr: number[] = [1, 2, 3];
            const [a, b, c] = arr;
            a + b + c
        "
        ),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_bytecode_array_destructure_skip() {
    assert_eq!(
        eval_bytecode(
            "
            const arr: number[] = [1, 2, 3, 4];
            const [first, , third] = arr;
            first + third
        "
        ),
        JsValue::Number(4.0)
    );
}

#[test]
fn test_bytecode_array_destructure_default() {
    assert_eq!(
        eval_bytecode(
            "
            const arr: number[] = [1];
            const [a, b = 10] = arr;
            a + b
        "
        ),
        JsValue::Number(11.0)
    );
}

#[test]
fn test_bytecode_array_destructure_rest() {
    assert_eq!(
        eval_bytecode(
            "
            const arr: number[] = [1, 2, 3, 4, 5];
            const [first, ...rest] = arr;
            first + rest.length
        "
        ),
        JsValue::Number(5.0) // 1 + 4
    );
}

#[test]
fn test_bytecode_destructure_in_function_params_single() {
    // Test just accessing a single destructured value
    assert_eq!(
        eval_bytecode(
            "
            function getA({ a }: { a: number }): number {
                return a;
            }
            getA({ a: 42 })
        "
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_bytecode_destructure_in_function_params_two() {
    // Test accessing each destructured value individually
    assert_eq!(
        eval_bytecode(
            "
            function getA({ a, b }: { a: number, b: number }): number {
                return a;
            }
            getA({ a: 3, b: 7 })
        "
        ),
        JsValue::Number(3.0)
    );

    assert_eq!(
        eval_bytecode(
            "
            function getB({ a, b }: { a: number, b: number }): number {
                return b;
            }
            getB({ a: 3, b: 7 })
        "
        ),
        JsValue::Number(7.0)
    );

    // Then test with both
    assert_eq!(
        eval_bytecode(
            "
            function sum({ a, b }: { a: number, b: number }): number {
                return a + b;
            }
            sum({ a: 3, b: 7 })
        "
        ),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_bytecode_destructure_array_in_params() {
    assert_eq!(
        eval_bytecode(
            "
            function getFirst([first]: number[]): number {
                return first;
            }
            getFirst([42, 1, 2])
        "
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_bytecode_destructure_default_param() {
    assert_eq!(
        eval_bytecode(
            "
            function greet(name: string = 'World'): string {
                return 'Hello ' + name;
            }
            greet()
        "
        ),
        JsValue::String("Hello World".into())
    );
}

#[test]
fn test_bytecode_destructure_default_param_with_value() {
    assert_eq!(
        eval_bytecode(
            "
            function add(a: number, b: number = 10): number {
                return a + b;
            }
            add(5)
        "
        ),
        JsValue::Number(15.0)
    );
}

#[test]
fn test_bytecode_destructure_nested_object_param() {
    assert_eq!(
        eval_bytecode(
            "
            function getInner({ outer: { value } }: { outer: { value: number } }): number {
                return value;
            }
            getInner({ outer: { value: 42 } })
        "
        ),
        JsValue::Number(42.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Generator Functions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_generator_basic() {
    // Generator function should return an iterator
    assert_eq!(
        eval_bytecode(
            r#"
            function* gen(): Generator<number> {
                yield 1;
                yield 2;
                yield 3;
            }
            const g = gen();
            g.next().value
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_bytecode_generator_multiple_next() {
    assert_eq!(
        eval_bytecode(
            r#"
            function* gen(): Generator<number> {
                yield 1;
                yield 2;
            }
            const g = gen();
            g.next();
            g.next().value
        "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_bytecode_generator_done() {
    assert_eq!(
        eval_bytecode(
            r#"
            function* gen(): Generator<number> {
                yield 1;
            }
            const g = gen();
            g.next();
            g.next().done
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_bytecode_generator_not_done() {
    assert_eq!(
        eval_bytecode(
            r#"
            function* gen(): Generator<number> {
                yield 1;
            }
            const g = gen();
            g.next().done
        "#
        ),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_bytecode_generator_return_value() {
    // Return value should appear when done
    assert_eq!(
        eval_bytecode(
            r#"
            function* gen(): Generator<number, string> {
                yield 1;
                return "done";
            }
            const g = gen();
            g.next();
            g.next().value
        "#
        ),
        JsValue::from("done")
    );
}

#[test]
fn test_bytecode_generator_no_yield() {
    // Generator without yield should be done immediately
    assert_eq!(
        eval_bytecode(
            r#"
            function* gen(): Generator<void, number> {
                return 42;
            }
            const g = gen();
            const result = g.next();
            result.value + (result.done ? 0 : 100)
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_bytecode_generator_expression() {
    // Generator expression (const gen = function*() {})
    assert_eq!(
        eval_bytecode(
            r#"
            const gen = function*(): Generator<number> {
                yield 10;
                yield 20;
            };
            const g = gen();
            g.next().value + g.next().value
        "#
        ),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_bytecode_generator_with_params() {
    assert_eq!(
        eval_bytecode(
            r#"
            function* range(start: number, end: number): Generator<number> {
                for (let i = start; i < end; i++) {
                    yield i;
                }
            }
            const g = range(1, 4);
            let sum = 0;
            sum += g.next().value;
            sum += g.next().value;
            sum += g.next().value;
            sum
        "#
        ),
        JsValue::Number(6.0) // 1 + 2 + 3
    );
}

#[test]
fn test_bytecode_generator_manual_iteration() {
    // Manually iterate through generator
    assert_eq!(
        eval_bytecode(
            r#"
            function* nums(): Generator<number> {
                yield 1;
                yield 2;
                yield 3;
            }
            const g = nums();
            let sum = 0;
            let result = g.next();
            while (!result.done) {
                sum += result.value;
                result = g.next();
            }
            sum
        "#
        ),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_bytecode_generator_collect_values() {
    // Collect generator values manually
    assert_eq!(
        eval_bytecode(
            r#"
            function* gen(): Generator<number> {
                yield 1;
                yield 2;
                yield 3;
            }
            const g = gen();
            const arr: number[] = [];
            let result = g.next();
            while (!result.done) {
                arr.push(result.value);
                result = g.next();
            }
            arr.length
        "#
        ),
        JsValue::Number(3.0)
    );
}

// Simple generator parameter test
#[test]
fn test_bytecode_generator_simple_param() {
    // A simpler test with just one parameter
    assert_eq!(
        eval_bytecode(
            r#"
            function* simple(n: number): Generator<number> {
                yield n;
            }
            const g = simple(42);
            g.next().value
        "#
        ),
        JsValue::Number(42.0)
    );
}

// Test generator with local variable
#[test]
fn test_bytecode_generator_local_var() {
    assert_eq!(
        eval_bytecode(
            r#"
            function* test(n: number): Generator<number> {
                let i = 0;
                yield i;
                yield n;
            }
            const g = test(42);
            g.next().value + g.next().value
        "#
        ),
        JsValue::Number(42.0) // 0 + 42
    );
}

// Test generator with for loop
#[test]
fn test_bytecode_generator_for_loop() {
    assert_eq!(
        eval_bytecode(
            r#"
            function* test(): Generator<number> {
                for (let i = 0; i < 3; i++) {
                    yield i;
                }
            }
            const g = test();
            g.next().value + g.next().value + g.next().value
        "#
        ),
        JsValue::Number(3.0) // 0 + 1 + 2
    );
}

// Passing values into generators via next()
#[test]
fn test_bytecode_generator_next_with_value() {
    assert_eq!(
        eval_bytecode(
            r#"
            function* gen(): Generator<number, void, number> {
                const x: number = yield 1;
                yield x * 2;
            }
            const g = gen();
            g.next();
            g.next(10).value
        "#
        ),
        JsValue::Number(20.0)
    );
}

#[test]
fn test_bytecode_generator_preserves_scope() {
    // Generator should preserve closure scope
    assert_eq!(
        eval_bytecode(
            r#"
            function makeGen(multiplier: number): () => Generator<number> {
                return function*(): Generator<number> {
                    yield 1 * multiplier;
                    yield 2 * multiplier;
                };
            }
            const gen = makeGen(10);
            const g = gen();
            g.next().value + g.next().value
        "#
        ),
        JsValue::Number(30.0) // 10 + 20
    );
}

// for...of iteration with generators
#[test]
fn test_bytecode_generator_for_of() {
    // for...of should iterate over generator values
    assert_eq!(
        eval_bytecode(
            r#"
            function* gen(): Generator<number> {
                yield 1;
                yield 2;
                yield 3;
            }
            let sum = 0;
            for (const value of gen()) {
                sum += value;
            }
            sum
        "#
        ),
        JsValue::Number(6.0)
    );
}

// yield* (delegation) tests
#[test]
fn test_bytecode_yield_star_array() {
    // yield* should delegate to arrays
    assert_eq!(
        eval_bytecode(
            r#"
            function* gen(): Generator<number> {
                yield* [1, 2, 3];
            }
            const g = gen();
            g.next().value + g.next().value + g.next().value
        "#
        ),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_bytecode_yield_star_to_generator() {
    // yield* should delegate to another generator - using manual iteration
    assert_eq!(
        eval_bytecode(
            r#"
            function* inner(): Generator<number> {
                yield 1;
                yield 2;
            }
            function* outer(): Generator<number> {
                yield 0;
                yield* inner();
                yield 3;
            }
            const g = outer();
            const r0 = g.next().value;
            const r1 = g.next().value;
            const r2 = g.next().value;
            const r3 = g.next().value;
            "" + r0 + r1 + r2 + r3
        "#
        ),
        JsValue::String("0123".into())
    );
}

// for-of with generators
#[test]
fn test_bytecode_for_of_generator() {
    // for-of should iterate over generator values
    assert_eq!(
        eval_bytecode(
            r#"
            function* gen(): Generator<number> {
                yield 1;
                yield 2;
                yield 3;
            }
            let sum: number = 0;
            for (const n of gen()) {
                sum = sum + n;
            }
            sum
        "#
        ),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_bytecode_for_of_array() {
    // for-of should iterate over arrays
    assert_eq!(
        eval_bytecode(
            r#"
            let sum: number = 0;
            for (const n of [1, 2, 3]) {
                sum = sum + n;
            }
            sum
        "#
        ),
        JsValue::Number(6.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Async/Await
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_async_function_returns_promise() {
    // Async function should return a Promise
    let result = eval_bytecode(
        r#"
        async function foo(): Promise<number> {
            return 42;
        }
        const p = foo();
        p instanceof Promise
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_bytecode_async_function_resolved_value() {
    // Async function returning a value should create fulfilled promise
    let result = eval_bytecode(
        r#"
        async function foo(): Promise<number> {
            return 42;
        }
        const p = foo();
        // Promise.then() with callback to extract value
        let result: number = 0;
        p.then((v: number) => { result = v; });
        result
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_bytecode_await_resolved_promise() {
    // await on a resolved Promise should return its value
    let result = eval_bytecode(
        r#"
        async function foo(): Promise<number> {
            const p = Promise.resolve(42);
            return await p;
        }
        let result: number = 0;
        foo().then((v: number) => { result = v; });
        result
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_bytecode_await_non_promise() {
    // await on a non-promise should wrap it in resolved promise
    let result = eval_bytecode(
        r#"
        async function foo(): Promise<number> {
            return await 42;
        }
        let result: number = 0;
        foo().then((v: number) => { result = v; });
        result
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_bytecode_async_arrow_function() {
    // Async arrow function
    let result = eval_bytecode(
        r#"
        const foo = async (): Promise<number> => 42;
        let result: number = 0;
        foo().then((v: number) => { result = v; });
        result
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Class Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_class_basic() {
    // Basic class with constructor and method
    let result = eval_bytecode(
        r#"
        class Point {
            x: number;
            y: number;
            constructor(x: number, y: number) {
                this.x = x;
                this.y = y;
            }
            sum(): number {
                return this.x + this.y;
            }
        }
        const p = new Point(3, 4);
        p.sum()
    "#,
    );
    assert_eq!(result, JsValue::Number(7.0));
}

#[test]
fn test_bytecode_class_no_constructor() {
    // Class without explicit constructor
    let result = eval_bytecode(
        r#"
        class Counter {
            count: number = 0;
            increment(): number {
                this.count = this.count + 1;
                return this.count;
            }
        }
        const c = new Counter();
        c.increment();
        c.increment();
        c.count
    "#,
    );
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn test_bytecode_class_inheritance() {
    // Class extending another class
    let result = eval_bytecode(
        r#"
        class Animal {
            name: string;
            constructor(name: string) {
                this.name = name;
            }
            speak(): string {
                return this.name + " makes a sound";
            }
        }
        class Dog extends Animal {
            constructor(name: string) {
                super(name);
            }
            speak(): string {
                return this.name + " barks";
            }
        }
        const d = new Dog("Rex");
        d.speak()
    "#,
    );
    assert_eq!(result, JsValue::String("Rex barks".into()));
}

#[test]
fn test_bytecode_class_super_method() {
    // Calling super method from subclass
    let result = eval_bytecode(
        r#"
        class Parent {
            getValue(): number {
                return 10;
            }
        }
        class Child extends Parent {
            getValue(): number {
                return super.getValue() + 5;
            }
        }
        const c = new Child();
        c.getValue()
    "#,
    );
    assert_eq!(result, JsValue::Number(15.0));
}

// TODO: Fix deep inheritance with super calls - causes stack overflow
// This is a pre-existing bug in the bytecode VM's super method handling
// #[test]
// fn test_bytecode_class_deep_inheritance() {
//     // Deep inheritance with multiple super calls
//     let result = eval_bytecode(
//         r#"
//         class A {
//             value(): string { return "A"; }
//         }
//         class B extends A {
//             value(): string { return super.value() + "B"; }
//         }
//         class C extends B {
//             value(): string { return super.value() + "C"; }
//         }
//         new C().value()
//     "#,
//     );
//     assert_eq!(result, JsValue::from("ABC"));
// }

#[test]
fn test_bytecode_class_static_method() {
    // Static method on class
    let result = eval_bytecode(
        r#"
        class MathUtils {
            static add(a: number, b: number): number {
                return a + b;
            }
        }
        MathUtils.add(3, 4)
    "#,
    );
    assert_eq!(result, JsValue::Number(7.0));
}

#[test]
fn test_bytecode_class_getter_simple() {
    // Simple getter only
    let result = eval_bytecode(
        r#"
        class Counter {
            _value: number = 42;
            get value(): number {
                return this._value;
            }
        }
        const c = new Counter();
        c.value
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_bytecode_class_setter_simple() {
    // Simple setter only
    let result = eval_bytecode(
        r#"
        class Counter {
            _value: number = 0;
            set value(v: number) {
                this._value = v;
            }
        }
        const c = new Counter();
        c.value = 99;
        c._value
    "#,
    );
    assert_eq!(result, JsValue::Number(99.0));
}

#[test]
fn test_bytecode_class_getter_setter() {
    // Getter and setter
    let result = eval_bytecode(
        r#"
        class Rectangle {
            _width: number = 0;
            _height: number = 0;

            get area(): number {
                return this._width * this._height;
            }

            set width(value: number) {
                this._width = value;
            }

            set height(value: number) {
                this._height = value;
            }
        }
        const r = new Rectangle();
        r.width = 5;
        r.height = 4;
        r.area
    "#,
    );
    assert_eq!(result, JsValue::Number(20.0));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Optional Chaining Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_optional_chain_property() {
    // Access property on object
    assert_eq!(
        eval_bytecode(
            r#"
            const obj = { a: 42 };
            obj?.a
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_bytecode_optional_chain_null() {
    // Access property on null returns undefined
    assert_eq!(
        eval_bytecode(
            r#"
            const obj: { a?: number } | null = null;
            obj?.a
        "#
        ),
        JsValue::Undefined
    );
}

#[test]
fn test_bytecode_optional_chain_undefined() {
    // Access property on undefined returns undefined
    assert_eq!(
        eval_bytecode(
            r#"
            let obj: { a?: number } | undefined;
            obj?.a
        "#
        ),
        JsValue::Undefined
    );
}

#[test]
fn test_bytecode_optional_chain_nested() {
    // Nested optional chaining
    assert_eq!(
        eval_bytecode(
            r#"
            const obj = { a: { b: { c: 100 } } };
            obj?.a?.b?.c
        "#
        ),
        JsValue::Number(100.0)
    );
}

#[test]
fn test_bytecode_optional_chain_nested_null() {
    // Nested optional chaining with null in the middle
    assert_eq!(
        eval_bytecode(
            r#"
            const obj = { a: null };
            obj?.a?.b?.c
        "#
        ),
        JsValue::Undefined
    );
}

#[test]
fn test_bytecode_optional_chain_call() {
    // Optional call
    assert_eq!(
        eval_bytecode(
            r#"
            const obj = { fn: () => 42 };
            obj.fn?.()
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_bytecode_optional_chain_call_undefined() {
    // Optional call on undefined function
    assert_eq!(
        eval_bytecode(
            r#"
            const obj: { fn?: () => number } = {};
            obj.fn?.()
        "#
        ),
        JsValue::Undefined
    );
}

#[test]
fn test_bytecode_regular_computed_access() {
    // Regular computed property access (regression test for ?.[)
    assert_eq!(
        eval_bytecode(
            r#"
            const obj = { a: 42 };
            const key = 'a';
            obj[key]
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_bytecode_optional_chain_computed() {
    // Optional chaining with computed property
    assert_eq!(
        eval_bytecode(
            r#"
            const obj = { a: 42 };
            const key = 'a';
            obj?.[key]
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_bytecode_optional_chain_computed_null() {
    // Optional chaining with computed property on null
    assert_eq!(
        eval_bytecode(
            r#"
            const obj: { [key: string]: number } | null = null;
            const key = 'a';
            obj?.[key]
        "#
        ),
        JsValue::Undefined
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tagged Template Literal Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_tagged_template_basic() {
    // Basic tagged template - returns the strings array
    let result = eval_bytecode(
        r#"
        function tag(strings: TemplateStringsArray): string {
            return strings[0];
        }
        tag`hello`
    "#,
    );
    assert_eq!(result, JsValue::String("hello".into()));
}

#[test]
fn test_bytecode_tagged_template_with_expression() {
    // Tagged template with interpolated expression - simpler version without rest params
    let result = eval_bytecode(
        r#"
        function tag(strings: TemplateStringsArray, val: number): string {
            return strings[0] + val + strings[1];
        }
        const x = 42;
        tag`value is ${x}!`
    "#,
    );
    assert_eq!(result, JsValue::String("value is 42!".into()));
}

#[test]
fn test_bytecode_tagged_template_multiple_expressions() {
    // Tagged template with multiple interpolations - simpler version without rest params
    let result = eval_bytecode(
        r#"
        function tag(strings: TemplateStringsArray, a: number, b: number, sum: number): number {
            return a + b;
        }
        const a = 10;
        const b = 20;
        tag`${a} + ${b} = ${a + b}`
    "#,
    );
    assert_eq!(result, JsValue::Number(30.0));
}

#[test]
fn test_bytecode_tagged_template_raw() {
    // Tagged template with raw strings access
    let result = eval_bytecode(
        r#"
        function tag(strings: TemplateStringsArray): boolean {
            return strings.hasOwnProperty('raw');
        }
        tag`hello`
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_bytecode_tagged_template_returns_value() {
    // Tagged template function can return an object
    let result = eval_bytecode(
        r#"
        function makeObj(strings: TemplateStringsArray, val: number): { strings: number, val: number } {
            return { strings: strings.length, val: val };
        }
        const x = 1;
        const obj = makeObj`a${x}b`;
        obj.val
    "#,
    );
    assert_eq!(result, JsValue::Number(1.0));
}

#[test]
fn test_bytecode_tagged_template_method_call() {
    // Tagged template on a method
    let result = eval_bytecode(
        r#"
        const obj = {
            tag(strings: TemplateStringsArray): string {
                return "tagged: " + strings[0];
            }
        };
        obj.tag`test`
    "#,
    );
    assert_eq!(result, JsValue::String("tagged: test".into()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Arguments Object Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_arguments_basic() {
    // Access arguments object in a regular function
    let result = eval_bytecode(
        r#"
        function foo() {
            return arguments.length;
        }
        foo(1, 2, 3)
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0));
}

#[test]
fn test_bytecode_arguments_access() {
    // Access individual argument via arguments object
    let result = eval_bytecode(
        r#"
        function foo() {
            return arguments[1];
        }
        foo("a", "b", "c")
    "#,
    );
    assert_eq!(result, JsValue::String("b".into()));
}

#[test]
fn test_bytecode_arguments_sum() {
    // Use arguments to sum all passed values
    let result = eval_bytecode(
        r#"
        function sum() {
            let total = 0;
            for (let i = 0; i < arguments.length; i++) {
                total += arguments[i];
            }
            return total;
        }
        sum(1, 2, 3, 4, 5)
    "#,
    );
    assert_eq!(result, JsValue::Number(15.0));
}

// NOTE: new.target tests are not included because the parser doesn't support
// the new.target meta-property syntax yet. The VM does support LoadNewTarget
// opcode, but the compiler can't emit it until parser support is added.

// ═══════════════════════════════════════════════════════════════════════════════
// For-In Loop Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_for_in_object() {
    // for-in should iterate over object's own enumerable property keys
    let result = eval_bytecode(
        r#"
        const obj = { a: 1, b: 2, c: 3 };
        let keys = "";
        for (let key in obj) {
            keys = keys + key;
        }
        keys
    "#,
    );
    // Note: property order is not guaranteed in ES spec, but V8/most engines preserve insertion order
    assert_eq!(result, JsValue::String("abc".into()));
}

#[test]
fn test_bytecode_for_in_with_values() {
    // Access values using the keys from for-in
    let result = eval_bytecode(
        r#"
        const obj = { x: 10, y: 20, z: 30 };
        let sum = 0;
        for (let key in obj) {
            sum = sum + obj[key];
        }
        sum
    "#,
    );
    assert_eq!(result, JsValue::Number(60.0));
}

#[test]
fn test_bytecode_for_in_array() {
    // for-in on array iterates over indices (as strings)
    let result = eval_bytecode(
        r#"
        const arr: number[] = [10, 20, 30];
        let indices = "";
        for (let i in arr) {
            indices = indices + i;
        }
        indices
    "#,
    );
    assert_eq!(result, JsValue::String("012".into()));
}

#[test]
fn test_bytecode_for_in_empty_object() {
    // for-in on empty object should not execute body
    let result = eval_bytecode(
        r#"
        const obj = {};
        let count = 0;
        for (let key in obj) {
            count = count + 1;
        }
        count
    "#,
    );
    assert_eq!(result, JsValue::Number(0.0));
}

#[test]
fn test_bytecode_for_in_with_break() {
    // break should exit for-in loop
    let result = eval_bytecode(
        r#"
        const obj = { a: 1, b: 2, c: 3 };
        let count = 0;
        for (let key in obj) {
            count = count + 1;
            if (count >= 2) break;
        }
        count
    "#,
    );
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn test_bytecode_for_in_with_continue() {
    // continue should skip to next iteration
    let result = eval_bytecode(
        r#"
        const obj = { a: 1, b: 2, c: 3 };
        let sum = 0;
        for (let key in obj) {
            if (key === "b") continue;
            sum = sum + obj[key];
        }
        sum
    "#,
    );
    assert_eq!(result, JsValue::Number(4.0)); // 1 + 3 = 4
}

// ═══════════════════════════════════════════════════════════════════════════════
// Spread Operator Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_spread_in_array() {
    // Spread elements into array literal
    let result = eval_bytecode(
        r#"
        const arr: number[] = [1, 2, 3];
        const spread: number[] = [0, ...arr, 4];
        spread.length
    "#,
    );
    assert_eq!(result, JsValue::Number(5.0));
}

#[test]
fn test_bytecode_spread_in_array_values() {
    // Verify spread copies values correctly
    let result = eval_bytecode(
        r#"
        const arr: number[] = [1, 2, 3];
        const spread: number[] = [0, ...arr, 4];
        spread[0] + spread[1] + spread[2] + spread[3] + spread[4]
    "#,
    );
    assert_eq!(result, JsValue::Number(10.0)); // 0 + 1 + 2 + 3 + 4 = 10
}

#[test]
fn test_bytecode_spread_multiple_arrays() {
    // Spread multiple arrays
    let result = eval_bytecode(
        r#"
        const a: number[] = [1, 2];
        const b: number[] = [3, 4];
        const combined: number[] = [...a, ...b];
        combined.length
    "#,
    );
    assert_eq!(result, JsValue::Number(4.0));
}

#[test]
fn test_bytecode_spread_in_function_call() {
    // Spread array as function arguments
    let result = eval_bytecode(
        r#"
        function sum(a: number, b: number, c: number): number {
            return a + b + c;
        }
        const args: number[] = [1, 2, 3];
        sum(...args)
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_bytecode_spread_in_function_call_mixed() {
    // Spread with regular arguments
    let result = eval_bytecode(
        r#"
        function sum(a: number, b: number, c: number, d: number): number {
            return a + b + c + d;
        }
        const args: number[] = [2, 3];
        sum(1, ...args, 4)
    "#,
    );
    assert_eq!(result, JsValue::Number(10.0));
}

#[test]
fn test_bytecode_spread_empty_array() {
    // Spread empty array
    let result = eval_bytecode(
        r#"
        const empty: number[] = [];
        const arr: number[] = [1, ...empty, 2];
        arr.length
    "#,
    );
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn test_bytecode_spread_in_new() {
    // Spread in constructor call
    let result = eval_bytecode(
        r#"
        function Pair(a: number, b: number) {
            this.sum = a + b;
        }
        const args: number[] = [3, 7];
        const p = new Pair(...args);
        p.sum
    "#,
    );
    assert_eq!(result, JsValue::Number(10.0));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Rest Parameter Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_rest_param_basic() {
    // Basic rest parameter
    let result = eval_bytecode(
        r#"
        function sum(...nums: number[]): number {
            let total = 0;
            for (let i = 0; i < nums.length; i++) {
                total = total + nums[i];
            }
            return total;
        }
        sum(1, 2, 3, 4, 5)
    "#,
    );
    assert_eq!(result, JsValue::Number(15.0));
}

#[test]
fn test_bytecode_rest_param_with_leading() {
    // Rest parameter with leading regular parameters
    let result = eval_bytecode(
        r#"
        function greet(greeting: string, ...names: string[]): string {
            return greeting + " " + names.join(", ");
        }
        greet("Hello", "Alice", "Bob", "Charlie")
    "#,
    );
    assert_eq!(result, JsValue::String("Hello Alice, Bob, Charlie".into()));
}

#[test]
fn test_bytecode_rest_param_empty() {
    // Rest parameter when no extra args passed
    let result = eval_bytecode(
        r#"
        function count(first: number, ...rest: number[]): number {
            return rest.length;
        }
        count(1)
    "#,
    );
    assert_eq!(result, JsValue::Number(0.0));
}

#[test]
fn test_bytecode_rest_param_single() {
    // Rest parameter with single extra arg
    let result = eval_bytecode(
        r#"
        function count(first: number, ...rest: number[]): number {
            return rest.length;
        }
        count(1, 2)
    "#,
    );
    assert_eq!(result, JsValue::Number(1.0));
}

#[test]
fn test_bytecode_rest_param_array_methods() {
    // Use array methods on rest parameter
    let result = eval_bytecode(
        r#"
        function sumDoubled(...nums: number[]): number {
            return nums.map((n: number): number => n * 2).reduce((a: number, b: number): number => a + b, 0);
        }
        sumDoubled(1, 2, 3)
    "#,
    );
    assert_eq!(result, JsValue::Number(12.0)); // (1*2) + (2*2) + (3*2) = 12
}

#[test]
fn test_bytecode_rest_param_arrow() {
    // Rest parameter in arrow function
    let result = eval_bytecode(
        r#"
        const sum = (...nums: number[]): number => {
            let total = 0;
            for (let i = 0; i < nums.length; i++) {
                total = total + nums[i];
            }
            return total;
        };
        sum(10, 20, 30)
    "#,
    );
    assert_eq!(result, JsValue::Number(60.0));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Delete Operator Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_delete_property() {
    // Delete a property from an object
    let result = eval_bytecode(
        r#"
        const obj = { a: 1, b: 2, c: 3 };
        delete obj.b;
        obj.b
    "#,
    );
    assert_eq!(result, JsValue::Undefined);
}

#[test]
fn test_bytecode_delete_returns_true() {
    // Delete returns true when property is deleted
    let result = eval_bytecode(
        r#"
        const obj = { a: 1 };
        delete obj.a
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_bytecode_delete_nonexistent() {
    // Delete returns true for non-existent property
    let result = eval_bytecode(
        r#"
        const obj = { a: 1 };
        delete obj.xyz
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_bytecode_delete_computed() {
    // Delete with computed property access
    let result = eval_bytecode(
        r#"
        const obj = { foo: 1, bar: 2 };
        const key = "foo";
        delete obj[key];
        obj.foo
    "#,
    );
    assert_eq!(result, JsValue::Undefined);
}

#[test]
fn test_bytecode_delete_array_element() {
    // Delete array element creates hole
    let result = eval_bytecode(
        r#"
        const arr: (number | undefined)[] = [1, 2, 3];
        delete arr[1];
        arr[1]
    "#,
    );
    assert_eq!(result, JsValue::Undefined);
}

#[test]
fn test_bytecode_delete_preserves_length() {
    // Delete doesn't change array length
    let result = eval_bytecode(
        r#"
        const arr: number[] = [1, 2, 3];
        delete arr[1];
        arr.length
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0));
}

// ============================================================================
// Labeled Statements
// ============================================================================

#[test]
fn test_bytecode_labeled_break() {
    // Labeled break exits the labeled loop
    let result = eval_bytecode(
        r#"
        let sum: number = 0;
        outer: for (let i = 0; i < 3; i++) {
            for (let j = 0; j < 3; j++) {
                if (i === 1 && j === 1) break outer;
                sum += 1;
            }
        }
        sum
    "#,
    );
    // i=0: j=0,1,2 (3 iterations)
    // i=1: j=0 (1 iteration), then break at j=1
    assert_eq!(result, JsValue::Number(4.0));
}

#[test]
fn test_bytecode_labeled_continue() {
    // Labeled continue jumps to the next iteration of the labeled loop
    let result = eval_bytecode(
        r#"
        let sum: number = 0;
        outer: for (let i = 0; i < 3; i++) {
            for (let j = 0; j < 3; j++) {
                if (j === 1) continue outer;
                sum += 1;
            }
        }
        sum
    "#,
    );
    // Each outer iteration only processes j=0 before continue outer
    // i=0: j=0 (1), continue outer
    // i=1: j=0 (1), continue outer
    // i=2: j=0 (1), continue outer
    assert_eq!(result, JsValue::Number(3.0));
}

#[test]
fn test_bytecode_labeled_while() {
    // Labeled break with while loops
    let result = eval_bytecode(
        r#"
        let i: number = 0;
        let j: number = 0;
        outer: while (i < 5) {
            while (j < 5) {
                if (j === 2) break outer;
                j++;
            }
            i++;
        }
        i * 10 + j
    "#,
    );
    // Should break out at i=0, j=2: 0 * 10 + 2 = 2
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn test_bytecode_nested_labels() {
    // Multiple nested labels
    let result = eval_bytecode(
        r#"
        let sum: number = 0;
        outer: for (let i = 0; i < 2; i++) {
            middle: for (let j = 0; j < 2; j++) {
                for (let k = 0; k < 2; k++) {
                    if (k === 1) break middle;
                    sum += 1;
                }
            }
        }
        sum
    "#,
    );
    // i=0: j=0: k=0 (1), k=1 breaks middle, j=1 skipped
    // i=1: j=0: k=0 (1), k=1 breaks middle, j=1 skipped
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn test_bytecode_labeled_block() {
    // Labeled block (non-loop) with break
    let result = eval_bytecode(
        r#"
        let x: number = 0;
        block: {
            x = 1;
            if (true) break block;
            x = 2;
        }
        x
    "#,
    );
    assert_eq!(result, JsValue::Number(1.0));
}

#[test]
fn test_bytecode_unlabeled_break_with_label() {
    // Unlabeled break only exits innermost loop
    let result = eval_bytecode(
        r#"
        let sum: number = 0;
        outer: for (let i = 0; i < 2; i++) {
            for (let j = 0; j < 3; j++) {
                if (j === 1) break;  // breaks inner loop only
                sum += 1;
            }
        }
        sum
    "#,
    );
    // i=0: j=0 (1), j=1 breaks inner
    // i=1: j=0 (1), j=1 breaks inner
    assert_eq!(result, JsValue::Number(2.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Async Generators
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_async_generator_basic() {
    // Test that async generator function can be created and called
    let result = eval_bytecode(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
        }
        let g = gen();
        let r = g.next();
        // Check it's a promise
        typeof r.then === 'function'
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_bytecode_async_generator_next_returns_promise() {
    // Verify next() returns a Promise that resolves to {value, done}
    let result = eval_bytecode(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 42;
        }
        let g = gen();
        let promise = g.next();
        let resolved: {value: number, done: boolean} | null = null;
        promise.then((r: {value: number, done: boolean}) => {
            resolved = r;
        });
        // After promise resolution, check the result
        resolved !== null && resolved.value === 42 && resolved.done === false
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_bytecode_async_generator_done() {
    // Test that done is true after generator completes
    let result = eval_bytecode(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 1;
        }
        let g = gen();
        let firstDone: boolean = false;
        let secondDone: boolean = false;

        g.next().then((r: {done: boolean}) => { firstDone = r.done; });
        g.next().then((r: {done: boolean}) => { secondDone = r.done; });

        firstDone === false && secondDone === true
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_bytecode_async_generator_return() {
    // Test that return() returns a fulfilled Promise
    let result = eval_bytecode(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 1;
            yield 2;
        }
        let g = gen();
        let returnValue: number = 0;
        let returnDone: boolean = false;

        g.return(99).then((r: {value: number, done: boolean}) => {
            returnValue = r.value;
            returnDone = r.done;
        });

        returnValue === 99 && returnDone === true
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_bytecode_async_generator_has_async_iterator() {
    // Async generators should have Symbol.asyncIterator
    let result = eval_bytecode(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 1;
        }
        let g = gen();
        typeof g[Symbol.asyncIterator] === 'function'
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_bytecode_async_generator_async_iterator_method() {
    // Check that [Symbol.asyncIterator] is a function
    let result = eval_bytecode(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 42;
        }
        let g = gen();
        // Get the method and check it's a function
        typeof g[Symbol.asyncIterator]
    "#,
    );
    assert_eq!(result, JsValue::String("function".into()));
}

#[test]
fn test_bytecode_async_generator_expression() {
    // Test async generator as an expression
    let result = eval_bytecode(
        r#"
        let gen = async function*(): AsyncGenerator<number> {
            yield 100;
        };
        let g = gen();
        let val: number = 0;
        g.next().then((r: {value: number}) => { val = r.value; });
        val
    "#,
    );
    assert_eq!(result, JsValue::Number(100.0));
}

#[test]
fn test_bytecode_async_generator_multiple_yields() {
    // Test multiple yields with values accumulating
    let result = eval_bytecode(
        r#"
        async function* counter(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
        }
        let g = counter();
        let sum: number = 0;

        g.next().then((r: {value: number, done: boolean}) => { if (!r.done) sum += r.value; });
        g.next().then((r: {value: number, done: boolean}) => { if (!r.done) sum += r.value; });
        g.next().then((r: {value: number, done: boolean}) => { if (!r.done) sum += r.value; });

        sum
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Super in Static Methods
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_super_in_instance_method() {
    // First verify super works in instance methods
    let result = eval_bytecode(
        r#"
        class B {
            method() { return 1; }
            get x() { return 2; }
        }
        class C extends B {
            method() {
                return super.x + super.method();
            }
        }
        new C().method()
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0));
}

#[test]
fn test_bytecode_super_in_static_method() {
    let result = eval_bytecode(
        r#"
        class B {
            static method() { return 1; }
            static get x() { return 2; }
        }
        class C extends B {
            static method() {
                return super.x + super.method();
            }
        }
        C.method()
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0));
}

#[test]
fn test_bytecode_super_in_static_getter() {
    let result = eval_bytecode(
        r#"
        class B {
            static get value() { return 42; }
        }
        class C extends B {
            static get doubled() {
                return super.value * 2;
            }
        }
        C.doubled
    "#,
    );
    assert_eq!(result, JsValue::Number(84.0));
}

#[test]
fn test_bytecode_super_in_static_setter() {
    let result = eval_bytecode(
        r#"
        class B {
            static get base() { return 10; }
        }
        class C extends B {
            static result: number = 0;
            static set value(v: number) {
                C.result = v + super.base;
            }
        }
        C.value = 5;
        C.result
    "#,
    );
    assert_eq!(result, JsValue::Number(15.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Static Initialization Blocks
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_static_block_runs() {
    // First verify static block actually runs
    let result = eval_bytecode(
        r#"
        let blockRan = false;
        class Config {
            static {
                blockRan = true;
            }
        }
        blockRan
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_bytecode_static_block_this_binding() {
    // Test that 'this' in static block refers to the class
    let result = eval_bytecode(
        r#"
        class Config {
            static value: number = 0;
            static {
                this.value = 42;
            }
        }
        Config.value
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_bytecode_static_block_class_name_access() {
    // Test that class name is accessible in static block
    let result = eval_bytecode(
        r#"
        class Config {
            static value: number = 0;
            static {
                Config.value = 42;
            }
        }
        Config.value
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_bytecode_static_initialization_block() {
    // Static block should be able to modify static fields
    let result = eval_bytecode(
        r#"
        class Config {
            static initialized: boolean = false;
            static value: number = 0;

            static {
                Config.initialized = true;
                Config.value = 42;
            }
        }
        Config.value
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Closure Scoping in Loops
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_for_let_closure_capture() {
    // Each iteration of for (let i...) should create fresh binding
    let result = eval_bytecode(
        r#"
        let funcs: any[] = [];
        for (let i: number = 0; i < 3; i = i + 1) {
            funcs.push(function(): number { return i; });
        }
        funcs[0]() + "," + funcs[1]() + "," + funcs[2]()
    "#,
    );
    assert_eq!(result, JsValue::String("0,1,2".into()));
}

// ═══════════════════════════════════════════════════════════════════════════
// Optional Chaining with Parentheses
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_optional_chain_basic() {
    // First verify basic optional chaining works
    let result = eval_bytecode(
        r#"
        const a = { b: 42 };
        a?.b
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_bytecode_optional_chain_shortcircuit() {
    // Optional chaining on undefined should return undefined
    let result = eval_bytecode(
        r#"
        let a: any = undefined;
        a?.b
    "#,
    );
    assert_eq!(result, JsValue::Undefined);
}

#[test]
fn test_bytecode_optional_call() {
    // Optional call: func?.()
    let result = eval_bytecode(
        r#"
        const a = {
            b() { return 42; }
        };
        a.b?.()
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_bytecode_optional_chain_parenthesized_debug() {
    // Debug: first test simpler case of parenthesized member access
    let result = eval_bytecode(
        r#"
        const a = { b: 42 };
        (a?.b)
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_bytecode_optional_chain_parenthesized_method() {
    // Debug: parenthesized method access without calling
    let result = eval_bytecode(
        r#"
        const a = { b() { return 42; } };
        typeof (a?.b)
    "#,
    );
    assert_eq!(result, JsValue::String("function".into()));
}

#[test]
fn test_bytecode_optional_chain_parenthesized_call() {
    // Debug: parenthesized method access then call
    let result = eval_bytecode(
        r#"
        const a = { b() { return 42; } };
        (a?.b)()
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_bytecode_optional_chain_parenthesized_optional_call() {
    // (a?.b)?.() - parenthesized optional chain with optional call
    let result = eval_bytecode(
        r#"
        const a = { b() { return 42; } };
        (a?.b)?.()
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_bytecode_optional_chain_parenthesized_optional_call_with_this() {
    // Test that this is correctly bound when using optional call after parenthesized chain
    // Optional chaining actually preserves this binding even through parentheses!
    let result = eval_bytecode(
        r#"
        const a = {
            b() { return this._b; },
            _b: 42
        };
        (a?.b)?.()
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_bytecode_optional_chain_parenthesized() {
    // (a?.b)?.() - parenthesized optional chain
    let result = eval_bytecode(
        r#"
        const a = {
            b() { return this._b; },
            _b: { c: 42 }
        };
        (a?.b)?.().c
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Argument Evaluation Order
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_args_not_evaluated_when_callee_throws() {
    // When evaluating the callee throws (e.g., undefined.prop),
    // arguments should NOT be evaluated
    let result = eval_bytecode(
        r#"
        var fooCalled = false;
        function foo() { fooCalled = true; }
        var o = {};
        try {
            o.bar.gar(foo());  // o.bar is undefined, .gar throws
        } catch (e) {}
        fooCalled
    "#,
    );
    assert_eq!(result, JsValue::Boolean(false));
}

// ═══════════════════════════════════════════════════════════════════════════
// Register Limit Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_collections_like_code() {
    // This reproduces the collections/main.ts pattern that was hitting register limits
    let result = eval_bytecode_result(
        r#"
        // Map/Set Collections Demo
        console.log("=== Map/Set Collections Demo ===");

        // --- Basic Map Operations ---
        const map = new Map();
        map.set("one", 1);
        map.set("two", 2);
        map.set("three", 3);

        console.log("Map size:", map.size);
        console.log("get('two'):", map.get("two"));

        // Map initialization with array of entries
        const fruitPrices = new Map([
            ["apple", 1.5],
            ["banana", 0.75],
            ["orange", 2.0],
            ["grape", 3.25]
        ]);
        console.log("Fruit prices map:");
        fruitPrices.forEach((price, fruit) => {
            console.log("  " + fruit + ": $" + price);
        });

        // --- Basic Set Operations ---
        const set = new Set();
        set.add(1);
        set.add(2);
        set.add(3);

        console.log("Set size:", set.size);

        // Set initialization with array
        const colors = new Set(["red", "green", "blue", "red", "yellow"]);
        console.log("Colors set size:", colors.size);

        // --- Set Operations ---
        const setA = new Set([1, 2, 3, 4, 5]);
        const setB = new Set([4, 5, 6, 7, 8]);

        function union(a, b) {
            const result = new Set(a);
            for (const item of b) {
                result.add(item);
            }
            return result;
        }

        function intersection(a, b) {
            const result = new Set();
            for (const item of a) {
                if (b.has(item)) {
                    result.add(item);
                }
            }
            return result;
        }

        console.log("Set A:", JSON.stringify(Array.from(setA)));
        console.log("Set B:", JSON.stringify(Array.from(setB)));
        console.log("Union:", JSON.stringify(Array.from(union(setA, setB))));
        console.log("Intersection:", JSON.stringify(Array.from(intersection(setA, setB))));

        // --- Using Map for Caching ---
        const cache = new Map();
        let computeCount = 0;

        function expensiveCompute(n) {
            if (cache.has(n)) {
                return cache.get(n);
            }
            computeCount++;
            const result = n * n + n;
            cache.set(n, result);
            return result;
        }

        console.log("Computing 5:", expensiveCompute(5));
        console.log("Computing 10:", expensiveCompute(10));
        console.log("Computing 5 (cached):", expensiveCompute(5));
        console.log("Compute count:", computeCount);

        "done"
    "#,
    );

    match result {
        Ok(v) => assert_eq!(v, JsValue::from("done")),
        Err(e) => panic!("Collections-like code failed: {}", e),
    }
}

#[test]
fn test_bytecode_combined_module_code() {
    // Combine code from graph.ts, counter.ts and main.ts to simulate
    // what happens when all modules are loaded together
    let result = eval_bytecode_result(
        r#"
        // === graph.ts code ===
        function createGraph() { return { nodes: new Map() }; }
        function addNode(graph, node) { if (!graph.nodes.has(node)) { graph.nodes.set(node, new Set()); } }
        function addEdge(graph, from, to) { addNode(graph, from); addNode(graph, to); graph.nodes.get(from).add(to); }
        function getNeighbors(graph, node) { return graph.nodes.get(node) || new Set(); }
        function bfs(graph, start) {
            const visited = new Set();
            const result = [];
            const queue = [start];
            while (queue.length > 0) {
                const current = queue.shift();
                if (visited.has(current)) continue;
                visited.add(current);
                result.push(current);
                for (const neighbor of getNeighbors(graph, current)) {
                    if (!visited.has(neighbor)) queue.push(neighbor);
                }
            }
            return result;
        }
        function dfs(graph, start) {
            const visited = new Set();
            const result = [];
            function visit(node) {
                if (visited.has(node)) return;
                visited.add(node);
                result.push(node);
                for (const neighbor of getNeighbors(graph, node)) visit(neighbor);
            }
            visit(start);
            return result;
        }
        function buildGraph() {
            const graph = createGraph();
            addEdge(graph, "A", "B");
            addEdge(graph, "A", "C");
            addEdge(graph, "B", "D");
            addEdge(graph, "B", "E");
            addEdge(graph, "C", "F");
            return graph;
        }

        // === counter.ts code ===
        function createWordCounter() {
            return { frequencies: new Map(), uniqueWords: new Set(), totalWords: 0 };
        }
        function addWord(counter, word) {
            const normalized = word.toLowerCase();
            counter.totalWords++;
            counter.uniqueWords.add(normalized);
            const count = counter.frequencies.get(normalized) || 0;
            counter.frequencies.set(normalized, count + 1);
        }
        function analyzeText(text) {
            const counter = createWordCounter();
            const words = text.split(/[\s,.!?;:'"()\[\]{}]+/);
            for (const word of words) {
                if (word.length > 0) addWord(counter, word);
            }
            return counter;
        }

        // === main.ts code (simplified) ===
        console.log("=== Combined Module Test ===");

        const map = new Map();
        map.set("one", 1);
        map.set("two", 2);
        console.log("Map size:", map.size);

        const fruitPrices = new Map([
            ["apple", 1.5], ["banana", 0.75], ["orange", 2.0], ["grape", 3.25]
        ]);

        const set = new Set();
        set.add(1); set.add(2); set.add(3);
        console.log("Set size:", set.size);

        const colors = new Set(["red", "green", "blue"]);
        const setA = new Set([1, 2, 3, 4, 5]);
        const setB = new Set([4, 5, 6, 7, 8]);

        function union(a, b) {
            const result = new Set(a);
            for (const item of b) result.add(item);
            return result;
        }

        const cache = new Map();
        function expensiveCompute(n) {
            if (cache.has(n)) return cache.get(n);
            const result = n * n + n;
            cache.set(n, result);
            return result;
        }

        const graph = buildGraph();
        console.log("BFS from A:", JSON.stringify(bfs(graph, "A")));

        const text = "the quick brown fox jumps over the lazy dog";
        const counter = analyzeText(text);
        console.log("Unique words:", counter.uniqueWords.size);

        "done"
    "#,
    );

    match result {
        Ok(v) => assert_eq!(v, JsValue::from("done")),
        Err(e) => panic!("Combined module code failed: {}", e),
    }
}

#[test]
fn test_bytecode_employees_array() {
    // Test the employees array pattern from collections/main.ts
    let result = eval_bytecode_result(
        r#"
        const employees = [
            { name: "Alice", department: "Engineering", salary: 75000 },
            { name: "Bob", department: "Sales", salary: 60000 },
            { name: "Charlie", department: "Engineering", salary: 80000 },
            { name: "Diana", department: "Sales", salary: 65000 },
            { name: "Eve", department: "Marketing", salary: 55000 }
        ];

        function groupBy(items, keyFn) {
            const groups = new Map();
            for (const item of items) {
                const key = keyFn(item);
                if (!groups.has(key)) {
                    groups.set(key, []);
                }
                groups.get(key).push(item);
            }
            return groups;
        }

        const byDepartment = groupBy(employees, e => e.department);
        console.log("Groups:", byDepartment.size);
        byDepartment.forEach((people, dept) => {
            const names = people.map(p => p.name).join(", ");
            console.log("  " + dept + ": " + names);
        });

        const data = [
            { id: 1, value: "a" },
            { id: 2, value: "b" },
            { id: 1, value: "c" },
            { id: 3, value: "d" },
            { id: 2, value: "e" }
        ];

        const seenIds = new Set();
        const uniqueById = [];

        for (const item of data) {
            if (!seenIds.has(item.id)) {
                seenIds.add(item.id);
                uniqueById.push(item);
            }
        }

        console.log("Unique:", uniqueById.length);
        uniqueById.length
    "#,
    );

    match result {
        Ok(v) => assert_eq!(v, JsValue::Number(3.0)),
        Err(e) => panic!("Employees array code failed: {}", e),
    }
}

#[test]
fn test_bytecode_collections_part1_maps_sets() {
    // Just the basic map and set operations
    let result = eval_bytecode_result(
        r#"
        const map = new Map();
        map.set("one", 1);
        map.set("two", 2);
        map.set("three", 3);

        const fruitPrices = new Map([
            ["apple", 1.5],
            ["banana", 0.75],
            ["orange", 2.0],
            ["grape", 3.25]
        ]);
        fruitPrices.forEach((price, fruit) => {
            console.log("  " + fruit + ": $" + price);
        });

        for (const [key, value] of fruitPrices.entries()) {
            console.log("  " + key + " => " + value);
        }

        const set = new Set();
        set.add(1);
        set.add(2);
        set.add(3);
        set.add(2);

        const colors = new Set(["red", "green", "blue", "red", "yellow"]);

        for (const color of colors) {
            console.log("  " + color);
        }

        colors.forEach(color => {
            console.log("  Color: " + color);
        });

        "part1_done"
    "#,
    );

    match result {
        Ok(v) => assert_eq!(v, JsValue::from("part1_done")),
        Err(e) => panic!("Part 1 (maps/sets) failed: {}", e),
    }
}

#[test]
fn test_bytecode_collections_part2_set_operations() {
    // Set operations: union, intersection, difference
    let result = eval_bytecode_result(
        r#"
        const setA = new Set([1, 2, 3, 4, 5]);
        const setB = new Set([4, 5, 6, 7, 8]);

        function union(a, b) {
            const result = new Set(a);
            for (const item of b) {
                result.add(item);
            }
            return result;
        }

        function intersection(a, b) {
            const result = new Set();
            for (const item of a) {
                if (b.has(item)) {
                    result.add(item);
                }
            }
            return result;
        }

        function difference(a, b) {
            const result = new Set();
            for (const item of a) {
                if (!b.has(item)) {
                    result.add(item);
                }
            }
            return result;
        }

        function symmetricDifference(a, b) {
            return union(difference(a, b), difference(b, a));
        }

        function isSubset(a, b) {
            for (const item of a) {
                if (!b.has(item)) {
                    return false;
                }
            }
            return true;
        }

        console.log("Union:", JSON.stringify(Array.from(union(setA, setB))));
        console.log("Intersection:", JSON.stringify(Array.from(intersection(setA, setB))));
        console.log("Difference:", JSON.stringify(Array.from(difference(setA, setB))));
        console.log("SymmetricDiff:", JSON.stringify(Array.from(symmetricDifference(setA, setB))));

        const setC = new Set([1, 2, 3]);
        console.log("Is C subset of A?", isSubset(setC, setA));

        "part2_done"
    "#,
    );

    match result {
        Ok(v) => assert_eq!(v, JsValue::from("part2_done")),
        Err(e) => panic!("Part 2 (set operations) failed: {}", e),
    }
}

#[test]
fn test_bytecode_collections_part3_graph_and_counter() {
    // Graph and word counter functions
    let result = eval_bytecode_result(
        r#"
        function createGraph() {
            return { nodes: new Map() };
        }

        function addNode(graph, node) {
            if (!graph.nodes.has(node)) {
                graph.nodes.set(node, new Set());
            }
        }

        function addEdge(graph, from, to) {
            addNode(graph, from);
            addNode(graph, to);
            graph.nodes.get(from).add(to);
        }

        function getNeighbors(graph, node) {
            return graph.nodes.get(node) || new Set();
        }

        function bfs(graph, start) {
            const visited = new Set();
            const result = [];
            const queue = [start];

            while (queue.length > 0) {
                const current = queue.shift();
                if (visited.has(current)) continue;
                visited.add(current);
                result.push(current);

                const neighbors = getNeighbors(graph, current);
                for (const neighbor of neighbors) {
                    if (!visited.has(neighbor)) {
                        queue.push(neighbor);
                    }
                }
            }
            return result;
        }

        function dfs(graph, start) {
            const visited = new Set();
            const result = [];

            function visit(node) {
                if (visited.has(node)) return;
                visited.add(node);
                result.push(node);
                const neighbors = getNeighbors(graph, node);
                for (const neighbor of neighbors) {
                    visit(neighbor);
                }
            }

            visit(start);
            return result;
        }

        const graph = createGraph();
        addEdge(graph, "A", "B");
        addEdge(graph, "A", "C");
        addEdge(graph, "B", "D");
        addEdge(graph, "B", "E");
        addEdge(graph, "C", "F");

        console.log("BFS:", JSON.stringify(bfs(graph, "A")));
        console.log("DFS:", JSON.stringify(dfs(graph, "A")));

        // Word counter
        function createWordCounter() {
            return { frequencies: new Map(), uniqueWords: new Set(), totalWords: 0 };
        }

        function addWord(counter, word) {
            const normalized = word.toLowerCase();
            counter.totalWords++;
            counter.uniqueWords.add(normalized);
            const count = counter.frequencies.get(normalized) || 0;
            counter.frequencies.set(normalized, count + 1);
        }

        function analyzeText(text) {
            const counter = createWordCounter();
            const words = text.split(/[\s,.!?;:'"()\[\]{}]+/);
            for (const word of words) {
                if (word.length > 0) addWord(counter, word);
            }
            return counter;
        }

        const text = "the quick brown fox jumps over the lazy dog the fox is quick";
        const counter = analyzeText(text);
        console.log("Unique words:", counter.uniqueWords.size);

        "part3_done"
    "#,
    );

    match result {
        Ok(v) => assert_eq!(v, JsValue::from("part3_done")),
        Err(e) => panic!("Part 3 (graph/counter) failed: {}", e),
    }
}

#[test]
fn test_bytecode_collections_part4_conversions() {
    // Converting between collections
    let result = eval_bytecode_result(
        r#"
        const arr = [1, 2, 2, 3, 3, 3, 4, 4, 4, 4];
        const uniqueSet = new Set(arr);
        console.log("Unique values:", JSON.stringify(Array.from(uniqueSet)));

        const userMap = new Map([["alice", 30], ["bob", 25], ["charlie", 35]]);
        const userObj = {};
        userMap.forEach((value, key) => { userObj[key] = value; });
        console.log("Map to Object:", JSON.stringify(userObj));

        const configObj = { host: "localhost", port: "8080", protocol: "https" };
        const configMap = new Map(Object.entries(configObj));
        console.log("Object to Map size:", configMap.size);

        const employees = [
            { name: "Alice", department: "Engineering", salary: 75000 },
            { name: "Bob", department: "Sales", salary: 60000 },
            { name: "Charlie", department: "Engineering", salary: 80000 },
            { name: "Diana", department: "Sales", salary: 65000 },
            { name: "Eve", department: "Marketing", salary: 55000 }
        ];

        function groupBy(items, keyFn) {
            const groups = new Map();
            for (const item of items) {
                const key = keyFn(item);
                if (!groups.has(key)) groups.set(key, []);
                groups.get(key).push(item);
            }
            return groups;
        }

        const byDepartment = groupBy(employees, e => e.department);
        console.log("Groups:", byDepartment.size);

        const data = [
            { id: 1, value: "a" },
            { id: 2, value: "b" },
            { id: 1, value: "c" },
            { id: 3, value: "d" },
            { id: 2, value: "e" }
        ];

        const seenIds = new Set();
        const uniqueById = [];

        for (const item of data) {
            if (!seenIds.has(item.id)) {
                seenIds.add(item.id);
                uniqueById.push(item);
            }
        }

        console.log("Unique by id length:", uniqueById.length);
        "part4_done"
    "#,
    );

    match result {
        Ok(v) => assert_eq!(v, JsValue::from("part4_done")),
        Err(e) => panic!("Part 4 (conversions) failed: {}", e),
    }
}

#[test]
fn test_bytecode_collections_parts_1_and_2() {
    // Combine parts 1 and 2
    let result = eval_bytecode_result(
        r#"
        // Part 1: basic map and set operations
        const map = new Map();
        map.set("one", 1);
        map.set("two", 2);
        map.set("three", 3);

        const fruitPrices = new Map([
            ["apple", 1.5],
            ["banana", 0.75],
            ["orange", 2.0],
            ["grape", 3.25]
        ]);
        fruitPrices.forEach((price, fruit) => {
            console.log("  " + fruit + ": $" + price);
        });

        for (const [key, value] of fruitPrices.entries()) {
            console.log("  " + key + " => " + value);
        }

        const set = new Set();
        set.add(1);
        set.add(2);
        set.add(3);
        set.add(2);

        const colors = new Set(["red", "green", "blue", "red", "yellow"]);

        for (const color of colors) {
            console.log("  " + color);
        }

        colors.forEach(color => {
            console.log("  Color: " + color);
        });

        // Part 2: set operations
        const setA = new Set([1, 2, 3, 4, 5]);
        const setB = new Set([4, 5, 6, 7, 8]);

        function union(a, b) {
            const result = new Set(a);
            for (const item of b) {
                result.add(item);
            }
            return result;
        }

        function intersection(a, b) {
            const result = new Set();
            for (const item of a) {
                if (b.has(item)) {
                    result.add(item);
                }
            }
            return result;
        }

        function difference(a, b) {
            const result = new Set();
            for (const item of a) {
                if (!b.has(item)) {
                    result.add(item);
                }
            }
            return result;
        }

        function symmetricDifference(a, b) {
            return union(difference(a, b), difference(b, a));
        }

        function isSubset(a, b) {
            for (const item of a) {
                if (!b.has(item)) {
                    return false;
                }
            }
            return true;
        }

        console.log("Union:", JSON.stringify(Array.from(union(setA, setB))));
        console.log("Intersection:", JSON.stringify(Array.from(intersection(setA, setB))));

        const setC = new Set([1, 2, 3]);
        console.log("Is C subset of A?", isSubset(setC, setA));

        "parts_1_2_done"
    "#,
    );

    match result {
        Ok(v) => assert_eq!(v, JsValue::from("parts_1_2_done")),
        Err(e) => panic!("Parts 1+2 failed: {}", e),
    }
}

#[test]
fn test_bytecode_collections_parts_1_2_3() {
    // Combine parts 1, 2, and 3
    let result = eval_bytecode_result(
        r#"
        // Part 1
        const map = new Map();
        map.set("one", 1);
        const fruitPrices = new Map([["apple", 1.5], ["banana", 0.75], ["orange", 2.0], ["grape", 3.25]]);
        const set = new Set();
        set.add(1);
        const colors = new Set(["red", "green", "blue", "red", "yellow"]);

        // Part 2
        const setA = new Set([1, 2, 3, 4, 5]);
        const setB = new Set([4, 5, 6, 7, 8]);

        function union(a, b) { const result = new Set(a); for (const item of b) result.add(item); return result; }
        function intersection(a, b) { const result = new Set(); for (const item of a) if (b.has(item)) result.add(item); return result; }
        function difference(a, b) { const result = new Set(); for (const item of a) if (!b.has(item)) result.add(item); return result; }
        function symmetricDifference(a, b) { return union(difference(a, b), difference(b, a)); }
        function isSubset(a, b) { for (const item of a) if (!b.has(item)) return false; return true; }

        const setC = new Set([1, 2, 3]);

        // Part 3
        function createGraph() { return { nodes: new Map() }; }
        function addNode(graph, node) { if (!graph.nodes.has(node)) graph.nodes.set(node, new Set()); }
        function addEdge(graph, from, to) { addNode(graph, from); addNode(graph, to); graph.nodes.get(from).add(to); }
        function getNeighbors(graph, node) { return graph.nodes.get(node) || new Set(); }

        function bfs(graph, start) {
            const visited = new Set();
            const result = [];
            const queue = [start];
            while (queue.length > 0) {
                const current = queue.shift();
                if (visited.has(current)) continue;
                visited.add(current);
                result.push(current);
                const neighbors = getNeighbors(graph, current);
                for (const neighbor of neighbors) if (!visited.has(neighbor)) queue.push(neighbor);
            }
            return result;
        }

        function dfs(graph, start) {
            const visited = new Set();
            const result = [];
            function visit(node) {
                if (visited.has(node)) return;
                visited.add(node);
                result.push(node);
                const neighbors = getNeighbors(graph, node);
                for (const neighbor of neighbors) visit(neighbor);
            }
            visit(start);
            return result;
        }

        const graph = createGraph();
        addEdge(graph, "A", "B");
        addEdge(graph, "A", "C");
        addEdge(graph, "B", "D");

        function createWordCounter() { return { frequencies: new Map(), uniqueWords: new Set(), totalWords: 0 }; }
        function addWord(counter, word) {
            const normalized = word.toLowerCase();
            counter.totalWords++;
            counter.uniqueWords.add(normalized);
            const count = counter.frequencies.get(normalized) || 0;
            counter.frequencies.set(normalized, count + 1);
        }
        function analyzeText(text) {
            const counter = createWordCounter();
            const words = text.split(/[\s,.!?;:'"()\[\]{}]+/);
            for (const word of words) if (word.length > 0) addWord(counter, word);
            return counter;
        }

        const text = "the quick brown fox";
        const counter = analyzeText(text);
        console.log("Unique words:", counter.uniqueWords.size);

        "parts_1_2_3_done"
    "#,
    );

    match result {
        Ok(v) => assert_eq!(v, JsValue::from("parts_1_2_3_done")),
        Err(e) => panic!("Parts 1+2+3 failed: {}", e),
    }
}

#[test]
fn test_bytecode_many_console_logs() {
    // Test if many console.log calls exhaust registers
    let result = eval_bytecode_result(
        r#"
        console.log("1");
        console.log("2");
        console.log("3");
        console.log("4");
        console.log("5");
        console.log("6");
        console.log("7");
        console.log("8");
        console.log("9");
        console.log("10");
        console.log("11");
        console.log("12");
        console.log("13");
        console.log("14");
        console.log("15");
        console.log("16");
        console.log("17");
        console.log("18");
        console.log("19");
        console.log("20");
        console.log("21");
        console.log("22");
        console.log("23");
        console.log("24");
        console.log("25");
        console.log("26");
        console.log("27");
        console.log("28");
        console.log("29");
        console.log("30");
        console.log("31");
        console.log("32");
        console.log("33");
        console.log("34");
        console.log("35");
        console.log("36");
        console.log("37");
        console.log("38");
        console.log("39");
        console.log("40");
        console.log("41");
        console.log("42");
        console.log("43");
        console.log("44");
        console.log("45");
        console.log("46");
        console.log("47");
        console.log("48");
        console.log("49");
        console.log("50");
        console.log("51");
        console.log("52");
        console.log("53");
        console.log("54");
        console.log("55");
        console.log("56");
        console.log("57");
        console.log("58");
        console.log("59");
        console.log("60");
        console.log("61");
        console.log("62");
        console.log("63");
        console.log("64");
        console.log("65");
        console.log("66");
        console.log("67");
        console.log("68");
        console.log("69");
        console.log("70");
        console.log("71");
        console.log("72");
        console.log("73");
        console.log("74");
        console.log("75");
        console.log("76");
        console.log("77");
        console.log("78");
        console.log("79");
        console.log("80");
        console.log("81");
        console.log("82");
        console.log("83");
        console.log("84");
        console.log("85");
        console.log("86");
        console.log("87");
        console.log("88");
        console.log("89");
        console.log("90");
        console.log("91");
        console.log("92");
        console.log("93");
        console.log("94");
        console.log("95");
        console.log("96");
        console.log("97");
        console.log("98");
        console.log("99");
        console.log("100");
        "done"
    "#,
    );

    match result {
        Ok(v) => assert_eq!(v, JsValue::from("done")),
        Err(e) => panic!("Many console logs failed: {}", e),
    }
}

#[test]
fn test_bytecode_collections_all_parts_compact() {
    // Combine all 4 parts in compact form
    let result = eval_bytecode_result(
        r#"
        // Part 1
        const map = new Map();
        map.set("one", 1);
        const fruitPrices = new Map([["apple", 1.5], ["banana", 0.75], ["orange", 2.0], ["grape", 3.25]]);
        const set = new Set();
        set.add(1);
        const colors = new Set(["red", "green", "blue", "red", "yellow"]);

        // Part 2
        const setA = new Set([1, 2, 3, 4, 5]);
        const setB = new Set([4, 5, 6, 7, 8]);

        function union(a, b) { const result = new Set(a); for (const item of b) result.add(item); return result; }
        function intersection(a, b) { const result = new Set(); for (const item of a) if (b.has(item)) result.add(item); return result; }
        function difference(a, b) { const result = new Set(); for (const item of a) if (!b.has(item)) result.add(item); return result; }
        function symmetricDifference(a, b) { return union(difference(a, b), difference(b, a)); }
        function isSubset(a, b) { for (const item of a) if (!b.has(item)) return false; return true; }

        const setC = new Set([1, 2, 3]);

        // Part 3
        function createGraph() { return { nodes: new Map() }; }
        function addNode(graph, node) { if (!graph.nodes.has(node)) graph.nodes.set(node, new Set()); }
        function addEdge(graph, from, to) { addNode(graph, from); addNode(graph, to); graph.nodes.get(from).add(to); }
        function getNeighbors(graph, node) { return graph.nodes.get(node) || new Set(); }

        function bfs(graph, start) {
            const visited = new Set();
            const result = [];
            const queue = [start];
            while (queue.length > 0) {
                const current = queue.shift();
                if (visited.has(current)) continue;
                visited.add(current);
                result.push(current);
                const neighbors = getNeighbors(graph, current);
                for (const neighbor of neighbors) if (!visited.has(neighbor)) queue.push(neighbor);
            }
            return result;
        }

        function dfs(graph, start) {
            const visited = new Set();
            const result = [];
            function visit(node) {
                if (visited.has(node)) return;
                visited.add(node);
                result.push(node);
                const neighbors = getNeighbors(graph, node);
                for (const neighbor of neighbors) visit(neighbor);
            }
            visit(start);
            return result;
        }

        const graph = createGraph();
        addEdge(graph, "A", "B");
        addEdge(graph, "A", "C");
        addEdge(graph, "B", "D");

        function createWordCounter() { return { frequencies: new Map(), uniqueWords: new Set(), totalWords: 0 }; }
        function addWord(counter, word) {
            const normalized = word.toLowerCase();
            counter.totalWords++;
            counter.uniqueWords.add(normalized);
            const count = counter.frequencies.get(normalized) || 0;
            counter.frequencies.set(normalized, count + 1);
        }
        function analyzeText(text) {
            const counter = createWordCounter();
            const words = text.split(/[\s,.!?;:'"()\[\]{}]+/);
            for (const word of words) if (word.length > 0) addWord(counter, word);
            return counter;
        }

        const text = "the quick brown fox";
        const counter = analyzeText(text);

        // Part 4 - conversions
        const arr = [1, 2, 2, 3, 3, 3, 4, 4, 4, 4];
        const uniqueSet = new Set(arr);

        const userMap = new Map([["alice", 30], ["bob", 25], ["charlie", 35]]);
        const userObj = {};
        userMap.forEach((value, key) => { userObj[key] = value; });

        const configObj = { host: "localhost", port: "8080", protocol: "https" };
        const configMap = new Map(Object.entries(configObj));

        const employees = [
            { name: "Alice", department: "Engineering", salary: 75000 },
            { name: "Bob", department: "Sales", salary: 60000 },
            { name: "Charlie", department: "Engineering", salary: 80000 },
            { name: "Diana", department: "Sales", salary: 65000 },
            { name: "Eve", department: "Marketing", salary: 55000 }
        ];

        function groupBy(items, keyFn) {
            const groups = new Map();
            for (const item of items) {
                const key = keyFn(item);
                if (!groups.has(key)) groups.set(key, []);
                groups.get(key).push(item);
            }
            return groups;
        }

        const byDepartment = groupBy(employees, e => e.department);

        const data = [
            { id: 1, value: "a" },
            { id: 2, value: "b" },
            { id: 1, value: "c" },
            { id: 3, value: "d" },
            { id: 2, value: "e" }
        ];

        const seenIds = new Set();
        const uniqueById = [];

        for (const item of data) {
            if (!seenIds.has(item.id)) {
                seenIds.add(item.id);
                uniqueById.push(item);
            }
        }

        console.log("All done:", uniqueById.length);
        "all_parts_done"
    "#,
    );

    match result {
        Ok(v) => assert_eq!(v, JsValue::from("all_parts_done")),
        Err(e) => panic!("All parts compact failed: {}", e),
    }
}

#[test]
fn test_bytecode_full_collections_main() {
    // Test ALL the collections/main.ts code (minus imports) to find register exhaustion
    let result = eval_bytecode_result(
        r#"
        console.log("=== Map/Set Collections Demo ===\n");

        // --- Basic Map Operations ---
        console.log("--- Basic Map Operations ---");

        const map = new Map();
        map.set("one", 1);
        map.set("two", 2);
        map.set("three", 3);

        console.log("Map size:", map.size);
        console.log("get('two'):", map.get("two"));
        console.log("has('three'):", map.has("three"));
        console.log("has('four'):", map.has("four"));

        // Map initialization with array of entries
        const fruitPrices = new Map([
            ["apple", 1.5],
            ["banana", 0.75],
            ["orange", 2.0],
            ["grape", 3.25]
        ]);
        console.log("\nFruit prices map:");
        fruitPrices.forEach((price, fruit) => {
            console.log("  " + fruit + ": $" + price);
        });

        // --- Map Iteration ---
        console.log("\n--- Map Iteration ---");

        console.log("\nMap entries:");
        for (const [key, value] of fruitPrices.entries()) {
            console.log("  " + key + " => " + value);
        }

        console.log("\nMap keys:", JSON.stringify(Array.from(fruitPrices.keys())));
        console.log("Map values:", JSON.stringify(Array.from(fruitPrices.values())));

        // --- Map Operations ---
        console.log("\n--- Map Operations ---");

        fruitPrices.set("mango", 2.5);
        console.log("After adding mango:", fruitPrices.size);

        fruitPrices.delete("banana");
        console.log("After deleting banana:", fruitPrices.size);
        console.log("has('banana'):", fruitPrices.has("banana"));

        // --- Basic Set Operations ---
        console.log("\n--- Basic Set Operations ---");

        const set = new Set();
        set.add(1);
        set.add(2);
        set.add(3);
        set.add(2);

        console.log("Set size (after adding 1, 2, 3, 2):", set.size);
        console.log("has(2):", set.has(2));
        console.log("has(4):", set.has(4));

        // Set initialization with array
        const colors = new Set(["red", "green", "blue", "red", "yellow"]);
        console.log("\nColors set size:", colors.size);
        console.log("Colors:", JSON.stringify(Array.from(colors)));

        // --- Set Iteration ---
        console.log("\n--- Set Iteration ---");

        console.log("Set values:");
        for (const color of colors) {
            console.log("  " + color);
        }

        // forEach
        console.log("\nUsing forEach:");
        colors.forEach(color => {
            console.log("  Color: " + color);
        });

        // --- Set Operations (Union, Intersection, Difference) ---
        console.log("\n--- Set Operations ---");

        const setA = new Set([1, 2, 3, 4, 5]);
        const setB = new Set([4, 5, 6, 7, 8]);

        // Union
        function union(a, b) {
            const result = new Set(a);
            for (const item of b) {
                result.add(item);
            }
            return result;
        }

        // Intersection
        function intersection(a, b) {
            const result = new Set();
            for (const item of a) {
                if (b.has(item)) {
                    result.add(item);
                }
            }
            return result;
        }

        // Difference (a - b)
        function difference(a, b) {
            const result = new Set();
            for (const item of a) {
                if (!b.has(item)) {
                    result.add(item);
                }
            }
            return result;
        }

        // Symmetric difference
        function symmetricDifference(a, b) {
            return union(difference(a, b), difference(b, a));
        }

        console.log("Set A:", JSON.stringify(Array.from(setA)));
        console.log("Set B:", JSON.stringify(Array.from(setB)));
        console.log("Union:", JSON.stringify(Array.from(union(setA, setB))));
        console.log("Intersection:", JSON.stringify(Array.from(intersection(setA, setB))));
        console.log("Difference (A - B):", JSON.stringify(Array.from(difference(setA, setB))));
        console.log("Symmetric Difference:", JSON.stringify(Array.from(symmetricDifference(setA, setB))));

        // Subset check
        function isSubset(a, b) {
            for (const item of a) {
                if (!b.has(item)) {
                    return false;
                }
            }
            return true;
        }

        const setC = new Set([1, 2, 3]);
        console.log("\nSet C:", JSON.stringify(Array.from(setC)));
        console.log("Is C subset of A?", isSubset(setC, setA));
        console.log("Is A subset of C?", isSubset(setA, setC));

        // --- Using Map for Caching ---
        console.log("\n--- Using Map for Caching ---");

        const cache = new Map();
        let computeCount = 0;

        function expensiveCompute(n) {
            if (cache.has(n)) {
                return cache.get(n);
            }
            computeCount++;
            const result = n * n + n;
            cache.set(n, result);
            return result;
        }

        console.log("Computing 5:", expensiveCompute(5));
        console.log("Computing 10:", expensiveCompute(10));
        console.log("Computing 5 (cached):", expensiveCompute(5));
        console.log("Computing 10 (cached):", expensiveCompute(10));
        console.log("Compute count (should be 2):", computeCount);
        console.log("Cache size:", cache.size);

        // --- Graph Example (inline) ---
        console.log("\n--- Graph with Map<node, Set<neighbor>> ---");

        function createGraph() {
            return { nodes: new Map() };
        }

        function addNode(graph, node) {
            if (!graph.nodes.has(node)) {
                graph.nodes.set(node, new Set());
            }
        }

        function addEdge(graph, from, to) {
            addNode(graph, from);
            addNode(graph, to);
            graph.nodes.get(from).add(to);
        }

        function getNeighbors(graph, node) {
            return graph.nodes.get(node) || new Set();
        }

        function bfs(graph, start) {
            const visited = new Set();
            const result = [];
            const queue = [start];

            while (queue.length > 0) {
                const current = queue.shift();

                if (visited.has(current)) {
                    continue;
                }

                visited.add(current);
                result.push(current);

                const neighbors = getNeighbors(graph, current);
                for (const neighbor of neighbors) {
                    if (!visited.has(neighbor)) {
                        queue.push(neighbor);
                    }
                }
            }

            return result;
        }

        function dfs(graph, start) {
            const visited = new Set();
            const result = [];

            function visit(node) {
                if (visited.has(node)) {
                    return;
                }

                visited.add(node);
                result.push(node);

                const neighbors = getNeighbors(graph, node);
                for (const neighbor of neighbors) {
                    visit(neighbor);
                }
            }

            visit(start);
            return result;
        }

        const graph = createGraph();
        addEdge(graph, "A", "B");
        addEdge(graph, "A", "C");
        addEdge(graph, "B", "D");
        addEdge(graph, "B", "E");
        addEdge(graph, "C", "F");

        console.log("\nGraph structure:");
        graph.nodes.forEach((neighbors, node) => {
            console.log("  " + node + " -> " + JSON.stringify(Array.from(neighbors)));
        });

        console.log("\nBFS from A:", JSON.stringify(bfs(graph, "A")));
        console.log("DFS from A:", JSON.stringify(dfs(graph, "A")));

        // --- Word Frequency Counter (inline) ---
        console.log("\n--- Word Frequency Counter ---");

        function createWordCounter() {
            return {
                frequencies: new Map(),
                uniqueWords: new Set(),
                totalWords: 0
            };
        }

        function addWord(counter, word) {
            const normalized = word.toLowerCase();
            counter.totalWords++;
            counter.uniqueWords.add(normalized);
            const count = counter.frequencies.get(normalized) || 0;
            counter.frequencies.set(normalized, count + 1);
        }

        function analyzeText(text) {
            const counter = createWordCounter();
            const words = text.split(/[\s,.!?;:'"()\[\]{}]+/);
            for (const word of words) {
                if (word.length > 0) {
                    addWord(counter, word);
                }
            }
            return counter;
        }

        const text = "the quick brown fox jumps over the lazy dog the fox is quick";
        const counter = analyzeText(text);

        console.log("\nWord frequencies:");
        const sorted = Array.from(counter.frequencies.entries())
            .sort((a, b) => b[1] - a[1]);
        for (const [word, count] of sorted) {
            console.log("  " + word + ": " + count);
        }

        console.log("\nUnique words:", counter.uniqueWords.size);
        console.log("Total words:", counter.totalWords);

        // --- Map with Complex Keys ---
        console.log("\n--- Map with Complex Keys ---");

        const pointMap = new Map();

        function pointKey(x, y) {
            return x + "," + y;
        }

        pointMap.set(pointKey(0, 0), "origin");
        pointMap.set(pointKey(1, 0), "unit-x");
        pointMap.set(pointKey(0, 1), "unit-y");
        pointMap.set(pointKey(1, 1), "diagonal");

        console.log("Point (0,0):", pointMap.get(pointKey(0, 0)));
        console.log("Point (1,1):", pointMap.get(pointKey(1, 1)));

        // --- Converting Between Collections ---
        console.log("\n--- Converting Between Collections ---");

        // Array to Set (removes duplicates)
        const arr = [1, 2, 2, 3, 3, 3, 4, 4, 4, 4];
        const uniqueSet = new Set(arr);
        console.log("Array:", JSON.stringify(arr));
        console.log("Unique values:", JSON.stringify(Array.from(uniqueSet)));

        // Map to Object
        const userMap = new Map([
            ["alice", 30],
            ["bob", 25],
            ["charlie", 35]
        ]);
        const userObj = {};
        userMap.forEach((value, key) => {
            userObj[key] = value;
        });
        console.log("\nMap to Object:", JSON.stringify(userObj));

        // Object to Map
        const configObj = {
            host: "localhost",
            port: "8080",
            protocol: "https"
        };
        const configMap = new Map(Object.entries(configObj));
        console.log("Object to Map size:", configMap.size);
        configMap.forEach((value, key) => {
            console.log("  " + key + " = " + value);
        });

        // --- Practical Example: Grouping ---
        console.log("\n--- Grouping with Map ---");

        const employees = [
            { name: "Alice", department: "Engineering", salary: 75000 },
            { name: "Bob", department: "Sales", salary: 60000 },
            { name: "Charlie", department: "Engineering", salary: 80000 },
            { name: "Diana", department: "Sales", salary: 65000 },
            { name: "Eve", department: "Marketing", salary: 55000 }
        ];

        function groupBy(items, keyFn) {
            const groups = new Map();
            for (const item of items) {
                const key = keyFn(item);
                if (!groups.has(key)) {
                    groups.set(key, []);
                }
                groups.get(key).push(item);
            }
            return groups;
        }

        const byDepartment = groupBy(employees, e => e.department);
        console.log("Employees by department:");
        byDepartment.forEach((people, dept) => {
            const names = people.map(p => p.name).join(", ");
            console.log("  " + dept + ": " + names);
        });

        // --- Set for Deduplication and Filtering ---
        console.log("\n--- Set for Deduplication ---");

        const data = [
            { id: 1, value: "a" },
            { id: 2, value: "b" },
            { id: 1, value: "c" },
            { id: 3, value: "d" },
            { id: 2, value: "e" }
        ];

        const seenIds = new Set();
        const uniqueById = [];

        for (const item of data) {
            if (!seenIds.has(item.id)) {
                seenIds.add(item.id);
                uniqueById.push(item);
            }
        }

        console.log("Original data length:", data.length);
        console.log("Unique by id length:", uniqueById.length);
        console.log("Unique items:", JSON.stringify(uniqueById));

        console.log("\n=== Demo Complete ===");
        "all_done"
    "#,
    );

    match result {
        Ok(v) => assert_eq!(v, JsValue::from("all_done")),
        Err(e) => panic!("Full collections main failed: {}", e),
    }
}
