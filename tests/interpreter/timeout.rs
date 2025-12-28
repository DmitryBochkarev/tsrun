//! Tests for execution timeout functionality

use tsrun::{JsError, Runtime, RuntimeResult};

#[test]
fn test_timeout_while_loop() {
    let mut runtime = Runtime::new();
    // Set a very short timeout for testing
    runtime.set_timeout_ms(50);

    let result = runtime.eval("while (true) {}");
    assert!(matches!(result, Err(JsError::Timeout { .. })));
}

#[test]
fn test_timeout_do_while_loop() {
    let mut runtime = Runtime::new();
    runtime.set_timeout_ms(50);

    let result = runtime.eval("do {} while (true)");
    assert!(matches!(result, Err(JsError::Timeout { .. })));
}

#[test]
fn test_timeout_for_loop() {
    let mut runtime = Runtime::new();
    runtime.set_timeout_ms(50);

    let result = runtime.eval("for (;;) {}");
    assert!(matches!(result, Err(JsError::Timeout { .. })));
}

#[test]
fn test_timeout_disabled_with_zero() {
    let mut runtime = Runtime::new();
    runtime.set_timeout_ms(0);

    // With timeout disabled, a non-infinite loop should complete fine
    let result = runtime.eval("let x = 0; for (let i = 0; i < 1000; i++) { x += i; } x");
    assert!(matches!(result, Ok(RuntimeResult::Complete(_))));
}

#[test]
fn test_timeout_get_and_set() {
    let mut runtime = Runtime::new();

    // Default timeout should be 3000ms
    assert_eq!(runtime.timeout_ms(), 3000);

    // Set a new timeout
    runtime.set_timeout_ms(5000);
    assert_eq!(runtime.timeout_ms(), 5000);

    // Disable timeout
    runtime.set_timeout_ms(0);
    assert_eq!(runtime.timeout_ms(), 0);
}

#[test]
fn test_timeout_error_message() {
    let mut runtime = Runtime::new();
    runtime.set_timeout_ms(50);

    let result = runtime.eval("while (true) {}");
    if let Err(JsError::Timeout {
        timeout_ms,
        elapsed_ms,
    }) = result
    {
        assert_eq!(timeout_ms, 50);
        assert!(elapsed_ms >= 50);
    } else {
        panic!("Expected Timeout error");
    }
}

#[test]
fn test_normal_execution_within_timeout() {
    let mut runtime = Runtime::new();
    runtime.set_timeout_ms(1000);

    // A simple computation should complete well within 1 second
    let result = runtime.eval("1 + 2 + 3");
    assert!(matches!(result, Ok(RuntimeResult::Complete(_))));
}

// Note: Infinite recursion test is not included because it causes Rust stack
// overflow before the timeout can trigger. The timeout check happens in the
// eval_stack loop and in loop iterations, but recursive function calls use
// Rust's call stack directly.

#[test]
fn test_timeout_labeled_while_loop() {
    let mut runtime = Runtime::new();
    runtime.set_timeout_ms(50);

    let result = runtime.eval("outer: while (true) {}");
    assert!(matches!(result, Err(JsError::Timeout { .. })));
}

#[test]
fn test_timeout_nested_loops() {
    let mut runtime = Runtime::new();
    runtime.set_timeout_ms(50);

    let result = runtime.eval("while (true) { for (;;) {} }");
    assert!(matches!(result, Err(JsError::Timeout { .. })));
}
