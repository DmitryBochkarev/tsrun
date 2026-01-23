//! Integration tests for character class combinators.

use trampoline_parser_tests::*;

// =============================================================================
// hex_digit() Tests
// =============================================================================

#[test]
fn hex_digit_lowercase() {
    let mut parser = hex_parser::Parser::new("deadbeef");
    let result = parser.parse().expect("Should parse");
    if let hex_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "deadbeef");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn hex_digit_uppercase() {
    let mut parser = hex_parser::Parser::new("DEADBEEF");
    let result = parser.parse().expect("Should parse");
    if let hex_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "DEADBEEF");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn hex_digit_mixed() {
    let mut parser = hex_parser::Parser::new("DeAdBeEf");
    let result = parser.parse().expect("Should parse");
    if let hex_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "DeAdBeEf");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn hex_digit_with_numbers() {
    let mut parser = hex_parser::Parser::new("0123456789abcdef");
    let result = parser.parse().expect("Should parse");
    if let hex_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "0123456789abcdef");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn hex_digit_stops_at_invalid() {
    // 'g' is not a hex digit, parsing should stop there
    let mut parser = hex_parser::Parser::new("abcdefg");
    let result = parser.parse().expect("Should parse");
    if let hex_parser::ParseResult::Text(text, _) = result {
        // Parser should capture up to but not including 'g'
        assert_eq!(text, "abcdef");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn hex_digit_rejects_non_hex() {
    // Starting with non-hex should fail
    let mut parser = hex_parser::Parser::new("ghij");
    assert!(parser.parse().is_err());
}

#[test]
fn hex_digit_all_valid_chars() {
    // Test all valid hex digits
    let mut parser = hex_parser::Parser::new("0123456789abcdefABCDEF");
    let result = parser.parse().expect("Should parse");
    if let hex_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "0123456789abcdefABCDEF");
    } else {
        panic!("Expected Text result");
    }
}

// =============================================================================
// alpha_num() Tests
// =============================================================================

#[test]
fn alphanum_letters_only() {
    let mut parser = alphanum_parser::Parser::new("hello");
    let result = parser.parse().expect("Should parse");
    if let alphanum_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "hello");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn alphanum_digits_only() {
    let mut parser = alphanum_parser::Parser::new("12345");
    let result = parser.parse().expect("Should parse");
    if let alphanum_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "12345");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn alphanum_mixed() {
    let mut parser = alphanum_parser::Parser::new("abc123xyz");
    let result = parser.parse().expect("Should parse");
    if let alphanum_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "abc123xyz");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn alphanum_stops_at_underscore() {
    // Underscore is not alphanumeric
    let mut parser = alphanum_parser::Parser::new("abc_123");
    let result = parser.parse().expect("Should parse");
    if let alphanum_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "abc");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn alphanum_stops_at_space() {
    let mut parser = alphanum_parser::Parser::new("abc 123");
    let result = parser.parse().expect("Should parse");
    if let alphanum_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "abc");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn alphanum_rejects_starting_special() {
    let mut parser = alphanum_parser::Parser::new("_abc");
    assert!(parser.parse().is_err());
}

// =============================================================================
// ident_start() and ident_cont() Tests
// =============================================================================

#[test]
fn ident_starts_with_letter() {
    let mut parser = ident_parser::Parser::new("foo123");
    let result = parser.parse().expect("Should parse");
    if let ident_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "foo123");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn ident_starts_with_underscore() {
    let mut parser = ident_parser::Parser::new("_private");
    let result = parser.parse().expect("Should parse");
    if let ident_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "_private");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn ident_starts_with_dollar() {
    let mut parser = ident_parser::Parser::new("$jquery");
    let result = parser.parse().expect("Should parse");
    if let ident_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "$jquery");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn ident_all_underscore() {
    let mut parser = ident_parser::Parser::new("___");
    let result = parser.parse().expect("Should parse");
    if let ident_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "___");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn ident_mixed_special() {
    let mut parser = ident_parser::Parser::new("$_abc123_$");
    let result = parser.parse().expect("Should parse");
    if let ident_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "$_abc123_$");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn ident_rejects_leading_digit() {
    // Identifier cannot start with a digit
    let mut parser = ident_parser::Parser::new("123abc");
    assert!(parser.parse().is_err());
}

