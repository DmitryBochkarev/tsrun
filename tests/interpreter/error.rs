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

// Error.prototype.toString tests
#[test]
fn test_error_tostring_basic() {
    // Standard format: "ErrorName: message"
    assert_eq!(
        eval("new Error('something went wrong').toString()"),
        JsValue::from("Error: something went wrong")
    );
}

#[test]
fn test_error_tostring_no_message() {
    // When message is empty, just return name
    assert_eq!(
        eval("new Error().toString()"),
        JsValue::from("Error")
    );
}

#[test]
fn test_error_tostring_typeerror() {
    assert_eq!(
        eval("new TypeError('invalid argument').toString()"),
        JsValue::from("TypeError: invalid argument")
    );
}

#[test]
fn test_error_tostring_referenceerror() {
    assert_eq!(
        eval("new ReferenceError('x is not defined').toString()"),
        JsValue::from("ReferenceError: x is not defined")
    );
}

#[test]
fn test_error_tostring_custom() {
    // Custom name and message
    assert_eq!(
        eval(r#"
            const e = new Error('oops');
            e.name = 'CustomError';
            e.toString()
        "#),
        JsValue::from("CustomError: oops")
    );
}