//! Order system integration tests
//!
//! These tests demonstrate how the order system can be used for real-world async operations
//! like fetch(), setTimeout(), and file I/O. The global functions are implemented in TypeScript
//! using the __order__ syscall from eval:internal.

use serde_json::json;
use typescript_eval::{
    interpreter::builtins::create_eval_internal_module, InternalModule, JsError, JsValue,
    OrderResponse, Runtime, RuntimeConfig, RuntimeResult, RuntimeValue,
};

// ═══════════════════════════════════════════════════════════════════════════════
// TypeScript Globals Source
// ═══════════════════════════════════════════════════════════════════════════════

/// TypeScript source that defines global functions using the order system.
/// This demonstrates how hosts can extend the interpreter with custom async operations.
const GLOBALS_SOURCE: &str = r#"
import { __order__ } from "eval:internal";

// Timer ID counter for setTimeout/clearTimeout
let nextTimerId = 1;
const pendingTimers = new Map<number, boolean>();

// setTimeout with callback support (standard API)
globalThis.setTimeout = function(callback: Function, delay: number = 0, ...args: any[]): number {
    const timerId = nextTimerId++;

    __order__({ type: "setTimeout", delay: delay }).then(() => {
        // Only invoke callback if timer wasn't cleared
        if (pendingTimers.has(timerId)) {
            pendingTimers.delete(timerId);
            callback(...args);
        }
    });

    pendingTimers.set(timerId, true);
    return timerId;
};

globalThis.clearTimeout = function(timerId: number): void {
    pendingTimers.delete(timerId);
};

// fetch(url, options?) - Returns promise with response
globalThis.fetch = function(url: string, options?: {
    method?: string;
    body?: string;
    headers?: Record<string, string>;
}): Promise<any> {
    return __order__({
        type: "fetch",
        url: url,
        method: options?.method || "GET",
        body: options?.body,
        headers: options?.headers
    });
};

// readFile(path) - Returns promise with file content
globalThis.readFile = function(path: string): Promise<string> {
    return __order__({ type: "readFile", path: path });
};

// writeFile(path, content) - Returns promise when done
globalThis.writeFile = function(path: string, content: string): Promise<void> {
    return __order__({ type: "writeFile", path: path, content: content });
};
"#;

// ═══════════════════════════════════════════════════════════════════════════════
// Test Helpers
// ═══════════════════════════════════════════════════════════════════════════════

/// Create a runtime with eval:internal and eval:globals modules
fn create_test_runtime() -> Runtime {
    let config = RuntimeConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        timeout_ms: 5000,
    };
    let runtime = Runtime::with_config(config);

    // Set aggressive GC for testing
    let gc_threshold = std::env::var("GC_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    runtime.set_gc_threshold(gc_threshold);

    runtime
}

/// Extract string property from JsValue object
fn get_string_prop(obj: &JsValue, key: &str) -> Option<String> {
    if let JsValue::Object(o) = obj {
        if let Some(JsValue::String(s)) = o.borrow().get_property(&key.into()) {
            return Some(s.to_string());
        }
    }
    None
}

/// Extract number property from JsValue object
fn get_number_prop(obj: &JsValue, key: &str) -> Option<f64> {
    if let JsValue::Object(o) = obj {
        if let Some(JsValue::Number(n)) = o.borrow().get_property(&key.into()) {
            return Some(n);
        }
    }
    None
}

/// Run script with globals, handling the import of eval:globals first
fn run_with_globals(runtime: &mut Runtime, script: &str) -> RuntimeResult {
    // Prepend import of globals module to register global functions
    let full_script = format!(
        r#"import "eval:globals";
{}"#,
        script
    );
    runtime.eval(&full_script).expect("eval should not fail")
}

