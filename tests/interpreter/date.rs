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

// Date setter method tests
#[test]
fn test_date_settime() {
    assert_eq!(
        eval(r#"
            const d = new Date(0);
            d.setTime(1000);
            d.getTime()
        "#),
        JsValue::Number(1000.0)
    );
}

#[test]
fn test_date_setfullyear() {
    assert_eq!(
        eval(r#"
            const d = new Date(0);
            d.setFullYear(2000);
            d.getFullYear()
        "#),
        JsValue::Number(2000.0)
    );
}

#[test]
fn test_date_setmonth() {
    assert_eq!(
        eval(r#"
            const d = new Date(0);
            d.setMonth(5);
            d.getMonth()
        "#),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_date_setdate() {
    assert_eq!(
        eval(r#"
            const d = new Date(0);
            d.setDate(15);
            d.getDate()
        "#),
        JsValue::Number(15.0)
    );
}

#[test]
fn test_date_sethours() {
    assert_eq!(
        eval(r#"
            const d = new Date(0);
            d.setHours(12);
            d.getHours()
        "#),
        JsValue::Number(12.0)
    );
}

#[test]
fn test_date_setminutes() {
    assert_eq!(
        eval(r#"
            const d = new Date(0);
            d.setMinutes(30);
            d.getMinutes()
        "#),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_date_setseconds() {
    assert_eq!(
        eval(r#"
            const d = new Date(0);
            d.setSeconds(45);
            d.getSeconds()
        "#),
        JsValue::Number(45.0)
    );
}

#[test]
fn test_date_setmilliseconds() {
    assert_eq!(
        eval(r#"
            const d = new Date(0);
            d.setMilliseconds(500);
            d.getMilliseconds()
        "#),
        JsValue::Number(500.0)
    );
}

#[test]
fn test_date_setfullyear_returns_timestamp() {
    // setFullYear should return the new timestamp
    assert!(matches!(
        eval(r#"
            const d = new Date(0);
            d.setFullYear(2000)
        "#),
        JsValue::Number(_)
    ));
}

// Date toString method tests
#[test]
fn test_date_tostring() {
    // toString returns a string representation of the date
    let result = eval("new Date(0).toString()");
    assert!(matches!(result, JsValue::String(_)));
    // Check that it contains the year
    if let JsValue::String(s) = result {
        assert!(s.to_string().contains("1970"));
    }
}

#[test]
fn test_date_todatestring() {
    // toDateString returns just the date part
    let result = eval("new Date(0).toDateString()");
    assert!(matches!(result, JsValue::String(_)));
    if let JsValue::String(s) = result {
        let s_str = s.to_string();
        assert!(s_str.contains("1970"));
        assert!(s_str.contains("Jan"));
    }
}

#[test]
fn test_date_totimestring() {
    // toTimeString returns just the time part
    let result = eval("new Date(0).toTimeString()");
    assert!(matches!(result, JsValue::String(_)));
    if let JsValue::String(s) = result {
        assert!(s.to_string().contains("00:00:00"));
    }
}