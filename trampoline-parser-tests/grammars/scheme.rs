//! Parser for Scheme (R5RS subset).
//!
//! Exercises underused trampoline-parser features:
//! - Multiple prefix operators (quote family)
//! - Extended symbol character classes
//! - Character literals
//! - Dotted pairs
//! - Hash-prefixed disambiguation

use trampoline_parser::{CombinatorExt, CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        // Entry rule - program with whitespace
        .rule("program", |r| {
            r.sequence((r.parse("ws"), r.parse("datum"), r.parse("ws")))
                .ast("|r, _| Ok(extract_datum(r))")
        })
        // Main datum rule
        .rule("datum", |r| {
            r.choice((
                r.parse("quoted"),
                r.parse("list"),
                r.parse("vector"),
                r.parse("atom"),
            ))
        })
        // === Quote family (prefix operators) ===
        // Order matters: unquote_splicing before unquote (,@ before ,)
        .rule("quoted", |r| {
            r.choice((
                r.parse("quote"),
                r.parse("quasiquote"),
                r.parse("unquote_splicing"),
                r.parse("unquote"),
            ))
        })
        .rule("quote", |r| {
            r.sequence((r.char('\''), r.parse("datum")))
                .ast("|r, _| Ok(build_quote(r))")
        })
        .rule("quasiquote", |r| {
            r.sequence((r.char('`'), r.parse("datum")))
                .ast("|r, _| Ok(build_quasiquote(r))")
        })
        .rule("unquote_splicing", |r| {
            r.sequence((r.lit(",@"), r.parse("datum")))
                .ast("|r, _| Ok(build_unquote_splicing(r))")
        })
        .rule("unquote", |r| {
            r.sequence((r.char(','), r.parse("datum")))
                .ast("|r, _| Ok(build_unquote(r))")
        })
        // === Lists ===
        // Factor out common prefix to avoid exponential backtracking
        .rule("list", |r| {
            r.choice((r.parse("empty_list"), r.parse("non_empty_list")))
        })
        .rule("empty_list", |r| {
            r.sequence((r.char('('), r.parse("ws"), r.char(')')))
                .ast("|r, _| Ok(build_empty_list(r))")
        })
        // Non-empty list: handles both proper (a b c) and dotted (a b . c)
        // The dotted tail is optional to avoid re-parsing the common prefix
        .rule("non_empty_list", |r| {
            r.sequence((
                r.char('('),
                r.parse("ws"),
                r.one_or_more(r.sequence((r.parse("datum"), r.parse("ws")))),
                r.optional(r.parse("dotted_tail")),
                r.char(')'),
            ))
            .ast("|r, _| Ok(build_non_empty_list(r))")
        })
        .rule("dotted_tail", |r| {
            r.sequence((
                r.char('.'),
                r.not_followed_by(r.digit()), // Don't match float like (1 .5)
                r.parse("ws1"),
                r.parse("datum"),
                r.parse("ws"),
            ))
        })
        // === Vector ===
        .rule("vector", |r| {
            r.sequence((
                r.lit("#("),
                r.parse("ws"),
                r.zero_or_more(r.sequence((r.parse("datum"), r.parse("ws")))),
                r.char(')'),
            ))
            .ast("|r, _| Ok(build_vector(r))")
        })
        // === Atoms ===
        .rule("atom", |r| {
            r.choice((
                r.parse("boolean"),
                r.parse("character"),
                r.parse("number"),
                r.parse("string"),
                r.parse("symbol"),
            ))
        })
        // Boolean: #t or #f
        .rule("boolean", |r| {
            r.choice((
                r.lit("#t")
                    .ast("|_, _| Ok(ParseResult::Scheme(SchemeValue::Boolean(true)))"),
                r.lit("#f")
                    .ast("|_, _| Ok(ParseResult::Scheme(SchemeValue::Boolean(false)))"),
            ))
        })
        // Character: #\x, #\newline, #\space, #\tab
        .rule("character", |r| {
            r.sequence((r.lit("#\\"), r.parse("char_value")))
                .ast("|r, _| Ok(build_character(r))")
        })
        .rule("char_value", |r| {
            r.capture(r.choice((r.lit("newline"), r.lit("space"), r.lit("tab"), r.any_char())))
        })
        // Number: integers, floats, scientific notation
        .rule("number", |r| {
            r.capture(r.sequence((
                r.optional(r.choice((r.char('+'), r.char('-')))),
                r.parse("unsigned_number"),
            )))
            .ast("|r, _| Ok(build_number(r))")
        })
        .rule("unsigned_number", |r| {
            r.choice((r.parse("float_number"), r.parse("int_number")))
        })
        .rule("float_number", |r| {
            r.sequence((
                r.one_or_more(r.digit()),
                r.char('.'),
                r.zero_or_more(r.digit()),
                r.optional(r.parse("exponent")),
            ))
        })
        .rule("int_number", |r| {
            r.sequence((r.one_or_more(r.digit()), r.optional(r.parse("exponent"))))
        })
        .rule("exponent", |r| {
            r.sequence((
                r.choice((r.char('e'), r.char('E'))),
                r.optional(r.choice((r.char('+'), r.char('-')))),
                r.one_or_more(r.digit()),
            ))
        })
        // String: "..."
        .rule("string", |r| {
            r.sequence((r.char('"'), r.parse("string_chars"), r.char('"')))
                .ast("|r, _| Ok(build_string(r))")
        })
        .rule("string_chars", |r| {
            r.capture(r.zero_or_more(r.choice((
                r.sequence((r.char('\\'), r.any_char())),
                r.sequence((
                    r.not_followed_by(r.choice((r.char('"'), r.char('\\')))),
                    r.any_char(),
                )),
            ))))
        })
        // Symbol: extended identifiers
        // Scheme symbols can include many special characters
        .rule("symbol", |r| {
            r.choice((
                r.parse("normal_identifier"), // Try normal first (handles ->, +x, etc.)
                r.parse("peculiar_identifier"), // Then standalone +, -, ...
            ))
            .ast("|r, _| Ok(build_symbol(r))")
        })
        // Peculiar identifiers: standalone +, -, ...
        // Only match when NOT followed by subsequent characters
        .rule("peculiar_identifier", |r| {
            r.capture(r.choice((
                r.lit("..."),
                r.sequence((
                    r.choice((r.char('+'), r.char('-'))),
                    r.not_followed_by(r.parse("subsequent")),
                )),
            )))
        })
        // Normal identifiers: start with letter or special, continue with alphanum or special
        .rule("normal_identifier", |r| {
            r.capture(r.sequence((r.parse("initial"), r.zero_or_more(r.parse("subsequent")))))
        })
        // Initial: letter or special initial character
        .rule("initial", |r| {
            r.choice((r.alpha(), r.parse("special_initial")))
        })
        // Special initial characters (includes + and - for identifiers like -> and +inf)
        .rule("special_initial", |r| {
            r.choice(vec![
                r.char('!'),
                r.char('$'),
                r.char('%'),
                r.char('&'),
                r.char('*'),
                r.char('+'),
                r.char('-'),
                r.char('/'),
                r.char(':'),
                r.char('<'),
                r.char('='),
                r.char('>'),
                r.char('?'),
                r.char('^'),
                r.char('_'),
                r.char('~'),
            ])
        })
        // Subsequent: initial or digit or special subsequent
        .rule("subsequent", |r| {
            r.choice((r.parse("initial"), r.digit(), r.parse("special_subsequent")))
        })
        // Special subsequent characters
        .rule("special_subsequent", |r| {
            r.choice((r.char('+'), r.char('-'), r.char('.'), r.char('@')))
        })
        // === Whitespace and comments ===
        .rule("ws", |r| {
            r.skip(r.zero_or_more(r.choice((r.ws(), r.parse("comment")))))
        })
        .rule("ws1", |r| {
            r.skip(r.one_or_more(r.choice((r.ws(), r.parse("comment")))))
        })
        .rule("comment", |r| {
            r.sequence((
                r.char(';'),
                r.zero_or_more(r.sequence((r.not_followed_by(r.char('\n')), r.any_char()))),
            ))
        })
        .ast_config(|c| {
            c.helper(HELPER_CODE)
                .result_variant("Scheme", "SchemeValue")
                .apply_mappings()
        })
        .build()
}

