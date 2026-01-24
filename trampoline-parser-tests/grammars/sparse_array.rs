//! Parser for sparse arrays like JavaScript arrays with holes: [, , 3, , 5]

use trampoline_parser::{CompiledGrammar, Grammar};

/// Sparse array grammar - tests separated_by_trailing with optional elements
pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("array", |r| {
            r.sequence((
                r.lit("["),
                r.optional(r.separated_by_trailing(r.parse("element"), r.lit(","))),
                r.lit("]"),
            ))
        })
        // Element is just an optional identifier (to simulate sparse arrays)
        .rule("element", |r| r.capture(r.one_or_more(r.alpha())))
        .build()
}
