//! Integration tests for parser combinators.

use trampoline_parser_tests::*;

// =============================================================================
// Literal Parser Tests
// =============================================================================

#[test]
fn literal_exact_match() {
    let mut parser = literal_parser::Parser::new("hello");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse exact match");
}

#[test]
fn literal_partial_fails() {
    let mut parser = literal_parser::Parser::new("hell");
    let result = parser.parse();
    assert!(result.is_err(), "Should fail on partial match");
}

#[test]
fn literal_extra_input_succeeds() {
    let mut parser = literal_parser::Parser::new("helloworld");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse with trailing input");
}

#[test]
fn literal_case_sensitive() {
    let mut parser = literal_parser::Parser::new("HELLO");
    let result = parser.parse();
    assert!(result.is_err(), "Should be case sensitive");
}

// =============================================================================
// Digit Parser Tests
// =============================================================================

#[test]
fn digit_matches_all_digits() {
    for c in '0'..='9' {
        let input = c.to_string();
        let mut parser = digit_parser::Parser::new(&input);
        let result = parser.parse();
        assert!(result.is_ok(), "Should match digit '{}'", c);
    }
}

#[test]
fn digit_rejects_letters() {
    let mut parser = digit_parser::Parser::new("a");
    let result = parser.parse();
    assert!(result.is_err(), "Should reject letter");
}

#[test]
fn digit_rejects_empty() {
    let mut parser = digit_parser::Parser::new("");
    let result = parser.parse();
    assert!(result.is_err(), "Should reject empty input");
}

// =============================================================================
// Number Parser Tests (Capture)
// =============================================================================

#[test]
fn number_single_digit() {
    let mut parser = number_parser::Parser::new("5");
    let result = parser.parse().expect("Should parse");
    if let number_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "5");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn number_multiple_digits() {
    let mut parser = number_parser::Parser::new("12345");
    let result = parser.parse().expect("Should parse");
    if let number_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "12345");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn number_span_correct() {
    let mut parser = number_parser::Parser::new("123");
    let result = parser.parse().expect("Should parse");
    if let number_parser::ParseResult::Text(_, span) = result {
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 3);
    } else {
        panic!("Expected Text result");
    }
}

// =============================================================================
// Sequence Parser Tests
// =============================================================================

#[test]
fn sequence_all_parts() {
    let mut parser = sequence_parser::Parser::new("abc");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse full sequence");
}

#[test]
fn sequence_partial_fails() {
    let mut parser = sequence_parser::Parser::new("ab");
    let result = parser.parse();
    assert!(result.is_err(), "Should fail on partial sequence");
}

#[test]
fn sequence_wrong_order_fails() {
    let mut parser = sequence_parser::Parser::new("acb");
    let result = parser.parse();
    assert!(result.is_err(), "Should fail on wrong order");
}

// =============================================================================
// Choice Parser Tests
// =============================================================================

#[test]
fn choice_first_alternative() {
    let mut parser = choice_parser::Parser::new("ab");
    let result = parser.parse();
    assert!(result.is_ok(), "Should match first alternative 'ab'");
}

#[test]
fn choice_second_alternative() {
    let mut parser = choice_parser::Parser::new("a");
    let result = parser.parse();
    assert!(result.is_ok(), "Should match second alternative 'a'");
}

#[test]
fn choice_no_match() {
    let mut parser = choice_parser::Parser::new("b");
    let result = parser.parse();
    assert!(result.is_err(), "Should fail when no alternative matches");
}

// =============================================================================
// Zero-or-More Parser Tests
// =============================================================================

#[test]
fn zero_or_more_empty() {
    let mut parser = zero_or_more_parser::Parser::new("");
    let result = parser.parse();
    assert!(result.is_ok(), "zero_or_more should accept empty");
}

#[test]
fn zero_or_more_one() {
    let mut parser = zero_or_more_parser::Parser::new("a");
    let result = parser.parse();
    assert!(result.is_ok(), "zero_or_more should accept one");
}

#[test]
fn zero_or_more_many() {
    let mut parser = zero_or_more_parser::Parser::new("aaaaa");
    let result = parser.parse();
    assert!(result.is_ok(), "zero_or_more should accept many");
}

// =============================================================================
// One-or-More Parser Tests
// =============================================================================

#[test]
fn one_or_more_empty_fails() {
    let mut parser = one_or_more_parser::Parser::new("");
    let result = parser.parse();
    assert!(result.is_err(), "one_or_more should reject empty");
}

#[test]
fn one_or_more_one() {
    let mut parser = one_or_more_parser::Parser::new("a");
    let result = parser.parse();
    assert!(result.is_ok(), "one_or_more should accept one");
}

#[test]
fn one_or_more_many() {
    let mut parser = one_or_more_parser::Parser::new("aaaaa");
    let result = parser.parse();
    assert!(result.is_ok(), "one_or_more should accept many");
}

// =============================================================================
// Lookahead Parser Tests
// =============================================================================

#[test]
fn not_followed_by_success() {
    // "a" not followed by "b" should succeed for "ac"
    let mut parser = not_followed_parser::Parser::new("ac");
    let result = parser.parse();
    assert!(
        result.is_ok(),
        "Should succeed when 'a' not followed by 'b'"
    );
}

#[test]
fn not_followed_by_fails() {
    // "a" not followed by "b" should fail for "ab"
    let mut parser = not_followed_parser::Parser::new("ab");
    let result = parser.parse();
    assert!(result.is_err(), "Should fail when 'a' IS followed by 'b'");
}

#[test]
fn followed_by_success() {
    // "a" followed by "b" should succeed for "ab"
    let mut parser = followed_by_parser::Parser::new("ab");
    let result = parser.parse();
    assert!(result.is_ok(), "Should succeed when 'a' followed by 'b'");
}

#[test]
fn followed_by_fails_without_target() {
    // "a" followed by "b" should fail for just "a"
    let mut parser = followed_by_parser::Parser::new("a");
    let result = parser.parse();
    assert!(
        result.is_err(),
        "Should fail when 'b' not present after 'a'"
    );
}

// =============================================================================
// Separated Parser Tests
// =============================================================================

#[test]
fn separated_single_item() {
    let mut parser = list_parser::Parser::new("foo");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse single item");
}

#[test]
fn separated_multiple_items() {
    let mut parser = list_parser::Parser::new("foo,bar,baz");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse multiple items");
}

#[test]
fn separated_trailing_not_allowed() {
    let mut parser = list_parser::Parser::new("foo,bar,");
    let result = parser.parse();
    // The list rule doesn't allow trailing, so it should parse "foo,bar" and leave ","
    assert!(result.is_ok(), "Should parse up to trailing comma");
}

#[test]
fn separated_trailing_allowed() {
    let mut parser = list_trailing_parser::Parser::new("foo,bar,");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse with trailing comma");
}
