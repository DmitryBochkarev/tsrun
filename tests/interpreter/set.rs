//! Set-related tests

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_set_basic() {
    // Basic Set creation and operations
    assert_eq!(eval("let s = new Set(); s.size"), JsValue::Number(0.0));
    assert_eq!(
        eval("let s = new Set(); s.add(1); s.has(1)"),
        JsValue::Boolean(true)
    );
    assert_eq!(eval("let s = new Set(); s.has(1)"), JsValue::Boolean(false));
    assert_eq!(
        eval("let s = new Set(); s.add(1); s.size"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_set_uniqueness() {
    // Uniqueness - adding same value twice doesn't increase size
    assert_eq!(
        eval("let s = new Set(); s.add(1); s.add(1); s.size"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_set_delete() {
    // Delete (use bracket notation for 'delete' since it's a reserved word)
    assert_eq!(
        eval("let s = new Set(); s.add(1); s['delete'](1); s.has(1)"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_set_clear() {
    assert_eq!(
        eval("let s = new Set(); s.add(1); s.add(2); s.clear(); s.size"),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_set_object_values() {
    // Object values
    assert_eq!(
        eval("let s = new Set(); let obj = {}; s.add(obj); s.has(obj)"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_set_initialize_with_array() {
    // Initialize with array
    assert_eq!(
        eval("let s = new Set([1, 2, 3]); s.size"),
        JsValue::Number(3.0)
    );
    // Duplicates removed
    assert_eq!(
        eval("let s = new Set([1, 2, 2, 3]); s.size"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_set_foreach() {
    // forEach
    assert_eq!(
        eval("let result = []; let s = new Set([1, 2, 3]); s.forEach(v => result.push(v)); result.join(',')"),
        JsValue::from("1,2,3")
    );
}

#[test]
fn test_set_method_chaining() {
    // Method chaining (add returns Set)
    assert_eq!(
        eval("let s = new Set(); s.add(1).add(2).has(2)"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_set_keys() {
    // For Set, keys() returns values (same as values())
    assert_eq!(
        eval("let s = new Set([1, 2, 3]); Array.from(s.keys()).join(',')"),
        JsValue::from("1,2,3")
    );
}

#[test]
fn test_set_values() {
    assert_eq!(
        eval("let s = new Set(['a', 'b', 'c']); Array.from(s.values()).join(',')"),
        JsValue::from("a,b,c")
    );
}

#[test]
fn test_set_entries() {
    // For Set, entries() returns [value, value] pairs
    assert_eq!(
        eval("let s = new Set([1, 2]); let result = []; for (let e of s.entries()) { result.push(e[0] + ':' + e[1]); } result.join(',')"),
        JsValue::from("1:1,2:2")
    );
}
