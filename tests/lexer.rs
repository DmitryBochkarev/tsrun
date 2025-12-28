//! Tests for the lexer
//!
//! These tests verify that the lexer correctly tokenizes TypeScript/JavaScript source.

use tsrun::lexer::{Lexer, TokenKind};
use tsrun::string_dict::StringDict;
use tsrun::JsString;

/// Helper to create JsString from &str in tests
fn s(value: &str) -> JsString {
    JsString::from(value)
}

fn lex(source: &str) -> Vec<TokenKind> {
    let mut dict = StringDict::new();
    let mut lexer = Lexer::new(source, &mut dict);
    let mut tokens = vec![];
    loop {
        let token = lexer.next_token();
        if token.kind == TokenKind::Eof {
            break;
        }
        tokens.push(token.kind);
    }
    tokens
}

#[test]
fn test_numbers() {
    assert_eq!(lex("42"), vec![TokenKind::Number(42.0)]);
    assert_eq!(lex("3.14"), vec![TokenKind::Number(3.14)]);
    assert_eq!(lex("1e10"), vec![TokenKind::Number(1e10)]);
    assert_eq!(lex("0xff"), vec![TokenKind::Number(255.0)]);
    assert_eq!(lex("0b1010"), vec![TokenKind::Number(10.0)]);
    assert_eq!(lex("0o17"), vec![TokenKind::Number(15.0)]);
}

#[test]
fn test_number_literal_with_trailing_dot() {
    // 1. followed by non-digit should keep dot as part of number
    // 1.. becomes Number(1.0), Dot
    assert_eq!(
        lex("1..toString()"),
        vec![
            TokenKind::Number(1.0),
            TokenKind::Dot,
            TokenKind::Identifier(JsString::from("toString")),
            TokenKind::LParen,
            TokenKind::RParen
        ]
    );
    // 1.1 followed by dot for method call
    assert_eq!(
        lex("1.1.toFixed(5)"),
        vec![
            TokenKind::Number(1.1),
            TokenKind::Dot,
            TokenKind::Identifier(JsString::from("toFixed")),
            TokenKind::LParen,
            TokenKind::Number(5.0),
            TokenKind::RParen
        ]
    );
    // 1. followed by bracket access - the dot is part of the number
    // In JS, 1.["toFixed"] means (1.)["toFixed"] - computed member access on number literal
    assert_eq!(
        lex("1.[\"toFixed\"]"),
        vec![
            TokenKind::Number(1.0),
            TokenKind::LBracket,
            TokenKind::String(JsString::from("toFixed")),
            TokenKind::RBracket
        ]
    );
}

