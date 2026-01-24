//! Parser for JSON.

use trampoline_parser::{CombinatorExt, CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        // Entry rule - value with optional whitespace
        .rule("json", |r| {
            r.sequence((r.parse("ws"), r.parse("value"), r.parse("ws")))
                .ast("|r, _| Ok(extract_value(r))")
        })
        // JSON value: object, array, string, number, true, false, null
        .rule("value", |r| {
            r.choice((
                r.parse("object"),
                r.parse("array"),
                r.parse("string"),
                r.parse("number"),
                r.parse("true"),
                r.parse("false"),
                r.parse("null"),
            ))
        })
        // Object: { } or { members }
        .rule("object", |r| {
            r.sequence((
                r.lit("{"),
                r.parse("ws"),
                r.optional(r.parse("members")),
                r.parse("ws"),
                r.lit("}"),
            ))
            .ast("|r, _| Ok(build_object(r))")
        })
        // Members: pair (, pair)*
        .rule("members", |r| {
            r.separated_by(r.parse("pair"), r.parse("comma"))
        })
        // Pair: string : value
        .rule("pair", |r| {
            r.sequence((
                r.parse("string"),
                r.parse("ws"),
                r.lit(":"),
                r.parse("ws"),
                r.parse("value"),
            ))
            .ast("|r, _| Ok(build_pair(r))")
        })
        // Comma with whitespace
        .rule("comma", |r| {
            r.sequence((r.parse("ws"), r.lit(","), r.parse("ws")))
        })
        // Array: [ ] or [ elements ]
        .rule("array", |r| {
            r.sequence((
                r.lit("["),
                r.parse("ws"),
                r.optional(r.parse("elements")),
                r.parse("ws"),
                r.lit("]"),
            ))
            .ast("|r, _| Ok(build_array(r))")
        })
        // Elements: value (, value)*
        .rule("elements", |r| {
            r.separated_by(r.parse("value"), r.parse("comma"))
        })
        // String: " chars "
        .rule("string", |r| {
            r.sequence((r.lit("\""), r.parse("chars"), r.lit("\"")))
                .ast("|r, _| Ok(build_string(r))")
        })
        // Characters inside string
        .rule("chars", |r| {
            r.capture(r.zero_or_more(r.choice((r.parse("escaped"), r.parse("unescaped")))))
        })
        // Escaped character: \n, \t, \", \\, etc.
        .rule("escaped", |r| {
            r.sequence((
                r.lit("\\"),
                r.choice((
                    r.lit("\""),
                    r.lit("\\"),
                    r.lit("/"),
                    r.lit("b"),
                    r.lit("f"),
                    r.lit("n"),
                    r.lit("r"),
                    r.lit("t"),
                )),
            ))
        })
        // Unescaped character: anything except " and \ and control chars
        .rule("unescaped", |r| {
            r.sequence((
                r.not_followed_by(r.choice((r.lit("\""), r.lit("\\")))),
                r.any_char(),
            ))
        })
        // Number: -? int frac? exp?
        .rule("number", |r| {
            r.capture(r.sequence((
                r.optional(r.lit("-")),
                r.parse("int"),
                r.optional(r.parse("frac")),
                r.optional(r.parse("exp")),
            )))
            .ast("|r, _| Ok(build_number(r))")
        })
        // Integer part
        .rule("int", |r| {
            r.choice((
                r.lit("0"),
                r.sequence((r.parse("digit19"), r.zero_or_more(r.digit()))),
            ))
        })
        // Non-zero digit
        .rule("digit19", |r| r.range('1', '9'))
        // Fraction
        .rule("frac", |r| {
            r.sequence((r.lit("."), r.one_or_more(r.digit())))
        })
        // Exponent
        .rule("exp", |r| {
            r.sequence((
                r.choice((r.lit("e"), r.lit("E"))),
                r.optional(r.choice((r.lit("+"), r.lit("-")))),
                r.one_or_more(r.digit()),
            ))
        })
        // Literals
        .rule("true", |r| {
            r.lit("true")
                .ast("|_, _| Ok(ParseResult::Json(JsonValue::Bool(true)))")
        })
        .rule("false", |r| {
            r.lit("false")
                .ast("|_, _| Ok(ParseResult::Json(JsonValue::Bool(false)))")
        })
        .rule("null", |r| {
            r.lit("null")
                .ast("|_, _| Ok(ParseResult::Json(JsonValue::Null))")
        })
        // Whitespace
        .rule("ws", |r| {
            r.skip(r.zero_or_more(r.choice((r.lit(" "), r.lit("\t"), r.lit("\n"), r.lit("\r")))))
        })
        .ast_config(|c| {
            c.helper(HELPER_CODE)
                .result_variant("Json", "JsonValue")
                .apply_mappings()
        })
        .build()
}

const HELPER_CODE: &str = r#"
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

fn extract_value(r: ParseResult) -> ParseResult {
    match r {
        ParseResult::List(items) => {
            // [ws, value, ws] - extract the middle element
            items.into_iter().nth(1).unwrap_or(ParseResult::None)
        }
        other => other,
    }
}

fn build_object(r: ParseResult) -> ParseResult {
    let mut map = HashMap::new();
    if let ParseResult::List(items) = r {
        // items: ["{", ws, optional(members), ws, "}"]
        if let Some(ParseResult::List(members)) = items.into_iter().nth(2) {
            for member in members {
                if let ParseResult::List(pair) = member {
                    let mut iter = pair.into_iter();
                    if let (Some(ParseResult::Json(JsonValue::String(key))), Some(value)) =
                        (iter.next(), iter.next())
                    {
                        if let ParseResult::Json(v) = value {
                            map.insert(key, v);
                        }
                    }
                }
            }
        }
    }
    ParseResult::Json(JsonValue::Object(map))
}

fn build_pair(r: ParseResult) -> ParseResult {
    if let ParseResult::List(items) = r {
        // items: [string, ws, ":", ws, value]
        let mut iter = items.into_iter();
        let key = iter.next();
        let _ = iter.next(); // ws
        let _ = iter.next(); // ":"
        let _ = iter.next(); // ws
        let value = iter.next();
        if let (Some(k), Some(v)) = (key, value) {
            return ParseResult::List(vec![k, v]);
        }
    }
    ParseResult::None
}

fn build_array(r: ParseResult) -> ParseResult {
    let mut arr = Vec::new();
    if let ParseResult::List(items) = r {
        // items: ["[", ws, optional(elements), ws, "]"]
        if let Some(ParseResult::List(elements)) = items.into_iter().nth(2) {
            for elem in elements {
                if let ParseResult::Json(v) = elem {
                    arr.push(v);
                }
            }
        }
    }
    ParseResult::Json(JsonValue::Array(arr))
}

fn build_string(r: ParseResult) -> ParseResult {
    if let ParseResult::List(items) = r {
        // items: ["\"", chars, "\""]
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
                        Some('/') => result.push('/'),
                        Some('b') => result.push('\u{0008}'),
                        Some('f') => result.push('\u{000C}'),
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
            return ParseResult::Json(JsonValue::String(result));
        }
    }
    ParseResult::Json(JsonValue::String(String::new()))
}

fn build_number(r: ParseResult) -> ParseResult {
    if let ParseResult::Text(s, _) = r {
        if let Ok(n) = s.parse::<f64>() {
            return ParseResult::Json(JsonValue::Number(n));
        }
    }
    ParseResult::Json(JsonValue::Number(0.0))
}
"#;
