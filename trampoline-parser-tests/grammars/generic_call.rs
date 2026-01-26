//! Grammar demonstrating backtracking issue with generic call expressions.
//!
//! The pattern `identifier < type_args > ( args )` can cause exponential backtracking
//! when parsing input with many `<` characters, because the parser tries many
//! interpretations of `<` as either a type argument delimiter or less-than operator.
//!
//! This module contains:
//! - `bad_grammar`: Has exponential backtracking due to naive generic call matching
//! - `good_grammar`: Uses simpler type arguments to avoid exponential backtracking

use trampoline_parser::{Assoc, CombinatorExt, CompiledGrammar, Grammar, RuleBuilder};

/// Helper for operators with whitespace
fn op(r: &RuleBuilder, operator: &str) -> trampoline_parser::Combinator {
    r.sequence((r.lit(operator), r.parse("ws")))
}

/// Grammar with exponential backtracking on generic calls.
///
/// The problem: `generic_call` uses `type_arguments` which allows nested types.
/// When parsing `a<b<c<d<...`, the parser explores many possibilities:
/// - Is `<b` a type argument or comparison?
/// - If type argument, is `<c` nested type or end of type args?
/// - etc.
///
/// This leads to exponential backtracking as each `<` doubles the possibilities.
pub fn bad_grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("expr", |r| {
            r.pratt(r.parse("primary"), |ops| {
                ops.infix(r.sequence((r.parse("ws"), r.lit("<"))), 5, Assoc::Left, "|l, r, _| Ok(l)")
                    .infix(r.sequence((r.parse("ws"), r.lit(">"))), 5, Assoc::Left, "|l, r, _| Ok(l)")
                    .postfix_call("(", ")", ",", 10, "|c, a, _| Ok(c)")
            })
        })
        .rule("primary", |r| {
            r.sequence((r.parse("ws"), r.parse("primary_inner")))
                .ast("|r, _| { if let ParseResult::List(mut items) = r { Ok(items.pop().unwrap_or(ParseResult::None)) } else { Ok(r) } }")
        })
        .rule("primary_inner", |r| {
            r.choice((
                // generic_call before identifier - causes backtracking!
                r.parse("generic_call"),
                r.parse("identifier"),
            ))
        })
        // Generic call: identifier<types>(args)
        // Uses full type_arguments which allows nested types
        .rule("generic_call", |r| {
            r.sequence((
                r.parse("identifier"),
                r.parse("type_arguments"),
                op(r, "("),
                r.optional(r.parse("identifier")),
                op(r, ")"),
            ))
        })
        // Type arguments with full type support - causes exponential backtracking
        .rule("type_arguments", |r| {
            r.sequence((
                op(r, "<"),
                r.separated_by(r.parse("type"), op(r, ",")),
                op(r, ">"),
            ))
        })
        // Full type - can be nested and cause backtracking
        .rule("type", |r| {
            r.choice((
                r.parse("type_reference"),
                r.parse("identifier"),
            ))
        })
        // Type reference with optional nested type arguments
        .rule("type_reference", |r| {
            r.sequence((r.parse("identifier"), r.optional(r.parse("type_arguments"))))
        })
        .rule("identifier", |r| {
            r.sequence((r.capture(r.one_or_more(r.alpha())), r.parse("ws")))
        })
        .rule("ws", |r| r.skip(r.zero_or_more(r.ws())))
        .build()
}