const HELPER_CODE: &str = r#"
#[derive(Debug, Clone, PartialEq)]
pub enum SchemeValue {
    Number(f64),
    Symbol(String),
    String(String),
    Boolean(bool),
    Character(char),
    Nil,
    Pair(Box<SchemeValue>, Box<SchemeValue>),
    Vector(Vec<SchemeValue>),
    Quote(Box<SchemeValue>),
    Quasiquote(Box<SchemeValue>),
    Unquote(Box<SchemeValue>),
    UnquoteSplicing(Box<SchemeValue>),
}

fn build_empty_list(_r: ParseResult) -> ParseResult {
    ParseResult::Scheme(SchemeValue::Nil)
}

fn extract_datum(r: ParseResult) -> ParseResult {
    match r {
        ParseResult::List(items) => {
            // [ws, datum, ws] - extract the middle element
            items.into_iter().nth(1).unwrap_or(ParseResult::None)
        }
        other => other,
    }
}

fn to_scheme(r: ParseResult) -> SchemeValue {
    match r {
        ParseResult::Scheme(v) => v,
        ParseResult::List(items) => {
            // Try to extract scheme value from list
            for item in items {
                if let ParseResult::Scheme(v) = item {
                    return v;
                }
            }
            SchemeValue::Nil
        }
        ParseResult::Text(s, _) => SchemeValue::Symbol(s),
        ParseResult::None => SchemeValue::Nil,
    }
}

