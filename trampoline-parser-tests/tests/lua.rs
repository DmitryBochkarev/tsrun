//! Tests for Lua parser.
//!
//! Tests cover:
//! - Literals (numbers, strings, nil, true, false)
//! - Expressions (operators, precedence, associativity)
//! - Postfix operators (calls, indexing, member access)
//! - Tables
//! - Statements
//! - Deep nesting stress tests

// Use lua_expr_parser for expression parsing (entry point: expr)
use trampoline_parser_tests::lua_expr_parser::{
    BinOp, Expr, Field, ParseResult as ExprParseResult, Parser as ExprParser, UnOp,
};

// Use lua_parser for statement parsing (entry point: chunk)
use trampoline_parser_tests::lua_parser::{
    ParseResult as ChunkParseResult, Parser as ChunkParser, Stmt,
};

fn parse_expr(input: &str) -> Expr {
    let mut parser = ExprParser::new(input);
    let result = parser.parse().expect("parse should succeed");
    to_expr(result)
}

fn parse_chunk(input: &str) -> Vec<Stmt> {
    let mut parser = ChunkParser::new(input);
    let result = parser.parse().expect("parse should succeed");
    if let ChunkParseResult::Stmts(stmts) = result {
        stmts
    } else {
        panic!("expected Stmts, got {:?}", result);
    }
}

fn to_expr(r: ExprParseResult) -> Expr {
    match r {
        ExprParseResult::Expr(e) => e,
        ExprParseResult::List(items) => {
            if let Some(first) = items.into_iter().next() {
                to_expr(first)
            } else {
                Expr::Nil
            }
        }
        _ => panic!("expected Expr, got {:?}", r),
    }
}

// =============================================================================
// Literals
// =============================================================================

#[test]
fn lua_nil() {
    assert_eq!(parse_expr("nil"), Expr::Nil);
}

#[test]
fn lua_true() {
    assert_eq!(parse_expr("true"), Expr::Bool(true));
}

#[test]
fn lua_false() {
    assert_eq!(parse_expr("false"), Expr::Bool(false));
}

#[test]
fn lua_integer() {
    assert_eq!(parse_expr("42"), Expr::Number("42".into()));
}

#[test]
fn lua_float() {
    assert_eq!(parse_expr("3.14"), Expr::Number("3.14".into()));
}

#[test]
fn lua_float_no_decimal() {
    assert_eq!(parse_expr("3."), Expr::Number("3.".into()));
}

#[test]
fn lua_scientific() {
    assert_eq!(parse_expr("1e10"), Expr::Number("1e10".into()));
}

#[test]
fn lua_scientific_negative() {
    assert_eq!(parse_expr("1e-5"), Expr::Number("1e-5".into()));
}

#[test]
fn lua_hex() {
    assert_eq!(parse_expr("0xFF"), Expr::Number("0xFF".into()));
}

#[test]
fn lua_hex_lower() {
    assert_eq!(parse_expr("0xabc"), Expr::Number("0xabc".into()));
}

#[test]
fn lua_double_string() {
    assert_eq!(parse_expr("\"hello\""), Expr::String("hello".into()));
}

#[test]
fn lua_single_string() {
    assert_eq!(parse_expr("'world'"), Expr::String("world".into()));
}

#[test]
fn lua_raw_string() {
    assert_eq!(parse_expr("[[raw]]"), Expr::String("raw".into()));
}

#[test]
fn lua_raw_string_multiline() {
    assert_eq!(
        parse_expr("[[line1\nline2]]"),
        Expr::String("line1\nline2".into())
    );
}

#[test]
fn lua_string_escape() {
    assert_eq!(parse_expr("\"a\\nb\""), Expr::String("a\\nb".into()));
}

#[test]
fn lua_identifier() {
    assert_eq!(parse_expr("foo"), Expr::Ident("foo".into()));
}

#[test]
fn lua_identifier_with_underscore() {
    assert_eq!(parse_expr("foo_bar"), Expr::Ident("foo_bar".into()));
}

#[test]
fn lua_identifier_starting_underscore() {
    assert_eq!(parse_expr("_private"), Expr::Ident("_private".into()));
}

// =============================================================================
// Binary operators
// =============================================================================

