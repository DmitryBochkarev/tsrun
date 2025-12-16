//! Tests for the stack-based evaluation

use typescript_eval::ast::Statement;
use typescript_eval::interpreter::stack::{ExecutionState, Frame, StepResult};
use typescript_eval::parser::Parser;
use typescript_eval::{Interpreter, JsValue, StringDict};

use std::rc::Rc;

/// Helper to evaluate using stack-based execution
fn stack_eval(source: &str) -> JsValue {
    let mut interp = Interpreter::new();
    let mut string_dict = StringDict::new();

    // Parse the source to get an expression
    let program = Parser::new(source, &mut string_dict)
        .parse_program()
        .expect("parse failed");

    // For simple expression statements, extract the expression
    let expr = match program.body.first() {
        Some(Statement::Expression(es)) => Rc::clone(&es.expression),
        _ => panic!("Expected expression statement"),
    };

    // Set up execution state
    let mut state = ExecutionState::new();
    state.push_frame(Frame::Expr(expr));

    // Run to completion
    match interp.run(&mut state) {
        StepResult::Done(g) => g.value,
        StepResult::Error(e) => panic!("Stack eval error: {:?}", e),
        StepResult::Suspend(_) => panic!("Unexpected suspension"),
        StepResult::Continue => panic!("Unexpected continue"),
    }
}

#[test]
fn test_stack_literal_number() {
    assert_eq!(stack_eval("42"), JsValue::Number(42.0));
}

#[test]
fn test_stack_literal_string() {
    let result = stack_eval("\"hello\"");
    if let JsValue::String(s) = result {
        assert_eq!(s.as_str(), "hello");
    } else {
        panic!("Expected string");
    }
}

#[test]
fn test_stack_literal_boolean() {
    assert_eq!(stack_eval("true"), JsValue::Boolean(true));
    assert_eq!(stack_eval("false"), JsValue::Boolean(false));
}

#[test]
fn test_stack_binary_add() {
    assert_eq!(stack_eval("1 + 2"), JsValue::Number(3.0));
}

#[test]
fn test_stack_binary_sub() {
    assert_eq!(stack_eval("5 - 3"), JsValue::Number(2.0));
}

#[test]
fn test_stack_binary_mul() {
    assert_eq!(stack_eval("3 * 4"), JsValue::Number(12.0));
}

#[test]
fn test_stack_binary_div() {
    assert_eq!(stack_eval("10 / 2"), JsValue::Number(5.0));
}

#[test]
fn test_stack_binary_comparison() {
    assert_eq!(stack_eval("1 < 2"), JsValue::Boolean(true));
    assert_eq!(stack_eval("2 > 1"), JsValue::Boolean(true));
    assert_eq!(stack_eval("1 === 1"), JsValue::Boolean(true));
    assert_eq!(stack_eval("1 !== 2"), JsValue::Boolean(true));
}

#[test]
fn test_stack_unary_not() {
    assert_eq!(stack_eval("!true"), JsValue::Boolean(false));
    assert_eq!(stack_eval("!false"), JsValue::Boolean(true));
}

#[test]
fn test_stack_unary_minus() {
    assert_eq!(stack_eval("-5"), JsValue::Number(-5.0));
}

#[test]
fn test_stack_unary_typeof() {
    let result = stack_eval("typeof 42");
    if let JsValue::String(s) = result {
        assert_eq!(s.as_str(), "number");
    } else {
        panic!("Expected string");
    }
}

#[test]
fn test_stack_logical_and_short_circuit() {
    // false && anything should return false
    assert_eq!(stack_eval("false && true"), JsValue::Boolean(false));
}

#[test]
fn test_stack_logical_and_continue() {
    // true && x should return x
    assert_eq!(stack_eval("true && 42"), JsValue::Number(42.0));
}

#[test]
fn test_stack_logical_or_short_circuit() {
    // true || anything should return true
    assert_eq!(stack_eval("true || false"), JsValue::Boolean(true));
}

#[test]
fn test_stack_logical_or_continue() {
    // false || x should return x
    assert_eq!(stack_eval("false || 42"), JsValue::Number(42.0));
}

#[test]
fn test_stack_nullish_coalescing() {
    assert_eq!(stack_eval("null ?? 42"), JsValue::Number(42.0));
    assert_eq!(stack_eval("undefined ?? 42"), JsValue::Number(42.0));
    assert_eq!(stack_eval("0 ?? 42"), JsValue::Number(0.0));
}

#[test]
fn test_stack_conditional_true() {
    assert_eq!(stack_eval("true ? 1 : 2"), JsValue::Number(1.0));
}

#[test]
fn test_stack_conditional_false() {
    assert_eq!(stack_eval("false ? 1 : 2"), JsValue::Number(2.0));
}

#[test]
fn test_stack_nested_binary() {
    assert_eq!(stack_eval("1 + 2 + 3"), JsValue::Number(6.0));
    assert_eq!(stack_eval("2 * 3 + 4"), JsValue::Number(10.0));
}