// ═══════════════════════════════════════════════════════════════════════════════
// Promise.then() callback tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_promise_then_callback_closure() {
    let mut runtime = create_test_runtime();

    // Test that .then() callbacks can access closure variables after fulfillment
    // The callback should modify a variable, and we await the result of .then()
    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        // Create a captured variable in module scope
        let captured = "initial";

        // Call __order__ and attach a .then() callback that modifies captured
        // Await the result to ensure the callback runs before we return
        await __order__({ type: "test" }).then(() => {
            captured = "modified";
        });

        // Return the captured value (should be "modified" after callback ran)
        captured;
    "#,
    );

    match result {
        RuntimeResult::Suspended { pending, .. } => {
            assert_eq!(pending.len(), 1);
            assert_eq!(
                get_string_prop(pending[0].payload.value(), "type"),
                Some("test".into())
            );

            // Fulfill the order - this should trigger the .then() callback
            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
            };

            let result2 = runtime.fulfill_orders(vec![response]).unwrap();

            match result2 {
                RuntimeResult::Complete(value) => {
                    // After fulfillment, the callback should have run and modified `captured`
                    assert_eq!(*value, JsValue::String("modified".into()));
                }
                other => panic!("Expected Complete after fulfillment, got {:?}", other),
            }
        }
        other => panic!("Expected Suspended, got {:?}", other),
    }
}

#[test]
fn test_promise_then_callback_nested_closure() {
    let mut runtime = create_test_runtime();

    // Test that .then() callbacks can access module-level variables from inside
    // a function call (nested closure)
    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        // Module-level variable
        let moduleVar = "initial";

        // Function that returns a callback capturing moduleVar
        function createCallback(): () => void {
            return () => {
                moduleVar = "modified";
            };
        }

        // Call createCallback to get a callback, then use it in .then()
        const cb = createCallback();
        await __order__({ type: "test" }).then(cb);

        moduleVar;
    "#,
    );

    match result {
        RuntimeResult::Suspended { pending, .. } => {
            assert_eq!(pending.len(), 1);

            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
            };

            let result2 = runtime.fulfill_orders(vec![response]).unwrap();

            match result2 {
                RuntimeResult::Complete(value) => {
                    assert_eq!(*value, JsValue::String("modified".into()));
                }
                other => panic!("Expected Complete, got {:?}", other),
            }
        }
        other => panic!("Expected Suspended, got {:?}", other),
    }
}

#[test]
fn test_cross_module_closure_simple() {
    // Test that callbacks from another module can access that module's variables
    // This mimics the setTimeout pattern where the callback accesses pendingTimers
    let config = RuntimeConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source(
                "eval:timer-module",
                r#"
                import { __order__ } from "eval:internal";

                // Module-level variable
                let moduleState: string = "initial";

                // Function that uses __order__ with a .then() callback
                // The callback captures moduleState from this module
                export function runWithCallback(): Promise<void> {
                    return __order__({ type: "timer" }).then(() => {
                        moduleState = "from-callback";
                    });
                }

                export function getState(): string {
                    return moduleState;
                }
            "#,
            ),
        ],
        timeout_ms: 5000,
    };
    let mut runtime = Runtime::with_config(config);

    let gc_threshold = std::env::var("GC_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    runtime.set_gc_threshold(gc_threshold);

    let result = runtime
        .eval(
            r#"
            import { runWithCallback, getState } from "eval:timer-module";

            // Call the function that uses .then()
            await runWithCallback();

            // Check the state
            getState();
        "#,
        )
        .expect("eval should work");

    match result {
        RuntimeResult::Suspended { pending, .. } => {
            assert_eq!(pending.len(), 1);

            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
            };

            let result2 = runtime.fulfill_orders(vec![response]).unwrap();

            match result2 {
                RuntimeResult::Complete(value) => {
                    assert_eq!(*value, JsValue::String("from-callback".into()));
                }
                other => panic!("Expected Complete, got {:?}", other),
            }
        }
        other => panic!("Expected Suspended, got {:?}", other),
    }
}

