//! String-related tests

use super::{eval, eval_result};
use tsrun::JsValue;
use tsrun::value::JsString;

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
fn test_string_fromcharcode_touint16() {
    // ToUint16 wraps values to 0-65535 range
    // -1 should wrap to 65535
    assert_eq!(
        eval("String.fromCharCode(-1).charCodeAt(0)"),
        JsValue::Number(65535.0)
    );
    // 65536 should wrap to 0
    assert_eq!(
        eval("String.fromCharCode(65536).charCodeAt(0)"),
        JsValue::Number(0.0)
    );
    // 65537 should wrap to 1
    assert_eq!(
        eval("String.fromCharCode(65537).charCodeAt(0)"),
        JsValue::Number(1.0)
    );
    // Large negative number
    assert_eq!(
        eval("String.fromCharCode(-65536).charCodeAt(0)"),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_string_fromcharcode_infinity() {
    // Infinity should convert to 0
    assert_eq!(
        eval("String.fromCharCode(Infinity).charCodeAt(0)"),
        JsValue::Number(0.0)
    );
    assert_eq!(
        eval("String.fromCharCode(-Infinity).charCodeAt(0)"),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_string_fromcharcode_nan() {
    // NaN should convert to 0
    assert_eq!(
        eval("String.fromCharCode(NaN).charCodeAt(0)"),
        JsValue::Number(0.0)
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
    assert_eq!(eval("'hello'.at(0)"), JsValue::String(JsString::from("h")));
    assert_eq!(eval("'hello'.at(1)"), JsValue::String(JsString::from("e")));
    assert_eq!(eval("'hello'.at(-1)"), JsValue::String(JsString::from("o")));
    assert_eq!(eval("'hello'.at(-2)"), JsValue::String(JsString::from("l")));
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
    assert_eq!(eval("'hello'.match('xyz')"), JsValue::Null);
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
    assert_eq!(eval("'hello world'.search('world')"), JsValue::Number(6.0));
}

#[test]
fn test_string_search_not_found() {
    assert_eq!(eval("'hello'.search('xyz')"), JsValue::Number(-1.0));
}

#[test]
fn test_string_search_regexp() {
    assert_eq!(eval("'hello world'.search(/o/)"), JsValue::Number(4.0));
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
    assert_eq!(eval("'ABC'.codePointAt(0)"), JsValue::Number(65.0));
    assert_eq!(eval("'ABC'.codePointAt(1)"), JsValue::Number(66.0));
}

#[test]
fn test_string_code_point_at_emoji() {
    // Emoji (surrogate pair in JS, but we handle as code point)
    assert_eq!(
        eval("'😀'.codePointAt(0)"),
        JsValue::Number(128512.0) // 0x1F600
    );
}

#[test]
fn test_string_code_point_at_out_of_range() {
    // Out of range returns undefined
    assert_eq!(eval("'abc'.codePointAt(5)"), JsValue::Undefined);
}

#[test]
fn test_string_code_point_at_negative() {
    // Negative index returns undefined
    assert_eq!(eval("'abc'.codePointAt(-1)"), JsValue::Undefined);
}

// String.prototype.normalize tests
// Note: normalize is a no-op in this implementation (returns string unchanged)
#[test]
fn test_string_normalize_default() {
    // Default is NFC - with no-op implementation, input equals output
    assert_eq!(
        *eval("'café'.normalize()"),
        *eval("'café'.normalize('NFC')")
    );
}

#[test]
fn test_string_normalize_returns_string() {
    // normalize returns the input string unchanged (no-op implementation)
    assert_eq!(
        eval("'hello'.normalize('NFC')"),
        JsValue::String(JsString::from("hello"))
    );
}

#[test]
fn test_string_normalize_invalid_form() {
    // Invalid form should throw RangeError
    let result = std::panic::catch_unwind(|| {
        eval("'hello'.normalize('INVALID')");
    });
    assert!(result.is_err());
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
    assert_eq!(eval("'abc'.localeCompare('abc')"), JsValue::Number(0.0));
}

#[test]
fn test_string_locale_compare_less() {
    assert_eq!(eval("'abc'.localeCompare('abd')"), JsValue::Number(-1.0));
}

#[test]
fn test_string_locale_compare_greater() {
    assert_eq!(eval("'abd'.localeCompare('abc')"), JsValue::Number(1.0));
}

#[test]
fn test_string_locale_compare_empty() {
    // Empty string comes before any non-empty string
    assert_eq!(eval("''.localeCompare('a')"), JsValue::Number(-1.0));
}

// substr tests (deprecated but still needs support)
#[test]
fn test_string_substr_basic() {
    // substr(start) - from start to end
    assert_eq!(
        eval("'hello world'.substr(6)"),
        JsValue::String(JsString::from("world"))
    );
}

#[test]
fn test_string_substr_with_length() {
    // substr(start, length) - from start for length characters
    assert_eq!(
        eval("'hello world'.substr(0, 5)"),
        JsValue::String(JsString::from("hello"))
    );
}

#[test]
fn test_string_substr_negative_start() {
    // Negative start counts from end
    assert_eq!(
        eval("'hello world'.substr(-5)"),
        JsValue::String(JsString::from("world"))
    );
}

#[test]
fn test_string_substr_zero_length() {
    // Zero length returns empty string
    assert_eq!(
        eval("'hello'.substr(0, 0)"),
        JsValue::String(JsString::from(""))
    );
}

#[test]
fn test_string_substr_exceeds_length() {
    // Length exceeding string bounds clips to end
    assert_eq!(
        eval("'hello'.substr(0, 100)"),
        JsValue::String(JsString::from("hello"))
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// String constructor function
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_string_constructor_with_number() {
    // String(42) should return "42"
    assert_eq!(eval("String(42)"), JsValue::String(JsString::from("42")));
}

#[test]
fn test_string_constructor_with_boolean() {
    // String(true) should return "true"
    assert_eq!(
        eval("String(true)"),
        JsValue::String(JsString::from("true"))
    );
    assert_eq!(
        eval("String(false)"),
        JsValue::String(JsString::from("false"))
    );
}

#[test]
fn test_string_constructor_with_null() {
    // String(null) should return "null"
    assert_eq!(
        eval("String(null)"),
        JsValue::String(JsString::from("null"))
    );
}

#[test]
fn test_string_constructor_with_undefined() {
    // String(undefined) should return "undefined"
    assert_eq!(
        eval("String(undefined)"),
        JsValue::String(JsString::from("undefined"))
    );
}

#[test]
fn test_string_constructor_with_string() {
    // String("hello") should return "hello"
    assert_eq!(
        eval(r#"String("hello")"#),
        JsValue::String(JsString::from("hello"))
    );
}

#[test]
fn test_string_constructor_no_args() {
    // String() with no args should return ""
    assert_eq!(eval("String()"), JsValue::String(JsString::from("")));
}

// === Split tests ===

#[test]
fn test_string_split_basic() {
    // Split with string separator
    assert_eq!(
        eval(r#"JSON.stringify("a,b,c".split(","))"#),
        JsValue::String(JsString::from(r#"["a","b","c"]"#))
    );
}

#[test]
fn test_string_split_with_limit() {
    // Split with limit
    assert_eq!(
        eval(r#"JSON.stringify("a,b,c,d".split(",", 2))"#),
        JsValue::String(JsString::from(r#"["a","b"]"#))
    );
}

#[test]
fn test_string_split_regexp() {
    // Split with RegExp separator
    assert_eq!(
        eval(r#"JSON.stringify("apple,banana;cherry orange".split(/[,;\s]+/))"#),
        JsValue::String(JsString::from(r#"["apple","banana","cherry","orange"]"#))
    );
}

#[test]
fn test_string_split_regexp_simple() {
    // Split with simple RegExp
    assert_eq!(
        eval(r#"JSON.stringify("a1b2c3".split(/\d/))"#),
        JsValue::String(JsString::from(r#"["a","b","c",""]"#))
    );
}

#[test]
fn test_string_split_undefined_separator() {
    // Per ECMAScript spec: If separator is undefined, return array containing the string
    // This is different from split("undefined") which would split on the literal string
    assert_eq!(
        eval(r#"JSON.stringify("hello".split(undefined))"#),
        JsValue::String(JsString::from(r#"["hello"]"#))
    );

    // Also test with explicit undefined
    assert_eq!(
        eval(r#"JSON.stringify("undefinedd".split(undefined))"#),
        JsValue::String(JsString::from(r#"["undefinedd"]"#))
    );

    // Test no argument (same behavior as undefined)
    assert_eq!(
        eval(r#"JSON.stringify("hello".split())"#),
        JsValue::String(JsString::from(r#"["hello"]"#))
    );
}

// === Replace tests ===

#[test]
fn test_string_replace_basic() {
    // Replace with string
    assert_eq!(
        eval(r#""hello world".replace("world", "there")"#),
        JsValue::String(JsString::from("hello there"))
    );
}

#[test]
fn test_string_replace_first_only() {
    // Replace only first occurrence
    assert_eq!(
        eval(r#""foo foo foo".replace("foo", "bar")"#),
        JsValue::String(JsString::from("bar foo foo"))
    );
}

#[test]
fn test_string_replace_regexp() {
    // Replace with RegExp (first match only without global flag)
    assert_eq!(
        eval(r#""hello 123 world".replace(/\d+/, "XXX")"#),
        JsValue::String(JsString::from("hello XXX world"))
    );
}

#[test]
fn test_string_replace_regexp_global() {
    // Replace all matches with global flag
    assert_eq!(
        eval(r#""a1b2c3".replace(/\d/g, "X")"#),
        JsValue::String(JsString::from("aXbXcX"))
    );
}

#[test]
fn test_string_replace_callback() {
    // Replace with callback function
    assert_eq!(
        eval(r#""foo bar baz".replace(/\b\w/g, (c) => c.toUpperCase())"#),
        JsValue::String(JsString::from("Foo Bar Baz"))
    );
}

#[test]
fn test_string_replace_callback_with_capture_group() {
    // Replace with callback that receives capture group
    // In JS: callback receives (match, p1, p2, ..., offset, string)
    assert_eq!(
        eval(r#""my-variable-name".replace(/-([a-z])/g, (_, letter) => letter.toUpperCase())"#),
        JsValue::String(JsString::from("myVariableName"))
    );
}

#[test]
fn test_string_replace_callback_with_string_search() {
    // Replace with callback function and string search value (not regex)
    assert_eq!(
        eval(r#""hello world".replace("world", () => "universe")"#),
        JsValue::String(JsString::from("hello universe"))
    );
}

#[test]
fn test_string_replace_callback_returning_undefined() {
    // Callback that returns undefined should replace with "undefined"
    assert_eq!(
        eval(
            r#"
            let x;
            "hello".replace("l", () => x)
        "#
        ),
        JsValue::String(JsString::from("heundefinedlo"))
    );
}

#[test]
fn test_string_replace_callback_with_object_search() {
    // Test with object search value that has toString()
    assert_eq!(
        eval(
            r#"
            const obj = {
                toString() { return "AB"; }
            };
            "ABBABABAB".replace(obj, () => "X")
        "#
        ),
        JsValue::String(JsString::from("XBABABAB"))
    );
}

#[test]
fn test_string_replace_callback_undefined_variable() {
    // Test262 case: callback returns undefined variable
    assert_eq!(
        eval(
            r#"
            var x;
            var __obj = {
                toString: function() { return "AB"; }
            };
            "ABBABABAB".replace(__obj, function() { return x; })
        "#
        ),
        JsValue::String(JsString::from("undefinedBABABAB"))
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// String Wrapper Object Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_string_wrapper_typeof() {
    // new String creates an object, not a string primitive
    assert_eq!(
        eval("typeof new String('hello')"),
        JsValue::String("object".into())
    );
    // String() without new returns primitive
    assert_eq!(
        eval("typeof String('hello')"),
        JsValue::String("string".into())
    );
}

#[test]
fn test_string_wrapper_valueof() {
    // valueOf returns the primitive string value
    assert_eq!(
        eval("new String('hello').valueOf()"),
        JsValue::String("hello".into())
    );
}

#[test]
fn test_string_wrapper_tostring() {
    // toString returns the primitive string value
    assert_eq!(
        eval("new String('world').toString()"),
        JsValue::String("world".into())
    );
}

#[test]
fn test_string_wrapper_length() {
    // String wrapper objects have a length property
    assert_eq!(eval("new String('hello').length"), JsValue::Number(5.0));
    assert_eq!(eval("new String('').length"), JsValue::Number(0.0));
}

#[test]
fn test_string_wrapper_addition() {
    // String wrapper in addition uses ToPrimitive
    assert_eq!(
        eval("new String('hello') + ' world'"),
        JsValue::String("hello world".into())
    );
    assert_eq!(
        eval("'hello ' + new String('world')"),
        JsValue::String("hello world".into())
    );
    assert_eq!(
        eval("new String('foo') + new String('bar')"),
        JsValue::String("foobar".into())
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// charAt ToInteger coercion tests (Test262 conformance)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_charat_with_undefined_index() {
    // ToInteger(undefined) === 0, so charAt(undefined) should be charAt(0)
    assert_eq!(
        eval(
            r#"
            let x: any;
            "lego".charAt(x)
        "#
        ),
        JsValue::String("l".into())
    );
}

#[test]
fn test_charat_with_boolean_index() {
    // ToInteger(false) === 0, ToInteger(true) === 1
    assert_eq!(
        eval(r#""hello".charAt(false)"#),
        JsValue::String("h".into())
    );
    assert_eq!(eval(r#""hello".charAt(true)"#), JsValue::String("e".into()));
}

#[test]
fn test_charat_on_number_object() {
    // String.prototype.charAt called on Number object should coerce to string first
    assert_eq!(
        eval(
            r#"
            let obj: any = new Object(42);
            obj.charAt = String.prototype.charAt;
            obj.charAt(0) + obj.charAt(1)
        "#
        ),
        JsValue::String("42".into())
    );
}

// NOTE: ToPrimitive coercion for object index (calling valueOf/toString)
// is not yet implemented - it requires interpreter access in to_number().
// Test262 tests S15.5.4.4_A1_T1 through A1_T10 fail because of this.

#[test]
fn test_charat_out_of_bounds() {
    // Out of bounds returns empty string
    assert_eq!(eval(r#""hello".charAt(-1)"#), JsValue::String("".into()));
    assert_eq!(eval(r#""hello".charAt(10)"#), JsValue::String("".into()));
}

#[test]
fn test_charat_no_args() {
    // charAt() with no args should use index 0
    assert_eq!(eval(r#""hello".charAt()"#), JsValue::String("h".into()));
}

// =============================================================================
// String.replace $ pattern tests
// =============================================================================

#[test]
fn test_string_replace_dollar_ampersand() {
    // $& - inserts the matched substring
    // "seashells" -> "sea" + "sh" + "sch" + "ells" = "seashschells"
    assert_eq!(
        eval(r#""She sells seashells".replace(/sh/g, "$&sch")"#),
        JsValue::String(JsString::from("She sells seashschells"))
    );
    // Full test262 test case
    assert_eq!(
        eval(r#""She sells seashells by the seashore.".replace(/sh/g, "$&sch")"#),
        JsValue::String(JsString::from("She sells seashschells by the seashschore."))
    );
}

#[test]
fn test_string_replace_dollar_ampersand_single() {
    // $& with non-global regex
    assert_eq!(
        eval(r#""She sells seashells".replace(/sh/, "$&sch")"#),
        JsValue::String(JsString::from("She sells seashschells"))
    );
}

#[test]
fn test_string_replace_dollar_backtick() {
    // $` - inserts the portion of the string that precedes the match
    assert_eq!(
        eval(r#""hello world".replace("world", "$`")"#),
        JsValue::String(JsString::from("hello hello "))
    );
}

#[test]
fn test_string_replace_dollar_quote() {
    // $' - inserts the portion of the string that follows the match
    assert_eq!(
        eval(r#""hello world".replace("hello", "$'")"#),
        JsValue::String(JsString::from(" world world"))
    );
}

#[test]
fn test_string_replace_dollar_dollar() {
    // $$ - inserts a literal "$"
    assert_eq!(
        eval(r#""price: 10".replace("10", "$$20")"#),
        JsValue::String(JsString::from("price: $20"))
    );
}

#[test]
fn test_string_replace_dollar_number() {
    // $n - inserts the nth capture group
    assert_eq!(
        eval(r#""John Smith".replace(/(\w+) (\w+)/, "$2, $1")"#),
        JsValue::String(JsString::from("Smith, John"))
    );
}

#[test]
fn test_string_replace_dollar_number_double_digit() {
    // $nn - double digit capture group reference
    // Note: This test uses a regex with many groups
    assert_eq!(
        eval(r#""abcdefghijkl".replace(/(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)/, "$12$11$10")"#),
        JsValue::String(JsString::from("lkj"))
    );
}

#[test]
fn test_string_replace_dollar_mixed() {
    // Mix of $ patterns
    assert_eq!(
        eval(r#""hello world".replace(/(\w+)/, "[$1] = $&")"#),
        JsValue::String(JsString::from("[hello] = hello world"))
    );
}

#[test]
fn test_string_replace_dollar_undefined_group() {
    // $n where group didn't participate - should insert empty string
    // Replace "a" with "$1$2" = "a" + "" = "a", so "abc" -> "abc"
    assert_eq!(
        eval(r#""abc".replace(/(a)(x)?/, "$1$2")"#),
        JsValue::String(JsString::from("abc"))
    );
    // With brackets to verify empty group
    assert_eq!(
        eval(r#""abc".replace(/(a)(x)?/, "[$1][$2]")"#),
        JsValue::String(JsString::from("[a][]bc"))
    );
}

#[test]
fn test_string_replace_dollar_string_search() {
    // $ patterns should work with string search too
    assert_eq!(
        eval(r#""foo bar".replace("bar", "$$100")"#),
        JsValue::String(JsString::from("foo $100"))
    );
}

#[test]
fn test_string_replace_global_dollar_ampersand() {
    // Global replacement with $&
    assert_eq!(
        eval(r#""a1b2c3".replace(/\d/g, "[$&]")"#),
        JsValue::String(JsString::from("a[1]b[2]c[3]"))
    );
}

// =============================================================================
// ToPrimitive / String Coercion Tests
// =============================================================================

#[test]
fn test_string_calls_tostring() {
    // String() should call the object's toString method
    assert_eq!(
        eval(
            r#"
            let obj = { toString: function() { return "custom"; } };
            String(obj)
        "#
        ),
        JsValue::String(JsString::from("custom"))
    );
}

#[test]
fn test_string_array_tostring() {
    // String(array) should call Array.prototype.toString
    assert_eq!(
        eval(
            r#"
            let oldToString = Array.prototype.toString;
            Array.prototype.toString = function() { return "__ARRAY__"; };
            let result = String(new Array());
            Array.prototype.toString = oldToString;
            result
        "#
        ),
        JsValue::String(JsString::from("__ARRAY__"))
    );
}

#[test]
fn test_string_falls_back_to_valueof() {
    // If toString returns non-primitive, fall back to valueOf
    assert_eq!(
        eval(
            r#"
            let obj = {
                toString: function() { return {}; },
                valueOf: function() { return "from_valueof"; }
            };
            String(obj)
        "#
        ),
        JsValue::String(JsString::from("from_valueof"))
    );
}

#[test]
fn test_template_literal_calls_tostring() {
    // Template literals should call toString
    assert_eq!(
        eval(
            r#"
            let obj = { toString: function() { return "interpolated"; } };
            `${obj}`
        "#
        ),
        JsValue::String(JsString::from("interpolated"))
    );
}

#[test]
fn test_string_concat_calls_tostring() {
    // String concatenation should call toString
    assert_eq!(
        eval(
            r#"
            let obj = { toString: function() { return "hello"; } };
            "" + obj
        "#
        ),
        JsValue::String(JsString::from("hello"))
    );
}

#[test]
fn test_charat_valueof_coercion() {
    // charAt should call valueOf/toString on the index argument
    assert_eq!(
        eval(
            r#"
            "lego".charAt({ valueOf: function() { return 1; } })
        "#
        ),
        JsValue::String(JsString::from("e"))
    );
    assert_eq!(
        eval(
            r#"
            "lego".charAt({ toString: function() { return "1"; } })
        "#
        ),
        JsValue::String(JsString::from("e"))
    );
}

#[test]
fn test_null_character_escape_in_string() {
    // \0 should produce the null character (U+0000)
    assert_eq!(eval(r#""\0".charCodeAt(0)"#), JsValue::Number(0.0));
}

#[test]
fn test_null_character_escape_in_template() {
    // \0 in template literal should produce the null character (U+0000)
    assert_eq!(eval(r#"`\0`.charCodeAt(0)"#), JsValue::Number(0.0));
}

#[test]
fn test_unicode_escape_in_template() {
    // \uXXXX in template literal should produce the Unicode character
    assert_eq!(eval(r#"`\u0062`"#), JsValue::String(JsString::from("b")));
    // \u{X...} brace notation
    assert_eq!(eval(r#"`\u{62}`"#), JsValue::String(JsString::from("b")));
}

#[test]
fn test_hex_escape_in_template() {
    // \xNN in template literal should produce the character
    assert_eq!(eval(r#"`\x62`"#), JsValue::String(JsString::from("b")));
}

// =============================================================================
// Template Literal Invalid Escape Tests
// =============================================================================

#[test]
fn test_template_invalid_hex_escape() {
    // \xZZ is not a valid hex escape - should throw SyntaxError
    let result = eval_result(r#"`\xZZ`"#);
    assert!(
        result.is_err(),
        "Invalid hex escape in template should be SyntaxError"
    );
}

#[test]
fn test_template_invalid_unicode_escape() {
    // \uXXXG is not valid - should throw SyntaxError
    let result = eval_result(r#"`\u00GG`"#);
    assert!(
        result.is_err(),
        "Invalid unicode escape in template should be SyntaxError"
    );
}

#[test]
fn test_template_invalid_unicode_brace_escape() {
    // \u{ZZZZ} is not valid - should throw SyntaxError
    let result = eval_result(r#"`\u{ZZZZ}`"#);
    assert!(
        result.is_err(),
        "Invalid unicode brace escape in template should be SyntaxError"
    );
}

// NOTE: According to ES2018+, tagged templates should allow invalid escapes
// (with undefined in cooked and preserved raw values).
// Currently, our implementation rejects invalid escapes in ALL templates.
// This is slightly stricter than the spec but prevents common errors.
// TODO: Support invalid escapes in tagged templates for full ES2018+ compliance.
// TODO: raw values are currently the same as cooked values - should preserve escapes
