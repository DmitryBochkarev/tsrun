//! Parser for zero_or_more.

use trampoline_parser::{CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("zero_or_more_a", |r| r.zero_or_more(r.lit("a")))
        .build()
}
