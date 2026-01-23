//! Parser for choice with backtracking.

use trampoline_parser::{CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        // Choice between "ab" and "a" - tests backtracking
        .rule("choice", |r| r.choice((r.lit("ab"), r.lit("a"))))
        .build()
}
