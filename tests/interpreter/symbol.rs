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

// ═══════════════════════════════════════════════════════════════════════════════
// Custom Iterator / Symbol.iterator Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_spread_with_custom_iterator() {
    // Test spread operator using Symbol.iterator protocol
    assert_eq!(
        eval(
            r#"
            const iter = {};
            iter[Symbol.iterator] = function() {
                let count = 0;
                return {
                    next: function() {
                        count += 1;
                        return { done: count === 3, value: count };
                    }
                };
            };

            const result = [...iter];
            JSON.stringify(result)
        "#
        ),
        JsValue::from("[1,2]")
    );
}

#[test]
fn test_spread_in_function_call_with_custom_iterator() {
    // Test spread operator in function call using Symbol.iterator
    assert_eq!(
        eval(
            r#"
            const iter = {};
            iter[Symbol.iterator] = function() {
                let count = 0;
                return {
                    next: function() {
                        count += 1;
                        return { done: count === 3, value: count };
                    }
                };
            };

            function sum(a: number, b: number): number {
                return a + b;
            }

            sum(...iter)
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_for_of_with_custom_iterator() {
    // Test for-of loop using Symbol.iterator protocol
    assert_eq!(
        eval(
            r#"
            const iter = {};
            iter[Symbol.iterator] = function() {
                let count = 0;
                return {
                    next: function() {
                        count += 1;
                        return { done: count === 4, value: count * 10 };
                    }
                };
            };

            let sum = 0;
            for (const v of iter) {
                sum += v;
            }
            sum
        "#
        ),
        JsValue::Number(60.0) // 10 + 20 + 30 = 60
    );
}

#[test]
fn test_iterator_close_on_break() {
    // Iterator close protocol: return() should be called on early exit
    assert_eq!(
        eval(
            r#"
            let returnCalled: number = 0;
            const iter = {
                [Symbol.iterator]() {
                    return {
                        next() { return { value: 1, done: false }; },
                        return() {
                            returnCalled = 1;
                            return { done: true };
                        }
                    };
                }
            };
            for (const x of iter) {
                break;
            }
            returnCalled
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_iterator_close_on_return() {
    // Iterator close protocol: return() called when function returns early
    assert_eq!(
        eval(
            r#"
            let returnCalled: number = 0;
            const iter = {
                [Symbol.iterator]() {
                    return {
                        next() { return { value: 1, done: false }; },
                        return() {
                            returnCalled = 1;
                            return { done: true };
                        }
                    };
                }
            };
            function test(): number {
                for (const x of iter) {
                    return 42;
                }
                return 0;
            }
            test();
            returnCalled
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_iterator_close_on_throw() {
    // Iterator close protocol: return() called when exception is thrown
    assert_eq!(
        eval(
            r#"
            let returnCalled: number = 0;
            const iter = {
                [Symbol.iterator]() {
                    return {
                        next() { return { value: 1, done: false }; },
                        return() {
                            returnCalled = 1;
                            return { done: true };
                        }
                    };
                }
            };
            try {
                for (const x of iter) {
                    throw new Error("test");
                }
            } catch (e) {
                // caught
            }
            returnCalled
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_iterator_close_not_called_on_normal_completion() {
    // return() should NOT be called when loop completes normally
    assert_eq!(
        eval(
            r#"
            let returnCalled: number = 0;
            const iter = {
                [Symbol.iterator]() {
                    let count = 0;
                    return {
                        next() {
                            count += 1;
                            return { value: count, done: count > 3 };
                        },
                        return() {
                            returnCalled = 1;
                            return { done: true };
                        }
                    };
                }
            };
            let sum = 0;
            for (const x of iter) {
                sum += x;
            }
            returnCalled  // Should be 0 - return() not called for normal completion
        "#
        ),
        JsValue::Number(0.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Symbol.species Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_array_symbol_species() {
    // Array[Symbol.species] should be Array
    assert_eq!(
        eval("Array[Symbol.species] === Array"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_promise_symbol_species() {
    // Promise[Symbol.species] should be Promise
    assert_eq!(
        eval("Promise[Symbol.species] === Promise"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_map_symbol_species() {
    // Map[Symbol.species] should be Map
    assert_eq!(eval("Map[Symbol.species] === Map"), JsValue::Boolean(true));
}

#[test]
fn test_set_symbol_species() {
    // Set[Symbol.species] should be Set
    assert_eq!(eval("Set[Symbol.species] === Set"), JsValue::Boolean(true));
}

#[test]
fn test_regexp_symbol_species() {
    // RegExp[Symbol.species] should be RegExp
    assert_eq!(
        eval("RegExp[Symbol.species] === RegExp"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_symbol_species_is_symbol() {
    // Symbol.species should be a symbol
    assert_eq!(eval("typeof Symbol.species"), JsValue::from("symbol"));
}