fn build_quote(r: ParseResult) -> ParseResult {
    if let ParseResult::List(items) = r {
        // [', datum]
        let datum = items.into_iter().nth(1).unwrap_or(ParseResult::None);
        let value = to_scheme(datum);
        ParseResult::Scheme(SchemeValue::Quote(Box::new(value)))
    } else {
        ParseResult::None
    }
}

fn build_quasiquote(r: ParseResult) -> ParseResult {
    if let ParseResult::List(items) = r {
        // [`, datum]
        let datum = items.into_iter().nth(1).unwrap_or(ParseResult::None);
        let value = to_scheme(datum);
        ParseResult::Scheme(SchemeValue::Quasiquote(Box::new(value)))
    } else {
        ParseResult::None
    }
}

fn build_unquote(r: ParseResult) -> ParseResult {
    if let ParseResult::List(items) = r {
        // [,, datum]
        let datum = items.into_iter().nth(1).unwrap_or(ParseResult::None);
        let value = to_scheme(datum);
        ParseResult::Scheme(SchemeValue::Unquote(Box::new(value)))
    } else {
        ParseResult::None
    }
}

fn build_unquote_splicing(r: ParseResult) -> ParseResult {
    if let ParseResult::List(items) = r {
        // [,@, datum]
        let datum = items.into_iter().nth(1).unwrap_or(ParseResult::None);
        let value = to_scheme(datum);
        ParseResult::Scheme(SchemeValue::UnquoteSplicing(Box::new(value)))
    } else {
        ParseResult::None
    }
}