#[test]
fn test_cross_module_nested_closure() {
    // Test that a callback defined in one module can access a variable from an outer
    // function's closure, where that function is defined in another module
    let config = RuntimeConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source(
                "eval:helper-module",
                r#"
                import { __order__ } from "eval:internal";

                // Module-level state
                let moduleState: string = "module-initial";

                // This function wraps __order__.then() with a callback that
                // accesses BOTH function-local variable AND module variable
                export function wrapWithThen(userCallback: () => void): Promise<void> {
                    const functionLocal = "function-local";

                    return __order__({ type: "wrapped" }).then(() => {
                        // Access module variable
                        moduleState = "from-then";
                        // Access function-local variable
                        const combined = functionLocal + "+" + moduleState;
                        // Call user callback
                        userCallback();
                    });
                }

                export function getState(): string {
                    return moduleState;
                }
            "#,
            ),
        ],
        timeout_ms: 5000,
    };
    let mut runtime = Runtime::with_config(config);

    let gc_threshold = std::env::var("GC_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    runtime.set_gc_threshold(gc_threshold);

    let result = runtime
        .eval(
            r#"
            import { wrapWithThen, getState } from "eval:helper-module";

            let userResult = "not-called";

            await wrapWithThen(() => {
                userResult = "user-callback-ran";
            });

            // Return both the module state and user result
            getState() + " / " + userResult;
        "#,
        )
        .expect("eval should work");

    match result {
        RuntimeResult::Suspended { pending, .. } => {
            assert_eq!(pending.len(), 1);

            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
            };

            let result2 = runtime.fulfill_orders(vec![response]).unwrap();

            match result2 {
                RuntimeResult::Complete(value) => {
                    assert_eq!(
                        *value,
                        JsValue::String("from-then / user-callback-ran".into())
                    );
                }
                other => panic!("Expected Complete, got {:?}", other),
            }
        }
        other => panic!("Expected Suspended, got {:?}", other),
    }
}

#[test]
fn test_debug_closure_gc() {
    // Minimal reproduction of the GC bug
    let config = RuntimeConfig {
        internal_modules: vec![create_eval_internal_module()],
        timeout_ms: 5000,
    };
    let mut runtime = Runtime::with_config(config);
    runtime.set_gc_threshold(1);

    // This should work - no function-local variables, callback accesses module var directly
    let result = runtime
        .eval(
            r#"
            import { __order__ } from "eval:internal";

            let state: string = "initial";

            // Wrapper function WITH a local variable
            function wrapper(): Promise<void> {
                const local = "local";  // This triggers call env creation
                return __order__({ type: "test" }).then(() => {
                    state = "modified-" + local;
                });
            }

            await wrapper();
            state;
        "#,
        )
        .expect("eval should work");

    match result {
        RuntimeResult::Suspended { pending, .. } => {
            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
            };

            let result2 = runtime.fulfill_orders(vec![response]).unwrap();

            match result2 {
                RuntimeResult::Complete(value) => {
                    assert_eq!(*value, JsValue::String("modified-local".into()));
                }
                other => panic!("Expected Complete, got {:?}", other),
            }
        }
        other => panic!("Expected Suspended, got {:?}", other),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// setTimeout Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_set_timeout_basic() {
    let mut runtime = create_test_runtime();

    // Use callback-based setTimeout wrapped in a Promise for async/await
    let result = run_with_globals(
        &mut runtime,
        r#"
        let result = "";
        await new Promise((resolve) => {
            setTimeout(() => {
                result = "done";
                resolve(undefined);
            }, 100);
        });
        result;
    "#,
    );

    match result {
        RuntimeResult::Suspended { pending, .. } => {
            assert_eq!(pending.len(), 1);

            // Verify the order payload
            let payload = pending[0].payload.value();
            assert_eq!(get_string_prop(payload, "type"), Some("setTimeout".into()));
            assert_eq!(get_number_prop(payload, "delay"), Some(100.0));

            // Fulfill the order
            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
            };

            let result2 = runtime.fulfill_orders(vec![response]).unwrap();

            match result2 {
                RuntimeResult::Complete(value) => {
                    assert_eq!(*value, JsValue::String("done".into()));
                }
                _ => panic!("Expected Complete after fulfillment"),
            }
        }
        _ => panic!("Expected Suspended, got {:?}", result),
    }
}

