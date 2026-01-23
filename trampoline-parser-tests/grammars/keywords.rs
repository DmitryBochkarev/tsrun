//! Grammar for testing keyword operators (prefix_kw, infix_kw).
//!
//! Note: This tests the keyword boundary checking - prefix_kw and infix_kw
//! should only match when the keyword is followed by a non-identifier character.

use trampoline_parser::{Assoc, CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("expr", |r| {
            r.pratt(r.parse("atom"), |ops| {
                ops.prefix_kw("not", 3, "|e, _| Ok(unary(\"not\", e))")
                    .infix_kw("and", 2, Assoc::Left, "|l, r, _| Ok(binary(\"and\", l, r))")
                    .infix_kw("or", 1, Assoc::Left, "|l, r, _| Ok(binary(\"or\", l, r))")
            })
        })
        .rule("atom", |r| {
            r.sequence((
                r.skip(r.zero_or_more(r.ws())),
                r.choice((r.lit("true"), r.lit("false"), r.parse("ident"))),
            ))
        })
        .rule("ident", |r| {
            r.capture(r.sequence((r.ident_start(), r.zero_or_more(r.ident_cont()))))
        })
        .ast_config(|c| {
            c.helper(HELPER_CODE)
                .result_variant("Expr", "Expr")
                .apply_mappings()
        })
        .build()
}

const HELPER_CODE: &str = r#"
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    True,
    False,
    Ident(String),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
}

fn unary(op: &str, e: ParseResult) -> ParseResult {
    ParseResult::Expr(Expr::Unary(op.to_string(), Box::new(to_expr(e))))
}

fn binary(op: &str, l: ParseResult, r: ParseResult) -> ParseResult {
    ParseResult::Expr(Expr::Binary(
        op.to_string(),
        Box::new(to_expr(l)),
        Box::new(to_expr(r)),
    ))
}

fn to_expr(r: ParseResult) -> Expr {
    match r {
        ParseResult::Text(s, _) => Expr::Ident(s),
        ParseResult::Expr(e) => e,
        ParseResult::None => {
            // This happens for literals like "true" and "false" that don't capture
            Expr::True // Default, but shouldn't happen
        }
        ParseResult::List(items) => {
            // Unwrap sequence results
            for item in items {
                match item {
                    ParseResult::None => continue,
                    other => return to_expr(other),
                }
            }
            Expr::True
        }
    }
}
"#;
