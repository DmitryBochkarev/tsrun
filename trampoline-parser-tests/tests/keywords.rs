//! Integration tests for keyword operators (prefix_kw, infix_kw).

use trampoline_parser_tests::keywords_parser::{Expr, ParseResult, Parser};

fn parse_expr(input: &str) -> Expr {
    let mut parser = Parser::new(input);
    let result = parser
        .parse()
        .expect(&format!("Parse failed for: {}", input));
    result_to_expr(result)
}

fn result_to_expr(result: ParseResult) -> Expr {
    match result {
        ParseResult::Expr(e) => e,
        ParseResult::Text(s, _) => Expr::Ident(s),
        ParseResult::None => Expr::True, // "true" or "false" literal
        ParseResult::List(items) => {
            // Unwrap sequence results - find the first non-None item
            for item in items {
                match item {
                    ParseResult::None => continue,
                    other => return result_to_expr(other),
                }
            }
            Expr::True
        }
    }
}

// =============================================================================
// Basic Keyword Prefix Tests
// =============================================================================

#[test]
fn keyword_prefix_not_true() {
    let expr = parse_expr("not true");
    match expr {
        Expr::Unary(op, _inner) => {
            assert_eq!(op, "not");
            // inner should be True (from the literal "true")
        }
        _ => panic!("Expected Unary"),
    }
}

#[test]
fn keyword_prefix_not_false() {
    let expr = parse_expr("not false");
    match expr {
        Expr::Unary(op, _) => {
            assert_eq!(op, "not");
        }
        _ => panic!("Expected Unary"),
    }
}

#[test]
fn keyword_prefix_not_ident() {
    let expr = parse_expr("not x");
    match expr {
        Expr::Unary(op, inner) => {
            assert_eq!(op, "not");
            assert_eq!(*inner, Expr::Ident("x".to_string()));
        }
        _ => panic!("Expected Unary"),
    }
}

// =============================================================================
// Basic Keyword Infix Tests
// =============================================================================

#[test]
fn keyword_infix_and() {
    let expr = parse_expr("a and b");
    match expr {
        Expr::Binary(op, l, r) => {
            assert_eq!(op, "and");
            assert_eq!(*l, Expr::Ident("a".to_string()));
            assert_eq!(*r, Expr::Ident("b".to_string()));
        }
        _ => panic!("Expected Binary"),
    }
}

#[test]
fn keyword_infix_or() {
    let expr = parse_expr("a or b");
    match expr {
        Expr::Binary(op, l, r) => {
            assert_eq!(op, "or");
            assert_eq!(*l, Expr::Ident("a".to_string()));
            assert_eq!(*r, Expr::Ident("b".to_string()));
        }
        _ => panic!("Expected Binary"),
    }
}

// =============================================================================
// Keyword Boundary Tests - These are the critical tests for keyword operators
// =============================================================================

#[test]
fn keyword_not_attached_to_ident() {
    // "nottrue" should parse as a single identifier, not "not true"
    let expr = parse_expr("nottrue");
    assert_eq!(expr, Expr::Ident("nottrue".to_string()));
}

#[test]
fn keyword_and_in_identifier() {
    // "android" should parse as a single identifier, not "and roid"
    let expr = parse_expr("android");
    assert_eq!(expr, Expr::Ident("android".to_string()));
}

#[test]
fn keyword_or_in_identifier() {
    // "oregon" should parse as a single identifier, not "or egon"
    let expr = parse_expr("oregon");
    assert_eq!(expr, Expr::Ident("oregon".to_string()));
}

#[test]
fn keyword_notably() {
    // "notably" contains "not" but should be an identifier
    let expr = parse_expr("notably");
    assert_eq!(expr, Expr::Ident("notably".to_string()));
}

#[test]
fn keyword_band() {
    // "band" contains "and" at the end, should be an identifier
    let expr = parse_expr("band");
    assert_eq!(expr, Expr::Ident("band".to_string()));
}

// =============================================================================
// Precedence Tests
// =============================================================================

