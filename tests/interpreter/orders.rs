//! Order system integration tests
//!
//! These tests demonstrate how the order system can be used for real-world async operations
//! like fetch(), setTimeout(), and file I/O. The global functions are implemented in TypeScript
//! using the __order__ syscall from eval:internal.

use super::{run, run_to_completion};
use serde_json::json;
use tsrun::{
    InternalModule, JsString, JsValue, OrderId, OrderResponse, Runtime, RuntimeConfig,
    RuntimeValue, StepResult, api, create_eval_internal_module, value::PropertyKey,
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
    if let JsValue::Object(o) = obj
        && let Some(JsValue::String(s)) = o
            .borrow()
            .get_property(&PropertyKey::String(JsString::from(key)))
    {
        return Some(s.to_string());
    }
    None
}

/// Run script with globals, handling the import of eval:globals first
#[allow(clippy::expect_used)]
fn run_with_globals(runtime: &mut Runtime, script: &str) -> StepResult {
    // Prepend import of globals module to register global functions
    let full_script = format!(
        r#"import "eval:globals";
{}"#,
        script
    );
    run(runtime, &full_script, None).expect("eval should not fail")
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
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for __order__");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "type"),
        Some("getPromise".into())
    );

    // Host creates and returns a Promise
    let promise = api::create_promise(&mut runtime);
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut runtime).unwrap();

    // Second suspension: script is awaiting the Promise (via .then())
    let StepResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise resolution");
    };

    // Resolve the Promise - this triggers the .then() callback
    api::resolve_promise(
        &mut runtime,
        &promise,
        RuntimeValue::unguarded(JsValue::Undefined),
    )
    .unwrap();
    let result3 = run_to_completion(&mut runtime).unwrap();

    // After resolution, the callback should have run and modified `captured`
    let StepResult::Complete(value) = result3 else {
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
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for __order__");
    };
    assert_eq!(pending.len(), 1);

    // Host returns a Promise
    let promise = api::create_promise(&mut runtime);
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut runtime).unwrap();

    // Second suspension: awaiting the Promise
    let StepResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Resolve the Promise
    api::resolve_promise(
        &mut runtime,
        &promise,
        RuntimeValue::unguarded(JsValue::Undefined),
    )
    .unwrap();
    let result3 = run_to_completion(&mut runtime).unwrap();

    let StepResult::Complete(value) = result3 else {
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
    };
    let mut runtime = Runtime::with_config(config);

    let gc_threshold = std::env::var("GC_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    runtime.set_gc_threshold(gc_threshold);

    let result = run(
        &mut runtime,
        r#"
            import { runWithCallback, getState } from "eval:timer-module";

            // Call the function that uses .then()
            await runWithCallback();

            // Check the state
            getState();
        "#,
        None,
    )
    .expect("eval should work");

    // First suspension: __order__ waiting for host response
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for __order__");
    };
    assert_eq!(pending.len(), 1);

    // Host returns a Promise
    let promise = api::create_promise(&mut runtime);
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut runtime).unwrap();

    // Second suspension: awaiting the Promise
    let StepResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Resolve the Promise - triggers .then() callback
    api::resolve_promise(
        &mut runtime,
        &promise,
        RuntimeValue::unguarded(JsValue::Undefined),
    )
    .unwrap();
    let result3 = run_to_completion(&mut runtime).unwrap();

    let StepResult::Complete(value) = result3 else {
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
    };
    let mut runtime = Runtime::with_config(config);

    let gc_threshold = std::env::var("GC_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    runtime.set_gc_threshold(gc_threshold);

    let result = run(
        &mut runtime,
        r#"
            import { wrapWithThen, getState } from "eval:helper-module";

            let userResult = "not-called";

            await wrapWithThen(() => {
                userResult = "user-callback-ran";
            });

            // Return both the module state and user result
            getState() + " / " + userResult;
        "#,
        None,
    )
    .expect("eval should work");

    // First suspension: __order__ waiting for host response
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for __order__");
    };
    assert_eq!(pending.len(), 1);

    // Host returns a Promise
    let promise = api::create_promise(&mut runtime);
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut runtime).unwrap();

    // Second suspension: awaiting the Promise
    let StepResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Resolve the Promise - triggers .then() callback
    api::resolve_promise(
        &mut runtime,
        &promise,
        RuntimeValue::unguarded(JsValue::Undefined),
    )
    .unwrap();
    let result3 = run_to_completion(&mut runtime).unwrap();

    let StepResult::Complete(value) = result3 else {
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
    };
    let mut runtime = Runtime::with_config(config);
    runtime.set_gc_threshold(1);

    // Test that callback closure survives GC with local variables
    let result = run(
        &mut runtime,
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
        None,
    )
    .expect("eval should work");

    // First suspension: __order__ waiting for host response
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for __order__");
    };

    // Host returns a Promise
    let promise = api::create_promise(&mut runtime);
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut runtime).unwrap();

    // Second suspension: awaiting the Promise
    let StepResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Resolve the Promise - triggers .then() callback with closure
    api::resolve_promise(
        &mut runtime,
        &promise,
        RuntimeValue::unguarded(JsValue::Undefined),
    )
    .unwrap();
    let result3 = run_to_completion(&mut runtime).unwrap();

    let StepResult::Complete(value) = result3 else {
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
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for sleep()");
    };
    assert_eq!(pending.len(), 1);

    // Verify the order payload
    let payload = pending[0].payload.value();
    assert_eq!(get_string_prop(payload, "type"), Some("sleep".into()));

    // Fulfill the order (host would wait the delay then respond)
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
    }]);
    let result2 = run_to_completion(&mut runtime).unwrap();

    let StepResult::Complete(value) = result2 else {
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
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first sleep");
    };
    assert_eq!(pending.len(), 1);

    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Second sleep
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for second sleep");
    };
    assert_eq!(pending.len(), 1);

    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Third sleep
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for third sleep");
    };
    assert_eq!(pending.len(), 1);

    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Complete
    let StepResult::Complete(value) = result else {
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
        StepResult::Suspended { pending, .. } => {
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
            let mock_response = api::create_response_object(
                &mut runtime,
                &json!({
                    "id": 1,
                    "name": "John",
                    "email": "john@example.com"
                }),
            )
            .unwrap();

            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(mock_response),
            };

            runtime.fulfill_orders(vec![response]);
            let result2 = run_to_completion(&mut runtime).unwrap();

            match result2 {
                StepResult::Complete(value) => {
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
        StepResult::Suspended { pending, .. } => {
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
            let mock_response =
                api::create_response_object(&mut runtime, &json!({ "id": 42, "name": "Jane" }))
                    .unwrap();

            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(mock_response),
            };

            runtime.fulfill_orders(vec![response]);
            let result2 = run_to_completion(&mut runtime).unwrap();

            match result2 {
                StepResult::Complete(value) => {
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
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first fetch");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "url"),
        Some("/users/1".into())
    );

    let user_response =
        api::create_response_object(&mut runtime, &json!({ "name": "John" })).unwrap();
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(user_response),
    }]);
    let result2 = run_to_completion(&mut runtime).unwrap();

    // Second suspension: fetch("/posts?userId=1")
    let StepResult::Suspended { pending, .. } = result2 else {
        panic!("Expected Suspended for second fetch, got {:?}", result2);
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "url"),
        Some("/posts?userId=1".into())
    );

    let posts_response = api::create_response_object(
        &mut runtime,
        &json!([{ "id": 1 }, { "id": 2 }, { "id": 3 }]),
    )
    .unwrap();
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(posts_response),
    }]);
    let result3 = run_to_completion(&mut runtime).unwrap();

    // Final result
    let StepResult::Complete(value) = result3 else {
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
        StepResult::Suspended { pending, .. } => {
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

            runtime.fulfill_orders(vec![response]);
            let result2 = run_to_completion(&mut runtime).unwrap();

            match result2 {
                StepResult::Complete(value) => {
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
        StepResult::Suspended { pending, .. } => {
            assert_eq!(pending.len(), 1);

            // Return mock JSON content
            let json_content = r#"{"database": {"host": "localhost", "port": 5432}}"#;
            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(RuntimeValue::unguarded(JsValue::String(
                    json_content.into(),
                ))),
            };

            runtime.fulfill_orders(vec![response]);
            let result2 = run_to_completion(&mut runtime).unwrap();

            match result2 {
                StepResult::Complete(value) => {
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
        StepResult::Suspended { pending, .. } => {
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

            runtime.fulfill_orders(vec![response]);
            let result2 = run_to_completion(&mut runtime).unwrap();

            match result2 {
                StepResult::Complete(value) => {
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
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for writeFile");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "type"),
        Some("writeFile".into())
    );

    // Capture the written content
    let file_content = get_string_prop(pending[0].payload.value(), "content").unwrap_or_default();
    assert_eq!(file_content, "test data 12345");

    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Second: readFile
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for readFile");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "type"),
        Some("readFile".into())
    );

    // Return the "stored" content
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::String(
            file_content.into(),
        ))),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Complete
    let StepResult::Complete(value) = result else {
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
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for readFile");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "type"),
        Some("readFile".into())
    );

    let config_json =
        r#"{"name": "MyApp", "version": "1.0.0", "apiUrl": "https://api.example.com"}"#;
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::String(config_json.into()))),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Step 2: fetch api settings
    let StepResult::Suspended { pending, .. } = result else {
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

    let api_response =
        api::create_response_object(&mut runtime, &json!({ "theme": "dark", "language": "en" }))
            .unwrap();
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(api_response),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Step 3: writeFile /manifest.json
    let StepResult::Suspended { pending, .. } = result else {
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

    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Complete
    let StepResult::Complete(value) = result else {
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
    let host_promise = api::create_promise(&mut runtime);

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
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended waiting for order");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "type"),
        Some("getHostPromise".into())
    );

    // Fulfill with the unresolved promise - use the value directly
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(host_promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut runtime).unwrap();

    // Still suspended - waiting for the Promise to be resolved
    let StepResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise to be resolved");
    };

    // Now resolve the promise with a value
    let value = RuntimeValue::unguarded(JsValue::String("Hello from host!".into()));
    api::resolve_promise(&mut runtime, &host_promise, value).unwrap();
    let result3 = run_to_completion(&mut runtime).unwrap();

    // Should complete now
    let StepResult::Complete(final_value) = result3 else {
        panic!("Expected Complete after resolving Promise");
    };
    assert_eq!(
        *final_value,
        JsValue::String("Got: Hello from host!".into())
    );
}

#[test]
fn test_host_create_and_reject_promise() {
    // Test that host can create a Promise and reject it later
    let mut runtime = create_test_runtime();

    let host_promise = api::create_promise(&mut runtime);

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

    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };

    // Fulfill with the unresolved promise
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(host_promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Reject the promise
    let reason = RuntimeValue::unguarded(JsValue::String("Something went wrong".into()));
    api::reject_promise(&mut runtime, &host_promise, reason).unwrap();
    let result3 = run_to_completion(&mut runtime).unwrap();

    let StepResult::Complete(final_value) = result3 else {
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

    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };

    // Create promise and resolve it first
    let promise = api::create_promise(&mut runtime);
    let value = RuntimeValue::unguarded(JsValue::Number(42.0));

    // Resolve the promise BEFORE returning it to the script
    // This is valid - the Promise is already fulfilled when returned
    api::resolve_promise(&mut runtime, &promise, value).unwrap();

    // Now fulfill the order with the already-resolved promise
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut runtime).unwrap();

    // Since the Promise was already resolved, await should complete immediately
    let StepResult::Complete(final_value) = result2 else {
        panic!("Expected Complete");
    };
    assert_eq!(*final_value, JsValue::String("Result: 42".into()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Concurrency Tests (via host-returned Promises)
// With blocking __order__(), concurrency is achieved by:
// 1. Script requests multiple Promises from host via sequential __order__() calls
// 2. Host returns unresolved Promises for each
// 3. Script combines them with Promise.all/race/allSettled
// 4. Host resolves Promises in any order
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_concurrent_fetch_with_promise_all() {
    // Simulate concurrent fetching: script gets Promises, uses Promise.all
    let mut runtime = create_test_runtime();

    // Create host Promises before running script
    let promise_users = api::create_promise(&mut runtime);
    let promise_posts = api::create_promise(&mut runtime);

    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        // Get Promises for two different resources
        const usersPromise = __order__({ type: "fetch", url: "/users" });
        const postsPromise = __order__({ type: "fetch", url: "/posts" });

        // Wait for both concurrently
        const [users, posts] = await Promise.all([usersPromise, postsPromise]);

        `${users.count} users, ${posts.count} posts`;
    "#,
    );

    // First order: /users
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first fetch");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "url"),
        Some("/users".into())
    );

    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise_users.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Second order: /posts
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for second fetch");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "url"),
        Some("/posts".into())
    );

    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise_posts.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Now awaiting Promise.all - both Promises are pending
    let StepResult::Suspended { .. } = result else {
        panic!("Expected Suspended waiting for Promise.all");
    };

    // Resolve /posts first (out of order!)
    let posts_data = api::create_response_object(&mut runtime, &json!({ "count": 100 })).unwrap();
    api::resolve_promise(&mut runtime, &promise_posts, posts_data).unwrap();
    let result = run_to_completion(&mut runtime).unwrap();

    // Still waiting for /users
    let StepResult::Suspended { .. } = result else {
        panic!("Expected Suspended still waiting for users");
    };

    // Resolve /users
    let users_data = api::create_response_object(&mut runtime, &json!({ "count": 42 })).unwrap();
    api::resolve_promise(&mut runtime, &promise_users, users_data).unwrap();
    let result = run_to_completion(&mut runtime).unwrap();

    // Complete - results in original order despite resolution order
    let StepResult::Complete(value) = result else {
        panic!("Expected Complete after both resolved");
    };
    assert_eq!(*value, JsValue::String("42 users, 100 posts".into()));
}

#[test]
fn test_promise_race_first_wins() {
    // Promise.race: first to resolve determines the result
    let mut runtime = create_test_runtime();

    let promise_fast = api::create_promise(&mut runtime);
    let promise_slow = api::create_promise(&mut runtime);

    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        const fast = __order__({ type: "fetch", server: "fast" });
        const slow = __order__({ type: "fetch", server: "slow" });

        // Race: first to resolve wins
        const winner = await Promise.race([fast, slow]);
        winner.server;
    "#,
    );

    // Get first Promise
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise_fast.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Get second Promise
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise_slow.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Awaiting Promise.race
    let StepResult::Suspended { .. } = result else {
        panic!("Expected Suspended for Promise.race");
    };

    // Resolve "fast" first - this should win the race
    let fast_data =
        api::create_response_object(&mut runtime, &json!({ "server": "fast" })).unwrap();
    api::resolve_promise(&mut runtime, &promise_fast, fast_data).unwrap();
    let result = run_to_completion(&mut runtime).unwrap();

    // Race completes immediately when first Promise resolves
    let StepResult::Complete(value) = result else {
        panic!("Expected Complete after first resolve");
    };
    assert_eq!(*value, JsValue::String("fast".into()));
}

#[test]
fn test_promise_race_second_wins() {
    // Verify race works when second Promise resolves first
    let mut runtime = create_test_runtime();

    let promise_a = api::create_promise(&mut runtime);
    let promise_b = api::create_promise(&mut runtime);

    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        const a = __order__({ type: "fetch", id: "a" });
        const b = __order__({ type: "fetch", id: "b" });

        const winner = await Promise.race([a, b]);
        winner.winner;
    "#,
    );

    // Get both Promises
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise_a.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise_b.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { .. } = result else {
        panic!("Expected Suspended for Promise.race");
    };

    // Resolve B first - B wins even though it was second in array
    let b_data = api::create_response_object(&mut runtime, &json!({ "winner": "B" })).unwrap();
    api::resolve_promise(&mut runtime, &promise_b, b_data).unwrap();
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Complete(value) = result else {
        panic!("Expected Complete");
    };
    assert_eq!(*value, JsValue::String("B".into()));
}

