//! Array-related tests

use super::{eval, eval_result};
use tsrun::value::JsString;
use tsrun::JsValue;

#[test]
fn test_array() {
    assert_eq!(eval("const arr = [1, 2, 3]; arr[1]"), JsValue::Number(2.0));
}

// Debug test to understand map issue
#[test]
fn test_debug_map_step_by_step() {
    // Step 1: Test callback is called
    assert_eq!(
        eval("let r = 0; [1].map(x => { r = x * 2; }); r"),
        JsValue::Number(2.0),
        "Callback should be called and set r"
    );

    // Step 2: Test that map returns something
    let result = eval("[1].map(x => x * 2)");
    println!("map result: {:?}", result);
    // assert it's an object (array)

    // Step 2b: Test length directly on result
    assert_eq!(
        eval("[1].map(x => x * 2).length"),
        JsValue::Number(1.0),
        "Mapped array should have length 1"
    );
}

// Array.prototype.push tests
#[test]
fn test_array_push_single() {
    assert_eq!(
        eval("const arr = [1, 2]; arr.push(3); arr.length"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_array_push_returns_length() {
    assert_eq!(
        eval("const arr = [1, 2]; arr.push(3)"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_array_push_multiple() {
    assert_eq!(
        eval("const arr = [1]; arr.push(2, 3, 4); arr.length"),
        JsValue::Number(4.0)
    );
}

#[test]
fn test_array_push_multiple_element_access() {
    // Verify each pushed element is accessible
    assert_eq!(
        eval("const arr: number[] = []; arr.push(1, 2, 3); arr[0]"),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval("const arr: number[] = []; arr.push(1, 2, 3); arr[1]"),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval("const arr: number[] = []; arr.push(1, 2, 3); arr[2]"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_array_push_multiple_sum() {
    // Bug reproduction: push multiple, then sum - was returning NaN
    assert_eq!(
        eval(
            r#"
            const arr: number[] = [];
            arr.push(1, 2, 3);
            let sum: number = 0;
            for (let i = 0; i < arr.length; i++) {
                sum = sum + arr[i];
            }
            sum
        "#
        ),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_array_push_objects_multiple() {
    // Bug reproduction: push multiple objects, access property - was causing NaN
    assert_eq!(
        eval(
            r#"
            interface Node { id: number; edges: Node[]; }
            const n1: Node = { id: 1, edges: [] };
            const n2: Node = { id: 2, edges: [] };
            const n3: Node = { id: 3, edges: [] };
            n1.edges.push(n2, n3);
            n1.edges[0].id + n1.edges[1].id
        "#
        ),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_gc_cycles_test6_minimal() {
    // Minimal reproduction of gc-cycles.ts Test 6 which was returning NaN
    // BUG: The interface declaration is inside the for loop!
    assert_eq!(
        eval(
            r#"
            let sum: number = 0;
            for (let i = 0; i < 1000; i++) {
                interface GraphNode { id: number; edges: GraphNode[]; }

                const n1: GraphNode = { id: 1, edges: [] };
                const n2: GraphNode = { id: 2, edges: [] };
                const n3: GraphNode = { id: 3, edges: [] };
                const n4: GraphNode = { id: 4, edges: [] };
                const n5: GraphNode = { id: 5, edges: [] };

                n1.edges.push(n2, n3);
                n2.edges.push(n1, n3, n4);
                n3.edges.push(n2, n4, n5);
                n4.edges.push(n3, n5);
                n5.edges.push(n4, n1);

                sum = sum + n1.id + n2.id + n3.id + n4.id + n5.id;
            }
            sum
        "#
        ),
        JsValue::Number(15000.0)
    );
}

#[test]
fn test_gc_cycles_test7_minimal() {
    // Minimal reproduction of gc-cycles.ts Test 7 which was returning NaN
    assert_eq!(
        eval(
            r#"
            interface ArrayNode { value: number; refs: ArrayNode[]; }
            let sum: number = 0;
            for (let i = 0; i < 2; i++) {
                const a: ArrayNode = { value: 1, refs: [] };
                const b: ArrayNode = { value: 2, refs: [] };
                const c: ArrayNode = { value: 3, refs: [] };

                a.refs.push(b, c);
                b.refs.push(c, a);
                c.refs.push(a, b);

                sum = sum + a.value + b.value + c.value;
            }
            sum
        "#
        ),
        JsValue::Number(12.0)
    );
}

#[test]
fn test_gc_cycles_all_tests() {
    // Run all gc-cycles tests in sequence to see if one corrupts state for later ones
    // Test 1: 100*(0+1+1+2+...+99+100) = sum(2i+1) for i=0..99 = 10000
    assert_eq!(
        eval(
            r#"
const results: number[] = [];

// Test 1: Two-node cycles
{
    let sum: number = 0;
    for (let i = 0; i < 100; i++) {
        const a: { id: number; other: any } = { id: i, other: null };
        const b: { id: number; other: any } = { id: i + 1, other: null };
        a.other = b;
        b.other = a;
        sum = sum + a.id + b.id;
    }
    results.push(sum);
}

// Test 6: Complex graph with multiple cycles
{
    let sum: number = 0;
    for (let i = 0; i < 100; i++) {
        interface GraphNode { id: number; edges: GraphNode[]; }
        const n1: GraphNode = { id: 1, edges: [] };
        const n2: GraphNode = { id: 2, edges: [] };
        const n3: GraphNode = { id: 3, edges: [] };
        const n4: GraphNode = { id: 4, edges: [] };
        const n5: GraphNode = { id: 5, edges: [] };

        n1.edges.push(n2, n3);
        n2.edges.push(n1, n3, n4);
        n3.edges.push(n2, n4, n5);
        n4.edges.push(n3, n5);
        n5.edges.push(n4, n1);

        sum = sum + n1.id + n2.id + n3.id + n4.id + n5.id;
    }
    results.push(sum);
}

// Test 7: Cycles through arrays
{
    let sum: number = 0;
    for (let i = 0; i < 100; i++) {
        interface ArrayNode { value: number; refs: ArrayNode[]; }
        const a: ArrayNode = { value: 1, refs: [] };
        const b: ArrayNode = { value: 2, refs: [] };
        const c: ArrayNode = { value: 3, refs: [] };

        a.refs.push(b, c);
        b.refs.push(c, a);
        c.refs.push(a, b);

        sum = sum + a.value + b.value + c.value;
    }
    results.push(sum);
}

// Return test 6 result as check (should be 1500)
results[1]
        "#
        ),
        JsValue::Number(1500.0)
    );
}

#[test]
fn test_array_push_element_access() {
    assert_eq!(
        eval("const arr = [1, 2]; arr.push(3); arr[2]"),
        JsValue::Number(3.0)
    );
}

// Array.prototype.pop tests
#[test]
fn test_array_pop_returns_last() {
    assert_eq!(
        eval("const arr = [1, 2, 3]; arr.pop()"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_array_pop_modifies_length() {
    assert_eq!(
        eval("const arr = [1, 2, 3]; arr.pop(); arr.length"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_array_pop_empty() {
    assert_eq!(eval("const arr = []; arr.pop()"), JsValue::Undefined);
}

// Array.prototype.map tests
#[test]
fn test_array_map_double() {
    // [1, 2, 3].map(x => x * 2) should equal [2, 4, 6]
    assert_eq!(
        eval("const arr = [1, 2, 3].map(x => x * 2); arr[0]"),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval("const arr = [1, 2, 3].map(x => x * 2); arr[1]"),
        JsValue::Number(4.0)
    );
    assert_eq!(
        eval("const arr = [1, 2, 3].map(x => x * 2); arr[2]"),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_array_map_preserves_length() {
    assert_eq!(
        eval("[1, 2, 3].map(x => x * 2).length"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_array_map_with_index() {
    // map callback receives (element, index, array)
    assert_eq!(
        eval("[10, 20, 30].map((x, i) => i)[1]"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_array_map_to_strings() {
    assert_eq!(
        eval("[1, 2, 3].map(x => 'n' + x)[0]"),
        JsValue::String(JsString::from("n1"))
    );
}

// Array.prototype.filter tests
#[test]
fn test_array_filter_evens() {
    assert_eq!(
        eval("[1, 2, 3, 4].filter(x => x % 2 === 0).length"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_array_filter_values() {
    assert_eq!(
        eval("[1, 2, 3, 4].filter(x => x % 2 === 0)[0]"),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval("[1, 2, 3, 4].filter(x => x % 2 === 0)[1]"),
        JsValue::Number(4.0)
    );
}

#[test]
fn test_array_filter_none_match() {
    assert_eq!(
        eval("[1, 2, 3].filter(x => x > 10).length"),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_array_filter_all_match() {
    assert_eq!(
        eval("[1, 2, 3].filter(x => x > 0).length"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_array_filter_with_index() {
    // Filter elements at even indices
    assert_eq!(
        eval("[10, 20, 30, 40].filter((x, i) => i % 2 === 0).length"),
        JsValue::Number(2.0)
    );
}

// Chaining tests
#[test]
fn test_array_map_filter_chain() {
    // [1, 2, 3, 4].map(x => x * 2).filter(x => x > 4) should be [6, 8]
    assert_eq!(
        eval("[1, 2, 3, 4].map(x => x * 2).filter(x => x > 4).length"),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval("[1, 2, 3, 4].map(x => x * 2).filter(x => x > 4)[0]"),
        JsValue::Number(6.0)
    );
}

// Array.prototype.forEach tests
#[test]
fn test_array_foreach_side_effect() {
    assert_eq!(
        eval("let sum = 0; [1, 2, 3].forEach(x => sum += x); sum"),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_array_foreach_returns_undefined() {
    assert_eq!(eval("[1, 2, 3].forEach(x => x * 2)"), JsValue::Undefined);
}

#[test]
fn test_array_foreach_with_index() {
    assert_eq!(
        eval("let result = 0; [10, 20, 30].forEach((x, i) => result += i); result"),
        JsValue::Number(3.0)
    );
}

// Array.prototype.reduce tests
#[test]
fn test_array_reduce_sum() {
    assert_eq!(
        eval("[1, 2, 3, 4].reduce((acc, x) => acc + x, 0)"),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_array_reduce_no_initial() {
    // Without initial value, uses first element as initial
    assert_eq!(
        eval("[1, 2, 3, 4].reduce((acc, x) => acc + x)"),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_array_reduce_multiply() {
    assert_eq!(
        eval("[1, 2, 3, 4].reduce((acc, x) => acc * x, 1)"),
        JsValue::Number(24.0)
    );
}

#[test]
fn test_array_reduce_with_index() {
    // Sum of indices
    assert_eq!(
        eval("[10, 20, 30].reduce((acc, x, i) => acc + i, 0)"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_array_reduce_to_object() {
    assert_eq!(
        eval("const obj = [['a', 1], ['b', 2]].reduce((acc, [k, v]) => { acc[k] = v; return acc; }, {}); obj.a"),
        JsValue::Number(1.0)
    );
}

// Array.prototype.find tests
#[test]
fn test_array_find_found() {
    assert_eq!(eval("[1, 2, 3, 4].find(x => x > 2)"), JsValue::Number(3.0));
}

#[test]
fn test_array_find_not_found() {
    assert_eq!(eval("[1, 2, 3].find(x => x > 10)"), JsValue::Undefined);
}

#[test]
fn test_array_find_with_index() {
    assert_eq!(
        eval("[10, 20, 30].find((x, i) => i === 1)"),
        JsValue::Number(20.0)
    );
}

// Array.prototype.findIndex tests
#[test]
fn test_array_findindex_found() {
    assert_eq!(
        eval("[1, 2, 3, 4].findIndex(x => x > 2)"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_array_findindex_not_found() {
    assert_eq!(
        eval("[1, 2, 3].findIndex(x => x > 10)"),
        JsValue::Number(-1.0)
    );
}

#[test]
fn test_array_findindex_first() {
    assert_eq!(
        eval("[5, 10, 15].findIndex(x => x >= 5)"),
        JsValue::Number(0.0)
    );
}

// Array.prototype.indexOf tests
#[test]
fn test_array_indexof_found() {
    assert_eq!(eval("[1, 2, 3, 4].indexOf(3)"), JsValue::Number(2.0));
}

#[test]
fn test_array_indexof_not_found() {
    assert_eq!(eval("[1, 2, 3].indexOf(5)"), JsValue::Number(-1.0));
}

#[test]
fn test_array_indexof_first_occurrence() {
    assert_eq!(eval("[1, 2, 3, 2, 1].indexOf(2)"), JsValue::Number(1.0));
}

#[test]
fn test_array_indexof_from_index() {
    assert_eq!(eval("[1, 2, 3, 2, 1].indexOf(2, 2)"), JsValue::Number(3.0));
}

// Array.prototype.includes tests
#[test]
fn test_array_includes_found() {
    assert_eq!(eval("[1, 2, 3].includes(2)"), JsValue::Boolean(true));
}

#[test]
fn test_array_includes_not_found() {
    assert_eq!(eval("[1, 2, 3].includes(5)"), JsValue::Boolean(false));
}

#[test]
fn test_array_includes_from_index() {
    assert_eq!(eval("[1, 2, 3].includes(1, 1)"), JsValue::Boolean(false));
}

// Array.prototype.slice tests
#[test]
fn test_array_slice_basic() {
    assert_eq!(
        eval("[1, 2, 3, 4, 5].slice(1, 4).length"),
        JsValue::Number(3.0)
    );
    assert_eq!(eval("[1, 2, 3, 4, 5].slice(1, 4)[0]"), JsValue::Number(2.0));
}

#[test]
fn test_array_slice_no_args() {
    assert_eq!(eval("[1, 2, 3].slice().length"), JsValue::Number(3.0));
}

#[test]
fn test_array_slice_negative() {
    assert_eq!(
        eval("[1, 2, 3, 4, 5].slice(-2).length"),
        JsValue::Number(2.0)
    );
    assert_eq!(eval("[1, 2, 3, 4, 5].slice(-2)[0]"), JsValue::Number(4.0));
}

#[test]
fn test_array_slice_start_only() {
    assert_eq!(eval("[1, 2, 3, 4].slice(2).length"), JsValue::Number(2.0));
}

// Array.prototype.concat tests
#[test]
fn test_array_concat_arrays() {
    assert_eq!(eval("[1, 2].concat([3, 4]).length"), JsValue::Number(4.0));
    assert_eq!(eval("[1, 2].concat([3, 4])[2]"), JsValue::Number(3.0));
}

#[test]
fn test_array_concat_values() {
    assert_eq!(eval("[1, 2].concat(3, 4).length"), JsValue::Number(4.0));
}

#[test]
fn test_array_concat_mixed() {
    assert_eq!(
        eval("[1].concat([2, 3], 4, [5]).length"),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_array_concat_is_concat_spreadable_false() {
    // When Symbol.isConcatSpreadable is false, the array should not be spread
    assert_eq!(
        eval(
            r#"
            const arr = [1, 2, 3];
            arr[Symbol.isConcatSpreadable] = false;
            [].concat(arr).length
        "#
        ),
        JsValue::Number(1.0) // arr is treated as a single element, not spread
    );
}

#[test]
fn test_array_concat_is_concat_spreadable_true() {
    // When Symbol.isConcatSpreadable is explicitly true, array-like objects are spread
    assert_eq!(
        eval(
            r#"
            const obj = { 0: 'a', 1: 'b', length: 2, [Symbol.isConcatSpreadable]: true };
            [].concat(obj).length
        "#
        ),
        JsValue::Number(2.0)
    );

    assert_eq!(
        eval(
            r#"
            const obj = { 0: 'a', 1: 'b', length: 2, [Symbol.isConcatSpreadable]: true };
            [].concat(obj)[0]
        "#
        ),
        JsValue::from("a")
    );
}

#[test]
fn test_array_concat_non_array_without_spreadable() {
    // Plain objects without Symbol.isConcatSpreadable are not spread
    assert_eq!(
        eval(
            r#"
            const obj = { 0: 'a', 1: 'b', length: 2 };
            [].concat(obj).length
        "#
        ),
        JsValue::Number(1.0)
    );
}

// Array.prototype.join tests
#[test]
fn test_array_join_default() {
    assert_eq!(
        eval("[1, 2, 3].join()"),
        JsValue::String(JsString::from("1,2,3"))
    );
}

#[test]
fn test_array_join_custom_separator() {
    assert_eq!(
        eval("[1, 2, 3].join('-')"),
        JsValue::String(JsString::from("1-2-3"))
    );
}

#[test]
fn test_array_join_empty() {
    assert_eq!(
        eval("[1, 2, 3].join('')"),
        JsValue::String(JsString::from("123"))
    );
}

// Array.prototype.every tests
#[test]
fn test_array_every_all_pass() {
    assert_eq!(
        eval("[2, 4, 6].every(x => x % 2 === 0)"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_array_every_some_fail() {
    assert_eq!(
        eval("[2, 3, 6].every(x => x % 2 === 0)"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_array_every_empty() {
    assert_eq!(eval("[].every(x => false)"), JsValue::Boolean(true));
}

// Array.prototype.some tests
#[test]
fn test_array_some_one_passes() {
    assert_eq!(eval("[1, 2, 3].some(x => x > 2)"), JsValue::Boolean(true));
}

#[test]
fn test_array_some_none_pass() {
    assert_eq!(eval("[1, 2, 3].some(x => x > 10)"), JsValue::Boolean(false));
}

#[test]
fn test_array_some_empty() {
    assert_eq!(eval("[].some(x => true)"), JsValue::Boolean(false));
}

// Array.prototype.shift tests
#[test]
fn test_array_shift() {
    assert_eq!(eval("let a = [1, 2, 3]; a.shift()"), JsValue::Number(1.0));
    assert_eq!(
        eval("let a = [1, 2, 3]; a.shift(); a.length"),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval("let a = [1, 2, 3]; a.shift(); a[0]"),
        JsValue::Number(2.0)
    );
    assert_eq!(eval("let a = []; a.shift()"), JsValue::Undefined);
}

// Array.prototype.unshift tests
#[test]
fn test_array_unshift() {
    assert_eq!(
        eval("let a = [1, 2, 3]; a.unshift(0)"),
        JsValue::Number(4.0)
    );
    assert_eq!(
        eval("let a = [1, 2, 3]; a.unshift(0); a[0]"),
        JsValue::Number(0.0)
    );
    assert_eq!(
        eval("let a = [1, 2, 3]; a.unshift(-1, 0); a.length"),
        JsValue::Number(5.0)
    );
}

// Array.prototype.reverse tests
#[test]
fn test_array_reverse() {
    assert_eq!(
        eval("let a = [1, 2, 3]; a.reverse(); a[0]"),
        JsValue::Number(3.0)
    );
    assert_eq!(
        eval("let a = [1, 2, 3]; a.reverse(); a[2]"),
        JsValue::Number(1.0)
    );
}

// Array.prototype.sort tests
#[test]
fn test_array_sort() {
    assert_eq!(
        eval("let a = [3, 1, 2]; a.sort(); a[0]"),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval("let a = [3, 1, 2]; a.sort(); a[2]"),
        JsValue::Number(3.0)
    );
    assert_eq!(
        eval("let a = ['c', 'a', 'b']; a.sort(); a[0]"),
        JsValue::String(JsString::from("a"))
    );
    // Sort with comparator
    assert_eq!(
        eval("let a = [3, 1, 2]; a.sort((a, b) => b - a); a[0]"),
        JsValue::Number(3.0)
    );
}

// Array.prototype.fill tests
#[test]
fn test_array_fill() {
    assert_eq!(
        eval("let a = [1, 2, 3]; a.fill(0); a[1]"),
        JsValue::Number(0.0)
    );
    assert_eq!(
        eval("let a = [1, 2, 3]; a.fill(0, 1); a[0]"),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval("let a = [1, 2, 3]; a.fill(0, 1); a[1]"),
        JsValue::Number(0.0)
    );
    assert_eq!(
        eval("let a = [1, 2, 3]; a.fill(0, 1, 2); a[2]"),
        JsValue::Number(3.0)
    );
}

// Array.prototype.copyWithin tests
#[test]
fn test_array_copywithin() {
    assert_eq!(
        eval("let a = [1, 2, 3, 4, 5]; a.copyWithin(0, 3); a[0]"),
        JsValue::Number(4.0)
    );
    assert_eq!(
        eval("let a = [1, 2, 3, 4, 5]; a.copyWithin(0, 3); a[1]"),
        JsValue::Number(5.0)
    );
}

// Array.prototype.splice tests
#[test]
fn test_array_splice() {
    assert_eq!(
        eval("let a = [1, 2, 3]; let r = a.splice(1, 1); r[0]"),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval("let a = [1, 2, 3]; a.splice(1, 1); a.length"),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval("let a = [1, 2, 3]; a.splice(1, 1, 'a', 'b'); a.length"),
        JsValue::Number(4.0)
    );
    assert_eq!(
        eval("let a = [1, 2, 3]; a.splice(1, 1, 'a', 'b'); a[1]"),
        JsValue::String(JsString::from("a"))
    );
}

// Array.of tests
#[test]
fn test_array_of() {
    assert_eq!(eval("Array.of(1, 2, 3).length"), JsValue::Number(3.0));
    assert_eq!(eval("Array.of(1, 2, 3)[0]"), JsValue::Number(1.0));
    assert_eq!(eval("Array.of(7).length"), JsValue::Number(1.0));
    assert_eq!(eval("Array.of().length"), JsValue::Number(0.0));
}

// Array.from tests
#[test]
fn test_array_from() {
    assert_eq!(eval("Array.from([1, 2, 3]).length"), JsValue::Number(3.0));
    assert_eq!(eval("Array.from([1, 2, 3])[1]"), JsValue::Number(2.0));
    // With map function
    assert_eq!(
        eval("Array.from([1, 2, 3], x => x * 2)[1]"),
        JsValue::Number(4.0)
    );
}

// Array.prototype.at tests
#[test]
fn test_array_at() {
    assert_eq!(eval("[1, 2, 3].at(0)"), JsValue::Number(1.0));
    assert_eq!(eval("[1, 2, 3].at(2)"), JsValue::Number(3.0));
    assert_eq!(eval("[1, 2, 3].at(-1)"), JsValue::Number(3.0));
    assert_eq!(eval("[1, 2, 3].at(-2)"), JsValue::Number(2.0));
    assert_eq!(eval("[1, 2, 3].at(5)"), JsValue::Undefined);
}

#[test]
fn test_array_at_valueof() {
    // Test that valueOf is called on the index argument (ToIntegerOrInfinity)
    assert_eq!(
        eval(
            r#"
            let a = [0, 1, 2, 3];
            let index = { valueOf() { return 1; } };
            a.at(index);
            "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_array_at_symbol_throws() {
    // Symbol cannot be converted to number - should throw TypeError
    let result = eval_result("[1, 2, 3].at(Symbol())");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("TypeError") || err.to_string().contains("Symbol"));
}

#[test]
fn test_array_at_function_length() {
    // Array.prototype.at.length should be 1 (one formal parameter)
    assert_eq!(eval("Array.prototype.at.length"), JsValue::Number(1.0));
}

#[test]
fn test_array_at_function_name() {
    // Array.prototype.at.name should be "at"
    assert_eq!(
        eval("Array.prototype.at.name"),
        JsValue::String(JsString::from("at"))
    );
}

#[test]
fn test_function_call_bind() {
    // Test Function.prototype.call.bind() pattern used by test262 harness
    assert_eq!(
        eval(
            r#"
            var __hasOwnProperty = Function.prototype.call.bind(Object.prototype.hasOwnProperty);
            __hasOwnProperty({a: 1}, 'a');
            "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_get_own_property_descriptor_function_length() {
    // Test Object.getOwnPropertyDescriptor on function length property
    assert_eq!(
        eval(
            r#"
            var desc = Object.getOwnPropertyDescriptor(Array.prototype.at, 'length');
            desc.value;
            "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_function_length_descriptor_attributes() {
    // Test that function.length has correct property descriptor attributes
    // Per spec: writable: false, enumerable: false, configurable: true
    assert_eq!(
        eval(
            r#"
            var desc = Object.getOwnPropertyDescriptor(Array.prototype.at, 'length');
            [desc.writable, desc.enumerable, desc.configurable].join(',');
            "#
        ),
        JsValue::String(JsString::from("false,false,true"))
    );
}

#[test]
fn test_function_name_descriptor_attributes() {
    // Test that function.name has correct property descriptor attributes
    // Per spec: writable: false, enumerable: false, configurable: true
    assert_eq!(
        eval(
            r#"
            var desc = Object.getOwnPropertyDescriptor(Array.prototype.at, 'name');
            [desc.value, desc.writable, desc.enumerable, desc.configurable].join(',');
            "#
        ),
        JsValue::String(JsString::from("at,false,false,true"))
    );
}

#[test]
fn test_verify_property_pattern() {
    // Test the exact pattern used by test262 propertyHelper.js
    assert_eq!(
        eval(
            r#"
            // Capture primordials like propertyHelper.js does
            var __getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
            var __hasOwnProperty = Function.prototype.call.bind(Object.prototype.hasOwnProperty);

            // Verify length property like the test does
            var obj = Array.prototype.at;
            var name = "length";
            var originalDesc = __getOwnPropertyDescriptor(obj, name);

            // Return the result
            [
                originalDesc.value,
                originalDesc.writable,
                originalDesc.enumerable,
                originalDesc.configurable
            ].join(',');
            "#
        ),
        JsValue::String(JsString::from("1,false,false,true"))
    );
}

#[test]
fn test_full_verify_property_simulation() {
    // Simulate the full verifyProperty call from test262
    assert_eq!(
        eval(
            r#"
            // From propertyHelper.js
            var __hasOwnProperty = Function.prototype.call.bind(Object.prototype.hasOwnProperty);
            var __getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;

            function verifyProperty(obj, name, desc) {
                var originalDesc = __getOwnPropertyDescriptor(obj, name);
                var nameStr = String(name);

                if (!__hasOwnProperty(obj, name)) {
                    throw new Error("obj should have own property " + nameStr);
                }

                if (__hasOwnProperty(desc, 'value')) {
                    if (desc.value !== originalDesc.value) {
                        throw new Error("Expected " + nameStr + " to have value " + desc.value + " but got " + originalDesc.value);
                    }
                }

                return true;
            }

            // The actual test
            verifyProperty(Array.prototype.at, "length", { value: 1 });
            "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_assert_samevalue_pattern() {
    // Test assert.sameValue pattern from test262
    assert_eq!(
        eval(
            r#"
            // From assert.js
            function assert(mustBeTrue, message) {
                if (mustBeTrue === true) return;
                throw new Error(message || 'Expected true');
            }

            assert._isSameValue = function(a, b) {
                if (a === b) {
                    return a !== 0 || 1 / a === 1 / b;
                }
                return a !== a && b !== b;
            };

            assert.sameValue = function(actual, expected, message) {
                if (assert._isSameValue(actual, expected)) return;
                throw new Error(message || 'Expected same value');
            };

            // Test what the test262 test does
            assert.sameValue(typeof Array.prototype.at, 'function');
            "passed";
            "#
        ),
        JsValue::String(JsString::from("passed"))
    );
}

#[test]
fn test_array_map_call() {
    // Test Array.prototype.map.call pattern used in assert.js
    assert_eq!(
        eval(
            r#"
            var result = Array.prototype.map.call([1, 2, 3], String).join(', ');
            result;
            "#
        ),
        JsValue::String(JsString::from("1, 2, 3"))
    );
}

// Array.prototype.lastIndexOf tests
#[test]
fn test_array_lastindexof() {
    assert_eq!(eval("[1, 2, 3, 2, 1].lastIndexOf(2)"), JsValue::Number(3.0));
    assert_eq!(eval("[1, 2, 3].lastIndexOf(4)"), JsValue::Number(-1.0));
}

// Array.prototype.reduceRight tests
#[test]
fn test_array_reduceright() {
    assert_eq!(
        eval("[1, 2, 3].reduceRight((acc, x) => acc + x, 0)"),
        JsValue::Number(6.0)
    );
    assert_eq!(
        eval("['a', 'b', 'c'].reduceRight((acc, x) => acc + x, '')"),
        JsValue::String(JsString::from("cba"))
    );
}

// Array.prototype.flat tests
#[test]
fn test_array_flat() {
    assert_eq!(eval("[[1, 2], [3, 4]].flat()[0]"), JsValue::Number(1.0));
    assert_eq!(eval("[[1, 2], [3, 4]].flat().length"), JsValue::Number(4.0));
    assert_eq!(eval("[1, [2, [3]]].flat(2).length"), JsValue::Number(3.0));
}

// Array.prototype.flatMap tests
#[test]
fn test_array_flatmap() {
    assert_eq!(
        eval("[1, 2, 3].flatMap(x => [x, x * 2]).length"),
        JsValue::Number(6.0)
    );
    assert_eq!(
        eval("[1, 2, 3].flatMap(x => [x, x * 2])[1]"),
        JsValue::Number(2.0)
    );
}

// Array.prototype.findLast tests
#[test]
fn test_array_findlast() {
    assert_eq!(
        eval("[1, 2, 3, 2].findLast(x => x === 2)"),
        JsValue::Number(2.0)
    );
    assert_eq!(eval("[1, 2, 3].findLast(x => x > 1)"), JsValue::Number(3.0));
    assert_eq!(eval("[1, 2, 3].findLast(x => x > 10)"), JsValue::Undefined);
}

// Array.prototype.findLastIndex tests
#[test]
fn test_array_findlastindex() {
    assert_eq!(
        eval("[1, 2, 3, 2].findLastIndex(x => x === 2)"),
        JsValue::Number(3.0)
    );
    assert_eq!(
        eval("[1, 2, 3].findLastIndex(x => x > 1)"),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval("[1, 2, 3].findLastIndex(x => x > 10)"),
        JsValue::Number(-1.0)
    );
}

// Array.prototype.toReversed tests
#[test]
fn test_array_toreversed() {
    assert_eq!(
        eval("let a = [1, 2, 3]; let b = a.toReversed(); b[0]"),
        JsValue::Number(3.0)
    );
    assert_eq!(
        eval("let a = [1, 2, 3]; let b = a.toReversed(); a[0]"),
        JsValue::Number(1.0)
    ); // Original unchanged
}

// Array.prototype.toSorted tests
#[test]
fn test_array_tosorted() {
    assert_eq!(
        eval("let a = [3, 1, 2]; let b = a.toSorted(); b[0]"),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval("let a = [3, 1, 2]; let b = a.toSorted(); a[0]"),
        JsValue::Number(3.0)
    ); // Original unchanged
}

// Array.prototype.toSpliced tests
#[test]
fn test_array_tospliced() {
    assert_eq!(
        eval("[1, 2, 3].toSpliced(1, 1, 'a', 'b')[1]"),
        JsValue::String(JsString::from("a"))
    );
    assert_eq!(
        eval("[1, 2, 3].toSpliced(1, 1, 'a', 'b').length"),
        JsValue::Number(4.0)
    );
}

// Array.prototype.with tests
#[test]
fn test_array_with() {
    assert_eq!(
        eval("[1, 2, 3].with(1, 'x')[1]"),
        JsValue::String(JsString::from("x"))
    );
    assert_eq!(
        eval("let a = [1, 2, 3]; let b = a.with(1, 'x'); a[1]"),
        JsValue::Number(2.0)
    ); // Original unchanged
}

// Array.prototype.keys tests
#[test]
fn test_array_keys() {
    // keys() returns an array of indices
    assert_eq!(
        eval("let arr: string[] = ['a', 'b', 'c']; let keys: number[] = arr.keys(); keys[0]"),
        JsValue::Number(0.0)
    );
    assert_eq!(
        eval("let arr: string[] = ['a', 'b', 'c']; let keys: number[] = arr.keys(); keys[2]"),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval("([1, 2, 3] as number[]).keys().length"),
        JsValue::Number(3.0)
    );
}

// Array.prototype.values tests
#[test]
fn test_array_values() {
    // values() returns an iterator - use next() to get values
    assert_eq!(
        eval("let arr: string[] = ['a', 'b', 'c']; let iter = arr.values(); iter.next().value"),
        JsValue::String(JsString::from("a"))
    );
    assert_eq!(
        eval("let iter = ([1, 2, 3] as number[]).values(); iter.next(); iter.next().value"),
        JsValue::Number(2.0)
    );
    // Iterator should be exhaustible
    assert_eq!(
        eval("let iter = [42].values(); iter.next(); iter.next().done"),
        JsValue::Boolean(true)
    );
}

// Array.prototype.entries tests
#[test]
fn test_array_entries() {
    // entries() returns an array of [index, value] pairs
    assert_eq!(
        eval("let arr: string[] = ['a', 'b']; let entries = arr.entries(); entries[0][0]"),
        JsValue::Number(0.0)
    );
    assert_eq!(
        eval("let arr: string[] = ['a', 'b']; let entries = arr.entries(); entries[0][1]"),
        JsValue::String(JsString::from("a"))
    );
    assert_eq!(
        eval("let arr: string[] = ['a', 'b']; let entries = arr.entries(); entries[1][0]"),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval("([1, 2, 3] as number[]).entries().length"),
        JsValue::Number(3.0)
    );
}

// Array holes tests
#[test]
fn test_array_holes_basic() {
    // Holes should be undefined when accessed
    assert_eq!(
        eval("const arr: (number | undefined)[] = [1, , 3]; arr[1]"),
        JsValue::Undefined
    );
}

#[test]
fn test_array_holes_length() {
    // Holes count toward length
    assert_eq!(
        eval("const arr: (number | undefined)[] = [1, , 3]; arr.length"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_array_holes_at_start() {
    assert_eq!(
        eval("const arr: (number | undefined)[] = [, 1, 2]; arr[0]"),
        JsValue::Undefined
    );
    assert_eq!(
        eval("const arr: (number | undefined)[] = [, 1, 2]; arr[1]"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_array_holes_multiple() {
    assert_eq!(
        eval("const arr: (number | undefined)[] = [, , 3, , 5]; arr.length"),
        JsValue::Number(5.0)
    );
    assert_eq!(
        eval("const arr: (number | undefined)[] = [, , 3, , 5]; arr[2]"),
        JsValue::Number(3.0)
    );
    assert_eq!(
        eval("const arr: (number | undefined)[] = [, , 3, , 5]; arr[3]"),
        JsValue::Undefined
    );
}

#[test]
fn test_array_holes_trailing_comma() {
    // Trailing comma doesn't create a hole
    assert_eq!(
        eval("const arr: number[] = [1, 2, ]; arr.length"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_reduce_with_object_destructuring() {
    // Reduce with object destructuring in callback
    assert_eq!(
        eval(
            r#"
            const products = [
                { price: 10, stock: 5 },
                { price: 20, stock: 3 },
            ];
            products.reduce((total, { price, stock }) => total + price * stock, 0)
        "#
        ),
        JsValue::Number(110.0)
    );
}

#[test]
fn test_reduce_grouping_pattern() {
    // Common grouping pattern using reduce
    assert_eq!(
        eval(
            r#"
            const products = [
                { id: 1, category: "X" },
                { id: 2, category: "Y" },
                { id: 3, category: "X" },
            ];
            const grouped = products.reduce((groups, product) => {
                const category = product.category;
                if (!groups[category]) {
                    groups[category] = [];
                }
                groups[category].push(product);
                return groups;
            }, {});
            Object.keys(grouped).length
        "#
        ),
        JsValue::Number(2.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Algorithm tests (complex array operations)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_array_operations_chain() {
    // Test chained array operations
    assert_eq!(
        eval(
            r#"
            const data: number[] = [1, 2, 3, 4, 5];
            const result = data
                .filter(x => x > 2)
                .map(x => x * 2)
                .reduce((sum, x) => sum + x, 0);
            result
        "#
        ),
        JsValue::Number(24.0) // (3+4+5)*2 = 24
    );
}

#[test]
fn test_quicksort() {
    let result = eval(
        r#"
        function quickSort(arr: number[]): number[] {
            if (arr.length <= 1) return arr;
            const pivot = arr[Math.floor(arr.length / 2)];
            const left = arr.filter((x) => x < pivot);
            const middle = arr.filter((x) => x === pivot);
            const right = arr.filter((x) => x > pivot);
            return [...quickSort(left), ...middle, ...quickSort(right)];
        }
        quickSort([64, 34, 25, 12]).join(",")
    "#,
    );
    assert_eq!(result, JsValue::String("12,25,34,64".into()));
}

#[test]
fn test_array_spread_sort() {
    // Test spreading into array and sorting
    assert_eq!(
        eval(
            r#"
            const arr: number[] = [3, 1, 2];
            const sorted = [...arr].sort((a, b) => a - b);
            sorted.join(",")
        "#
        ),
        JsValue::String("1,2,3".into())
    );
}

#[test]
fn test_debug_array_method_lookup() {
    use super::eval_result;

    // Test 1: Check if map method exists on array
    let result = eval_result("const arr = [1, 2, 3]; typeof arr.map");
    println!("typeof arr.map: {:?}", result);

    // Test 2: Check if push method exists on array
    let result2 = eval_result("const arr = [1, 2, 3]; typeof arr.push");
    println!("typeof arr.push: {:?}", result2);

    // Test 3: Try calling map with a simple function
    let result3 = eval_result("[1, 2, 3].map(function(x) { return x * 2; })");
    println!("map with function: {:?}", result3);

    // Test 4: Try calling map with arrow function
    let result4 = eval_result("[1, 2, 3].map(x => x * 2)");
    println!("map with arrow: {:?}", result4);

    // Test 5: Check what arr.map actually is
    let result5 = eval_result("const arr = [1, 2, 3]; arr.map");
    println!("arr.map value: {:?}", result5);

    // Test 6: Check hasOwnProperty for map
    let result6 = eval_result("const arr = [1, 2, 3]; arr.hasOwnProperty('map')");
    println!("arr.hasOwnProperty('map'): {:?}", result6);

    // Test 7: Check hasOwnProperty for push
    let result7 = eval_result("const arr = [1, 2, 3]; arr.hasOwnProperty('push')");
    println!("arr.hasOwnProperty('push'): {:?}", result7);

    // Test 8: Check Object.keys on array
    let result8 = eval_result("const arr = [1, 2, 3]; Object.keys(arr).length");
    println!("Object.keys(arr).length: {:?}", result8);

    // Test 9: Test with stored reference vs literal
    let result9 = eval_result("const arr = [1, 2, 3]; const m = arr.map; typeof m");
    println!("stored method typeof: {:?}", result9);

    // Test 10: Try calling stored reference
    let result10 = eval_result("const arr = [1, 2, 3]; const m = arr.map; m.call(arr, x => x * 2)");
    println!("m.call(arr, fn): {:?}", result10);

    // Test 11: Check prototype explicitly
    let result11 = eval_result("const arr = [1, 2, 3]; Object.getPrototypeOf(arr) !== null");
    println!("has prototype: {:?}", result11);

    // Test 12: push works via direct call
    let result12 = eval_result("const arr = [1, 2, 3]; arr.push(4)");
    println!("arr.push(4): {:?}", result12);

    // Test 13: Literal array direct method call
    let result13 = eval_result("[1, 2, 3].push(4)");
    println!("[1, 2, 3].push(4): {:?}", result13);

    // Test 14: Literal array direct method call with map
    let result14 = eval_result("[1, 2, 3].push");
    println!("[1, 2, 3].push: {:?}", result14);

    // Test 15: Simplest possible callback
    let result15 = eval_result("const arr = [1]; arr.map(function(x) { return x; })");
    println!("arr.map with simplest callback: {:?}", result15);

    // Assert something to make test visible
    assert!(
        result.is_ok() || result.is_err(),
        "This test is for debugging"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Array methods on array-like objects (Test262 conformance)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_array_map_on_array_like() {
    // Array.prototype.map should work on array-like objects
    assert_eq!(
        eval(
            r#"
            const obj: { length: number; 0: number; 1: number } = { length: 2, 0: 10, 1: 20 };
            const result: number[] = Array.prototype.map.call(obj, function(x: number): number { return x * 2; });
            result[0]
            "#
        ),
        JsValue::Number(20.0)
    );
}

#[test]
fn test_array_map_on_array_like_result_length() {
    assert_eq!(
        eval(
            r#"
            const obj: { length: number; 0: number; 1: number } = { length: 2, 0: 10, 1: 20 };
            const result: number[] = Array.prototype.map.call(obj, function(x: number): number { return x * 2; });
            result.length
            "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_array_filter_on_array_like() {
    assert_eq!(
        eval(
            r#"
            const obj: { length: number; 0: number; 1: number; 2: number } = { length: 3, 0: 1, 1: 2, 2: 3 };
            const result: number[] = Array.prototype.filter.call(obj, function(x: number): boolean { return x > 1; });
            result.length
            "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_array_foreach_on_array_like() {
    assert_eq!(
        eval(
            r#"
            const obj: { length: number; 0: number; 1: number } = { length: 2, 0: 10, 1: 20 };
            let sum: number = 0;
            Array.prototype.forEach.call(obj, function(x: number): void { sum += x; });
            sum
            "#
        ),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_array_reduce_on_array_like() {
    assert_eq!(
        eval(
            r#"
            const obj: { length: number; 0: number; 1: number; 2: number } = { length: 3, 0: 1, 1: 2, 2: 3 };
            Array.prototype.reduce.call(obj, function(acc: number, x: number): number { return acc + x; }, 0)
            "#
        ),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_array_some_on_array_like() {
    assert_eq!(
        eval(
            r#"
            const obj: { length: number; 0: number; 1: number } = { length: 2, 0: 1, 1: 5 };
            Array.prototype.some.call(obj, function(x: number): boolean { return x > 3; })
            "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_array_every_on_array_like() {
    assert_eq!(
        eval(
            r#"
            const obj: { length: number; 0: number; 1: number } = { length: 2, 0: 1, 1: 2 };
            Array.prototype.every.call(obj, function(x: number): boolean { return x > 0; })
            "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_array_find_on_array_like() {
    assert_eq!(
        eval(
            r#"
            const obj: { length: number; 0: number; 1: number; 2: number } = { length: 3, 0: 1, 1: 5, 2: 10 };
            Array.prototype.find.call(obj, function(x: number): boolean { return x > 3; })
            "#
        ),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_array_indexof_on_array_like() {
    assert_eq!(
        eval(
            r#"
            const obj: { length: number; 0: string; 1: string; 2: string } = { length: 3, 0: "a", 1: "b", 2: "c" };
            Array.prototype.indexOf.call(obj, "b")
            "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_array_includes_on_array_like() {
    assert_eq!(
        eval(
            r#"
            const obj: { length: number; 0: number; 1: number } = { length: 2, 0: 10, 1: 20 };
            Array.prototype.includes.call(obj, 20)
            "#
        ),
        JsValue::Boolean(true)
    );
}

// =============================================================================
// Array-like object tests with ToLength coercion
// =============================================================================
// Per ECMAScript spec, Array methods should work with array-like objects.
// The `length` property is coerced via ToLength, which calls ToNumber first.
// This means objects with valueOf/toString should work.

#[test]
fn test_array_map_on_array_like_with_object_length() {
    // Test262: 15.4.4.19-3-19
    // length property is an object with toString returning a number string
    assert_eq!(
        eval(
            r#"
            function callbackfn(val: number, idx: number, obj: any): boolean {
                return val < 10;
            }
            const obj: any = {
                0: 11,
                1: 9,
                length: {
                    toString: function(): string {
                        return '2';
                    }
                }
            };
            const newArr: boolean[] = Array.prototype.map.call(obj, callbackfn);
            newArr.length
            "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_array_map_on_array_like_with_valueof_length() {
    // length property is an object with valueOf returning a number
    assert_eq!(
        eval(
            r#"
            const obj: any = {
                0: 'a',
                1: 'b',
                2: 'c',
                length: {
                    valueOf: function(): number {
                        return 3;
                    }
                }
            };
            const newArr: string[] = Array.prototype.map.call(obj, (x: string) => x.toUpperCase());
            newArr.length
            "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_array_map_on_array_like_valueof_result() {
    // Verify the actual mapped values
    assert_eq!(
        eval(
            r#"
            const obj: any = {
                0: 'a',
                1: 'b',
                length: { valueOf: () => 2 }
            };
            const arr: string[] = Array.prototype.map.call(obj, (x: string) => x.toUpperCase());
            arr[0] + arr[1]
            "#
        ),
        JsValue::String(JsString::from("AB"))
    );
}

#[test]
fn test_array_filter_on_array_like_with_object_length() {
    assert_eq!(
        eval(
            r#"
            const obj: any = {
                0: 1,
                1: 2,
                2: 3,
                3: 4,
                length: { toString: () => '4' }
            };
            const arr: number[] = Array.prototype.filter.call(obj, (x: number) => x > 2);
            arr.length
            "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_array_filter_on_array_like_result_values() {
    assert_eq!(
        eval(
            r#"
            const obj: any = {
                0: 1,
                1: 2,
                2: 3,
                length: { valueOf: () => 3 }
            };
            const arr: number[] = Array.prototype.filter.call(obj, (x: number) => x >= 2);
            arr[0] + arr[1]
            "#
        ),
        JsValue::Number(5.0) // 2 + 3
    );
}

#[test]
fn test_array_foreach_on_array_like_with_object_length() {
    assert_eq!(
        eval(
            r#"
            const obj: any = {
                0: 10,
                1: 20,
                2: 30,
                length: { toString: () => '3' }
            };
            let sum: number = 0;
            Array.prototype.forEach.call(obj, (x: number) => { sum = sum + x; });
            sum
            "#
        ),
        JsValue::Number(60.0)
    );
}

#[test]
fn test_array_reduce_on_array_like_with_object_length() {
    assert_eq!(
        eval(
            r#"
            const obj: any = {
                0: 1,
                1: 2,
                2: 3,
                length: { valueOf: () => 3 }
            };
            Array.prototype.reduce.call(obj, (acc: number, x: number) => acc + x, 0)
            "#
        ),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_array_every_on_array_like_with_object_length() {
    assert_eq!(
        eval(
            r#"
            const obj: any = {
                0: 2,
                1: 4,
                2: 6,
                length: { toString: () => '3' }
            };
            Array.prototype.every.call(obj, (x: number) => x % 2 === 0)
            "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_array_some_on_array_like_with_object_length() {
    assert_eq!(
        eval(
            r#"
            const obj: any = {
                0: 1,
                1: 3,
                2: 5,
                length: { valueOf: () => 3 }
            };
            Array.prototype.some.call(obj, (x: number) => x > 4)
            "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_array_find_on_array_like_with_object_length() {
    assert_eq!(
        eval(
            r#"
            const obj: any = {
                0: 'apple',
                1: 'banana',
                2: 'cherry',
                length: { toString: () => '3' }
            };
            Array.prototype.find.call(obj, (x: string) => x.startsWith('b'))
            "#
        ),
        JsValue::String(JsString::from("banana"))
    );
}

#[test]
fn test_array_findindex_on_array_like_with_object_length() {
    assert_eq!(
        eval(
            r#"
            const obj: any = {
                0: 10,
                1: 20,
                2: 30,
                length: { valueOf: () => 3 }
            };
            Array.prototype.findIndex.call(obj, (x: number) => x === 20)
            "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_array_indexof_on_array_like_with_object_length() {
    assert_eq!(
        eval(
            r#"
            const obj: any = {
                0: 'x',
                1: 'y',
                2: 'z',
                length: { toString: () => '3' }
            };
            Array.prototype.indexOf.call(obj, 'y')
            "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_array_includes_on_array_like_with_object_length() {
    assert_eq!(
        eval(
            r#"
            const obj: any = {
                0: 100,
                1: 200,
                length: { valueOf: () => 2 }
            };
            Array.prototype.includes.call(obj, 200)
            "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_array_length_coercion_boolean_true() {
    // Boolean true should coerce to 1
    assert_eq!(
        eval(
            r#"
            const obj: any = { 0: 'first', length: true };
            Array.prototype.map.call(obj, (x: string) => x).length
            "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_array_length_coercion_boolean_false() {
    // Boolean false should coerce to 0
    assert_eq!(
        eval(
            r#"
            const obj: any = { 0: 'first', length: false };
            Array.prototype.map.call(obj, (x: string) => x).length
            "#
        ),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_array_length_coercion_null() {
    // null should coerce to 0
    assert_eq!(
        eval(
            r#"
            const obj: any = { 0: 'first', length: null };
            Array.prototype.map.call(obj, (x: string) => x).length
            "#
        ),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_array_length_coercion_undefined() {
    // undefined should coerce to 0 (NaN -> 0)
    assert_eq!(
        eval(
            r#"
            const obj: any = { 0: 'first', length: undefined };
            Array.prototype.map.call(obj, (x: string) => x).length
            "#
        ),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_array_length_coercion_string_number() {
    // String "3" should coerce to 3
    assert_eq!(
        eval(
            r#"
            const obj: any = { 0: 'a', 1: 'b', 2: 'c', length: '3' };
            Array.prototype.map.call(obj, (x: string) => x).length
            "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_array_length_coercion_float_truncated() {
    // Float 2.9 should be truncated to 2
    assert_eq!(
        eval(
            r#"
            const obj: any = { 0: 'a', 1: 'b', 2: 'c', length: 2.9 };
            Array.prototype.map.call(obj, (x: string) => x).length
            "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_array_length_coercion_negative() {
    // Negative numbers should be clamped to 0
    assert_eq!(
        eval(
            r#"
            const obj: any = { 0: 'a', length: -5 };
            Array.prototype.map.call(obj, (x: string) => x).length
            "#
        ),
        JsValue::Number(0.0)
    );
}
