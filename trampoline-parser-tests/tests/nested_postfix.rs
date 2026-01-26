//! Tests for postfix operators in deeply nested contexts.

use trampoline_parser_tests::nested_postfix_parser;

// =============================================================================
// Basic Tests - These should all work
// =============================================================================

#[test]
fn test_simple_identifier() {
    let mut parser = nested_postfix_parser::Parser::new("foo");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse simple identifier");
}

#[test]
fn test_simple_member_access() {
    let mut parser = nested_postfix_parser::Parser::new("foo.bar");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse member access");
}

#[test]
fn test_simple_binary() {
    let mut parser = nested_postfix_parser::Parser::new("foo + bar");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse binary expression");
}

#[test]
fn test_empty_object() {
    let mut parser = nested_postfix_parser::Parser::new("{}");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse empty object");
}

#[test]
fn test_object_with_simple_value() {
    // Try without spaces
    let mut parser = nested_postfix_parser::Parser::new("{a:1}");
    let result = parser.parse();
    println!("Result for {{a:1}}: {:?}", result);

    // Try with spaces
    let mut parser2 = nested_postfix_parser::Parser::new("{ a: 1 }");
    let result2 = parser2.parse();
    println!("Result for {{ a: 1 }}: {:?}", result2);

    assert!(
        result.is_ok() || result2.is_ok(),
        "Should parse object with simple value"
    );
}

#[test]
fn test_object_with_identifier_value() {
    let mut parser = nested_postfix_parser::Parser::new("{ a: x }");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse object with identifier value");
}

#[test]
fn test_object_with_binary_value() {
    let mut parser = nested_postfix_parser::Parser::new("{ a: x + y }");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse object with binary value");
}

// =============================================================================
// These are the failing cases in TypeScript
// =============================================================================

#[test]
fn test_object_with_member_value() {
    // This is the critical test - member access inside object literal
    let mut parser = nested_postfix_parser::Parser::new("{ a: x.y }");
    let result = parser.parse();
    assert!(
        result.is_ok(),
        "Should parse object with member value: {:?}",
        result.err()
    );
}

#[test]
fn test_parens_around_object() {
    let mut parser = nested_postfix_parser::Parser::new("({ a: 1 })");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse parenthesized object");
}

#[test]
fn test_parens_object_with_member() {
    // This reproduces the TypeScript bug
    let mut parser = nested_postfix_parser::Parser::new("({ a: x.y })");
    let result = parser.parse();
    assert!(
        result.is_ok(),
        "Should parse ({{ a: x.y }}): {:?}",
        result.err()
    );
}

#[test]
fn test_assignment_to_object() {
    let mut parser = nested_postfix_parser::Parser::new("o = { a: x }");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse assignment to object");
}

#[test]
fn test_assignment_to_object_with_member() {
    // Another failing case in TypeScript
    let mut parser = nested_postfix_parser::Parser::new("o = { a: x.y }");
    let result = parser.parse();
    assert!(
        result.is_ok(),
        "Should parse o = {{ a: x.y }}: {:?}",
        result.err()
    );
}

#[test]
fn test_object_with_call_value() {
    let mut parser = nested_postfix_parser::Parser::new("{ a: f() }");
    let result = parser.parse();
    assert!(
        result.is_ok(),
        "Should parse object with call value: {:?}",
        result.err()
    );
}

// =============================================================================
// Complex nested cases
// =============================================================================

#[test]
fn test_deeply_nested() {
    let mut parser = nested_postfix_parser::Parser::new("({ a: { b: x.y } })");
    let result = parser.parse();
    assert!(
        result.is_ok(),
        "Should parse deeply nested: {:?}",
        result.err()
    );
}
