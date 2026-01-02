//! Tests for step-based execution and host-controlled limits
//!
//! The host controls execution using the step() API instead of runtime-enforced
//! timeouts or depth limits. This allows the host to:
//! - Execute one bytecode instruction at a time
//! - Query call depth with call_depth()
//! - Enforce time limits, step limits, or depth limits as needed

use tsrun::{Interpreter, StepResult};

#[test]
fn test_step_basic_execution() {
    let mut interp = Interpreter::new();

    // Prepare execution
    let result = interp.prepare("1 + 2 + 3", None);
    assert!(matches!(result, Ok(StepResult::Continue)));

    // Step through until complete
    for _ in 0..1000 {
        match interp.step().unwrap() {
            StepResult::Continue => continue,
            StepResult::Complete(value) => {
                assert_eq!(value.as_number(), Some(6.0));
                return;
            }
            other => panic!("Unexpected result: {:?}", other),
        }
    }
    panic!("Too many steps");
}

#[test]
fn test_step_returns_done_when_no_active_vm() {
    let mut interp = Interpreter::new();

    // Without prepare(), step() should return Done
    let result = interp.step().unwrap();
    assert!(matches!(result, StepResult::Done));
}

#[test]
fn test_step_can_stop_infinite_loop() {
    let mut interp = Interpreter::new();

    // Prepare an infinite loop
    let result = interp.prepare("while (true) {}", None);
    assert!(matches!(result, Ok(StepResult::Continue)));

    // Step for a limited number of iterations
    let max_steps = 100;
    let mut steps = 0;
    for _ in 0..max_steps {
        let result = interp.step().unwrap();
        steps += 1;
        match result {
            StepResult::Continue => continue,
            StepResult::Complete(_) => panic!("Infinite loop should not complete"),
            _ => break,
        }
    }

    // Should have stopped due to reaching max_steps
    assert_eq!(steps, max_steps);
}

#[test]
fn test_step_function_calls() {
    let mut interp = Interpreter::new();

    // Prepare code with function calls
    let result = interp.prepare(
        r#"
        function add(a, b) { return a + b; }
        add(1, 2)
        "#,
        None,
    );
    assert!(matches!(result, Ok(StepResult::Continue)));

    // Step through until complete
    for _ in 0..1000 {
        match interp.step().unwrap() {
            StepResult::Continue => continue,
            StepResult::Complete(value) => {
                assert_eq!(value.as_number(), Some(3.0));
                return;
            }
            other => panic!("Unexpected result: {:?}", other),
        }
    }
    panic!("Too many steps");
}

#[test]
fn test_step_loop_with_counter() {
    let mut interp = Interpreter::new();

    // Prepare a loop that terminates
    let result = interp.prepare(
        r#"
        let sum = 0;
        for (let i = 0; i < 10; i++) {
            sum += i;
        }
        sum
        "#,
        None,
    );
    assert!(matches!(result, Ok(StepResult::Continue)));

    // Step through until complete
    for _ in 0..10000 {
        match interp.step().unwrap() {
            StepResult::Continue => continue,
            StepResult::Complete(value) => {
                // sum = 0 + 1 + 2 + ... + 9 = 45
                assert_eq!(value.as_number(), Some(45.0));
                return;
            }
            other => panic!("Unexpected result: {:?}", other),
        }
    }
    panic!("Too many steps");
}

#[test]
fn test_run_to_completion() {
    // run() helper should execute to completion in a single call
    use super::{create_test_runtime, run};
    let mut interp = create_test_runtime();
    let result = run(&mut interp, "1 + 2 * 3", None);
    assert!(result.is_ok());
    if let Ok(StepResult::Complete(value)) = result {
        assert_eq!(value.as_number(), Some(7.0));
    }
}

#[test]
fn test_step_recursion() {
    let mut interp = Interpreter::new();

    // Prepare recursive function
    let result = interp.prepare(
        r#"
        function fib(n) {
            if (n <= 1) return n;
            return fib(n - 1) + fib(n - 2);
        }
        fib(10)
        "#,
        None,
    );
    assert!(matches!(result, Ok(StepResult::Continue)));

    // Step through until complete
    for _ in 0..100000 {
        match interp.step().unwrap() {
            StepResult::Continue => continue,
            StepResult::Complete(value) => {
                assert_eq!(value.as_number(), Some(55.0)); // fib(10) = 55
                return;
            }
            other => panic!("Unexpected result: {:?}", other),
        }
    }
    panic!("Too many steps");
}

// =============================================================================
// call_depth() tests
// =============================================================================

#[test]
fn test_call_depth_starts_at_zero() {
    let interp = Interpreter::new();
    assert_eq!(interp.call_depth(), 0);
}

#[test]
fn test_call_depth_after_prepare() {
    let mut interp = Interpreter::new();
    interp.prepare("1 + 2", None).unwrap();
    // After prepare, there's an active VM but no function calls yet
    // The depth should be 0 (no trampoline stack entries)
    assert_eq!(interp.call_depth(), 0);
}

#[test]
fn test_call_depth_increases_during_function_call() {
    let mut interp = Interpreter::new();
    interp
        .prepare(
            r#"
        function outer() {
            return inner();
        }
        function inner() {
            return 42;
        }
        outer()
        "#,
            None,
        )
        .unwrap();

    let mut max_depth = 0;
    loop {
        let depth = interp.call_depth();
        if depth > max_depth {
            max_depth = depth;
        }
        match interp.step().unwrap() {
            StepResult::Continue => continue,
            StepResult::Complete(_) => break,
            _ => panic!("Unexpected result"),
        }
    }

    // Should have reached at least depth 2 (outer -> inner)
    assert!(max_depth >= 2, "max_depth was {}", max_depth);
}

