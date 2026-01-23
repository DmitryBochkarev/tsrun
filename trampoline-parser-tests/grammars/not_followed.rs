//! Parser for negative lookahead: "a" not followed by "b"

use trampoline_parser::{CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("a_not_b", |r| {
            r.sequence((r.lit("a"), r.not_followed_by(r.lit("b"))))
        })
        .build()
}
