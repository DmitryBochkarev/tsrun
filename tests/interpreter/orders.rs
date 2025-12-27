//! Order system integration tests
//!
//! These tests demonstrate how the order system can be used for real-world async operations
//! like fetch(), setTimeout(), and file I/O. The global functions are implemented in TypeScript
//! using the __order__ syscall from eval:internal.

use serde_json::json;
use typescript_eval::{
    interpreter::builtins::create_eval_internal_module, value::PropertyKey, InternalModule,
    JsString, JsValue, OrderResponse, Runtime, RuntimeConfig, RuntimeResult, RuntimeValue,
};

// ═══════════════════════════════════════════════════════════════════════════════
// TypeScript Globals Source
// ═══════════════════════════════════════════════════════════════════════════════

/// TypeScript source that defines global functions using the order system.
/// This demonstrates how hosts can extend the interpreter with custom async operations.
///
/// With blocking __order__() semantics:
/// - __order__() suspends VM immediately and returns host's value when resumed
/// - Functions return actual values, not Promises
/// - await is optional (used only if host returns a Promise)
const GLOBALS_SOURCE: &str = r#"
import { __order__ } from "eval:internal";

// sleep(ms) - Blocks until delay passes
// Host responds when timer fires
globalThis.sleep = function(ms: number): void {
    __order__({ type: "sleep", delay: ms });
};

// fetch(url, options?) - Blocks until response received
globalThis.fetch = function(url: string, options?: {
    method?: string;
    body?: string;
    headers?: Record<string, string>;
}): any {
    return __order__({
        type: "fetch",
        url: url,
        method: options?.method || "GET",
        body: options?.body,
        headers: options?.headers
    });
};

// readFile(path) - Blocks until file content is read
globalThis.readFile = function(path: string): string {
    return __order__({ type: "readFile", path: path });
};

// writeFile(path, content) - Blocks until file is written
globalThis.writeFile = function(path: string, content: string): string {
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
        if let Some(JsValue::String(s)) = o
            .borrow()
            .get_property(&PropertyKey::String(JsString::from(key)))
        {
            return Some(s.to_string());
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
// Promise .then() Tests
// With blocking __order__() semantics, to use .then() patterns:
// - Host returns a Promise via fulfill_orders()
// - Script calls .then() on that Promise
// - Host resolves the Promise to trigger the callback
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn test_promise_then_callback_closure() {
    let mut runtime = create_test_runtime();

    // Test that .then() callbacks can access closure variables after Promise resolution
    // Host returns a Promise, script attaches .then() callback, host resolves Promise
    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        // Create a captured variable in module scope
        let captured = "initial";

        // Call __order__ to get a Promise from host, then attach .then() callback
        const promise = __order__({ type: "getPromise" });
        await promise.then(() => {
            captured = "modified";
        });

        // Return the captured value (should be "modified" after callback ran)
        captured;
    "#,
    );

    // First suspension: waiting for __order__ response
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for __order__");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "type"),
        Some("getPromise".into())
    );

    // Host creates and returns a Promise
    let promise = runtime.create_promise();
    let result2 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(promise.value().clone())),
        }])
        .unwrap();

    // Second suspension: script is awaiting the Promise (via .then())
    let RuntimeResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise resolution");
    };

    // Resolve the Promise - this triggers the .then() callback
    let result3 = runtime
        .resolve_promise(&promise, RuntimeValue::unguarded(JsValue::Undefined))
        .unwrap();

    // After resolution, the callback should have run and modified `captured`
    let RuntimeResult::Complete(value) = result3 else {
        panic!("Expected Complete after Promise resolution");
    };
    assert_eq!(*value, JsValue::String("modified".into()));
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
        const promise = __order__({ type: "getPromise" });
        await promise.then(cb);

        moduleVar;
    "#,
    );

    // First suspension: waiting for __order__ response
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for __order__");
    };
    assert_eq!(pending.len(), 1);

    // Host returns a Promise
    let promise = runtime.create_promise();
    let result2 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(promise.value().clone())),
        }])
        .unwrap();

    // Second suspension: awaiting the Promise
    let RuntimeResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Resolve the Promise
    let result3 = runtime
        .resolve_promise(&promise, RuntimeValue::unguarded(JsValue::Undefined))
        .unwrap();

    let RuntimeResult::Complete(value) = result3 else {
        panic!("Expected Complete after Promise resolution");
    };
    assert_eq!(*value, JsValue::String("modified".into()));
}

