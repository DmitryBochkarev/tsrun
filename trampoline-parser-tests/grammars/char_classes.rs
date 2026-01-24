//! Grammars for testing character class combinators.

use trampoline_parser::{CompiledGrammar, Grammar};

/// Test hex_digit() character class
pub fn hex() -> CompiledGrammar {
    Grammar::new()
        .rule("hex", |r| r.capture(r.one_or_more(r.hex_digit())))
        .build()
}

/// Test alpha_num() character class
pub fn alphanum() -> CompiledGrammar {
    Grammar::new()
        .rule("alphanum", |r| r.capture(r.one_or_more(r.alpha_num())))
        .build()
}

/// Test ident_start() and ident_cont() character classes
pub fn ident() -> CompiledGrammar {
    Grammar::new()
        .rule("ident", |r| {
            r.capture(r.sequence((r.ident_start(), r.zero_or_more(r.ident_cont()))))
        })
        .build()
}

/// Test range() combinator with lowercase letters
pub fn lowercase() -> CompiledGrammar {
    Grammar::new()
        .rule("lowercase", |r| r.capture(r.one_or_more(r.range('a', 'z'))))
        .build()
}

/// Test range() combinator with uppercase letters
pub fn uppercase() -> CompiledGrammar {
    Grammar::new()
        .rule("uppercase", |r| r.capture(r.one_or_more(r.range('A', 'Z'))))
        .build()
}

/// Test range() combinator with custom range (0-5)
pub fn custom_range() -> CompiledGrammar {
    Grammar::new()
        .rule("custom_range", |r| {
            r.capture(r.one_or_more(r.range('0', '5')))
        })
        .build()
}
