//! Integration tests for the skip() combinator.

use trampoline_parser_tests::skip_parser::{ParseResult, Parser};

fn parse_trimmed(input: &str) -> Option<String> {
    let mut parser = Parser::new(input);
    parser.parse().ok().and_then(|r| match r {
        ParseResult::List(items) => {
            // The skip results in None, so we need to find the Text
            items.into_iter().find_map(|item| match item {
                ParseResult::Text(s, _) => Some(s),
                _ => None,
            })
        }
        ParseResult::Text(s, _) => Some(s),
        _ => None,
    })
}

// =============================================================================
// Basic Skip Tests
// =============================================================================

#[test]
fn skip_no_whitespace() {
    // No whitespace to skip
    assert_eq!(parse_trimmed("42"), Some("42".to_string()));
}

#[test]
fn skip_leading_whitespace() {
    // Leading whitespace should be skipped
    assert_eq!(parse_trimmed("  42"), Some("42".to_string()));
}

#[test]
fn skip_trailing_whitespace() {
    // Trailing whitespace should be skipped
    assert_eq!(parse_trimmed("42  "), Some("42".to_string()));
}

#[test]
fn skip_both_sides() {
    // Both leading and trailing whitespace
    assert_eq!(parse_trimmed("  42  "), Some("42".to_string()));
}

// =============================================================================
// Different Whitespace Characters
// =============================================================================

#[test]
fn skip_spaces() {
    assert_eq!(parse_trimmed("   123   "), Some("123".to_string()));
}

#[test]
fn skip_tabs() {
    assert_eq!(parse_trimmed("\t\t123\t\t"), Some("123".to_string()));
}

#[test]
fn skip_newlines() {
    assert_eq!(parse_trimmed("\n\n123\n\n"), Some("123".to_string()));
}

#[test]
fn skip_mixed_whitespace() {
    assert_eq!(parse_trimmed(" \t\n  123  \n\t "), Some("123".to_string()));
}

#[test]
fn skip_carriage_return() {
    assert_eq!(parse_trimmed("\r\r123\r\r"), Some("123".to_string()));
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn skip_single_digit() {
    assert_eq!(parse_trimmed("5"), Some("5".to_string()));
}

#[test]
fn skip_many_digits() {
    assert_eq!(
        parse_trimmed("  9999999999  "),
        Some("9999999999".to_string())
    );
}

#[test]
fn skip_lots_of_whitespace() {
    let ws = " ".repeat(100);
    let input = format!("{}42{}", ws, ws);
    assert_eq!(parse_trimmed(&input), Some("42".to_string()));
}

// =============================================================================
// Failure Cases
// =============================================================================

#[test]
fn skip_whitespace_only_fails() {
    // Just whitespace with no digits should fail
    let mut parser = Parser::new("   ");
    assert!(parser.parse().is_err());
}

#[test]
fn skip_empty_fails() {
    let mut parser = Parser::new("");
    assert!(parser.parse().is_err());
}

#[test]
fn skip_letters_fail() {
    // Letters are not whitespace nor digits
    let mut parser = Parser::new("abc");
    assert!(parser.parse().is_err());
}
