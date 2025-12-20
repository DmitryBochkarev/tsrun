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
        eval_bytecode("let sum: number = 0; let i: number = 1; while (i <= 10) { sum = sum + i; i = i + 1; } sum"),
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
        eval_bytecode("let sum: number = 0; for (let i: number = 0; i < 10; i = i + 1) { if (i % 2 !== 0) continue; sum = sum + i; } sum"),
        JsValue::Number(20.0) // 0+2+4+6+8 = 20
    );
}

#[test]
fn test_bytecode_for_break() {
    assert_eq!(
        eval_bytecode("let sum: number = 0; for (let i: number = 0; i < 100; i = i + 1) { if (i >= 5) break; sum = sum + i; } sum"),
        JsValue::Number(10.0) // 0+1+2+3+4 = 10
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Control Flow - Switch Statement
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bytecode_switch_match() {
    assert_eq!(
        eval_bytecode("let x: number = 2; let result: string = ''; switch (x) { case 1: result = 'one'; break; case 2: result = 'two'; break; case 3: result = 'three'; break; } result"),
        JsValue::String("two".into())
    );
}

#[test]
fn test_bytecode_switch_default() {
    assert_eq!(
        eval_bytecode("let x: number = 99; let result: string = ''; switch (x) { case 1: result = 'one'; break; default: result = 'other'; break; } result"),
        JsValue::String("other".into())
    );
}

#[test]
fn test_bytecode_switch_fallthrough() {
    assert_eq!(
        eval_bytecode("let x: number = 1; let result: number = 0; switch (x) { case 1: result = result + 1; case 2: result = result + 2; break; case 3: result = result + 3; break; } result"),
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
        eval_bytecode("let result: string = ''; try { throw 'error'; result = 'try'; } catch (e) { result = 'catch'; } result"),
        JsValue::String("catch".into())
    );
}

#[test]
fn test_bytecode_throw_and_catch() {
    assert_eq!(
        eval_bytecode("let result: string = ''; try { throw 'test error'; } catch (e) { result = e as string; } result"),
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
        eval_bytecode("let result: number = 0; try { throw 'error'; } catch (e) { result = 1; } finally { result = result + 10; } result"),
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
