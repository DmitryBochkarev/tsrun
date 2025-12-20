//! Tests for strict mode enforcement
//!
//! The interpreter always runs in strict mode (like ES modules).

use super::{eval, eval_result};
use typescript_eval::JsValue;

// ============================================================================
// Assignment to read-only globals (Infinity, NaN, undefined)
// ============================================================================

#[test]
fn test_strict_assign_infinity() {
    // Assignment to Infinity should throw TypeError
    let result = eval_result("Infinity = 1; Infinity");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("constant") || err.to_string().contains("Assignment"),
        "Expected constant assignment error, got: {}",
        err
    );
}

#[test]
fn test_strict_assign_nan() {
    // Assignment to NaN should throw TypeError
    let result = eval_result("NaN = 1; NaN");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("constant") || err.to_string().contains("Assignment"),
        "Expected constant assignment error, got: {}",
        err
    );
}

#[test]
fn test_strict_assign_undefined() {
    // Assignment to undefined should throw TypeError
    let result = eval_result("undefined = 1; undefined");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("constant") || err.to_string().contains("Assignment"),
        "Expected constant assignment error, got: {}",
        err
    );
}

#[test]
fn test_strict_can_shadow_infinity_in_function() {
    // Shadowing with var in function scope is allowed
    let result = eval(
        r#"
        function test() {
            var Infinity = 42;
            return Infinity;
        }
        test()
        "#,
    );
    assert_eq!(result.value(), &JsValue::Number(42.0));
}

#[test]
fn test_strict_can_shadow_with_let() {
    // Shadowing with let in block scope is allowed
    let result = eval(
        r#"
        {
            let Infinity = 42;
            Infinity
        }
        "#,
    );
    assert_eq!(result.value(), &JsValue::Number(42.0));
}

// ============================================================================
// Reserved words: eval and arguments
// ============================================================================

#[test]
fn test_strict_no_var_eval() {
    // Cannot use 'eval' as variable name in strict mode
    let result = eval_result("var eval = 1;");
    assert!(result.is_err(), "Should not allow var eval in strict mode");
}

#[test]
fn test_strict_no_let_eval() {
    // Cannot use 'eval' as variable name in strict mode
    let result = eval_result("let eval = 1;");
    assert!(result.is_err(), "Should not allow let eval in strict mode");
}

#[test]
fn test_strict_no_const_eval() {
    // Cannot use 'eval' as variable name in strict mode
    let result = eval_result("const eval = 1;");
    assert!(
        result.is_err(),
        "Should not allow const eval in strict mode"
    );
}

#[test]
fn test_strict_no_var_arguments() {
    // Cannot use 'arguments' as variable name in strict mode
    let result = eval_result("var arguments = 1;");
    assert!(
        result.is_err(),
        "Should not allow var arguments in strict mode"
    );
}

#[test]
fn test_strict_no_let_arguments() {
    // Cannot use 'arguments' as variable name in strict mode
    let result = eval_result("let arguments = 1;");
    assert!(
        result.is_err(),
        "Should not allow let arguments in strict mode"
    );
}

#[test]
fn test_strict_no_const_arguments() {
    // Cannot use 'arguments' as variable name in strict mode
    let result = eval_result("const arguments = 1;");
    assert!(
        result.is_err(),
        "Should not allow const arguments in strict mode"
    );
}

#[test]
fn test_strict_no_param_eval() {
    // Cannot use 'eval' as parameter name
    let result = eval_result("function f(eval) { return eval; }");
    assert!(
        result.is_err(),
        "Should not allow eval as parameter in strict mode"
    );
}

#[test]
fn test_strict_no_param_arguments() {
    // Cannot use 'arguments' as parameter name
    let result = eval_result("function f(arguments) { return arguments; }");
    assert!(
        result.is_err(),
        "Should not allow arguments as parameter in strict mode"
    );
}

#[test]
fn test_strict_no_assign_eval() {
    // Cannot assign to 'eval'
    let result = eval_result("eval = 1;");
    assert!(
        result.is_err(),
        "Should not allow assignment to eval in strict mode"
    );
}

#[test]
fn test_strict_no_assign_arguments() {
    // Cannot assign to 'arguments' (in global scope)
    let result = eval_result("arguments = 1;");
    assert!(
        result.is_err(),
        "Should not allow assignment to arguments in strict mode"
    );
}

// ============================================================================
// Duplicate parameter names
// ============================================================================

#[test]
fn test_strict_no_duplicate_params() {
    // Duplicate parameter names are not allowed in strict mode
    let result = eval_result("function f(a, a) { return a; }");
    assert!(
        result.is_err(),
        "Should not allow duplicate parameter names in strict mode"
    );
}

#[test]
fn test_strict_no_duplicate_params_arrow() {
    // Duplicate parameter names in arrow functions
    let result = eval_result("const f = (a, a) => a;");
    assert!(
        result.is_err(),
        "Should not allow duplicate params in arrow functions"
    );
}

// ============================================================================
// Delete on unqualified identifier
// ============================================================================

#[test]
fn test_strict_no_delete_variable() {
    // Cannot delete unqualified identifier in strict mode
    let result = eval_result("var x = 1; delete x;");
    assert!(
        result.is_err(),
        "Should not allow delete on variable in strict mode"
    );
}

#[test]
fn test_strict_delete_property_allowed() {
    // Deleting object property is allowed
    let result = eval("const obj = { x: 1 }; delete obj.x; obj.x");
    assert_eq!(result.value(), &JsValue::Undefined);
}

// ============================================================================
// Octal literals
// ============================================================================

#[test]
fn test_strict_no_legacy_octal() {
    // Legacy octal literals (0777) are not allowed in strict mode
    let result = eval_result("0777");
    assert!(
        result.is_err(),
        "Should not allow legacy octal literals in strict mode"
    );
}

#[test]
fn test_strict_es6_octal_allowed() {
    // ES6 octal literals (0o777) are allowed
    let result = eval("0o777");
    assert_eq!(result.value(), &JsValue::Number(511.0));
}

#[test]
fn test_strict_no_octal_escape() {
    // Octal escape sequences in strings are not allowed
    let result = eval_result(r#""\077""#);
    assert!(
        result.is_err(),
        "Should not allow octal escape in string in strict mode"
    );
}

// ============================================================================
// With statement
// ============================================================================

#[test]
fn test_strict_no_with_statement() {
    // With statement is not allowed in strict mode
    let result = eval_result("with ({}) {}");
    assert!(
        result.is_err(),
        "Should not allow with statement in strict mode"
    );
}
