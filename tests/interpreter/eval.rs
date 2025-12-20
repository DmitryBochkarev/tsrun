//! Tests for the global eval() function
//!
//! eval() executes JavaScript/TypeScript code in a string and returns the result.
//! Unlike the Function constructor, eval() has access to the current scope.

use super::{eval, throws_error};
use typescript_eval::JsValue;

// ============================================================================
// Basic eval() behavior
// ============================================================================

#[test]
fn test_eval_basic_expression() {
    assert_eq!(eval("eval('1 + 2')"), JsValue::Number(3.0));
}

#[test]
fn test_eval_string_literal() {
    assert_eq!(eval("eval('\"hello\"')"), JsValue::from("hello"));
}

#[test]
fn test_eval_number_literal() {
    assert_eq!(eval("eval('42')"), JsValue::Number(42.0));
}

#[test]
fn test_eval_boolean_literal() {
    assert_eq!(eval("eval('true')"), JsValue::Boolean(true));
    assert_eq!(eval("eval('false')"), JsValue::Boolean(false));
}

#[test]
fn test_eval_null() {
    assert_eq!(eval("eval('null')"), JsValue::Null);
}

#[test]
fn test_eval_undefined() {
    assert_eq!(eval("eval('undefined')"), JsValue::Undefined);
}

#[test]
fn test_eval_empty_string() {
    // eval('') returns undefined
    assert_eq!(eval("eval('')"), JsValue::Undefined);
}

// ============================================================================
// eval() with statements
// ============================================================================

#[test]
fn test_eval_variable_declaration() {
    // In strict mode, var declarations in eval are scoped to eval itself
    // So x is not visible after eval returns
    assert_eq!(
        eval("eval('var x = 42'); typeof x"),
        JsValue::from("undefined")
    );
}

#[test]
fn test_eval_variable_inside_eval() {
    // But var is visible inside eval
    assert_eq!(eval("eval('var x = 42; x')"), JsValue::Number(42.0));
}

#[test]
fn test_eval_let_declaration() {
    // let in eval is scoped to eval in strict mode
    assert_eq!(
        eval("eval('let y = 100'); typeof y"),
        JsValue::from("undefined")
    );
}

#[test]
fn test_eval_multiple_statements() {
    // eval returns the value of the last statement/expression
    assert_eq!(eval("eval('1; 2; 3')"), JsValue::Number(3.0));
}

#[test]
fn test_eval_function_declaration() {
    // In strict mode, functions declared in eval are scoped to eval itself
    assert_eq!(
        eval("eval('function add(a, b) { return a + b; }'); typeof add"),
        JsValue::from("undefined")
    );
}

#[test]
fn test_eval_function_inside_eval() {
    // But the function is usable inside eval
    assert_eq!(
        eval("eval('function add(a, b) { return a + b; } add(2, 3)')"),
        JsValue::Number(5.0)
    );
}

// ============================================================================
// eval() scope access
// ============================================================================

#[test]
fn test_eval_access_outer_variable() {
    // eval can read variables from enclosing scope
    assert_eq!(eval("let x = 10; eval('x')"), JsValue::Number(10.0));
}

#[test]
fn test_eval_modify_outer_variable() {
    // eval can modify variables from enclosing scope
    assert_eq!(eval("let x = 10; eval('x = 20'); x"), JsValue::Number(20.0));
}

