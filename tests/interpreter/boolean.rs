//! Boolean tests

use super::eval;
use typescript_eval::JsValue;

// ═══════════════════════════════════════════════════════════════════════════════
// Boolean Constructor
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_boolean_defined() {
    // Boolean constructor should be defined globally
    assert_eq!(eval("typeof Boolean"), JsValue::String("function".into()));
}

#[test]
fn test_boolean_call_no_args() {
    // Boolean() with no arguments returns false
    assert_eq!(eval("Boolean()"), JsValue::Boolean(false));
}

#[test]
fn test_boolean_call_undefined() {
    assert_eq!(eval("Boolean(undefined)"), JsValue::Boolean(false));
}

#[test]
fn test_boolean_call_null() {
    assert_eq!(eval("Boolean(null)"), JsValue::Boolean(false));
}

#[test]
fn test_boolean_call_false() {
    assert_eq!(eval("Boolean(false)"), JsValue::Boolean(false));
}

#[test]
fn test_boolean_call_true() {
    assert_eq!(eval("Boolean(true)"), JsValue::Boolean(true));
}

#[test]
fn test_boolean_call_zero() {
    assert_eq!(eval("Boolean(0)"), JsValue::Boolean(false));
}

#[test]
fn test_boolean_call_negative_zero() {
    assert_eq!(eval("Boolean(-0)"), JsValue::Boolean(false));
}

#[test]
fn test_boolean_call_nan() {
    assert_eq!(eval("Boolean(NaN)"), JsValue::Boolean(false));
}

#[test]
fn test_boolean_call_empty_string() {
    assert_eq!(eval("Boolean('')"), JsValue::Boolean(false));
}

#[test]
fn test_boolean_call_nonzero_number() {
    assert_eq!(eval("Boolean(1)"), JsValue::Boolean(true));
    assert_eq!(eval("Boolean(-1)"), JsValue::Boolean(true));
    assert_eq!(eval("Boolean(42)"), JsValue::Boolean(true));
}

#[test]
fn test_boolean_call_nonempty_string() {
    assert_eq!(eval("Boolean('hello')"), JsValue::Boolean(true));
    assert_eq!(eval("Boolean('false')"), JsValue::Boolean(true));
    assert_eq!(eval("Boolean('0')"), JsValue::Boolean(true));
}

#[test]
fn test_boolean_call_object() {
    assert_eq!(eval("Boolean({})"), JsValue::Boolean(true));
    assert_eq!(eval("Boolean([])"), JsValue::Boolean(true));
    assert_eq!(eval("Boolean(function() {})"), JsValue::Boolean(true));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Boolean Constructor with `new`
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_boolean_new_creates_object() {
    assert_eq!(
        eval("typeof new Boolean(true)"),
        JsValue::String("object".into())
    );
}

#[test]
fn test_boolean_new_false() {
    // new Boolean(false) is truthy object wrapping false
    assert_eq!(
        eval("new Boolean(false) ? 'truthy' : 'falsy'"),
        JsValue::String("truthy".into())
    );
}

#[test]
fn test_boolean_new_valueof() {
    // Boolean objects have valueOf that returns the primitive
    assert_eq!(eval("new Boolean(true).valueOf()"), JsValue::Boolean(true));
    assert_eq!(
        eval("new Boolean(false).valueOf()"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_boolean_new_tostring() {
    assert_eq!(
        eval("new Boolean(true).toString()"),
        JsValue::String("true".into())
    );
    assert_eq!(
        eval("new Boolean(false).toString()"),
        JsValue::String("false".into())
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Boolean.prototype
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_boolean_prototype_exists() {
    assert_eq!(
        eval("typeof Boolean.prototype"),
        JsValue::String("object".into())
    );
}

#[test]
fn test_boolean_prototype_valueof() {
    // valueOf on primitive boolean (via prototype)
    // First check that the method exists on the prototype
    assert_eq!(
        eval("typeof Boolean.prototype.valueOf"),
        JsValue::String("function".into())
    );
    assert_eq!(eval("true.valueOf()"), JsValue::Boolean(true));
    assert_eq!(eval("false.valueOf()"), JsValue::Boolean(false));
}

#[test]
fn test_boolean_prototype_tostring() {
    // toString on primitive boolean (via prototype)
    assert_eq!(eval("true.toString()"), JsValue::String("true".into()));
    assert_eq!(eval("false.toString()"), JsValue::String("false".into()));
}

#[test]
fn test_boolean_prototype_constructor() {
    assert_eq!(
        eval("Boolean.prototype.constructor === Boolean"),
        JsValue::Boolean(true)
    );
}
