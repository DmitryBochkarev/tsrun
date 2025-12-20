//! Number-related tests

use super::eval;
use typescript_eval::value::JsString;
use typescript_eval::JsValue;

#[test]
fn test_number_isnan() {
    assert_eq!(eval("Number.isNaN(NaN)"), JsValue::Boolean(true));
    assert_eq!(eval("Number.isNaN(42)"), JsValue::Boolean(false));
    assert_eq!(eval("Number.isNaN('NaN')"), JsValue::Boolean(false)); // Different from global isNaN
}

#[test]
fn test_number_isfinite() {
    assert_eq!(eval("Number.isFinite(42)"), JsValue::Boolean(true));
    assert_eq!(eval("Number.isFinite(Infinity)"), JsValue::Boolean(false));
    assert_eq!(eval("Number.isFinite('42')"), JsValue::Boolean(false)); // Different from global isFinite
}

#[test]
fn test_number_isinteger() {
    assert_eq!(eval("Number.isInteger(42)"), JsValue::Boolean(true));
    assert_eq!(eval("Number.isInteger(42.0)"), JsValue::Boolean(true));
    assert_eq!(eval("Number.isInteger(42.5)"), JsValue::Boolean(false));
    assert_eq!(eval("Number.isInteger('42')"), JsValue::Boolean(false));
}

