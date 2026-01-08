//! Order system integration tests
//!
//! These tests demonstrate how the order system can be used for real-world async operations
//! like fetch(), setTimeout(), and file I/O. The global functions are implemented in TypeScript
//! using the __order__ syscall from eval:internal.

use super::{run, run_to_completion};
use serde_json::json;
use tsrun::{
    InternalModule, Interpreter, InterpreterConfig, JsString, JsValue, OrderId, OrderResponse,
    RuntimeValue, StepResult, api, create_eval_internal_module, value::PropertyKey,
};

// ═══════════════════════════════════════════════════════════════════════════════
// TypeScript Globals Source
// ═══════════════════════════════════════════════════════════════════════════════

/// TypeScript source that defines global functions using the order system.
/// This demonstrates how hosts can extend the interpreter with custom async operations.
///
/// With blocking __order__() semantics:
/// - __order__() suspends immediately, host provides any value
/// - Host can return plain values or Promises
/// - For parallel operations, host returns Promises and resolves them later
const GLOBALS_SOURCE: &str = r#"
import { __order__ } from "eval:internal";

// sleep(ms) - Returns Promise that resolves after delay
globalThis.sleep = async function(ms: number): Promise<void> {
    await __order__({ type: "sleep", delay: ms });
};

// fetch(url, options?) - Returns Promise with response
globalThis.fetch = async function(url: string, options?: {
    method?: string;
    body?: string;
    headers?: Record<string, string>;
}): Promise<any> {
    return await __order__({
        type: "fetch",
        url: url,
        method: options?.method || "GET",
        body: options?.body,
        headers: options?.headers
    });
};

// readFile(path) - Returns Promise with file content
globalThis.readFile = async function(path: string): Promise<string> {
    return await __order__({ type: "readFile", path: path });
};

// writeFile(path, content) - Returns Promise when complete
globalThis.writeFile = async function(path: string, content: string): Promise<string> {
    return await __order__({ type: "writeFile", path: path, content: content });
};
"#;

// ═══════════════════════════════════════════════════════════════════════════════
// Test Helpers
// ═══════════════════════════════════════════════════════════════════════════════