#[test]
fn keyword_precedence_and_over_or() {
    // "a or b and c" should parse as "a or (b and c)" since and has higher precedence
    let expr = parse_expr("a or b and c");
    match expr {
        Expr::Binary(op, l, r) => {
            assert_eq!(op, "or");
            assert_eq!(*l, Expr::Ident("a".to_string()));
            match *r {
                Expr::Binary(inner_op, inner_l, inner_r) => {
                    assert_eq!(inner_op, "and");
                    assert_eq!(*inner_l, Expr::Ident("b".to_string()));
                    assert_eq!(*inner_r, Expr::Ident("c".to_string()));
                }
                _ => panic!("Expected inner Binary"),
            }
        }
        _ => panic!("Expected Binary"),
    }
}

#[test]
fn keyword_precedence_not_highest() {
    // "not a and b" should parse as "(not a) and b" since not has highest precedence
    let expr = parse_expr("not a and b");
    match expr {
        Expr::Binary(op, l, r) => {
            assert_eq!(op, "and");
            match *l {
                Expr::Unary(inner_op, inner_e) => {
                    assert_eq!(inner_op, "not");
                    assert_eq!(*inner_e, Expr::Ident("a".to_string()));
                }
                _ => panic!("Expected Unary on left"),
            }
            assert_eq!(*r, Expr::Ident("b".to_string()));
        }
        _ => panic!("Expected Binary"),
    }
}

// =============================================================================
// Combination Tests
// =============================================================================

#[test]
fn keyword_double_not() {
    let expr = parse_expr("not not a");
    match expr {
        Expr::Unary(op1, inner) => {
            assert_eq!(op1, "not");
            match *inner {
                Expr::Unary(op2, e) => {
                    assert_eq!(op2, "not");
                    assert_eq!(*e, Expr::Ident("a".to_string()));
                }
                _ => panic!("Expected inner Unary"),
            }
        }
        _ => panic!("Expected Unary"),
    }
}

#[test]
fn keyword_complex_expression() {
    // "not a or b and not c"
    // Should parse as: (not a) or (b and (not c))
    let expr = parse_expr("not a or b and not c");
    match expr {
        Expr::Binary(op, l, r) => {
            assert_eq!(op, "or");
            // Left should be (not a)
            match *l {
                Expr::Unary(inner_op, e) => {
                    assert_eq!(inner_op, "not");
                    assert_eq!(*e, Expr::Ident("a".to_string()));
                }
                _ => panic!("Expected Unary on left"),
            }
            // Right should be (b and (not c))
            match *r {
                Expr::Binary(and_op, and_l, and_r) => {
                    assert_eq!(and_op, "and");
                    assert_eq!(*and_l, Expr::Ident("b".to_string()));
                    match *and_r {
                        Expr::Unary(not_op, e) => {
                            assert_eq!(not_op, "not");
                            assert_eq!(*e, Expr::Ident("c".to_string()));
                        }
                        _ => panic!("Expected Unary for (not c)"),
                    }
                }
                _ => panic!("Expected Binary on right"),
            }
        }
        _ => panic!("Expected Binary at top level"),
    }
}

// =============================================================================
// Left Associativity Tests
// =============================================================================

#[test]
fn keyword_left_assoc_and() {
    // "a and b and c" should parse as "(a and b) and c"
    let expr = parse_expr("a and b and c");
    match expr {
        Expr::Binary(op, l, r) => {
            assert_eq!(op, "and");
            assert_eq!(*r, Expr::Ident("c".to_string()));
            match *l {
                Expr::Binary(inner_op, inner_l, inner_r) => {
                    assert_eq!(inner_op, "and");
                    assert_eq!(*inner_l, Expr::Ident("a".to_string()));
                    assert_eq!(*inner_r, Expr::Ident("b".to_string()));
                }
                _ => panic!("Expected inner Binary"),
            }
        }
        _ => panic!("Expected Binary"),
    }
}

#[test]
fn keyword_left_assoc_or() {
    // "a or b or c" should parse as "(a or b) or c"
    let expr = parse_expr("a or b or c");
    match expr {
        Expr::Binary(op, l, r) => {
            assert_eq!(op, "or");
            assert_eq!(*r, Expr::Ident("c".to_string()));
            match *l {
                Expr::Binary(inner_op, inner_l, inner_r) => {
                    assert_eq!(inner_op, "or");
                    assert_eq!(*inner_l, Expr::Ident("a".to_string()));
                    assert_eq!(*inner_r, Expr::Ident("b".to_string()));
                }
                _ => panic!("Expected inner Binary"),
            }
        }
        _ => panic!("Expected Binary"),
    }
}