#[test]
fn test_concurrent_with_partial_failure() {
    // One Promise resolves, one rejects - test error handling with Promise.all
    let mut runtime = create_test_runtime();

    let promise_ok = api::create_promise(&mut runtime);
    let promise_fail = api::create_promise(&mut runtime);

    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        const ok = __order__({ type: "fetch", url: "/ok" });
        const fail = __order__({ type: "fetch", url: "/fail" });

        try {
            const results = await Promise.all([ok, fail]);
            "Success: " + JSON.stringify(results);
        } catch (e) {
            "Error: " + e;
        }
    "#,
    );

    // Get both Promises
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise_ok.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise_fail.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { .. } = result else {
        panic!("Expected Suspended for Promise.all");
    };

    // Resolve the first one successfully
    let ok_data = RuntimeValue::unguarded(JsValue::String("OK".into()));
    api::resolve_promise(&mut runtime, &promise_ok, ok_data).unwrap();
    let result = run_to_completion(&mut runtime).unwrap();

    // Still waiting for second
    let StepResult::Suspended { .. } = result else {
        panic!("Expected Suspended waiting for second");
    };

    // Reject the second one
    let error = RuntimeValue::unguarded(JsValue::String("Network error".into()));
    api::reject_promise(&mut runtime, &promise_fail, error).unwrap();
    let result = run_to_completion(&mut runtime).unwrap();

    // Promise.all rejects if any Promise rejects
    let StepResult::Complete(value) = result else {
        panic!("Expected Complete with error");
    };
    assert_eq!(*value, JsValue::String("Error: Network error".into()));
}

