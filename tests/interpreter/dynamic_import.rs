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
            let module =
                runtime.create_module_object(vec![("baseValue".to_string(), JsValue::Number(5.0))]);
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
            let module = runtime.create_module_object(vec![(
                "shouldLoadExtra".to_string(),
                JsValue::Boolean(false),
            )]);
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

// ═══════════════════════════════════════════════════════════════════════════
// Nested module loading with state save/restore
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_nested_static_imports_with_state_save_restore() {
    // Test that nested static imports work correctly when using save/restore
    let mut runtime = Runtime::new();

    // Main module imports from module A
    let result = runtime
        .eval(
            r#"
        import { getValue } from './moduleA';

        // Use the imported value
        getValue() + 100
    "#,
        )
        .unwrap();

    // First import: moduleA
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./moduleA");

            // Save state before loading moduleA
            let saved_state = runtime.save_execution_state();

            // Simulate loading moduleA (which has no nested imports)
            runtime
                .eval(
                    r#"
                export function getValue(): number {
                    return 42;
                }
            "#,
                )
                .unwrap();

            // Create module object from exports
            let exports: Vec<(String, JsValue)> = runtime
                .get_exports()
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect();
            let module_a = runtime.create_module_object(exports);

            // Restore state and fill slot
            runtime.restore_execution_state(saved_state);
            slot.set_success(module_a);
        }
        _ => panic!("Expected ImportAwaited for moduleA"),
    }

    // Continue execution
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(142.0)); // 42 + 100
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_module_returning_array_with_prototype() {
    // Test that arrays from nested modules have proper prototype chain
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import { getNumbers } from './arrayModule';

        // Use array methods on imported array
        const nums = getNumbers();
        nums.map(x => x * 2).join(",")
    "#,
        )
        .unwrap();

    // Import: arrayModule
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./arrayModule");

            // Save state before loading module
            let saved_state = runtime.save_execution_state();

            // Load the array module - it exports a function that returns an array
            runtime
                .eval(
                    r#"
                export function getNumbers(): number[] {
                    return [1, 2, 3, 4, 5];
                }
            "#,
                )
                .unwrap();

            // Create module object from exports
            let exports: Vec<(String, JsValue)> = runtime
                .get_exports()
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect();
            let module = runtime.create_module_object(exports);

            // Restore state and fill slot
            runtime.restore_execution_state(saved_state);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    // Continue execution
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            // Array methods should work because prototype is shared
            assert_eq!(value, JsValue::String("2,4,6,8,10".into()));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_deeply_nested_imports_with_state_save_restore() {
    // Test: main -> A -> B chain of imports
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        import { aValue } from './moduleA';
        aValue + 1000
    "#,
        )
        .unwrap();

    // First import: moduleA
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./moduleA");

            // Save main state
            let main_saved_state = runtime.save_execution_state();

            // Load moduleA which imports from moduleB
            let result_a = runtime
                .eval(
                    r#"
                import { bValue } from './moduleB';
                export const aValue: number = bValue * 10;
            "#,
                )
                .unwrap();

            // moduleA wants moduleB
            match result_a {
                RuntimeResult::ImportAwaited {
                    slot: slot_b,
                    specifier: spec_b,
                } => {
                    assert_eq!(spec_b, "./moduleB");

                    // Save moduleA state
                    let a_saved_state = runtime.save_execution_state();

                    // Load moduleB
                    runtime
                        .eval(
                            r#"
                        export const bValue: number = 5;
                    "#,
                        )
                        .unwrap();

                    // Create moduleB object
                    let exports_b: Vec<(String, JsValue)> = runtime
                        .get_exports()
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.clone()))
                        .collect();
                    let module_b = runtime.create_module_object(exports_b);

                    // Restore moduleA state and continue
                    runtime.restore_execution_state(a_saved_state);
                    slot_b.set_success(module_b);

                    // Continue moduleA execution
                    let result_a_continued = runtime.continue_eval().unwrap();
                    match result_a_continued {
                        RuntimeResult::Complete(_) => {
                            // moduleA completed
                        }
                        _ => panic!("Expected moduleA to complete"),
                    }
                }
                _ => panic!("Expected ImportAwaited for moduleB"),
            }

            // Create moduleA object
            let exports_a: Vec<(String, JsValue)> = runtime
                .get_exports()
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect();
            let module_a = runtime.create_module_object(exports_a);

            // Restore main state and continue
            runtime.restore_execution_state(main_saved_state);
            slot.set_success(module_a);
        }
        _ => panic!("Expected ImportAwaited for moduleA"),
    }

    // Continue main execution
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            // 5 (bValue) * 10 (in moduleA) + 1000 (in main) = 1050
            assert_eq!(value, JsValue::Number(1050.0));
        }
        _ => panic!("Expected Complete"),
    }
}
