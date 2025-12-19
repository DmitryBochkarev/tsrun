//! Tests for async iteration (for await...of, async generators, Symbol.asyncIterator)

use super::eval;
use typescript_eval::JsValue;

// ═══════════════════════════════════════════════════════════════════════════
// Debug tests for isolating issues
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_debug_single_await_gen_next() {
    // Simplest: single await on generator.next()
    let result = eval(
        r#"
        async function* source(): AsyncGenerator<number> {
            yield 1;
        }
        async function test(): Promise<number> {
            const gen = source();
            const r = await gen.next();
            return r.value;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(1.0));
}

#[test]
fn test_debug_double_await_gen_next() {
    // Two await calls
    let result = eval(
        r#"
        async function* source(): AsyncGenerator<number> {
            yield 1;
            yield 2;
        }
        async function test(): Promise<number> {
            const gen = source();
            const r1 = await gen.next();
            const r2 = await gen.next();
            return r1.value + r2.value;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0)); // 1 + 2
}

#[test]
fn test_debug_await_in_while_loop() {
    // Using let r with reassignment in loop
    let result = eval(
        r#"
        async function* source(): AsyncGenerator<number> {
            yield 1;
            yield 2;
        }
        async function test(): Promise<number> {
            const gen = source();
            let sum = 0;
            let r = await gen.next();
            while (!r.done) {
                sum += r.value;
                r = await gen.next();
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0)); // 1 + 2
}

#[test]
fn test_debug_three_await_sequential() {
    // Three sequential awaits without loop
    let result = eval(
        r#"
        async function* source(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
        }
        async function test(): Promise<number> {
            const gen = source();
            let sum = 0;
            let r = await gen.next();
            sum += r.value;
            r = await gen.next();
            sum += r.value;
            r = await gen.next();
            sum += r.value;
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0)); // 1 + 2 + 3
}

#[test]
fn test_debug_simple_while_no_await_in_loop() {
    // While loop but await BEFORE the loop, not inside
    let result = eval(
        r#"
        async function* source(): AsyncGenerator<number> {
            yield 1;
        }
        async function test(): Promise<number> {
            const gen = source();
            let r = await gen.next();  // Only one await
            let sum = 0;
            let count = 0;
            while (count < 3) {
                sum += r.value;
                count++;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0)); // 1 + 1 + 1
}

#[test]
fn test_debug_two_await_with_let_reassign() {
    // Two awaits with let reassignment (no loop)
    let result = eval(
        r#"
        async function* source(): AsyncGenerator<number> {
            yield 1;
            yield 2;
        }
        async function test(): Promise<number> {
            const gen = source();
            let r = await gen.next();
            const first = r.value;
            r = await gen.next();  // Reassign r
            return first + r.value;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0)); // 1 + 2
}

#[test]
fn test_debug_while_with_done_check() {
    // Just while with done check, one iteration
    let result = eval(
        r#"
        async function* source(): AsyncGenerator<number> {
            yield 1;
        }
        async function test(): Promise<number> {
            const gen = source();
            let r = await gen.next();
            let sum = 0;
            while (!r.done) {
                sum += r.value;
                break;  // Just one iteration
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(1.0));
}

#[test]
fn test_debug_await_in_while_with_counter() {
    // Await inside while, but use counter to limit iterations
    let result = eval(
        r#"
        async function* source(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
        }
        async function test(): Promise<number> {
            const gen = source();
            let sum = 0;
            let count = 0;
            while (count < 2) {
                const res = await gen.next();  // Use const instead of reassigning
                sum += res.value;
                count++;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0)); // 1 + 2
}

#[test]
fn test_debug_while_r_done_with_second_await() {
    // while (!r.done) with a second await - THE FAILING PATTERN
    // Let's try with just one yield to see if the second iteration is the issue
    let result = eval(
        r#"
        async function* source(): AsyncGenerator<number> {
            yield 1;
            // No second yield - r.done will be true after first next()
        }
        async function test(): Promise<number> {
            const gen = source();
            let sum = 0;
            let r = await gen.next();  // First: {value: 1, done: false}
            while (!r.done) {
                sum += r.value;
                r = await gen.next();  // Second: {value: undefined, done: true}
                // Loop should exit after this
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(1.0));
}

#[test]
fn test_debug_minimal_failing_case() {
    // Minimal reproduction: while with await and assignment
    let result = eval(
        r#"
        async function test(): Promise<number> {
            let x = 0;
            let r = { done: false, value: 1 };
            while (!r.done) {
                x += r.value;
                r = await Promise.resolve({ done: true, value: 0 });
            }
            return x;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(1.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// Debug tests for nested async generators
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_debug_nested_async_gen_simple() {
    // Simplest nested case: async gen that iterates over another async gen
    let result = eval(
        r#"
        async function* inner(): AsyncGenerator<number> {
            yield 1;
            yield 2;
        }
        async function* outer(): AsyncGenerator<number> {
            for await (const x of inner()) {
                yield x;
            }
        }
        async function test(): Promise<number> {
            let sum = 0;
            for await (const x of outer()) {
                sum += x;
            }
            return sum;
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0)); // 1 + 2
}

#[test]
fn test_debug_nested_async_gen_three_yields() {
    // Three yields to match the transform test
    let result = eval(
        r#"
        async function* inner(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
        }
        async function* outer(): AsyncGenerator<number> {
            for await (const x of inner()) {
                yield x;
            }
        }
        async function test(): Promise<number> {
            let sum = 0;
            for await (const x of outer()) {
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
fn test_debug_nested_async_gen_transform() {
    // Transform like the failing test
    let result = eval(
        r#"
        async function* inner(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
        }
        async function* outer(): AsyncGenerator<number> {
            for await (const x of inner()) {
                yield x * 2;
            }
        }
        async function test(): Promise<number> {
            let sum = 0;
            for await (const x of outer()) {
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
fn test_debug_nested_with_parameter() {
    // Pass generator as parameter like the failing test
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
fn test_debug_simple_double_manual() {
    // Manually iterate without nesting
    let result = eval(
        r#"
        async function* source(): AsyncGenerator<number> {
            yield 1;
            yield 2;
            yield 3;
        }
        async function test(): Promise<string> {
            const gen = source();
            const results: number[] = [];
            let r = await gen.next();
            while (!r.done) {
                results.push(r.value * 2);
                r = await gen.next();
            }
            return results.join(",");
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::String("2,4,6".into()));
}

#[test]
fn test_debug_double_gen_manual_iteration() {
    // Manually iterate over double(source()) to see what it produces
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
        async function test(): Promise<string> {
            const gen = double(source());
            const results: number[] = [];
            let r = await gen.next();
            while (!r.done) {
                results.push(r.value);
                r = await gen.next();
            }
            return results.join(",");
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::String("2,4,6".into()));
}

#[test]
fn test_debug_for_await_in_async_gen() {
    // for await inside an async generator without nesting the result
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number> {
            const arr = [Promise.resolve(10), Promise.resolve(20)];
            for await (const x of arr) {
                yield x;
            }
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
    assert_eq!(result, JsValue::Number(30.0)); // 10 + 20
}

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

#[test]
fn test_for_await_nested_for_of() {
    // Test for await with async generator and nested for-of
    let result = eval(
        r#"
        async function* gen(): AsyncGenerator<number[]> {
            yield [1, 2];
            yield [3, 4];
        }
        async function test(): Promise<string> {
            const all: number[] = [];
            for await (const arr of gen()) {
                for (const x of arr) {
                    all.push(x);
                }
            }
            return all.join(",");
        }
        await test()
    "#,
    );
    assert_eq!(result, JsValue::String("1,2,3,4".into()));
}
