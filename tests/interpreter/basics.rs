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

// Tagged template literals
#[test]
#[ignore] // TODO: Fix - returns "Hello w!" instead of "Hello world!"
fn test_tagged_template_basic() {
    // Tag function receives strings array and values
    assert_eq!(
        eval(r#"
            function tag(strings: any, ...values: any): string {
                return strings[0] + values[0] + strings[1];
            }
            const name: string = "world";
            tag`Hello ${name}!`
        "#),
        JsValue::String("Hello world!".into())
    );
}

#[test]
fn test_tagged_template_no_substitution() {
    // Tag function with no interpolations
    assert_eq!(
        eval(r#"
            function tag(strings: any): string {
                return strings[0];
            }
            tag`hello`
        "#),
        JsValue::String("hello".into())
    );
}

#[test]
#[ignore] // TODO: Fix - syntax error with multiple substitutions
fn test_tagged_template_multiple_values() {
    // Tag function with multiple interpolated values
    assert_eq!(
        eval(r#"
            function join(strings: any, ...values: any): string {
                let result: string = "";
                for (let i: number = 0; i < strings.length; i = i + 1) {
                    result = result + strings[i];
                    if (i < values.length) {
                        result = result + values[i];
                    }
                }
                return result;
            }
            const a: number = 1;
            const b: number = 2;
            const c: number = 3;
            join`${a} + ${b} = ${c}`
        "#),
        JsValue::String("1 + 2 = 3".into())
    );
}

#[test]
fn test_tagged_template_raw() {
    // Tag function can access raw strings via strings.raw
    assert_eq!(
        eval(r#"
            function getRaw(strings: any): string {
                return strings.raw[0];
            }
            getRaw`hello`
        "#),
        JsValue::String("hello".into())
    );
}

// Simple tagged template tests to verify basic functionality
#[test]
fn test_tagged_template_strings_length() {
    // Verify strings array has correct length
    assert_eq!(
        eval(r#"
            function tag(strings: any): number {
                return strings.length;
            }
            tag`hello`
        "#),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_tagged_template_with_one_substitution_strings() {
    // With one substitution, strings array should have 2 elements
    assert_eq!(
        eval(r#"
            function tag(strings: any): number {
                return strings.length;
            }
            const x: number = 1;
            tag`a${x}b`
        "#),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_tagged_template_first_string() {
    // Verify first string in array
    assert_eq!(
        eval(r#"
            function tag(strings: any): string {
                return strings[0];
            }
            const x: number = 1;
            tag`hello${x}world`
        "#),
        JsValue::String("hello".into())
    );
}

#[test]
fn test_tagged_template_second_string() {
    // Verify second string in array
    assert_eq!(
        eval(r#"
            function tag(strings: any): string {
                return strings[1];
            }
            const x: number = 1;
            tag`hello${x}world`
        "#),
        JsValue::String("world".into())
    );
}

#[test]
fn test_tagged_template_value_passed() {
    // Verify the substituted value is passed correctly
    assert_eq!(
        eval(r#"
            function tag(strings: any, val: any): number {
                return val;
            }
            const x: number = 42;
            tag`test${x}end`
        "#),
        JsValue::Number(42.0)
    );
}