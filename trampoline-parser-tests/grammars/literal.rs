//! Parser for exact literal matching: "hello"

use trampoline_parser::{CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new().rule("hello", |r| r.lit("hello")).build()
}
