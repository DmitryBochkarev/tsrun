// Tests for async/await implementation

use super::{eval, run_to_completion};
use tsrun::{InternalModule, Interpreter, InterpreterConfig, JsValue, create_eval_internal_module};

// ═══════════════════════════════════════════════════════════════════════════
// Async function declaration
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_async_function_returns_promise() {
    // Async function should return a promise
    let result = eval(
        r#"
        async function foo() {
            return 42;
        }
        const p = foo();
        typeof p === "object" && p !== null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_async_function_resolved_value() {
    // The returned promise should resolve with the return value
    let result = eval(
        r#"
        let captured = 0;
        async function foo() {
            return 42;
        }
        foo().then(function(x) {
            captured = x;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_async_function_no_return() {
    // Async function with no return should resolve with undefined
    let result = eval(
        r#"
        let captured = "not-undefined";
        async function foo() {
            // no return
        }
        foo().then(function(x) {
            captured = typeof x;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::String("undefined".into()));
}

// ═══════════════════════════════════════════════════════════════════════════
// Async arrow functions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_async_arrow_function() {
    // Async arrow function should return a promise
    let result = eval(
        r#"
        let captured = 0;
        const foo = async () => 42;
        foo().then(function(x) {
            captured = x;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_async_arrow_function_with_params() {
    // Async arrow function with parameters
    let result = eval(
        r#"
        let captured = 0;
        const add = async (a, b) => a + b;
        add(10, 20).then(function(x) {
            captured = x;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(30.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Await expression
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_await_promise_resolve() {
    // await should unwrap a resolved promise
    let result = eval(
        r#"
        let captured = 0;
        async function foo() {
            const x = await Promise.resolve(42);
            return x;
        }
        foo().then(function(x) {
            captured = x;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_await_non_promise() {
    // await on non-promise should resolve immediately
    let result = eval(
        r#"
        let captured = 0;
        async function foo() {
            const x = await 42;
            return x;
        }
        foo().then(function(x) {
            captured = x;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_await_chained() {
    // Multiple await expressions in sequence
    let result = eval(
        r#"
        let captured = 0;
        async function foo() {
            const a = await Promise.resolve(1);
            const b = await Promise.resolve(2);
            const c = await Promise.resolve(3);
            return a + b + c;
        }
        foo().then(function(x) {
            captured = x;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_await_in_expression() {
    // await can be used in expressions
    let result = eval(
        r#"
        let captured = 0;
        async function foo() {
            return (await Promise.resolve(10)) + (await Promise.resolve(20));
        }
        foo().then(function(x) {
            captured = x;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(30.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Error handling in async functions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_async_function_throw_rejects() {
    // throw in async function should reject the promise
    let result = eval(
        r#"
        let caught = false;
        async function foo() {
            throw new Error("oops");
        }
        foo().then(null, function(err) {
            caught = true;
        });
        caught
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_await_rejected_promise() {
    // await on rejected promise should throw
    let result = eval(
        r#"
        let caught = false;
        async function foo() {
            await Promise.reject("error");
            return "should not reach";
        }
        foo().then(null, function(err) {
            caught = true;
        });
        caught
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_async_try_catch() {
    // try-catch inside async function should work
    let result = eval(
        r#"
        let captured = "";
        async function foo() {
            try {
                await Promise.reject("error");
            } catch (e) {
                return "caught: " + e;
            }
        }
        foo().then(function(x) {
            captured = x;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::String("caught: error".into()));
}

// ═══════════════════════════════════════════════════════════════════════════
// Async function expression
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_async_function_expression() {
    // Async function expression
    let result = eval(
        r#"
        let captured = 0;
        const foo = async function() {
            return 42;
        };
        foo().then(function(x) {
            captured = x;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_async_function_expression_named() {
    // Named async function expression
    let result = eval(
        r#"
        let captured = 0;
        const foo = async function bar() {
            return 42;
        };
        foo().then(function(x) {
            captured = x;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

// Regression: async function expression assigned to globalThis property
#[test]
fn test_async_function_expression_globalthis() {
    let result = eval(
        r#"
        globalThis.myFunc = async function() {
            return 42;
        };
        let captured = 0;
        myFunc().then(function(x) {
            captured = x;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

// Regression: async function with TypeScript type annotations in assignment
#[test]
fn test_async_function_expression_with_types() {
    let result = eval(
        r#"
        globalThis.sleep = async function(ms: number): Promise<void> {
            return undefined;
        };
        let captured = 0;
        sleep(100).then(function() {
            captured = 42;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

// Regression: async function in source module (mimics orders test setup)
#[test]
fn test_async_function_in_source_module() {
    const MODULE_SOURCE: &str = r#"
globalThis.myAsyncFunc = async function(): Promise<void> {
    return undefined;
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![InternalModule::source("test:module", MODULE_SOURCE)],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    // Import the module
    interp
        .prepare(r#"import "test:module"; myAsyncFunc"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    // Should complete successfully (the async function should be callable)
    match result {
        tsrun::StepResult::Complete(_) => {} // Good
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: async function in source module with import - minimal version
#[test]
fn test_async_function_in_source_module_with_import_minimal() {
    // Minimal test with just the import
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.myFunc = async function(): Promise<void> {
    await order({ type: "test" });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof myFunc"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: async function in source module with import - two functions
#[test]
fn test_async_function_in_source_module_with_import_two_funcs() {
    // Two async functions
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.sleep = async function(ms: number): Promise<void> {
    await order({ type: "sleep", delay: ms });
};

globalThis.fetch = async function(url: string): Promise<any> {
    return await order({ type: "fetch", url: url });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof fetch"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: Optional chaining - direct comparison
#[test]
fn test_optional_chaining_direct() {
    // Direct optional chaining works
    let result = eval(
        r#"
const x = { method: "GET" };
x?.method
"#,
    );
    assert_eq!(result, JsValue::String("GET".into()));
}

// Regression: Optional chaining in object literal
#[test]
fn test_optional_chaining_in_object() {
    // Optional chaining inside an object literal value
    let result = eval(
        r#"
const x = { method: "GET" };
const obj = { value: x?.method };
obj.value
"#,
    );
    assert_eq!(result, JsValue::String("GET".into()));
}

// Regression: Optional chaining in single-line await
#[test]
fn test_async_optional_chaining_single_line() {
    // Single line await with optional chaining
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.b = async function(x: any): Promise<any> {
    return await order({ method: x?.method });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: Simplest - optional chaining in multiline await
#[test]
fn test_async_optional_chaining_simplest() {
    // Simplest: optional chaining in multiline await object
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.b = async function(x: any): Promise<any> {
    return await order({
        method: x?.method
    });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: Optional chaining in await - no first function
#[test]
fn test_async_optional_chaining_no_first_func() {
    // Just one async function with optional chaining
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.b = async function(x?: { method?: string }): Promise<any> {
    return await order({
        method: x?.method
    });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: Optional chaining in await - minimal
#[test]
fn test_async_optional_chaining_minimal() {
    // Minimal reproduction: optional chaining in await object
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.a = async function(ms: number): Promise<void> {
    await order({ type: "a" });
};

globalThis.b = async function(x?: { method?: string }): Promise<any> {
    return await order({
        method: x?.method
    });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: Optional chaining in multiline await
#[test]
fn test_async_optional_chaining_in_await() {
    // Optional chaining (x?.method) in multiline await object
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.a = async function(ms: number): Promise<void> {
    await order({ type: "a" });
};

globalThis.b = async function(x?: {
    method?: string;
    body?: string;
    headers?: string;
}): Promise<any> {
    return await order({
        type: "b",
        method: x?.method
    });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: Three props in type, two in await
#[test]
fn test_async_three_type_two_await() {
    // Three props in multiline type, two in multiline await
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.a = async function(ms: number): Promise<void> {
    await order({ type: "a" });
};

globalThis.b = async function(x?: {
    method?: string;
    body?: string;
    headers?: string;
}): Promise<any> {
    return await order({
        type: "b",
        extra: "value"
    });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: Two props in multiline type
#[test]
fn test_async_two_props_multiline_type() {
    // Two props in multiline type annotation
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.a = async function(ms: number): Promise<void> {
    await order({ type: "a" });
};

globalThis.b = async function(x?: {
    method?: string;
    body?: string;
}): Promise<any> {
    return await order({
        type: "b"
    });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: Minimal multiline type and await
#[test]
fn test_async_minimal_multiline_type_and_await() {
    // Minimal multiline type (2 props) and multiline await (2 props)
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.a = async function(ms: number): Promise<void> {
    await order({ type: "a" });
};

globalThis.b = async function(x?: {
    method?: string;
}): Promise<any> {
    return await order({
        type: "b"
    });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: No url param but multiline type and await
#[test]
fn test_async_no_url_multiline_type_and_await() {
    // No url param, just optional multiline type and multiline await
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.a = async function(ms: number): Promise<void> {
    await order({ type: "a" });
};

globalThis.b = async function(x?: {
    method?: string;
    body?: string;
    headers?: string;
}): Promise<any> {
    return await order({
        type: "b",
        method: x?.method
    });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: No Record but with multiline await
#[test]
fn test_async_no_record_multiline_await() {
    // Three string properties (no Record) but multiline await
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.a = async function(ms: number): Promise<void> {
    await order({ type: "a" });
};

globalThis.b = async function(url: string, x?: {
    method?: string;
    body?: string;
    headers?: string;
}): Promise<any> {
    return await order({
        type: "b",
        url: url,
        method: x?.method
    });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: Record type with simple await
#[test]
fn test_async_record_with_simple_await() {
    // Record type but simple await (not multiline)
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.a = async function(ms: number): Promise<void> {
    await order({ type: "a" });
};

globalThis.b = async function(url: string, x?: {
    method?: string;
    body?: string;
    headers?: Record<string, string>;
}): Promise<any> {
    return await order({ type: "b" });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: Record type and multiline await
#[test]
fn test_async_record_and_multiline_await() {
    // Record type plus multiline await
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.a = async function(ms: number): Promise<void> {
    await order({ type: "a" });
};

globalThis.b = async function(url: string, x?: {
    method?: string;
    body?: string;
    headers?: Record<string, string>;
}): Promise<any> {
    return await order({
        type: "b",
        url: url,
        method: x?.method
    });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: url param and three properties in type annotation
#[test]
fn test_async_url_and_three_props_in_type() {
    // Second function has url param plus 3 properties in type annotation
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.a = async function(ms: number): Promise<void> {
    await order({ type: "a" });
};

globalThis.b = async function(url: string, x?: {
    method?: string;
    body?: string;
    headers?: string;
}): Promise<any> {
    return await order({ type: "b" });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: three properties in type annotation
#[test]
fn test_async_three_props_in_type() {
    // Second function has 3 properties in type annotation
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.a = async function(ms: number): Promise<void> {
    await order({ type: "a" });
};

globalThis.b = async function(x?: {
    method?: string;
    body?: string;
    headers?: string;
}): Promise<any> {
    return await order({ type: "b" });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: multiline await argument
#[test]
fn test_async_multiline_await_arg() {
    // Second function has multiline object as argument to await
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.a = async function(ms: number): Promise<void> {
    await order({ type: "a" });
};

globalThis.b = async function(x?: {
    y?: string;
}): Promise<any> {
    return await order({
        type: "b",
        something: "else"
    });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: function with param then multiline type
#[test]
fn test_async_with_param_then_multiline() {
    // First function has parameter, second has multiline type
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.a = async function(ms: number): Promise<void> {
    await order({ type: "a" });
};

globalThis.b = async function(x?: {
    y?: string;
}): Promise<any> {
    return await order({ type: "b" });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: simple function then multiline type - MINIMAL REPRODUCTION
#[test]
fn test_async_simple_then_multiline_minimal() {
    // MINIMAL: simple async function, then one with multiline type annotation
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.a = async function(): Promise<void> {
    await order({ type: "a" });
};

globalThis.b = async function(x?: {
    y?: string;
}): Promise<any> {
    return await order({ type: "b" });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof b"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: sleep then multiline fetch (exact original order)
#[test]
fn test_async_sleep_then_multiline_fetch() {
    // Test with sleep first, then fetch with multiline type
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.sleep = async function(ms: number): Promise<void> {
    await order({ type: "sleep", delay: ms });
};

globalThis.fetch = async function(url: string, options?: {
    method?: string;
    body?: string;
    headers?: Record<string, string>;
}): Promise<any> {
    return await order({
        type: "fetch",
        url: url,
        method: options?.method || "GET",
        body: options?.body
    });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof fetch"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: two async functions - second one after multiline type
#[test]
fn test_async_two_funcs_multiline_type_first() {
    // Test with multiline type annotation, then another async function
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.fetch = async function(options?: {
    method?: string;
}): Promise<any> {
    return await order({ type: "fetch" });
};

globalThis.readFile = async function(path: string): Promise<string> {
    return await order({ type: "readFile", path: path });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof readFile"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: async function with Record type annotation
#[test]
fn test_async_function_in_source_module_with_record_type() {
    // Test with Record type annotation
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.fetch = async function(options?: {
    headers?: Record<string, string>;
}): Promise<any> {
    return await order({ type: "fetch" });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof fetch"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: async function with multiline object type annotation
#[test]
fn test_async_function_in_source_module_with_multiline_type() {
    // Test with multiline object type annotation
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.fetch = async function(options?: {
    method?: string;
    body?: string;
}): Promise<any> {
    return await order({ type: "fetch" });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof fetch"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: async function with object type annotation - two properties same line
#[test]
fn test_async_function_in_source_module_with_object_type_two_props() {
    // Test with object type annotation - two properties on same line
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.fetch = async function(options?: { method?: string; body?: string }): Promise<any> {
    return await order({ type: "fetch" });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof fetch"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: async function with object type annotation
#[test]
fn test_async_function_in_source_module_with_object_type() {
    // Test with object type annotation - single property
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.fetch = async function(options?: { method?: string }): Promise<any> {
    return await order({ type: "fetch" });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof fetch"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: async function with complex type annotation
#[test]
fn test_async_function_in_source_module_with_complex_type() {
    // Test with complex type annotation
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.fetch = async function(url: string, options?: {
    method?: string;
    body?: string;
    headers?: Record<string, string>;
}): Promise<any> {
    return await order({
        type: "fetch",
        url: url,
        method: options?.method || "GET",
        body: options?.body
    });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    interp
        .prepare(r#"import "eval:globals"; typeof fetch"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// Regression: async function in source module with import (exact orders setup)
#[test]
fn test_async_function_in_source_module_with_import() {
    // Testing with 5-line object literal
    const GLOBALS_SOURCE: &str = r#"
import { order } from "tsrun:host";

globalThis.sleep = async function(ms: number): Promise<void> {
    await order({ type: "sleep", delay: ms });
};

globalThis.fetch = async function(url: string, options?: {
    method?: string;
    body?: string;
    headers?: Record<string, string>;
}): Promise<any> {
    return await order({
        type: "fetch",
        url: url,
        method: options?.method || "GET",
        body: options?.body
    });
};

globalThis.readFile = async function(path: string): Promise<string> {
    return await order({ type: "readFile", path: path });
};

globalThis.writeFile = async function(path: string, content: string): Promise<string> {
    return await order({ type: "writeFile", path: path, content: content });
};
"#;

    let config = InterpreterConfig {
        internal_modules: vec![
            create_eval_internal_module(),
            InternalModule::source("eval:globals", GLOBALS_SOURCE),
        ],
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(config);
    interp.set_gc_threshold(1);

    // Import the globals module
    interp
        .prepare(r#"import "eval:globals"; typeof sleep"#, None)
        .expect("prepare should succeed");

    let result = run_to_completion(&mut interp).expect("run should succeed");
    match result {
        tsrun::StepResult::Complete(rv) => {
            assert_eq!(*rv, JsValue::String("function".into()));
        }
        other => panic!("Expected Complete, got {:?}", other),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Async with closures
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_async_closure_capture() {
    // Async function should capture variables from outer scope
    let result = eval(
        r#"
        let captured = 0;
        const x = 10;
        async function foo() {
            return x + (await Promise.resolve(5));
        }
        foo().then(function(v) {
            captured = v;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(15.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Async method
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_async_method() {
    // Async method in object literal
    let result = eval(
        r#"
        let captured = 0;
        const obj = {
            async getValue() {
                return 42;
            }
        };
        obj.getValue().then(function(x) {
            captured = x;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_async_class_method() {
    // Async method in class
    let result = eval(
        r#"
        let captured = 0;
        class Foo {
            async getValue() {
                return 42;
            }
        }
        const foo = new Foo();
        foo.getValue().then(function(x) {
            captured = x;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Top-level await with complex results
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_toplevel_await_returns_object() {
    // Top-level await should return the resolved object value
    // Verify by accessing the count property in JavaScript
    let result = eval(
        r#"
        async function getData(): Promise<{ count: number }> {
            return { count: 42 };
        }

        const obj = await getData();
        obj.count
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_toplevel_await_returns_object_typeof() {
    // Verify that top-level await returns an actual object
    let result = eval(
        r#"
        async function getData(): Promise<{ count: number }> {
            return { count: 42 };
        }

        const obj = await getData();
        typeof obj
    "#,
    );
    assert_eq!(result, JsValue::String("object".into()));
}

#[test]
fn test_toplevel_await_json_stringify() {
    // JSON.stringify on top-level await result should return a string
    let result = eval(
        r#"
        async function getData(): Promise<{ count: number }> {
            return { count: 42 };
        }

        const data = await getData();
        JSON.stringify(data)
    "#,
    );
    assert_eq!(result, JsValue::String(r#"{"count":42}"#.into()));
}

// ═══════════════════════════════════════════════════════════════════════════
// Async utility function tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_async_safe_process_success() {
    // safeProcess should wrap successful results
    let result = eval(
        r#"
        async function safeProcess<T, R>(
            input: T,
            processor: (data: T) => R
        ): Promise<{ success: boolean; data?: R; error?: string }> {
            try {
                const result = processor(input);
                return { success: true, data: result };
            } catch (e) {
                return { success: false, error: String(e) };
            }
        }

        const r = await safeProcess([1, 2, 3], (arr: number[]) => arr.reduce((a, b) => a + b, 0));
        r.success
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_async_safe_process_with_throw() {
    // safeProcess should catch errors from throwing processor
    let result = eval(
        r#"
        async function safeProcess<T, R>(
            input: T,
            processor: (data: T) => R
        ): Promise<{ success: boolean; data?: R; error?: string }> {
            try {
                const result = processor(input);
                return { success: true, data: result };
            } catch (e) {
                return { success: false, error: String(e) };
            }
        }

        const r = await safeProcess(5, (n: number) => {
            if (n > 3) throw new Error("too big");
            return n;
        });
        r.success
    "#,
    );
    assert_eq!(result, JsValue::Boolean(false));
}

#[test]
fn test_async_calculate_stats() {
    // calculateStats should compute count and items
    let result = eval(
        r#"
        interface Statistics {
            count: number;
            items: string[];
        }

        async function calculateStats<T>(
            fetchData: () => Promise<T[]>,
            getName: (item: T) => string
        ): Promise<Statistics> {
            const data = await fetchData();
            return {
                count: data.length,
                items: data.map(getName),
            };
        }

        async function fetchUsers(): Promise<{ name: string }[]> {
            return [{ name: "Alice" }, { name: "Bob" }];
        }

        const stats = await calculateStats(fetchUsers, (u) => u.name);
        stats.count
    "#,
    );
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn test_async_calculate_stats_items() {
    // calculateStats items array
    let result = eval(
        r#"
        interface Statistics {
            count: number;
            items: string[];
        }

        async function calculateStats<T>(
            fetchData: () => Promise<T[]>,
            getName: (item: T) => string
        ): Promise<Statistics> {
            const data = await fetchData();
            return {
                count: data.length,
                items: data.map(getName),
            };
        }

        async function fetchUsers(): Promise<{ name: string }[]> {
            return [{ name: "Alice" }, { name: "Bob" }];
        }

        const stats = await calculateStats(fetchUsers, (u) => u.name);
        stats.items.join(",")
    "#,
    );
    assert_eq!(result, JsValue::String("Alice,Bob".into()));
}

#[test]
fn test_async_retry_success() {
    // retry should return result on success
    let result = eval(
        r#"
        async function retry<T>(
            operation: () => Promise<T>,
            maxAttempts: number
        ): Promise<T | null> {
            let attempts = 0;
            while (attempts < maxAttempts) {
                try {
                    return await operation();
                } catch (e) {
                    attempts++;
                    if (attempts >= maxAttempts) {
                        return null;
                    }
                }
            }
            return null;
        }

        const r = await retry(async () => 42, 3);
        r
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_async_retry_simple() {
    // Simpler test for async retry pattern
    let result = eval(
        r#"
        async function test(): Promise<number | null> {
            let attempts: number = 0;
            while (attempts < 3) {
                try {
                    throw new Error("fail");
                } catch (e: any) {
                    attempts++;
                    if (attempts >= 3) {
                        return null;
                    }
                }
            }
            return null;
        }

        const r = await test();
        r
    "#,
    );
    assert_eq!(result, JsValue::Null);
}

#[test]
fn test_async_retry_with_await() {
    // Test without return keyword first
    let result = eval(
        r#"
        let captured = "";
        async function test() {
            try {
                await Promise.reject("fail");
            } catch (e: any) {
                captured = "caught";
            }
        }
        test().then(function() {});
        captured
    "#,
    );
    assert_eq!(result, JsValue::String("caught".into()));
}

#[test]
fn test_async_try_catch_with_return() {
    // Await on rejected promise with return - uses top-level await
    let result = eval(
        r#"
        async function test(): Promise<number | null> {
            try {
                return await Promise.reject("fail");
            } catch (e: any) {
                return null;
            }
        }

        const r = await test();
        r
    "#,
    );
    assert_eq!(result, JsValue::Null);
}

#[test]
fn test_async_await_throwing_function() {
    // Await on async function that throws - this is different from Promise.reject
    let result = eval(
        r#"
        async function alwaysFail(): Promise<number> {
            throw new Error("fail");
        }

        async function test(): Promise<number | null> {
            try {
                return await alwaysFail();
            } catch (e: any) {
                return null;
            }
        }

        const r = await test();
        r
    "#,
    );
    assert_eq!(result, JsValue::Null);
}

#[test]
fn test_async_retry_with_await_loop() {
    // Test with await inside try block inside loop
    let result = eval(
        r#"
        async function alwaysFail(): Promise<number> {
            throw new Error("fail");
        }

        async function test(): Promise<number | null> {
            let attempts: number = 0;
            while (attempts < 3) {
                try {
                    return await alwaysFail();
                } catch (e: any) {
                    attempts++;
                    if (attempts >= 3) {
                        return null;
                    }
                }
            }
            return null;
        }

        const r = await test();
        r
    "#,
    );
    assert_eq!(result, JsValue::Null);
}

#[test]
fn test_async_retry_failure() {
    // retry should return null after max attempts
    let result = eval(
        r#"
        async function retry<T>(
            operation: () => Promise<T>,
            maxAttempts: number
        ): Promise<T | null> {
            let attempts = 0;
            while (attempts < maxAttempts) {
                try {
                    return await operation();
                } catch (e) {
                    attempts++;
                    if (attempts >= maxAttempts) {
                        return null;
                    }
                }
            }
            return null;
        }

        const r = await retry(async () => { throw new Error("fail"); }, 3);
        r === null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_async_aggregate_results() {
    // aggregateResults should flatten multiple promise arrays
    let result = eval(
        r#"
        async function aggregateResults<T>(
            sources: Promise<T[]>[]
        ): Promise<T[]> {
            const allArrays = await Promise.all(sources);
            return allArrays.flat();
        }

        const p1 = Promise.resolve([1, 2]);
        const p2 = Promise.resolve([3, 4]);
        const combined = await aggregateResults([p1, p2]);
        combined.join(",")
    "#,
    );
    assert_eq!(result, JsValue::String("1,2,3,4".into()));
}

#[test]
fn test_async_pipeline_chain() {
    // pipeline should chain async operations
    let result = eval(
        r#"
        async function pipeline<A, B, C>(
            initial: Promise<A>,
            step1: (a: A) => Promise<B>,
            step2: (b: B) => Promise<C>
        ): Promise<C> {
            const a = await initial;
            const b = await step1(a);
            return step2(b);
        }

        const result = await pipeline(
            Promise.resolve(5),
            async (n) => n * 2,
            async (n) => n + 1
        );
        result
    "#,
    );
    assert_eq!(result, JsValue::Number(11.0)); // (5 * 2) + 1 = 11
}

#[test]
fn test_async_filter() {
    // asyncFilter should filter based on async predicate
    let result = eval(
        r#"
        async function asyncFilter<T>(
            items: T[],
            predicate: (item: T) => Promise<boolean>
        ): Promise<T[]> {
            const results: T[] = [];
            for (const item of items) {
                if (await predicate(item)) {
                    results.push(item);
                }
            }
            return results;
        }

        const filtered = await asyncFilter([1, 2, 3, 4, 5], async (n) => n % 2 === 0);
        filtered.join(",")
    "#,
    );
    assert_eq!(result, JsValue::String("2,4".into()));
}

#[test]
fn test_async_promise_assimilation() {
    // When an async function returns a Promise, it should not double-wrap
    // (Promise assimilation / thenable unwrapping)
    let result = eval(
        r#"
        async function returnsPromise(): Promise<number> {
            return Promise.resolve(42);
        }
        await returnsPromise()
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_async_nested_promise() {
    // Nested async calls should properly unwrap
    let result = eval(
        r#"
        async function inner(): Promise<number> {
            return 100;
        }
        async function outer(): Promise<number> {
            return inner();
        }
        await outer()
    "#,
    );
    assert_eq!(result, JsValue::Number(100.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Return await with .then() - regression tests for tail call optimization
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_return_await_then_with_intermediate_variable() {
    // This pattern works: assign to variable, then return
    let result = eval(
        r#"
        async function process(): Promise<string> {
            let r = await Promise.resolve("hello").then(s => s.toUpperCase());
            return r;
        }
        await process()
    "#,
    );
    assert_eq!(result, JsValue::String("HELLO".into()));
}

#[test]
fn test_return_await_then_direct() {
    // This pattern should also work: return await expr.then(...)
    let result = eval(
        r#"
        async function process(): Promise<string> {
            return await Promise.resolve("hello").then(s => s.toUpperCase());
        }
        await process()
    "#,
    );
    assert_eq!(result, JsValue::String("HELLO".into()));
}

#[test]
fn test_return_await_then_chain() {
    // Multiple .then() calls in chain
    let result = eval(
        r#"
        async function process(): Promise<number> {
            return await Promise.resolve(5)
                .then(n => n * 2)
                .then(n => n + 1);
        }
        await process()
    "#,
    );
    assert_eq!(result, JsValue::Number(11.0));
}

#[test]
fn test_return_await_then_with_method_chain() {
    // Simulating a real-world pattern like exec().then(o => o.stdout.trim())
    let result = eval(
        r#"
        async function exec(): Promise<{ stdout: string }> {
            return { stdout: "  hello world  " };
        }

        async function du(): Promise<string> {
            return await exec().then(o => o.stdout.trim());
        }
        await du()
    "#,
    );
    assert_eq!(result, JsValue::String("hello world".into()));
}

#[test]
fn test_return_await_then_with_method_chain_intermediate() {
    // Same pattern with intermediate variable (should work)
    let result = eval(
        r#"
        async function exec(): Promise<{ stdout: string }> {
            return { stdout: "  hello world  " };
        }

        async function du(): Promise<string> {
            let r = await exec().then(o => o.stdout.trim());
            return r;
        }
        await du()
    "#,
    );
    assert_eq!(result, JsValue::String("hello world".into()));
}

#[test]
fn test_return_await_then_with_replace() {
    // Pattern from user's bug report: exec().then(o => o.stdout.replace(...).trim())
    let result = eval(
        r#"
        async function exec(): Promise<{ stdout: string }> {
            return { stdout: "100K\ttarget/release" };
        }

        async function du(): Promise<string> {
            return await exec().then(o => o.stdout.replace(/\s+/g, ' ').trim());
        }
        await du()
    "#,
    );
    assert_eq!(result, JsValue::String("100K target/release".into()));
}

#[test]
fn test_return_await_then_with_replace_intermediate() {
    // Same pattern with intermediate variable
    let result = eval(
        r#"
        async function exec(): Promise<{ stdout: string }> {
            return { stdout: "100K\ttarget/release" };
        }

        async function du(): Promise<string> {
            let r = await exec().then(o => o.stdout.replace(/\s+/g, ' ').trim());
            return r;
        }
        await du()
    "#,
    );
    assert_eq!(result, JsValue::String("100K target/release".into()));
}
