//! Array-related tests

use super::eval;
use typescript_eval::value::JsString;
use typescript_eval::JsValue;

#[test]
fn test_array() {
    assert_eq!(eval("const arr = [1, 2, 3]; arr[1]"), JsValue::Number(2.0));
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
    assert_eq!(eval("const arr = [1, 2]; arr.push(3)"), JsValue::Number(3.0));
}

#[test]
fn test_array_push_multiple() {
    assert_eq!(
        eval("const arr = [1]; arr.push(2, 3, 4); arr.length"),
        JsValue::Number(4.0)
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
    assert_eq!(eval("const arr = [1, 2, 3]; arr.pop()"), JsValue::Number(3.0));
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
    assert_eq!(
        eval("[1, 2, 3, 4].find(x => x > 2)"),
        JsValue::Number(3.0)
    );
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
    assert_eq!(
        eval("[1, 2, 3, 4, 5].slice(1, 4)[0]"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_array_slice_no_args() {
    assert_eq!(eval("[1, 2, 3].slice().length"), JsValue::Number(3.0));
}

#[test]
fn test_array_slice_negative() {
    assert_eq!(eval("[1, 2, 3, 4, 5].slice(-2).length"), JsValue::Number(2.0));
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
    assert_eq!(eval("let a = [1, 2, 3]; a.unshift(0)"), JsValue::Number(4.0));
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
    assert_eq!(
        eval("[1, 2, 3].findLast(x => x > 1)"),
        JsValue::Number(3.0)
    );
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
    // values() returns an array of values
    assert_eq!(
        eval("let arr: string[] = ['a', 'b', 'c']; let vals: string[] = arr.values(); vals[0]"),
        JsValue::String(JsString::from("a"))
    );
    assert_eq!(
        eval("([1, 2, 3] as number[]).values()[1]"),
        JsValue::Number(2.0)
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