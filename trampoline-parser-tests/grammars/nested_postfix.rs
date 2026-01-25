//! Test grammar for postfix operators in deeply nested contexts.
//! Mimics the TypeScript issue where postfix member access fails inside
//! object literals that are inside parenthesized expressions.

use trampoline_parser::{Assoc, Combinator, CombinatorExt, CompiledGrammar, Grammar, RuleBuilder};

/// Helper for operator with trailing whitespace
fn op(r: &RuleBuilder, operator: &str) -> Combinator {
    r.sequence((r.lit(operator), r.parse("ws")))
}

/// Grammar that reproduces the TypeScript postfix bug:
/// - expression = assignment_expression (outer Pratt)
/// - primary includes object_expression and parenthesized
/// - object_property value is assignment_expression (inner Pratt)
/// - Pratt has postfix_member
pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        // Expression = assignment_expression (, assignment_expression)* like TypeScript
        .rule("expression", |r| {
            r.sequence((
                r.parse("assignment_expression"),
                r.zero_or_more(r.sequence((op(r, ","), r.parse("assignment_expression")))),
            ))
            .ast("|r, _| { if let ParseResult::List(mut items) = r { Ok(items.remove(0)) } else { Ok(r) } }")
        })
        // Assignment expression - Pratt with MANY operators like TypeScript
        // NOTE: Wrapped in sequence to mimic TypeScript's structure
        .rule("assignment_expression", |r| {
            r.sequence((
                r.pratt(r.parse("primary"), |ops| {
                    ops
                        // Assignment operators
                        .infix(ws_op(r, "="), 2, Assoc::Right, "|l, r, _| Ok(r)")
                        .infix(ws_op(r, "+="), 2, Assoc::Right, "|l, r, _| Ok(r)")
                        .infix(ws_op(r, "-="), 2, Assoc::Right, "|l, r, _| Ok(r)")
                        // Logical operators
                        .infix(ws_op(r, "||"), 4, Assoc::Left, "|l, r, _| Ok(l)")
                        .infix(ws_op(r, "&&"), 5, Assoc::Left, "|l, r, _| Ok(l)")
                        // Comparison
                        .infix(ws_op(r, "==="), 10, Assoc::Left, "|l, r, _| Ok(l)")
                        .infix(ws_op(r, "!=="), 10, Assoc::Left, "|l, r, _| Ok(l)")
                        .infix(ws_op(r, "=="), 10, Assoc::Left, "|l, r, _| Ok(l)")
                        .infix(ws_op(r, "!="), 10, Assoc::Left, "|l, r, _| Ok(l)")
                        // Relational
                        .infix(ws_op(r, "<="), 11, Assoc::Left, "|l, r, _| Ok(l)")
                        .infix(ws_op(r, ">="), 11, Assoc::Left, "|l, r, _| Ok(l)")
                        .infix(ws_op(r, "<"), 11, Assoc::Left, "|l, r, _| Ok(l)")
                        .infix(ws_op(r, ">"), 11, Assoc::Left, "|l, r, _| Ok(l)")
                        // Arithmetic
                        .infix(ws_op(r, "+"), 13, Assoc::Left, "|l, r, _| Ok(l)")
                        .infix(ws_op(r, "-"), 13, Assoc::Left, "|l, r, _| Ok(l)")
                        .infix(ws_op(r, "*"), 14, Assoc::Left, "|l, r, _| Ok(l)")
                        .infix(ws_op(r, "/"), 14, Assoc::Left, "|l, r, _| Ok(l)")
                        // Prefix
                        .prefix(ws_op(r, "-"), 16, "|e, _| Ok(e)")
                        .prefix(ws_op(r, "+"), 16, "|e, _| Ok(e)")
                        .prefix(ws_op(r, "!"), 16, "|e, _| Ok(e)")
                        // Postfix
                        .postfix("++", 17, "|e, _| Ok(e)")
                        .postfix("--", 17, "|e, _| Ok(e)")
                        // Member access and call (highest precedence)
                        .postfix_call("?.(", ")", ",", 18, "|c, a, _| Ok(c)")
                        .postfix_call("(", ")", ",", 18, "|c, a, _| Ok(c)")
                        .postfix_index("?.[", "]", 18, "|o, e, _| Ok(o)")
                        .postfix_index("[", "]", 18, "|o, e, _| Ok(o)")
                        .postfix_member("?.", 18, "|o, p, _| Ok(o)")
                        .postfix_member(".", 18, "|o, p, _| Ok(o)")
                }),
                r.parse("ws"),  // Like TypeScript's optional "as Type" suffix
            ))
            .ast("|r, _| { if let ParseResult::List(mut items) = r { Ok(items.remove(0)) } else { Ok(r) } }")
        })
        // Primary expression with leading ws (for after infix operators)
        .rule("primary", |r| {
            r.sequence((r.parse("ws"), r.parse("primary_inner")))
                .ast("|r, _| { if let ParseResult::List(mut items) = r { Ok(items.pop().unwrap_or(ParseResult::None)) } else { Ok(r) } }")
        })
        // Primary inner - various expression types
        // NOTE: arrow_function before parenthesized to mimic TypeScript
        .rule("primary_inner", |r| {
            r.choice((
                r.parse("object_expression"),
                r.parse("arrow_function"),  // tries first, may fail and backtrack
                r.parse("parenthesized"),
                r.parse("identifier"), // identifier with trailing ws
                r.capture(r.one_or_more(r.digit())), // number
            ))
        })
        // Arrow function: (params) => body
        // This will fail for ({ a: x.y }) and backtrack to parenthesized
        .rule("arrow_function", |r| {
            r.sequence((
                op(r, "("),
                r.optional(r.parse("param_list")),
                op(r, ")"),
                op(r, "=>"),
                r.parse("assignment_expression"),
            ))
        })
        // Parameter list: may include destructuring patterns
        .rule("param_list", |r| {
            r.separated_by(r.parse("param"), op(r, ","))
        })
        // Parameter: identifier or object pattern
        .rule("param", |r| {
            r.choice((
                r.parse("object_pattern"),  // { a: x } - destructuring
                r.parse("identifier"),
            ))
        })
        // Object pattern (destructuring): { a: b, c }
        .rule("object_pattern", |r| {
            r.sequence((
                op(r, "{"),
                r.optional(r.separated_by_trailing(r.parse("object_pattern_prop"), op(r, ","))),
                op(r, "}"),
            ))
        })
        // Object pattern property: key or key: pattern
        .rule("object_pattern_prop", |r| {
            r.sequence((
                r.parse("identifier"),  // key
                r.optional(r.sequence((op(r, ":"), r.parse("param")))),  // optional: pattern
            ))
        })
        // Parenthesized expression - wraps expression in parens
        .rule("parenthesized", |r| {
            r.sequence((op(r, "("), r.parse("expression"), op(r, ")")))
                .ast("|r, _| { if let ParseResult::List(mut items) = r { items.remove(1); Ok(items.remove(0)) } else { Ok(r) } }")
        })
        // Object expression: { properties }
        .rule("object_expression", |r| {
            r.sequence((
                op(r, "{"),
                r.optional(r.separated_by_trailing(r.parse("object_property"), op(r, ","))),
                op(r, "}"),
            ))
        })
        // Object property: choice like TypeScript (method_property before key_value)
        .rule("object_property", |r| {
            r.choice((
                r.parse("method_property"),   // method(params) { } - tries first, will backtrack
                r.parse("key_value_property"), // key: value
            ))
        })
        // Method property - like TypeScript with optional async/*, will match key then fail on (
        .rule("method_property", |r| {
            r.sequence((
                r.optional(r.sequence((r.lit("async"), r.parse("ws")))),  // optional async
                r.optional(r.sequence((r.lit("*"), r.parse("ws")))),      // optional *
                r.parse("identifier"),  // parses key with trailing ws
                op(r, "("),             // this will fail and cause backtrack
                // ... rest doesn't matter, we'll never get here for key: value
            ))
        })
        // Key-value property: key: value (uses assignment_expression to avoid comma issues)
        .rule("key_value_property", |r| {
            r.sequence((
                r.parse("identifier"),  // uses identifier with trailing ws
                op(r, ":"),             // colon + ws after
                r.parse("assignment_expression"),  // NESTED PRATT PARSER!
                r.parse("ws"),          // ws after value (before , or })
            ))
        })
        // Identifier with trailing whitespace (like TypeScript)
        .rule("identifier", |r| {
            r.sequence((
                r.capture(r.one_or_more(r.alpha())),
                r.parse("ws"),
            ))
            .ast("|r, _| { if let ParseResult::List(mut items) = r { Ok(items.remove(0)) } else { Ok(r) } }")
        })
        // Whitespace - skip spaces
        .rule("ws", |r| r.skip(r.zero_or_more(r.ws())))
        .build()
}

/// Helper: operator with leading whitespace (for Pratt infix)
fn ws_op(r: &RuleBuilder, operator: &str) -> Combinator {
    r.sequence((r.parse("ws"), r.lit(operator)))
}
