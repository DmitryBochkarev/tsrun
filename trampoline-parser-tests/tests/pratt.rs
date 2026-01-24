//! Integration tests for Pratt expression parsing.

use trampoline_parser_tests::arithmetic_parser::{Expr, Op, ParseResult, Parser};

fn parse_expr(input: &str) -> Expr {
    let mut parser = Parser::new(input);
    let result = parser.parse().expect("Parse failed");
    match result {
        ParseResult::Expr(e) => e,
        ParseResult::Text(s, _) => Expr::Num(s.parse().unwrap_or(0)),
        _ => panic!("Unexpected result type"),
    }
}

fn eval(expr: &Expr) -> i64 {
    match expr {
        Expr::Num(n) => *n,
        Expr::Binary(l, op, r) => {
            let l = eval(l);
            let r = eval(r);
            match op {
                Op::Add => l + r,
                Op::Sub => l - r,
                Op::Mul => l * r,
                Op::Div => {
                    if r != 0 {
                        l / r
                    } else {
                        0
                    }
                }
                _ => 0,
            }
        }
        Expr::Unary(op, e) => {
            let v = eval(e);
            match op {
                Op::Neg => -v,
                _ => v,
            }
        }
    }
}

// =============================================================================
// Single Operand
// =============================================================================

#[test]
fn single_number() {
    let expr = parse_expr("42");
    assert_eq!(expr, Expr::Num(42));
}

#[test]
fn single_number_eval() {
    let expr = parse_expr("42");
    assert_eq!(eval(&expr), 42);
}

// =============================================================================
// Binary Operators
// =============================================================================

#[test]
fn simple_addition() {
    let expr = parse_expr("1+2");
    assert_eq!(eval(&expr), 3);
}

#[test]
fn simple_subtraction() {
    let expr = parse_expr("5-3");
    assert_eq!(eval(&expr), 2);
}

#[test]
fn simple_multiplication() {
    let expr = parse_expr("3*4");
    assert_eq!(eval(&expr), 12);
}

#[test]
fn simple_division() {
    let expr = parse_expr("8/2");
    assert_eq!(eval(&expr), 4);
}

// =============================================================================
// Precedence
// =============================================================================

#[test]
fn mul_before_add() {
    // 1 + 2 * 3 = 1 + (2 * 3) = 7
    let expr = parse_expr("1+2*3");
    assert_eq!(eval(&expr), 7);
}

#[test]
fn div_before_sub() {
    // 10 - 6 / 2 = 10 - (6 / 2) = 7
    let expr = parse_expr("10-6/2");
    assert_eq!(eval(&expr), 7);
}

#[test]
fn complex_precedence() {
    // 2 + 3 * 4 - 5 = 2 + 12 - 5 = 9
    let expr = parse_expr("2+3*4-5");
    assert_eq!(eval(&expr), 9);
}

// =============================================================================
// Left Associativity
// =============================================================================

#[test]
fn left_assoc_subtraction() {
    // 10 - 3 - 2 = (10 - 3) - 2 = 5
    let expr = parse_expr("10-3-2");
    assert_eq!(eval(&expr), 5);
}

#[test]
fn left_assoc_division() {
    // 24 / 4 / 2 = (24 / 4) / 2 = 3
    let expr = parse_expr("24/4/2");
    assert_eq!(eval(&expr), 3);
}

// =============================================================================
// Prefix Operators
// =============================================================================

#[test]
fn prefix_negation() {
    let expr = parse_expr("-5");
    assert_eq!(eval(&expr), -5);
}

#[test]
fn prefix_with_binary() {
    // -5 + 3 = (-5) + 3 = -2
    let expr = parse_expr("-5+3");
    assert_eq!(eval(&expr), -2);
}

#[test]
fn double_negation() {
    // --5 = -(-5) = 5
    let expr = parse_expr("--5");
    assert_eq!(eval(&expr), 5);
}
