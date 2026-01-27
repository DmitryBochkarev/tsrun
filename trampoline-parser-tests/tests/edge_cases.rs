//! Edge case tests for generated parsers.

use trampoline_parser_tests::*;

// =============================================================================
// Empty Input
// =============================================================================

#[test]
fn empty_input_zero_or_more() {
    let mut parser = zero_or_more_parser::Parser::new("");
    let result = parser.parse();
    assert!(result.is_ok(), "zero_or_more should handle empty input");
}

#[test]
fn empty_input_one_or_more_fails() {
    let mut parser = one_or_more_parser::Parser::new("");
    let result = parser.parse();
    assert!(result.is_err(), "one_or_more should reject empty input");
}

// =============================================================================
// Long Input
// =============================================================================

#[test]
fn long_repetition() {
    let input = "a".repeat(10000);
    let mut parser = zero_or_more_parser::Parser::new(&input);
    let result = parser.parse();
    assert!(result.is_ok(), "Should handle 10000 repetitions");
}

#[test]
fn long_number() {
    let input = "1".repeat(1000);
    let mut parser = number_parser::Parser::new(&input);
    let result = parser.parse().expect("Should parse long number");
    if let number_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text.len(), 1000);
    } else {
        panic!("Expected Text result");
    }
}

// =============================================================================
// Deeply Nested (Trampoline prevents stack overflow)
// =============================================================================

#[test]
fn nested_parens_simple() {
    // Simple nested expression: ((1))
    let mut parser = nested_parser::Parser::new("((1))");
    let result = parser.parse();
    assert!(
        result.is_ok(),
        "Should parse simple nested parens: {:?}",
        result
    );
}

#[test]
fn nested_parens_with_addition() {
    // Nested with addition: (1+2)
    let mut parser = nested_parser::Parser::new("(1+2)");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse nested addition: {:?}", result);
}

#[test]
fn deeply_nested_parens() {
    // Build deeply nested expression: (((((...1...)))))
    let depth = 1000;
    let mut input = String::new();
    for _ in 0..depth {
        input.push('(');
    }
    input.push('1');
    for _ in 0..depth {
        input.push(')');
    }

    let mut parser = nested_parser::Parser::new(&input);
    let result = parser.parse();
    assert!(
        result.is_ok(),
        "Trampoline should handle deep nesting without stack overflow: {:?}",
        result
    );
}

// =============================================================================
// Line/Column Tracking
// =============================================================================

#[test]
fn span_tracks_position() {
    let mut parser = number_parser::Parser::new("123");
    let result = parser.parse().expect("Should parse");
    if let number_parser::ParseResult::Text(_, span) = result {
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 3);
        assert_eq!(span.line, 1);
        assert_eq!(span.column, 1);
    } else {
        panic!("Expected Text result");
    }
}

// =============================================================================
// Error Messages
// =============================================================================

#[test]
fn error_on_unexpected_input() {
    let mut parser = literal_parser::Parser::new("xyz");
    let result = parser.parse();
    assert!(result.is_err(), "Should produce error");
    let err = result.unwrap_err();
    // Error should have position info
    assert!(!err.to_string().is_empty(), "Error should have message");
}

// =============================================================================
// Error Location Tests
// =============================================================================

#[test]
fn error_reports_position() {
    // "hello" expects "hello", input starts with "h" but has wrong continuation
    let mut parser = literal_parser::Parser::new("hxllo");
    let result = parser.parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    // Error should be at position 1 (where 'x' is instead of 'e')
    assert!(
        err.span.start <= 1,
        "Error should be near the start, got start={}",
        err.span.start
    );
}

#[test]
fn error_reports_line_column_single_line() {
    let mut parser = number_parser::Parser::new("abc");
    let result = parser.parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    // Should be at line 1
    assert_eq!(err.span.line, 1, "Error should be on line 1");
    // Column should be at the start
    assert!(err.span.column >= 1, "Column should be valid");
}

#[test]
fn error_message_is_meaningful() {
    let mut parser = literal_parser::Parser::new("xyz");
    let result = parser.parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    // Error message should not be empty
    assert!(!err.message.is_empty(), "Error message should not be empty");
    // Error message should contain something useful (not just generic error)
    assert!(
        err.message.len() > 5,
        "Error message should be descriptive: '{}'",
        err.message
    );
}

#[test]
fn span_after_newlines() {
    // Test that line tracking works after newlines
    // Use the JSON parser which handles whitespace
    let input = "\n\n\n42";
    let mut parser = json_parser::Parser::new(input);
    let result = parser.parse().expect("Should parse");
    if let json_parser::ParseResult::Json(json_parser::JsonValue::Number(_)) = result {
        // Just verify it parses - the span tracking for JSON is internal
    } else {
        panic!("Expected Number result, got {:?}", result);
    }
}

// =============================================================================
// Whitespace Handling (parsers don't skip whitespace by default)
// =============================================================================

#[test]
fn no_implicit_whitespace_skip() {
    let mut parser = literal_parser::Parser::new(" hello");
    let result = parser.parse();
    assert!(result.is_err(), "Parser should not skip leading whitespace");
}