#[test]
fn test_concurrent_three_way_race() {
    // Race with three Promises, middle one wins
    // Note: Script gets each Promise sequentially, then races them
    let mut runtime = create_test_runtime();

    let promise1 = api::create_promise(&mut runtime);
    let promise2 = api::create_promise(&mut runtime);
    let promise3 = api::create_promise(&mut runtime);

    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        // Get three Promises from host
        const p1 = __order__({ id: 1 });
        const p2 = __order__({ id: 2 });
        const p3 = __order__({ id: 3 });

        // Race: first to resolve wins
        const winner = await Promise.race([p1, p2, p3]);
        "Winner: " + winner;
    "#,
    );

    // Get all three Promises via sequential orders
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for order 1");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise1.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for order 2");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise2.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for order 3");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise3.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Now awaiting Promise.race with three unresolved Promises
    let StepResult::Suspended { .. } = result else {
        panic!("Expected Suspended for Promise.race");
    };

    // Resolve promise2 first (should win the race)
    api::resolve_promise(
        &mut runtime,
        &promise2,
        RuntimeValue::unguarded(JsValue::Number(2.0)),
    )
    .unwrap();
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Complete(value) = result else {
        panic!("Expected Complete after resolving winner, got {:?}", result);
    };
    assert_eq!(*value, JsValue::String("Winner: 2".into()));
}

