//! Parser for expressions with right-associative operators.

use trampoline_parser::{Assoc, CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("expr", |r| {
            r.pratt(r.parse("number"), |ops| {
                ops.infix("+", 1, Assoc::Left, "|l, r, _| Ok(binary(l, r, Op::Add))")
                    .infix("-", 1, Assoc::Left, "|l, r, _| Ok(binary(l, r, Op::Sub))")
                    .infix("*", 2, Assoc::Left, "|l, r, _| Ok(binary(l, r, Op::Mul))")
                    .infix("^", 3, Assoc::Right, "|l, r, _| Ok(binary(l, r, Op::Pow))")
            })
        })
        .rule("number", |r| r.capture(r.one_or_more(r.digit())))
        .ast_config(|c| {
            c.helper(HELPER_CODE)
                .result_variant("Expr", "Expr")
                .apply_mappings()
        })
        .build()
}

const HELPER_CODE: &str = r#"
#[derive(Debug, Clone, PartialEq)]
pub enum Op { Add, Sub, Mul, Pow }

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(i64),
    Binary(Box<Expr>, Op, Box<Expr>),
}

fn binary(l: ParseResult, r: ParseResult, op: Op) -> ParseResult {
    let l = to_expr(l);
    let r = to_expr(r);
    ParseResult::Expr(Expr::Binary(Box::new(l), op, Box::new(r)))
}

fn to_expr(r: ParseResult) -> Expr {
    match r {
        ParseResult::Text(s, _) => Expr::Num(s.parse().unwrap_or(0)),
        ParseResult::Expr(e) => e,
        ParseResult::None => Expr::Num(0),
        ParseResult::List(items) => {
            if let Some(first) = items.into_iter().next() {
                to_expr(first)
            } else {
                Expr::Num(0)
            }
        }
    }
}
"#;
