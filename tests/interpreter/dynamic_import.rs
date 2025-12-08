// Tests for dynamic import() implementation

use super::eval;
use typescript_eval::JsValue;

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
