//! Parser for positive lookahead: "a" followed by "b" (but don't consume b)

use trampoline_parser::{CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("a_before_b", |r| {
            r.sequence((r.lit("a"), r.followed_by(r.lit("b"))))
        })
        .build()
}
