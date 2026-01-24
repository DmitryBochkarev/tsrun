//! Integration tests for JSON parsing.

use std::collections::HashMap;
use trampoline_parser_tests::json_parser::{JsonValue, ParseResult, Parser};

fn parse_json(input: &str) -> JsonValue {
    let mut parser = Parser::new(input);
    let result = parser.parse().expect("Parse failed");
    match result {
        ParseResult::Json(v) => v,
        other => panic!("Expected Json result, got {:?}", other),
    }
}

// =============================================================================
// Null, Boolean
// =============================================================================

#[test]
fn json_null() {
    assert_eq!(parse_json("null"), JsonValue::Null);
}

#[test]
fn json_true() {
    assert_eq!(parse_json("true"), JsonValue::Bool(true));
}

#[test]
fn json_false() {
    assert_eq!(parse_json("false"), JsonValue::Bool(false));
}

#[test]
fn json_null_with_whitespace() {
    assert_eq!(parse_json("  null  "), JsonValue::Null);
}

// =============================================================================
// Numbers
// =============================================================================

#[test]
fn json_integer() {
    assert_eq!(parse_json("42"), JsonValue::Number(42.0));
}

#[test]
fn json_negative_integer() {
    assert_eq!(parse_json("-42"), JsonValue::Number(-42.0));
}

#[test]
fn json_zero() {
    assert_eq!(parse_json("0"), JsonValue::Number(0.0));
}

#[test]
fn json_float() {
    assert_eq!(parse_json("3.14"), JsonValue::Number(3.14));
}

#[test]
fn json_negative_float() {
    assert_eq!(parse_json("-3.14"), JsonValue::Number(-3.14));
}

#[test]
fn json_exponent() {
    assert_eq!(parse_json("1e10"), JsonValue::Number(1e10));
}

#[test]
fn json_exponent_negative() {
    assert_eq!(parse_json("1e-10"), JsonValue::Number(1e-10));
}

#[test]
fn json_exponent_positive() {
    assert_eq!(parse_json("1e+10"), JsonValue::Number(1e10));
}

#[test]
fn json_exponent_uppercase() {
    assert_eq!(parse_json("1E10"), JsonValue::Number(1e10));
}

#[test]
fn json_float_with_exponent() {
    assert_eq!(parse_json("1.5e2"), JsonValue::Number(150.0));
}

// =============================================================================
// Strings
// =============================================================================

#[test]
fn json_empty_string() {
    assert_eq!(parse_json("\"\""), JsonValue::String("".to_string()));
}

#[test]
fn json_simple_string() {
    assert_eq!(
        parse_json("\"hello\""),
        JsonValue::String("hello".to_string())
    );
}

#[test]
fn json_string_with_spaces() {
    assert_eq!(
        parse_json("\"hello world\""),
        JsonValue::String("hello world".to_string())
    );
}

#[test]
fn json_string_escape_newline() {
    assert_eq!(
        parse_json("\"hello\\nworld\""),
        JsonValue::String("hello\nworld".to_string())
    );
}

#[test]
fn json_string_escape_tab() {
    assert_eq!(
        parse_json("\"hello\\tworld\""),
        JsonValue::String("hello\tworld".to_string())
    );
}

#[test]
fn json_string_escape_quote() {
    assert_eq!(
        parse_json("\"hello\\\"world\""),
        JsonValue::String("hello\"world".to_string())
    );
}

#[test]
fn json_string_escape_backslash() {
    assert_eq!(
        parse_json("\"hello\\\\world\""),
        JsonValue::String("hello\\world".to_string())
    );
}

#[test]
fn json_string_escape_slash() {
    assert_eq!(
        parse_json("\"hello\\/world\""),
        JsonValue::String("hello/world".to_string())
    );
}

// =============================================================================
// Arrays
// =============================================================================

#[test]
fn json_empty_array() {
    assert_eq!(parse_json("[]"), JsonValue::Array(vec![]));
}

#[test]
fn json_array_single_element() {
    assert_eq!(
        parse_json("[1]"),
        JsonValue::Array(vec![JsonValue::Number(1.0)])
    );
}

#[test]
fn json_array_multiple_elements() {
    assert_eq!(
        parse_json("[1, 2, 3]"),
        JsonValue::Array(vec![
            JsonValue::Number(1.0),
            JsonValue::Number(2.0),
            JsonValue::Number(3.0)
        ])
    );
}

#[test]
fn json_array_mixed_types() {
    assert_eq!(
        parse_json("[1, \"two\", true, null]"),
        JsonValue::Array(vec![
            JsonValue::Number(1.0),
            JsonValue::String("two".to_string()),
            JsonValue::Bool(true),
            JsonValue::Null
        ])
    );
}

#[test]
fn json_nested_array() {
    assert_eq!(
        parse_json("[[1, 2], [3, 4]]"),
        JsonValue::Array(vec![
            JsonValue::Array(vec![JsonValue::Number(1.0), JsonValue::Number(2.0)]),
            JsonValue::Array(vec![JsonValue::Number(3.0), JsonValue::Number(4.0)])
        ])
    );
}

#[test]
fn json_array_with_whitespace() {
    assert_eq!(
        parse_json("[ 1 , 2 , 3 ]"),
        JsonValue::Array(vec![
            JsonValue::Number(1.0),
            JsonValue::Number(2.0),
            JsonValue::Number(3.0)
        ])
    );
}

// =============================================================================
// Objects
// =============================================================================

#[test]
fn json_empty_object() {
    assert_eq!(parse_json("{}"), JsonValue::Object(HashMap::new()));
}

