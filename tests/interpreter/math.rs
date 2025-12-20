//! Math-related tests

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_math_abs() {
    assert_eq!(eval("Math.abs(-5)"), JsValue::Number(5.0));
    assert_eq!(eval("Math.abs(5)"), JsValue::Number(5.0));
}

#[test]
fn test_math_floor_ceil_round() {
    assert_eq!(eval("Math.floor(4.7)"), JsValue::Number(4.0));
    assert_eq!(eval("Math.ceil(4.3)"), JsValue::Number(5.0));
    assert_eq!(eval("Math.round(4.5)"), JsValue::Number(5.0));
    assert_eq!(eval("Math.round(4.4)"), JsValue::Number(4.0));
}

#[test]
fn test_math_trunc_sign() {
    assert_eq!(eval("Math.trunc(4.7)"), JsValue::Number(4.0));
    assert_eq!(eval("Math.trunc(-4.7)"), JsValue::Number(-4.0));
    assert_eq!(eval("Math.sign(-5)"), JsValue::Number(-1.0));
    assert_eq!(eval("Math.sign(5)"), JsValue::Number(1.0));
    assert_eq!(eval("Math.sign(0)"), JsValue::Number(0.0));
}

#[test]
fn test_math_min_max() {
    assert_eq!(eval("Math.min(1, 2, 3)"), JsValue::Number(1.0));
    assert_eq!(eval("Math.max(1, 2, 3)"), JsValue::Number(3.0));
}

#[test]
fn test_math_pow_sqrt() {
    assert_eq!(eval("Math.pow(2, 3)"), JsValue::Number(8.0));
    assert_eq!(eval("Math.sqrt(16)"), JsValue::Number(4.0));
}

#[test]
fn test_math_log_exp() {
    assert_eq!(eval("Math.log(Math.E)"), JsValue::Number(1.0));
    assert_eq!(eval("Math.exp(0)"), JsValue::Number(1.0));
}

#[test]
fn test_math_constants() {
    assert!(
        matches!(*eval("Math.PI"), JsValue::Number(n) if (n - std::f64::consts::PI).abs() < 0.0001)
    );
    assert!(
        matches!(*eval("Math.E"), JsValue::Number(n) if (n - std::f64::consts::E).abs() < 0.0001)
    );
}

#[test]
fn test_math_random() {
    // Random should return a number between 0 and 1
    let result = eval("Math.random()");
    if let JsValue::Number(n) = *result {
        assert!(n >= 0.0 && n < 1.0);
    } else {
        panic!("Math.random() should return a number");
    }
}

#[test]
fn test_math_trig() {
    assert_eq!(eval("Math.sin(0)"), JsValue::Number(0.0));
    assert_eq!(eval("Math.cos(0)"), JsValue::Number(1.0));
}

#[test]
fn test_math_cbrt() {
    assert_eq!(eval("Math.cbrt(27)"), JsValue::Number(3.0));
    assert_eq!(eval("Math.cbrt(8)"), JsValue::Number(2.0));
    assert_eq!(eval("Math.cbrt(-8)"), JsValue::Number(-2.0));
}

#[test]
fn test_math_hypot() {
    assert_eq!(eval("Math.hypot(3, 4)"), JsValue::Number(5.0));
    assert_eq!(eval("Math.hypot(5, 12)"), JsValue::Number(13.0));
    assert_eq!(eval("Math.hypot()"), JsValue::Number(0.0));
}

#[test]
fn test_math_log10_log2() {
    assert_eq!(eval("Math.log10(100)"), JsValue::Number(2.0));
    assert_eq!(eval("Math.log10(1000)"), JsValue::Number(3.0));
    assert_eq!(eval("Math.log2(8)"), JsValue::Number(3.0));
    assert_eq!(eval("Math.log2(16)"), JsValue::Number(4.0));
}

#[test]
fn test_math_log1p_expm1() {
    // log1p(0) = 0
    assert_eq!(eval("Math.log1p(0)"), JsValue::Number(0.0));
    // expm1(0) = 0
    assert_eq!(eval("Math.expm1(0)"), JsValue::Number(0.0));
}

#[test]
fn test_math_inverse_trig() {
    assert_eq!(eval("Math.asin(0)"), JsValue::Number(0.0));
    assert_eq!(eval("Math.acos(1)"), JsValue::Number(0.0));
    assert_eq!(eval("Math.atan(0)"), JsValue::Number(0.0));
    assert_eq!(eval("Math.atan2(0, 1)"), JsValue::Number(0.0));
}

#[test]
fn test_math_hyperbolic() {
    assert_eq!(eval("Math.sinh(0)"), JsValue::Number(0.0));
    assert_eq!(eval("Math.cosh(0)"), JsValue::Number(1.0));
    assert_eq!(eval("Math.tanh(0)"), JsValue::Number(0.0));
    assert_eq!(eval("Math.asinh(0)"), JsValue::Number(0.0));
    assert_eq!(eval("Math.atanh(0)"), JsValue::Number(0.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Math with spread operator
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_math_spread_min() {
    // Test Math.min with spread
    assert_eq!(
        eval(
            r#"
            const arr: number[] = [5, 3, 9, 1];
            Math.min(...arr)
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_math_spread_max() {
    // Test Math.max with spread
    assert_eq!(
        eval(
            r#"
            const arr: number[] = [5, 3, 9, 1];
            Math.max(...arr)
        "#
        ),
        JsValue::Number(9.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Property Descriptor Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_math_constant_descriptors() {
    // Math constants should be non-writable, non-enumerable, non-configurable
    assert_eq!(
        eval(
            r#"
            const desc = Object.getOwnPropertyDescriptor(Math, 'E');
            [desc.writable, desc.enumerable, desc.configurable].join(',')
        "#
        ),
        JsValue::from("false,false,false")
    );

    assert_eq!(
        eval(
            r#"
            const desc = Object.getOwnPropertyDescriptor(Math, 'PI');
            [desc.writable, desc.enumerable, desc.configurable].join(',')
        "#
        ),
        JsValue::from("false,false,false")
    );
}

#[test]
fn test_math_method_descriptors() {
    // Math methods should be writable, non-enumerable, configurable
    assert_eq!(
        eval(
            r#"
            const desc = Object.getOwnPropertyDescriptor(Math, 'abs');
            [desc.writable, desc.enumerable, desc.configurable].join(',')
        "#
        ),
        JsValue::from("true,false,true")
    );
}
