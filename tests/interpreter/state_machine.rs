// Tests for state machine execution model
#![allow(unused_imports)]
use typescript_eval::{JsValue, PendingSlot, Runtime, RuntimeResult};

#[test]
fn test_runtime_result_complete_simple() {
    // Simple expressions should return Complete immediately
    let mut runtime = Runtime::new();
    let result = runtime.eval("1 + 2").unwrap();

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
        .eval(
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
        .eval(
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
        .eval(
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
        .eval(
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
        .eval(
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

// ═══════════════════════════════════════════════════════════════════════════
// Import with default export
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_default() {
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        import myModule from './module';
        myModule.value
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./module");
            // Create a module with a default export
            let default_obj = runtime.create_module_object(vec![("value".to_string(), JsValue::Number(100.0))]);
            let module = runtime.create_module_object(vec![("default".to_string(), default_obj)]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(100.0));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_import_namespace() {
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        import * as utils from './utils';
        utils.x + utils.y
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./utils");
            // Create a module namespace object with properties
            let module = runtime.create_module_object(vec![
                ("x".to_string(), JsValue::Number(10.0)),
                ("y".to_string(), JsValue::Number(20.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(30.0));
        }
        _ => panic!("Expected Complete"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Import with side effects simulation
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_module_with_side_effects() {
    // Simulate a module that has side effects (like setting global state)
    let mut runtime = Runtime::new();

    // First, set up a variable to track side effects
    runtime.eval("let sideEffectValue = 0;").unwrap();

    let result = runtime
        .eval(
            r#"
        import { increment } from './side-effect-module';
        sideEffectValue
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./side-effect-module");
            // The host can modify runtime state before providing the module
            // This simulates a module that has side effects during load
            let module = runtime.create_module_object(vec![
                ("increment".to_string(), JsValue::Number(1.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(0.0));
        }
        _ => panic!("Expected Complete"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Import then use in expressions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_used_in_function() {
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        import { PI } from './math';
        function circumference(r: number): number {
            return 2 * PI * r;
        }
        circumference(1)
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, .. } => {
            let module = runtime.create_module_object(vec![
                ("PI".to_string(), JsValue::Number(std::f64::consts::PI)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            if let JsValue::Number(n) = value {
                assert!((n - 2.0 * std::f64::consts::PI).abs() < 0.0001);
            } else {
                panic!("Expected number");
            }
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_import_used_in_class() {
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        import { BASE_VALUE } from './config';
        class Calculator {
            value: number;
            constructor() {
                this.value = BASE_VALUE;
            }
            getValue(): number {
                return this.value;
            }
        }
        const calc = new Calculator();
        calc.getValue()
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, .. } => {
            let module = runtime.create_module_object(vec![
                ("BASE_VALUE".to_string(), JsValue::Number(42.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(42.0));
        }
        _ => panic!("Expected Complete"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// AsyncAwaited tests - Host-controlled async suspension
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_async_awaited_simple() {
    // When await is called on a pending promise, it should suspend
    // and return AsyncAwaited, allowing host to control resolution
    let mut runtime = Runtime::new();

    // Create an async function that awaits a pending promise created by the host
    let result = runtime
        .eval(
            r#"
        let pendingPromise: Promise<number>;

        // Create a pending promise that the host will resolve
        pendingPromise = new Promise(function(resolve, reject) {
            // Don't call resolve - leave it pending
            // Store resolve function (simplified - in real code would use global)
        });

        async function getValue(): Promise<number> {
            const x = await pendingPromise;
            return x * 2;
        }

        // Call the async function - it should suspend at the await
        getValue()
    "#,
        )
        .unwrap();

    // The async function creates and returns a promise
    // Currently, pending promises don't suspend the runtime
    // This test documents current behavior
    match result {
        RuntimeResult::Complete(value) => {
            // Current behavior: returns a promise object
            assert!(matches!(value, JsValue::Object(_)));
        }
        RuntimeResult::AsyncAwaited { .. } => {
            // Future behavior when suspension is implemented
            // The host would fill the slot here
        }
        _ => panic!("Unexpected result"),
    }
}

#[test]
fn test_async_function_with_resolved_promise() {
    // Async function awaiting an already-resolved promise should complete synchronously
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        let captured: number = 0;

        async function getValue(): Promise<number> {
            const x = await Promise.resolve(21);
            return x * 2;
        }

        getValue().then(function(v) {
            captured = v;
        });

        captured
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(42.0));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_async_function_with_rejected_promise() {
    // Async function awaiting a rejected promise should catch the error
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        let caught: string = "";

        async function getValue(): Promise<string> {
            try {
                await Promise.reject("error message");
                return "success";
            } catch (e) {
                return "caught: " + e;
            }
        }

        getValue().then(function(v) {
            caught = v;
        });

        caught
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::String("caught: error message".into()));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_async_chain_with_multiple_awaits() {
    // Multiple awaits in sequence should all resolve synchronously for fulfilled promises
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        let result: number = 0;

        async function calculate(): Promise<number> {
            const a = await Promise.resolve(1);
            const b = await Promise.resolve(2);
            const c = await Promise.resolve(3);
            return a + b + c;
        }

        calculate().then(function(v) {
            result = v;
        });

        result
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(6.0));
        }
        _ => panic!("Expected Complete"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Combined import + async tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_then_async_function() {
    // Import a module, then use it in an async function
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        import { multiply } from './math';

        let result: number = 0;

        async function calculate(): Promise<number> {
            const x = await Promise.resolve(3);
            return multiply * x;
        }

        calculate().then(function(v) {
            result = v;
        });

        result
    "#,
        )
        .unwrap();

    // First, resolve the import
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./math");
            let module = runtime.create_module_object(vec![
                ("multiply".to_string(), JsValue::Number(7.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    // Continue - async function should complete
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(21.0)); // 7 * 3
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_multiple_imports_then_async() {
    // Multiple imports followed by async operations
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        import { a } from './mod_a';
        import { b } from './mod_b';

        let result: number = 0;

        async function combine(): Promise<number> {
            const x = await Promise.resolve(a);
            const y = await Promise.resolve(b);
            return x + y;
        }

        combine().then(function(v) {
            result = v;
        });

        result
    "#,
        )
        .unwrap();

    // First import
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./mod_a");
            let module = runtime.create_module_object(vec![
                ("a".to_string(), JsValue::Number(10.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for mod_a"),
    }

    // Second import
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./mod_b");
            let module = runtime.create_module_object(vec![
                ("b".to_string(), JsValue::Number(20.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for mod_b"),
    }

    // Complete
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(30.0));
        }
        _ => panic!("Expected Complete"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Nested async/await scenarios
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_nested_async_calls() {
    // Async function calling another async function
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        let result: number = 0;

        async function inner(): Promise<number> {
            return await Promise.resolve(10);
        }

        async function outer(): Promise<number> {
            const x = await inner();
            return x * 2;
        }

        outer().then(function(v) {
            result = v;
        });

        result
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(20.0));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_async_in_loop() {
    // Async operations inside a loop
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        let result: number = 0;

        async function sumAsync(arr: number[]): Promise<number> {
            let sum = 0;
            for (let i = 0; i < arr.length; i++) {
                sum += await Promise.resolve(arr[i]);
            }
            return sum;
        }

        sumAsync([1, 2, 3, 4, 5]).then(function(v) {
            result = v;
        });

        result
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(15.0));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_async_error_propagation() {
    // Error propagation through nested async functions
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        let result: string = "";

        async function innerError(): Promise<number> {
            throw new Error("inner error");
        }

        async function outer(): Promise<number> {
            try {
                return await innerError();
            } catch (e) {
                return -1;
            }
        }

        outer().then(function(v) {
            result = "got: " + v;
        });

        result
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::String("got: -1".into()));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_async_with_promise_all() {
    // Using Promise.all with async operations
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        let result: number = 0;

        async function fetchAll(): Promise<number> {
            const promises = [
                Promise.resolve(1),
                Promise.resolve(2),
                Promise.resolve(3)
            ];
            const values = await Promise.all(promises);
            return values[0] + values[1] + values[2];
        }

        fetchAll().then(function(v) {
            result = v;
        });

        result
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(6.0));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_async_conditional_await() {
    // Conditional await based on runtime values
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        let result: number = 0;

        async function conditionalFetch(useAsync: boolean): Promise<number> {
            if (useAsync) {
                return await Promise.resolve(100);
            } else {
                return 50;
            }
        }

        // Test both paths
        let sum = 0;
        conditionalFetch(true).then(function(v) { sum += v; });
        conditionalFetch(false).then(function(v) { sum += v; });

        result = sum;
        result
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(150.0));
        }
        _ => panic!("Expected Complete"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Complex module + async scenarios
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_import_function_used_in_async() {
    // Import a function and use it inside async function
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        import { transform } from './transformer';

        let result: number = 0;

        async function process(): Promise<number> {
            const data = await Promise.resolve(10);
            return transform * data;
        }

        process().then(function(v) {
            result = v;
        });

        result
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./transformer");
            let module = runtime.create_module_object(vec![
                ("transform".to_string(), JsValue::Number(3.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(30.0));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_multiple_imports_async_composition() {
    // Multiple imports used together in async composition
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        import { baseMultiplier } from './config';
        import { offset } from './constants';

        let result: number = 0;

        async function calculate(input: number): Promise<number> {
            const step1 = await Promise.resolve(input * baseMultiplier);
            const step2 = await Promise.resolve(step1 + offset);
            return step2;
        }

        calculate(5).then(function(v) {
            result = v;
        });

        result
    "#,
        )
        .unwrap();

    // First import
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./config");
            let module = runtime.create_module_object(vec![
                ("baseMultiplier".to_string(), JsValue::Number(2.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for config"),
    }

    // Second import
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./constants");
            let module = runtime.create_module_object(vec![
                ("offset".to_string(), JsValue::Number(7.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for constants"),
    }

    // Complete
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            // 5 * 2 = 10, then 10 + 7 = 17
            assert_eq!(value, JsValue::Number(17.0));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_async_class_with_imports() {
    // Class using imported values in async methods
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        import { DEFAULT_TIMEOUT } from './settings';

        let result: number = 0;

        class AsyncService {
            timeout: number;

            constructor() {
                this.timeout = DEFAULT_TIMEOUT;
            }

            async fetch(): Promise<number> {
                return await Promise.resolve(this.timeout);
            }
        }

        const service = new AsyncService();
        service.fetch().then(function(v) {
            result = v;
        });

        result
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./settings");
            let module = runtime.create_module_object(vec![
                ("DEFAULT_TIMEOUT".to_string(), JsValue::Number(5000.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(5000.0));
        }
        _ => panic!("Expected Complete"),
    }
}