#[test]
fn test_set_timeout_sequential() {
    let mut runtime = create_test_runtime();

    // Helper to wrap callback-based setTimeout in a promise
    let result = run_with_globals(
        &mut runtime,
        r#"
        function delay(ms: number): Promise<void> {
            return new Promise((resolve) => setTimeout(resolve, ms));
        }

        let count = 0;
        await delay(10);
        count += 1;
        await delay(20);
        count += 1;
        await delay(30);
        count += 1;
        count;
    "#,
    );

    // First setTimeout
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first setTimeout");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_number_prop(pending[0].payload.value(), "delay"),
        Some(10.0)
    );

    let result = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
        }])
        .unwrap();

    // Second setTimeout
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for second setTimeout");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_number_prop(pending[0].payload.value(), "delay"),
        Some(20.0)
    );

    let result = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
        }])
        .unwrap();

    // Third setTimeout
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for third setTimeout");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_number_prop(pending[0].payload.value(), "delay"),
        Some(30.0)
    );

    let result = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
        }])
        .unwrap();

    // Complete
    let RuntimeResult::Complete(value) = result else {
        panic!("Expected Complete after all timeouts");
    };
    assert_eq!(*value, JsValue::Number(3.0));
}

// ═══════════════════════════════════════════════════════════════════════════════
// fetch Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_fetch_get_basic() {
    let mut runtime = create_test_runtime();

    let result = run_with_globals(
        &mut runtime,
        r#"
        const response = await fetch("https://api.example.com/users/1");
        response.name;
    "#,
    );

    match result {
        RuntimeResult::Suspended { pending, .. } => {
            assert_eq!(pending.len(), 1);

            // Verify the order payload
            let payload = pending[0].payload.value();
            assert_eq!(get_string_prop(payload, "type"), Some("fetch".into()));
            assert_eq!(
                get_string_prop(payload, "url"),
                Some("https://api.example.com/users/1".into())
            );
            assert_eq!(get_string_prop(payload, "method"), Some("GET".into()));

            // Return mock response using create_response_object
            let mock_response = runtime
                .create_response_object(&json!({
                    "id": 1,
                    "name": "John",
                    "email": "john@example.com"
                }))
                .unwrap();

            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(mock_response),
            };

            let result2 = runtime.fulfill_orders(vec![response]).unwrap();

            match result2 {
                RuntimeResult::Complete(value) => {
                    assert_eq!(*value, JsValue::String("John".into()));
                }
                _ => panic!("Expected Complete after fulfillment"),
            }
        }
        _ => panic!("Expected Suspended"),
    }
}

#[test]
fn test_fetch_post_with_body() {
    let mut runtime = create_test_runtime();

    let result = run_with_globals(
        &mut runtime,
        r#"
        const response = await fetch("https://api.example.com/users", {
            method: "POST",
            body: JSON.stringify({ name: "Jane" }),
            headers: { "Content-Type": "application/json" }
        });
        response.id;
    "#,
    );

    match result {
        RuntimeResult::Suspended { pending, .. } => {
            assert_eq!(pending.len(), 1);

            let payload = pending[0].payload.value();
            assert_eq!(get_string_prop(payload, "type"), Some("fetch".into()));
            assert_eq!(
                get_string_prop(payload, "url"),
                Some("https://api.example.com/users".into())
            );
            assert_eq!(get_string_prop(payload, "method"), Some("POST".into()));
            assert_eq!(
                get_string_prop(payload, "body"),
                Some(r#"{"name":"Jane"}"#.into())
            );

            // Return mock created response
            let mock_response = runtime
                .create_response_object(&json!({ "id": 42, "name": "Jane" }))
                .unwrap();

            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(mock_response),
            };

            let result2 = runtime.fulfill_orders(vec![response]).unwrap();

            match result2 {
                RuntimeResult::Complete(value) => {
                    assert_eq!(*value, JsValue::Number(42.0));
                }
                _ => panic!("Expected Complete after fulfillment"),
            }
        }
        _ => panic!("Expected Suspended"),
    }
}

