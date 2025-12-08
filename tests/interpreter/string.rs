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

// String.prototype.match tests
#[test]
fn test_string_match_basic() {
    // match with string pattern returns first match
    assert_eq!(
        eval("'hello world'.match('world')[0]"),
        JsValue::String(JsString::from("world"))
    );
}

#[test]
fn test_string_match_no_match() {
    // No match returns null
    assert_eq!(
        eval("'hello'.match('xyz')"),
        JsValue::Null
    );
}

#[test]
fn test_string_match_regexp() {
    // match with RegExp (non-global)
    assert_eq!(
        eval("'hello world'.match(/o/)[0]"),
        JsValue::String(JsString::from("o"))
    );
}

#[test]
fn test_string_match_global() {
    // match with global flag returns array of all matches
    assert_eq!(
        eval("'hello world'.match(/o/g).length"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_string_match_index() {
    // Non-global match includes index property
    assert_eq!(
        eval("'hello world'.match('world').index"),
        JsValue::Number(6.0)
    );
}

// String.prototype.search tests
#[test]
fn test_string_search_basic() {
    assert_eq!(
        eval("'hello world'.search('world')"),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_string_search_not_found() {
    assert_eq!(
        eval("'hello'.search('xyz')"),
        JsValue::Number(-1.0)
    );
}

#[test]
fn test_string_search_regexp() {
    assert_eq!(
        eval("'hello world'.search(/o/)"),
        JsValue::Number(4.0)
    );
}

// String.prototype.matchAll tests
#[test]
fn test_string_matchall_basic() {
    // matchAll returns array of match results
    assert_eq!(
        eval("Array.from('hello world'.matchAll(/o/g)).length"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_string_matchall_index() {
    // Each result has index property
    assert_eq!(
        eval("Array.from('hello world'.matchAll(/o/g))[0].index"),
        JsValue::Number(4.0)
    );
    assert_eq!(
        eval("Array.from('hello world'.matchAll(/o/g))[1].index"),
        JsValue::Number(7.0)
    );
}

// String.fromCodePoint tests
#[test]
fn test_string_from_code_point_basic() {
    // Basic ASCII
    assert_eq!(
        eval("String.fromCodePoint(65)"),
        JsValue::String(JsString::from("A"))
    );
}

#[test]
fn test_string_from_code_point_multiple() {
    // Multiple code points
    assert_eq!(
        eval("String.fromCodePoint(72, 101, 108, 108, 111)"),
        JsValue::String(JsString::from("Hello"))
    );
}

#[test]
fn test_string_from_code_point_emoji() {
    // Emoji (supplementary character)
    assert_eq!(
        eval("String.fromCodePoint(0x1F600)"),
        JsValue::String(JsString::from("😀"))
    );
}

#[test]
fn test_string_from_code_point_unicode() {
    // Various Unicode characters
    assert_eq!(
        eval("String.fromCodePoint(0x2764)"),
        JsValue::String(JsString::from("❤"))
    );
}

// String.prototype.codePointAt tests
#[test]
fn test_string_code_point_at_basic() {
    // Basic ASCII
    assert_eq!(
        eval("'ABC'.codePointAt(0)"),
        JsValue::Number(65.0)
    );
    assert_eq!(
        eval("'ABC'.codePointAt(1)"),
        JsValue::Number(66.0)
    );
}

#[test]
fn test_string_code_point_at_emoji() {
    // Emoji (surrogate pair in JS, but we handle as code point)
    assert_eq!(
        eval("'😀'.codePointAt(0)"),
        JsValue::Number(128512.0)  // 0x1F600
    );
}

#[test]
fn test_string_code_point_at_out_of_range() {
    // Out of range returns undefined
    assert_eq!(
        eval("'abc'.codePointAt(5)"),
        JsValue::Undefined
    );
}

#[test]
fn test_string_code_point_at_negative() {
    // Negative index returns undefined
    assert_eq!(
        eval("'abc'.codePointAt(-1)"),
        JsValue::Undefined
    );
}

// String.prototype.normalize tests
#[test]
fn test_string_normalize_default() {
    // Default is NFC
    assert_eq!(
        eval("'café'.normalize()"),
        eval("'café'.normalize('NFC')")
    );
}

#[test]
fn test_string_normalize_nfc() {
    // NFC should compose characters
    // é (e + combining acute) becomes é (single character)
    assert_eq!(
        eval("'\\u0065\\u0301'.normalize('NFC')"),
        JsValue::String(JsString::from("é"))
    );
}

#[test]
fn test_string_normalize_nfd() {
    // NFD should decompose characters
    // Use the composed form (U+00E9) and check that NFD produces a longer string than NFC
    // Since NFD decomposes 'é' into 'e' + combining acute
    let nfc_len = eval("'\\u00E9'.normalize('NFC').length");
    let nfd_len = eval("'\\u00E9'.normalize('NFD').length");

    // NFC should be 1 character (composed)
    // NFD should be 2 characters (decomposed: e + combining accent)
    if let (JsValue::Number(nfc), JsValue::Number(nfd)) = (nfc_len, nfd_len) {
        assert!(nfd > nfc, "NFD should produce a longer string than NFC for composed characters");
    }
}

#[test]
fn test_string_normalize_ascii() {
    // ASCII strings should be unchanged
    assert_eq!(
        eval("'hello'.normalize()"),
        JsValue::String(JsString::from("hello"))
    );
}

// String.prototype.localeCompare tests
#[test]
fn test_string_locale_compare_equal() {
    assert_eq!(
        eval("'abc'.localeCompare('abc')"),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_string_locale_compare_less() {
    assert_eq!(
        eval("'abc'.localeCompare('abd')"),
        JsValue::Number(-1.0)
    );
}

#[test]
fn test_string_locale_compare_greater() {
    assert_eq!(
        eval("'abd'.localeCompare('abc')"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_string_locale_compare_empty() {
    // Empty string comes before any non-empty string
    assert_eq!(
        eval("''.localeCompare('a')"),
        JsValue::Number(-1.0)
    );
}