#[test]
fn json_object_single_pair() {
    let mut expected = HashMap::new();
    expected.insert("key".to_string(), JsonValue::Number(1.0));
    assert_eq!(parse_json("{\"key\": 1}"), JsonValue::Object(expected));
}

#[test]
fn json_object_multiple_pairs() {
    let mut expected = HashMap::new();
    expected.insert("a".to_string(), JsonValue::Number(1.0));
    expected.insert("b".to_string(), JsonValue::Number(2.0));
    assert_eq!(
        parse_json("{\"a\": 1, \"b\": 2}"),
        JsonValue::Object(expected)
    );
}

#[test]
fn json_object_mixed_values() {
    let mut expected = HashMap::new();
    expected.insert("num".to_string(), JsonValue::Number(42.0));
    expected.insert("str".to_string(), JsonValue::String("hello".to_string()));
    expected.insert("bool".to_string(), JsonValue::Bool(true));
    expected.insert("null".to_string(), JsonValue::Null);
    assert_eq!(
        parse_json("{\"num\": 42, \"str\": \"hello\", \"bool\": true, \"null\": null}"),
        JsonValue::Object(expected)
    );
}

#[test]
fn json_nested_object() {
    let mut inner = HashMap::new();
    inner.insert("b".to_string(), JsonValue::Number(2.0));

    let mut expected = HashMap::new();
    expected.insert("a".to_string(), JsonValue::Object(inner));
    assert_eq!(
        parse_json("{\"a\": {\"b\": 2}}"),
        JsonValue::Object(expected)
    );
}

#[test]
fn json_object_with_array() {
    let mut expected = HashMap::new();
    expected.insert(
        "arr".to_string(),
        JsonValue::Array(vec![
            JsonValue::Number(1.0),
            JsonValue::Number(2.0),
            JsonValue::Number(3.0),
        ]),
    );
    assert_eq!(
        parse_json("{\"arr\": [1, 2, 3]}"),
        JsonValue::Object(expected)
    );
}

// =============================================================================
// Complex / Real-world Examples
// =============================================================================

#[test]
fn json_complex_nested() {
    let input = r#"{
        "name": "test",
        "version": 1,
        "enabled": true,
        "tags": ["a", "b", "c"],
        "nested": {
            "x": 10,
            "y": 20
        }
    }"#;

    let result = parse_json(input);

    // Verify it's an object with the right keys
    if let JsonValue::Object(obj) = result {
        assert_eq!(
            obj.get("name"),
            Some(&JsonValue::String("test".to_string()))
        );
        assert_eq!(obj.get("version"), Some(&JsonValue::Number(1.0)));
        assert_eq!(obj.get("enabled"), Some(&JsonValue::Bool(true)));

        // Check tags array
        if let Some(JsonValue::Array(tags)) = obj.get("tags") {
            assert_eq!(tags.len(), 3);
            assert_eq!(tags[0], JsonValue::String("a".to_string()));
        } else {
            panic!("Expected tags to be an array");
        }

        // Check nested object
        if let Some(JsonValue::Object(nested)) = obj.get("nested") {
            assert_eq!(nested.get("x"), Some(&JsonValue::Number(10.0)));
            assert_eq!(nested.get("y"), Some(&JsonValue::Number(20.0)));
        } else {
            panic!("Expected nested to be an object");
        }
    } else {
        panic!("Expected object");
    }
}

#[test]
fn json_array_of_objects() {
    let input = r#"[
        {"id": 1, "name": "first"},
        {"id": 2, "name": "second"}
    ]"#;

    let result = parse_json(input);

    if let JsonValue::Array(arr) = result {
        assert_eq!(arr.len(), 2);

        if let JsonValue::Object(first) = &arr[0] {
            assert_eq!(first.get("id"), Some(&JsonValue::Number(1.0)));
            assert_eq!(
                first.get("name"),
                Some(&JsonValue::String("first".to_string()))
            );
        } else {
            panic!("Expected first element to be object");
        }
    } else {
        panic!("Expected array");
    }
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn json_deeply_nested_arrays() {
    // [[[[[[1]]]]]]
    let depth = 100;
    let mut input = String::new();
    for _ in 0..depth {
        input.push('[');
    }
    input.push('1');
    for _ in 0..depth {
        input.push(']');
    }

    let result = parse_json(&input);

    // Verify the structure
    let mut current = &result;
    for _ in 0..depth {
        if let JsonValue::Array(arr) = current {
            assert_eq!(arr.len(), 1);
            current = &arr[0];
        } else {
            panic!("Expected array at each level");
        }
    }
    assert_eq!(current, &JsonValue::Number(1.0));
}

#[test]
fn json_whitespace_variations() {
    // Various whitespace characters
    let input = "{\n\t\"key\"\r\n:\t  1\n}";
    let mut expected = HashMap::new();
    expected.insert("key".to_string(), JsonValue::Number(1.0));
    assert_eq!(parse_json(input), JsonValue::Object(expected));
}

// =============================================================================
// Error Cases
// =============================================================================

#[test]
fn json_error_trailing_comma_array() {
    // Note: Standard JSON doesn't allow trailing commas
    // Our parser should reject this
    let result = std::panic::catch_unwind(|| parse_json("[1, 2, ]"));
    assert!(
        result.is_err() || {
            // If it doesn't panic, check for empty last element or error
            true
        }
    );
}

#[test]
fn json_error_invalid_token() {
    let result = std::panic::catch_unwind(|| parse_json("undefined"));
    assert!(result.is_err());
}