#[test]
fn test_fetch_network_error() {
    let mut runtime = create_test_runtime();

    let result = run_with_globals(
        &mut runtime,
        r#"
        let errorMessage = "";
        try {
            await fetch("https://api.example.com/fail");
        } catch (e) {
            errorMessage = String(e);
        }
        errorMessage;
    "#,
    );

    match result {
        RuntimeResult::Suspended { pending, .. } => {
            assert_eq!(pending.len(), 1);

            // Reject the order with an error
            let response = OrderResponse {
                id: pending[0].id,
                result: Err(JsError::type_error("Network error: connection refused")),
            };

            let result2 = runtime.fulfill_orders(vec![response]).unwrap();

            match result2 {
                RuntimeResult::Complete(value) => {
                    let msg = value.value().to_js_string().to_string();
                    assert!(
                        msg.contains("Network error"),
                        "Expected error message, got: {}",
                        msg
                    );
                }
                _ => panic!("Expected Complete after rejection"),
            }
        }
        _ => panic!("Expected Suspended"),
    }
}

#[test]
fn test_fetch_parallel() {
    let mut runtime = create_test_runtime();

    // Simplified test - two sequential awaits instead of Promise.all
    let result = run_with_globals(
        &mut runtime,
        r#"
        const user = await fetch("/users/1");
        const posts = await fetch("/posts?userId=1");
        user.name + " has " + posts.length + " posts";
    "#,
    );

    // First suspension: fetch("/users/1")
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first fetch");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "url"),
        Some("/users/1".into())
    );

    let user_response = runtime
        .create_response_object(&json!({ "name": "John" }))
        .unwrap();
    let result2 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(user_response),
        }])
        .unwrap();

    // Second suspension: fetch("/posts?userId=1")
    let RuntimeResult::Suspended { pending, .. } = result2 else {
        panic!("Expected Suspended for second fetch, got {:?}", result2);
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "url"),
        Some("/posts?userId=1".into())
    );

    let posts_response = runtime
        .create_response_object(&json!([{ "id": 1 }, { "id": 2 }, { "id": 3 }]))
        .unwrap();
    let result3 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(posts_response),
        }])
        .unwrap();

    // Final result
    let RuntimeResult::Complete(value) = result3 else {
        panic!("Expected Complete after fulfillment, got {:?}", result3);
    };
    assert_eq!(*value, JsValue::String("John has 3 posts".into()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// File System Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_read_file_basic() {
    let mut runtime = create_test_runtime();

    let result = run_with_globals(
        &mut runtime,
        r#"
        const content = await readFile("/config.txt");
        content;
    "#,
    );

    match result {
        RuntimeResult::Suspended { pending, .. } => {
            assert_eq!(pending.len(), 1);

            let payload = pending[0].payload.value();
            assert_eq!(get_string_prop(payload, "type"), Some("readFile".into()));
            assert_eq!(get_string_prop(payload, "path"), Some("/config.txt".into()));

            // Return mock file content
            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(RuntimeValue::unguarded(JsValue::String(
                    "Hello, World!".into(),
                ))),
            };

            let result2 = runtime.fulfill_orders(vec![response]).unwrap();

            match result2 {
                RuntimeResult::Complete(value) => {
                    assert_eq!(*value, JsValue::String("Hello, World!".into()));
                }
                _ => panic!("Expected Complete after fulfillment"),
            }
        }
        _ => panic!("Expected Suspended"),
    }
}

#[test]
fn test_read_file_json_parse() {
    let mut runtime = create_test_runtime();

    let result = run_with_globals(
        &mut runtime,
        r#"
        const raw = await readFile("/config.json");
        const config = JSON.parse(raw);
        config.database.host;
    "#,
    );

    match result {
        RuntimeResult::Suspended { pending, .. } => {
            assert_eq!(pending.len(), 1);

            // Return mock JSON content
            let json_content = r#"{"database": {"host": "localhost", "port": 5432}}"#;
            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(RuntimeValue::unguarded(JsValue::String(
                    json_content.into(),
                ))),
            };

            let result2 = runtime.fulfill_orders(vec![response]).unwrap();

            match result2 {
                RuntimeResult::Complete(value) => {
                    assert_eq!(*value, JsValue::String("localhost".into()));
                }
                _ => panic!("Expected Complete after fulfillment"),
            }
        }
        _ => panic!("Expected Suspended"),
    }
}