#[test]
fn test_number_issafeinteger() {
    assert_eq!(eval("Number.isSafeInteger(42)"), JsValue::Boolean(true));
    assert_eq!(
        eval("Number.isSafeInteger(9007199254740991)"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("Number.isSafeInteger(9007199254740992)"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_number_constants() {
    assert_eq!(
        eval("Number.POSITIVE_INFINITY"),
        JsValue::Number(f64::INFINITY)
    );
    assert_eq!(
        eval("Number.NEGATIVE_INFINITY"),
        JsValue::Number(f64::NEG_INFINITY)
    );
    assert_eq!(
        eval("Number.MAX_SAFE_INTEGER"),
        JsValue::Number(9007199254740991.0)
    );
    assert_eq!(
        eval("Number.MIN_SAFE_INTEGER"),
        JsValue::Number(-9007199254740991.0)
    );
}

#[test]
fn test_number_tofixed() {
    assert_eq!(
        eval("(3.14159).toFixed(2)"),
        JsValue::String(JsString::from("3.14"))
    );
    assert_eq!(
        eval("(3.14159).toFixed(0)"),
        JsValue::String(JsString::from("3"))
    );
    assert_eq!(
        eval("(3.5).toFixed(0)"),
        JsValue::String(JsString::from("4"))
    );
}

#[test]
fn test_number_tostring() {
    assert_eq!(
        eval("(255).toString(16)"),
        JsValue::String(JsString::from("ff"))
    );
    assert_eq!(
        eval("(10).toString(2)"),
        JsValue::String(JsString::from("1010"))
    );
    assert_eq!(
        eval("(42).toString()"),
        JsValue::String(JsString::from("42"))
    );
}

#[test]
fn test_number_toprecision() {
    assert_eq!(
        eval("(123.456).toPrecision(4)"),
        JsValue::String(JsString::from("123.5"))
    );
    assert_eq!(
        eval("(0.000123).toPrecision(2)"),
        JsValue::String(JsString::from("0.00012"))
    );
    assert_eq!(
        eval("(1234.5).toPrecision(2)"),
        JsValue::String(JsString::from("1.2e+3"))
    );
}

#[test]
fn test_number_toexponential() {
    assert_eq!(
        eval("(123.456).toExponential(2)"),
        JsValue::String(JsString::from("1.23e+2"))
    );
    assert_eq!(
        eval("(0.00123).toExponential(2)"),
        JsValue::String(JsString::from("1.23e-3"))
    );
    assert_eq!(
        eval("(12345).toExponential(1)"),
        JsValue::String(JsString::from("1.2e+4"))
    );
}

#[test]
fn test_number_tostring_radix() {
    // Test Number.toString with different radixes
    assert_eq!(eval("(12).toString(2)"), JsValue::String("1100".into()));
    assert_eq!(eval("(255).toString(16)"), JsValue::String("ff".into()));
    assert_eq!(eval("(8).toString(8)"), JsValue::String("10".into()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Number Wrapper toString Tests (Test262 conformance)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_number_prototype_tostring_on_prototype() {
    // Number.prototype.toString() should return "0"
    assert_eq!(
        eval("Number.prototype.toString()"),
        JsValue::String(JsString::from("0"))
    );
}

#[test]
fn test_number_wrapper_tostring() {
    // new Number().toString() should work
    assert_eq!(
        eval("new Number().toString()"),
        JsValue::String(JsString::from("0"))
    );
    assert_eq!(
        eval("new Number(0).toString()"),
        JsValue::String(JsString::from("0"))
    );
    assert_eq!(
        eval("new Number(-1).toString()"),
        JsValue::String(JsString::from("-1"))
    );
    assert_eq!(
        eval("new Number(1).toString()"),
        JsValue::String(JsString::from("1"))
    );
    assert_eq!(
        eval("new Number(NaN).toString()"),
        JsValue::String(JsString::from("NaN"))
    );
    assert_eq!(
        eval("new Number(Infinity).toString()"),
        JsValue::String(JsString::from("Infinity"))
    );
    assert_eq!(
        eval("new Number(-Infinity).toString()"),
        JsValue::String(JsString::from("-Infinity"))
    );
}

#[test]
fn test_number_wrapper_tostring_radix() {
    // new Number(x).toString(radix) should work
    assert_eq!(
        eval("new Number(255).toString(16)"),
        JsValue::String(JsString::from("ff"))
    );
    assert_eq!(
        eval("new Number(8).toString(2)"),
        JsValue::String(JsString::from("1000"))
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Number Wrapper Object ToPrimitive Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_number_wrapper_valueof_exists() {
    // Check that valueOf exists on Number wrapper
    assert_eq!(
        eval("typeof new Number(1).valueOf"),
        JsValue::String("function".into())
    );
    // Check that calling valueOf returns the primitive value
    assert_eq!(eval("new Number(42).valueOf()"), JsValue::Number(42.0));
}

#[test]
fn test_number_wrapper_is_object() {
    // Check that new Number creates an object
    assert_eq!(
        eval("typeof new Number(42)"),
        JsValue::String("object".into())
    );
    // But Number() without new returns primitive
    assert_eq!(eval("typeof Number(42)"), JsValue::String("number".into()));
}

#[test]
fn test_number_wrapper_valueof_method() {
    // Check that we can access valueOf explicitly
    assert_eq!(
        eval("let n = new Number(42); n.valueOf()"),
        JsValue::Number(42.0)
    );
    // valueOf works in expressions too
    assert_eq!(
        eval("let n = new Number(5); n.valueOf() + 1"),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_number_wrapper_addition() {
    // new Number(1) + 1 should call valueOf via ToPrimitive and result in 2
    assert_eq!(eval("new Number(1) + 1"), JsValue::Number(2.0));
    assert_eq!(eval("1 + new Number(2)"), JsValue::Number(3.0));
    assert_eq!(eval("new Number(5) + new Number(3)"), JsValue::Number(8.0));
    // Addition with variable (triggers ToPrimitive path)
    assert_eq!(eval("let n = new Number(5); n + 1"), JsValue::Number(6.0));
}

#[test]
fn test_number_wrapper_arithmetic() {
    // Division
    assert_eq!(eval("new Number(10) / new Number(2)"), JsValue::Number(5.0));
    // Multiplication
    assert_eq!(eval("new Number(3) * new Number(4)"), JsValue::Number(12.0));
    // Subtraction
    assert_eq!(eval("new Number(10) - new Number(3)"), JsValue::Number(7.0));
}

#[test]
fn test_number_wrapper_string_concat() {
    // When concatenating with string, should call toString
    assert_eq!(eval("'' + new Number(42)"), JsValue::String("42".into()));
    assert_eq!(eval("new Number(42) + ''"), JsValue::String("42".into()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Number() String Conversion Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_number_from_empty_string() {
    // Empty string should convert to 0
    assert_eq!(eval("Number('')"), JsValue::Number(0.0));
}

#[test]
fn test_number_from_whitespace_string() {
    // Whitespace-only strings should convert to 0
    assert_eq!(eval("Number(' ')"), JsValue::Number(0.0));
    assert_eq!(eval("Number('  ')"), JsValue::Number(0.0));
    assert_eq!(eval("Number('\\t')"), JsValue::Number(0.0));
    assert_eq!(eval("Number('\\n')"), JsValue::Number(0.0));
    assert_eq!(eval("Number('  \\t\\n  ')"), JsValue::Number(0.0));
}

#[test]
fn test_number_from_hex_string() {
    // Hex strings should be parsed correctly
    assert_eq!(eval("Number('0x0')"), JsValue::Number(0.0));
    assert_eq!(eval("Number('0X0')"), JsValue::Number(0.0));
    assert_eq!(eval("Number('0x1')"), JsValue::Number(1.0));
    assert_eq!(eval("Number('0xff')"), JsValue::Number(255.0));
    assert_eq!(eval("Number('0xFF')"), JsValue::Number(255.0));
    assert_eq!(eval("Number('0xA')"), JsValue::Number(10.0));
}

#[test]
fn test_number_from_string_with_whitespace() {
    // Leading/trailing whitespace should be trimmed
    assert_eq!(eval("Number('  42  ')"), JsValue::Number(42.0));
    assert_eq!(eval("Number('\\t123\\n')"), JsValue::Number(123.0));
    assert_eq!(eval("Number('  0x10  ')"), JsValue::Number(16.0));
}

#[test]
fn test_number_from_octal_string() {
    // Octal strings (0o prefix) should be parsed
    assert_eq!(eval("Number('0o10')"), JsValue::Number(8.0));
    assert_eq!(eval("Number('0O17')"), JsValue::Number(15.0));
}

#[test]
fn test_number_from_binary_string() {
    // Binary strings (0b prefix) should be parsed
    assert_eq!(eval("Number('0b10')"), JsValue::Number(2.0));
    assert_eq!(eval("Number('0B1010')"), JsValue::Number(10.0));
}

#[test]
fn test_number_from_infinity_string() {
    // "Infinity" and "-Infinity" should parse correctly
    assert_eq!(eval("Number('Infinity')"), JsValue::Number(f64::INFINITY));
    assert_eq!(
        eval("Number('-Infinity')"),
        JsValue::Number(f64::NEG_INFINITY)
    );
    assert_eq!(eval("Number('+Infinity')"), JsValue::Number(f64::INFINITY));
    // But "INFINITY" (uppercase) should be NaN
    assert!(eval("Number('INFINITY')").to_number().is_nan());
}

#[test]
fn test_number_from_invalid_string() {
    // Invalid strings should return NaN
    assert!(eval("Number('abc')").to_number().is_nan());
    assert!(eval("Number('12abc')").to_number().is_nan());
    assert!(eval("Number('0x')").to_number().is_nan());
    assert!(eval("Number('0xg')").to_number().is_nan());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Decimal Literal With Exponent Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_decimal_with_exponent_no_fraction() {
    // Format: DecimalIntegerLiteral. ExponentPart (no fractional digits)
    assert_eq!(eval("0.e1"), JsValue::Number(0.0));
    assert_eq!(eval("1.e1"), JsValue::Number(10.0));
    assert_eq!(eval("2.e1"), JsValue::Number(20.0));
    assert_eq!(eval("5.e2"), JsValue::Number(500.0));
    assert_eq!(eval("1.E1"), JsValue::Number(10.0));
    assert_eq!(eval("0.e-1"), JsValue::Number(0.0));
    assert_eq!(eval("1.e-1"), JsValue::Number(0.1));
    assert_eq!(eval("0.e+1"), JsValue::Number(0.0));
    assert_eq!(eval("1.e+1"), JsValue::Number(10.0));
    assert_eq!(eval("0.e0"), JsValue::Number(0.0));
    assert_eq!(eval("1.e0"), JsValue::Number(1.0));
}
