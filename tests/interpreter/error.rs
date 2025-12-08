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

// Stack trace tests
#[test]
fn test_error_stack_exists() {
    // Error objects should have a stack property
    assert_eq!(
        eval("typeof new Error('test').stack"),
        JsValue::from("string")
    );
}

#[test]
fn test_error_stack_contains_error_name() {
    // Stack should start with error type and message
    assert_eq!(
        eval("new Error('test message').stack.includes('Error: test message')"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_error_stack_in_function() {
    // Stack should include function names
    assert_eq!(
        eval(r#"
            function foo(): Error {
                return new Error('in foo');
            }
            foo().stack.includes('foo')
        "#),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_urierror() {
    assert_eq!(
        eval("new URIError('invalid URI').name"),
        JsValue::from("URIError")
    );
    assert_eq!(
        eval("new URIError('invalid URI').message"),
        JsValue::from("invalid URI")
    );
    assert_eq!(
        eval("new URIError('malformed').toString()"),
        JsValue::from("URIError: malformed")
    );
}

#[test]
fn test_evalerror() {
    assert_eq!(
        eval("new EvalError('eval failed').name"),
        JsValue::from("EvalError")
    );
    assert_eq!(
        eval("new EvalError('eval failed').message"),
        JsValue::from("eval failed")
    );
    assert_eq!(
        eval("new EvalError('bad eval').toString()"),
        JsValue::from("EvalError: bad eval")
    );
}