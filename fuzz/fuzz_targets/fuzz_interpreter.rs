#![no_main]

use libfuzzer_sys::fuzz_target;
use tsrun::{Interpreter, StepResult};

const MAX_STEPS: usize = 100_000;

fuzz_target!(|data: &[u8]| {
    // Only process valid UTF-8
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    // Smaller limit for interpreter (more expensive per byte)
    if source.len() > 10_000 {
        return;
    }

    let mut interp = Interpreter::new();

    // Prepare should return Ok or Err
    if interp.prepare(source, None).is_err() {
        return;
    }

    // Step loop with timeout protection
    let mut steps = 0;
    loop {
        match interp.step() {
            Ok(StepResult::Continue) => {
                steps += 1;
                if steps > MAX_STEPS {
                    break; // Prevent infinite loops
                }
            }
            Ok(StepResult::Complete(_)) => break,
            Ok(StepResult::Done) => break,
            Ok(StepResult::NeedImports(_)) => break, // No module loading in fuzz
            Ok(StepResult::Suspended { .. }) => break, // No async in fuzz
            Err(_) => break,                         // Errors are expected
        }
    }
});
