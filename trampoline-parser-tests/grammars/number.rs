//! Parser for captured number (one or more digits).

use trampoline_parser::{CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("number", |r| r.capture(r.one_or_more(r.digit())))
        .build()
}
