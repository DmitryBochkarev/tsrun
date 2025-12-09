//! Map-related tests

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_map_creation() {
    assert_eq!(eval("let m = new Map(); m.size"), JsValue::Number(0.0));
}

#[test]
fn test_map_set_get() {
    assert_eq!(
        eval("let m = new Map(); m.set('a', 1); m.get('a')"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_map_has() {
    assert_eq!(
        eval("let m = new Map(); m.set('a', 1); m.has('a')"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("let m = new Map(); m.has('a')"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_map_size() {
    assert_eq!(
        eval("let m = new Map(); m.set('a', 1); m.size"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_map_delete() {
    // Use bracket notation for 'delete' since it's a reserved word
    assert_eq!(
        eval("let m = new Map(); m.set('a', 1); m['delete']('a'); m.has('a')"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_map_clear() {
    assert_eq!(
        eval("let m = new Map(); m.set('a', 1); m.set('b', 2); m.clear(); m.size"),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_map_object_keys() {
    assert_eq!(
        eval("let m = new Map(); let obj = {}; m.set(obj, 'value'); m.get(obj)"),
        JsValue::from("value")
    );
}

#[test]
fn test_map_init_with_array() {
    assert_eq!(
        eval("let m = new Map([['a', 1], ['b', 2]]); m.get('b')"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_map_foreach() {
    assert_eq!(
        eval("let result = []; let m = new Map([['a', 1], ['b', 2]]); m.forEach((v, k) => result.push(k + ':' + v)); result.join(',')"),
        JsValue::from("a:1,b:2")
    );
}

#[test]
fn test_map_chaining() {
    // Method chaining (set returns Map)
    assert_eq!(
        eval("let m = new Map(); m.set('a', 1).set('b', 2).get('b')"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_map_keys() {
    assert_eq!(
        eval("let m = new Map([['a', 1], ['b', 2]]); Array.from(m.keys()).join(',')"),
        JsValue::from("a,b")
    );
}

#[test]
fn test_map_values() {
    assert_eq!(
        eval("let m = new Map([['a', 1], ['b', 2]]); Array.from(m.values()).join(',')"),
        JsValue::from("1,2")
    );
}

#[test]
fn test_map_entries() {
    assert_eq!(
        eval("let m = new Map([['a', 1], ['b', 2]]); let result = []; for (let e of m.entries()) { result.push(e[0] + ':' + e[1]); } result.join(',')"),
        JsValue::from("a:1,b:2")
    );
}