#[test]
fn lua_addition() {
    let expr = parse_expr("1 + 2");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Number("1".into())),
            BinOp::Add,
            Box::new(Expr::Number("2".into()))
        )
    );
}

#[test]
fn lua_subtraction() {
    let expr = parse_expr("5 - 3");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Number("5".into())),
            BinOp::Sub,
            Box::new(Expr::Number("3".into()))
        )
    );
}

#[test]
fn lua_multiplication() {
    let expr = parse_expr("2 * 3");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Number("2".into())),
            BinOp::Mul,
            Box::new(Expr::Number("3".into()))
        )
    );
}

#[test]
fn lua_division() {
    let expr = parse_expr("10 / 2");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Number("10".into())),
            BinOp::Div,
            Box::new(Expr::Number("2".into()))
        )
    );
}

#[test]
fn lua_floor_division() {
    let expr = parse_expr("10 // 3");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Number("10".into())),
            BinOp::FloorDiv,
            Box::new(Expr::Number("3".into()))
        )
    );
}

#[test]
fn lua_modulo() {
    let expr = parse_expr("10 % 3");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Number("10".into())),
            BinOp::Mod,
            Box::new(Expr::Number("3".into()))
        )
    );
}

#[test]
fn lua_power() {
    let expr = parse_expr("2 ^ 3");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Number("2".into())),
            BinOp::Pow,
            Box::new(Expr::Number("3".into()))
        )
    );
}

#[test]
fn lua_concat() {
    let expr = parse_expr("\"a\" .. \"b\"");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::String("a".into())),
            BinOp::Concat,
            Box::new(Expr::String("b".into()))
        )
    );
}

#[test]
fn lua_comparison_eq() {
    let expr = parse_expr("a == b");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Ident("a".into())),
            BinOp::Eq,
            Box::new(Expr::Ident("b".into()))
        )
    );
}

#[test]
fn lua_comparison_neq() {
    let expr = parse_expr("a ~= b");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Ident("a".into())),
            BinOp::NotEq,
            Box::new(Expr::Ident("b".into()))
        )
    );
}

#[test]
fn lua_comparison_lt() {
    let expr = parse_expr("a < b");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Ident("a".into())),
            BinOp::Lt,
            Box::new(Expr::Ident("b".into()))
        )
    );
}

#[test]
fn lua_comparison_le() {
    let expr = parse_expr("a <= b");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Ident("a".into())),
            BinOp::Le,
            Box::new(Expr::Ident("b".into()))
        )
    );
}

#[test]
fn lua_comparison_gt() {
    let expr = parse_expr("a > b");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Ident("a".into())),
            BinOp::Gt,
            Box::new(Expr::Ident("b".into()))
        )
    );
}

#[test]
fn lua_comparison_ge() {
    let expr = parse_expr("a >= b");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Ident("a".into())),
            BinOp::Ge,
            Box::new(Expr::Ident("b".into()))
        )
    );
}

#[test]
fn lua_logical_and() {
    let expr = parse_expr("a and b");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Ident("a".into())),
            BinOp::And,
            Box::new(Expr::Ident("b".into()))
        )
    );
}

#[test]
fn lua_logical_or() {
    let expr = parse_expr("a or b");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Ident("a".into())),
            BinOp::Or,
            Box::new(Expr::Ident("b".into()))
        )
    );
}

// =============================================================================
// Unary operators
// =============================================================================

#[test]
fn lua_unary_neg() {
    let expr = parse_expr("-x");
    assert_eq!(
        expr,
        Expr::Unary(UnOp::Neg, Box::new(Expr::Ident("x".into())))
    );
}

#[test]
fn lua_unary_not() {
    let expr = parse_expr("not x");
    assert_eq!(
        expr,
        Expr::Unary(UnOp::Not, Box::new(Expr::Ident("x".into())))
    );
}

#[test]
fn lua_unary_len() {
    let expr = parse_expr("#t");
    assert_eq!(
        expr,
        Expr::Unary(UnOp::Len, Box::new(Expr::Ident("t".into())))
    );
}

#[test]
fn lua_double_not() {
    let expr = parse_expr("not not x");
    assert_eq!(
        expr,
        Expr::Unary(
            UnOp::Not,
            Box::new(Expr::Unary(UnOp::Not, Box::new(Expr::Ident("x".into()))))
        )
    );
}

