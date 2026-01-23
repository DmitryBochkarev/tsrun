//! Parser for comma-separated list (with trailing).

use trampoline_parser::{CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("list_trailing", |r| {
            r.separated_by_trailing(r.parse("ident"), r.lit(","))
        })
        .rule("ident", |r| r.capture(r.one_or_more(r.alpha())))
        .build()
}