#[test]
fn test_read_file_not_found() {
    let mut runtime = create_test_runtime();

    let result = run_with_globals(
        &mut runtime,
        r#"
        let errorMsg = "";
        try {
            await readFile("/nonexistent.txt");
        } catch (e) {
            errorMsg = String(e);
        }
        errorMsg;
    "#,
    );

    match result {
        RuntimeResult::Suspended { pending, .. } => {
            assert_eq!(pending.len(), 1);

            // Reject with file not found error
            let response = OrderResponse {
                id: pending[0].id,
                result: Err(JsError::type_error(
                    "ENOENT: no such file or directory '/nonexistent.txt'",
                )),
            };

            let result2 = runtime.fulfill_orders(vec![response]).unwrap();

            match result2 {
                RuntimeResult::Complete(value) => {
                    let msg = value.value().to_js_string().to_string();
                    assert!(
                        msg.contains("ENOENT"),
                        "Expected ENOENT error, got: {}",
                        msg
                    );
                }
                _ => panic!("Expected Complete after rejection"),
            }
        }
        _ => panic!("Expected Suspended"),
    }
}

#[test]
fn test_write_file_basic() {
    let mut runtime = create_test_runtime();

    let result = run_with_globals(
        &mut runtime,
        r#"
        await writeFile("/output.txt", "Hello from TypeScript!");
        "written";
    "#,
    );

    match result {
        RuntimeResult::Suspended { pending, .. } => {
            assert_eq!(pending.len(), 1);

            let payload = pending[0].payload.value();
            assert_eq!(get_string_prop(payload, "type"), Some("writeFile".into()));
            assert_eq!(get_string_prop(payload, "path"), Some("/output.txt".into()));
            assert_eq!(
                get_string_prop(payload, "content"),
                Some("Hello from TypeScript!".into())
            );

            // Acknowledge write success
            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
            };

            let result2 = runtime.fulfill_orders(vec![response]).unwrap();

            match result2 {
                RuntimeResult::Complete(value) => {
                    assert_eq!(*value, JsValue::String("written".into()));
                }
                _ => panic!("Expected Complete after fulfillment"),
            }
        }
        _ => panic!("Expected Suspended"),
    }
}