/// Create a runtime with eval:internal and eval:globals modules
fn create_test_interp() -> Interpreter {
    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let interp = Interpreter::with_config(config);

    // Set aggressive GC for testing
    let gc_threshold = std::env::var("GC_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    interp.set_gc_threshold(gc_threshold);

    interp
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

/// Extract number property from JsValue object
fn get_number_prop(obj: &JsValue, key: &str) -> Option<f64> {
    if let JsValue::Object(o) = obj
        && let Some(JsValue::Number(n)) = o
            .borrow()
            .get_property(&PropertyKey::String(JsString::from(key)))
    {
        return Some(n);
    }
    None
}

/// Run script with globals, handling the import of eval:globals first
#[allow(clippy::expect_used)]
fn run_with_globals(interp: &mut Interpreter, script: &str) -> StepResult {
    // Prepend import of globals module to register global functions
    let full_script = format!(
        r#"import "eval:globals";
{}"#,
        script
    );
    run(interp, &full_script, None).expect("eval should not fail")
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
    let mut interp = create_test_interp();

    // Test that .then() callbacks can access closure variables after Promise resolution
    // Host returns a Promise, script attaches .then() callback, host resolves Promise
    let result = run_with_globals(
        &mut interp,
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
    let promise = api::create_promise(&mut interp);
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut interp).unwrap();

    // Second suspension: script is awaiting the Promise (via .then())
    let StepResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise resolution");
    };

    // Resolve the Promise - this triggers the .then() callback
    api::resolve_promise(
        &mut interp,
        &promise,
        RuntimeValue::unguarded(JsValue::Undefined),
    )
    .unwrap();
    let result3 = run_to_completion(&mut interp).unwrap();

    // After resolution, the callback should have run and modified `captured`
    let StepResult::Complete(value) = result3 else {
        panic!("Expected Complete after Promise resolution");
    };
    assert_eq!(*value, JsValue::String("modified".into()));
}

#[test]
fn test_promise_then_callback_nested_closure() {
    let mut interp = create_test_interp();

    // Test that .then() callbacks can access module-level variables from inside
    // a function call (nested closure)
    let result = run_with_globals(
        &mut interp,
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
    let promise = api::create_promise(&mut interp);
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut interp).unwrap();

    // Second suspension: awaiting the Promise
    let StepResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Resolve the Promise
    api::resolve_promise(
        &mut interp,
        &promise,
        RuntimeValue::unguarded(JsValue::Undefined),
    )
    .unwrap();
    let result3 = run_to_completion(&mut interp).unwrap();

    let StepResult::Complete(value) = result3 else {
        panic!("Expected Complete after Promise resolution");
    };
    assert_eq!(*value, JsValue::String("modified".into()));
}

#[test]
fn test_cross_module_closure_simple() {
    // Test that callbacks from another module can access that module's variables
    // __order__ returns a Promise immediately, host fulfills with a value
    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source(
                "eval:timer-module",
                r#"
                import { __order__ } from "eval:internal";

                // Module-level variable
                let moduleState: string = "initial";

                // Function that gets a Promise from __order__ and attaches .then() callback
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
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);

    let gc_threshold = std::env::var("GC_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    interp.set_gc_threshold(gc_threshold);

    let result = run(
        &mut interp,
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

    // Suspension: __order__ waiting for host response
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for __order__");
    };
    assert_eq!(pending.len(), 1);

    // Host returns a Promise (since script calls .then() on the result)
    let promise = api::create_promise(&mut interp);
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut interp).unwrap();

    // Second suspension: awaiting the Promise (via .then())
    let StepResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Resolve the Promise to trigger the .then() callback
    api::resolve_promise(
        &mut interp,
        &promise,
        RuntimeValue::unguarded(JsValue::Undefined),
    )
    .unwrap();
    let result3 = run_to_completion(&mut interp).unwrap();

    // The .then() callback should have run and modified `moduleState`
    let StepResult::Complete(value) = result3 else {
        panic!("Expected Complete after Promise resolution");
    };
    assert_eq!(*value, JsValue::String("from-callback".into()));
}

#[test]
fn test_cross_module_nested_closure() {
    // Test that a callback defined in one module can access a variable from an outer
    // function's closure, where that function is defined in another module
    // __order__ returns a Promise immediately, host fulfills with a value
    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source(
                "eval:helper-module",
                r#"
                import { __order__ } from "eval:internal";

                // Module-level state
                let moduleState: string = "module-initial";

                // This function gets a Promise from __order__ and attaches .then() callback that
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
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);

    let gc_threshold = std::env::var("GC_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    interp.set_gc_threshold(gc_threshold);

    let result = run(
        &mut interp,
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

    // Suspension: __order__ waiting for host response
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for __order__");
    };
    assert_eq!(pending.len(), 1);

    // Host returns a Promise (since script calls .then() on the result)
    let promise = api::create_promise(&mut interp);
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut interp).unwrap();

    // Second suspension: awaiting the Promise (via .then())
    let StepResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Resolve the Promise to trigger the .then() callback
    api::resolve_promise(
        &mut interp,
        &promise,
        RuntimeValue::unguarded(JsValue::Undefined),
    )
    .unwrap();
    let result3 = run_to_completion(&mut interp).unwrap();

    // The .then() callback should have run
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
    // __order__ returns a Promise immediately, host fulfills with a value
    let config = InterpreterConfig {
        internal_modules: vec![create_eval_internal_module()],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    // Test that callback closure survives GC with local variables
    let result = run(
        &mut interp,
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

    // Suspension: __order__ waiting for host response
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for __order__");
    };

    // Host returns a Promise (since script calls .then() on the result)
    let promise = api::create_promise(&mut interp);
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut interp).unwrap();

    // Second suspension: awaiting the Promise (via .then())
    let StepResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Resolve the Promise to trigger the .then() callback
    api::resolve_promise(
        &mut interp,
        &promise,
        RuntimeValue::unguarded(JsValue::Undefined),
    )
    .unwrap();
    let result3 = run_to_completion(&mut interp).unwrap();

    // The .then() callback should have run with the closure
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
    let mut interp = create_test_interp();

    // Use async sleep() which returns a Promise
    let result = run_with_globals(
        &mut interp,
        r#"
        let result = "before";
        await sleep(100);
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
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
    }]);
    let result2 = run_to_completion(&mut interp).unwrap();

    let StepResult::Complete(value) = result2 else {
        panic!("Expected Complete after sleep");
    };
    assert_eq!(*value, JsValue::String("after".into()));
}

