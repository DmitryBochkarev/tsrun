//! RegExp-related tests

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_regexp_test_basic() {
    assert_eq!(
        eval("new RegExp('abc').test('abc')"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("new RegExp('abc').test('def')"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_regexp_test_pattern() {
    assert_eq!(
        eval("new RegExp('a.c').test('abc')"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("new RegExp('a.c').test('adc')"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_regexp_case_insensitive() {
    assert_eq!(
        eval("new RegExp('abc', 'i').test('ABC')"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_regexp_source() {
    assert_eq!(eval("new RegExp('abc', 'gi').source"), JsValue::from("abc"));
}

#[test]
fn test_regexp_flags() {
    assert_eq!(eval("new RegExp('abc', 'gi').flags"), JsValue::from("gi"));
}

#[test]
fn test_regexp_exec_match() {
    assert_eq!(
        eval("new RegExp('a(b)c').exec('abc')[0]"),
        JsValue::from("abc")
    );
    assert_eq!(
        eval("new RegExp('a(b)c').exec('abc')[1]"),
        JsValue::from("b")
    );
}

#[test]
fn test_regexp_exec_no_match() {
    assert_eq!(eval("new RegExp('xyz').exec('abc')"), JsValue::Null);
}

// Tests for RegExp literals
#[test]
fn test_regexp_literal_basic() {
    assert_eq!(eval("/abc/.test('abc')"), JsValue::Boolean(true));
    assert_eq!(eval("/abc/.test('def')"), JsValue::Boolean(false));
}

#[test]
fn test_regexp_literal_with_flags() {
    assert_eq!(eval("/abc/i.test('ABC')"), JsValue::Boolean(true));
    assert_eq!(eval("/abc/gi.flags"), JsValue::from("gi"));
}

#[test]
fn test_regexp_literal_source() {
    assert_eq!(eval("/pattern/.source"), JsValue::from("pattern"));
}

#[test]
fn test_regexp_literal_exec() {
    assert_eq!(eval("/a(b)c/.exec('abc')[0]"), JsValue::from("abc"));
    assert_eq!(eval("/a(b)c/.exec('abc')[1]"), JsValue::from("b"));
}

#[test]
fn test_regexp_literal_in_variable() {
    assert_eq!(
        eval("const re: RegExp = /test/i; re.test('TEST')"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_regexp_literal_with_escapes() {
    assert_eq!(eval(r"/\d+/.test('123')"), JsValue::Boolean(true));
    assert_eq!(eval(r"/\d+/.test('abc')"), JsValue::Boolean(false));
}

// Tests for dotAll flag (s)
#[test]
fn test_regexp_dotall_flag() {
    // Without dotAll, . doesn't match newlines
    assert_eq!(eval(r"/a.b/.test('a\nb')"), JsValue::Boolean(false));
    // With dotAll, . matches newlines
    assert_eq!(eval(r"/a.b/s.test('a\nb')"), JsValue::Boolean(true));
}

#[test]
fn test_regexp_dotall_property() {
    assert_eq!(eval("/abc/s.dotAll"), JsValue::Boolean(true));
    assert_eq!(eval("/abc/.dotAll"), JsValue::Boolean(false));
}

// Tests for unicode flag (u)
#[test]
fn test_regexp_unicode_flag() {
    assert_eq!(eval("/abc/u.unicode"), JsValue::Boolean(true));
    assert_eq!(eval("/abc/.unicode"), JsValue::Boolean(false));
}

// Tests for sticky flag (y)
#[test]
fn test_regexp_sticky_flag() {
    assert_eq!(eval("/abc/y.sticky"), JsValue::Boolean(true));
    assert_eq!(eval("/abc/.sticky"), JsValue::Boolean(false));
}

#[test]
fn test_regexp_combined_flags() {
    // All flags at once
    assert_eq!(eval("/abc/gimsuy.global"), JsValue::Boolean(true));
    assert_eq!(eval("/abc/gimsuy.ignoreCase"), JsValue::Boolean(true));
    assert_eq!(eval("/abc/gimsuy.multiline"), JsValue::Boolean(true));
    assert_eq!(eval("/abc/gimsuy.dotAll"), JsValue::Boolean(true));
    assert_eq!(eval("/abc/gimsuy.unicode"), JsValue::Boolean(true));
    assert_eq!(eval("/abc/gimsuy.sticky"), JsValue::Boolean(true));
}