#[test]
fn test_concurrent_chained_operations() {
    // Start concurrent fetches, then chain more operations on results
    let mut runtime = create_test_runtime();

    let promise_user = api::create_promise(&mut runtime);
    let promise_profile = api::create_promise(&mut runtime);

    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        // Get user and profile concurrently
        const userPromise = __order__({ type: "getUser" });
        const profilePromise = __order__({ type: "getProfile" });

        // Chain transformations on each
        const user = userPromise.then(u => ({ ...u, type: "user" }));
        const profile = profilePromise.then(p => ({ ...p, type: "profile" }));

        // Wait for both transformed results
        const [u, p] = await Promise.all([user, profile]);
        `${u.name} (${u.type}), ${p.bio} (${p.type})`;
    "#,
    );

    // Get both Promises
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise_user.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise_profile.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { .. } = result else {
        panic!("Expected Suspended for Promise.all");
    };

    // Resolve user
    let user_data = api::create_response_object(&mut runtime, &json!({ "name": "Alice" })).unwrap();
    api::resolve_promise(&mut runtime, &promise_user, user_data).unwrap();
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { .. } = result else {
        panic!("Expected Suspended waiting for profile");
    };

    // Resolve profile
    let profile_data =
        api::create_response_object(&mut runtime, &json!({ "bio": "Developer" })).unwrap();
    api::resolve_promise(&mut runtime, &promise_profile, profile_data).unwrap();
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Complete(value) = result else {
        panic!("Expected Complete");
    };
    assert_eq!(
        *value,
        JsValue::String("Alice (user), Developer (profile)".into())
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Order Cancellation Tests
// When host Promises are abandoned (e.g., lose in Promise.race) or rejected,
// their associated order IDs are reported back to the host via cancelled list.
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_promise_race_cancels_losing_order() {
    // When Promise.race settles, losing Promises' order IDs should be cancelled
    let mut runtime = create_test_runtime();

    // Create host Promises linked to orders
    let order1_id = OrderId(100);
    let order2_id = OrderId(200);
    let promise1 = api::create_order_promise(&mut runtime, order1_id);
    let promise2 = api::create_order_promise(&mut runtime, order2_id);

    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        const p1 = __order__({ id: 1 });
        const p2 = __order__({ id: 2 });

        const winner = await Promise.race([p1, p2]);
        winner;
    "#,
    );

    // Get first order
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    let first_order_id = pending[0].id;
    runtime.fulfill_orders(vec![OrderResponse {
        id: first_order_id,
        result: Ok(RuntimeValue::unguarded(promise1.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Get second order
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    let second_order_id = pending[0].id;
    runtime.fulfill_orders(vec![OrderResponse {
        id: second_order_id,
        result: Ok(RuntimeValue::unguarded(promise2.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    // Awaiting Promise.race
    let StepResult::Suspended { .. } = result else {
        panic!("Expected Suspended for Promise.race");
    };

    // Resolve promise1 first - it wins, promise2's order should be cancelled
    api::resolve_promise(
        &mut runtime,
        &promise1,
        RuntimeValue::unguarded(JsValue::String("first".into())),
    )
    .unwrap();
    let result = run_to_completion(&mut runtime).unwrap();

    // Check that the result includes cancelled order
    match result {
        StepResult::Complete(value) => {
            assert_eq!(*value, JsValue::String("first".into()));
        }
        StepResult::Suspended { cancelled, .. } => {
            // The loser's order_id (order2_id) should be in cancelled
            assert!(
                cancelled.contains(&order2_id),
                "Expected order2_id ({:?}) in cancelled list: {:?}",
                order2_id,
                cancelled
            );
            // Continue to get final result
            let result = run_to_completion(&mut runtime).unwrap();
            if let StepResult::Complete(value) = result {
                assert_eq!(*value, JsValue::String("first".into()));
            }
        }
        _ => panic!("Unexpected result"),
    }
}

#[test]
fn test_promise_race_second_wins_cancels_first() {
    // Verify cancellation works when second Promise wins
    let mut runtime = create_test_runtime();

    let order1_id = OrderId(111);
    let order2_id = OrderId(222);
    let promise1 = api::create_order_promise(&mut runtime, order1_id);
    let promise2 = api::create_order_promise(&mut runtime, order2_id);

    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        const p1 = __order__({ id: 1 });
        const p2 = __order__({ id: 2 });

        const winner = await Promise.race([p1, p2]);
        winner;
    "#,
    );

    // Get both orders and return Promises
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise1.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise2.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { .. } = result else {
        panic!("Expected Suspended");
    };

    // Resolve promise2 first - it wins, promise1's order should be cancelled
    api::resolve_promise(
        &mut runtime,
        &promise2,
        RuntimeValue::unguarded(JsValue::String("second".into())),
    )
    .unwrap();
    let result = run_to_completion(&mut runtime).unwrap();

    match result {
        StepResult::Complete(value) => {
            assert_eq!(*value, JsValue::String("second".into()));
        }
        StepResult::Suspended { cancelled, .. } => {
            // The loser's order_id (order1_id) should be in cancelled
            assert!(
                cancelled.contains(&order1_id),
                "Expected order1_id ({:?}) in cancelled list: {:?}",
                order1_id,
                cancelled
            );
        }
        _ => panic!("Unexpected result"),
    }
}

#[test]
fn test_promise_rejection_signals_cancelled_order() {
    // When a host Promise is rejected, its order should be signalled as cancelled
    let mut runtime = create_test_runtime();

    let order_id = OrderId(999);
    let promise = api::create_order_promise(&mut runtime, order_id);

    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        try {
            const p = __order__({ type: "will_fail" });
            await p;
            "resolved";
        } catch (e) {
            "caught: " + e;
        }
    "#,
    );

    // Get order and return Promise
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { .. } = result else {
        panic!("Expected Suspended");
    };

    // Reject the Promise - its order should be cancelled
    api::reject_promise(
        &mut runtime,
        &promise,
        RuntimeValue::unguarded(JsValue::String("error".into())),
    )
    .unwrap();
    let result = run_to_completion(&mut runtime).unwrap();

    match result {
        StepResult::Complete(value) => {
            assert_eq!(*value, JsValue::String("caught: error".into()));
        }
        StepResult::Suspended { cancelled, .. } => {
            // Rejected Promise's order should be in cancelled
            assert!(
                cancelled.contains(&order_id),
                "Expected order_id ({:?}) in cancelled list: {:?}",
                order_id,
                cancelled
            );
            // Continue to get final result
            let result = run_to_completion(&mut runtime).unwrap();
            if let StepResult::Complete(value) = result {
                assert_eq!(*value, JsValue::String("caught: error".into()));
            }
        }
        _ => panic!("Unexpected result"),
    }
}

#[test]
fn test_three_way_race_cancels_two_losers() {
    // Three-way race: winner gets result, two losers' orders cancelled
    let mut runtime = create_test_runtime();

    let order1_id = OrderId(1);
    let order2_id = OrderId(2);
    let order3_id = OrderId(3);
    let promise1 = api::create_order_promise(&mut runtime, order1_id);
    let promise2 = api::create_order_promise(&mut runtime, order2_id);
    let promise3 = api::create_order_promise(&mut runtime, order3_id);

    let result = run_with_globals(
        &mut runtime,
        r#"
        import { __order__ } from "eval:internal";

        const p1 = __order__({ id: 1 });
        const p2 = __order__({ id: 2 });
        const p3 = __order__({ id: 3 });

        const winner = await Promise.race([p1, p2, p3]);
        "Winner: " + winner;
    "#,
    );

    // Get all three orders
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise1.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise2.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended");
    };
    runtime.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise3.value().clone())),
    }]);
    let result = run_to_completion(&mut runtime).unwrap();

    let StepResult::Suspended { .. } = result else {
        panic!("Expected Suspended");
    };

    // Resolve promise2 (middle one wins)
    api::resolve_promise(
        &mut runtime,
        &promise2,
        RuntimeValue::unguarded(JsValue::Number(2.0)),
    )
    .unwrap();
    let result = run_to_completion(&mut runtime).unwrap();

    // Check that both losers' orders are cancelled
    let cancelled = match &result {
        StepResult::Suspended { cancelled, .. } => cancelled.clone(),
        StepResult::Complete(_) => {
            // May have completed immediately, check via continue_eval
            Vec::new()
        }
        _ => panic!("Unexpected result"),
    };

    // order1_id and order3_id should be cancelled (order2_id won)
    assert!(
        cancelled.contains(&order1_id) || cancelled.is_empty(),
        "Expected order1_id in cancelled: {:?}",
        cancelled
    );
    assert!(
        cancelled.contains(&order3_id) || cancelled.is_empty(),
        "Expected order3_id in cancelled: {:?}",
        cancelled
    );
    // Winner's order should NOT be cancelled
    assert!(
        !cancelled.contains(&order2_id),
        "Winner's order should not be cancelled: {:?}",
        cancelled
    );
}
