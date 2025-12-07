//! Console-related tests

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_console_log() {
    assert_eq!(eval("console.log('test')"), JsValue::Undefined);
}

#[test]
fn test_console_error() {
    assert_eq!(eval("console.error('test')"), JsValue::Undefined);
}

#[test]
fn test_console_warn() {
    assert_eq!(eval("console.warn('test')"), JsValue::Undefined);
}

#[test]
fn test_console_info() {
    assert_eq!(eval("console.info('test')"), JsValue::Undefined);
}

#[test]
fn test_console_debug() {
    assert_eq!(eval("console.debug('test')"), JsValue::Undefined);
}