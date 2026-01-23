//! Grammar for testing keyword operators (prefix_kw, infix_kw).
//!
//! Note: This tests the keyword boundary checking - prefix_kw and infix_kw
//! should only match when the keyword is followed by a non-identifier character.
//!
//! Whitespace is handled entirely by the grammar - no automatic ws skipping.
//! Prefix operators are handled as grammar rules for proper ws handling.

use trampoline_parser::{Assoc, CombinatorExt, CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("expr", |r| {
            r.pratt(r.parse("unary"), |ops| {
                // Only infix operators in Pratt - prefix handled in grammar
                ops.infix_kw("and", 2, Assoc::Left, "|l, r, _| Ok(binary(\"and\", l, r))")
                    .infix_kw("or", 1, Assoc::Left, "|l, r, _| Ok(binary(\"or\", l, r))")
            })
        })
        // Unary handles prefix 'not' with proper ws handling
        .rule("unary", |r| {
            r.sequence((
                r.skip(r.zero_or_more(r.ws())),
                r.parse("unary_inner"),
                r.skip(r.zero_or_more(r.ws())),
            ))
            .ast("|r, _| { if let ParseResult::List(mut items) = r { Ok(items.remove(1)) } else { Ok(r) } }")
        })
        .rule("unary_inner", |r| {
            r.choice((
                r.parse("prefix_not"),
                r.parse("atom"),
            ))
        })
        .rule("prefix_not", |r| {
            r.sequence((
                r.lit("not"),
                r.not_followed_by(r.ident_cont()),
                r.parse("unary"),
            ))
            .ast("|r, _| { if let ParseResult::List(items) = r { let e = items.into_iter().last().unwrap_or(ParseResult::None); Ok(unary(\"not\", e)) } else { Ok(r) } }")
        })
        .rule("atom", |r| {
            r.choice((r.lit("true"), r.lit("false"), r.parse("ident")))
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
