//! Parser for one_or_more.

use trampoline_parser::{CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("one_or_more_a", |r| r.one_or_more(r.lit("a")))
        .build()
}
