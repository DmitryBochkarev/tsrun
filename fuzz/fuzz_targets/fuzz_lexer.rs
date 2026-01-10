#![no_main]

use libfuzzer_sys::fuzz_target;
use tsrun::lexer::{Lexer, TokenKind};
use tsrun::StringDict;

fuzz_target!(|data: &[u8]| {
    // Only process valid UTF-8
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    // Limit input size to avoid timeout
    if source.len() > 100_000 {
        return;
    }

    let mut dict = StringDict::new();
    let mut lexer = Lexer::new(source, &mut dict);

    // Consume all tokens - should never panic
    loop {
        let token = lexer.next_token();
        if matches!(token.kind, TokenKind::Eof) {
            break;
        }
    }
});
