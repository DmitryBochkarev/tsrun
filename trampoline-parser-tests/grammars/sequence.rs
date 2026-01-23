//! Parser for sequence: "abc"

use trampoline_parser::{CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("abc", |r| r.sequence((r.lit("a"), r.lit("b"), r.lit("c"))))
        .build()
}