#[test]
fn test_cross_module_closure_simple() {
    // Test that callbacks from another module can access that module's variables
    // Host returns a Promise, module attaches .then() callback
    let config = RuntimeConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source(
                "eval:timer-module",
                r#"
                import { __order__ } from "eval:internal";

                // Module-level variable
                let moduleState: string = "initial";

                // Function that gets a Promise from host and attaches .then() callback
                // The callback captures moduleState from this module
                export function runWithCallback(): Promise<void> {
                    const promise = __order__({ type: "getPromise" });
                    return promise.then(() => {
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

    // First suspension: __order__ waiting for host response
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for __order__");
    };
    assert_eq!(pending.len(), 1);

    // Host returns a Promise
    let promise = runtime.create_promise();
    let result2 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(promise.value().clone())),
        }])
        .unwrap();

    // Second suspension: awaiting the Promise
    let RuntimeResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Resolve the Promise - triggers .then() callback
    let result3 = runtime
        .resolve_promise(&promise, RuntimeValue::unguarded(JsValue::Undefined))
        .unwrap();

    let RuntimeResult::Complete(value) = result3 else {
        panic!("Expected Complete after Promise resolution");
    };
    assert_eq!(*value, JsValue::String("from-callback".into()));
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

                // This function gets a Promise from host and attaches .then() callback that
                // accesses BOTH function-local variable AND module variable
                export function wrapWithThen(userCallback: () => void): Promise<void> {
                    const functionLocal = "function-local";

                    const promise = __order__({ type: "getPromise" });
                    return promise.then(() => {
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

    // First suspension: __order__ waiting for host response
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for __order__");
    };
    assert_eq!(pending.len(), 1);

    // Host returns a Promise
    let promise = runtime.create_promise();
    let result2 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(promise.value().clone())),
        }])
        .unwrap();

    // Second suspension: awaiting the Promise
    let RuntimeResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Resolve the Promise - triggers .then() callback
    let result3 = runtime
        .resolve_promise(&promise, RuntimeValue::unguarded(JsValue::Undefined))
        .unwrap();

    let RuntimeResult::Complete(value) = result3 else {
        panic!("Expected Complete after Promise resolution");
    };
    assert_eq!(
        *value,
        JsValue::String("from-then / user-callback-ran".into())
    );
}

#[test]
fn test_debug_closure_gc() {
    // Test GC with closures accessing local and module variables
    let config = RuntimeConfig {
        internal_modules: vec![create_eval_internal_module()],
        timeout_ms: 5000,
    };
    let mut runtime = Runtime::with_config(config);
    runtime.set_gc_threshold(1);

    // Test that callback closure survives GC with local variables
    let result = runtime
        .eval(
            r#"
            import { __order__ } from "eval:internal";

            let state: string = "initial";

            // Wrapper function WITH a local variable
            function wrapper(): Promise<void> {
                const local = "local";  // This triggers call env creation
                const promise = __order__({ type: "getPromise" });
                return promise.then(() => {
                    state = "modified-" + local;
                });
            }

            await wrapper();
            state;
        "#,
        )
        .expect("eval should work");

    // First suspension: __order__ waiting for host response
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for __order__");
    };

    // Host returns a Promise
    let promise = runtime.create_promise();
    let result2 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(promise.value().clone())),
        }])
        .unwrap();

    // Second suspension: awaiting the Promise
    let RuntimeResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Resolve the Promise - triggers .then() callback with closure
    let result3 = runtime
        .resolve_promise(&promise, RuntimeValue::unguarded(JsValue::Undefined))
        .unwrap();

    let RuntimeResult::Complete(value) = result3 else {
        panic!("Expected Complete after Promise resolution");
    };
    assert_eq!(*value, JsValue::String("modified-local".into()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// sleep() Tests (blocking delay using orders)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_sleep_basic() {
    let mut runtime = create_test_runtime();

    // Use blocking sleep() which suspends for the host to handle
    let result = run_with_globals(
        &mut runtime,
        r#"
        let result = "before";
        sleep(100);
        result = "after";
        result;
    "#,
    );

    // Should suspend for sleep()
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for sleep()");
    };
    assert_eq!(pending.len(), 1);

    // Verify the order payload
    let payload = pending[0].payload.value();
    assert_eq!(get_string_prop(payload, "type"), Some("sleep".into()));

    // Fulfill the order (host would wait the delay then respond)
    let result2 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
        }])
        .unwrap();

    let RuntimeResult::Complete(value) = result2 else {
        panic!("Expected Complete after sleep");
    };
    assert_eq!(*value, JsValue::String("after".into()));
}

