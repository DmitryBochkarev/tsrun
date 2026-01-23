use std::env;
use std::fs;
use std::path::Path;
use trampoline_parser::{Assoc, Grammar};

fn main() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir);

    // Generate all test parsers
    generate_literal_parser(out_path);
    generate_digit_parser(out_path);
    generate_number_parser(out_path);
    generate_sequence_parser(out_path);
    generate_choice_parser(out_path);
    generate_zero_or_more_parser(out_path);
    generate_one_or_more_parser(out_path);
    generate_not_followed_parser(out_path);
    generate_followed_by_parser(out_path);
    generate_list_parser(out_path);
    generate_list_trailing_parser(out_path);
    generate_arithmetic_parser(out_path);
    generate_nested_parser(out_path);

    // Tell Cargo to rerun if trampoline-parser changes
    println!("cargo:rerun-if-changed=../trampoline-parser/src");
}

fn write_parser(out_path: &Path, name: &str, code: &str) {
    let file_path = out_path.join(format!("{}.rs", name));
    fs::write(&file_path, code).expect(&format!("Failed to write {}", name));
}

/// Parser for exact literal matching: "hello"
fn generate_literal_parser(out_path: &Path) {
    let grammar = Grammar::new()
        .rule("hello", |r| r.lit("hello"))
        .build();
    write_parser(out_path, "literal_parser", &grammar.generate());
}

/// Parser for a single digit
fn generate_digit_parser(out_path: &Path) {
    let grammar = Grammar::new()
        .rule("digit", |r| r.digit())
        .build();
    write_parser(out_path, "digit_parser", &grammar.generate());
}

/// Parser for captured number (one or more digits)
fn generate_number_parser(out_path: &Path) {
    let grammar = Grammar::new()
        .rule("number", |r| r.capture(r.one_or_more(r.digit())))
        .build();
    write_parser(out_path, "number_parser", &grammar.generate());
}

/// Parser for sequence: "abc"
fn generate_sequence_parser(out_path: &Path) {
    let grammar = Grammar::new()
        .rule("abc", |r| r.sequence((r.lit("a"), r.lit("b"), r.lit("c"))))
        .build();
    write_parser(out_path, "sequence_parser", &grammar.generate());
}

/// Parser for choice with backtracking
fn generate_choice_parser(out_path: &Path) {
    let grammar = Grammar::new()
        // Choice between "ab" and "a" - tests backtracking
        .rule("choice", |r| r.choice((r.lit("ab"), r.lit("a"))))
        .build();
    write_parser(out_path, "choice_parser", &grammar.generate());
}

/// Parser for zero_or_more
fn generate_zero_or_more_parser(out_path: &Path) {
    let grammar = Grammar::new()
        .rule("zero_or_more_a", |r| r.zero_or_more(r.lit("a")))
        .build();
    write_parser(out_path, "zero_or_more_parser", &grammar.generate());
}

/// Parser for one_or_more
fn generate_one_or_more_parser(out_path: &Path) {
    let grammar = Grammar::new()
        .rule("one_or_more_a", |r| r.one_or_more(r.lit("a")))
        .build();
    write_parser(out_path, "one_or_more_parser", &grammar.generate());
}

/// Parser for negative lookahead: "a" not followed by "b"
fn generate_not_followed_parser(out_path: &Path) {
    let grammar = Grammar::new()
        .rule("a_not_b", |r| {
            r.sequence((r.lit("a"), r.not_followed_by(r.lit("b"))))
        })
        .build();
    write_parser(out_path, "not_followed_parser", &grammar.generate());
}

/// Parser for positive lookahead: "a" followed by "b" (but don't consume b)
fn generate_followed_by_parser(out_path: &Path) {
    let grammar = Grammar::new()
        .rule("a_before_b", |r| {
            r.sequence((r.lit("a"), r.followed_by(r.lit("b"))))
        })
        .build();
    write_parser(out_path, "followed_by_parser", &grammar.generate());
}

