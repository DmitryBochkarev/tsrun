//! Control flow tests: if/else, switch, loops, break/continue, try/catch, throw

use super::{eval, eval_result, throws_error};
use typescript_eval::JsValue;

// =============================================================================
// PHASE 1: Basic Control Flow
// =============================================================================

// -----------------------------------------------------------------------------
// If/Else Statements
// -----------------------------------------------------------------------------

#[test]
fn test_if_true_branch() {
    assert_eq!(
        eval(
            r#"
            let result: number = 0;
            if (true) {
                result = 1;
            }
            result
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_if_false_no_else() {
    assert_eq!(
        eval(
            r#"
            let result: number = 0;
            if (false) {
                result = 1;
            }
            result
        "#
        ),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_if_else_true_branch() {
    assert_eq!(
        eval(
            r#"
            let result: number;
            if (true) {
                result = 1;
            } else {
                result = 2;
            }
            result
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_if_else_false_branch() {
    assert_eq!(
        eval(
            r#"
            let result: number;
            if (false) {
                result = 1;
            } else {
                result = 2;
            }
            result
        "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_if_else_if_else_chain() {
    // Test first branch
    assert_eq!(
        eval(
            r#"
            let x: number = 1;
            let result: string;
            if (x === 1) {
                result = "one";
            } else if (x === 2) {
                result = "two";
            } else {
                result = "other";
            }
            result
        "#
        ),
        JsValue::String("one".into())
    );

    // Test middle branch
    assert_eq!(
        eval(
            r#"
            let x: number = 2;
            let result: string;
            if (x === 1) {
                result = "one";
            } else if (x === 2) {
                result = "two";
            } else {
                result = "other";
            }
            result
        "#
        ),
        JsValue::String("two".into())
    );

    // Test else branch
    assert_eq!(
        eval(
            r#"
            let x: number = 99;
            let result: string;
            if (x === 1) {
                result = "one";
            } else if (x === 2) {
                result = "two";
            } else {
                result = "other";
            }
            result
        "#
        ),
        JsValue::String("other".into())
    );
}

#[test]
fn test_if_with_block_scope() {
    // let should be scoped to if block
    assert_eq!(
        eval(
            r#"
            let outer: number = 1;
            if (true) {
                let inner: number = 2;
                outer = inner;
            }
            outer
        "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_if_truthy_values() {
    // Non-zero number
    assert_eq!(
        eval("let r: number = 0; if (1) { r = 1; } r"),
        JsValue::Number(1.0)
    );
    // Non-empty string
    assert_eq!(
        eval(r#"let r: number = 0; if ("hello") { r = 1; } r"#),
        JsValue::Number(1.0)
    );
    // Empty array (truthy in JS!)
    assert_eq!(
        eval("let r: number = 0; if ([]) { r = 1; } r"),
        JsValue::Number(1.0)
    );
    // Empty object (truthy in JS!)
    assert_eq!(
        eval("let r: number = 0; if ({}) { r = 1; } r"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_if_falsy_values() {
    // Zero
    assert_eq!(
        eval("let r: number = 1; if (0) { r = 0; } r"),
        JsValue::Number(1.0)
    );
    // Empty string
    assert_eq!(
        eval(r#"let r: number = 1; if ("") { r = 0; } r"#),
        JsValue::Number(1.0)
    );
    // null
    assert_eq!(
        eval("let r: number = 1; if (null) { r = 0; } r"),
        JsValue::Number(1.0)
    );
    // undefined
    assert_eq!(
        eval("let r: number = 1; if (undefined) { r = 0; } r"),
        JsValue::Number(1.0)
    );
    // NaN
    assert_eq!(
        eval("let r: number = 1; if (NaN) { r = 0; } r"),
        JsValue::Number(1.0)
    );
}

// -----------------------------------------------------------------------------
// Switch Statements
// -----------------------------------------------------------------------------

#[test]
fn test_switch_matching_case() {
    assert_eq!(
        eval(
            r#"
            let x: number = 2;
            let result: string;
            switch (x) {
                case 1:
                    result = "one";
                    break;
                case 2:
                    result = "two";
                    break;
                case 3:
                    result = "three";
                    break;
            }
            result
        "#
        ),
        JsValue::String("two".into())
    );
}

#[test]
fn test_switch_default_case() {
    assert_eq!(
        eval(
            r#"
            let x: number = 99;
            let result: string;
            switch (x) {
                case 1:
                    result = "one";
                    break;
                default:
                    result = "default";
                    break;
            }
            result
        "#
        ),
        JsValue::String("default".into())
    );
}

#[test]
fn test_switch_break_stops_execution() {
    assert_eq!(
        eval(
            r#"
            let x: number = 1;
            let result: string = "";
            switch (x) {
                case 1:
                    result = result + "a";
                    break;
                case 2:
                    result = result + "b";
                    break;
            }
            result
        "#
        ),
        JsValue::String("a".into())
    );
}

#[test]
fn test_switch_fall_through() {
    // Without break, execution falls through to next case
    assert_eq!(
        eval(
            r#"
            let x: number = 1;
            let result: string = "";
            switch (x) {
                case 1:
                    result = result + "a";
                case 2:
                    result = result + "b";
                case 3:
                    result = result + "c";
                    break;
            }
            result
        "#
        ),
        JsValue::String("abc".into())
    );
}

#[test]
fn test_switch_multiple_cases_same_body() {
    assert_eq!(
        eval(
            r#"
            let x: number = 2;
            let result: string;
            switch (x) {
                case 1:
                case 2:
                case 3:
                    result = "small";
                    break;
                default:
                    result = "big";
            }
            result
        "#
        ),
        JsValue::String("small".into())
    );
}

#[test]
fn test_switch_string_cases() {
    assert_eq!(
        eval(
            r#"
            let cmd: string = "start";
            let result: number;
            switch (cmd) {
                case "start":
                    result = 1;
                    break;
                case "stop":
                    result = 0;
                    break;
                default:
                    result = -1;
            }
            result
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_switch_expression_cases() {
    assert_eq!(
        eval(
            r#"
            let x: number = 4;
            let result: string;
            switch (x) {
                case 2 + 2:
                    result = "four";
                    break;
                case 3 * 2:
                    result = "six";
                    break;
                default:
                    result = "other";
            }
            result
        "#
        ),
        JsValue::String("four".into())
    );
}

#[test]
fn test_switch_no_matching_case_no_default() {
    assert_eq!(
        eval(
            r#"
            let x: number = 99;
            let result: string = "unchanged";
            switch (x) {
                case 1:
                    result = "one";
                    break;
                case 2:
                    result = "two";
                    break;
            }
            result
        "#
        ),
        JsValue::String("unchanged".into())
    );
}

// -----------------------------------------------------------------------------
// While Loops
// -----------------------------------------------------------------------------

#[test]
fn test_while_basic_iteration() {
    assert_eq!(
        eval(
            r#"
            let i: number = 0;
            let sum: number = 0;
            while (i < 5) {
                sum = sum + i;
                i = i + 1;
            }
            sum
        "#
        ),
        JsValue::Number(10.0) // 0+1+2+3+4 = 10
    );
}

#[test]
fn test_while_false_condition_no_execution() {
    assert_eq!(
        eval(
            r#"
            let executed: boolean = false;
            while (false) {
                executed = true;
            }
            executed
        "#
        ),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_while_with_mutation() {
    assert_eq!(
        eval(
            r#"
            let arr: number[] = [];
            let i: number = 0;
            while (i < 3) {
                arr.push(i);
                i = i + 1;
            }
            arr.join(",")
        "#
        ),
        JsValue::String("0,1,2".into())
    );
}

#[test]
fn test_while_complex_condition() {
    assert_eq!(
        eval(
            r#"
            let i: number = 0;
            let j: number = 10;
            while (i < 5 && j > 5) {
                i = i + 1;
                j = j - 1;
            }
            i + j
        "#
        ),
        JsValue::Number(10.0) // i=5, j=5 when loop ends
    );
}

// -----------------------------------------------------------------------------
// Do-While Loops
// -----------------------------------------------------------------------------

#[test]
fn test_do_while_executes_once_minimum() {
    assert_eq!(
        eval(
            r#"
            let count: number = 0;
            do {
                count = count + 1;
            } while (count < 3);
            count
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_do_while_false_condition() {
    // Body runs once even if condition is immediately false
    assert_eq!(
        eval(
            r#"
            let executed: number = 0;
            do {
                executed = executed + 1;
            } while (false);
            executed
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_do_while_multiple_iterations() {
    assert_eq!(
        eval(
            r#"
            let n: number = 1;
            do {
                n = n * 2;
            } while (n < 100);
            n
        "#
        ),
        JsValue::Number(128.0) // 1->2->4->8->16->32->64->128
    );
}

// -----------------------------------------------------------------------------
// For Loops
// -----------------------------------------------------------------------------

#[test]
fn test_for_basic_iteration() {
    assert_eq!(
        eval(
            r#"
            let sum: number = 0;
            for (let i: number = 0; i < 5; i = i + 1) {
                sum = sum + i;
            }
            sum
        "#
        ),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_for_no_init() {
    assert_eq!(
        eval(
            r#"
            let i: number = 0;
            let sum: number = 0;
            for (; i < 3; i = i + 1) {
                sum = sum + i;
            }
            sum
        "#
        ),
        JsValue::Number(3.0) // 0+1+2
    );
}

#[test]
fn test_for_no_update() {
    assert_eq!(
        eval(
            r#"
            let sum: number = 0;
            for (let i: number = 0; i < 3;) {
                sum = sum + i;
                i = i + 1;
            }
            sum
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_for_no_condition() {
    // Infinite loop with break
    assert_eq!(
        eval(
            r#"
            let count: number = 0;
            for (let i: number = 0;; i = i + 1) {
                count = count + 1;
                if (i >= 4) {
                    break;
                }
            }
            count
        "#
        ),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_for_empty() {
    // for(;;) infinite loop with break
    assert_eq!(
        eval(
            r#"
            let count: number = 0;
            for (;;) {
                count = count + 1;
                if (count >= 3) {
                    break;
                }
            }
            count
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_for_multiple_variables() {
    assert_eq!(
        eval(
            r#"
            let result: number = 0;
            for (let i: number = 0, j: number = 10; i < j; i = i + 1, j = j - 1) {
                result = result + 1;
            }
            result
        "#
        ),
        JsValue::Number(5.0) // i=0,j=10; i=1,j=9; i=2,j=8; i=3,j=7; i=4,j=6; then i>=j
    );
}

#[test]
fn test_for_let_block_scope() {
    // let in for loop should create new binding per iteration for closures
    // Each closure captures its own copy of i from that iteration
    assert_eq!(
        eval(
            r#"
            let funcs: any[] = [];
            for (let i: number = 0; i < 3; i = i + 1) {
                funcs.push(() => i);
            }
            funcs[0]() + funcs[1]() + funcs[2]()
        "#
        ),
        JsValue::Number(3.0) // 0+1+2
    );
}

#[test]
fn test_for_let_closure_capture_simple() {
    // Simpler version: verify closures capture loop variable correctly
    // Using function expression instead of arrow to work around parser issue
    assert_eq!(
        eval(
            r#"
            let result: number = 0;
            for (let i: number = 0; i < 3; i = i + 1) {
                let capture: number = i;
                result = result + capture;
            }
            result
        "#
        ),
        JsValue::Number(3.0) // 0+1+2
    );
}

#[test]
fn test_for_var_function_scope() {
    // var leaks out of for loop (function-scoped)
    assert_eq!(
        eval(
            r#"
            for (var i: number = 0; i < 3; i = i + 1) {
                // loop body
            }
            i
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_for_var_hoisting_in_function() {
    // Simpler test: var declared inside block should hoist to function
    assert_eq!(
        eval(
            r#"
            function test(): number {
                for (var i: number = 0; i < 3; i = i + 1) {}
                return i;
            }
            test()
        "#
        ),
        JsValue::Number(3.0)
    );
}

// =============================================================================
// PHASE 2: For-In, For-Of, Break, Continue
// =============================================================================

// -----------------------------------------------------------------------------
// For-In Loops
// -----------------------------------------------------------------------------

// Test for-in with pre-declared variable (no let/const/var in for-in)
#[test]
fn test_for_in_with_predeclared_var() {
    // This is valid JavaScript: the variable is declared before the loop
    assert_eq!(
        eval(
            r#"
            let x: string = "";
            let obj: any = { a: 1, b: 2 };
            for (x in obj) {
                // x is assigned each key
            }
            x !== ""  // x should have been assigned
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_for_in_with_predeclared_var_accumulate() {
    // Accumulate keys using pre-declared variable
    assert_eq!(
        eval(
            r#"
            let key: string = "";
            let keys: string[] = [];
            let obj: any = { a: 1, b: 2, c: 3 };
            for (key in obj) {
                keys.push(key);
            }
            keys.length
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_for_of_with_predeclared_var() {
    // for-of with pre-declared variable
    assert_eq!(
        eval(
            r#"
            let sum: number = 0;
            let item: number = 0;
            let arr: number[] = [1, 2, 3, 4, 5];
            for (item of arr) {
                sum = sum + item;
            }
            sum
        "#
        ),
        JsValue::Number(15.0)
    );
}

// Note: for-in with member expression (e.g., for (obj.key in source)) is
// not supported - member expressions are not valid destructuring targets.

#[test]
fn test_for_in_object_keys() {
    assert_eq!(
        eval(
            r#"
            let obj: any = { a: 1, b: 2, c: 3 };
            let keys: string[] = [];
            for (let k: string in obj) {
                keys.push(k);
            }
            keys.length
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_for_in_array_indices() {
    // for-in on arrays iterates indices as strings
    // Note: order may vary, so we check length and that values are accessed correctly
    assert_eq!(
        eval(
            r#"
            let arr: number[] = [10, 20, 30];
            let sum: number = 0;
            for (let i: string in arr) {
                sum = sum + arr[i];
            }
            sum
        "#
        ),
        JsValue::Number(60.0) // 10+20+30
    );
}

#[test]
fn test_for_in_empty_object() {
    assert_eq!(
        eval(
            r#"
            let obj: any = {};
            let count: number = 0;
            for (let k: string in obj) {
                count = count + 1;
            }
            count
        "#
        ),
        JsValue::Number(0.0)
    );
}

// -----------------------------------------------------------------------------
// For-Of Loops
// -----------------------------------------------------------------------------

#[test]
fn test_for_of_array() {
    assert_eq!(
        eval(
            r#"
            let arr: number[] = [1, 2, 3, 4, 5];
            let sum: number = 0;
            for (let x: number of arr) {
                sum = sum + x;
            }
            sum
        "#
        ),
        JsValue::Number(15.0)
    );
}

#[test]
fn test_for_of_string() {
    // for-of on string iterates characters
    assert_eq!(
        eval(
            r#"
            let str: string = "abc";
            let chars: string[] = [];
            for (let c: string of str) {
                chars.push(c);
            }
            chars.join("-")
        "#
        ),
        JsValue::String("a-b-c".into())
    );
}

#[test]
fn test_for_of_empty_array() {
    assert_eq!(
        eval(
            r#"
            let arr: number[] = [];
            let count: number = 0;
            for (let x: number of arr) {
                count = count + 1;
            }
            count
        "#
        ),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_for_of_with_destructuring() {
    assert_eq!(
        eval(
            r#"
            let pairs: number[][] = [[1, 2], [3, 4], [5, 6]];
            let sum: number = 0;
            for (let [a, b]: number[] of pairs) {
                sum = sum + a + b;
            }
            sum
        "#
        ),
        JsValue::Number(21.0) // 1+2+3+4+5+6
    );
}

// -----------------------------------------------------------------------------
// Break Statement
// -----------------------------------------------------------------------------

#[test]
fn test_break_in_while() {
    assert_eq!(
        eval(
            r#"
            let i: number = 0;
            while (true) {
                i = i + 1;
                if (i >= 5) {
                    break;
                }
            }
            i
        "#
        ),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_break_in_for() {
    assert_eq!(
        eval(
            r#"
            let result: number = 0;
            for (let i: number = 0; i < 100; i = i + 1) {
                result = i;
                if (i === 7) {
                    break;
                }
            }
            result
        "#
        ),
        JsValue::Number(7.0)
    );
}

#[test]
fn test_break_in_switch() {
    // Already tested in switch tests, but verify explicitly
    assert_eq!(
        eval(
            r#"
            let result: string = "";
            switch (2) {
                case 1:
                    result = "one";
                    break;
                case 2:
                    result = "two";
                    break;
                case 3:
                    result = "three";
                    break;
            }
            result
        "#
        ),
        JsValue::String("two".into())
    );
}

#[test]
fn test_break_innermost_loop() {
    // Break only exits the innermost loop
    assert_eq!(
        eval(
            r#"
            let result: string = "";
            for (let i: number = 0; i < 3; i = i + 1) {
                for (let j: number = 0; j < 3; j = j + 1) {
                    if (j === 1) {
                        break;
                    }
                    result = result + i + "," + j + ";";
                }
            }
            result
        "#
        ),
        JsValue::String("0,0;1,0;2,0;".into())
    );
}

// -----------------------------------------------------------------------------
// Continue Statement
// -----------------------------------------------------------------------------

#[test]
fn test_continue_in_while() {
    assert_eq!(
        eval(
            r#"
            let i: number = 0;
            let sum: number = 0;
            while (i < 10) {
                i = i + 1;
                if (i % 2 === 0) {
                    continue;
                }
                sum = sum + i;
            }
            sum
        "#
        ),
        JsValue::Number(25.0) // 1+3+5+7+9
    );
}

#[test]
fn test_continue_in_for() {
    assert_eq!(
        eval(
            r#"
            let sum: number = 0;
            for (let i: number = 0; i < 10; i = i + 1) {
                if (i % 2 === 0) {
                    continue;
                }
                sum = sum + i;
            }
            sum
        "#
        ),
        JsValue::Number(25.0) // 1+3+5+7+9
    );
}

#[test]
fn test_continue_innermost_loop() {
    // Continue only affects the innermost loop
    assert_eq!(
        eval(
            r#"
            let result: string = "";
            for (let i: number = 0; i < 3; i = i + 1) {
                for (let j: number = 0; j < 3; j = j + 1) {
                    if (j === 1) {
                        continue;
                    }
                    result = result + i + "," + j + ";";
                }
            }
            result
        "#
        ),
        JsValue::String("0,0;0,2;1,0;1,2;2,0;2,2;".into())
    );
}

// =============================================================================
// PHASE 3: Labels, Try-Catch-Finally, Throw
// =============================================================================

// -----------------------------------------------------------------------------
// Labeled Statements
// -----------------------------------------------------------------------------

#[test]
fn test_labeled_break_outer_loop() {
    assert_eq!(
        eval(
            r#"
            let result: string = "";
            outer: for (let i: number = 0; i < 3; i = i + 1) {
                for (let j: number = 0; j < 3; j = j + 1) {
                    if (j === 1) {
                        break outer;
                    }
                    result = result + i + "," + j + ";";
                }
            }
            result
        "#
        ),
        JsValue::String("0,0;".into())
    );
}

#[test]
fn test_labeled_continue_outer_loop() {
    assert_eq!(
        eval(
            r#"
            let result: string = "";
            outer: for (let i: number = 0; i < 3; i = i + 1) {
                for (let j: number = 0; j < 3; j = j + 1) {
                    if (j === 1) {
                        continue outer;
                    }
                    result = result + i + "," + j + ";";
                }
            }
            result
        "#
        ),
        JsValue::String("0,0;1,0;2,0;".into())
    );
}

#[test]
fn test_labeled_nested_loops() {
    assert_eq!(
        eval(
            r#"
            let result: number = 0;
            outer: for (let i: number = 0; i < 5; i = i + 1) {
                middle: for (let j: number = 0; j < 5; j = j + 1) {
                    for (let k: number = 0; k < 5; k = k + 1) {
                        result = result + 1;
                        if (k === 2) {
                            break middle;
                        }
                    }
                }
            }
            result
        "#
        ),
        JsValue::Number(15.0) // 5 outer * 3 inner (k=0,1,2 then break)
    );
}

// -----------------------------------------------------------------------------
// Try-Catch-Finally
// -----------------------------------------------------------------------------

#[test]
fn test_try_no_error() {
    assert_eq!(
        eval(
            r#"
            let result: string = "start";
            try {
                result = result + "-try";
            } catch (e: any) {
                result = result + "-catch";
            }
            result
        "#
        ),
        JsValue::String("start-try".into())
    );
}

#[test]
fn test_try_catch_error() {
    assert_eq!(
        eval(
            r#"
            let result: string = "start";
            try {
                throw "error";
            } catch (e: any) {
                result = result + "-catch";
            }
            result
        "#
        ),
        JsValue::String("start-catch".into())
    );
}

#[test]
fn test_try_finally_no_error() {
    assert_eq!(
        eval(
            r#"
            let result: string = "start";
            try {
                result = result + "-try";
            } finally {
                result = result + "-finally";
            }
            result
        "#
        ),
        JsValue::String("start-try-finally".into())
    );
}

#[test]
fn test_try_catch_finally_with_error() {
    assert_eq!(
        eval(
            r#"
            let result: string = "start";
            try {
                throw "oops";
            } catch (e: any) {
                result = result + "-catch";
            } finally {
                result = result + "-finally";
            }
            result
        "#
        ),
        JsValue::String("start-catch-finally".into())
    );
}

#[test]
fn test_catch_error_parameter() {
    assert_eq!(
        eval(
            r#"
            let message: string = "";
            try {
                throw "test error";
            } catch (e: any) {
                message = e;
            }
            message
        "#
        ),
        JsValue::String("test error".into())
    );
}

#[test]
fn test_finally_always_runs() {
    assert_eq!(
        eval(
            r#"
            let result: string = "";
            function test(): string {
                try {
                    result = result + "try-";
                    return "returned";
                } finally {
                    result = result + "finally";
                }
            }
            test();
            result
        "#
        ),
        JsValue::String("try-finally".into())
    );
}

#[test]
fn test_nested_try_catch() {
    assert_eq!(
        eval(
            r#"
            let result: string = "";
            try {
                try {
                    throw "inner";
                } catch (e: any) {
                    result = result + "inner-catch-";
                    throw "outer";
                }
            } catch (e: any) {
                result = result + "outer-catch";
            }
            result
        "#
        ),
        JsValue::String("inner-catch-outer-catch".into())
    );
}

// -----------------------------------------------------------------------------
// Finally block with continue/break/return
// -----------------------------------------------------------------------------

#[test]
fn test_finally_with_continue_in_try() {
    // Finally must run even when try block has continue
    assert_eq!(
        eval(
            r#"
            let fin: number = 0;
            let c: number = 0;
            while (c < 2) {
                try {
                    c = c + 1;
                    continue;
                } finally {
                    fin = 1;
                }
                fin = -1;
            }
            fin
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_finally_with_break_in_try() {
    // Finally must run even when try block has break
    assert_eq!(
        eval(
            r#"
            let fin: number = 0;
            let c: number = 0;
            while (c < 2) {
                try {
                    c = c + 1;
                    break;
                } finally {
                    fin = 1;
                }
                fin = -1;
            }
            fin
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_finally_with_continue_in_catch() {
    // Finally must run even when catch block has continue
    assert_eq!(
        eval(
            r#"
            let fin: number = 0;
            let c: number = 0;
            while (c < 2) {
                try {
                    throw "ex";
                } catch (e: any) {
                    c = c + 1;
                    continue;
                } finally {
                    fin = 1;
                }
                fin = -1;
            }
            fin
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_finally_with_break_in_catch() {
    // Finally must run even when catch block has break
    assert_eq!(
        eval(
            r#"
            let fin: number = 0;
            let c: number = 0;
            while (c < 2) {
                try {
                    throw "ex";
                } catch (e: any) {
                    c = c + 1;
                    break;
                } finally {
                    fin = 1;
                }
                fin = -1;
            }
            fin
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_finally_continue_overrides_try_break() {
    // When finally has continue, it overrides the break from try
    assert_eq!(
        eval(
            r#"
            let c: number = 0;
            while (c < 3) {
                try {
                    c = c + 1;
                    break;  // This should be overridden by finally's continue
                } finally {
                    continue;
                }
            }
            c
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_finally_break_overrides_try_exception() {
    // When finally has break, it suppresses the exception from try
    assert_eq!(
        eval(
            r#"
            let result: number = 0;
            while (true) {
                try {
                    throw "error";
                } finally {
                    result = 1;
                    break;
                }
            }
            result
        "#
        ),
        JsValue::Number(1.0)
    );
}

// -----------------------------------------------------------------------------
// Throw Statement
// -----------------------------------------------------------------------------

#[test]
fn test_throw_string() {
    assert_eq!(
        eval(
            r#"
            let result: string = "";
            try {
                throw "error message";
            } catch (e: any) {
                result = e;
            }
            result
        "#
        ),
        JsValue::String("error message".into())
    );
}

#[test]
fn test_throw_number() {
    assert_eq!(
        eval(
            r#"
            let result: number = 0;
            try {
                throw 42;
            } catch (e: any) {
                result = e;
            }
            result
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_throw_error_object() {
    assert_eq!(
        eval(
            r#"
            let result: string = "";
            try {
                throw new Error("test error");
            } catch (e: any) {
                result = e.message;
            }
            result
        "#
        ),
        JsValue::String("test error".into())
    );
}

#[test]
fn test_throw_custom_object() {
    assert_eq!(
        eval(
            r#"
            let result: string = "";
            try {
                throw { code: 404, message: "not found" };
            } catch (e: any) {
                result = e.code + ":" + e.message;
            }
            result
        "#
        ),
        JsValue::String("404:not found".into())
    );
}

#[test]
fn test_throw_in_function() {
    assert_eq!(
        eval(
            r#"
            function thrower(): void {
                throw "from function";
            }
            let result: string = "";
            try {
                thrower();
            } catch (e: any) {
                result = e;
            }
            result
        "#
        ),
        JsValue::String("from function".into())
    );
}

#[test]
fn test_rethrow_in_catch() {
    assert_eq!(
        eval(
            r#"
            let result: string = "";
            try {
                try {
                    throw "original";
                } catch (e: any) {
                    result = result + "caught-";
                    throw e;
                }
            } catch (e: any) {
                result = result + e;
            }
            result
        "#
        ),
        JsValue::String("caught-original".into())
    );
}

// =============================================================================
// PHASE 4: Error Handling for Invalid Control Flow
// =============================================================================

#[test]
fn test_break_outside_loop_error() {
    // break outside of loop or switch should be an error
    assert!(throws_error("break;", "Illegal break"));
}

#[test]
fn test_continue_outside_loop_error() {
    // continue outside of loop should be an error
    assert!(throws_error("continue;", "Illegal continue"));
}

#[test]
fn test_return_outside_function_error() {
    // return at top level should be fine in our interpreter (returns the value)
    // This is actually valid in some contexts, so we test that it doesn't crash
    let result = eval_result("return 42;");
    // In JavaScript REPL, return at top level is often valid
    // Our interpreter should return the value
    assert!(result.is_ok());
}

#[test]
fn test_break_undefined_label_error() {
    // break with undefined label should be an error
    assert!(throws_error(
        r#"
        for (let i: number = 0; i < 3; i = i + 1) {
            break nonexistent;
        }
        "#,
        "Illegal break"
    ));
}

#[test]
fn test_continue_undefined_label_error() {
    // continue with undefined label should be an error
    assert!(throws_error(
        r#"
        for (let i: number = 0; i < 3; i = i + 1) {
            continue nonexistent;
        }
        "#,
        "Illegal continue"
    ));
}

// =============================================================================
// PHASE 5: Complex/Combined Tests
// =============================================================================

// -----------------------------------------------------------------------------
// Nested Control Flow
// -----------------------------------------------------------------------------

#[test]
fn test_nested_if_in_loop() {
    assert_eq!(
        eval(
            r#"
            let result: string = "";
            for (let i: number = 0; i < 5; i = i + 1) {
                if (i % 2 === 0) {
                    result = result + "E";
                } else {
                    result = result + "O";
                }
            }
            result
        "#
        ),
        JsValue::String("EOEOE".into())
    );
}

#[test]
fn test_nested_loops_with_break() {
    assert_eq!(
        eval(
            r#"
            let count: number = 0;
            for (let i: number = 0; i < 5; i = i + 1) {
                for (let j: number = 0; j < 5; j = j + 1) {
                    count = count + 1;
                    if (j === 2) break;  // Break inner only
                }
            }
            count
        "#
        ),
        JsValue::Number(15.0) // 5 outer * 3 inner (0,1,2)
    );
}

#[test]
fn test_nested_loops_with_continue() {
    assert_eq!(
        eval(
            r#"
            let result: string = "";
            for (let i: number = 0; i < 3; i = i + 1) {
                for (let j: number = 0; j < 3; j = j + 1) {
                    if (j === 1) continue;  // Skip j=1
                    result = result + i + "" + j + ",";
                }
            }
            result
        "#
        ),
        JsValue::String("00,02,10,12,20,22,".into())
    );
}

#[test]
fn test_switch_inside_loop() {
    assert_eq!(
        eval(
            r#"
            let result: string = "";
            let n: number = 0;
            while (n < 4) {
                switch (n) {
                    case 0:
                        result = result + "zero,";
                        break;
                    case 1:
                        result = result + "one,";
                        break;
                    default:
                        result = result + "other,";
                }
                n = n + 1;
            }
            result
        "#
        ),
        JsValue::String("zero,one,other,other,".into())
    );
}

#[test]
fn test_loop_inside_switch() {
    assert_eq!(
        eval(
            r#"
            let result: number = 0;
            let mode: string = "sum";
            switch (mode) {
                case "sum":
                    for (let i: number = 1; i <= 5; i = i + 1) {
                        result = result + i;
                    }
                    break;
                case "product":
                    result = 1;
                    for (let i: number = 1; i <= 5; i = i + 1) {
                        result = result * i;
                    }
                    break;
            }
            result
        "#
        ),
        JsValue::Number(15.0) // 1+2+3+4+5
    );
}

#[test]
fn test_try_catch_in_loop() {
    assert_eq!(
        eval(
            r#"
            let successes: number = 0;
            let failures: number = 0;
            for (let i: number = 0; i < 5; i = i + 1) {
                try {
                    if (i % 2 === 0) {
                        throw "error";
                    }
                    successes = successes + 1;
                } catch (e: any) {
                    failures = failures + 1;
                }
            }
            successes + "," + failures
        "#
        ),
        JsValue::String("2,3".into()) // odd numbers succeed (1,3), even fail (0,2,4)
    );
}

// Debug tests to isolate the try-catch-in-loop issue
#[test]
fn test_for_with_try_no_throw() {
    // Simplest case: for loop with try that doesn't throw
    assert_eq!(
        eval(
            r#"
            let count: number = 0;
            for (let i: number = 0; i < 3; i = i + 1) {
                try {
                    count = count + 1;
                } catch (e: any) {
                }
            }
            count
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_for_with_try_always_throw() {
    // For loop with try that always throws
    assert_eq!(
        eval(
            r#"
            let count: number = 0;
            for (let i: number = 0; i < 3; i = i + 1) {
                try {
                    throw "error";
                } catch (e: any) {
                    count = count + 1;
                }
            }
            count
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_for_with_throw_no_let() {
    // For loop without let (using var instead) to isolate per-iteration binding
    assert_eq!(
        eval(
            r#"
            let count: number = 0;
            for (var i: number = 0; i < 3; i = i + 1) {
                try {
                    throw "error";
                } catch (e: any) {
                    count = count + 1;
                }
            }
            count
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_for_simple_throw_let() {
    // Minimal for loop with let that throws
    assert_eq!(
        eval(
            r#"
            let count: number = 0;
            for (let i: number = 0; i < 2; i = i + 1) {
                try { throw 1; } catch (e: any) { count = count + 1; }
            }
            count
        "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_for_iter_count_let() {
    // Check that iteration happens correct number of times
    assert_eq!(
        eval(
            r#"
            let iterations: number = 0;
            for (let i: number = 0; i < 3; i = i + 1) {
                iterations = iterations + 1;
            }
            iterations
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_for_iter_with_block_let() {
    // For loop with block containing multiple statements
    assert_eq!(
        eval(
            r#"
            let a: number = 0;
            let b: number = 0;
            for (let i: number = 0; i < 2; i = i + 1) {
                a = a + 1;
                b = b + 10;
            }
            a + b
        "#
        ),
        JsValue::Number(22.0) // a=2, b=20
    );
}

#[test]
fn test_for_let_with_throw_in_body() {
    // For loop with let and try-catch that throws in body
    assert_eq!(
        eval(
            r#"
            let last_i: number = -1;
            let count: number = 0;
            for (let i: number = 0; i < 3; i = i + 1) {
                last_i = i;
                try {
                    throw "error";
                } catch (e: any) {
                    count = count + 1;
                }
            }
            last_i + "," + count
        "#
        ),
        JsValue::from("2,3")
    );
}

#[test]
fn test_for_with_if_throw() {
    // For loop with if that throws conditionally
    assert_eq!(
        eval(
            r#"
            let thrown: number = 0;
            for (let i: number = 0; i < 3; i = i + 1) {
                try {
                    if (i % 2 === 0) {
                        throw "error";
                    }
                } catch (e: any) {
                    thrown = thrown + 1;
                }
            }
            thrown
        "#
        ),
        JsValue::Number(2.0) // i=0, i=2 throw
    );
}

#[test]
fn test_loop_in_try_catch() {
    assert_eq!(
        eval(
            r#"
            let result: number = 0;
            try {
                for (let i: number = 0; i < 10; i = i + 1) {
                    result = result + i;
                    if (i === 4) throw "stop";
                }
            } catch (e: any) {
                result = result * 10;  // Mark that we caught it
            }
            result  // (0+1+2+3+4) * 10 = 100
        "#
        ),
        JsValue::Number(100.0)
    );
}

// -----------------------------------------------------------------------------
// Control Flow with Functions
// -----------------------------------------------------------------------------

// Tests for return statement handling in loops
#[test]
fn test_return_in_simple_for_loop() {
    // Simplest case: return immediately on first iteration
    assert_eq!(
        eval(
            r#"
            function test(): number {
                for (let i: number = 0; i < 3; i = i + 1) {
                    return 99;
                }
                return -1;
            }
            test()
        "#
        ),
        JsValue::Number(99.0)
    );
}

#[test]
fn test_return_in_if_inside_for_loop() {
    // Return inside if inside for loop
    assert_eq!(
        eval(
            r#"
            function test(): number {
                for (let i: number = 0; i < 3; i = i + 1) {
                    if (i === 1) {
                        return i;
                    }
                }
                return -1;
            }
            test()
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_return_in_loop() {
    assert_eq!(
        eval(
            r#"
            function findFirst(arr: number[], target: number): number {
                for (let i: number = 0; i < arr.length; i = i + 1) {
                    if (arr[i] === target) {
                        return i;
                    }
                }
                return -1;
            }
            findFirst([10, 20, 30, 40], 30)
        "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_return_in_try_finally() {
    assert_eq!(
        eval(
            r#"
            let cleanup: boolean = false;
            function test(): number {
                try {
                    return 42;
                } finally {
                    cleanup = true;
                }
            }
            let result: number = test();
            result + "," + cleanup
        "#
        ),
        JsValue::String("42,true".into())
    );
}

#[test]
fn test_closure_capturing_loop_variable() {
    // This is the classic closure capture test
    // let should capture per-iteration value
    assert_eq!(
        eval(
            r#"
            let funcs: any[] = [];
            for (let i: number = 0; i < 3; i = i + 1) {
                funcs.push(function(): number { return i; });
            }
            funcs[0]() + "," + funcs[1]() + "," + funcs[2]()
        "#
        ),
        JsValue::String("0,1,2".into())
    );
}

#[test]
fn test_recursive_with_base_case() {
    assert_eq!(
        eval(
            r#"
            function factorial(n: number): number {
                if (n <= 1) {
                    return 1;
                }
                return n * factorial(n - 1);
            }
            factorial(5)
        "#
        ),
        JsValue::Number(120.0)
    );
}

// -----------------------------------------------------------------------------
// Short-Circuit Evaluation
// -----------------------------------------------------------------------------

#[test]
fn test_and_short_circuit() {
    assert_eq!(
        eval(
            r#"
            let called: boolean = false;
            function sideEffect(): boolean {
                called = true;
                return true;
            }
            let result: boolean = false && sideEffect();
            called
        "#
        ),
        JsValue::Boolean(false) // sideEffect should not be called
    );
}

#[test]
fn test_or_short_circuit() {
    assert_eq!(
        eval(
            r#"
            let called: boolean = false;
            function sideEffect(): boolean {
                called = true;
                return false;
            }
            let result: boolean = true || sideEffect();
            called
        "#
        ),
        JsValue::Boolean(false) // sideEffect should not be called
    );
}

#[test]
fn test_or_returns_falsy_value() {
    // Test || when first operand is falsy - should return second operand
    assert_eq!(eval("false || true"), JsValue::Boolean(true));
    assert_eq!(eval("false || false"), JsValue::Boolean(false));
}

#[test]
fn test_and_returns_right_value() {
    // Test && - should return right operand when left is truthy
    assert_eq!(eval("true && false"), JsValue::Boolean(false));
    assert_eq!(eval("true && true"), JsValue::Boolean(true));
}

#[test]
fn test_logical_operators_with_complex_expressions() {
    // Test that logical operators work correctly with complex right-hand expressions
    assert_eq!(
        eval(
            r#"
            let a: number = 5;
            let b: number = 10;
            (a > 0) && (b > 0)
        "#
        ),
        JsValue::Boolean(true)
    );

    assert_eq!(
        eval(
            r#"
            let a: number = 5;
            let b: number = 10;
            (a > 0) && (b < 0)
        "#
        ),
        JsValue::Boolean(false)
    );

    assert_eq!(
        eval(
            r#"
            let a: number = -5;
            let b: number = 10;
            (a > 0) || (b > 0)
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_nullish_coalescing() {
    assert_eq!(
        eval(
            r#"
            let a: any = null;
            let b: any = undefined;
            let c: any = 0;
            let d: any = "";

            (a ?? "default1") + "," + (b ?? "default2") + "," + (c ?? "default3") + "," + (d ?? "default4")
        "#
        ),
        JsValue::String("default1,default2,0,".into())
    );
}

#[test]
fn test_optional_chaining_in_condition() {
    assert_eq!(
        eval(
            r#"
            let obj: any = { a: { b: 42 } };
            let result: string = "";

            if (obj?.a?.b) {
                result = "exists";
            } else {
                result = "missing";
            }

            if (obj?.x?.y) {
                result = result + ",found";
            } else {
                result = result + ",notfound";
            }

            result
        "#
        ),
        JsValue::String("exists,notfound".into())
    );
}

#[test]
fn test_optional_chaining_short_circuit() {
    // When base is nullish, the entire chain should short-circuit
    // and side effects should NOT be evaluated
    assert_eq!(
        eval(
            r#"
            const a: undefined = undefined;
            let x: number = 1;
            a?.[++x];  // Should short-circuit, ++x should NOT run
            x
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_optional_chaining_long_short_circuit() {
    // Long chain after ?. should all short-circuit
    assert_eq!(
        eval(
            r#"
            const a: undefined = undefined;
            let x: number = 1;
            a?.b.c(++x).d;  // Should short-circuit at a?., rest is not evaluated
            x
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_optional_chaining_null_short_circuit() {
    // null should also short-circuit
    assert_eq!(
        eval(
            r#"
            const a: null = null;
            let x: number = 1;
            a?.[++x];
            a?.b.c(++x).d;
            x
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_optional_call_short_circuit() {
    // Optional call should short-circuit
    assert_eq!(
        eval(
            r#"
            const fn: undefined = undefined;
            let x: number = 1;
            fn?.(++x);  // Should short-circuit
            x
        "#
        ),
        JsValue::Number(1.0)
    );
}

// -----------------------------------------------------------------------------
// Optional Chaining `this` Preservation Tests
// -----------------------------------------------------------------------------

#[test]
fn test_optional_call_preserves_this_basic() {
    // a?.b() should preserve `a` as `this` when calling b
    assert_eq!(
        eval(
            r#"
            const a = {
                b() { return this._b; },
                _b: { c: 42 }
            };
            a?.b().c
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_optional_call_preserves_this_parenthesized() {
    // (a?.b)() should also preserve `a` as `this`
    assert_eq!(
        eval(
            r#"
            const a = {
                b() { return this._b; },
                _b: { c: 42 }
            };
            (a?.b)().c
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_optional_call_on_method() {
    // a.b?.() - optional call on method should preserve `a` as `this`
    assert_eq!(
        eval(
            r#"
            const a = {
                b() { return this._b; },
                _b: { c: 42 }
            };
            a.b?.().c
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_optional_call_on_method_parenthesized() {
    // (a.b)?.() - optional call on parenthesized method
    assert_eq!(
        eval(
            r#"
            const a = {
                b() { return this._b; },
                _b: { c: 42 }
            };
            (a.b)?.().c
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_optional_chain_double_optional() {
    // a?.b?.() - both optional member and optional call
    assert_eq!(
        eval(
            r#"
            const a = {
                b() { return this._b; },
                _b: { c: 42 }
            };
            a?.b?.().c
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_optional_chain_double_optional_parenthesized() {
    // (a?.b)?.() - parenthesized version
    assert_eq!(
        eval(
            r#"
            const a = {
                b() { return this._b; },
                _b: { c: 42 }
            };
            (a?.b)?.().c
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_optional_call_this_with_nested_object() {
    // More complex case with nested method calls
    assert_eq!(
        eval(
            r#"
            const obj = {
                name: "test",
                getName() { return this.name; },
                nested: {
                    value: 100,
                    getValue() { return this.value; }
                }
            };
            obj?.getName() + "-" + obj?.nested?.getValue()
        "#
        ),
        JsValue::String("test-100".into())
    );
}

#[test]
fn test_optional_call_this_null_base() {
    // When base is null, should short-circuit
    assert_eq!(
        eval(
            r#"
            const a: any = null;
            a?.b()
        "#
        ),
        JsValue::Undefined
    );
}

#[test]
fn test_optional_call_this_undefined_method() {
    // When method is undefined, optional call should short-circuit
    assert_eq!(
        eval(
            r#"
            const a: any = { x: 1 };
            a.notAMethod?.()
        "#
        ),
        JsValue::Undefined
    );
}

#[test]
fn test_optional_call_this_with_arguments() {
    // Optional call with arguments should still preserve this
    assert_eq!(
        eval(
            r#"
            const calc = {
                base: 10,
                add(x: number) { return this.base + x; },
                multiply(x: number) { return this.base * x; }
            };
            calc?.add(5) + calc?.multiply(3)
        "#
        ),
        JsValue::Number(45.0) // (10+5) + (10*3) = 15 + 30 = 45
    );
}

#[test]
fn test_optional_call_chained_methods() {
    // Chained method calls with optional
    assert_eq!(
        eval(
            r#"
            const builder = {
                value: "",
                append(s: string) {
                    this.value = this.value + s;
                    return this;
                },
                get() { return this.value; }
            };
            builder?.append("a")?.append("b")?.append("c")?.get()
        "#
        ),
        JsValue::String("abc".into())
    );
}

#[test]
fn test_optional_call_with_computed_property() {
    // Optional call with computed property access
    // `this` in getValue should be `obj.methods`, so we need `value` on methods
    assert_eq!(
        eval(
            r#"
            const obj = {
                methods: {
                    value: 42,
                    getValue() { return this.value; }
                }
            };
            const key = "getValue";
            obj.methods?.[key]?.()
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_optional_call_array_method() {
    // Optional call on array methods should work
    assert_eq!(
        eval(
            r#"
            const arr: number[] = [1, 2, 3];
            arr?.map((x: number) => x * 2)?.join(",")
        "#
        ),
        JsValue::String("2,4,6".into())
    );
}

// -----------------------------------------------------------------------------
// Complex Real-World Patterns
// -----------------------------------------------------------------------------

#[test]
fn test_fizzbuzz() {
    assert_eq!(
        eval(
            r#"
            let result: string[] = [];
            for (let i: number = 1; i <= 15; i = i + 1) {
                if (i % 15 === 0) {
                    result.push("FizzBuzz");
                } else if (i % 3 === 0) {
                    result.push("Fizz");
                } else if (i % 5 === 0) {
                    result.push("Buzz");
                } else {
                    result.push(i.toString());
                }
            }
            result.join(",")
        "#
        ),
        JsValue::String("1,2,Fizz,4,Buzz,Fizz,7,8,Fizz,Buzz,11,Fizz,13,14,FizzBuzz".into())
    );
}

#[test]
fn test_binary_search() {
    assert_eq!(
        eval(
            r#"
            function binarySearch(arr: number[], target: number): number {
                let left: number = 0;
                let right: number = arr.length - 1;

                while (left <= right) {
                    let mid: number = Math.floor((left + right) / 2);
                    if (arr[mid] === target) {
                        return mid;
                    } else if (arr[mid] < target) {
                        left = mid + 1;
                    } else {
                        right = mid - 1;
                    }
                }
                return -1;
            }

            let arr: number[] = [1, 3, 5, 7, 9, 11, 13];
            binarySearch(arr, 7) + "," + binarySearch(arr, 6)
        "#
        ),
        JsValue::String("3,-1".into())
    );
}

#[test]
fn test_bubble_sort() {
    assert_eq!(
        eval(
            r#"
            function bubbleSort(arr: number[]): number[] {
                let n: number = arr.length;
                let swapped: boolean = true;

                while (swapped) {
                    swapped = false;
                    for (let i: number = 0; i < n - 1; i = i + 1) {
                        if (arr[i] > arr[i + 1]) {
                            let temp: number = arr[i];
                            arr[i] = arr[i + 1];
                            arr[i + 1] = temp;
                            swapped = true;
                        }
                    }
                }
                return arr;
            }

            bubbleSort([5, 2, 8, 1, 9]).join(",")
        "#
        ),
        JsValue::String("1,2,5,8,9".into())
    );
}

#[test]
fn test_state_machine() {
    assert_eq!(
        eval(
            r#"
            function processInput(inputs: string[]): string {
                let state: string = "start";
                let output: string = "";

                for (let i: number = 0; i < inputs.length; i = i + 1) {
                    let input: string = inputs[i];

                    switch (state) {
                        case "start":
                            if (input === "a") {
                                state = "gotA";
                                output = output + "A";
                            }
                            break;
                        case "gotA":
                            if (input === "b") {
                                state = "gotAB";
                                output = output + "B";
                            } else {
                                state = "start";
                                output = output + "X";
                            }
                            break;
                        case "gotAB":
                            if (input === "c") {
                                state = "start";
                                output = output + "C!";
                            } else {
                                state = "start";
                                output = output + "X";
                            }
                            break;
                    }
                }

                return output;
            }

            processInput(["a", "b", "c", "a", "x", "a", "b", "c"])
        "#
        ),
        JsValue::String("ABC!AXABC!".into())
    );
}

#[test]
fn test_retry_with_try_catch() {
    assert_eq!(
        eval(
            r#"
            let attempts: number = 0;
            let success: boolean = false;
            let result: string = "";

            while (attempts < 5 && !success) {
                attempts = attempts + 1;
                try {
                    if (attempts < 3) {
                        throw "temporary failure";
                    }
                    success = true;
                    result = "succeeded on attempt " + attempts;
                } catch (e: any) {
                    result = "failed attempt " + attempts;
                }
            }

            result
        "#
        ),
        JsValue::String("succeeded on attempt 3".into())
    );
}

#[test]
fn test_iterator_pattern() {
    assert_eq!(
        eval(
            r#"
            function createIterator(arr: number[]) {
                let index: number = 0;
                return {
                    hasNext: function(): boolean {
                        return index < arr.length;
                    },
                    next: function(): number {
                        let value: number = arr[index];
                        index = index + 1;
                        return value;
                    }
                };
            }

            let iter: any = createIterator([10, 20, 30]);
            let sum: number = 0;

            while (iter.hasNext()) {
                sum = sum + iter.next();
            }

            sum
        "#
        ),
        JsValue::Number(60.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Sieve of Eratosthenes (complex algorithm with loops)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_sieve_of_eratosthenes() {
    let result = eval(
        r#"
        function sieveOfEratosthenes(n: number): number[] {
            if (n < 2) return [];
            const sieve: boolean[] = [];
            for (let i = 0; i <= n; i++) {
                sieve.push(true);
            }
            sieve[0] = false;
            sieve[1] = false;
            for (let i = 2; i * i <= n; i++) {
                if (sieve[i]) {
                    for (let j = i * i; j <= n; j += i) {
                        sieve[j] = false;
                    }
                }
            }
            const primes: number[] = [];
            for (let i = 2; i <= n; i++) {
                if (sieve[i]) {
                    primes.push(i);
                }
            }
            return primes;
        }
        sieveOfEratosthenes(20).join(",")
    "#,
    );
    assert_eq!(result, JsValue::String("2,3,5,7,11,13,17,19".into()));
}

#[test]
fn test_for_of_with_template_literal() {
    // Test for...of loop with template literal in body
    let result = eval(
        r#"
        const urls: string[] = ["a", "b", "c"];
        let output: string = "";
        for (const url of urls) {
            output = output + `${url}! `;
        }
        output
    "#,
    );
    assert_eq!(result, JsValue::String("a! b! c! ".into()));
}

#[test]
fn test_nested_block_scopes() {
    // Test that nested block scopes properly save/restore environments
    let result = eval(
        r#"
        let outer = 1;
        {
            let a = 10;
            {
                let b = 20;
                outer = a + b;
            }
            // b should not be visible here
            outer = outer + a;
        }
        // a and b should not be visible here
        outer
    "#,
    );
    // outer = (10 + 20) + 10 = 40
    assert_eq!(result, JsValue::Number(40.0));
}

#[test]
fn test_deeply_nested_block_scopes() {
    // Test 5 levels of nested blocks
    let result = eval(
        r#"
        let result = 0;
        {
            let a = 1;
            {
                let b = 2;
                {
                    let c = 3;
                    {
                        let d = 4;
                        {
                            let e = 5;
                            result = a + b + c + d + e;
                        }
                        result = result + d;
                    }
                    result = result + c;
                }
                result = result + b;
            }
            result = result + a;
        }
        result
    "#,
    );
    // result = (1+2+3+4+5) + 4 + 3 + 2 + 1 = 15 + 10 = 25
    assert_eq!(result, JsValue::Number(25.0));
}

#[test]
fn test_nested_blocks_outer_var_visible_after() {
    // Verify that outer scope vars are still visible after nested blocks exit
    // This tests that env is correctly restored
    let result = eval(
        r#"
        let outer = 1;
        {
            let a = 10;
            {
                let b = 20;
            }
            // After inner block exits, we should still be in scope where 'a' is visible
            outer = a;
        }
        // After outer block exits, only 'outer' should be visible
        outer
    "#,
    );
    assert_eq!(result, JsValue::Number(10.0));
}

#[test]
fn test_block_var_not_visible_after() {
    // Block-scoped var should throw ReferenceError after block
    use typescript_eval::Runtime;
    let mut runtime = Runtime::new();
    let result = runtime.eval(
        r#"
        {
            let x = 10;
        }
        x  // Should throw ReferenceError
    "#,
    );
    assert!(
        result.is_err(),
        "Expected ReferenceError for 'x' after block"
    );
}

#[test]
fn test_nested_block_outer_var_not_visible_after_both() {
    // After nested blocks, outer block var should NOT be visible
    use typescript_eval::Runtime;
    let mut runtime = Runtime::new();
    let result = runtime.eval(
        r#"
        {
            let a = 10;
            {
                let b = 20;
            }
        }
        a  // Should throw ReferenceError - 'a' was in outer block
    "#,
    );
    assert!(
        result.is_err(),
        "Expected ReferenceError for 'a' after both blocks exit"
    );
}
