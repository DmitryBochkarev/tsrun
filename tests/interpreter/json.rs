//! Tests for JSON object (parse and stringify)

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_json_stringify_simple_object() {
    assert_eq!(
        eval(r#"JSON.stringify({ a: 1 })"#),
        JsValue::String(r#"{"a":1}"#.into())
    );
}

#[test]
fn test_json_stringify_with_indent() {
    // With indentation
    let result = eval(r#"JSON.stringify({ a: 1 }, null, 2)"#);
    if let JsValue::String(s) = result {
        assert!(
            s.as_str().contains("{\n"),
            "Expected formatted JSON with newlines"
        );
    } else {
        panic!("Expected String, got {:?}", result);
    }
}

#[test]
fn test_json_stringify_array() {
    assert_eq!(
        eval(r#"JSON.stringify([1, 2, 3])"#),
        JsValue::String("[1,2,3]".into())
    );
}

#[test]
fn test_json_stringify_nested() {
    assert_eq!(
        eval(r#"JSON.stringify({ a: { b: 1 } })"#),
        JsValue::String(r#"{"a":{"b":1}}"#.into())
    );
}

#[test]
fn test_json_parse_simple() {
    assert_eq!(eval(r#"JSON.parse('{"a":1}').a"#), JsValue::Number(1.0));
}

#[test]
fn test_json_parse_array() {
    assert_eq!(eval(r#"JSON.parse('[1,2,3]')[1]"#), JsValue::Number(2.0));
}

#[test]
fn test_json_round_trip() {
    assert_eq!(
        eval(r#"JSON.stringify(JSON.parse('{"a":1,"b":"hello"}'))"#),
        JsValue::String(r#"{"a":1,"b":"hello"}"#.into())
    );
}

#[test]
#[ignore] // TODO: GC issue with async results
fn test_json_stringify_with_async_result() {
    // JSON.stringify on async function result
    let result = eval(
        r#"
        async function getData(): Promise<{ count: number }> {
            return { count: 42 };
        }

        const data = await getData();
        JSON.stringify(data)
    "#,
    );
    assert_eq!(result, JsValue::String(r#"{"count":42}"#.into()));
}

#[test]
#[ignore] // TODO: GC issue with async results
fn test_json_stringify_async_nested_object() {
    // More complex async result with nested object
    let result = eval(
        r#"
        async function fetchData(): Promise<{ user: { name: string }; items: number[] }> {
            return {
                user: { name: "Alice" },
                items: [1, 2, 3]
            };
        }

        const result = await fetchData();
        JSON.stringify(result)
    "#,
    );
    // Object property order may vary, so check content rather than exact string
    if let JsValue::String(s) = result {
        assert!(
            s.as_str().contains(r#""name":"Alice""#),
            "Should contain name"
        );
        assert!(
            s.as_str().contains(r#""items":[1,2,3]"#),
            "Should contain items"
        );
    } else {
        panic!("Expected String, got {:?}", result);
    }
}