#[test]
fn test_sleep_sequential() {
    let mut interp = create_test_interp();

    // Multiple sequential await sleep() calls
    let result = run_with_globals(
        &mut interp,
        r#"
        let count = 0;
        await sleep(10);
        count += 1;
        await sleep(20);
        count += 1;
        await sleep(30);
        count += 1;
        count;
    "#,
    );

    // First sleep
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first sleep");
    };
    assert_eq!(pending.len(), 1);

    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Second sleep
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for second sleep");
    };
    assert_eq!(pending.len(), 1);

    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Third sleep
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for third sleep");
    };
    assert_eq!(pending.len(), 1);

    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

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
    let mut interp = create_test_interp();

    let result = run_with_globals(
        &mut interp,
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
                &mut interp,
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

            interp.fulfill_orders(vec![response]);
            let result2 = run_to_completion(&mut interp).unwrap();

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
    let mut interp = create_test_interp();

    let result = run_with_globals(
        &mut interp,
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
                api::create_response_object(&mut interp, &json!({ "id": 42, "name": "Jane" }))
                    .unwrap();

            let response = OrderResponse {
                id: pending[0].id,
                result: Ok(mock_response),
            };

            interp.fulfill_orders(vec![response]);
            let result2 = run_to_completion(&mut interp).unwrap();

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
    let mut interp = create_test_interp();

    // Simplified test - two sequential awaits instead of Promise.all
    let result = run_with_globals(
        &mut interp,
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
        api::create_response_object(&mut interp, &json!({ "name": "John" })).unwrap();
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(user_response),
    }]);
    let result2 = run_to_completion(&mut interp).unwrap();

    // Second suspension: fetch("/posts?userId=1")
    let StepResult::Suspended { pending, .. } = result2 else {
        panic!("Expected Suspended for second fetch, got {:?}", result2);
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "url"),
        Some("/posts?userId=1".into())
    );

    let posts_response =
        api::create_response_object(&mut interp, &json!([{ "id": 1 }, { "id": 2 }, { "id": 3 }]))
            .unwrap();
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(posts_response),
    }]);
    let result3 = run_to_completion(&mut interp).unwrap();

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
    let mut interp = create_test_interp();

    let result = run_with_globals(
        &mut interp,
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

            interp.fulfill_orders(vec![response]);
            let result2 = run_to_completion(&mut interp).unwrap();

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
    let mut interp = create_test_interp();

    let result = run_with_globals(
        &mut interp,
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

            interp.fulfill_orders(vec![response]);
            let result2 = run_to_completion(&mut interp).unwrap();

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
    let mut interp = create_test_interp();

    let result = run_with_globals(
        &mut interp,
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

            interp.fulfill_orders(vec![response]);
            let result2 = run_to_completion(&mut interp).unwrap();

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
    let mut interp = create_test_interp();

    let result = run_with_globals(
        &mut interp,
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

    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

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
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::String(
            file_content.into(),
        ))),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

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
    let mut interp = create_test_interp();

    let result = run_with_globals(
        &mut interp,
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
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::String(config_json.into()))),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

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
        api::create_response_object(&mut interp, &json!({ "theme": "dark", "language": "en" }))
            .unwrap();
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(api_response),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

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

    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(JsValue::Undefined)),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

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
    let mut interp = create_test_interp();

    // Create an unresolved promise from the host
    let host_promise = api::create_promise(&mut interp);

    // The script must await the returned Promise separately
    let result = run_with_globals(
        &mut interp,
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
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(host_promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut interp).unwrap();

    // Still suspended - waiting for the Promise to be resolved
    let StepResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise to be resolved");
    };

    // Now resolve the promise with a value
    let value = RuntimeValue::unguarded(JsValue::String("Hello from host!".into()));
    api::resolve_promise(&mut interp, &host_promise, value).unwrap();
    let result3 = run_to_completion(&mut interp).unwrap();

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
    let mut interp = create_test_interp();

    let host_promise = api::create_promise(&mut interp);

    let result = run_with_globals(
        &mut interp,
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
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(host_promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut interp).unwrap();

    let StepResult::Suspended { .. } = result2 else {
        panic!("Expected Suspended waiting for Promise");
    };

    // Reject the promise
    let reason = RuntimeValue::unguarded(JsValue::String("Something went wrong".into()));
    api::reject_promise(&mut interp, &host_promise, reason).unwrap();
    let result3 = run_to_completion(&mut interp).unwrap();

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
    let mut interp = create_test_interp();

    let result = run_with_globals(
        &mut interp,
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
    let promise = api::create_promise(&mut interp);
    let value = RuntimeValue::unguarded(JsValue::Number(42.0));

    // Resolve the promise BEFORE returning it to the script
    // This is valid - the Promise is already fulfilled when returned
    api::resolve_promise(&mut interp, &promise, value).unwrap();

    // Now fulfill the order with the already-resolved promise
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise.value().clone())),
    }]);
    let result2 = run_to_completion(&mut interp).unwrap();

    // Since the Promise was already resolved, await should complete immediately
    let StepResult::Complete(final_value) = result2 else {
        panic!("Expected Complete");
    };
    assert_eq!(*final_value, JsValue::String("Result: 42".into()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Concurrency Tests (via host-returned Promises)
// With blocking __order__(), concurrency is achieved by:
// 1. Script calls __order__() which SUSPENDS immediately
// 2. Host returns an unresolved Promise (or any other value)
// 3. Script continues to next __order__(), which also suspends
// 4. Eventually script reaches await Promise.all with multiple host Promises
// 5. Host resolves Promises in any order (can do work in parallel)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_concurrent_fetch_with_promise_all() {
    // Simulate concurrent fetching via host-returned Promises
    // Blocking __order__ semantics:
    // 1. First __order__ suspends immediately, host sees "/users" order
    // 2. Host fulfills with unresolved Promise
    // 3. Second __order__ suspends, host sees "/posts" order
    // 4. Host fulfills with unresolved Promise
    // 5. await Promise.all waits for both Promises
    // 6. Host resolves Promises (can do work in parallel)
    let mut interp = create_test_interp();

    // Create two Promises that will be returned by orders
    let users_promise = api::create_promise(&mut interp);
    let posts_promise = api::create_promise(&mut interp);

    let result = run_with_globals(
        &mut interp,
        r#"
        import { __order__ } from "eval:internal";

        // Each __order__ suspends immediately - host sees one at a time
        const usersPromise = __order__({ type: "fetch", url: "/users" });
        const postsPromise = __order__({ type: "fetch", url: "/posts" });

        // await Promise.all waits for both host Promises
        const [users, posts] = await Promise.all([usersPromise, postsPromise]);

        `${users.count} users, ${posts.count} posts`;
    "#,
    );

    // First suspension: __order__ for "/users"
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended at first __order__");
    };
    assert_eq!(pending.len(), 1, "First order pending");
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "url"),
        Some("/users".into())
    );
    let users_order_id = pending[0].id;

    // Fulfill first order with unresolved Promise
    // Note: users_promise RuntimeValue stays alive, keeping the Promise guarded
    interp.fulfill_orders(vec![OrderResponse {
        id: users_order_id,
        result: Ok(RuntimeValue::unguarded(users_promise.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Second suspension: __order__ for "/posts"
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended at second __order__, got {:?}", result);
    };
    assert_eq!(pending.len(), 1, "Second order pending");
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "url"),
        Some("/posts".into())
    );
    let posts_order_id = pending[0].id;

    // Fulfill second order with unresolved Promise
    // Note: posts_promise RuntimeValue stays alive, keeping the Promise guarded
    interp.fulfill_orders(vec![OrderResponse {
        id: posts_order_id,
        result: Ok(RuntimeValue::unguarded(posts_promise.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Now awaiting Promise.all with two unresolved Promises
    // No more orders pending, just waiting for Promises
    match &result {
        StepResult::Suspended { pending, .. } if pending.is_empty() => {
            // Suspended with no orders - just awaiting Promises
        }
        StepResult::Done => {
            // Done - async work pending, no more sync work
        }
        other => {
            panic!(
                "Expected Done or Suspended while awaiting Promises, got {:?}",
                other
            );
        }
    }

    // Resolve both Promises - host can do this in parallel
    let users_data = api::create_response_object(&mut interp, &json!({ "count": 5 })).unwrap();
    let posts_data = api::create_response_object(&mut interp, &json!({ "count": 10 })).unwrap();
    api::resolve_promise(&mut interp, &users_promise, users_data).unwrap();
    api::resolve_promise(&mut interp, &posts_promise, posts_data).unwrap();

    // After resolving Promises, step to process the results
    let result = run_to_completion(&mut interp).unwrap();

    // Complete - results in original order
    let StepResult::Complete(value) = result else {
        panic!("Expected Complete after both resolved, got {:?}", result);
    };
    assert_eq!(*value, JsValue::String("5 users, 10 posts".into()));
}

#[test]
fn test_promise_race_first_wins() {
    // Promise.race: first to resolve determines the result
    // With blocking __order__:
    // 1. First __order__ suspends, host returns Promise1
    // 2. Second __order__ suspends, host returns Promise2
    // 3. Promise.race waits for first Promise to resolve
    // 4. Host resolves first Promise - it wins
    let mut interp = create_test_interp();

    // Create two Promises that will be returned by orders
    let fast_promise = api::create_promise(&mut interp);
    let slow_promise = api::create_promise(&mut interp);

    let result = run_with_globals(
        &mut interp,
        r#"
        import { __order__ } from "eval:internal";

        // Each __order__ suspends, host returns a Promise
        const fast = __order__({ type: "fetch", server: "fast" });
        const slow = __order__({ type: "fetch", server: "slow" });

        // Race: first to resolve wins
        const winner = await Promise.race([fast, slow]);
        winner.server;
    "#,
    );

    // First order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    let first_order_id = pending[0].id;
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "server"),
        Some("fast".into())
    );

    // Fulfill first order with Promise (not resolved yet)
    interp.fulfill_orders(vec![OrderResponse {
        id: first_order_id,
        result: Ok(RuntimeValue::unguarded(fast_promise.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Second order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for second order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    let second_order_id = pending[0].id;
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "server"),
        Some("slow".into())
    );

    // Fulfill second order with Promise (not resolved yet)
    interp.fulfill_orders(vec![OrderResponse {
        id: second_order_id,
        result: Ok(RuntimeValue::unguarded(slow_promise.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Now awaiting Promise.race with two unresolved Promises
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for Promise.race");
    };
    assert!(
        pending.is_empty(),
        "No orders pending, just awaiting Promises"
    );

    // Resolve first Promise - it wins the race
    let fast_data = api::create_response_object(&mut interp, &json!({ "server": "fast" })).unwrap();
    api::resolve_promise(&mut interp, &fast_promise, fast_data).unwrap();

    let result = run_to_completion(&mut interp).unwrap();

    // Race completes - first resolved wins
    let StepResult::Complete(value) = result else {
        panic!("Expected Complete after fulfillment, got {:?}", result);
    };
    assert_eq!(*value, JsValue::String("fast".into()));
}

#[test]
fn test_promise_race_second_wins() {
    // Verify race works when second Promise is resolved first
    // With blocking __order__:
    // 1. First __order__ suspends, host returns Promise1
    // 2. Second __order__ suspends, host returns Promise2
    // 3. Promise.race waits for first Promise to resolve
    // 4. Host resolves second Promise first - it wins
    let mut interp = create_test_interp();

    // Create two Promises that will be returned by orders
    let a_promise = api::create_promise(&mut interp);
    let b_promise = api::create_promise(&mut interp);

    let result = run_with_globals(
        &mut interp,
        r#"
        import { __order__ } from "eval:internal";

        // Each __order__ suspends, host returns a Promise
        const a = __order__({ type: "fetch", id: "a" });
        const b = __order__({ type: "fetch", id: "b" });

        const winner = await Promise.race([a, b]);
        winner.winner;
    "#,
    );

    // First order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    let first_order_id = pending[0].id;
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "id"),
        Some("a".into())
    );

    // Fulfill first order with Promise (not resolved yet)
    interp.fulfill_orders(vec![OrderResponse {
        id: first_order_id,
        result: Ok(RuntimeValue::unguarded(a_promise.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Second order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for second order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    let second_order_id = pending[0].id;
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "id"),
        Some("b".into())
    );

    // Fulfill second order with Promise (not resolved yet)
    interp.fulfill_orders(vec![OrderResponse {
        id: second_order_id,
        result: Ok(RuntimeValue::unguarded(b_promise.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Now awaiting Promise.race with two unresolved Promises
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for Promise.race");
    };
    assert!(
        pending.is_empty(),
        "No orders pending, just awaiting Promises"
    );

    // Resolve B first - it wins even though it was second in array
    let b_data = api::create_response_object(&mut interp, &json!({ "winner": "B" })).unwrap();
    api::resolve_promise(&mut interp, &b_promise, b_data).unwrap();

    let result = run_to_completion(&mut interp).unwrap();

    // Race completes - B wins because it was resolved first
    let StepResult::Complete(value) = result else {
        panic!("Expected Complete, got {:?}", result);
    };
    assert_eq!(*value, JsValue::String("B".into()));
}

#[test]
fn test_concurrent_with_partial_failure() {
    // One Promise resolves, one rejects - test error handling with Promise.all
    // With blocking __order__:
    // 1. First __order__ suspends, host returns Promise1
    // 2. Second __order__ suspends, host returns Promise2
    // 3. Promise.all waits for both Promises
    // 4. Host resolves first, rejects second - Promise.all catches the error
    let mut interp = create_test_interp();

    // Create two Promises that will be returned by orders
    let ok_promise = api::create_promise(&mut interp);
    let fail_promise = api::create_promise(&mut interp);

    let result = run_with_globals(
        &mut interp,
        r#"
        import { __order__ } from "eval:internal";

        // Each __order__ suspends, host returns a Promise
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

    // First order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    let first_order_id = pending[0].id;
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "url"),
        Some("/ok".into())
    );

    // Fulfill first order with Promise (not resolved yet)
    interp.fulfill_orders(vec![OrderResponse {
        id: first_order_id,
        result: Ok(RuntimeValue::unguarded(ok_promise.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Second order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for second order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    let second_order_id = pending[0].id;
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "url"),
        Some("/fail".into())
    );

    // Fulfill second order with Promise (not resolved yet)
    interp.fulfill_orders(vec![OrderResponse {
        id: second_order_id,
        result: Ok(RuntimeValue::unguarded(fail_promise.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Now awaiting Promise.all with two unresolved Promises
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for Promise.all");
    };
    assert!(
        pending.is_empty(),
        "No orders pending, just awaiting Promises"
    );

    // Resolve first Promise with success
    let ok_data = RuntimeValue::unguarded(JsValue::String("OK".into()));
    api::resolve_promise(&mut interp, &ok_promise, ok_data).unwrap();

    // Reject second Promise with error
    let error_value = RuntimeValue::unguarded(JsValue::String("Network error".into()));
    api::reject_promise(&mut interp, &fail_promise, error_value).unwrap();

    let result = run_to_completion(&mut interp).unwrap();

    // Promise.all rejects if any Promise rejects
    let StepResult::Complete(value) = result else {
        panic!("Expected Complete with error, got {:?}", result);
    };
    assert_eq!(*value, JsValue::String("Error: Network error".into()));
}

#[test]
fn test_concurrent_three_way_race() {
    // Race with three Promises, middle one wins
    // With blocking __order__, each order suspends one at a time
    let mut interp = create_test_interp();

    // Create three host Promises upfront
    let promise1 = api::create_promise(&mut interp);
    let promise2 = api::create_promise(&mut interp);
    let promise3 = api::create_promise(&mut interp);

    let result = run_with_globals(
        &mut interp,
        r#"
        import { __order__ } from "eval:internal";

        // Get three Promises from host (each __order__ suspends)
        const p1 = __order__({ id: 1 });
        const p2 = __order__({ id: 2 });
        const p3 = __order__({ id: 3 });

        // Race: first to resolve wins
        const winner = await Promise.race([p1, p2, p3]);
        "Winner: " + winner;
    "#,
    );

    // First order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    assert_eq!(get_number_prop(pending[0].payload.value(), "id"), Some(1.0));

    // Fulfill first order with Promise1
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise1.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Second order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for second order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    assert_eq!(get_number_prop(pending[0].payload.value(), "id"), Some(2.0));

    // Fulfill second order with Promise2
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise2.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Third order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for third order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    assert_eq!(get_number_prop(pending[0].payload.value(), "id"), Some(3.0));

    // Fulfill third order with Promise3
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise3.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Now awaiting Promise.race with three unresolved Promises
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for Promise.race");
    };
    assert!(
        pending.is_empty(),
        "No orders pending, just awaiting Promises"
    );

    // Resolve Promise2 first - it should win the race
    api::resolve_promise(
        &mut interp,
        &promise2,
        RuntimeValue::unguarded(JsValue::Number(2.0)),
    )
    .unwrap();

    let result = run_to_completion(&mut interp).unwrap();

    let StepResult::Complete(value) = result else {
        panic!("Expected Complete after resolving winner, got {:?}", result);
    };
    assert_eq!(*value, JsValue::String("Winner: 2".into()));
}

#[test]
fn test_concurrent_chained_operations() {
    // Start concurrent fetches, then chain more operations on results
    // With blocking __order__:
    // 1. First __order__ suspends, host returns Promise1
    // 2. Second __order__ suspends, host returns Promise2
    // 3. .then() chains transformations on each Promise
    // 4. Promise.all waits for both transformed Promises
    // 5. Host resolves both Promises
    let mut interp = create_test_interp();

    // Create two Promises that will be returned by orders
    let user_promise = api::create_promise(&mut interp);
    let profile_promise = api::create_promise(&mut interp);

    let result = run_with_globals(
        &mut interp,
        r#"
        import { __order__ } from "eval:internal";

        // Each __order__ suspends, host returns a Promise
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

    // First order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    let first_order_id = pending[0].id;
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "type"),
        Some("getUser".into())
    );

    // Fulfill first order with Promise (not resolved yet)
    interp.fulfill_orders(vec![OrderResponse {
        id: first_order_id,
        result: Ok(RuntimeValue::unguarded(user_promise.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Second order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for second order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    let second_order_id = pending[0].id;
    assert_eq!(
        get_string_prop(pending[0].payload.value(), "type"),
        Some("getProfile".into())
    );

    // Fulfill second order with Promise (not resolved yet)
    interp.fulfill_orders(vec![OrderResponse {
        id: second_order_id,
        result: Ok(RuntimeValue::unguarded(profile_promise.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Now awaiting Promise.all with two unresolved Promises (with .then() chains)
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for Promise.all");
    };
    assert!(
        pending.is_empty(),
        "No orders pending, just awaiting Promises"
    );

    // Resolve both Promises - host can do this in parallel
    let user_data = api::create_response_object(&mut interp, &json!({ "name": "Alice" })).unwrap();
    let profile_data =
        api::create_response_object(&mut interp, &json!({ "bio": "Developer" })).unwrap();
    api::resolve_promise(&mut interp, &user_promise, user_data).unwrap();
    api::resolve_promise(&mut interp, &profile_promise, profile_data).unwrap();

    let result = run_to_completion(&mut interp).unwrap();

    // Complete - .then() chains should have run
    let StepResult::Complete(value) = result else {
        panic!("Expected Complete, got {:?}", result);
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
    // With blocking __order__, each order suspends one at a time
    let mut interp = create_test_interp();

    // Create two host Promises with order IDs for cancellation tracking
    let order1_id = OrderId(1);
    let order2_id = OrderId(2);
    let promise1 = api::create_order_promise(&mut interp, order1_id);
    let promise2 = api::create_order_promise(&mut interp, order2_id);

    let result = run_with_globals(
        &mut interp,
        r#"
        import { __order__ } from "eval:internal";

        const p1 = __order__({ id: 1 });
        const p2 = __order__({ id: 2 });

        const winner = await Promise.race([p1, p2]);
        winner;
    "#,
    );

    // First order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    assert_eq!(get_number_prop(pending[0].payload.value(), "id"), Some(1.0));

    // Fulfill first order with Promise1 (with order_id for cancellation tracking)
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise1.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Second order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for second order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    assert_eq!(get_number_prop(pending[0].payload.value(), "id"), Some(2.0));

    // Fulfill second order with Promise2 (with order_id for cancellation tracking)
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise2.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Now awaiting Promise.race with two unresolved Promises
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for Promise.race");
    };
    assert!(
        pending.is_empty(),
        "No orders pending, just awaiting Promises"
    );

    // Resolve Promise1 first - it wins the race, Promise2's order should be cancelled
    api::resolve_promise(
        &mut interp,
        &promise1,
        RuntimeValue::unguarded(JsValue::String("first".into())),
    )
    .unwrap();

    let result = run_to_completion(&mut interp).unwrap();

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
            let result = run_to_completion(&mut interp).unwrap();
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
    // With blocking __order__, each order suspends one at a time
    let mut interp = create_test_interp();

    // Create two host Promises with order IDs for cancellation tracking
    let order1_id = OrderId(1);
    let order2_id = OrderId(2);
    let promise1 = api::create_order_promise(&mut interp, order1_id);
    let promise2 = api::create_order_promise(&mut interp, order2_id);

    let result = run_with_globals(
        &mut interp,
        r#"
        import { __order__ } from "eval:internal";

        const p1 = __order__({ id: 1 });
        const p2 = __order__({ id: 2 });

        const winner = await Promise.race([p1, p2]);
        winner;
    "#,
    );

    // First order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    assert_eq!(get_number_prop(pending[0].payload.value(), "id"), Some(1.0));

    // Fulfill first order with Promise1
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise1.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Second order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for second order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    assert_eq!(get_number_prop(pending[0].payload.value(), "id"), Some(2.0));

    // Fulfill second order with Promise2
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise2.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Now awaiting Promise.race with two unresolved Promises
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for Promise.race");
    };
    assert!(
        pending.is_empty(),
        "No orders pending, just awaiting Promises"
    );

    // Resolve Promise2 first - it wins the race, Promise1's order should be cancelled
    api::resolve_promise(
        &mut interp,
        &promise2,
        RuntimeValue::unguarded(JsValue::String("second".into())),
    )
    .unwrap();

    let result = run_to_completion(&mut interp).unwrap();

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
    let mut interp = create_test_interp();

    let order_id = OrderId(999);
    let promise = api::create_order_promise(&mut interp, order_id);

    let result = run_with_globals(
        &mut interp,
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
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    let StepResult::Suspended { .. } = result else {
        panic!("Expected Suspended");
    };

    // Reject the Promise - its order should be cancelled
    api::reject_promise(
        &mut interp,
        &promise,
        RuntimeValue::unguarded(JsValue::String("error".into())),
    )
    .unwrap();
    let result = run_to_completion(&mut interp).unwrap();

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
            let result = run_to_completion(&mut interp).unwrap();
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
    // With blocking __order__, each order suspends one at a time
    let mut interp = create_test_interp();

    // Create three host Promises with order IDs for cancellation tracking
    let order1_id = OrderId(1);
    let order2_id = OrderId(2);
    let order3_id = OrderId(3);
    let promise1 = api::create_order_promise(&mut interp, order1_id);
    let promise2 = api::create_order_promise(&mut interp, order2_id);
    let promise3 = api::create_order_promise(&mut interp, order3_id);

    let result = run_with_globals(
        &mut interp,
        r#"
        import { __order__ } from "eval:internal";

        const p1 = __order__({ id: 1 });
        const p2 = __order__({ id: 2 });
        const p3 = __order__({ id: 3 });

        const winner = await Promise.race([p1, p2, p3]);
        "Winner: " + winner;
    "#,
    );

    // First order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for first order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    assert_eq!(get_number_prop(pending[0].payload.value(), "id"), Some(1.0));

    // Fulfill first order with Promise1
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise1.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Second order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for second order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    assert_eq!(get_number_prop(pending[0].payload.value(), "id"), Some(2.0));

    // Fulfill second order with Promise2
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise2.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Third order suspends
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for third order");
    };
    assert_eq!(pending.len(), 1, "One order at a time");
    assert_eq!(get_number_prop(pending[0].payload.value(), "id"), Some(3.0));

    // Fulfill third order with Promise3
    interp.fulfill_orders(vec![OrderResponse {
        id: pending[0].id,
        result: Ok(RuntimeValue::unguarded(promise3.value().clone())),
    }]);
    let result = run_to_completion(&mut interp).unwrap();

    // Now awaiting Promise.race with three unresolved Promises
    let StepResult::Suspended { pending, .. } = result else {
        panic!("Expected Suspended for Promise.race");
    };
    assert!(
        pending.is_empty(),
        "No orders pending, just awaiting Promises"
    );

    // Resolve Promise2 first (order2 wins), Promise1 and Promise3's orders should be cancelled
    api::resolve_promise(
        &mut interp,
        &promise2,
        RuntimeValue::unguarded(JsValue::Number(2.0)),
    )
    .unwrap();

    let result = run_to_completion(&mut interp).unwrap();

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
