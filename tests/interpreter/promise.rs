// Tests for Promise implementation

use super::eval;
use typescript_eval::JsValue;

// ═══════════════════════════════════════════════════════════════════════════
// Promise constructor and basic state
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_promise_constructor_basic() {
    // Promise constructor should accept executor function
    let result = eval(
        r#"
        let called = false;
        const p = new Promise(function(resolve, reject) {
            called = true;
        });
        called
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_promise_executor_receives_resolve_reject() {
    // Executor should receive resolve and reject as functions
    let result = eval(
        r#"
        let resolveType = "";
        let rejectType = "";
        new Promise(function(resolve, reject) {
            resolveType = typeof resolve;
            rejectType = typeof reject;
        });
        resolveType + "," + rejectType
    "#,
    );
    assert_eq!(result, JsValue::String("function,function".into()));
}

// ═══════════════════════════════════════════════════════════════════════════
// Promise.resolve and Promise.reject
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_promise_resolve_static() {
    // Promise.resolve should create a fulfilled promise
    let result = eval(
        r#"
        const p = Promise.resolve(42);
        typeof p === "object" && p !== null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_promise_reject_static() {
    // Promise.reject should create a rejected promise
    let result = eval(
        r#"
        const p = Promise.reject("error");
        typeof p === "object" && p !== null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

// ═══════════════════════════════════════════════════════════════════════════
// Synchronous then/catch handling
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_promise_then_on_fulfilled() {
    // .then() on a fulfilled promise should call the handler synchronously
    let result = eval(
        r#"
        let captured = 0;
        Promise.resolve(42).then(function(value) {
            captured = value;
        });
        captured
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_promise_then_returns_promise() {
    // .then() should return a promise
    let result = eval(
        r#"
        const p = Promise.resolve(1).then(function(x) {});
        typeof p === "object" && p !== null
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_promise_then_chaining() {
    // .then() can chain transformations
    let result = eval(
        r#"
        let finalValue = 0;
        Promise.resolve(1)
            .then(function(x) { return x + 1; })
            .then(function(x) { return x * 2; })
            .then(function(x) {
                finalValue = x;
            });
        finalValue
    "#,
    );
    assert_eq!(result, JsValue::Number(4.0)); // (1 + 1) * 2 = 4
}

#[test]
fn test_promise_catch_on_rejected() {
    // .then(null, handler) on a rejected promise should call the handler
    let result = eval(
        r#"
        let caught = "";
        Promise.reject("oops").then(null, function(err) {
            caught = err;
        });
        caught
    "#,
    );
    assert_eq!(result, JsValue::String("oops".into()));
}

#[test]
fn test_promise_catch_skipped_on_fulfilled() {
    // .then(null, handler) should not be called on fulfilled promises
    let result = eval(
        r#"
        let caught = false;
        Promise.resolve(42).then(null, function() {
            caught = true;
        });
        caught
    "#,
    );
    assert_eq!(result, JsValue::Boolean(false));
}

#[test]
fn test_promise_then_onrejected() {
    // .then(null, onRejected) should handle rejection
    let result = eval(
        r#"
        let caught = "";
        Promise.reject("error").then(null, function(err) {
            caught = err;
        });
        caught
    "#,
    );
    assert_eq!(result, JsValue::String("error".into()));
}

// ═══════════════════════════════════════════════════════════════════════════
// Error propagation
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_promise_error_propagation() {
    // Error in then handler should reject the chain
    let result = eval(
        r#"
        let caught = false;
        Promise.resolve(1)
            .then(function(x) {
                throw new Error("oops");
            })
            .then(null, function(err) {
                caught = true;
            });
        caught
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_promise_rejection_skips_then() {
    // Rejected promise should skip .then() onFulfilled handlers
    let result = eval(
        r#"
        let thenCalled = false;
        let catchCalled = false;
        Promise.reject("error")
            .then(function() {
                thenCalled = true;
            })
            .then(null, function() {
                catchCalled = true;
            });
        !thenCalled && catchCalled
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

// ═══════════════════════════════════════════════════════════════════════════
// Promise in executor
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_promise_sync_resolve_in_executor() {
    // resolve() called synchronously in executor
    let result = eval(
        r#"
        let value = 0;
        new Promise(function(resolve, reject) {
            resolve(100);
        }).then(function(x) {
            value = x;
        });
        value
    "#,
    );
    assert_eq!(result, JsValue::Number(100.0));
}

#[test]
fn test_promise_sync_reject_in_executor() {
    // reject() called synchronously in executor
    let result = eval(
        r#"
        let caught = "";
        new Promise(function(resolve, reject) {
            reject("failed");
        }).then(null, function(err) {
            caught = err;
        });
        caught
    "#,
    );
    assert_eq!(result, JsValue::String("failed".into()));
}

#[test]
fn test_promise_executor_error_rejects() {
    // Error thrown in executor should reject the promise
    let result = eval(
        r#"
        let caught = false;
        new Promise(function(resolve, reject) {
            throw new Error("executor error");
        }).then(null, function(err) {
            caught = true;
        });
        caught
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_promise_multiple_resolve_ignored() {
    // Only first resolve() should take effect
    let result = eval(
        r#"
        let value = 0;
        new Promise(function(resolve, reject) {
            resolve(1);
            resolve(2);
            resolve(3);
        }).then(function(x) {
            value = x;
        });
        value
    "#,
    );
    assert_eq!(result, JsValue::Number(1.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Promise.all, Promise.race, Promise.allSettled, Promise.any
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_promise_all_with_values() {
    // Promise.all with array of promises
    let result = eval(
        r#"
        let values = [];
        Promise.all([
            Promise.resolve(1),
            Promise.resolve(2),
            Promise.resolve(3)
        ]).then(function(arr) {
            values = arr;
        });
        values.join(",")
    "#,
    );
    assert_eq!(result, JsValue::String("1,2,3".into()));
}

#[test]
fn test_promise_all_rejects_on_first_rejection() {
    // Promise.all should reject if any promise rejects
    let result = eval(
        r#"
        let caught = "";
        Promise.all([
            Promise.resolve(1),
            Promise.reject("fail"),
            Promise.resolve(3)
        ]).then(null, function(err) {
            caught = err;
        });
        caught
    "#,
    );
    assert_eq!(result, JsValue::String("fail".into()));
}

#[test]
fn test_promise_race_first_wins() {
    // Promise.race should resolve with first settled value
    let result = eval(
        r#"
        let value = 0;
        Promise.race([
            Promise.resolve(1),
            Promise.resolve(2)
        ]).then(function(x) {
            value = x;
        });
        value
    "#,
    );
    assert_eq!(result, JsValue::Number(1.0));
}

#[test]
fn test_promise_allsettled() {
    // Promise.allSettled should report status of all promises
    let result = eval(
        r#"
        let results = [];
        Promise.allSettled([
            Promise.resolve(1),
            Promise.reject("error")
        ]).then(function(arr) {
            results = arr;
        });
        results.length
    "#,
    );
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn test_promise_any_first_success() {
    // Promise.any should resolve with first fulfilled value
    let result = eval(
        r#"
        let value = 0;
        Promise.any([
            Promise.reject("err1"),
            Promise.resolve(42),
            Promise.reject("err2")
        ]).then(function(x) {
            value = x;
        });
        value
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}
