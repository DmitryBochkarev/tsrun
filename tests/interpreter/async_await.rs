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

// ═══════════════════════════════════════════════════════════════════════════
// Top-level await with complex results
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_toplevel_await_returns_object() {
    // Top-level await should return the resolved object value
    let result = eval(
        r#"
        async function getData(): Promise<{ count: number }> {
            return { count: 42 };
        }

        await getData()
    "#,
    );
    // The result should be an object
    if let JsValue::Object(obj) = &result {
        let borrowed = obj.borrow();
        let count = borrowed
            .get_property(&typescript_eval::value::PropertyKey::from("count"))
            .unwrap_or(JsValue::Undefined);
        assert_eq!(count, JsValue::Number(42.0));
    } else {
        panic!("Expected Object, got {:?}", result);
    }
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
