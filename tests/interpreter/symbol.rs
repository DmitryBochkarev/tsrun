//! Tests for Symbol primitive

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_symbol_typeof() {
    assert_eq!(eval("typeof Symbol()"), JsValue::from("symbol"));
    assert_eq!(eval("typeof Symbol('foo')"), JsValue::from("symbol"));
}

#[test]
fn test_symbol_description() {
    // Symbol().description should return the description
    assert_eq!(eval("Symbol('foo').description"), JsValue::from("foo"));
    assert_eq!(eval("Symbol().description"), JsValue::Undefined);
}

#[test]
fn test_symbol_uniqueness() {
    // Each Symbol() call should create a unique symbol
    assert_eq!(eval("Symbol() === Symbol()"), JsValue::Boolean(false));
    assert_eq!(
        eval("Symbol('foo') === Symbol('foo')"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_symbol_for() {
    // Symbol.for() should return the same symbol for the same key
    // Note: "for" is a reserved keyword so use bracket notation
    assert_eq!(
        eval("Symbol['for']('foo') === Symbol['for']('foo')"),
        JsValue::Boolean(true)
    );
    // But different from Symbol()
    assert_eq!(
        eval("Symbol['for']('foo') === Symbol('foo')"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_symbol_key_for() {
    // Symbol.keyFor() should return the key for a global symbol
    // Note: "for" is a reserved keyword so use bracket notation
    assert_eq!(
        eval("Symbol.keyFor(Symbol['for']('test'))"),
        JsValue::from("test")
    );
    // And undefined for a non-global symbol
    assert_eq!(eval("Symbol.keyFor(Symbol('test'))"), JsValue::Undefined);
}

#[test]
fn test_symbol_as_property_key() {
    // Symbols can be used as property keys
    assert_eq!(
        eval(
            r#"
            const sym = Symbol('key');
            const obj = { [sym]: 'value' };
            obj[sym]
        "#
        ),
        JsValue::from("value")
    );
}

#[test]
fn test_symbol_to_string() {
    assert_eq!(
        eval("Symbol('foo').toString()"),
        JsValue::from("Symbol(foo)")
    );
    assert_eq!(eval("Symbol().toString()"), JsValue::from("Symbol()"));
}

#[test]
fn test_symbol_iterator() {
    // Symbol.iterator should be a symbol
    assert_eq!(eval("typeof Symbol.iterator"), JsValue::from("symbol"));
}

#[test]
fn test_symbol_to_string_tag() {
    // Symbol.toStringTag should be a symbol
    assert_eq!(eval("typeof Symbol.toStringTag"), JsValue::from("symbol"));
}

#[test]
fn test_symbol_has_instance() {
    // Symbol.hasInstance should be a symbol
    assert_eq!(eval("typeof Symbol.hasInstance"), JsValue::from("symbol"));
}

#[test]
fn test_well_known_symbols_are_unique() {
    // Well-known symbols should be the same across accesses
    assert_eq!(
        eval("Symbol.iterator === Symbol.iterator"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("Symbol.toStringTag === Symbol.toStringTag"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_symbol_not_enumerable_in_for_in() {
    // Symbol properties should not appear in for-in loops
    assert_eq!(
        eval(
            r#"
            const sym = Symbol('hidden');
            const obj = { a: 1, [sym]: 2 };
            let keys: string[] = [];
            for (let k in obj) {
                keys.push(k);
            }
            keys.join(',')
        "#
        ),
        JsValue::from("a")
    );
}

#[test]
fn test_object_get_own_property_symbols() {
    // Object.getOwnPropertySymbols should return symbol keys
    assert_eq!(
        eval(
            r#"
            const sym1 = Symbol('a');
            const sym2 = Symbol('b');
            const obj = { [sym1]: 1, [sym2]: 2, c: 3 };
            Object.getOwnPropertySymbols(obj).length
        "#
        ),
        JsValue::Number(2.0)
    );
}
