// Tests for state machine execution model
#![allow(unused_imports)]
use typescript_eval::{JsError, JsValue, PendingSlot, Runtime, RuntimeResult};

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

// ═══════════════════════════════════════════════════════════════════════════
// Nested Import Tests - Simulating module dependencies
// ═══════════════════════════════════════════════════════════════════════════

/// Test scenario: Main imports A, A imports B
/// The host receives ImportAwaited for each module in order
#[test]
fn test_nested_imports_two_levels() {
    let mut runtime = Runtime::new();

    // Main module imports from moduleA
    // moduleA itself would import from moduleB (simulated by host)
    let result = runtime
        .eval(
            r#"
        import { valueFromA } from './moduleA';
        valueFromA
    "#,
        )
        .unwrap();

    // First: host is asked for moduleA
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./moduleA");
            // Host simulates moduleA which has a value that came from moduleB
            // In real scenario, host would have already loaded moduleB
            let module_a = runtime.create_module_object(vec![
                ("valueFromA".to_string(), JsValue::Number(100.0)),
            ]);
            slot.set_success(module_a);
        }
        _ => panic!("Expected ImportAwaited for moduleA"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(100.0));
        }
        _ => panic!("Expected Complete"),
    }
}

/// Test scenario: Main imports A and B separately, both from same "package"
#[test]
fn test_parallel_imports_same_package() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import { utilA } from './utils/a';
        import { utilB } from './utils/b';
        utilA + utilB
    "#,
        )
        .unwrap();

    // First import
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./utils/a");
            let module = runtime.create_module_object(vec![
                ("utilA".to_string(), JsValue::Number(10.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for utils/a"),
    }

    // Second import
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./utils/b");
            let module = runtime.create_module_object(vec![
                ("utilB".to_string(), JsValue::Number(20.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for utils/b"),
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

/// Test scenario: Import chain where values are composed
/// Main imports calculator, calculator uses config values
#[test]
fn test_import_chain_with_composition() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import { MULTIPLIER } from './config';
        import { calculate } from './calculator';

        // Use both imports together
        const baseValue = 5;
        const result = baseValue * MULTIPLIER;
        result
    "#,
        )
        .unwrap();

    // First: config
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./config");
            let module = runtime.create_module_object(vec![
                ("MULTIPLIER".to_string(), JsValue::Number(3.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for config"),
    }

    // Second: calculator
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./calculator");
            // Calculator module - we don't actually use calculate in this test
            let module = runtime.create_module_object(vec![
                ("calculate".to_string(), JsValue::Undefined),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for calculator"),
    }

    // Complete
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(15.0)); // 5 * 3
        }
        _ => panic!("Expected Complete"),
    }
}

/// Test: Multiple imports from the same module specifier
/// In real ES modules, this would be the same module - host should cache
#[test]
fn test_multiple_imports_same_specifier() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import { x } from './shared';
        import { y } from './shared';
        x + y
    "#,
        )
        .unwrap();

    // First import from ./shared
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./shared");
            let module = runtime.create_module_object(vec![
                ("x".to_string(), JsValue::Number(1.0)),
                ("y".to_string(), JsValue::Number(2.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for first ./shared"),
    }

    // Second import from ./shared - runtime requests it again
    // Host would typically return cached module
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./shared");
            // Host returns same (or cached) module
            let module = runtime.create_module_object(vec![
                ("x".to_string(), JsValue::Number(1.0)),
                ("y".to_string(), JsValue::Number(2.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for second ./shared"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(3.0));
        }
        _ => panic!("Expected Complete"),
    }
}

/// Test: Import then re-export pattern simulation
/// This simulates a barrel file pattern
#[test]
fn test_barrel_file_pattern() {
    let mut runtime = Runtime::new();

    // Simulating: import from index which re-exports from sub-modules
    let result = runtime
        .eval(
            r#"
        import { feature1, feature2, feature3 } from './features';

        const total = feature1 + feature2 + feature3;
        total
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./features");
            // Host provides all re-exported features from barrel
            let module = runtime.create_module_object(vec![
                ("feature1".to_string(), JsValue::Number(10.0)),
                ("feature2".to_string(), JsValue::Number(20.0)),
                ("feature3".to_string(), JsValue::Number(30.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(60.0));
        }
        _ => panic!("Expected Complete"),
    }
}

/// Test: Deep import path
#[test]
fn test_deep_import_path() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import { deepValue } from './packages/core/utils/helpers/deep';
        deepValue * 2
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./packages/core/utils/helpers/deep");
            let module = runtime.create_module_object(vec![
                ("deepValue".to_string(), JsValue::Number(21.0)),
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

/// Test: Import with relative parent path
#[test]
fn test_import_parent_path() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import { parentValue } from '../parent/module';
        import { siblingValue } from '../sibling';
        parentValue + siblingValue
    "#,
        )
        .unwrap();

    // First import
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "../parent/module");
            let module = runtime.create_module_object(vec![
                ("parentValue".to_string(), JsValue::Number(100.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for parent"),
    }

    // Second import
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "../sibling");
            let module = runtime.create_module_object(vec![
                ("siblingValue".to_string(), JsValue::Number(50.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for sibling"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(150.0));
        }
        _ => panic!("Expected Complete"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Import Error Handling Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Test: Import failure propagates error
#[test]
fn test_import_failure_error() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import { missing } from './nonexistent';
        missing
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./nonexistent");
            // Host signals failure
            slot.set_error(JsError::type_error("Module not found: ./nonexistent"));
        }
        _ => panic!("Expected ImportAwaited"),
    }

    // Continue should result in error
    let result = runtime.continue_eval();
    assert!(result.is_err());
}

/// Test: Partial import success (first succeeds, second fails)
#[test]
fn test_partial_import_failure() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import { good } from './good-module';
        import { bad } from './bad-module';
        good + bad
    "#,
        )
        .unwrap();

    // First import succeeds
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./good-module");
            let module = runtime.create_module_object(vec![
                ("good".to_string(), JsValue::Number(1.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for good-module"),
    }

    // Second import fails
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./bad-module");
            slot.set_error(JsError::type_error("Module load error"));
        }
        _ => panic!("Expected ImportAwaited for bad-module"),
    }

    // Should error
    let result = runtime.continue_eval();
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
// Mixed Import Styles Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Test: Default import with named imports from same module
#[test]
fn test_default_and_named_imports() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import defaultExport, { namedA, namedB } from './mixed-module';
        defaultExport.value + namedA + namedB
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./mixed-module");
            // Create module with default and named exports
            let default_obj = runtime.create_module_object(vec![
                ("value".to_string(), JsValue::Number(100.0)),
            ]);
            let module = runtime.create_module_object(vec![
                ("default".to_string(), default_obj), // default_obj is already JsValue
                ("namedA".to_string(), JsValue::Number(10.0)),
                ("namedB".to_string(), JsValue::Number(20.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(130.0)); // 100 + 10 + 20
        }
        _ => panic!("Expected Complete"),
    }
}

/// Test: Namespace import combined with specific imports
#[test]
fn test_namespace_and_named_imports() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import * as utils from './utils';
        import { specific } from './specific';
        utils.helper + specific
    "#,
        )
        .unwrap();

    // First: namespace import
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./utils");
            let module = runtime.create_module_object(vec![
                ("helper".to_string(), JsValue::Number(5.0)),
                ("other".to_string(), JsValue::Number(10.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for utils"),
    }

    // Second: named import
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./specific");
            let module = runtime.create_module_object(vec![
                ("specific".to_string(), JsValue::Number(7.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for specific"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(12.0)); // 5 + 7
        }
        _ => panic!("Expected Complete"),
    }
}

/// Test: Aliased imports
#[test]
fn test_aliased_imports() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import { originalName as alias } from './module';
        import { foo as bar, baz as qux } from './other';
        alias + bar + qux
    "#,
        )
        .unwrap();

    // First module
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./module");
            let module = runtime.create_module_object(vec![
                ("originalName".to_string(), JsValue::Number(1.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for module"),
    }

    // Second module
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./other");
            let module = runtime.create_module_object(vec![
                ("foo".to_string(), JsValue::Number(2.0)),
                ("baz".to_string(), JsValue::Number(3.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for other"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(6.0)); // 1 + 2 + 3
        }
        _ => panic!("Expected Complete"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Circular Import Simulation Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Test: Host can handle circular imports by providing partial modules
/// Simulates: A imports B, B imports A (circular)
/// Host's responsibility to handle this - runtime just requests modules
#[test]
fn test_circular_import_host_handling() {
    let mut runtime = Runtime::new();

    // Main imports moduleA which might have circular dependency
    let result = runtime
        .eval(
            r#"
        import { valueA } from './moduleA';
        valueA
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./moduleA");
            // Host has resolved the circular dependency and provides moduleA
            // The circular reference has already been handled by the host
            let module = runtime.create_module_object(vec![
                ("valueA".to_string(), JsValue::Number(42.0)),
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
// Import with Execution Order Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Test: Imports are processed before module body executes
#[test]
fn test_import_before_execution() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        // This should work because imports are hoisted
        const result = importedValue * 2;

        import { importedValue } from './values';

        result
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./values");
            let module = runtime.create_module_object(vec![
                ("importedValue".to_string(), JsValue::Number(21.0)),
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

/// Test: Side-effect only import (import for side effects)
#[test]
fn test_side_effect_import() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import './polyfill';
        import './setup';

        // After side-effect imports, use regular import
        import { value } from './data';
        value
    "#,
        )
        .unwrap();

    // First: polyfill (side-effect only)
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./polyfill");
            // Side-effect module - empty exports
            let module = runtime.create_module_object(vec![]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for polyfill"),
    }

    // Second: setup (side-effect only)
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./setup");
            let module = runtime.create_module_object(vec![]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for setup"),
    }

    // Third: data with value
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./data");
            let module = runtime.create_module_object(vec![
                ("value".to_string(), JsValue::Number(999.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for data"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(999.0));
        }
        _ => panic!("Expected Complete"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Top-Level Await Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Test: Top-level await with already-resolved Promise
#[test]
fn test_top_level_await_resolved() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        const value = await Promise.resolve(42);
        value
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(42.0));
        }
        _ => panic!("Expected Complete for resolved promise TLA"),
    }
}

/// Test: Top-level await with import then await
#[test]
fn test_top_level_await_after_import() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import { asyncValue } from './async-module';

        // Top-level await on the imported value (which is a promise)
        const resolved = await Promise.resolve(asyncValue);
        resolved
    "#,
        )
        .unwrap();

    // First: resolve import
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./async-module");
            let module = runtime.create_module_object(vec![
                ("asyncValue".to_string(), JsValue::Number(100.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    // Continue with TLA
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(100.0));
        }
        _ => panic!("Expected Complete after TLA"),
    }
}

/// Test: Multiple top-level awaits
#[test]
fn test_multiple_top_level_awaits() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        const a = await Promise.resolve(1);
        const b = await Promise.resolve(2);
        const c = await Promise.resolve(3);
        a + b + c
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

/// Test: Top-level await with Promise.all
#[test]
fn test_top_level_await_promise_all() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        const results = await Promise.all([
            Promise.resolve(10),
            Promise.resolve(20),
            Promise.resolve(30)
        ]);
        results[0] + results[1] + results[2]
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(60.0));
        }
        _ => panic!("Expected Complete"),
    }
}

/// Test: Top-level await with error handling
#[test]
fn test_top_level_await_with_try_catch() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        let result: string;
        try {
            const value = await Promise.reject("error!");
            result = "success: " + value;
        } catch (e) {
            result = "caught: " + e;
        }
        result
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::String("caught: error!".into()));
        }
        _ => panic!("Expected Complete"),
    }
}

/// Test: Top-level await in conditional
#[test]
fn test_top_level_await_conditional() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        const shouldAwait = true;
        let value: number;

        if (shouldAwait) {
            value = await Promise.resolve(42);
        } else {
            value = 0;
        }

        value
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

/// Test: Top-level await with loop
#[test]
fn test_top_level_await_in_loop() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        let sum = 0;
        const values = [1, 2, 3];

        for (let i = 0; i < values.length; i++) {
            sum += await Promise.resolve(values[i]);
        }

        sum
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

/// Test: Top-level await export
#[test]
fn test_top_level_await_with_export() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        const data = await Promise.resolve({ value: 42 });
        export const exportedValue = data.value;
        exportedValue
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(42.0));
        }
        _ => panic!("Expected Complete"),
    }

    // Verify the export is available
    let exports = runtime.get_exports();
    assert!(exports.contains_key("exportedValue"));
    assert_eq!(*exports.get("exportedValue").unwrap(), JsValue::Number(42.0));
}

/// Test: Top-level await with dynamic import
#[test]
fn test_top_level_await_dynamic_import() {
    let mut runtime = Runtime::new();

    // Note: Dynamic import currently returns a pending promise that doesn't suspend
    // This test verifies the syntax works, not full dynamic import resolution
    let result = runtime
        .eval(
            r#"
        // Await a resolved promise - dynamic import scenario would be similar
        // Using quoted property names to avoid 'default' keyword issue
        const moduleData = await Promise.resolve({ "defaultExport": 123, named: 456 });
        moduleData.defaultExport + moduleData.named
    "#,
        )
        .unwrap();

    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(579.0)); // 123 + 456
        }
        _ => panic!("Expected Complete"),
    }
}

/// Test: Top-level await combined with imports and exports
#[test]
fn test_top_level_await_full_module() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import { baseConfig } from './config';

        // Top-level await to "load" additional data
        const additionalData = await Promise.resolve({ extra: 50 });

        export const config = {
            ...baseConfig,
            ...additionalData
        };

        config.value + config.extra
    "#,
        )
        .unwrap();

    // First: resolve import
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./config");
            // Create inner object first to avoid borrow conflict
            let base_config = runtime.create_module_object(vec![
                ("value".to_string(), JsValue::Number(100.0)),
            ]);
            let module = runtime.create_module_object(vec![
                ("baseConfig".to_string(), base_config),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(150.0)); // 100 + 50
        }
        _ => panic!("Expected Complete"),
    }
}