#[test]
fn test_call_depth_with_recursion() {
    let mut interp = Interpreter::new();
    interp
        .prepare(
            r#"
        function recurse(n) {
            if (n <= 0) return 0;
            return recurse(n - 1);
        }
        recurse(5)
        "#,
            None,
        )
        .unwrap();

    let mut max_depth = 0;
    loop {
        let depth = interp.call_depth();
        if depth > max_depth {
            max_depth = depth;
        }
        match interp.step().unwrap() {
            StepResult::Continue => continue,
            StepResult::Complete(_) => break,
            _ => panic!("Unexpected result"),
        }
    }

    // Should reach depth of at least 5 (recurse called 6 times: n=5,4,3,2,1,0)
    assert!(max_depth >= 5, "max_depth was {}", max_depth);
}

// =============================================================================
// Host-controlled limit tests (patterns for tsrun binary)
// =============================================================================

#[test]
fn test_host_can_enforce_depth_limit() {
    let mut interp = Interpreter::new();
    interp
        .prepare(
            r#"
        function recurse(n) {
            return recurse(n + 1);
        }
        recurse(0)
        "#,
            None,
        )
        .unwrap();

    let max_allowed_depth = 10;
    let mut stopped_due_to_depth = false;

    for _ in 0..10000 {
        let depth = interp.call_depth();
        if depth > max_allowed_depth {
            stopped_due_to_depth = true;
            break;
        }
        match interp.step().unwrap() {
            StepResult::Continue => continue,
            StepResult::Complete(_) => break,
            _ => break,
        }
    }

    assert!(
        stopped_due_to_depth,
        "Should have stopped due to depth limit"
    );
}

#[test]
fn test_host_can_enforce_step_limit() {
    let mut interp = Interpreter::new();
    interp.prepare("while (true) {}", None).unwrap();

    let max_steps = 50;
    let mut steps_executed = 0;

    for _ in 0..max_steps {
        steps_executed += 1;
        match interp.step().unwrap() {
            StepResult::Continue => continue,
            StepResult::Complete(_) => panic!("Infinite loop should not complete"),
            _ => break,
        }
    }

    assert_eq!(steps_executed, max_steps);
}

#[test]
fn test_host_can_enforce_time_limit() {
    use std::time::{Duration, Instant};

    let mut interp = Interpreter::new();
    interp.prepare("while (true) {}", None).unwrap();

    let start = Instant::now();
    let timeout = Duration::from_millis(50);
    let mut timed_out = false;

    for _ in 0..1_000_000 {
        if start.elapsed() >= timeout {
            timed_out = true;
            break;
        }
        match interp.step().unwrap() {
            StepResult::Continue => continue,
            StepResult::Complete(_) => panic!("Infinite loop should not complete"),
            _ => break,
        }
    }

    assert!(timed_out, "Should have timed out");
}

#[test]
fn test_depth_limit_allows_normal_execution() {
    let mut interp = Interpreter::new();
    interp
        .prepare(
            r#"
        function a() { return b(); }
        function b() { return c(); }
        function c() { return 42; }
        a()
        "#,
            None,
        )
        .unwrap();

    let max_allowed_depth = 10; // More than enough for a -> b -> c
    let mut result_value = None;

    for _ in 0..10000 {
        let depth = interp.call_depth();
        if depth > max_allowed_depth {
            panic!("Depth limit exceeded unexpectedly");
        }
        match interp.step().unwrap() {
            StepResult::Continue => continue,
            StepResult::Complete(value) => {
                result_value = Some(value);
                break;
            }
            _ => break,
        }
    }

    assert_eq!(result_value.unwrap().as_number(), Some(42.0));
}

#[test]
fn test_call_depth_returns_to_zero_after_completion() {
    let mut interp = Interpreter::new();
    interp
        .prepare(
            r#"
        function foo() { return 1; }
        foo()
        "#,
            None,
        )
        .unwrap();

    // Run to completion
    loop {
        match interp.step().unwrap() {
            StepResult::Continue => continue,
            StepResult::Complete(_) => break,
            _ => break,
        }
    }

    // After completion, depth should be 0
    assert_eq!(interp.call_depth(), 0);
}

#[test]
fn test_call_depth_with_nested_user_functions() {
    let mut interp = Interpreter::new();
    interp
        .prepare(
            r#"
        function level1() {
            return level2();
        }
        function level2() {
            return level3();
        }
        function level3() {
            return level4();
        }
        function level4() {
            return 42;
        }
        level1()
        "#,
            None,
        )
        .unwrap();

    let mut max_depth = 0;
    loop {
        let depth = interp.call_depth();
        if depth > max_depth {
            max_depth = depth;
        }
        match interp.step().unwrap() {
            StepResult::Continue => continue,
            StepResult::Complete(value) => {
                assert_eq!(value.as_number(), Some(42.0));
                break;
            }
            _ => panic!("Unexpected result"),
        }
    }

    // Should have reached depth 4 (level1 -> level2 -> level3 -> level4)
    assert!(max_depth >= 4, "max_depth was {}", max_depth);
}
