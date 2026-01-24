//! Tests for postfix operators: call, index, member, simple (++, --).

use trampoline_parser_tests::postfix_parser::{Expr, ParseResult, Parser};

fn parse_expr(input: &str) -> Expr {
    let mut parser = Parser::new(input);
    let result = parser.parse().expect("parse should succeed");
    to_expr(result)
}

fn to_expr(r: ParseResult) -> Expr {
    match r {
        ParseResult::Expr(e) => e,
        ParseResult::Text(s, _) => {
            if let Ok(n) = s.parse::<i64>() {
                Expr::Num(n)
            } else {
                Expr::Ident(s)
            }
        }
        _ => panic!("expected Expr"),
    }
}

// =============================================================================
// Simple postfix operators (++, --)
// =============================================================================

#[test]
fn postfix_simple_increment() {
    let expr = parse_expr("a++");
    assert_eq!(expr, Expr::PostInc(Box::new(Expr::Ident("a".into()))));
}

#[test]
fn postfix_simple_decrement() {
    let expr = parse_expr("a--");
    assert_eq!(expr, Expr::PostDec(Box::new(Expr::Ident("a".into()))));
}

#[test]
fn postfix_double_increment() {
    // a++++ should be (a++)++
    let expr = parse_expr("a++++");
    assert_eq!(
        expr,
        Expr::PostInc(Box::new(Expr::PostInc(Box::new(Expr::Ident("a".into())))))
    );
}

// =============================================================================
// Call expressions
// =============================================================================

#[test]
fn postfix_call_no_args() {
    let expr = parse_expr("f()");
    assert_eq!(expr, Expr::Call(Box::new(Expr::Ident("f".into())), vec![]));
}

#[test]
fn postfix_call_one_arg() {
    let expr = parse_expr("f(1)");
    assert_eq!(
        expr,
        Expr::Call(Box::new(Expr::Ident("f".into())), vec![Expr::Num(1)])
    );
}

#[test]
fn postfix_call_multiple_args() {
    let expr = parse_expr("f(1, 2, 3)");
    assert_eq!(
        expr,
        Expr::Call(
            Box::new(Expr::Ident("f".into())),
            vec![Expr::Num(1), Expr::Num(2), Expr::Num(3)]
        )
    );
}

#[test]
fn postfix_call_with_identifiers() {
    let expr = parse_expr("f(a, b)");
    assert_eq!(
        expr,
        Expr::Call(
            Box::new(Expr::Ident("f".into())),
            vec![Expr::Ident("a".into()), Expr::Ident("b".into())]
        )
    );
}

#[test]
fn postfix_call_chained() {
    // f()() should be (f())()
    let expr = parse_expr("f()()");
    assert_eq!(
        expr,
        Expr::Call(
            Box::new(Expr::Call(Box::new(Expr::Ident("f".into())), vec![])),
            vec![]
        )
    );
}

// =============================================================================
// Index expressions
// =============================================================================

#[test]
fn postfix_index_simple() {
    let expr = parse_expr("a[0]");
    assert_eq!(
        expr,
        Expr::Index(Box::new(Expr::Ident("a".into())), Box::new(Expr::Num(0)))
    );
}

#[test]
fn postfix_index_with_identifier() {
    let expr = parse_expr("a[i]");
    assert_eq!(
        expr,
        Expr::Index(
            Box::new(Expr::Ident("a".into())),
            Box::new(Expr::Ident("i".into()))
        )
    );
}

#[test]
fn postfix_index_chained() {
    // a[0][1] should be (a[0])[1]
    let expr = parse_expr("a[0][1]");
    assert_eq!(
        expr,
        Expr::Index(
            Box::new(Expr::Index(
                Box::new(Expr::Ident("a".into())),
                Box::new(Expr::Num(0))
            )),
            Box::new(Expr::Num(1))
        )
    );
}

// =============================================================================
// Member access expressions
// =============================================================================

#[test]
fn postfix_member_simple() {
    let expr = parse_expr("a.b");
    assert_eq!(
        expr,
        Expr::Member(Box::new(Expr::Ident("a".into())), "b".into())
    );
}

#[test]
fn postfix_member_chained() {
    // a.b.c should be (a.b).c
    let expr = parse_expr("a.b.c");
    assert_eq!(
        expr,
        Expr::Member(
            Box::new(Expr::Member(Box::new(Expr::Ident("a".into())), "b".into())),
            "c".into()
        )
    );
}

#[test]
fn postfix_member_long_chain() {
    let expr = parse_expr("a.b.c.d.e");
    // ((((a.b).c).d).e)
    let expected = Expr::Member(
        Box::new(Expr::Member(
            Box::new(Expr::Member(
                Box::new(Expr::Member(Box::new(Expr::Ident("a".into())), "b".into())),
                "c".into(),
            )),
            "d".into(),
        )),
        "e".into(),
    );
    assert_eq!(expr, expected);
}

// =============================================================================
// Mixed postfix chains
// =============================================================================

#[test]
fn postfix_mixed_member_call() {
    // a.b() should be (a.b)()
    let expr = parse_expr("a.b()");
    assert_eq!(
        expr,
        Expr::Call(
            Box::new(Expr::Member(Box::new(Expr::Ident("a".into())), "b".into())),
            vec![]
        )
    );
}

#[test]
fn postfix_mixed_call_member() {
    // a().b should be (a()).b
    let expr = parse_expr("a().b");
    assert_eq!(
        expr,
        Expr::Member(
            Box::new(Expr::Call(Box::new(Expr::Ident("a".into())), vec![])),
            "b".into()
        )
    );
}

