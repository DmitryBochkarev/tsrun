//! Test repeated prepare/free cycles for memory issues

use tsrun::Interpreter;

#[test]
fn test_repeated_prepare_free() {
    // Same pattern as the browser test: create/prepare/free in a loop
    for i in 1..=100 {
        let mut interp = Interpreter::new();
        match interp.prepare("1", None) {
            Ok(_) => {}
            Err(e) => panic!("Iteration {}: prepare failed: {}", i, e),
        }
        // interp is dropped here, freeing all memory
    }
    println!("100 iterations completed successfully");
}

#[test]
fn test_repeated_prepare_run_free() {
    // Same pattern with run() included
    for i in 1..=100 {
        let mut interp = Interpreter::new();
        match interp.prepare("1", None) {
            Ok(_) => {}
            Err(e) => panic!("Iteration {}: prepare failed: {}", i, e),
        }
        loop {
            match interp.step() {
                Ok(tsrun::StepResult::Continue) => continue,
                Ok(tsrun::StepResult::Complete(_)) | Ok(tsrun::StepResult::Done) => break,
                Ok(other) => panic!("Iteration {}: unexpected step result: {:?}", i, other),
                Err(e) => panic!("Iteration {}: step failed: {}", i, e),
            }
        }
        // interp is dropped here
    }
    println!("100 iterations completed successfully");
}
