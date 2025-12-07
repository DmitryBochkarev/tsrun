//! Date-related tests

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_date_now() {
    // Date.now() returns a number (timestamp)
    let result = eval("Date.now()");
    assert!(matches!(result, JsValue::Number(_)));
}

#[test]
fn test_date_with_timestamp() {
    // new Date() with timestamp
    assert_eq!(eval("new Date(0).getTime()"), JsValue::Number(0.0));
    assert_eq!(eval("new Date(1000).getTime()"), JsValue::Number(1000.0));
}

#[test]
fn test_date_getfullyear() {
    assert_eq!(eval("new Date(0).getFullYear()"), JsValue::Number(1970.0));
}

#[test]
fn test_date_getmonth() {
    assert_eq!(eval("new Date(0).getMonth()"), JsValue::Number(0.0)); // January = 0
}

#[test]
fn test_date_getdate() {
    assert_eq!(eval("new Date(0).getDate()"), JsValue::Number(1.0));
}

#[test]
fn test_date_utc() {
    assert_eq!(eval("Date.UTC(1970, 0, 1)"), JsValue::Number(0.0));
}

#[test]
fn test_date_toisostring() {
    assert_eq!(
        eval("new Date(0).toISOString()"),
        JsValue::from("1970-01-01T00:00:00.000Z")
    );
}