//! Parser for a single digit.

use trampoline_parser::{CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new().rule("digit", |r| r.digit()).build()
}
