//! Parser for comma-separated list (no trailing).

use trampoline_parser::{CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("list", |r| r.separated_by(r.parse("ident"), r.lit(",")))
        .rule("ident", |r| r.capture(r.one_or_more(r.alpha())))
        .build()
}
