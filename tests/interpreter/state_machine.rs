// Tests for state machine execution model
#![allow(unused_imports)]
use typescript_eval::{JsValue, Runtime, RuntimeResult};

#[test]
fn test_runtime_result_complete_simple() {
    // Simple expressions should return Complete immediately
    let mut runtime = Runtime::new();
    let result = runtime.eval_resumable("1 + 2").unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(3.0));
        }
        _ => panic!("Expected Complete, got {:?}", result),
    }
}

#[test]
fn test_runtime_result_complete_with_variables() {
    let mut runtime = Runtime::new();
    let result = runtime
        .eval_resumable(
            r#"
        const x: number = 10;
        const y: number = 20;
        x + y
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(30.0));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_runtime_result_complete_with_function() {
    let mut runtime = Runtime::new();
    let result = runtime
        .eval_resumable(
            r#"
        function add(a: number, b: number): number {
            return a + b;
        }
        add(5, 7)
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(12.0));
        }
        _ => panic!("Expected Complete"),
    }
}