#[test]
fn test_stack_string_concat() {
    let result = stack_eval("\"hello\" + \" \" + \"world\"");
    if let JsValue::String(s) = result {
        assert_eq!(s.as_str(), "hello world");
    } else {
        panic!("Expected string");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Await Suspension Tests
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper to create a pending promise and test await suspension
#[test]
fn test_stack_await_resolved_promise() {
    let mut interp = Interpreter::new();
    let mut string_dict = StringDict::new();

    // Create a resolved promise using Promise.resolve(42)
    let source = "Promise.resolve(42)";
    let program = Parser::new(source, &mut string_dict)
        .parse_program()
        .expect("parse failed");

    let expr = match program.body.first() {
        Some(Statement::Expression(es)) => Rc::clone(&es.expression),
        _ => panic!("Expected expression statement"),
    };

    // First, evaluate to get the promise
    let mut state = ExecutionState::new();
    state.push_frame(Frame::Expr(expr));

    let promise_result = match interp.run(&mut state) {
        StepResult::Done(g) => g.value,
        StepResult::Error(e) => panic!("Error: {:?}", e),
        _ => panic!("Unexpected result"),
    };

    // Now await the promise
    let mut state2 = ExecutionState::new();
    state2.push_value(typescript_eval::Guarded::unguarded(promise_result));
    state2.push_frame(Frame::AwaitCheck);

    // Run - should return the resolved value (42)
    match interp.run(&mut state2) {
        StepResult::Done(g) => {
            assert_eq!(g.value, JsValue::Number(42.0));
        }
        StepResult::Error(e) => panic!("Error: {:?}", e),
        StepResult::Suspend(_) => panic!("Should not suspend for resolved promise"),
        StepResult::Continue => panic!("Unexpected continue"),
    }
}

#[test]
fn test_stack_await_pending_promise_suspends() {
    use typescript_eval::interpreter::builtins::promise_new::create_promise_with_guard;

    let mut interp = Interpreter::new();

    // Create a pending promise directly
    let (promise, _guard) = create_promise_with_guard(&mut interp);

    // Set up state to await the pending promise
    let mut state = ExecutionState::new();
    state.push_value(typescript_eval::Guarded::unguarded(JsValue::Object(
        promise.clone(),
    )));
    state.push_frame(Frame::AwaitCheck);

    // Run - should suspend
    match interp.run(&mut state) {
        StepResult::Suspend(suspended_promise) => {
            // Verify we get back the same promise
            assert_eq!(suspended_promise.id(), promise.id());
        }
        StepResult::Done(g) => panic!("Should suspend, got: {:?}", g.value),
        StepResult::Error(e) => panic!("Error: {:?}", e),
        StepResult::Continue => panic!("Unexpected continue"),
    }
}

#[test]
fn test_stack_await_resume_with_value() {
    use typescript_eval::interpreter::builtins::promise_new::create_promise_with_guard;

    let mut interp = Interpreter::new();

    // Create a pending promise directly
    let (promise, _guard) = create_promise_with_guard(&mut interp);

    // Set up state to await the pending promise, then add more operations
    // to verify execution continues after resume
    let mut state = ExecutionState::new();

    // Push "add 10" operation to be done after the await completes
    // (This is what would happen in: `await promise + 10`)
    state.push_frame(Frame::BinaryComplete {
        op: typescript_eval::ast::BinaryOp::Add,
    });

    // Push the second operand (10)
    state.push_value(typescript_eval::Guarded::unguarded(JsValue::Number(10.0)));

    // Now push the await - this will be processed first
    state.push_value(typescript_eval::Guarded::unguarded(JsValue::Object(
        promise.clone(),
    )));
    state.push_frame(Frame::AwaitCheck);

    // Run - should suspend at await
    let result = interp.run(&mut state);
    assert!(matches!(result, StepResult::Suspend(_)), "Should suspend");

    // Resume with value 32 (simulating promise fulfilled with 32)
    let final_result = interp.resume_with_value(&mut state, JsValue::Number(32.0));

    // Should complete with 32 + 10 = 42
    match final_result {
        StepResult::Done(g) => {
            assert_eq!(g.value, JsValue::Number(42.0));
        }
        StepResult::Error(e) => panic!("Error: {:?}", e),
        StepResult::Suspend(_) => panic!("Should not suspend again"),
        StepResult::Continue => panic!("Unexpected continue"),
    }
}

#[test]
fn test_stack_await_resume_with_error() {
    use typescript_eval::interpreter::builtins::promise_new::create_promise_with_guard;
    use typescript_eval::JsError;

    let mut interp = Interpreter::new();

    // Create a pending promise directly
    let (promise, _guard) = create_promise_with_guard(&mut interp);

    // Set up state to await the pending promise
    let mut state = ExecutionState::new();
    state.push_value(typescript_eval::Guarded::unguarded(JsValue::Object(
        promise.clone(),
    )));
    state.push_frame(Frame::AwaitCheck);

    // Run - should suspend at await
    let result = interp.run(&mut state);
    assert!(matches!(result, StepResult::Suspend(_)), "Should suspend");

    // Resume with error (simulating promise rejection)
    let error = JsError::thrown(JsValue::String("rejected!".into()));
    let final_result = interp.resume_with_error(&mut state, error);

    // Should return the error
    match final_result {
        StepResult::Error(e) => {
            // Verify it's the thrown error
            if let JsError::ThrownValue { value } = e {
                if let JsValue::String(s) = value {
                    assert_eq!(s.as_str(), "rejected!");
                } else {
                    panic!("Expected string error");
                }
            } else {
                panic!("Expected ThrownValue error");
            }
        }
        StepResult::Done(g) => panic!("Should error, got: {:?}", g.value),
        StepResult::Suspend(_) => panic!("Should not suspend again"),
        StepResult::Continue => panic!("Unexpected continue"),
    }
}