#[test]
fn ident_stops_at_dash() {
    let mut parser = ident_parser::Parser::new("foo-bar");
    let result = parser.parse().expect("Should parse");
    if let ident_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "foo");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn ident_single_char() {
    let mut parser = ident_parser::Parser::new("x");
    let result = parser.parse().expect("Should parse");
    if let ident_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "x");
    } else {
        panic!("Expected Text result");
    }
}

// =============================================================================
// range() Tests - Lowercase
// =============================================================================

#[test]
fn range_lowercase_matches() {
    let mut parser = lowercase_parser::Parser::new("hello");
    let result = parser.parse().expect("Should parse");
    if let lowercase_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "hello");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn range_lowercase_rejects_uppercase() {
    let mut parser = lowercase_parser::Parser::new("HELLO");
    assert!(parser.parse().is_err());
}

#[test]
fn range_lowercase_stops_at_uppercase() {
    let mut parser = lowercase_parser::Parser::new("helloWORLD");
    let result = parser.parse().expect("Should parse");
    if let lowercase_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "hello");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn range_boundary_a() {
    // Test boundary: 'a' is in range a-z
    let mut parser = lowercase_parser::Parser::new("a");
    let result = parser.parse().expect("Should parse");
    if let lowercase_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "a");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn range_boundary_z() {
    // Test boundary: 'z' is in range a-z
    let mut parser = lowercase_parser::Parser::new("z");
    let result = parser.parse().expect("Should parse");
    if let lowercase_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "z");
    } else {
        panic!("Expected Text result");
    }
}

// =============================================================================
// range() Tests - Uppercase
// =============================================================================

#[test]
fn range_uppercase_matches() {
    let mut parser = uppercase_parser::Parser::new("HELLO");
    let result = parser.parse().expect("Should parse");
    if let uppercase_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "HELLO");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn range_uppercase_rejects_lowercase() {
    let mut parser = uppercase_parser::Parser::new("hello");
    assert!(parser.parse().is_err());
}

#[test]
fn range_uppercase_boundary_a() {
    let mut parser = uppercase_parser::Parser::new("A");
    let result = parser.parse().expect("Should parse");
    if let uppercase_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "A");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn range_uppercase_boundary_z() {
    let mut parser = uppercase_parser::Parser::new("Z");
    let result = parser.parse().expect("Should parse");
    if let uppercase_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "Z");
    } else {
        panic!("Expected Text result");
    }
}

// =============================================================================
// range() Tests - Custom Range (0-5)
// =============================================================================

#[test]
fn range_custom_0_to_5() {
    let mut parser = custom_range_parser::Parser::new("012345");
    let result = parser.parse().expect("Should parse");
    if let custom_range_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "012345");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn range_custom_stops_at_6() {
    let mut parser = custom_range_parser::Parser::new("01234567");
    let result = parser.parse().expect("Should parse");
    if let custom_range_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "012345");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn range_custom_boundary_0() {
    let mut parser = custom_range_parser::Parser::new("0");
    let result = parser.parse().expect("Should parse");
    if let custom_range_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "0");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn range_custom_boundary_5() {
    let mut parser = custom_range_parser::Parser::new("5");
    let result = parser.parse().expect("Should parse");
    if let custom_range_parser::ParseResult::Text(text, _) = result {
        assert_eq!(text, "5");
    } else {
        panic!("Expected Text result");
    }
}

#[test]
fn range_custom_rejects_6() {
    let mut parser = custom_range_parser::Parser::new("6");
    assert!(parser.parse().is_err());
}

#[test]
fn range_custom_rejects_9() {
    let mut parser = custom_range_parser::Parser::new("9");
    assert!(parser.parse().is_err());
}