// =============================================================================
// Operator precedence
// =============================================================================

#[test]
fn lua_precedence_mul_add() {
    // 1 + 2 * 3 = 1 + (2 * 3)
    let expr = parse_expr("1 + 2 * 3");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Number("1".into())),
            BinOp::Add,
            Box::new(Expr::Binary(
                Box::new(Expr::Number("2".into())),
                BinOp::Mul,
                Box::new(Expr::Number("3".into()))
            ))
        )
    );
}

#[test]
fn lua_precedence_and_or() {
    // a or b and c = a or (b and c)
    let expr = parse_expr("a or b and c");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Ident("a".into())),
            BinOp::Or,
            Box::new(Expr::Binary(
                Box::new(Expr::Ident("b".into())),
                BinOp::And,
                Box::new(Expr::Ident("c".into()))
            ))
        )
    );
}

#[test]
fn lua_precedence_not_and() {
    // not a and b = (not a) and b
    let expr = parse_expr("not a and b");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Unary(UnOp::Not, Box::new(Expr::Ident("a".into())))),
            BinOp::And,
            Box::new(Expr::Ident("b".into()))
        )
    );
}

// =============================================================================
// Right associativity
// =============================================================================

#[test]
fn lua_right_assoc_power() {
    // 2 ^ 3 ^ 4 = 2 ^ (3 ^ 4)
    let expr = parse_expr("2 ^ 3 ^ 4");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Number("2".into())),
            BinOp::Pow,
            Box::new(Expr::Binary(
                Box::new(Expr::Number("3".into())),
                BinOp::Pow,
                Box::new(Expr::Number("4".into()))
            ))
        )
    );
}

#[test]
fn lua_right_assoc_concat() {
    // "a" .. "b" .. "c" = "a" .. ("b" .. "c")
    let expr = parse_expr("\"a\" .. \"b\" .. \"c\"");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::String("a".into())),
            BinOp::Concat,
            Box::new(Expr::Binary(
                Box::new(Expr::String("b".into())),
                BinOp::Concat,
                Box::new(Expr::String("c".into()))
            ))
        )
    );
}

// =============================================================================
// Postfix operators
// =============================================================================

#[test]
fn lua_call_no_args() {
    let expr = parse_expr("f()");
    assert_eq!(expr, Expr::Call(Box::new(Expr::Ident("f".into())), vec![]));
}

#[test]
fn lua_call_one_arg() {
    let expr = parse_expr("f(1)");
    assert_eq!(
        expr,
        Expr::Call(
            Box::new(Expr::Ident("f".into())),
            vec![Expr::Number("1".into())]
        )
    );
}

#[test]
fn lua_call_multiple_args() {
    let expr = parse_expr("f(1, 2, 3)");
    assert_eq!(
        expr,
        Expr::Call(
            Box::new(Expr::Ident("f".into())),
            vec![
                Expr::Number("1".into()),
                Expr::Number("2".into()),
                Expr::Number("3".into())
            ]
        )
    );
}

#[test]
fn lua_index() {
    let expr = parse_expr("t[1]");
    assert_eq!(
        expr,
        Expr::Index(
            Box::new(Expr::Ident("t".into())),
            Box::new(Expr::Number("1".into()))
        )
    );
}

#[test]
fn lua_member() {
    let expr = parse_expr("t.x");
    assert_eq!(
        expr,
        Expr::Member(Box::new(Expr::Ident("t".into())), "x".into())
    );
}

#[test]
fn lua_member_mul() {
    // Test member access with infix operator
    let expr = parse_expr("a.x * b.y");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Member(Box::new(Expr::Ident("a".into())), "x".into())),
            BinOp::Mul,
            Box::new(Expr::Member(Box::new(Expr::Ident("b".into())), "y".into()))
        )
    );
}

#[test]
fn lua_postfix_chain() {
    // a.b[0].c()
    let expr = parse_expr("a.b[0].c()");
    let expected = Expr::Call(
        Box::new(Expr::Member(
            Box::new(Expr::Index(
                Box::new(Expr::Member(Box::new(Expr::Ident("a".into())), "b".into())),
                Box::new(Expr::Number("0".into())),
            )),
            "c".into(),
        )),
        vec![],
    );
    assert_eq!(expr, expected);
}