#[test]
fn test_eval_access_function_scope() {
    // eval inside a function can access function scope
    assert_eq!(
        eval(
            r#"
            function test() {
                let y = 42;
                return eval('y');
            }
            test()
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_eval_access_global() {
    // eval can access global variables
    assert_eq!(
        eval("var globalVar = 100; eval('globalVar')"),
        JsValue::Number(100.0)
    );
}

// ============================================================================
// eval() properties
// ============================================================================

#[test]
fn test_eval_length() {
    // eval.length === 1
    assert_eq!(eval("eval.length"), JsValue::Number(1.0));
}

#[test]
fn test_eval_name() {
    // eval.name === "eval"
    assert_eq!(eval("eval.name"), JsValue::from("eval"));
}

#[test]
fn test_eval_typeof() {
    // typeof eval === "function"
    assert_eq!(eval("typeof eval"), JsValue::from("function"));
}

#[test]
fn test_eval_is_callable() {
    // eval can be called
    assert_eq!(eval("typeof eval('1 + 1')"), JsValue::from("number"));
}

// ============================================================================
// eval() with non-string arguments
// ============================================================================

#[test]
fn test_eval_non_string_number() {
    // If argument is not a string, return it directly
    assert_eq!(eval("eval(42)"), JsValue::Number(42.0));
}

#[test]
fn test_eval_non_string_boolean() {
    assert_eq!(eval("eval(true)"), JsValue::Boolean(true));
}

#[test]
fn test_eval_non_string_null() {
    assert_eq!(eval("eval(null)"), JsValue::Null);
}

#[test]
fn test_eval_non_string_undefined() {
    assert_eq!(eval("eval(undefined)"), JsValue::Undefined);
}

#[test]
fn test_eval_non_string_object() {
    // Objects passed directly are returned as-is
    assert_eq!(eval("let obj = {a: 1}; eval(obj).a"), JsValue::Number(1.0));
}

#[test]
fn test_eval_non_string_array() {
    assert_eq!(eval("eval([1, 2, 3])[1]"), JsValue::Number(2.0));
}

// ============================================================================
// eval() error handling
// ============================================================================

#[test]
fn test_eval_syntax_error() {
    // Invalid syntax should throw SyntaxError
    assert!(throws_error("eval('{')", "SyntaxError"));
}

#[test]
fn test_eval_reference_error() {
    // Reference to undefined variable should throw ReferenceError
    assert!(throws_error("eval('undefinedVariable')", "ReferenceError"));
}

#[test]
fn test_eval_type_error() {
    // Type errors should propagate
    assert!(throws_error("eval('null.foo')", "TypeError"));
}

// ============================================================================
// eval() is not a constructor
// ============================================================================

#[test]
fn test_eval_not_constructor() {
    // new eval() should throw TypeError
    assert!(throws_error("new eval()", "TypeError"));
}

#[test]
fn test_eval_not_constructor_with_arg() {
    // new eval('code') should throw TypeError
    assert!(throws_error("new eval('1 + 1')", "TypeError"));
}

// ============================================================================
// eval() with complex expressions
// ============================================================================

#[test]
fn test_eval_object_literal() {
    // eval can parse and return objects
    assert_eq!(eval("eval('({a: 1, b: 2})').a"), JsValue::Number(1.0));
}

#[test]
fn test_eval_array_literal() {
    assert_eq!(eval("eval('[1, 2, 3]').length"), JsValue::Number(3.0));
}

#[test]
fn test_eval_arrow_function() {
    assert_eq!(eval("eval('((x) => x * 2)(5)')"), JsValue::Number(10.0));
}

#[test]
fn test_eval_template_literal() {
    assert_eq!(
        eval("let x = 'world'; eval('`hello ${x}`')"),
        JsValue::from("hello world")
    );
}

#[test]
fn test_eval_regex() {
    assert_eq!(eval("eval('/abc/g').source"), JsValue::from("abc"));
}

// ============================================================================
// eval() and strict mode
// ============================================================================

#[test]
fn test_eval_strict_mode_this() {
    // In strict mode, `this` inside eval should be undefined in functions
    assert_eq!(
        eval(
            r#"
            function test() {
                return eval('this');
            }
            test()
        "#
        ),
        JsValue::Undefined
    );
}

#[test]
fn test_eval_strict_assignment_to_undeclared() {
    // Assigning to undeclared variable should throw ReferenceError in strict mode
    assert!(throws_error("eval('undeclaredVar = 1')", "ReferenceError"));
}

// ============================================================================
// Indirect eval (should use global scope)
// ============================================================================

#[test]
fn test_indirect_eval_uses_global_scope() {
    // Indirect eval (via assignment) should use global scope
    // Note: This is a subtle spec requirement - (1, eval)('code') is indirect eval
    assert_eq!(
        eval(
            r#"
            var globalX = 42;
            function test() {
                var localX = 100;
                // Indirect eval using comma operator
                return (1, eval)('globalX');
            }
            test()
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_indirect_eval_via_variable() {
    // Assigning eval to a variable makes it indirect
    assert_eq!(
        eval(
            r#"
            var globalY = 50;
            var indirectEval = eval;
            function test() {
                var localY = 200;
                return indirectEval('globalY');
            }
            test()
        "#
        ),
        JsValue::Number(50.0)
    );
}

// ============================================================================
// eval() with TypeScript syntax
// ============================================================================

#[test]
fn test_eval_with_type_annotation() {
    // TypeScript type annotations should be parsed and stripped
    assert_eq!(eval("eval('let x: number = 42; x')"), JsValue::Number(42.0));
}

#[test]
fn test_eval_with_type_assertion() {
    assert_eq!(
        eval("eval('let x = 42 as number; x')"),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_eval_with_interface() {
    // Interface declarations are no-ops at runtime
    assert_eq!(
        eval("eval('interface Foo { x: number }; 42')"),
        JsValue::Number(42.0)
    );
}

// ============================================================================
// eval() no arguments
// ============================================================================

#[test]
fn test_eval_no_args() {
    // eval() with no arguments returns undefined
    assert_eq!(eval("eval()"), JsValue::Undefined);
}

// ============================================================================
// eval() and closures
// ============================================================================

#[test]
fn test_eval_closure() {
    // Function created via eval should capture the scope
    assert_eq!(
        eval(
            r#"
            function outer() {
                let x = 10;
                let f = eval('(function() { return x; })');
                return f();
            }
            outer()
        "#
        ),
        JsValue::Number(10.0)
    );
}

// ============================================================================
// eval() return value semantics
// ============================================================================

#[test]
fn test_eval_return_last_expression() {
    // Return value is the value of the last evaluated expression
    assert_eq!(eval("eval('1; 2; 3')"), JsValue::Number(3.0));
}

#[test]
fn test_eval_statement_returns_undefined() {
    // Statements like if/for don't produce values
    // But the block might contain expressions
    assert_eq!(eval("eval('if (true) { 42 }')"), JsValue::Number(42.0));
}

#[test]
fn test_eval_empty_block() {
    assert_eq!(eval("eval('{}')"), JsValue::Undefined);
}

// ============================================================================
// eval() edge cases
// ============================================================================

#[test]
fn test_eval_whitespace_only() {
    // Whitespace-only string should return undefined
    assert_eq!(eval("eval('   ')"), JsValue::Undefined);
}

#[test]
fn test_eval_comments_only() {
    // Comments-only string should return undefined
    assert_eq!(eval("eval('// comment')"), JsValue::Undefined);
}

#[test]
fn test_eval_multiline() {
    assert_eq!(
        eval("eval('let a = 1;\\nlet b = 2;\\na + b')"),
        JsValue::Number(3.0)
    );
}

// ============================================================================
// eval() comparison with Function constructor
// ============================================================================

#[test]
fn test_eval_vs_function_scope() {
    // eval has access to local scope, Function() doesn't
    assert_eq!(
        eval(
            r#"
            function test() {
                let localVar = 123;
                // eval can see localVar
                let evalResult = eval('localVar');
                // Function constructor cannot (uses global scope)
                let fnResult = (new Function('return typeof localVar'))();
                return evalResult + '-' + fnResult;
            }
            test()
        "#
        ),
        JsValue::from("123-undefined")
    );
}

// ============================================================================
// eval() called as method
// ============================================================================

#[test]
fn test_eval_as_method() {
    // eval can be called as a method (but `this` doesn't affect it)
    assert_eq!(
        eval(
            r#"
            let obj = { eval: eval };
            obj.eval('1 + 1')
        "#
        ),
        JsValue::Number(2.0)
    );
}

// ============================================================================
// Property descriptor tests (matching test262)
// ============================================================================

#[test]
fn test_eval_property_writable() {
    // eval should be writable on global object
    assert_eq!(
        eval("let desc = Object.getOwnPropertyDescriptor(globalThis, 'eval'); desc.writable"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_eval_property_enumerable() {
    // eval should NOT be enumerable
    assert_eq!(
        eval("let desc = Object.getOwnPropertyDescriptor(globalThis, 'eval'); desc.enumerable"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_eval_property_configurable() {
    // eval should be configurable
    assert_eq!(
        eval("let desc = Object.getOwnPropertyDescriptor(globalThis, 'eval'); desc.configurable"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_eval_name_property_writable() {
    // eval.name should NOT be writable
    assert_eq!(
        eval("let desc = Object.getOwnPropertyDescriptor(eval, 'name'); desc.writable"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_eval_name_property_enumerable() {
    // eval.name should NOT be enumerable
    assert_eq!(
        eval("let desc = Object.getOwnPropertyDescriptor(eval, 'name'); desc.enumerable"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_eval_name_property_configurable() {
    // eval.name should be configurable
    assert_eq!(
        eval("let desc = Object.getOwnPropertyDescriptor(eval, 'name'); desc.configurable"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_eval_length_property_writable() {
    // eval.length should NOT be writable
    assert_eq!(
        eval("let desc = Object.getOwnPropertyDescriptor(eval, 'length'); desc.writable"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_eval_length_property_enumerable() {
    // eval.length should NOT be enumerable
    assert_eq!(
        eval("let desc = Object.getOwnPropertyDescriptor(eval, 'length'); desc.enumerable"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_eval_length_property_configurable() {
    // eval.length should be configurable
    assert_eq!(
        eval("let desc = Object.getOwnPropertyDescriptor(eval, 'length'); desc.configurable"),
        JsValue::Boolean(true)
    );
}

// ============================================================================
// Debug tests for eval scope investigation
// ============================================================================

#[test]
fn test_eval_scope_debug() {
    // Simplest case: eval reading a variable from enclosing function scope
    let result = eval(
        r#"
        function test() {
            let localVar = 123;
            console.log("localVar before eval:", localVar);
            let result = eval('localVar');
            console.log("eval result:", result);
            return result;
        }
        test()
    "#,
    );
    assert_eq!(result, JsValue::Number(123.0));
}

#[test]
fn test_eval_scope_with_function_constructor() {
    // Test similar to test_eval_vs_function_scope but with console.log to debug
    let result = eval(
        r#"
        function test() {
            let localVar = 123;
            console.log("Step 1: localVar =", localVar);
            // This is the line that fails in the original test
            let evalResult = eval('localVar');
            console.log("Step 2: evalResult =", evalResult);
            // Now try Function constructor
            let fnResult = (new Function('return typeof localVar'))();
            console.log("Step 3: fnResult =", fnResult);
            return evalResult + '-' + fnResult;
        }
        test()
    "#,
    );
    assert_eq!(result, JsValue::from("123-undefined"));
}

#[test]
fn test_typeof_undeclared_variable() {
    // typeof should return "undefined" for undeclared variables, not throw
    assert_eq!(
        eval("typeof nonExistentVariable"),
        JsValue::from("undefined")
    );
}

#[test]
fn test_eval_scope_minimal() {
    // Even simpler: just the function call
    assert_eq!(
        eval(
            r#"
            function f() {
                let x = 42;
                return eval('x');
            }
            f()
        "#
        ),
        JsValue::Number(42.0)
    );
}

// ============================================================================
// eval() completion value tests (test262 cptn-* tests)
// ============================================================================

#[test]
fn test_eval_if_empty_block_completion() {
    // eval('1; if (true) { }') should return undefined (from empty block)
    assert_eq!(eval("eval('1; if (true) { }')"), JsValue::Undefined);
}

#[test]
fn test_eval_if_expression_completion() {
    // eval('2; if (true) { 3; }') should return 3 (from expression in block)
    assert_eq!(eval("eval('2; if (true) { 3; }')"), JsValue::Number(3.0));
}

#[test]
fn test_eval_switch_completion() {
    // switch completion values
    assert_eq!(
        eval("eval('1; switch (\"a\") { case \"a\": 2; }')"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_eval_for_completion() {
    // for loop completion value is from last iteration's body
    assert_eq!(
        eval("eval('1; for (let i = 0; i < 3; i++) { i; }')"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_eval_while_completion() {
    // while loop completion value
    assert_eq!(
        eval("eval('let i = 0; while (i < 3) { i++; }')"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_eval_if_else_completion() {
    // if-else completion values
    assert_eq!(
        eval("eval('if (false) { 1 } else { 2 }')"),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval("eval('if (true) { 1 } else { 2 }')"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_eval_try_completion() {
    // try block completion
    assert_eq!(
        eval("eval('try { 42 } catch(e) { }')"),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_eval_block_completion() {
    // block completion value is from last statement
    assert_eq!(eval("eval('{ 1; 2; 3; }')"), JsValue::Number(3.0));
}
