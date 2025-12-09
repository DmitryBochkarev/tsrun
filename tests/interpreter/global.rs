//! Global function tests (parseInt, parseFloat, isNaN, isFinite, URI functions)

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_parseint() {
    assert_eq!(eval("parseInt('42')"), JsValue::Number(42.0));
    assert_eq!(eval("parseInt('  42  ')"), JsValue::Number(42.0));
    assert_eq!(eval("parseInt('42.5')"), JsValue::Number(42.0));
    assert_eq!(eval("parseInt('ff', 16)"), JsValue::Number(255.0));
    assert_eq!(eval("parseInt('101', 2)"), JsValue::Number(5.0));
}

#[test]
fn test_parsefloat() {
    assert_eq!(eval("parseFloat('3.14')"), JsValue::Number(3.14));
    assert_eq!(eval("parseFloat('  3.14  ')"), JsValue::Number(3.14));
    assert_eq!(eval("parseFloat('3.14abc')"), JsValue::Number(3.14));
}

#[test]
fn test_isnan() {
    assert_eq!(eval("isNaN(NaN)"), JsValue::Boolean(true));
    assert_eq!(eval("isNaN(42)"), JsValue::Boolean(false));
    assert_eq!(eval("isNaN('hello')"), JsValue::Boolean(true));
    assert_eq!(eval("isNaN('42')"), JsValue::Boolean(false));
}

#[test]
fn test_isfinite() {
    assert_eq!(eval("isFinite(42)"), JsValue::Boolean(true));
    assert_eq!(eval("isFinite(Infinity)"), JsValue::Boolean(false));
    assert_eq!(eval("isFinite(-Infinity)"), JsValue::Boolean(false));
    assert_eq!(eval("isFinite(NaN)"), JsValue::Boolean(false));
}

#[test]
fn test_encodeuri() {
    assert_eq!(
        eval("encodeURI('hello world')"),
        JsValue::from("hello%20world")
    );
    assert_eq!(eval("encodeURI('a=1&b=2')"), JsValue::from("a=1&b=2"));
    assert_eq!(
        eval("encodeURI('http://example.com/path?q=hello world')"),
        JsValue::from("http://example.com/path?q=hello%20world")
    );
}

#[test]
fn test_decodeuri() {
    assert_eq!(
        eval("decodeURI('hello%20world')"),
        JsValue::from("hello world")
    );
    assert_eq!(eval("decodeURI('a=1&b=2')"), JsValue::from("a=1&b=2"));
}

#[test]
fn test_encodeuricomponent() {
    assert_eq!(
        eval("encodeURIComponent('hello world')"),
        JsValue::from("hello%20world")
    );
    assert_eq!(
        eval("encodeURIComponent('a=1&b=2')"),
        JsValue::from("a%3D1%26b%3D2")
    );
    assert_eq!(
        eval("encodeURIComponent('http://example.com')"),
        JsValue::from("http%3A%2F%2Fexample.com")
    );
}

#[test]
fn test_decodeuricomponent() {
    assert_eq!(
        eval("decodeURIComponent('hello%20world')"),
        JsValue::from("hello world")
    );
    assert_eq!(
        eval("decodeURIComponent('a%3D1%26b%3D2')"),
        JsValue::from("a=1&b=2")
    );
}
