// Tests for async/await implementation

use super::eval;
use typescript_eval::JsValue;

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