#[test]
fn postfix_mixed_index_member() {
    // a[0].b should be (a[0]).b
    let expr = parse_expr("a[0].b");
    assert_eq!(
        expr,
        Expr::Member(
            Box::new(Expr::Index(
                Box::new(Expr::Ident("a".into())),
                Box::new(Expr::Num(0))
            )),
            "b".into()
        )
    );
}

#[test]
fn postfix_mixed_member_index() {
    // a.b[0] should be (a.b)[0]
    let expr = parse_expr("a.b[0]");
    assert_eq!(
        expr,
        Expr::Index(
            Box::new(Expr::Member(Box::new(Expr::Ident("a".into())), "b".into())),
            Box::new(Expr::Num(0))
        )
    );
}

#[test]
fn postfix_complex_chain() {
    // a.b[0].c().d should be ((((a.b)[0]).c)()).d
    let expr = parse_expr("a.b[0].c().d");
    let expected = Expr::Member(
        Box::new(Expr::Call(
            Box::new(Expr::Member(
                Box::new(Expr::Index(
                    Box::new(Expr::Member(Box::new(Expr::Ident("a".into())), "b".into())),
                    Box::new(Expr::Num(0)),
                )),
                "c".into(),
            )),
            vec![],
        )),
        "d".into(),
    );
    assert_eq!(expr, expected);
}

#[test]
fn postfix_call_with_member_arg() {
    // f(a.b) - call with member access as argument
    let expr = parse_expr("f(a.b)");
    assert_eq!(
        expr,
        Expr::Call(
            Box::new(Expr::Ident("f".into())),
            vec![Expr::Member(Box::new(Expr::Ident("a".into())), "b".into())]
        )
    );
}

#[test]
fn postfix_call_with_index_arg() {
    // f(a[0]) - call with index as argument
    let expr = parse_expr("f(a[0])");
    assert_eq!(
        expr,
        Expr::Call(
            Box::new(Expr::Ident("f".into())),
            vec![Expr::Index(
                Box::new(Expr::Ident("a".into())),
                Box::new(Expr::Num(0))
            )]
        )
    );
}

// =============================================================================
// Postfix with binary operators
// =============================================================================

#[test]
fn postfix_call_in_addition() {
    // f() + g() should be (f()) + (g())
    let expr = parse_expr("f() + g()");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Call(Box::new(Expr::Ident("f".into())), vec![])),
            trampoline_parser_tests::postfix_parser::BinOp::Add,
            Box::new(Expr::Call(Box::new(Expr::Ident("g".into())), vec![]))
        )
    );
}

#[test]
fn postfix_member_in_multiplication() {
    // a.x * b.y should be (a.x) * (b.y)
    let expr = parse_expr("a.x * b.y");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Member(Box::new(Expr::Ident("a".into())), "x".into())),
            trampoline_parser_tests::postfix_parser::BinOp::Mul,
            Box::new(Expr::Member(Box::new(Expr::Ident("b".into())), "y".into()))
        )
    );
}

#[test]
fn postfix_index_in_expression() {
    // a[0] + b[1] should be (a[0]) + (b[1])
    let expr = parse_expr("a[0] + b[1]");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Index(
                Box::new(Expr::Ident("a".into())),
                Box::new(Expr::Num(0))
            )),
            trampoline_parser_tests::postfix_parser::BinOp::Add,
            Box::new(Expr::Index(
                Box::new(Expr::Ident("b".into())),
                Box::new(Expr::Num(1))
            ))
        )
    );
}

// =============================================================================
// Postfix with whitespace
// =============================================================================

#[test]
fn postfix_call_with_spaces() {
    let expr = parse_expr("f( 1 , 2 )");
    assert_eq!(
        expr,
        Expr::Call(
            Box::new(Expr::Ident("f".into())),
            vec![Expr::Num(1), Expr::Num(2)]
        )
    );
}

#[test]
fn postfix_index_with_spaces() {
    let expr = parse_expr("a[ 0 ]");
    assert_eq!(
        expr,
        Expr::Index(Box::new(Expr::Ident("a".into())), Box::new(Expr::Num(0)))
    );
}

// =============================================================================
// Stress tests - deep chaining
// =============================================================================

#[test]
fn postfix_deep_member_chain() {
    // 50 levels of member access: a.b.c.d...
    let input: String = (0..50)
        .map(|i| format!("m{}", i))
        .collect::<Vec<_>>()
        .join(".");
    let mut parser = Parser::new(&input);
    let result = parser.parse();
    assert!(result.is_ok(), "should parse 50 levels of member access");
}

#[test]
fn postfix_deep_call_chain() {
    // 50 levels of calls: f()()()...
    let input = format!("f{}", "()".repeat(50));
    let mut parser = Parser::new(&input);
    let result = parser.parse();
    assert!(result.is_ok(), "should parse 50 levels of calls");
}

#[test]
fn postfix_deep_index_chain() {
    // 50 levels of indexing: a[0][1][2]...
    let indices: String = (0..50).map(|i| format!("[{}]", i)).collect();
    let input = format!("a{}", indices);
    let mut parser = Parser::new(&input);
    let result = parser.parse();
    assert!(result.is_ok(), "should parse 50 levels of indexing");
}

#[test]
fn postfix_mixed_deep_chain() {
    // Mixed chain: a.b[0].c().d[1]...
    let mut parts = vec!["a".to_string()];
    for i in 0..30 {
        match i % 3 {
            0 => parts.push(format!(".m{}", i)),
            1 => parts.push(format!("[{}]", i)),
            _ => parts.push("()".to_string()),
        }
    }
    let input = parts.join("");
    let mut parser = Parser::new(&input);
    let result = parser.parse();
    assert!(result.is_ok(), "should parse mixed postfix chain");
}
