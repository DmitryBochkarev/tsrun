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
    assert_eq!(
        eval("(1 as number) + (2 as number) * (3 as number)"),
        JsValue::Number(7.0)
    );
    assert_eq!(
        eval("((1 as number) + (2 as number)) * (3 as number)"),
        JsValue::Number(9.0)
    );
}

#[test]
fn test_comparison() {
    assert_eq!(
        eval("(1 as number) < (2 as number)"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("(2 as number) > (1 as number)"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("(1 as number) === (1 as number)"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("(1 as number) !== (2 as number)"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_variables() {
    assert_eq!(eval("let x: number = 5; x"), JsValue::Number(5.0));
    assert_eq!(eval("let x: number = 5; x = 10; x"), JsValue::Number(10.0));
}

#[test]
fn test_conditional() {
    assert_eq!(
        eval("(true as boolean) ? (1 as number) : (2 as number)"),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval("(false as boolean) ? (1 as number) : (2 as number)"),
        JsValue::Number(2.0)
    );
}

// Bitwise operators
#[test]
fn test_bitwise_shift() {
    // Left shift
    assert_eq!(
        eval("(8 as number) << (2 as number)"),
        JsValue::Number(32.0)
    );
    // Right shift (signed)
    assert_eq!(
        eval("(32 as number) >> (2 as number)"),
        JsValue::Number(8.0)
    );
    // Right shift preserves sign for negative numbers
    assert_eq!(
        eval("((-8 as number) >> (2 as number))"),
        JsValue::Number(-2.0)
    );
}

#[test]
fn test_unsigned_right_shift() {
    // Unsigned right shift (>>>)
    assert_eq!(
        eval("(32 as number) >>> (2 as number)"),
        JsValue::Number(8.0)
    );
    // Unsigned right shift converts to unsigned 32-bit first
    assert_eq!(
        eval("((-1 as number) >>> (0 as number))"),
        JsValue::Number(4294967295.0)
    );
    // Unsigned right shift on negative numbers
    assert_eq!(
        eval("((-8 as number) >>> (2 as number))"),
        JsValue::Number(1073741822.0)
    );
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

// Update expressions (++, --)
#[test]
fn test_update_prefix_increment() {
    assert_eq!(eval("let x: number = 5; ++x"), JsValue::Number(6.0));
    assert_eq!(eval("let x: number = 5; ++x; x"), JsValue::Number(6.0));
}

#[test]
fn test_update_postfix_increment() {
    assert_eq!(eval("let x: number = 5; x++"), JsValue::Number(5.0)); // Returns old value
    assert_eq!(eval("let x: number = 5; x++; x"), JsValue::Number(6.0)); // But x is updated
}

#[test]
fn test_update_prefix_decrement() {
    assert_eq!(eval("let x: number = 5; --x"), JsValue::Number(4.0));
    assert_eq!(eval("let x: number = 5; --x; x"), JsValue::Number(4.0));
}

#[test]
fn test_update_postfix_decrement() {
    assert_eq!(eval("let x: number = 5; x--"), JsValue::Number(5.0)); // Returns old value
    assert_eq!(eval("let x: number = 5; x--; x"), JsValue::Number(4.0)); // But x is updated
}

#[test]
fn test_update_in_for_loop() {
    // Classic for loop with i++
    assert_eq!(
        eval("let sum: number = 0; for (let i: number = 0; i < 5; i++) { sum = sum + i; } sum"),
        JsValue::Number(10.0) // 0 + 1 + 2 + 3 + 4 = 10
    );
}

#[test]
fn test_update_member_expression() {
    assert_eq!(
        eval("let obj: any = { x: 5 }; obj.x++; obj.x"),
        JsValue::Number(6.0)
    );
    assert_eq!(
        eval("let arr: number[] = [1, 2, 3]; arr[0]++; arr[0]"),
        JsValue::Number(2.0)
    );
}

// Sequence expressions (comma operator)
#[test]
fn test_sequence_expression() {
    // Sequence expression returns the last value
    assert_eq!(eval("(1, 2, 3)"), JsValue::Number(3.0));
    assert_eq!(
        eval("let x: number = 0; (x = 1, x = 2, x = 3); x"),
        JsValue::Number(3.0)
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
    assert_eq!(
        eval("(100n as number) + (200n as number)"),
        JsValue::Number(300.0)
    );
}

#[test]
fn test_bigint_variable() {
    assert_eq!(eval("const n: bigint = 42n; n"), JsValue::Number(42.0));
}

// Tagged template literals
#[test]
fn test_tagged_template_basic() {
    // Tag function receives strings array and values
    assert_eq!(
        eval(
            r#"
            function tag(strings: any, ...values: any): string {
                return strings[0] + values[0] + strings[1];
            }
            const name: string = "world";
            tag`Hello ${name}!`
        "#
        ),
        JsValue::String("Hello world!".into())
    );
}

#[test]
fn test_tagged_template_no_substitution() {
    // Tag function with no interpolations
    assert_eq!(
        eval(
            r#"
            function tag(strings: any): string {
                return strings[0];
            }
            tag`hello`
        "#
        ),
        JsValue::String("hello".into())
    );
}

#[test]
fn test_tagged_template_multiple_values() {
    // Tag function with multiple interpolated values
    assert_eq!(
        eval(
            r#"
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
        "#
        ),
        JsValue::String("1 + 2 = 3".into())
    );
}

#[test]
fn test_tagged_template_raw() {
    // Tag function can access raw strings via strings.raw
    assert_eq!(
        eval(
            r#"
            function getRaw(strings: any): string {
                return strings.raw[0];
            }
            getRaw`hello`
        "#
        ),
        JsValue::String("hello".into())
    );
}

// Simple tagged template tests to verify basic functionality
#[test]
fn test_tagged_template_strings_length() {
    // Verify strings array has correct length
    assert_eq!(
        eval(
            r#"
            function tag(strings: any): number {
                return strings.length;
            }
            tag`hello`
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_tagged_template_with_one_substitution_strings() {
    // With one substitution, strings array should have 2 elements
    assert_eq!(
        eval(
            r#"
            function tag(strings: any): number {
                return strings.length;
            }
            const x: number = 1;
            tag`a${x}b`
        "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_tagged_template_first_string() {
    // Verify first string in array
    assert_eq!(
        eval(
            r#"
            function tag(strings: any): string {
                return strings[0];
            }
            const x: number = 1;
            tag`hello${x}world`
        "#
        ),
        JsValue::String("hello".into())
    );
}

#[test]
fn test_tagged_template_second_string() {
    // Verify second string in array
    assert_eq!(
        eval(
            r#"
            function tag(strings: any): string {
                return strings[1];
            }
            const x: number = 1;
            tag`hello${x}world`
        "#
        ),
        JsValue::String("world".into())
    );
}

#[test]
fn test_tagged_template_value_passed() {
    // Verify the substituted value is passed correctly
    assert_eq!(
        eval(
            r#"
            function tag(strings: any, val: any): number {
                return val;
            }
            const x: number = 42;
            tag`test${x}end`
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_tagged_template_two_substitutions() {
    // Test with two substitutions
    assert_eq!(
        eval(
            r#"
            function tag(strings: any, a: any, b: any): string {
                return strings[0] + a + strings[1] + b + strings[2];
            }
            const x: number = 1;
            const y: number = 2;
            tag`${x}+${y}`
        "#
        ),
        JsValue::String("1+2".into())
    );
}

// Destructuring assignment tests
#[test]
fn test_destructuring_assignment_array() {
    assert_eq!(
        eval(
            r#"
            let a: number, b: number;
            [a, b] = [1, 2];
            a + b
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_destructuring_assignment_array_with_rest() {
    assert_eq!(
        eval(
            r#"
            let first: number, rest: number[];
            [first, ...rest] = [1, 2, 3, 4];
            rest.length
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_destructuring_assignment_object() {
    assert_eq!(
        eval(
            r#"
            let x: number, y: number;
            ({ x, y } = { x: 10, y: 20 });
            x + y
        "#
        ),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_destructuring_assignment_object_rename() {
    assert_eq!(
        eval(
            r#"
            let a: number, b: number;
            ({ x: a, y: b } = { x: 5, y: 15 });
            a + b
        "#
        ),
        JsValue::Number(20.0)
    );
}

// Temporal Dead Zone (TDZ) tests
#[test]
fn test_tdz_let_access_before_declaration() {
    // Accessing let variable before declaration should throw ReferenceError
    use super::throws_error;
    assert!(throws_error(
        r#"
        {
            console.log(x);
            let x: number = 5;
        }
        "#,
        "ReferenceError"
    ));
}

#[test]
fn test_tdz_const_access_before_declaration() {
    // Accessing const variable before declaration should throw ReferenceError
    use super::throws_error;
    assert!(throws_error(
        r#"
        {
            const y: number = x;
            const x: number = 5;
        }
        "#,
        "ReferenceError"
    ));
}

#[test]
fn test_tdz_var_hoisting_works() {
    // var should be hoisted and accessible (as undefined) before declaration
    assert_eq!(
        eval(
            r#"
            function test(): any {
                const before: any = x;
                var x: number = 5;
                return before;
            }
            test()
        "#
        ),
        JsValue::Undefined
    );
}

#[test]
fn test_tdz_let_after_declaration_works() {
    // Accessing let after declaration should work
    assert_eq!(
        eval(
            r#"
            {
                let x: number = 10;
                x
            }
        "#
        ),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_tdz_function_can_reference_later_let() {
    // Function defined before let can reference it if called after
    assert_eq!(
        eval(
            r#"
            {
                function getX(): number { return x; }
                let x: number = 42;
                getX()
            }
        "#
        ),
        JsValue::Number(42.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Bitwise operations
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bitwise_operations() {
    // Test bitwise operators
    assert_eq!(eval("12 & 10"), JsValue::Number(8.0));
    assert_eq!(eval("12 | 10"), JsValue::Number(14.0));
    assert_eq!(eval("12 ^ 10"), JsValue::Number(6.0));
    assert_eq!(eval("5 << 2"), JsValue::Number(20.0));
    assert_eq!(eval("20 >> 2"), JsValue::Number(5.0));
    assert_eq!(eval("~5"), JsValue::Number(-6.0));
    assert_eq!(eval("-20 >>> 2"), JsValue::Number(1073741819.0));
}

#[test]
fn test_array_function_call() {
    // Test storing function via push
    assert_eq!(
        eval(
            r#"
            let funcs: any[] = [];
            funcs.push(function(): number { return 42; });
            typeof funcs[0]
        "#
        ),
        JsValue::String("function".into())
    );
}

#[test]
fn test_array_function_call_simple() {
    // Simpler test: direct assignment and call
    assert_eq!(
        eval(
            r#"
            let fn = function(): number { return 42; };
            let funcs: any[] = [fn];
            funcs[0]()
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_closure_object_method() {
    // Test that closures with method properties work
    assert_eq!(
        eval(
            r#"
            function makeCounter(): any {
                let count: number = 0;
                return {
                    inc: function(): number {
                        count = count + 1;
                        return count;
                    }
                };
            }
            let c: any = makeCounter();
            c.inc() + c.inc()
        "#
        ),
        JsValue::Number(3.0) // 1 + 2
    );
}
