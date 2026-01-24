//! Parser for nested parentheses.

use trampoline_parser::{Assoc, CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("expr", |r| {
            r.pratt(r.parse("primary"), |ops| {
                ops.infix("+", 1, Assoc::Left, "|l, r, _| Ok(binary(l, r))")
            })
        })
        .rule("primary", |r| {
            r.choice((r.parse("paren"), r.parse("number")))
        })
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
        .build()
}
