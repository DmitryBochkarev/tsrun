//! Integration tests for right-associative operators in Pratt parsing.

use trampoline_parser_tests::right_assoc_parser::{Expr, Op, ParseResult, Parser};

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
                Op::Pow => l.pow(r as u32),
            }
        }
    }
}

// =============================================================================
// Right Associativity Tests
// =============================================================================

#[test]
fn right_assoc_simple() {
    // 2^3 = 8
    let expr = parse_expr("2^3");
    assert_eq!(eval(&expr), 8);
}

#[test]
fn right_assoc_chain() {
    // 2^3^2 should be 2^(3^2) = 2^9 = 512, NOT (2^3)^2 = 8^2 = 64
    let expr = parse_expr("2^3^2");
    assert_eq!(eval(&expr), 512);
}

#[test]
fn right_assoc_chain_structure() {
    // Verify the actual tree structure: 2^3^2 = Binary(2, ^, Binary(3, ^, 2))
    let expr = parse_expr("2^3^2");
    match expr {
        Expr::Binary(l, Op::Pow, r) => {
            assert_eq!(*l, Expr::Num(2));
            match *r {
                Expr::Binary(rl, Op::Pow, rr) => {
                    assert_eq!(*rl, Expr::Num(3));
                    assert_eq!(*rr, Expr::Num(2));
                }
                _ => panic!("Expected right operand to be Binary"),
            }
        }
        _ => panic!("Expected top-level Binary"),
    }
}

#[test]
fn right_assoc_triple() {
    // 2^2^2^2 = 2^(2^(2^2)) = 2^(2^4) = 2^16 = 65536
    let expr = parse_expr("2^2^2^2");
    assert_eq!(eval(&expr), 65536);
}

// =============================================================================
// Mixed Associativity Tests
// =============================================================================

#[test]
fn right_assoc_with_left_add() {
    // 1+2^3^2 = 1 + (2^(3^2)) = 1 + 512 = 513
    let expr = parse_expr("1+2^3^2");
    assert_eq!(eval(&expr), 513);
}

#[test]
fn right_assoc_with_left_mul() {
    // 2*2^3 = 2 * 8 = 16 (^ has higher precedence than *)
    let expr = parse_expr("2*2^3");
    assert_eq!(eval(&expr), 16);
}

#[test]
fn left_assoc_still_works() {
    // Verify left-associative operators still work correctly
    // 10-3-2 = (10-3)-2 = 5
    let expr = parse_expr("10-3-2");
    assert_eq!(eval(&expr), 5);
}

#[test]
fn mixed_precedence_and_assoc() {
    // 1+2^3*4 = 1 + (8 * 4) = 1 + 32 = 33
    // Wait, ^ has higher precedence than *, so: 1 + ((2^3) * 4) = 1 + (8 * 4) = 33
    let expr = parse_expr("1+2^3*4");
    assert_eq!(eval(&expr), 33);
}
