//! Grammars demonstrating backtracking issues.
//!
//! This module contains two versions of a simple nested list grammar:
//! - `bad_grammar`: Has exponential backtracking due to shared prefix
//! - `good_grammar`: Factored prefix eliminates backtracking

use trampoline_parser::{CombinatorExt, CompiledGrammar, Grammar};

/// Grammar with exponential backtracking.
///
/// The problem: `dotted_list` and `proper_list` share the prefix `'(' datum+`.
/// When dotted_list fails (no '.'), the parser backtracks and re-parses
/// the entire `datum+` content for proper_list.
///
/// At each nesting level, content is parsed twice -> O(2^n) time.
pub fn bad_grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("program", |r| {
            r.sequence((r.parse("ws"), r.parse("list"), r.parse("ws")))
                .ast("|r, _| { if let ParseResult::List(items) = r { Ok(items.into_iter().nth(1).unwrap_or(ParseResult::None)) } else { Ok(r) } }")
        })
        .rule("list", |r| {
            r.choice((
                r.parse("empty_list"),
                r.parse("dotted_list"),  // Tried first - shares prefix with proper_list
                r.parse("proper_list"),  // Re-parses entire content on backtrack
            ))
        })
        .rule("empty_list", |r| {
            r.sequence((r.char('('), r.parse("ws"), r.char(')')))
                .ast("|_, _| Ok(ParseResult::Text(\"()\".to_string(), Span::default()))")
        })
        // Dotted list: (a b . c)
        .rule("dotted_list", |r| {
            r.sequence((
                r.char('('),
                r.parse("ws"),
                r.one_or_more(r.sequence((r.parse("datum"), r.parse("ws")))),
                r.char('.'),
                r.parse("ws"),
                r.parse("datum"),
                r.parse("ws"),
                r.char(')'),
            ))
            .ast("|_, _| Ok(ParseResult::Text(\"dotted\".to_string(), Span::default()))")
        })
        // Proper list: (a b c) - shares prefix with dotted_list!
        .rule("proper_list", |r| {
            r.sequence((
                r.char('('),
                r.parse("ws"),
                r.one_or_more(r.sequence((r.parse("datum"), r.parse("ws")))),
                r.char(')'),
            ))
            .ast("|_, _| Ok(ParseResult::Text(\"proper\".to_string(), Span::default()))")
        })
        .rule("datum", |r| {
            r.choice((
                r.parse("list"),
                r.parse("symbol"),
            ))
        })
        .rule("symbol", |r| {
            r.capture(r.one_or_more(r.alpha()))
        })
        .rule("ws", |r| {
            r.skip(r.zero_or_more(r.ws()))
        })
        .build()
}

/// Grammar with factored prefix - no exponential backtracking.
///
/// The fix: Factor out the common prefix `'(' datum+` and make the
/// dotted tail optional. Now content is parsed only once.
///
/// Time complexity: O(n)
pub fn good_grammar() -> CompiledGrammar {
    Grammar::new()
        .rule("program", |r| {
            r.sequence((r.parse("ws"), r.parse("list"), r.parse("ws")))
                .ast("|r, _| { if let ParseResult::List(items) = r { Ok(items.into_iter().nth(1).unwrap_or(ParseResult::None)) } else { Ok(r) } }")
        })
        .rule("list", |r| {
            r.choice((
                r.parse("empty_list"),
                r.parse("non_empty_list"),  // Handles both proper and dotted
            ))
        })
        .rule("empty_list", |r| {
            r.sequence((r.char('('), r.parse("ws"), r.char(')')))
                .ast("|_, _| Ok(ParseResult::Text(\"()\".to_string(), Span::default()))")
        })
        // Non-empty list: handles both proper (a b c) and dotted (a b . c)
        // The dotted tail is OPTIONAL - no re-parsing needed
        .rule("non_empty_list", |r| {
            r.sequence((
                r.char('('),
                r.parse("ws"),
                r.one_or_more(r.sequence((r.parse("datum"), r.parse("ws")))),
                r.optional(r.parse("dotted_tail")),
                r.char(')'),
            ))
            .ast("|_, _| Ok(ParseResult::Text(\"list\".to_string(), Span::default()))")
        })
        .rule("dotted_tail", |r| {
            r.sequence((
                r.char('.'),
                r.parse("ws"),
                r.parse("datum"),
                r.parse("ws"),
            ))
        })
        .rule("datum", |r| {
            r.choice((
                r.parse("list"),
                r.parse("symbol"),
            ))
        })
        .rule("symbol", |r| {
            r.capture(r.one_or_more(r.alpha()))
        })
        .rule("ws", |r| {
            r.skip(r.zero_or_more(r.ws()))
        })
        .build()
}