#[test]
fn test_read_write_roundtrip() {
    let mut runtime = create_test_runtime();

    // Simulated file storage for this test
    let file_content: String;

    let result = run_with_globals(
        &mut runtime,
        r#"
        const data = "test data 12345";
        await writeFile("/temp.txt", data);
        const read = await readFile("/temp.txt");
        read === data;
    "#,
    );

    // First: writeFile
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for writeFile");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "type"),
        Some("writeFile".into())
    );

    // Capture the written content
    file_content = get_string_prop(pending[0].payload.value(), "content").unwrap_or_default();
    assert_eq!(file_content, "test data 12345");

    let result = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
        }])
        .unwrap();

    // Second: readFile
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for readFile");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "type"),
        Some("readFile".into())
    );

    // Return the "stored" content
    let result = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(JsValue::String(
                file_content.into(),
            ))),
        }])
        .unwrap();

    // Complete
    let RuntimeResult::Complete(value) = result else {
        panic!("Expected Complete after roundtrip");
    };
    assert_eq!(*value, JsValue::Boolean(true));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Parallel Fetch Tests (Promise.all-like behavior)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_fetch_parallel_resolve_one_at_a_time() {
    // Test parallel fetch requests where we resolve them one at a time.
    // The runtime should suspend with both pending orders initially,
    // then suspend again after we resolve just one, waiting for the other.
    let mut runtime = create_test_runtime();

    let result = run_with_globals(
        &mut runtime,
        r#"
        // Start two fetch requests in parallel using Promise.all
        const results = await Promise.all([
            fetch("/users"),
            fetch("/posts")
        ]);

        // Return the combined result
        results[0].count + " users, " + results[1].count + " posts";
    "#,
    );

    // First suspension: both fetch requests should be pending
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended with both fetch requests pending");
    };
    assert_eq!(
        pending.len(),
        2,
        "Expected 2 pending orders for parallel fetch"
    );

    // Verify both orders are fetch requests
    let urls: Vec<String> = pending
        .iter()
        .filter_map(|o| get_string_prop(o.payload.value(), "url"))
        .collect();
    assert!(urls.contains(&"/users".to_string()));
    assert!(urls.contains(&"/posts".to_string()));

    // Find which order ID corresponds to which URL
    let users_order = pending
        .iter()
        .find(|o| get_string_prop(o.payload.value(), "url") == Some("/users".into()))
        .expect("Should find /users order");
    let posts_order = pending
        .iter()
        .find(|o| get_string_prop(o.payload.value(), "url") == Some("/posts".into()))
        .expect("Should find /posts order");

    // Resolve only the first order (/users)
    let users_response = runtime
        .create_response_object(&json!({ "count": 42 }))
        .unwrap();
    let result2 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: users_order.id,
            result: Ok(users_response),
        }])
        .unwrap();

    // Second suspension: should still be waiting for /posts
    // Note: pending is empty because the host already received the order in the first suspension
    let RuntimeResult::Suspended { pending, .. } = result2 else {
        panic!(
            "Expected Suspended waiting for second fetch, got {:?}",
            result2
        );
    };
    assert!(
        pending.is_empty(),
        "Expected empty pending (host already has the order)"
    );

    // Now resolve the second order (/posts) using the ID we saved earlier
    let posts_response = runtime
        .create_response_object(&json!({ "count": 100 }))
        .unwrap();
    let result3 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: posts_order.id,
            result: Ok(posts_response),
        }])
        .unwrap();

    // Should be complete now
    let RuntimeResult::Complete(value) = result3 else {
        panic!("Expected Complete after both fetches resolved");
    };
    assert_eq!(*value, JsValue::String("42 users, 100 posts".into()));
}

#[test]
fn test_fetch_parallel_resolve_second_first() {
    // Similar to above but resolve the second request before the first
    // to verify order doesn't matter for Promise.all
    let mut runtime = create_test_runtime();

    let result = run_with_globals(
        &mut runtime,
        r#"
        const results = await Promise.all([
            fetch("/alpha"),
            fetch("/beta")
        ]);
        results[0].name + " and " + results[1].name;
    "#,
    );

    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    assert_eq!(pending.len(), 2);

    let alpha_order = pending
        .iter()
        .find(|o| get_string_prop(o.payload.value(), "url") == Some("/alpha".into()))
        .expect("Should find /alpha");
    let beta_order = pending
        .iter()
        .find(|o| get_string_prop(o.payload.value(), "url") == Some("/beta".into()))
        .expect("Should find /beta");

    // Resolve /beta first (the second request)
    let beta_response = runtime
        .create_response_object(&json!({ "name": "Beta" }))
        .unwrap();
    let result2 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: beta_order.id,
            result: Ok(beta_response),
        }])
        .unwrap();

    // Should still be suspended waiting for /alpha
    // Note: pending is empty because the host already has the order
    let RuntimeResult::Suspended { pending, .. } = result2 else {
        panic!("Expected Suspended waiting for /alpha");
    };
    assert!(pending.is_empty());

    // Now resolve /alpha using the ID we saved earlier
    let alpha_response = runtime
        .create_response_object(&json!({ "name": "Alpha" }))
        .unwrap();
    let result3 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: alpha_order.id,
            result: Ok(alpha_response),
        }])
        .unwrap();

    let RuntimeResult::Complete(value) = result3 else {
        panic!("Expected Complete");
    };
    // Results should be in original array order, not resolution order
    assert_eq!(*value, JsValue::String("Alpha and Beta".into()));
}

