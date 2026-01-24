//! Parser that uses Pratt expressions inside a separated list.
//! This mimics JavaScript array syntax like [1 + 2, foo * bar]

use trampoline_parser::{Assoc, Combinator, CompiledGrammar, Grammar};

/// Grammar with Pratt expressions inside arrays
pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("array", |r| {
            r.sequence((
                r.lit("["),
                r.optional(r.separated_by_trailing(r.parse("expr"), r.lit(","))),
                r.lit("]"),
            ))
        })
        .rule("expr", |r| {
            r.pratt(r.parse("primary"), |ops| {
                ops.infix("+", 1, Assoc::Left, "|l, r, _s| Ok(l)")
                    .infix("*", 2, Assoc::Left, "|l, r, _s| Ok(l)")
                    .prefix("-", 3, "|e, _s| Ok(e)")
            })
        })
        // Primary is just an identifier or number
        .rule("primary", |r| {
            r.choice((
                r.capture(r.one_or_more(r.alpha())), // identifier
                r.capture(r.one_or_more(r.digit())), // number
            ))
        })
        .build()
}

/// Grammar with postfix operators (call, member, index) - more like TypeScript
pub fn grammar_with_postfix() -> CompiledGrammar {
    Grammar::new()
        .rule("array", |r| {
            r.sequence((
                r.lit("["),
                r.optional(r.separated_by_trailing(r.parse("expr"), r.lit(","))),
                r.lit("]"),
            ))
        })
        .rule("expr", |r| {
            r.pratt(r.parse("primary"), |ops| {
                ops.infix("+", 1, Assoc::Left, "|l, r, _s| Ok(l)")
                    .infix("*", 2, Assoc::Left, "|l, r, _s| Ok(l)")
                    .prefix("-", 3, "|e, _s| Ok(e)")
                    // Postfix operators
                    .postfix_call("(", ")", ",", 4, "|c, a, _s| Ok(c)")
                    .postfix_member(".", 4, "|o, p, _s| Ok(o)")
                    .postfix_index("[", "]", 4, "|o, e, _s| Ok(o)")
            })
        })
        // Primary is just an identifier or number
        .rule("primary", |r| {
            r.choice((
                r.capture(r.one_or_more(r.alpha())), // identifier
                r.capture(r.one_or_more(r.digit())), // number
            ))
        })
        .build()
}

/// Grammar more like TypeScript: array_element = choice(spread, expression)
/// Uses whitespace after separators like TypeScript
pub fn grammar_ts_like() -> CompiledGrammar {
    Grammar::new()
        .rule("array", |r| {
            r.sequence((
                op(r, "["),
                r.optional(r.separated_by_trailing(r.parse("element"), op(r, ","))),
                op(r, "]"),
            ))
        })
        // Element wraps expression in a choice (like TypeScript's array_element)
        .rule("element", |r| {
            r.choice((r.parse("spread"), r.parse("expr")))
        })
        .rule("spread", |r| r.sequence((r.lit("..."), r.parse("expr"))))
        .rule("expr", |r| {
            r.pratt(r.parse("primary"), |ops| {
                ops.infix("+", 1, Assoc::Left, "|l, r, _s| Ok(l)")
                    .infix("*", 2, Assoc::Left, "|l, r, _s| Ok(l)")
                    .prefix("-", 3, "|e, _s| Ok(e)")
                    // Postfix operators
                    .postfix_call("(", ")", ",", 4, "|c, a, _s| Ok(c)")
                    .postfix_member(".", 4, "|o, p, _s| Ok(o)")
                    .postfix_index("[", "]", 4, "|o, e, _s| Ok(o)")
            })
        })
        .rule("primary", |r| {
            r.choice((
                r.sequence((r.capture(r.one_or_more(r.alpha())), r.parse("ws"))),
                r.sequence((r.capture(r.one_or_more(r.digit())), r.parse("ws"))),
                r.parse("array"), // Nested arrays!
            ))
        })
        .rule("ws", |r| r.skip(r.zero_or_more(r.ws())))
        .build()
}

/// Helper for operator with whitespace
fn op(r: &trampoline_parser::RuleBuilder, operator: &str) -> Combinator {
    r.sequence((r.lit(operator), r.parse("ws")))
}
