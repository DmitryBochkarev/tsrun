//! Error-related tests

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_error_constructor() {
    assert_eq!(eval("new Error('oops').message"), JsValue::from("oops"));
    assert_eq!(eval("new Error('oops').name"), JsValue::from("Error"));
}

#[test]
fn test_typeerror() {
    assert_eq!(
        eval("new TypeError('bad type').name"),
        JsValue::from("TypeError")
    );
    assert_eq!(
        eval("new TypeError('bad type').message"),
        JsValue::from("bad type")
    );
}

#[test]
fn test_rangeerror() {
    assert_eq!(
        eval("new RangeError('out of range').name"),
        JsValue::from("RangeError")
    );
    assert_eq!(
        eval("new RangeError('out of range').message"),
        JsValue::from("out of range")
    );
}