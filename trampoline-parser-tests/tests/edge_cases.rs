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
    assert!(result.is_ok(), "Should parse simple nested parens: {:?}", result);
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
// Whitespace Handling (parsers don't skip whitespace by default)
// =============================================================================

#[test]
fn no_implicit_whitespace_skip() {
    let mut parser = literal_parser::Parser::new(" hello");
    let result = parser.parse();
    assert!(
        result.is_err(),
        "Parser should not skip leading whitespace"
    );
}
