// Tests for state machine execution model
#![allow(unused_imports)]
use typescript_eval::{JsValue, PendingSlot, Runtime, RuntimeResult};

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

// ═══════════════════════════════════════════════════════════════════════════
// Import tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_awaited_simple() {
    // An import statement should suspend and return ImportAwaited
    let mut runtime = Runtime::new();
    let result = runtime
        .eval_resumable(
            r#"
        import { foo } from './module';
        foo
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./module");

            // Create a module object with 'foo' export
            let module = runtime.create_module_object(vec![("foo".to_string(), JsValue::Number(42.0))]);
            slot.set_success(module);
        }
        RuntimeResult::Complete(_) => panic!("Expected ImportAwaited, got Complete"),
        RuntimeResult::AsyncAwaited { .. } => panic!("Expected ImportAwaited, got AsyncAwaited"),
    }

    // Continue execution
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(42.0));
        }
        _ => panic!("Expected Complete after import resolution"),
    }
}

#[test]
fn test_import_awaited_multiple() {
    // Multiple imports should be processed in order
    let mut runtime = Runtime::new();
    let result = runtime
        .eval_resumable(
            r#"
        import { a } from './mod_a';
        import { b } from './mod_b';
        a + b
    "#,
        )
        .unwrap();

    // First import
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./mod_a");
            let module = runtime.create_module_object(vec![("a".to_string(), JsValue::Number(10.0))]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for first import"),
    }

    // Continue - should get second import
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./mod_b");
            let module = runtime.create_module_object(vec![("b".to_string(), JsValue::Number(20.0))]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for second import"),
    }

    // Continue - should complete
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(30.0));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_import_error_propagation() {
    // If host sets error on slot, it should throw at import point
    let mut runtime = Runtime::new();
    let result = runtime
        .eval_resumable(
            r#"
        import { foo } from './nonexistent';
        foo
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, .. } => {
            // Simulate module not found error
            slot.set_error(typescript_eval::JsError::type_error("Module not found: ./nonexistent"));
        }
        _ => panic!("Expected ImportAwaited"),
    }

    // Continue should return error
    let result = runtime.continue_eval();
    assert!(result.is_err());
}