#[test]
fn test_fetch_three_parallel_resolve_middle_last() {
    // Three parallel requests, resolve in order: first, third, second
    let mut runtime = create_test_runtime();

    let result = run_with_globals(
        &mut runtime,
        r#"
        const [a, b, c] = await Promise.all([
            fetch("/a"),
            fetch("/b"),
            fetch("/c")
        ]);
        a.v + b.v + c.v;
    "#,
    );

    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    assert_eq!(pending.len(), 3);

    let find_order = |url: &str| {
        pending
            .iter()
            .find(|o| get_string_prop(o.payload.value(), "url") == Some(url.into()))
            .expect(&format!("Should find {}", url))
    };

    let order_a = find_order("/a");
    let order_b = find_order("/b");
    let order_c = find_order("/c");

    // Resolve /a first
    let resp_a = runtime.create_response_object(&json!({ "v": 10 })).unwrap();
    let result2 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: order_a.id,
            result: Ok(resp_a),
        }])
        .unwrap();

    let RuntimeResult::Suspended { pending, .. } = result2 else {
        panic!("Expected Suspended after first resolve");
    };
    // pending is empty because host already has the orders
    assert!(pending.is_empty());

    // Resolve /c next (skipping /b)
    let resp_c = runtime.create_response_object(&json!({ "v": 30 })).unwrap();
    let result3 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: order_c.id,
            result: Ok(resp_c),
        }])
        .unwrap();

    let RuntimeResult::Suspended { pending, .. } = result3 else {
        panic!("Expected Suspended after second resolve");
    };
    // pending is empty because host already has the order
    assert!(pending.is_empty());

    // Finally resolve /b
    let resp_b = runtime.create_response_object(&json!({ "v": 20 })).unwrap();
    let result4 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: order_b.id,
            result: Ok(resp_b),
        }])
        .unwrap();

    let RuntimeResult::Complete(value) = result4 else {
        panic!("Expected Complete");
    };
    assert_eq!(*value, JsValue::Number(60.0)); // 10 + 20 + 30
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combined Workflow Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_config_generation_workflow() {
    let mut runtime = create_test_runtime();

    let result = run_with_globals(
        &mut runtime,
        r#"
        // 1. Read local config
        const configRaw = await readFile("/app.json");
        const config = JSON.parse(configRaw);

        // 2. Fetch remote data
        const apiData = await fetch(config.apiUrl + "/settings");

        // 3. Generate manifest
        const manifest = {
            name: config.name,
            version: config.version,
            settings: apiData,
        };

        // 4. Write output
        await writeFile("/manifest.json", JSON.stringify(manifest));

        manifest.name + " v" + manifest.version;
    "#,
    );

    // Step 1: readFile /app.json
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for readFile");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "type"),
        Some("readFile".into())
    );

    let config_json =
        r#"{"name": "MyApp", "version": "1.0.0", "apiUrl": "https://api.example.com"}"#;
    let result = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(JsValue::String(config_json.into()))),
        }])
        .unwrap();

    // Step 2: fetch api settings
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for fetch");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "type"),
        Some("fetch".into())
    );
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "url"),
        Some("https://api.example.com/settings".into())
    );

    let api_response = runtime
        .create_response_object(&json!({ "theme": "dark", "language": "en" }))
        .unwrap();
    let result = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(api_response),
        }])
        .unwrap();

    // Step 3: writeFile /manifest.json
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for writeFile");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "type"),
        Some("writeFile".into())
    );
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "path"),
        Some("/manifest.json".into())
    );

    // Verify the manifest content
    let manifest_content = get_string_prop(pending[0].payload.value(), "content").unwrap();
    assert!(manifest_content.contains("MyApp"));
    assert!(manifest_content.contains("1.0.0"));
    assert!(manifest_content.contains("dark"));

    let result = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
        }])
        .unwrap();

    // Complete
    let RuntimeResult::Complete(value) = result else {
        panic!("Expected Complete after workflow");
    };
    assert_eq!(*value, JsValue::String("MyApp v1.0.0".into()));
}