/// Grammar with optimized generic calls - uses simple type arguments.
///
/// The fix: Use `simple_type_arguments` that only allows identifiers,
/// not nested generic types. This prevents exponential backtracking.
///
/// For complex type arguments like `Map<K, V>`, the parser would need
/// additional strategies (lookahead, different parsing approach).
pub fn good_grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("expr", |r| {
            r.pratt(r.parse("primary"), |ops| {
                ops.infix(r.sequence((r.parse("ws"), r.lit("<"))), 5, Assoc::Left, "|l, r, _| Ok(l)")
                    .infix(r.sequence((r.parse("ws"), r.lit(">"))), 5, Assoc::Left, "|l, r, _| Ok(l)")
                    .postfix_call("(", ")", ",", 10, "|c, a, _| Ok(c)")
            })
        })
        .rule("primary", |r| {
            r.sequence((r.parse("ws"), r.parse("primary_inner")))
                .ast("|r, _| { if let ParseResult::List(mut items) = r { Ok(items.pop().unwrap_or(ParseResult::None)) } else { Ok(r) } }")
        })
        .rule("primary_inner", |r| {
            r.choice((
                r.parse("generic_call"),
                r.parse("identifier"),
            ))
        })
        // Generic call with simple type arguments
        .rule("generic_call", |r| {
            r.sequence((
                r.parse("identifier"),
                r.parse("simple_type_arguments"),
                op(r, "("),
                r.optional(r.parse("identifier")),
                op(r, ")"),
            ))
        })
        // Simple type arguments - only identifiers, no nesting
        .rule("simple_type_arguments", |r| {
            r.sequence((
                op(r, "<"),
                r.separated_by(r.parse("identifier"), op(r, ",")),
                op(r, ">"),
            ))
        })
        .rule("identifier", |r| {
            r.sequence((r.capture(r.one_or_more(r.alpha())), r.parse("ws")))
        })
        .rule("ws", |r| r.skip(r.zero_or_more(r.ws())))
        .build()
}

/// Grammar with automatic memoization using `build_with_memoization()`.
///
/// This grammar is identical to bad_grammar, but uses the automatic
/// memoization detection to wrap problematic rules.
pub fn auto_memoized_grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("expr", |r| {
            r.pratt(r.parse("primary"), |ops| {
                ops.infix(r.sequence((r.parse("ws"), r.lit("<"))), 5, Assoc::Left, "|l, r, _| Ok(l)")
                    .infix(r.sequence((r.parse("ws"), r.lit(">"))), 5, Assoc::Left, "|l, r, _| Ok(l)")
                    .postfix_call("(", ")", ",", 10, "|c, a, _| Ok(c)")
            })
        })
        .rule("primary", |r| {
            r.sequence((r.parse("ws"), r.parse("primary_inner")))
                .ast("|r, _| { if let ParseResult::List(mut items) = r { Ok(items.pop().unwrap_or(ParseResult::None)) } else { Ok(r) } }")
        })
        .rule("primary_inner", |r| {
            r.choice((
                r.parse("generic_call"),
                r.parse("identifier"),
            ))
        })
        // Generic call: identifier<types>(args)
        // Uses full type_arguments which would cause backtracking without memoization
        .rule("generic_call", |r| {
            r.sequence((
                r.parse("identifier"),
                r.parse("type_arguments"),
                op(r, "("),
                r.optional(r.parse("identifier")),
                op(r, ")"),
            ))
        })
        // Type arguments with full type support
        .rule("type_arguments", |r| {
            r.sequence((
                op(r, "<"),
                r.separated_by(r.parse("type"), op(r, ",")),
                op(r, ">"),
            ))
        })
        // Full type - can be nested
        .rule("type", |r| {
            r.choice((
                r.parse("type_reference"),
                r.parse("identifier"),
            ))
        })
        // Type reference with optional nested type arguments
        .rule("type_reference", |r| {
            r.sequence((r.parse("identifier"), r.optional(r.parse("type_arguments"))))
        })
        .rule("identifier", |r| {
            r.sequence((r.capture(r.one_or_more(r.alpha())), r.parse("ws")))
        })
        .rule("ws", |r| r.skip(r.zero_or_more(r.ws())))
        .build_with_memoization() // Uses automatic memoization detection
}

