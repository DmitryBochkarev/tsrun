//! String-related tests

use super::eval;
use typescript_eval::value::JsString;
use typescript_eval::JsValue;

#[test]
fn test_string_charat() {
    assert_eq!(
        eval("'hello'.charAt(1)"),
        JsValue::String(JsString::from("e"))
    );
}

#[test]
fn test_string_indexof() {
    assert_eq!(eval("'hello world'.indexOf('world')"), JsValue::Number(6.0));
    assert_eq!(eval("'hello'.indexOf('x')"), JsValue::Number(-1.0));
}

#[test]
fn test_string_includes() {
    assert_eq!(
        eval("'hello world'.includes('world')"),
        JsValue::Boolean(true)
    );
    assert_eq!(eval("'hello'.includes('x')"), JsValue::Boolean(false));
}

#[test]
fn test_string_startswith() {
    assert_eq!(
        eval("'hello world'.startsWith('hello')"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("'hello world'.startsWith('world')"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_string_endswith() {
    assert_eq!(
        eval("'hello world'.endsWith('world')"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("'hello world'.endsWith('hello')"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_string_slice() {
    assert_eq!(
        eval("'hello'.slice(1, 4)"),
        JsValue::String(JsString::from("ell"))
    );
    assert_eq!(
        eval("'hello'.slice(-2)"),
        JsValue::String(JsString::from("lo"))
    );
}

#[test]
fn test_string_substring() {
    assert_eq!(
        eval("'hello'.substring(1, 4)"),
        JsValue::String(JsString::from("ell"))
    );
}

#[test]
fn test_string_tolowercase() {
    assert_eq!(
        eval("'HELLO'.toLowerCase()"),
        JsValue::String(JsString::from("hello"))
    );
}

#[test]
fn test_string_touppercase() {
    assert_eq!(
        eval("'hello'.toUpperCase()"),
        JsValue::String(JsString::from("HELLO"))
    );
}

#[test]
fn test_string_trim() {
    assert_eq!(
        eval("'  hello  '.trim()"),
        JsValue::String(JsString::from("hello"))
    );
}

#[test]
fn test_string_trimstart() {
    assert_eq!(
        eval("'  hello  '.trimStart()"),
        JsValue::String(JsString::from("hello  "))
    );
}

#[test]
fn test_string_trimend() {
    assert_eq!(
        eval("'  hello  '.trimEnd()"),
        JsValue::String(JsString::from("  hello"))
    );
}

#[test]
fn test_string_split() {
    assert_eq!(eval("'a,b,c'.split(',').length"), JsValue::Number(3.0));
    assert_eq!(
        eval("'a,b,c'.split(',')[1]"),
        JsValue::String(JsString::from("b"))
    );
}

#[test]
fn test_string_repeat() {
    assert_eq!(
        eval("'ab'.repeat(3)"),
        JsValue::String(JsString::from("ababab"))
    );
}

#[test]
fn test_string_replace() {
    assert_eq!(
        eval("'hello world'.replace('world', 'rust')"),
        JsValue::String(JsString::from("hello rust"))
    );
}

#[test]
fn test_string_padstart() {
    assert_eq!(
        eval("'5'.padStart(3, '0')"),
        JsValue::String(JsString::from("005"))
    );
}

#[test]
fn test_string_padend() {
    assert_eq!(
        eval("'5'.padEnd(3, '0')"),
        JsValue::String(JsString::from("500"))
    );
}

#[test]
fn test_string_concat() {
    assert_eq!(
        eval("'hello'.concat(' ', 'world')"),
        JsValue::String(JsString::from("hello world"))
    );
}

#[test]
fn test_string_charat_index() {
    assert_eq!(eval("'hello'.charCodeAt(0)"), JsValue::Number(104.0));
    assert_eq!(eval("'hello'.charCodeAt(1)"), JsValue::Number(101.0));
}

#[test]
fn test_string_fromcharcode() {
    assert_eq!(
        eval("String.fromCharCode(104, 105)"),
        JsValue::String(JsString::from("hi"))
    );
}

#[test]
fn test_string_lastindexof() {
    assert_eq!(eval("'hello world'.lastIndexOf('o')"), JsValue::Number(7.0));
    assert_eq!(eval("'hello world'.lastIndexOf('l')"), JsValue::Number(9.0));
    assert_eq!(
        eval("'hello world'.lastIndexOf('x')"),
        JsValue::Number(-1.0)
    );
    assert_eq!(
        eval("'hello world'.lastIndexOf('o', 5)"),
        JsValue::Number(4.0)
    );
    assert_eq!(eval("'hello'.lastIndexOf('')"), JsValue::Number(5.0));
}

#[test]
fn test_string_at() {
    assert_eq!(
        eval("'hello'.at(0)"),
        JsValue::String(JsString::from("h"))
    );
    assert_eq!(
        eval("'hello'.at(1)"),
        JsValue::String(JsString::from("e"))
    );
    assert_eq!(
        eval("'hello'.at(-1)"),
        JsValue::String(JsString::from("o"))
    );
    assert_eq!(
        eval("'hello'.at(-2)"),
        JsValue::String(JsString::from("l"))
    );
    assert_eq!(eval("'hello'.at(10)"), JsValue::Undefined);
    assert_eq!(eval("'hello'.at(-10)"), JsValue::Undefined);
}

#[test]
fn test_string_replaceall() {
    assert_eq!(
        eval("'aabbcc'.replaceAll('b', 'x')"),
        JsValue::String(JsString::from("aaxxcc"))
    );
    assert_eq!(
        eval("'hello world'.replaceAll('o', '0')"),
        JsValue::String(JsString::from("hell0 w0rld"))
    );
    assert_eq!(
        eval("'aaa'.replaceAll('a', 'bb')"),
        JsValue::String(JsString::from("bbbbbb"))
    );
    assert_eq!(
        eval("'hello'.replaceAll('x', 'y')"),
        JsValue::String(JsString::from("hello"))
    );
    assert_eq!(
        eval("''.replaceAll('a', 'b')"),
        JsValue::String(JsString::from(""))
    );
}