// =============================================================================
// Tables
// =============================================================================

#[test]
fn lua_empty_table() {
    let expr = parse_expr("{}");
    assert_eq!(expr, Expr::Table(vec![]));
}

#[test]
fn lua_array_table() {
    let expr = parse_expr("{1, 2, 3}");
    assert_eq!(
        expr,
        Expr::Table(vec![
            Field::Array(Expr::Number("1".into())),
            Field::Array(Expr::Number("2".into())),
            Field::Array(Expr::Number("3".into())),
        ])
    );
}

#[test]
fn lua_hash_table() {
    let expr = parse_expr("{x = 1, y = 2}");
    assert_eq!(
        expr,
        Expr::Table(vec![
            Field::Named("x".into(), Expr::Number("1".into())),
            Field::Named("y".into(), Expr::Number("2".into())),
        ])
    );
}

#[test]
fn lua_computed_key_table() {
    let expr = parse_expr("{[1] = \"a\"}");
    assert_eq!(
        expr,
        Expr::Table(vec![Field::Computed(
            Expr::Number("1".into()),
            Expr::String("a".into())
        )])
    );
}

#[test]
fn lua_mixed_table() {
    let expr = parse_expr("{1, x = 2, [y] = 3}");
    assert_eq!(
        expr,
        Expr::Table(vec![
            Field::Array(Expr::Number("1".into())),
            Field::Named("x".into(), Expr::Number("2".into())),
            Field::Computed(Expr::Ident("y".into()), Expr::Number("3".into())),
        ])
    );
}

#[test]
fn lua_nested_table() {
    let expr = parse_expr("{{1}, {2}}");
    assert_eq!(
        expr,
        Expr::Table(vec![
            Field::Array(Expr::Table(vec![Field::Array(Expr::Number("1".into()))])),
            Field::Array(Expr::Table(vec![Field::Array(Expr::Number("2".into()))])),
        ])
    );
}

#[test]
fn lua_table_trailing_comma() {
    let expr = parse_expr("{1, 2,}");
    assert_eq!(
        expr,
        Expr::Table(vec![
            Field::Array(Expr::Number("1".into())),
            Field::Array(Expr::Number("2".into())),
        ])
    );
}

// =============================================================================
// Statements
// =============================================================================

#[test]
fn lua_local_debug() {
    // Debug test to understand why statement parsing fails
    use trampoline_parser_tests::lua_parser::{ParseResult, Parser};

    // The issue: "return" should parse to Stmts([Return]) but returns Stmts([])
    // This means the statement parsing is failing silently in zero_or_more

    // Let's test if we can access intermediate results
    // We'll use parse_rule if available, otherwise work around it

    let test_inputs = vec![
        ("return", "just return keyword"),
        ("return 1", "return with expr"),
        ("local x", "local without init"),
        ("local x = 1", "local with init"),
        ("if true then end", "if statement"),
    ];

    for (input, desc) in test_inputs {
        let mut parser = Parser::new(input);
        let result = parser.parse();
        match result {
            Ok(ParseResult::Stmts(stmts)) => {
                eprintln!("{}: {} statements: {:?}", desc, stmts.len(), stmts);
            }
            Ok(other) => {
                eprintln!("{}: unexpected result type: {:?}", desc, other);
            }
            Err(e) => {
                eprintln!("{}: error: {:?}", desc, e);
            }
        }
    }
}

#[test]
fn lua_local_decl() {
    let stmts = parse_chunk("local x = 1");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Stmt::Local(_, _)));
}

#[test]
fn lua_local_multi() {
    let stmts = parse_chunk("local x, y = 1, 2");
    assert_eq!(stmts.len(), 1);
    if let Stmt::Local(names, exprs) = &stmts[0] {
        assert_eq!(names.len(), 2);
        assert_eq!(exprs.len(), 2);
    } else {
        panic!("expected Local");
    }
}

#[test]
fn lua_if_statement() {
    let stmts = parse_chunk("if true then end");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Stmt::If));
}

