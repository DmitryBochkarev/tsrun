//! Integration tests for the optional() combinator.

use trampoline_parser_tests::optional_parser::{ParseResult, Parser};

fn get_sign_and_number(input: &str) -> (Option<String>, String) {
    let mut parser = Parser::new(input);
    let result = parser.parse().expect("Should parse");
    match result {
        ParseResult::List(items) => {
            let mut iter = items.into_iter();
            let sign = match iter.next() {
                Some(ParseResult::Text(s, _)) => Some(s),
                Some(ParseResult::None) => None,
                _ => None,
            };
            let number = match iter.next() {
                Some(ParseResult::Text(s, _)) => s,
                _ => String::new(),
            };
            (sign, number)
        }
        _ => panic!("Expected List result"),
    }
}

// =============================================================================
// Basic Optional Tests
// =============================================================================

#[test]
fn optional_absent() {
    // No sign, just digits
    let (sign, number) = get_sign_and_number("123");
    assert_eq!(sign, None);
    assert_eq!(number, "123");
}

#[test]
fn optional_plus() {
    let (sign, number) = get_sign_and_number("+123");
    assert_eq!(sign, Some("+".to_string()));
    assert_eq!(number, "123");
}

#[test]
fn optional_minus() {
    let (sign, number) = get_sign_and_number("-123");
    assert_eq!(sign, Some("-".to_string()));
    assert_eq!(number, "123");
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn optional_single_digit() {
    let (sign, number) = get_sign_and_number("5");
    assert_eq!(sign, None);
    assert_eq!(number, "5");
}

#[test]
fn optional_signed_single_digit() {
    let (sign, number) = get_sign_and_number("-5");
    assert_eq!(sign, Some("-".to_string()));
    assert_eq!(number, "5");
}

#[test]
fn optional_many_digits() {
    let (sign, number) = get_sign_and_number("+999999999");
    assert_eq!(sign, Some("+".to_string()));
    assert_eq!(number, "999999999");
}

// =============================================================================
// Failure Cases
// =============================================================================

#[test]
fn optional_sign_only_fails() {
    // Just a sign with no digits should fail
    let mut parser = Parser::new("+");
    assert!(parser.parse().is_err());
}

#[test]
fn optional_empty_fails() {
    let mut parser = Parser::new("");
    assert!(parser.parse().is_err());
}

#[test]
fn optional_letters_fail() {
    let mut parser = Parser::new("abc");
    assert!(parser.parse().is_err());
}
