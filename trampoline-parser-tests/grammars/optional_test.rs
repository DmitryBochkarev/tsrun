//! Grammar for testing the optional() combinator.

use trampoline_parser::{CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        // signed_number: optional sign followed by digits
        .rule("signed_number", |r| {
            r.sequence((
                r.optional(r.capture(r.choice((r.lit("+"), r.lit("-"))))),
                r.capture(r.one_or_more(r.digit())),
            ))
        })
        .build()
}
