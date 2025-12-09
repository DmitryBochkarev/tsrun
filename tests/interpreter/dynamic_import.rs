// Tests for dynamic import() implementation

use super::eval;
use typescript_eval::{JsValue, Runtime, RuntimeResult};

// ═══════════════════════════════════════════════════════════════════════════
// Dynamic import basic
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_dynamic_import_returns_promise() {
    // import() should return a promise
    // Note: In our sync model, the promise may already be resolved
    // but it should still be a promise object
    let result = eval(
        r#"
        const p = import("./module");
        typeof p === "object" && p !== null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_dynamic_import_with_await() {
    // Dynamic import with await should work in async context
    let result = eval(
        r#"
        let captured = 0;
        async function loadModule() {
            const mod = await import("./module");
            return mod;
        }
        // For now, just test that it parses and returns a promise
        const p = loadModule();
        typeof p === "object" && p !== null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_dynamic_import_expression_specifier() {
    // import() can take any expression as specifier
    let result = eval(
        r#"
        const modulePath = "./module";
        const p = import(modulePath);
        typeof p === "object" && p !== null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_dynamic_import_computed_specifier() {
    // import() with computed path
    let result = eval(
        r#"
        const base = "./";
        const name = "module";
        const p = import(base + name);
        typeof p === "object" && p !== null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_dynamic_import_in_function() {
    // Dynamic import inside a regular function
    let result = eval(
        r#"
        function loadModule(path) {
            return import(path);
        }
        const p = loadModule("./module");
        typeof p === "object" && p !== null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

// ═══════════════════════════════════════════════════════════════════════════
// Dynamic import with conditional loading
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_dynamic_import_conditional() {
    // Dynamic import can be used conditionally
    let result = eval(
        r#"
        function maybeLoad(shouldLoad: boolean): Promise<any> | null {
            if (shouldLoad) {
                return import("./module");
            }
            return null;
        }

        // When not loading
        const p1 = maybeLoad(false);
        // When loading
        const p2 = maybeLoad(true);

        p1 === null && typeof p2 === "object"
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_dynamic_import_in_array() {
    // Multiple dynamic imports can be collected
    let result = eval(
        r#"
        const modules = ["./a", "./b", "./c"];
        const promises = modules.map(function(m) { return import(m); });
        promises.length === 3 && typeof promises[0] === "object"
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

// ═══════════════════════════════════════════════════════════════════════════
// Dynamic import type annotation tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_dynamic_import_with_type_annotation() {
    // TypeScript-style dynamic import with type annotation
    let result = eval(
        r#"
        interface Module {
            value: number;
        }

        async function loadTyped(): Promise<Module> {
            const mod = await import("./typed-module");
            return mod as Module;
        }

        const p = loadTyped();
        typeof p === "object" && p !== null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

// ═══════════════════════════════════════════════════════════════════════════
// Dynamic import in class methods
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_dynamic_import_in_class_method() {
    // Dynamic import from within a class method
    let result = eval(
        r#"
        class ModuleLoader {
            private basePath: string;

            constructor(basePath: string) {
                this.basePath = basePath;
            }

            load(name: string): Promise<any> {
                return import(this.basePath + name);
            }
        }

        const loader = new ModuleLoader("./modules/");
        const p = loader.load("utils");
        typeof p === "object" && p !== null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_dynamic_import_in_async_class_method() {
    // Async class method with dynamic import
    let result = eval(
        r#"
        class AsyncLoader {
            async loadModule(path: string): Promise<any> {
                const mod = await import(path);
                return mod;
            }
        }

        const loader = new AsyncLoader();
        const p = loader.loadModule("./async-module");
        typeof p === "object" && p !== null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

// ═══════════════════════════════════════════════════════════════════════════
// Dynamic import error handling
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_dynamic_import_catch_error() {
    // Dynamic import errors can be caught with try-catch in async function
    let result = eval(
        r#"
        let caught: boolean = false;

        async function safeLoad(): Promise<any> {
            try {
                const mod = await import("./nonexistent");
                return mod;
            } catch (e) {
                caught = true;
                return null;
            }
        }

        // Just verify the async function returns a promise
        const p = safeLoad();
        typeof p === "object" && p !== null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_dynamic_import_promise_then() {
    // Dynamic import can be chained with .then()
    let result = eval(
        r#"
        let handled: boolean = false;

        const importPromise = import("./module");
        const p = importPromise.then(function(mod) {
            handled = true;
            return mod;
        });

        typeof p === "object" && p !== null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

// ═══════════════════════════════════════════════════════════════════════════
// Dynamic import state machine tests (with Runtime API)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_dynamic_import_combined_with_static() {
    // Static import followed by code that uses dynamic import
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        import { baseValue } from './config';

        let result: number = 0;

        // This creates a pending promise for dynamic import
        const dynamicPromise = import("./dynamic-module");

        // Use the static import value
        result = baseValue + 10;

        result
    "#,
        )
        .unwrap();

    // First, resolve the static import
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./config");
            let module = runtime.create_module_object(vec![
                ("baseValue".to_string(), JsValue::Number(5.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited for static import"),
    }

    // Continue - should complete (dynamic import creates a pending promise but doesn't suspend)
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(15.0)); // 5 + 10
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_dynamic_import_lazy_loading_pattern() {
    // Common lazy loading pattern: load module only when needed
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        import { shouldLoadExtra } from './config';

        let extraLoaded: boolean = false;

        // Conditional dynamic import - only load if needed
        if (shouldLoadExtra) {
            const p = import("./extra");
            // In real code, would await p
        }

        shouldLoadExtra
    "#,
        )
        .unwrap();

    // Resolve the static import
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./config");
            let module = runtime.create_module_object(vec![
                ("shouldLoadExtra".to_string(), JsValue::Boolean(false)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    // Continue - should complete without triggering dynamic import
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Boolean(false));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_dynamic_import_factory_pattern() {
    // Factory pattern that creates importers
    let result = eval(
        r#"
        function createImporter(basePath: string): (name: string) => Promise<any> {
            return function(name: string): Promise<any> {
                return import(basePath + "/" + name);
            };
        }

        const importFromLib = createImporter("./lib");
        const importFromUtils = createImporter("./utils");

        // Both should return promises
        const p1 = importFromLib("core");
        const p2 = importFromUtils("helpers");

        typeof p1 === "object" && typeof p2 === "object"
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}