#[test]
fn test_strings() {
    assert_eq!(
        lex(r#""hello""#),
        vec![TokenKind::String(JsString::from("hello"))]
    );
    assert_eq!(
        lex(r#"'world'"#),
        vec![TokenKind::String(JsString::from("world"))]
    );
    assert_eq!(
        lex(r#""line\nbreak""#),
        vec![TokenKind::String(JsString::from("line\nbreak"))]
    );
}

#[test]
fn test_operators() {
    assert_eq!(
        lex("+ - * /"),
        vec![
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash
        ]
    );
    assert_eq!(
        lex("=== !== "),
        vec![TokenKind::EqEqEq, TokenKind::BangEqEq]
    );
    assert_eq!(lex("&&="), vec![TokenKind::AmpAmpEq]);
    assert_eq!(lex("??="), vec![TokenKind::QuestionQuestionEq]);
}

#[test]
fn test_keywords() {
    assert_eq!(
        lex("let const var"),
        vec![TokenKind::Let, TokenKind::Const, TokenKind::Var]
    );
    assert_eq!(
        lex("function return"),
        vec![TokenKind::Function, TokenKind::Return]
    );
}

#[test]
fn test_identifiers() {
    assert_eq!(
        lex("foo bar_baz $test"),
        vec![
            TokenKind::Identifier(JsString::from("foo")),
            TokenKind::Identifier(JsString::from("bar_baz")),
            TokenKind::Identifier(JsString::from("$test")),
        ]
    );
}

#[test]
fn test_comments() {
    assert_eq!(
        lex("1 // comment\n2"),
        vec![TokenKind::Number(1.0), TokenKind::Number(2.0)]
    );
    assert_eq!(
        lex("1 /* comment */ 2"),
        vec![TokenKind::Number(1.0), TokenKind::Number(2.0)]
    );
}

#[test]
fn test_comparison_operators() {
    assert_eq!(
        lex("< > <= >= == !="),
        vec![
            TokenKind::Lt,
            TokenKind::Gt,
            TokenKind::LtEq,
            TokenKind::GtEq,
            TokenKind::EqEq,
            TokenKind::BangEq
        ]
    );
}

#[test]
fn test_logical_operators() {
    assert_eq!(
        lex("&& || !"),
        vec![TokenKind::AmpAmp, TokenKind::PipePipe, TokenKind::Bang]
    );
}

#[test]
fn test_bitwise_operators() {
    assert_eq!(
        lex("& | ^ ~ << >>"),
        vec![
            TokenKind::Amp,
            TokenKind::Pipe,
            TokenKind::Caret,
            TokenKind::Tilde,
            TokenKind::LtLt,
            TokenKind::GtGt
        ]
    );
}

#[test]
fn test_assignment_operators() {
    assert_eq!(
        lex("= += -= *= /= %="),
        vec![
            TokenKind::Eq,
            TokenKind::PlusEq,
            TokenKind::MinusEq,
            TokenKind::StarEq,
            TokenKind::SlashEq,
            TokenKind::PercentEq
        ]
    );
}

#[test]
fn test_increment_decrement() {
    assert_eq!(
        lex("++ --"),
        vec![TokenKind::PlusPlus, TokenKind::MinusMinus]
    );
}

#[test]
fn test_delimiters() {
    assert_eq!(
        lex("( ) [ ] { }"),
        vec![
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBracket,
            TokenKind::RBracket,
            TokenKind::LBrace,
            TokenKind::RBrace
        ]
    );
}

#[test]
fn test_punctuation() {
    assert_eq!(
        lex(", ; : ."),
        vec![
            TokenKind::Comma,
            TokenKind::Semicolon,
            TokenKind::Colon,
            TokenKind::Dot
        ]
    );
}

#[test]
fn test_arrow_and_spread() {
    assert_eq!(lex("=> ..."), vec![TokenKind::Arrow, TokenKind::DotDotDot]);
}

#[test]
fn test_optional_and_nullish() {
    assert_eq!(
        lex("?. ?? ?"),
        vec![
            TokenKind::QuestionDot,
            TokenKind::QuestionQuestion,
            TokenKind::Question
        ]
    );
}

#[test]
fn test_control_keywords() {
    assert_eq!(
        lex("if else switch case default"),
        vec![
            TokenKind::If,
            TokenKind::Else,
            TokenKind::Switch,
            TokenKind::Case,
            TokenKind::Default
        ]
    );
}

#[test]
fn test_loop_keywords() {
    assert_eq!(
        lex("for while do break continue"),
        vec![
            TokenKind::For,
            TokenKind::While,
            TokenKind::Do,
            TokenKind::Break,
            TokenKind::Continue
        ]
    );
}

#[test]
fn test_class_keywords() {
    assert_eq!(
        lex("class extends new this super"),
        vec![
            TokenKind::Class,
            TokenKind::Extends,
            TokenKind::New,
            TokenKind::This,
            TokenKind::Super
        ]
    );
}

#[test]
fn test_typescript_keywords() {
    assert_eq!(
        lex("interface type implements"),
        vec![TokenKind::Interface, TokenKind::Type, TokenKind::Implements]
    );
}

#[test]
fn test_error_handling_keywords() {
    assert_eq!(
        lex("try catch finally throw"),
        vec![
            TokenKind::Try,
            TokenKind::Catch,
            TokenKind::Finally,
            TokenKind::Throw
        ]
    );
}

#[test]
fn test_type_keywords() {
    assert_eq!(
        lex("void null undefined"),
        vec![
            TokenKind::Void,
            TokenKind::Null,
            TokenKind::Identifier(s("undefined"))
        ]
    );
}

#[test]
fn test_boolean_literals() {
    assert_eq!(lex("true false"), vec![TokenKind::True, TokenKind::False]);
}

#[test]
fn test_template_literal_basic() {
    // Simple template without interpolation becomes TemplateNoSub
    assert_eq!(lex("`hello`"), vec![TokenKind::TemplateNoSub(s("hello"))]);
}

#[test]
fn test_string_escape_sequences() {
    assert_eq!(
        lex(r#""hello\tworld""#),
        vec![TokenKind::String(s("hello\tworld"))]
    );
    assert_eq!(
        lex(r#""line\\break""#),
        vec![TokenKind::String(s("line\\break"))]
    );
    assert_eq!(
        lex(r#""quote\"test""#),
        vec![TokenKind::String(s("quote\"test"))]
    );
}

#[test]
fn test_exponentiation_operator() {
    assert_eq!(lex("**"), vec![TokenKind::StarStar]);
    assert_eq!(lex("**="), vec![TokenKind::StarStarEq]);
}

#[test]
fn test_in_instanceof_operators() {
    assert_eq!(
        lex("in instanceof"),
        vec![TokenKind::In, TokenKind::Instanceof]
    );
}

#[test]
fn test_typeof_operator() {
    assert_eq!(lex("typeof"), vec![TokenKind::Typeof]);
}

#[test]
fn test_unsigned_right_shift() {
    assert_eq!(lex(">>>"), vec![TokenKind::GtGtGt]);
    assert_eq!(lex(">>>="), vec![TokenKind::GtGtGtEq]);
}

#[test]
fn test_bigint_literal() {
    assert_eq!(lex("123n"), vec![TokenKind::BigInt("123".to_string())]);
    assert_eq!(lex("0n"), vec![TokenKind::BigInt("0".to_string())]);
    assert_eq!(
        lex("9007199254740991n"),
        vec![TokenKind::BigInt("9007199254740991".to_string())]
    );
}

#[test]
fn test_bigint_hex() {
    assert_eq!(lex("0xFFn"), vec![TokenKind::BigInt("255".to_string())]);
}

#[test]
fn test_regexp_literal_scan() {
    // Test the explicit regex scanning method
    let mut dict = StringDict::new();
    let mut lexer = Lexer::new("/abc/", &mut dict);
    let token = lexer.scan_regexp();
    assert_eq!(
        token.kind,
        TokenKind::RegExp("abc".to_string(), "".to_string())
    );
}

#[test]
fn test_regexp_literal_with_flags() {
    let mut dict = StringDict::new();
    let mut lexer = Lexer::new("/pattern/gi", &mut dict);
    let token = lexer.scan_regexp();
    assert_eq!(
        token.kind,
        TokenKind::RegExp("pattern".to_string(), "gi".to_string())
    );
}

#[test]
fn test_regexp_literal_with_escapes() {
    let mut dict = StringDict::new();
    let mut lexer = Lexer::new(r"/\d+\.\d+/", &mut dict);
    let token = lexer.scan_regexp();
    assert_eq!(
        token.kind,
        TokenKind::RegExp(r"\d+\.\d+".to_string(), "".to_string())
    );
}

#[test]
fn test_regexp_literal_with_class() {
    let mut dict = StringDict::new();
    let mut lexer = Lexer::new("/[a-z]/i", &mut dict);
    let token = lexer.scan_regexp();
    assert_eq!(
        token.kind,
        TokenKind::RegExp("[a-z]".to_string(), "i".to_string())
    );
}

#[test]
fn test_regexp_literal_with_slash_in_class() {
    // Forward slash inside character class doesn't end the regex
    let mut dict = StringDict::new();
    let mut lexer = Lexer::new("/[/]/", &mut dict);
    let token = lexer.scan_regexp();
    assert_eq!(
        token.kind,
        TokenKind::RegExp("[/]".to_string(), "".to_string())
    );
}

#[test]
fn test_unicode_escape_identifier() {
    // \u0078 is 'x'
    assert_eq!(
        lex(r"\u0078"),
        vec![TokenKind::Identifier(JsString::from("x"))]
    );
}

#[test]
fn test_unicode_escape_identifier_mixed() {
    // f\u006fo is "foo"
    assert_eq!(
        lex(r"f\u006fo"),
        vec![TokenKind::Identifier(JsString::from("foo"))]
    );
}

#[test]
fn test_unicode_escape_keyword_becomes_identifier() {
    // \u0063ase decodes to "case" but since it uses escapes, it's an identifier, not a keyword
    assert_eq!(
        lex(r"\u0063ase"),
        vec![TokenKind::Identifier(JsString::from("case"))]
    );
}

#[test]
fn test_unicode_escape_braced_form() {
    // \u{78} is 'x' (braced form)
    assert_eq!(
        lex(r"\u{78}"),
        vec![TokenKind::Identifier(JsString::from("x"))]
    );
}

#[test]
fn test_unicode_escape_braced_longer() {
    // \u{1F600} is emoji, but not valid in identifier
    // So this should fail
    let tokens = lex(r"\u{1F600}");
    assert!(matches!(tokens.first(), Some(TokenKind::Invalid(_))));
}

#[test]
fn test_unicode_escape_id_start_special() {
    // \u2118 (℘ - Weierstrass p) is Other_ID_Start in Unicode 4.0+
    // This should parse as identifier "a℘"
    assert_eq!(
        lex(r"a\u2118"),
        vec![TokenKind::Identifier(JsString::from("a℘"))]
    );
}

#[test]
fn test_unicode_escape_id_continue_middle_dot() {
    // \u00B7 (·) is a valid ID_Continue character (Other_ID_Continue)
    assert_eq!(
        lex(r"a\u00B7"),
        vec![TokenKind::Identifier(JsString::from("a·"))]
    );
}