#[test]
fn test_sleep_sequential() {
    let mut runtime = create_test_runtime();

    // Multiple sequential sleep() calls
    let result = run_with_globals(
        &mut runtime,
        r#"
        let count = 0;
        sleep(10);
        count += 1;
        sleep(20);
        count += 1;
        sleep(30);
        count += 1;
        count;
    "#,
    );

    // First sleep
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first sleep");
    };
    assert_eq!(pending.len(), 1);

    let result = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
        }])
        .unwrap();

    // Second sleep
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for second sleep");
    };
    assert_eq!(pending.len(), 1);

    let result = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
        }])
        .unwrap();

    // Third sleep
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for third sleep");
    };
    assert_eq!(pending.len(), 1);

    let result = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
        }])
        .unwrap();

    // Complete
    let RuntimeResult::Complete(value) = result else {
        panic!("Expected Complete after all sleeps");
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

// ═══════════════════════════════════════════════════════════════════════════════
// Host Promise API Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_host_create_and_resolve_promise() {
    // Test that host can create a Promise and resolve it later
    let mut runtime = create_test_runtime();

    // Create an unresolved promise from the host
    let host_promise = runtime.create_promise();

    // The script must await the returned Promise separately
    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        // __order__ returns a PendingOrder, await it to get the host's Promise
        const promise = await __order__({ type: "getHostPromise" });
        // Then await the Promise to get the actual value
        const result = await promise;
        "Got: " + result;
    "#,
    );

    // First suspension: waiting for order
    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended waiting for order");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "type"),
        Some("getHostPromise".into())
    );

    // Fulfill with the unresolved promise - use the value directly
    let result2 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(host_promise.value().clone())),
        }])
        .unwrap();

    // Still suspended - waiting for the Promise to be resolved
    let RuntimeResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise to be resolved");
    };

    // Now resolve the promise with a value
    let value = RuntimeValue::unguarded(JsValue::String("Hello from host!".into()));
    let result3 = runtime.resolve_promise(&host_promise, value).unwrap();

    // Should complete now
    let RuntimeResult::Complete(final_value) = result3 else {
        panic!("Expected Complete after resolving Promise");
    };
    assert_eq!(*final_value, JsValue::String("Got: Hello from host!".into()));
}

#[test]
fn test_host_create_and_reject_promise() {
    // Test that host can create a Promise and reject it later
    let mut runtime = create_test_runtime();

    let host_promise = runtime.create_promise();

    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        try {
            // Get the Promise from host and await it
            const promise = await __order__({ type: "getHostPromise" });
            const result = await promise;
            "Success: " + result;
        } catch (e) {
            "Error: " + e;
        }
    "#,
    );

    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };

    // Fulfill with the unresolved promise
    let result2 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(host_promise.value().clone())),
        }])
        .unwrap();

    let RuntimeResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Reject the promise
    let reason = RuntimeValue::unguarded(JsValue::String("Something went wrong".into()));
    let result3 = runtime.reject_promise(&host_promise, reason).unwrap();

    let RuntimeResult::Complete(final_value) = result3 else {
        panic!("Expected Complete after rejecting Promise");
    };
    assert_eq!(
        *final_value,
        JsValue::String("Error: Something went wrong".into())
    );
}

#[test]
fn test_host_promise_immediate_resolve() {
    // Test resolving a Promise immediately before returning it
    let mut runtime = create_test_runtime();

    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        // Get the Promise from host and await it
        const promise = await __order__({ type: "quickResolve" });
        const result = await promise;
        "Result: " + result;
    "#,
    );

    let RuntimeResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };

    // Create promise and resolve it first
    let promise = runtime.create_promise();
    let value = RuntimeValue::unguarded(JsValue::Number(42.0));

    // Resolve the promise BEFORE returning it to the script
    // This is valid - the Promise is already fulfilled when returned
    let _ = runtime.resolve_promise(&promise, value);

    // Now fulfill the order with the already-resolved promise
    let result2 = runtime
        .fulfill_orders(vec![OrderResponse {
            id: pending[0].id,
            result: Ok(RuntimeValue::unguarded(promise.value().clone())),
        }])
        .unwrap();

    // Since the Promise was already resolved, await should complete immediately
    let RuntimeResult::Complete(final_value) = result2 else {
        panic!("Expected Complete");
    };
    assert_eq!(*final_value, JsValue::String("Result: 42".into()));
}