/// Parser for comma-separated list (no trailing)
fn generate_list_parser(out_path: &Path) {
    let grammar = Grammar::new()
        // Entry rule must be first
        .rule("list", |r| r.separated_by(r.parse("ident"), r.lit(",")))
        .rule("ident", |r| r.capture(r.one_or_more(r.alpha())))
        .build();
    write_parser(out_path, "list_parser", &grammar.generate());
}

/// Parser for comma-separated list (with trailing)
fn generate_list_trailing_parser(out_path: &Path) {
    let grammar = Grammar::new()
        // Entry rule must be first
        .rule("list_trailing", |r| {
            r.separated_by_trailing(r.parse("ident"), r.lit(","))
        })
        .rule("ident", |r| r.capture(r.one_or_more(r.alpha())))
        .build();
    write_parser(out_path, "list_trailing_parser", &grammar.generate());
}

/// Parser for arithmetic expressions using Pratt parsing
fn generate_arithmetic_parser(out_path: &Path) {
    let grammar = Grammar::new()
        // Entry rule must be first
        .rule("expr", |r| {
            r.pratt(r.parse("number"), |ops| {
                ops.infix("+", 1, Assoc::Left, "|l, r, _| Ok(binary(l, r, Op::Add))")
                    .infix("-", 1, Assoc::Left, "|l, r, _| Ok(binary(l, r, Op::Sub))")
                    .infix("*", 2, Assoc::Left, "|l, r, _| Ok(binary(l, r, Op::Mul))")
                    .infix("/", 2, Assoc::Left, "|l, r, _| Ok(binary(l, r, Op::Div))")
                    .prefix("-", 3, "|e, _| Ok(unary(e, Op::Neg))")
            })
        })
        .rule("number", |r| r.capture(r.one_or_more(r.digit())))
        .ast_config(|c| {
            c.helper(
                r#"
#[derive(Debug, Clone, PartialEq)]
pub enum Op { Add, Sub, Mul, Div, Neg }

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(i64),
    Binary(Box<Expr>, Op, Box<Expr>),
    Unary(Op, Box<Expr>),
}

fn binary(l: ParseResult, r: ParseResult, op: Op) -> ParseResult {
    let l = to_expr(l);
    let r = to_expr(r);
    ParseResult::Expr(Expr::Binary(Box::new(l), op, Box::new(r)))
}

fn unary(e: ParseResult, op: Op) -> ParseResult {
    let e = to_expr(e);
    ParseResult::Expr(Expr::Unary(op, Box::new(e)))
}

fn to_expr(r: ParseResult) -> Expr {
    match r {
        ParseResult::Text(s, _) => Expr::Num(s.parse().unwrap_or(0)),
        ParseResult::Expr(e) => e,
        ParseResult::None => Expr::Num(0),
        ParseResult::List(items) => {
            // For Pratt parsing, the operand might be wrapped in a List
            if let Some(first) = items.into_iter().next() {
                to_expr(first)
            } else {
                Expr::Num(0)
            }
        }
    }
}
"#,
            )
            .result_variant("Expr", "Expr")
            .apply_mappings()
        })
        .build();
    write_parser(out_path, "arithmetic_parser", &grammar.generate());
}

/// Parser for nested parentheses
fn generate_nested_parser(out_path: &Path) {
    let grammar = Grammar::new()
        // Entry rule must be first
        .rule("expr", |r| {
            r.pratt(r.parse("primary"), |ops| {
                ops.infix("+", 1, Assoc::Left, "|l, r, _| Ok(binary(l, r))")
            })
        })
        .rule("primary", |r| r.choice((r.parse("paren"), r.parse("number"))))
        .rule("paren", |r| {
            r.sequence((r.lit("("), r.parse("expr"), r.lit(")")))
        })
        .rule("number", |r| r.capture(r.one_or_more(r.digit())))
        .ast_config(|c| {
            c.helper(
                r#"
fn binary(l: ParseResult, r: ParseResult) -> ParseResult {
    ParseResult::List(vec![l, r])
}
"#,
            )
            .apply_mappings()
        })
        .build();
    write_parser(out_path, "nested_parser", &grammar.generate());
}
