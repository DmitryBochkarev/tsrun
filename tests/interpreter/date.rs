//! Date-related tests

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_date_now() {
    // Date.now() returns a number (timestamp)
    let result = eval("Date.now()");
    assert!(matches!(*result, JsValue::Number(_)));
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
        eval(
            r#"
            const d = new Date(0);
            d.setTime(1000);
            d.getTime()
        "#
        ),
        JsValue::Number(1000.0)
    );
}

#[test]
fn test_date_setfullyear() {
    assert_eq!(
        eval(
            r#"
            const d = new Date(0);
            d.setFullYear(2000);
            d.getFullYear()
        "#
        ),
        JsValue::Number(2000.0)
    );
}

#[test]
fn test_date_setmonth() {
    assert_eq!(
        eval(
            r#"
            const d = new Date(0);
            d.setMonth(5);
            d.getMonth()
        "#
        ),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_date_setdate() {
    assert_eq!(
        eval(
            r#"
            const d = new Date(0);
            d.setDate(15);
            d.getDate()
        "#
        ),
        JsValue::Number(15.0)
    );
}

#[test]
fn test_date_sethours() {
    assert_eq!(
        eval(
            r#"
            const d = new Date(0);
            d.setHours(12);
            d.getHours()
        "#
        ),
        JsValue::Number(12.0)
    );
}

#[test]
fn test_date_setminutes() {
    assert_eq!(
        eval(
            r#"
            const d = new Date(0);
            d.setMinutes(30);
            d.getMinutes()
        "#
        ),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_date_setseconds() {
    assert_eq!(
        eval(
            r#"
            const d = new Date(0);
            d.setSeconds(45);
            d.getSeconds()
        "#
        ),
        JsValue::Number(45.0)
    );
}

#[test]
fn test_date_setmilliseconds() {
    assert_eq!(
        eval(
            r#"
            const d = new Date(0);
            d.setMilliseconds(500);
            d.getMilliseconds()
        "#
        ),
        JsValue::Number(500.0)
    );
}

#[test]
fn test_date_setfullyear_returns_timestamp() {
    // setFullYear should return the new timestamp
    assert!(matches!(
        *eval(
            r#"
            const d = new Date(0);
            d.setFullYear(2000)
        "#
        ),
        JsValue::Number(_)
    ));
}

// Date toString method tests
#[test]
fn test_date_tostring() {
    // toString returns a string representation of the date
    let result = eval("new Date(0).toString()");
    assert!(matches!(*result, JsValue::String(_)));
    // Check that it contains the year
    if let JsValue::String(s) = &*result {
        assert!(s.to_string().contains("1970"));
    }
}

#[test]
fn test_date_todatestring() {
    // toDateString returns just the date part
    let result = eval("new Date(0).toDateString()");
    assert!(matches!(*result, JsValue::String(_)));
    if let JsValue::String(s) = &*result {
        let s_str = s.to_string();
        assert!(s_str.contains("1970"));
        assert!(s_str.contains("Jan"));
    }
}

#[test]
fn test_date_totimestring() {
    // toTimeString returns just the time part
    let result = eval("new Date(0).toTimeString()");
    assert!(matches!(*result, JsValue::String(_)));
    if let JsValue::String(s) = &*result {
        assert!(s.to_string().contains("00:00:00"));
    }
}

// Date string parsing tests
#[test]
fn test_date_from_iso_string() {
    // new Date with ISO string should parse correctly
    assert_eq!(
        eval(r#"new Date("2024-12-25T10:30:00Z").toISOString()"#),
        JsValue::from("2024-12-25T10:30:00.000Z")
    );
}

#[test]
fn test_date_from_iso_string_no_timezone() {
    // ISO string without timezone should be treated as UTC
    assert_eq!(
        eval(r#"new Date("2024-12-25T10:30:00").toISOString()"#),
        JsValue::from("2024-12-25T10:30:00.000Z")
    );
}

#[test]
fn test_date_from_date_string() {
    // new Date with date-only string
    assert_eq!(
        eval(r#"new Date("2024-12-25").getFullYear()"#),
        JsValue::Number(2024.0)
    );
}

#[test]
fn test_date_from_components() {
    // new Date(year, month, day, hours, minutes, seconds)
    // Month is 0-indexed, so 11 = December
    assert_eq!(
        eval("new Date(2024, 11, 25, 10, 30, 0).getFullYear()"),
        JsValue::Number(2024.0)
    );
    assert_eq!(
        eval("new Date(2024, 11, 25, 10, 30, 0).getMonth()"),
        JsValue::Number(11.0)
    );
    assert_eq!(
        eval("new Date(2024, 11, 25, 10, 30, 0).getDate()"),
        JsValue::Number(25.0)
    );
}

// UTC getter tests
#[test]
fn test_date_getutchours() {
    assert_eq!(
        eval(r#"new Date("2024-07-15T14:30:45.123Z").getUTCHours()"#),
        JsValue::Number(14.0)
    );
}

#[test]
fn test_date_getutcminutes() {
    assert_eq!(
        eval(r#"new Date("2024-07-15T14:30:45.123Z").getUTCMinutes()"#),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_date_getutcseconds() {
    assert_eq!(
        eval(r#"new Date("2024-07-15T14:30:45.123Z").getUTCSeconds()"#),
        JsValue::Number(45.0)
    );
}

#[test]
fn test_date_getutcmilliseconds() {
    assert_eq!(
        eval(r#"new Date("2024-07-15T14:30:45.123Z").getUTCMilliseconds()"#),
        JsValue::Number(123.0)
    );
}

// Date with day=0 returns last day of previous month
#[test]
fn test_date_day_zero() {
    // new Date(year, month + 1, 0) should return last day of month
    // March (month 2) 2024 has 31 days, so new Date(2024, 3, 0) gives March 31
    assert_eq!(
        eval("new Date(2024, 3, 0).getDate()"),
        JsValue::Number(31.0)
    );
    // February 2024 (leap year) has 29 days
    assert_eq!(
        eval("new Date(2024, 2, 0).getDate()"),
        JsValue::Number(29.0)
    );
    // February 2023 (non-leap year) has 28 days
    assert_eq!(
        eval("new Date(2023, 2, 0).getDate()"),
        JsValue::Number(28.0)
    );
}

#[test]
fn test_date_called_as_function() {
    // Date() called without 'new' returns a string, not a Date object
    let result = eval("typeof Date()");
    assert_eq!(result, JsValue::String("string".into()));

    // Date() ignores arguments and returns current date as string
    let result = eval("typeof Date(2024, 0, 1)");
    assert_eq!(result, JsValue::String("string".into()));

    // Date() returns a formatted date string
    let result = eval("Date()");
    // Should match pattern like "Thu Dec 19 2024 12:34:56 GMT+0000 (UTC)"
    if let JsValue::String(s) = &*result {
        // Check it contains expected parts
        assert!(
            s.as_str().contains("GMT"),
            "Date() should contain 'GMT': {}",
            s
        );
    } else {
        panic!("Date() should return a string");
    }
}

#[test]
fn test_new_date_creates_object() {
    // new Date() creates an object, not a string
    assert_eq!(eval("typeof new Date()"), JsValue::String("object".into()));
    assert_eq!(eval("typeof new Date(0)"), JsValue::String("object".into()));
}

// Date ToPrimitive/addition tests
#[test]
fn test_date_addition_uses_tostring() {
    // Date objects use hint "string" for ToPrimitive by default (for "default" hint).
    // This is what Date.prototype[@@toPrimitive] does.
    // So date + date should concatenate their string representations.
    let result = eval(
        r#"
        var date = new Date(0);
        date + date === date.toString() + date.toString()
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_date_plus_number() {
    // date + 0 should be date.toString() + "0"
    let result = eval(
        r#"
        var date = new Date(0);
        date + 0 === date.toString() + "0"
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_date_plus_boolean() {
    // date + true should be date.toString() + "true"
    let result = eval(
        r#"
        var date = new Date(0);
        date + true === date.toString() + "true"
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_date_plus_object() {
    // date + {} should be date.toString() + "[object Object]"
    let result = eval(
        r#"
        var date = new Date(0);
        date + {} === date.toString() + "[object Object]"
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}