#[test]
fn lua_while_statement() {
    let stmts = parse_chunk("while true do end");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Stmt::While));
}

#[test]
fn lua_for_statement() {
    let stmts = parse_chunk("for i = 1, 10 do end");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Stmt::For));
}

#[test]
fn lua_for_with_step() {
    let stmts = parse_chunk("for i = 1, 10, 2 do end");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Stmt::For));
}

#[test]
fn lua_repeat_statement() {
    let stmts = parse_chunk("repeat until true");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Stmt::Repeat));
}

#[test]
fn lua_function_decl() {
    let stmts = parse_chunk("function f() end");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Stmt::Function));
}

#[test]
fn lua_return_statement() {
    let stmts = parse_chunk("return 1");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Stmt::Return));
}

#[test]
fn lua_multiple_statements() {
    let stmts = parse_chunk("local x = 1 local y = 2");
    assert_eq!(stmts.len(), 2);
}

// =============================================================================
// Comments
// =============================================================================

#[test]
fn lua_line_comment() {
    let expr = parse_expr("1 -- comment\n + 2");
    assert_eq!(
        expr,
        Expr::Binary(
            Box::new(Expr::Number("1".into())),
            BinOp::Add,
            Box::new(Expr::Number("2".into()))
        )
    );
}

// =============================================================================
// Stress tests
// =============================================================================

#[test]
fn lua_deep_nested_tables() {
    // 100 levels of nested tables: {{{{...}}}}
    let open = "{".repeat(100);
    let close = "}".repeat(100);
    let input = format!("{}{}", open, close);
    let mut parser = ExprParser::new(&input);
    let result = parser.parse();
    assert!(result.is_ok(), "should parse 100 levels of nested tables");
}

#[test]
fn lua_deep_nested_parens() {
    // 100 levels of nested parens: ((((1))))
    let open = "(".repeat(100);
    let close = ")".repeat(100);
    let input = format!("{}1{}", open, close);
    let mut parser = ExprParser::new(&input);
    let result = parser.parse();
    assert!(result.is_ok(), "should parse 100 levels of nested parens");
}

#[test]
fn lua_deep_call_chain() {
    // 50 levels of calls: f()()()...
    let calls = "()".repeat(50);
    let input = format!("f{}", calls);
    let mut parser = ExprParser::new(&input);
    let result = parser.parse();
    assert!(result.is_ok(), "should parse 50 levels of calls");
}

#[test]
fn lua_deep_member_chain() {
    // 100 levels of member access: a.b.c.d...
    let members: String = (0..100).map(|i| format!(".m{}", i)).collect();
    let input = format!("a{}", members);
    let mut parser = ExprParser::new(&input);
    let result = parser.parse();
    assert!(result.is_ok(), "should parse 100 levels of member access");
}

#[test]
fn lua_long_concat_chain() {
    // Long right-associative chain: "a" .. "b" .. "c" .. ...
    let parts: Vec<_> = (0..50).map(|i| format!("\"s{}\"", i)).collect();
    let input = parts.join(" .. ");
    let mut parser = ExprParser::new(&input);
    let result = parser.parse();
    assert!(result.is_ok(), "should parse long concat chain");
}

#[test]
fn lua_long_power_chain() {
    // Long right-associative chain: 2 ^ 2 ^ 2 ^ ...
    let parts: Vec<_> = (0..30).map(|_| "2").collect();
    let input = parts.join(" ^ ");
    let mut parser = ExprParser::new(&input);
    let result = parser.parse();
    assert!(result.is_ok(), "should parse long power chain");
}

#[test]
fn lua_complex_expression() {
    // Mix of all operators
    let input = "1 + 2 * 3 ^ 4 .. \"x\" == 5 and true or false";
    let mut parser = ExprParser::new(&input);
    let result = parser.parse();
    assert!(result.is_ok(), "should parse complex expression");
}

#[test]
fn lua_large_table() {
    // Table with many fields
    let fields: Vec<_> = (0..100).map(|i| format!("x{} = {}", i, i)).collect();
    let input = format!("{{{}}}", fields.join(", "));
    let mut parser = ExprParser::new(&input);
    let result = parser.parse();
    assert!(result.is_ok(), "should parse large table");
}