/// Grammar with memoization applied to fix the exponential backtracking.
///
/// By wrapping `generic_call` with `.memoize(id, ...)`, we cache the result
/// at each position. When parsing `a<b<c<d<...`, after we try (and fail)
/// `generic_call` at the position of `a`, we cache the failure. The next time
/// we need to try `generic_call` at that position, we instantly return the
/// cached failure instead of re-parsing.
pub fn memoized_grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("expr", |r| {
            r.pratt(r.parse("primary"), |ops| {
                ops.infix(r.sequence((r.parse("ws"), r.lit("<"))), 5, Assoc::Left, "|l, r, _| Ok(l)")
                    .infix(r.sequence((r.parse("ws"), r.lit(">"))), 5, Assoc::Left, "|l, r, _| Ok(l)")
                    .postfix_call("(", ")", ",", 10, "|c, a, _| Ok(c)")
            })
        })
        .rule("primary", |r| {
            r.sequence((r.parse("ws"), r.parse("primary_inner")))
                .ast("|r, _| { if let ParseResult::List(mut items) = r { Ok(items.pop().unwrap_or(ParseResult::None)) } else { Ok(r) } }")
        })
        .rule("primary_inner", |r| {
            r.choice((
                // Memoize the generic_call rule to avoid exponential backtracking
                r.memoize(1, r.parse("generic_call")),
                r.parse("identifier"),
            ))
        })
        // Generic call: identifier<types>(args)
        // Uses full type_arguments which would cause backtracking without memoization
        .rule("generic_call", |r| {
            r.sequence((
                r.parse("identifier"),
                r.parse("type_arguments"),
                op(r, "("),
                r.optional(r.parse("identifier")),
                op(r, ")"),
            ))
        })
        // Type arguments with full type support
        .rule("type_arguments", |r| {
            r.sequence((
                op(r, "<"),
                r.separated_by(r.parse("type"), op(r, ",")),
                op(r, ">"),
            ))
        })
        // Full type - can be nested
        .rule("type", |r| {
            r.choice((
                r.parse("type_reference"),
                r.parse("identifier"),
            ))
        })
        // Type reference with optional nested type arguments
        .rule("type_reference", |r| {
            r.sequence((r.parse("identifier"), r.optional(r.parse("type_arguments"))))
        })
        .rule("identifier", |r| {
            r.sequence((r.capture(r.one_or_more(r.alpha())), r.parse("ws")))
        })
        .rule("ws", |r| r.skip(r.zero_or_more(r.ws())))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bad_grammar_simple_case() {
        let grammar = bad_grammar();
        let code = grammar.generate();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_good_grammar_simple_case() {
        let grammar = good_grammar();
        let code = grammar.generate();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_memoized_grammar_simple_case() {
        let grammar = memoized_grammar();
        let code = grammar.generate();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_auto_memoization_identifies_candidates() {
        use trampoline_parser::identify_memoization_candidates;

        // The bad grammar should have memoization candidates identified
        let bad = Grammar::new()
            .rule("primary_inner", |r| {
                r.choice((r.parse("generic_call"), r.parse("identifier")))
            })
            .rule("generic_call", |r| {
                r.sequence((
                    r.parse("identifier"),
                    r.parse("type_arguments"),
                    r.lit("("),
                    r.optional(r.parse("identifier")),
                    r.lit(")"),
                ))
            })
            .rule("type_arguments", |r| {
                r.sequence((
                    r.lit("<"),
                    r.separated_by(r.parse("type"), r.lit(",")),
                    r.lit(">"),
                ))
            })
            .rule("type", |r| {
                r.choice((r.parse("type_reference"), r.parse("identifier")))
            })
            .rule("type_reference", |r| {
                r.sequence((r.parse("identifier"), r.optional(r.parse("type_arguments"))))
            })
            .rule("identifier", |r| r.capture(r.one_or_more(r.alpha())));

        let candidates = identify_memoization_candidates(&bad.rules);

        // Should identify rules in the common prefix of the choice
        // The prefix contains "generic_call" which should be identified
        assert!(
            candidates.contains("generic_call"),
            "Should identify generic_call as memoization candidate, got: {:?}",
            candidates
        );
    }
}