fn build_non_empty_list(r: ParseResult) -> ParseResult {
    if let ParseResult::List(items) = r {
        // items: [(, ws, one_or_more([datum, ws]), optional(dotted_tail), )]
        // Index: 0   1    2                        3                       4
        let elements = items.get(2).cloned().unwrap_or(ParseResult::None);
        let dotted_tail = items.get(3).cloned().unwrap_or(ParseResult::None);

        let values = extract_datum_list(elements);

        // Check if there's a dotted tail
        let tail_value = match dotted_tail {
            ParseResult::List(tail_items) if !tail_items.is_empty() => {
                // dotted_tail: [., not_followed_by, ws1, datum, ws]
                // Index:        0   1               2    3      4
                let tail_datum = tail_items.get(3).cloned().unwrap_or(ParseResult::None);
                to_scheme(tail_datum)
            }
            _ => SchemeValue::Nil, // Proper list ends with Nil
        };

        // Build list from values
        let mut result = tail_value;
        for v in values.into_iter().rev() {
            result = SchemeValue::Pair(Box::new(v), Box::new(result));
        }
        ParseResult::Scheme(result)
    } else {
        ParseResult::None
    }
}

fn extract_datum_list(r: ParseResult) -> Vec<SchemeValue> {
    let mut values = Vec::new();
    if let ParseResult::List(items) = r {
        for item in items {
            // Each item is [datum, ws]
            if let ParseResult::List(pair) = item {
                if let Some(datum) = pair.into_iter().next() {
                    values.push(to_scheme(datum));
                }
            }
        }
    }
    values
}

fn build_vector(r: ParseResult) -> ParseResult {
    if let ParseResult::List(items) = r {
        // items: [#(, ws, zero_or_more([datum, ws]), )]
        let elements = items.into_iter().nth(2).unwrap_or(ParseResult::None);
        let values = extract_datum_list(elements);
        ParseResult::Scheme(SchemeValue::Vector(values))
    } else {
        ParseResult::Scheme(SchemeValue::Vector(Vec::new()))
    }
}

fn build_character(r: ParseResult) -> ParseResult {
    if let ParseResult::List(items) = r {
        // items: [#\, char_value]
        if let Some(ParseResult::Text(s, _)) = items.into_iter().nth(1) {
            let c = match s.as_str() {
                "newline" => '\n',
                "space" => ' ',
                "tab" => '\t',
                _ => s.chars().next().unwrap_or(' '),
            };
            return ParseResult::Scheme(SchemeValue::Character(c));
        }
    }
    ParseResult::Scheme(SchemeValue::Character(' '))
}

fn build_number(r: ParseResult) -> ParseResult {
    if let ParseResult::Text(s, _) = r {
        if let Ok(n) = s.parse::<f64>() {
            return ParseResult::Scheme(SchemeValue::Number(n));
        }
    }
    ParseResult::Scheme(SchemeValue::Number(0.0))
}

fn build_string(r: ParseResult) -> ParseResult {
    if let ParseResult::List(items) = r {
        // items: [", chars, "]
        if let Some(ParseResult::Text(s, _)) = items.into_iter().nth(1) {
            // Process escape sequences
            let mut result = String::new();
            let mut chars = s.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    match chars.next() {
                        Some('n') => result.push('\n'),
                        Some('r') => result.push('\r'),
                        Some('t') => result.push('\t'),
                        Some('\\') => result.push('\\'),
                        Some('"') => result.push('"'),
                        Some(other) => {
                            result.push('\\');
                            result.push(other);
                        }
                        None => result.push('\\'),
                    }
                } else {
                    result.push(c);
                }
            }
            return ParseResult::Scheme(SchemeValue::String(result));
        }
    }
    ParseResult::Scheme(SchemeValue::String(String::new()))
}

fn build_symbol(r: ParseResult) -> ParseResult {
    match r {
        ParseResult::Text(s, _) => ParseResult::Scheme(SchemeValue::Symbol(s)),
        ParseResult::List(items) => {
            // Could be from normal_identifier or peculiar_identifier
            for item in items {
                if let ParseResult::Text(s, _) = item {
                    return ParseResult::Scheme(SchemeValue::Symbol(s));
                }
            }
            ParseResult::Scheme(SchemeValue::Symbol(String::new()))
        }
        _ => ParseResult::Scheme(SchemeValue::Symbol(String::new())),
    }
}
"#;
