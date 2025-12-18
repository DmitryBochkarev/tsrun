//! Tests for async iteration (for await...of, async generators, Symbol.asyncIterator)

use super::eval;
use typescript_eval::JsValue;

// ═══════════════════════════════════════════════════════════════════════════
// for await...of basic syntax
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_for_await_of_array_of_promises() {
    // for await...of should unwrap promises in an array
    let result = eval(
        r#"
        async function test(): Promise<number> {
            const promises = [
                Promise.resolve(1),
                Promise.resolve(2),
                Promise.resolve(3)
            ];
            let sum = 0;
            for await (const x of promises) {
                sum += x;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_for_await_of_mixed_values() {
    // for await...of should handle mix of promises and plain values
    let result = eval(
        r#"
        async function test(): Promise<string> {
            const values = [
                Promise.resolve("a"),
                "b",
                Promise.resolve("c")
            ];
            let result = "";
            for await (const x of values) {
                result += x;
            }
            return result;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::String("abc".into()));
}

#[test]
fn test_for_await_of_plain_array() {
    // for await...of should work with plain values (await each element)
    let result = eval(
        r#"
        async function test(): Promise<number> {
            const arr = [1, 2, 3];
            let sum = 0;
            for await (const x of arr) {
                sum += x;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_for_await_of_empty_array() {
    // for await...of on empty array should work
    let result = eval(
        r#"
        async function test(): Promise<number> {
            const arr: Promise<number>[] = [];
            let count = 0;
            for await (const x of arr) {
                count++;
            }
            return count;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(0.0));
}

#[test]
fn test_for_await_of_with_break() {
    // for await...of should support break
    let result = eval(
        r#"
        async function test(): Promise<number> {
            const promises = [
                Promise.resolve(1),
                Promise.resolve(2),
                Promise.resolve(3),
                Promise.resolve(4)
            ];
            let sum = 0;
            for await (const x of promises) {
                sum += x;
                if (x === 2) break;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0)); // 1 + 2
}

#[test]
fn test_for_await_of_with_continue() {
    // for await...of should support continue
    let result = eval(
        r#"
        async function test(): Promise<number> {
            const promises = [
                Promise.resolve(1),
                Promise.resolve(2),
                Promise.resolve(3),
                Promise.resolve(4)
            ];
            let sum = 0;
            for await (const x of promises) {
                if (x === 2) continue;
                sum += x;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(8.0)); // 1 + 3 + 4
}

#[test]
fn test_for_await_of_with_destructuring() {
    // for await...of should support destructuring
    let result = eval(
        r#"
        async function test(): Promise<number> {
            const promises = [
                Promise.resolve({ x: 1, y: 2 }),
                Promise.resolve({ x: 3, y: 4 })
            ];
            let sum = 0;
            for await (const { x, y } of promises) {
                sum += x + y;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(10.0)); // (1+2) + (3+4)
}

#[test]
fn test_for_await_of_rejected_promise() {
    // for await...of should throw on rejected promise
    let result = eval(
        r#"
        async function test(): Promise<string> {
            const promises = [
                Promise.resolve(1),
                Promise.reject("error"),
                Promise.resolve(3)
            ];
            try {
                for await (const x of promises) {
                    // should stop at second iteration
                }
                return "no error";
            } catch (e) {
                return "caught: " + e;
            }
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::String("caught: error".into()));
}

// ═══════════════════════════════════════════════════════════════════════════
// Async generators (async function*)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_async_generator_basic() {
    // async generator should return an async iterator
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
        }
        const g = gen();
        const first = await g.next();
        first.value
    "#,
    );
    assert_eq!(result, JsValue::Number(1.0));
}

#[test]
fn test_async_generator_multiple_next() {
    // Calling next() multiple times on async generator
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
        }
        const g = gen();
        await g.next();
        const second = await g.next();
        second.value
    "#,
    );
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn test_async_generator_done() {
    // Async generator should set done=true after all yields
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 1;
        }
        const g = gen();
        await g.next();
        const second = await g.next();
        second.done
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_async_generator_with_await() {
    // async generator should be able to await
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield await Promise.resolve(10);
            yield await Promise.resolve(20);
        }
        const g = gen();
        const first = await g.next();
        const second = await g.next();
        first.value + second.value
    "#,
    );
    assert_eq!(result, JsValue::Number(30.0));
}

#[test]
fn test_async_generator_return_value() {
    // async generator return value
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number, string> {
            yield 1;
            return "done";
        }
        const g = gen();
        await g.next();
        const last = await g.next();
        last.value
    "#,
    );
    assert_eq!(result, JsValue::String("done".into()));
}

#[test]
fn test_async_generator_with_loop() {
    // async generator with for loop
    let result = eval(
        r#"
        async function* range(start: number, end: number): AsyncGenerator<number> {
            for (let i = start; i < end; i++) {
                yield i;
            }
        }
        const g = range(1, 4);
        let sum = 0;
        sum += (await g.next()).value;
        sum += (await g.next()).value;
        sum += (await g.next()).value;
        sum
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0)); // 1 + 2 + 3
}

#[test]
fn test_async_generator_expression() {
    // async generator expression
    let result = eval(
        r#"
        const gen = async function*(): AsyncGenerator<number> {
            yield 10;
            yield 20;
        };
        const g = gen();
        const first = await g.next();
        const second = await g.next();
        first.value + second.value
    "#,
    );
    assert_eq!(result, JsValue::Number(30.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// for await...of with async generators
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_for_await_of_async_generator() {
    // for await...of should work with async generators
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
        }
        async function test(): Promise<number> {
            let sum = 0;
            for await (const x of gen()) {
                sum += x;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_for_await_of_async_generator_with_await() {
    // for await...of with async generator that awaits
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield await Promise.resolve(10);
            yield await Promise.resolve(20);
            yield await Promise.resolve(30);
        }
        async function test(): Promise<number> {
            let sum = 0;
            for await (const x of gen()) {
                sum += x;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(60.0));
}

#[test]
fn test_for_await_of_async_generator_break() {
    // for await...of with async generator and break
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
            yield 4;
        }
        async function test(): Promise<number> {
            let sum = 0;
            for await (const x of gen()) {
                sum += x;
                if (x === 2) break;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0)); // 1 + 2
}

// ═══════════════════════════════════════════════════════════════════════════
// Symbol.asyncIterator
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_symbol_async_iterator_exists() {
    // Symbol.asyncIterator should exist
    let result = eval(
        r#"
        typeof Symbol.asyncIterator
    "#,
    );
    assert_eq!(result, JsValue::String("symbol".into()));
}

#[test]
fn test_custom_async_iterable() {
    // Custom object with Symbol.asyncIterator
    let result = eval(
        r#"
        const asyncIterable = {
            [Symbol.asyncIterator]() {
                let count = 0;
                return {
                    async next() {
                        count++;
                        if (count <= 3) {
                            return { value: count * 10, done: false };
                        }
                        return { value: undefined, done: true };
                    }
                };
            }
        };
        async function test(): Promise<number> {
            let sum = 0;
            for await (const x of asyncIterable) {
                sum += x;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(60.0)); // 10 + 20 + 30
}

#[test]
fn test_custom_async_iterable_with_promises() {
    // Custom async iterator returning promises
    let result = eval(
        r#"
        const asyncIterable = {
            [Symbol.asyncIterator]() {
                let count = 0;
                return {
                    next() {
                        count++;
                        if (count <= 3) {
                            return Promise.resolve({ value: count, done: false });
                        }
                        return Promise.resolve({ value: undefined, done: true });
                    }
                };
            }
        };
        async function test(): Promise<number> {
            let sum = 0;
            for await (const x of asyncIterable) {
                sum += x;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0)); // 1 + 2 + 3
}

#[test]
fn test_async_generator_has_async_iterator() {
    // Async generators should have Symbol.asyncIterator
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 1;
        }
        const g = gen();
        typeof g[Symbol.asyncIterator]
    "#,
    );
    assert_eq!(result, JsValue::String("function".into()));
}

#[test]
fn test_async_generator_async_iterator_returns_self() {
    // g[Symbol.asyncIterator]() should return g itself
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 1;
        }
        const g = gen();
        g[Symbol.asyncIterator]() === g
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

// ═══════════════════════════════════════════════════════════════════════════
// Real-world patterns
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_async_generator_pagination() {
    // Simulating paginated API fetch
    let result = eval(
        r#"
        async function* fetchPages(): AsyncGenerator<number[]> {
            yield await Promise.resolve([1, 2]);
            yield await Promise.resolve([3, 4]);
            yield await Promise.resolve([5]);
        }
        async function getAllItems(): Promise<number[]> {
            const all: number[] = [];
            for await (const page of fetchPages()) {
                for (const item of page) {
                    all.push(item);
                }
            }
            return all;
        }
        const items = await getAllItems();
        items.join(",")
    "#,
    );
    assert_eq!(result, JsValue::String("1,2,3,4,5".into()));
}

#[test]
fn test_async_generator_transform() {
    // Transform async generator values
    let result = eval(
        r#"
        async function* source(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
        }
        async function* double(gen: AsyncGenerator<number>): AsyncGenerator<number> {
            for await (const x of gen) {
                yield x * 2;
            }
        }
        async function test(): Promise<number> {
            let sum = 0;
            for await (const x of double(source())) {
                sum += x;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(12.0)); // 2 + 4 + 6
}

#[test]
fn test_async_generator_filter() {
    // Filter async generator values
    let result = eval(
        r#"
        async function* numbers(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
            yield 4;
            yield 5;
        }
        async function* filter(
            gen: AsyncGenerator<number>,
            pred: (x: number) => boolean
        ): AsyncGenerator<number> {
            for await (const x of gen) {
                if (pred(x)) yield x;
            }
        }
        async function test(): Promise<number> {
            let sum = 0;
            for await (const x of filter(numbers(), x => x % 2 === 0)) {
                sum += x;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0)); // 2 + 4
}

#[test]
fn test_async_generator_take() {
    // Take first n items from async generator
    let result = eval(
        r#"
        async function* infinite(): AsyncGenerator<number> {
            let i = 0;
            while (true) {
                yield i++;
            }
        }
        async function* take(gen: AsyncGenerator<number>, n: number): AsyncGenerator<number> {
            let count = 0;
            for await (const x of gen) {
                if (count >= n) break;
                yield x;
                count++;
            }
        }
        async function test(): Promise<number> {
            let sum = 0;
            for await (const x of take(infinite(), 5)) {
                sum += x;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(10.0)); // 0 + 1 + 2 + 3 + 4
}

#[test]
fn test_async_generator_yield_star() {
    // yield* with async generator
    let result = eval(
        r#"
        async function* gen1(): AsyncGenerator<number> {
            yield 1;
            yield 2;
        }
        async function* gen2(): AsyncGenerator<number> {
            yield* gen1();
            yield 3;
        }
        async function test(): Promise<number> {
            let sum = 0;
            for await (const x of gen2()) {
                sum += x;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0)); // 1 + 2 + 3
}

// ═══════════════════════════════════════════════════════════════════════════
// Error handling in async iteration
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_async_generator_throw() {
    // Throwing inside async generator
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 1;
            throw new Error("oops");
            yield 2;
        }
        async function test(): Promise<string> {
            try {
                for await (const x of gen()) {
                    // first iteration succeeds
                }
                return "no error";
            } catch (e) {
                return "caught";
            }
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::String("caught".into()));
}

#[test]
fn test_async_generator_return_method() {
    // Calling return() on async generator
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
        }
        async function test(): Promise<string> {
            const g = gen();
            await g.next();
            const result = await g.return("early");
            return result.done + ":" + result.value;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::String("true:early".into()));
}

#[test]
fn test_async_generator_throw_method() {
    // Calling throw() on async generator
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number> {
            try {
                yield 1;
                yield 2;
            } catch (e) {
                yield 100;
            }
        }
        async function test(): Promise<number> {
            const g = gen();
            await g.next(); // yields 1
            const result = await g.throw(new Error("test"));
            return result.value;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(100.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Top-level for await
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_top_level_for_await() {
    // for await...of at top level (module context)
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
        }
        let sum = 0;
        for await (const x of gen()) {
            sum += x;
        }
        sum
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_top_level_for_await_array() {
    // for await...of with array of promises at top level
    let result = eval(
        r#"
        const promises = [
            Promise.resolve(10),
            Promise.resolve(20),
            Promise.resolve(30)
        ];
        let sum = 0;
        for await (const x of promises) {
            sum += x;
        }
        sum
    "#,
    );
    assert_eq!(result, JsValue::Number(60.0));
}
