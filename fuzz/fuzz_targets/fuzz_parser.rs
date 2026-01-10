#![no_main]

use libfuzzer_sys::fuzz_target;
use tsrun::parser::Parser;
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
    let mut parser = Parser::new(source, &mut dict);

    // Parse should return Ok or Err, never panic
    let _ = parser.parse_program();
});
