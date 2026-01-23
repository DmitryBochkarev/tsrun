//! Grammar for testing the skip() combinator.

use trampoline_parser::{CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        // trimmed: skip leading and trailing whitespace, capture digits
        .rule("trimmed", |r| {
            r.sequence((
                r.skip(r.zero_or_more(r.ws())),
                r.capture(r.one_or_more(r.digit())),
                r.skip(r.zero_or_more(r.ws())),
            ))
        })
        .build()
}
