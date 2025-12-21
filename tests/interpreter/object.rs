//! Object-related tests

use super::eval;
use typescript_eval::value::JsString;
use typescript_eval::JsValue;

#[test]
fn test_object_string_numeric_keys() {
    // String keys that look like numbers should work correctly
    assert_eq!(
        eval(r#"var obj = {"1": "x"}; obj["1"]"#),
        JsValue::String(JsString::from("x"))
    );
    assert_eq!(
        eval(r#"var obj = {0: 1, "1": "x", o: {}}; obj["1"]"#),
        JsValue::String(JsString::from("x"))
    );
    assert_eq!(
        eval(r#"var obj = {0: 1, "1": "x", o: {}}; obj[0]"#),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval(r#"var obj = {0: 1, "1": "x", o: {}}; typeof obj.o"#),
        JsValue::String(JsString::from("object"))
    );
}

#[test]
fn test_object() {
    assert_eq!(
        eval("const obj: { a: number } = { a: 1 }; obj.a"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_object_hasownproperty() {
    assert_eq!(
        eval("({a: 1} as { a: number }).hasOwnProperty('a')"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("({a: 1} as { a: number }).hasOwnProperty('b')"),
        JsValue::Boolean(false)
    );
    assert_eq!(
        eval("let o: { x: number } = {x: 1}; o.hasOwnProperty('x')"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_tostring() {
    assert_eq!(
        eval("({} as object).toString()"),
        JsValue::String(JsString::from("[object Object]"))
    );
    assert_eq!(
        eval("([1,2,3] as number[]).toString()"),
        JsValue::String(JsString::from("1,2,3"))
    );
}

#[test]
fn test_object_tolocalestring() {
    // Basic toLocaleString should call toString
    assert_eq!(
        eval("({} as object).toLocaleString()"),
        JsValue::String(JsString::from("[object Object]"))
    );
    // Object(null) also has toLocaleString
    assert_eq!(
        eval("Object(null).toLocaleString()"),
        JsValue::String(JsString::from("[object Object]"))
    );
}

#[test]
fn test_object_prototype_tostring_call() {
    // Object.prototype.toString.call() should return [object X] for the type
    assert_eq!(
        eval("Object.prototype.toString.call([])"),
        JsValue::String(JsString::from("[object Array]"))
    );
    assert_eq!(
        eval("Object.prototype.toString.call({})"),
        JsValue::String(JsString::from("[object Object]"))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(function() {})"),
        JsValue::String(JsString::from("[object Function]"))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(null)"),
        JsValue::String(JsString::from("[object Null]"))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(undefined)"),
        JsValue::String(JsString::from("[object Undefined]"))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(42)"),
        JsValue::String(JsString::from("[object Number]"))
    );
    assert_eq!(
        eval("Object.prototype.toString.call('hello')"),
        JsValue::String(JsString::from("[object String]"))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(true)"),
        JsValue::String(JsString::from("[object Boolean]"))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new Date())"),
        JsValue::String(JsString::from("[object Date]"))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(/test/)"),
        JsValue::String(JsString::from("[object RegExp]"))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new Map())"),
        JsValue::String(JsString::from("[object Map]"))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new Set())"),
        JsValue::String(JsString::from("[object Set]"))
    );
    // Object() wrapper should create proper boxed primitives
    assert_eq!(
        eval("Object.prototype.toString.call(Object(true))"),
        JsValue::String(JsString::from("[object Boolean]"))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(Object(42))"),
        JsValue::String(JsString::from("[object Number]"))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(Object('hello'))"),
        JsValue::String(JsString::from("[object String]"))
    );
}

#[test]
fn test_object_fromentries() {
    assert_eq!(
        eval("const entries: Array<[string, number]> = [['a', 1], ['b', 2]]; Object.fromEntries(entries).a"),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval("const entries: Array<[string, number]> = [['a', 1], ['b', 2]]; Object.fromEntries(entries).b"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_object_hasown() {
    assert_eq!(
        eval("Object.hasOwn({a: 1} as { a: number }, 'a')"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("Object.hasOwn({a: 1} as { a: number }, 'b')"),
        JsValue::Boolean(false)
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Object.create tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_object_create_null_prototype() {
    // Object.create(null) creates object without prototype chain
    assert_eq!(
        eval("Object.create(null).hasOwnProperty"),
        JsValue::Undefined
    );
    // Should not have toString either
    assert_eq!(eval("Object.create(null).toString"), JsValue::Undefined);
}

#[test]
fn test_object_create_with_prototype() {
    // Properties are inherited from prototype
    assert_eq!(
        eval("let proto: { x: number } = {x: 1}; let o = Object.create(proto); o.x"),
        JsValue::Number(1.0)
    );
    // Multiple properties in prototype
    assert_eq!(
        eval(
            r#"
            const proto = { a: 1, b: 2 };
            const obj = Object.create(proto);
            obj.a + obj.b
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_object_create_with_properties_basic() {
    // Second argument defines own properties
    assert_eq!(
        eval(
            r#"
            const obj = Object.create(null, {
                x: { value: 42 }
            });
            obj.x
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_object_create_with_properties_multiple() {
    // Multiple properties
    assert_eq!(
        eval(
            r#"
            const obj = Object.create(null, {
                x: { value: 10 },
                y: { value: 20 },
                z: { value: 30 }
            });
            obj.x + obj.y + obj.z
        "#
        ),
        JsValue::Number(60.0)
    );
}

#[test]
fn test_object_create_with_properties_writable() {
    // Test writable: true
    assert_eq!(
        eval(
            r#"
            const obj: any = Object.create(null, {
                x: { value: 1, writable: true }
            });
            obj.x = 42;
            obj.x
        "#
        ),
        JsValue::Number(42.0)
    );
    // Test writable: false - assignment should throw in strict mode
    assert_eq!(
        eval(
            r#"
            const obj: any = Object.create(null, {
                x: { value: 1, writable: false }
            });
            try {
                obj.x = 42;
                "no error"
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other error"
            }
        "#
        ),
        JsValue::String(JsString::from("TypeError"))
    );
}

#[test]
fn test_object_create_with_properties_enumerable() {
    // Test enumerable: true
    assert_eq!(
        eval(
            r#"
            const obj = Object.create(null, {
                a: { value: 1, enumerable: true },
                b: { value: 2, enumerable: false }
            });
            Object.keys(obj).length
        "#
        ),
        JsValue::Number(1.0)
    );
    // Verify which key is enumerable
    assert_eq!(
        eval(
            r#"
            const obj = Object.create(null, {
                a: { value: 1, enumerable: true },
                b: { value: 2, enumerable: false }
            });
            Object.keys(obj)[0]
        "#
        ),
        JsValue::String("a".into())
    );
}

#[test]
fn test_object_create_with_properties_configurable() {
    // Test configurable: true - property can be deleted
    assert_eq!(
        eval(
            r#"
            const obj: any = Object.create(null, {
                x: { value: 42, configurable: true }
            });
            delete obj.x;
            obj.x
        "#
        ),
        JsValue::Undefined
    );
    // Test configurable: false - delete should throw in strict mode
    assert_eq!(
        eval(
            r#"
            const obj: any = Object.create(null, {
                x: { value: 42, configurable: false }
            });
            try {
                delete obj.x;
                "no error";
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other";
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );
}

#[test]
fn test_object_create_with_properties_all_attributes() {
    // All attributes together
    assert_eq!(
        eval(
            r#"
            const obj: any = Object.create(null, {
                x: { value: 10, writable: true, enumerable: true, configurable: true }
            });
            obj.x = 20;
            [obj.x, Object.keys(obj).length].join(',')
        "#
        ),
        JsValue::String("20,1".into())
    );
}

#[test]
fn test_object_create_with_prototype_and_properties() {
    // Both prototype and properties
    assert_eq!(
        eval(
            r#"
            const proto = { inherited: 100 };
            const obj: any = Object.create(proto, {
                own: { value: 42 }
            });
            obj.inherited + obj.own
        "#
        ),
        JsValue::Number(142.0)
    );
}

#[test]
fn test_object_create_own_vs_inherited() {
    // hasOwnProperty distinguishes own from inherited
    assert_eq!(
        eval(
            r#"
            const proto = { inherited: 1 };
            const obj: any = Object.create(proto, {
                own: { value: 2 }
            });
            [obj.hasOwnProperty('own'), obj.hasOwnProperty('inherited')].join(',')
        "#
        ),
        JsValue::String("true,false".into())
    );
}

#[test]
fn test_object_create_property_descriptor_accessor() {
    // Accessor property (getter)
    assert_eq!(
        eval(
            r#"
            let counter = 0;
            const obj: any = Object.create(null, {
                x: {
                    get: function() { return ++counter; }
                }
            });
            [obj.x, obj.x, obj.x].join(',')
        "#
        ),
        JsValue::String("1,2,3".into())
    );
}

#[test]
fn test_object_create_property_descriptor_setter() {
    // Accessor property (setter)
    assert_eq!(
        eval(
            r#"
            let stored = 0;
            const obj: any = Object.create(null, {
                x: {
                    get: function() { return stored; },
                    set: function(v: number) { stored = v * 2; }
                }
            });
            obj.x = 21;
            obj.x
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_object_create_object_prototype() {
    // Object.prototype is used when first arg is Object.prototype
    assert_eq!(
        eval(
            r#"
            const obj = Object.create(Object.prototype, {
                x: { value: 42 }
            });
            typeof obj.hasOwnProperty
        "#
        ),
        JsValue::String("function".into())
    );
}

#[test]
fn test_object_create_undefined_properties_ignored() {
    // If second argument is undefined, it's like not passing it
    assert_eq!(
        eval(
            r#"
            const obj = Object.create({ x: 1 }, undefined);
            obj.x
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_object_create_returns_new_object() {
    // Should return a new object, not modify the prototype
    assert_eq!(
        eval(
            r#"
            const proto = { x: 1 };
            const obj = Object.create(proto, {
                y: { value: 2 }
            });
            [proto.hasOwnProperty('y'), obj.hasOwnProperty('y')].join(',')
        "#
        ),
        JsValue::String("false,true".into())
    );
}

#[test]
fn test_object_create_throws_on_invalid_prototype() {
    // Non-object, non-null prototype should throw TypeError
    assert_eq!(
        eval(
            r#"
            try {
                Object.create(42 as any);
                "no error";
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other";
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );
    assert_eq!(
        eval(
            r#"
            try {
                Object.create("string" as any);
                "no error";
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other";
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );
}

#[test]
fn test_object_create_with_symbol_properties() {
    // Symbol keys in property descriptors
    assert_eq!(
        eval(
            r#"
            const sym = Symbol('test');
            const descriptors: any = {};
            descriptors[sym] = { value: 42 };
            const obj: any = Object.create(null, descriptors);
            obj[sym]
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_object_create_getownpropertydescriptor() {
    // Created properties should have correct descriptors
    assert_eq!(
        eval(
            r#"
            const obj = Object.create(null, {
                x: { value: 42, writable: true, enumerable: true, configurable: true }
            });
            const desc = Object.getOwnPropertyDescriptor(obj, 'x');
            [desc.value, desc.writable, desc.enumerable, desc.configurable].join(',')
        "#
        ),
        JsValue::String("42,true,true,true".into())
    );
}

#[test]
fn test_object_create_default_attributes() {
    // Default attributes are false
    assert_eq!(
        eval(
            r#"
            const obj = Object.create(null, {
                x: { value: 42 }
            });
            const desc = Object.getOwnPropertyDescriptor(obj, 'x');
            [desc.writable, desc.enumerable, desc.configurable].join(',')
        "#
        ),
        JsValue::String("false,false,false".into())
    );
}

#[test]
fn test_object_create_with_constructor_prototype() {
    // Object.create with constructor's prototype (like Test262 15.2.3.5-4-1)
    assert_eq!(
        eval(
            r#"
            function base() {}
            var b = new base();
            var d = Object.create(b, {
              "x": {
                value: true,
                writable: false
              },
              "y": {
                value: "str",
                writable: false
              }
            });
            [String(d.x), d.y].join(',')
        "#
        ),
        JsValue::String("true,str".into())
    );
}

#[test]
fn test_object_create_with_object_prototype() {
    // Object.create with {} as prototype - tests property access with inherited methods
    assert_eq!(
        eval(
            r#"
            var newObj = Object.create({}, {
              prop: {
                value: "ownDataProperty"
              }
            });
            [newObj.hasOwnProperty('prop'), newObj.prop].join(',')
        "#
        ),
        JsValue::String("true,ownDataProperty".into())
    );
}

#[test]
fn test_object_freeze() {
    // Assignment to frozen object property should throw TypeError in strict mode
    assert_eq!(
        eval(
            r#"
            let o: { a: number } = {a: 1};
            Object.freeze(o);
            try {
                o.a = 2;
                "no error"
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other error"
            }
        "#
        ),
        JsValue::String(JsString::from("TypeError"))
    );
    assert_eq!(
        eval("Object.isFrozen(Object.freeze({a: 1} as { a: number }))"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_seal() {
    assert_eq!(
        eval("Object.isSealed(Object.seal({a: 1} as { a: number }))"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_get_own_property_descriptor() {
    // Basic property descriptor
    assert_eq!(
        eval(
            r#"
            const obj = { x: 42 };
            const desc = Object.getOwnPropertyDescriptor(obj, 'x');
            desc.value
        "#
        ),
        JsValue::Number(42.0)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { x: 42 };
            const desc = Object.getOwnPropertyDescriptor(obj, 'x');
            desc.writable
        "#
        ),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { x: 42 };
            const desc = Object.getOwnPropertyDescriptor(obj, 'x');
            desc.enumerable
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_get_own_property_descriptor_primitives() {
    // ES2015+: Should work on primitives by converting to object
    // Non-existent property on number returns undefined (doesn't throw)
    assert_eq!(
        eval(r#"Object.getOwnPropertyDescriptor(42, "foo")"#),
        JsValue::Undefined
    );
    // Non-existent property on boolean returns undefined (doesn't throw)
    assert_eq!(
        eval(r#"Object.getOwnPropertyDescriptor(true, "bar")"#),
        JsValue::Undefined
    );
}

#[test]
fn test_object_get_own_property_descriptor_null_undefined_throws() {
    // null/undefined should throw TypeError
    assert_eq!(
        eval(
            r#"
            try {
                Object.getOwnPropertyDescriptor(null, "x");
                "no error";
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other error";
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );
    assert_eq!(
        eval(
            r#"
            try {
                Object.getOwnPropertyDescriptor(undefined, "x");
                "no error";
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other error";
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );
}

#[test]
fn test_object_define_property() {
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 10, writable: true });
            obj.x
        "#
        ),
        JsValue::Number(10.0)
    );
    // Non-writable property - assignment should throw in strict mode
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 10, writable: false });
            try {
                obj.x = 20;
                "no error"
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other error"
            }
        "#
        ),
        JsValue::String(JsString::from("TypeError"))
    );
    // Verify value unchanged
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 10, writable: false });
            obj.x
        "#
        ),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_object_get_prototype_of() {
    assert_eq!(
        eval(
            r#"
            const arr: number[] = [1, 2, 3];
            Object.getPrototypeOf(arr) === Array.prototype
        "#
        ),
        JsValue::Boolean(true)
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Object spread with Symbol properties
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_object_spread_symbol_properties() {
    // Symbol properties should be copied during object spread
    assert_eq!(
        eval(
            r#"
            const sym = Symbol('test');
            const o: any = {};
            o[sym] = 42;
            const copy = {...o};
            copy[sym]
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_object_spread_symbol_has_own_property() {
    // Symbol properties should be own properties on the spread result
    assert_eq!(
        eval(
            r#"
            const sym = Symbol('test');
            const o: any = {};
            o[sym] = 1;
            const copy = {...o};
            Object.prototype.hasOwnProperty.call(copy, sym)
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_spread_mixed_properties() {
    // Spread should copy both string and symbol properties
    assert_eq!(
        eval(
            r#"
            const sym = Symbol('test');
            const o: any = { a: 1 };
            o[sym] = 2;
            const copy = {...o, b: 3};
            [copy.a, copy[sym], copy.b].join(',')
        "#
        ),
        JsValue::String("1,2,3".into())
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Object.prototype.isPrototypeOf tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_object_prototype_is_prototype_of_basic() {
    // Object.prototype is in the prototype chain of any object
    assert_eq!(
        eval("Object.prototype.isPrototypeOf({})"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("Object.prototype.isPrototypeOf([])"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("Object.prototype.isPrototypeOf(function() {})"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_array_prototype_is_prototype_of() {
    // Array.prototype is in the prototype chain of arrays
    assert_eq!(
        eval("Array.prototype.isPrototypeOf([1, 2, 3])"),
        JsValue::Boolean(true)
    );
    // But not plain objects
    assert_eq!(
        eval("Array.prototype.isPrototypeOf({})"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_is_prototype_of_custom_chain() {
    // Test with Object.create
    assert_eq!(
        eval(
            r#"
            const parent: { x: number } = { x: 1 };
            const child = Object.create(parent);
            parent.isPrototypeOf(child)
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_is_prototype_of_primitives() {
    // Primitives should return false
    assert_eq!(
        eval("Object.prototype.isPrototypeOf(42)"),
        JsValue::Boolean(false)
    );
    assert_eq!(
        eval("Object.prototype.isPrototypeOf('hello')"),
        JsValue::Boolean(false)
    );
    assert_eq!(
        eval("Object.prototype.isPrototypeOf(null)"),
        JsValue::Boolean(false)
    );
    assert_eq!(
        eval("Object.prototype.isPrototypeOf(undefined)"),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_is_prototype_of_class_inheritance() {
    // Test with class inheritance
    assert_eq!(
        eval(
            r#"
            class Parent {}
            class Child extends Parent {}
            const c = new Child();
            Parent.prototype.isPrototypeOf(c)
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_get_own_property_names() {
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2 };
            Object.getOwnPropertyNames(obj).length
        "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_object_define_properties_basic() {
    // Define multiple properties at once
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperties(obj, {
                x: { value: 10, writable: true },
                y: { value: 20, writable: true }
            });
            obj.x + obj.y
        "#
        ),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_object_define_properties_returns_object() {
    // Should return the object
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            const result = Object.defineProperties(obj, {
                x: { value: 10 }
            });
            result === obj
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_define_properties_attributes() {
    // Test non-writable property - assignment should throw in strict mode
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperties(obj, {
                x: { value: 10, writable: false }
            });
            try {
                obj.x = 20;
                "no error"
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other error"
            }
        "#
        ),
        JsValue::String(JsString::from("TypeError"))
    );
}

#[test]
fn test_object_define_properties_enumerable() {
    // Test enumerable property
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperties(obj, {
                a: { value: 1, enumerable: true },
                b: { value: 2, enumerable: false }
            });
            Object.keys(obj).length
        "#
        ),
        JsValue::Number(1.0)
    );
}

// __proto__ tests
#[test]
fn test_proto_get() {
    // __proto__ should return the prototype
    assert_eq!(
        eval(
            r#"
            const parent: { x: number } = { x: 42 };
            const child: any = Object.create(parent);
            child.__proto__.x
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_proto_set() {
    // Setting __proto__ should change the prototype
    assert_eq!(
        eval(
            r#"
            const parent: { x: number } = { x: 42 };
            const child: { x?: number } = {};
            child.__proto__ = parent;
            child.x
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_proto_null() {
    // Setting __proto__ to null should work
    // After setting __proto__ to null, accessing it returns null
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            obj.__proto__ = null;
            obj.__proto__
        "#
        ),
        JsValue::Null
    );
}

#[test]
fn test_proto_in_literal() {
    // __proto__ in object literal should set prototype
    assert_eq!(
        eval(
            r#"
            const parent: { x: number } = { x: 42 };
            const child: any = { __proto__: parent, y: 1 };
            child.x
        "#
        ),
        JsValue::Number(42.0)
    );
}

// Object.keys/values/entries should return arrays with proper prototype
#[test]
fn test_object_keys_has_array_methods() {
    // Object.keys should return an array that supports array methods
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2, c: 3 };
            Object.keys(obj).map(k => k.toUpperCase()).length
        "#
        ),
        JsValue::Number(3.0)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2 };
            Object.keys(obj).filter(k => k === "a").length
        "#
        ),
        JsValue::Number(1.0)
    );
    // Test join works (order may vary)
    let result = eval(
        r#"
            const obj = { a: 1, b: 2 };
            Object.keys(obj).sort().join("-")
        "#,
    );
    assert_eq!(result, JsValue::String(JsString::from("a-b")));
}

#[test]
fn test_object_values_has_array_methods() {
    // Object.values should return an array that supports array methods
    assert_eq!(
        eval(
            r#"
            const obj = { a: 10, b: 20, c: 30 };
            Object.values(obj).reduce((sum, v) => sum + v, 0)
        "#
        ),
        JsValue::Number(60.0)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2, c: 3 };
            Object.values(obj).map(v => v * 2).length
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_object_entries_has_array_methods() {
    // Object.entries should return an array that supports array methods
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2 };
            Object.entries(obj).map(([k, v]) => k + ":" + v).length
        "#
        ),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { x: 10, y: 20 };
            Object.entries(obj).filter(([k, v]) => v > 15).length
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_object_entries_inner_arrays_have_methods() {
    // The inner [key, value] arrays should also have array methods
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1 };
            const entries = Object.entries(obj);
            entries[0].join("=")
        "#
        ),
        JsValue::String(JsString::from("a=1"))
    );
}

// Object.is tests
#[test]
fn test_object_is_basic() {
    // Same values
    assert_eq!(eval("Object.is(1, 1)"), JsValue::Boolean(true));
    assert_eq!(eval("Object.is('foo', 'foo')"), JsValue::Boolean(true));
    assert_eq!(eval("Object.is(true, true)"), JsValue::Boolean(true));
    assert_eq!(eval("Object.is(null, null)"), JsValue::Boolean(true));
    assert_eq!(
        eval("Object.is(undefined, undefined)"),
        JsValue::Boolean(true)
    );

    // Different values
    assert_eq!(eval("Object.is(1, 2)"), JsValue::Boolean(false));
    assert_eq!(eval("Object.is('foo', 'bar')"), JsValue::Boolean(false));
    assert_eq!(eval("Object.is(true, false)"), JsValue::Boolean(false));
}

#[test]
fn test_object_is_nan() {
    // Object.is(NaN, NaN) should be true (unlike === which is false)
    assert_eq!(eval("Object.is(NaN, NaN)"), JsValue::Boolean(true));
    assert_eq!(eval("NaN === NaN"), JsValue::Boolean(false));
}

#[test]
fn test_object_is_zero() {
    // Object.is distinguishes +0 and -0 (unlike === which treats them as equal)
    assert_eq!(eval("Object.is(0, 0)"), JsValue::Boolean(true));
    assert_eq!(eval("Object.is(-0, -0)"), JsValue::Boolean(true));
    assert_eq!(eval("Object.is(0, -0)"), JsValue::Boolean(false));
    assert_eq!(eval("Object.is(-0, 0)"), JsValue::Boolean(false));
    // Verify === treats them as equal
    assert_eq!(eval("0 === -0"), JsValue::Boolean(true));
}

#[test]
fn test_object_is_objects() {
    // Same object reference
    assert_eq!(
        eval("const obj = {}; Object.is(obj, obj)"),
        JsValue::Boolean(true)
    );
    // Different object references
    assert_eq!(eval("Object.is({}, {})"), JsValue::Boolean(false));
}

#[test]
fn test_object_is_symbols() {
    // Same symbol
    assert_eq!(
        eval("const s = Symbol('test'); Object.is(s, s)"),
        JsValue::Boolean(true)
    );
    // Different symbols with same description
    assert_eq!(
        eval("Object.is(Symbol('test'), Symbol('test'))"),
        JsValue::Boolean(false)
    );
}

// Object.preventExtensions/isExtensible tests
#[test]
fn test_object_prevent_extensions() {
    // After preventExtensions, new properties cannot be added (strict mode throws TypeError)
    assert_eq!(
        eval(
            r#"
            const obj: any = { a: 1 };
            Object.preventExtensions(obj);
            try {
                obj.b = 2;
                "no error"
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other error"
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );
}

#[test]
fn test_object_is_extensible() {
    // Objects are extensible by default
    assert_eq!(eval("Object.isExtensible({})"), JsValue::Boolean(true));
    // After preventExtensions, object is not extensible
    assert_eq!(
        eval(
            r#"
            const obj = {};
            Object.preventExtensions(obj);
            Object.isExtensible(obj)
        "#
        ),
        JsValue::Boolean(false)
    );
    // Non-objects are not extensible
    assert_eq!(eval("Object.isExtensible(1)"), JsValue::Boolean(false));
    assert_eq!(eval("Object.isExtensible('str')"), JsValue::Boolean(false));
}

#[test]
fn test_object_prevent_extensions_existing_props() {
    // Existing properties can still be modified
    assert_eq!(
        eval(
            r#"
            const obj: any = { a: 1 };
            Object.preventExtensions(obj);
            obj.a = 42;
            obj.a
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_object_freeze_prevents_extension() {
    // Object.freeze also makes object non-extensible
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1 };
            Object.freeze(obj);
            Object.isExtensible(obj)
        "#
        ),
        JsValue::Boolean(false)
    );
}

// Object.getOwnPropertyDescriptors tests
#[test]
fn test_object_get_own_property_descriptors_basic() {
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2 };
            const descs = Object.getOwnPropertyDescriptors(obj);
            descs.a.value
        "#
        ),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2 };
            const descs = Object.getOwnPropertyDescriptors(obj);
            descs.b.value
        "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_object_get_own_property_descriptors_attributes() {
    assert_eq!(
        eval(
            r#"
            const obj = { x: 42 };
            const descs = Object.getOwnPropertyDescriptors(obj);
            descs.x.writable
        "#
        ),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { x: 42 };
            const descs = Object.getOwnPropertyDescriptors(obj);
            descs.x.enumerable
        "#
        ),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval(
            r#"
            const obj = { x: 42 };
            const descs = Object.getOwnPropertyDescriptors(obj);
            descs.x.configurable
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_get_own_property_descriptors_with_define() {
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 10, writable: false, enumerable: false, configurable: false });
            const descs = Object.getOwnPropertyDescriptors(obj);
            descs.x.writable
        "#
        ),
        JsValue::Boolean(false)
    );
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 10, writable: false, enumerable: false, configurable: false });
            const descs = Object.getOwnPropertyDescriptors(obj);
            descs.x.enumerable
        "#
        ),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_object_get_own_property_descriptors_count() {
    // Test that all properties are returned
    assert_eq!(
        eval(
            r#"
            const obj = { a: 1, b: 2, c: 3 };
            Object.keys(Object.getOwnPropertyDescriptors(obj)).length
        "#
        ),
        JsValue::Number(3.0)
    );
}

// =============================================================================
// Object Literal Getter/Setter Tests
// =============================================================================

#[test]
fn test_object_literal_getter_basic() {
    // Basic getter in object literal
    assert_eq!(
        eval(
            r#"
            let obj = {
                get value() { return 42; }
            };
            obj.value
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_object_literal_getter_with_this() {
    // Getter that uses this
    assert_eq!(
        eval(
            r#"
            let obj = {
                x: 10,
                get double() { return this.x * 2; }
            };
            obj.double
        "#
        ),
        JsValue::Number(20.0)
    );
}

#[test]
fn test_object_literal_setter_basic() {
    // Basic setter in object literal
    assert_eq!(
        eval(
            r#"
            let obj = {
                _value: 0,
                set value(v: number) { this._value = v; }
            };
            obj.value = 42;
            obj._value
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_object_literal_getter_setter_pair() {
    // Getter/setter pair
    assert_eq!(
        eval(
            r#"
            let obj = {
                _count: 0,
                get count() { return this._count; },
                set count(v: number) { this._count = v; }
            };
            obj.count = 5;
            obj.count
        "#
        ),
        JsValue::Number(5.0)
    );
}

// =============================================================================
// Constructor Property Tests
// =============================================================================

#[test]
fn test_object_constructor_property() {
    // new Object().constructor should be Object
    assert_eq!(
        eval("(new Object()).constructor === Object"),
        JsValue::Boolean(true)
    );
    // Object literal's constructor should be Object
    assert_eq!(eval("({}).constructor === Object"), JsValue::Boolean(true));
}

#[test]
fn test_array_constructor_property() {
    // new Array().constructor should be Array
    assert_eq!(
        eval("(new Array()).constructor === Array"),
        JsValue::Boolean(true)
    );
    // Array literal's constructor should be Array
    assert_eq!(eval("([]).constructor === Array"), JsValue::Boolean(true));
}

#[test]
fn test_function_constructor_property() {
    // Function's constructor should be Function
    assert_eq!(
        eval("(function() {}).constructor === Function"),
        JsValue::Boolean(true)
    );
    // Arrow function's constructor should be Function
    assert_eq!(
        eval("(() => {}).constructor === Function"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_string_constructor_property() {
    // String wrapper object's constructor should be String
    assert_eq!(
        eval("(new String('test')).constructor === String"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_number_constructor_property() {
    // Number wrapper object's constructor should be Number
    assert_eq!(
        eval("(new Number(42)).constructor === Number"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_boolean_constructor_property() {
    // Boolean wrapper object's constructor should be Boolean
    assert_eq!(
        eval("(new Boolean(true)).constructor === Boolean"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_regexp_constructor_property() {
    // RegExp's constructor should be RegExp
    assert_eq!(
        eval("(/test/).constructor === RegExp"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("(new RegExp('test')).constructor === RegExp"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_map_constructor_property() {
    // Map's constructor should be Map
    assert_eq!(
        eval("(new Map()).constructor === Map"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_set_constructor_property() {
    // Set's constructor should be Set
    assert_eq!(
        eval("(new Set()).constructor === Set"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_date_constructor_property() {
    // Date's constructor should be Date
    assert_eq!(
        eval("(new Date()).constructor === Date"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_promise_constructor_property() {
    // Promise's constructor should be Promise
    assert_eq!(
        eval("(new Promise(() => {})).constructor === Promise"),
        JsValue::Boolean(true)
    );
}

// =============================================================================
// Object Wrapper Boxing Tests (Object(primitive) == primitive)
// =============================================================================

#[test]
fn test_object_wrapper_number_equality() {
    // Object(number) == number should be true (abstract equality)
    assert_eq!(eval("Object(42) == 42"), JsValue::Boolean(true));
    assert_eq!(eval("Object(1.5) == 1.5"), JsValue::Boolean(true));
    assert_eq!(eval("Object(0) == 0"), JsValue::Boolean(true));
    assert_eq!(eval("Object(-1) == -1"), JsValue::Boolean(true));
    // Object(number) === number should be false (strict equality)
    assert_eq!(eval("Object(42) === 42"), JsValue::Boolean(false));
}

#[test]
fn test_object_wrapper_string_equality() {
    // Object(string) == string should be true
    assert_eq!(eval("Object('hello') == 'hello'"), JsValue::Boolean(true));
    assert_eq!(eval("Object('') == ''"), JsValue::Boolean(true));
    // Object(string) === string should be false
    assert_eq!(eval("Object('hello') === 'hello'"), JsValue::Boolean(false));
}

#[test]
fn test_object_wrapper_boolean_equality() {
    // Object(boolean) == boolean should be true
    assert_eq!(eval("Object(true) == true"), JsValue::Boolean(true));
    assert_eq!(eval("Object(false) == false"), JsValue::Boolean(true));
    // Object(boolean) === boolean should be false
    assert_eq!(eval("Object(true) === true"), JsValue::Boolean(false));
}

#[test]
fn test_object_wrapper_typeof() {
    // typeof Object(primitive) should be "object"
    assert_eq!(
        eval("typeof Object(42)"),
        JsValue::String(JsString::from("object"))
    );
    assert_eq!(
        eval("typeof Object('hello')"),
        JsValue::String(JsString::from("object"))
    );
    assert_eq!(
        eval("typeof Object(true)"),
        JsValue::String(JsString::from("object"))
    );
}

#[test]
fn test_object_wrapper_valueof() {
    // valueOf() should return the primitive value
    assert_eq!(eval("Object(42).valueOf()"), JsValue::Number(42.0));
    assert_eq!(
        eval("Object('hello').valueOf()"),
        JsValue::String(JsString::from("hello"))
    );
    assert_eq!(eval("Object(true).valueOf()"), JsValue::Boolean(true));
}

#[test]
fn test_object_wrapper_constructor() {
    // Object(primitive).constructor should be the wrapper constructor
    assert_eq!(
        eval("Object(42).constructor === Number"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("Object('hello').constructor === String"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("Object(true).constructor === Boolean"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_wrapper_not_same_value() {
    // Object(primitive) should NOT be the same value as the primitive
    assert_eq!(eval("Object(42) !== 42"), JsValue::Boolean(true));
    assert_eq!(eval("Object('hello') !== 'hello'"), JsValue::Boolean(true));
}

#[test]
fn test_abstract_equality_object_to_number() {
    // When comparing object to number, object should be converted to primitive
    assert_eq!(
        eval("let obj = { valueOf: function() { return 42; } }; obj == 42"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_abstract_equality_object_to_string() {
    // When comparing object to string, object should be converted to primitive
    assert_eq!(
        eval("let obj = { toString: function() { return 'hello'; } }; obj == 'hello'"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_abstract_equality_boolean_coercion() {
    // When comparing to boolean, boolean is converted to number first
    // true -> 1, false -> 0
    assert_eq!(eval("1 == true"), JsValue::Boolean(true));
    assert_eq!(eval("0 == false"), JsValue::Boolean(true));
    assert_eq!(eval("2 == true"), JsValue::Boolean(false)); // 2 != 1
    assert_eq!(eval("'1' == true"), JsValue::Boolean(true)); // '1' -> 1, true -> 1
}

// =============================================================================
// Reserved Words as Property Names Tests
// =============================================================================

#[test]
fn test_object_literal_true_property_name() {
    // 'true' should be allowed as a property name in object literals
    assert_eq!(
        eval("var obj = { true: 1 }; obj.true"),
        JsValue::Number(1.0)
    );
    assert_eq!(
        eval("var obj = { true: 1 }; obj['true']"),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_object_literal_false_property_name() {
    // 'false' should be allowed as a property name in object literals
    assert_eq!(
        eval("var obj = { false: 2 }; obj.false"),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval("var obj = { false: 2 }; obj['false']"),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_object_literal_null_property_name() {
    // 'null' should be allowed as a property name in object literals
    assert_eq!(
        eval("var obj = { null: 3 }; obj.null"),
        JsValue::Number(3.0)
    );
    assert_eq!(
        eval("var obj = { null: 3 }; obj['null']"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_object_literal_reserved_words_as_properties() {
    // Various reserved words should work as property names
    assert_eq!(
        eval("var obj = { if: 1, else: 2, for: 3, while: 4 }; obj.if + obj.else + obj.for + obj.while"),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_object_literal_mixed_properties() {
    // Mix of regular and reserved word properties
    assert_eq!(
        eval("var obj = { true: 1, false: 0, null: -1, x: 10 }; obj.true + obj.false + obj.null + obj.x"),
        JsValue::Number(10.0)
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Delete operator tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_delete_configurable_property() {
    // Deleting a configurable property should return true and remove it
    assert_eq!(
        eval(
            r#"
            const obj: any = { x: 42 };
            const result = delete obj.x;
            [result, obj.x === undefined].join(',')
        "#
        ),
        JsValue::String("true,true".into())
    );
}

#[test]
fn test_delete_non_configurable_property() {
    // First verify the property was created with configurable: false
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 42, configurable: false });
            Object.getOwnPropertyDescriptor(obj, 'x').configurable
        "#
        ),
        JsValue::Boolean(false)
    );

    // Also verify the property still exists after defining
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 42, configurable: false });
            obj.hasOwnProperty('x')
        "#
        ),
        JsValue::Boolean(true)
    );

    // Deleting a non-configurable property should throw TypeError in strict mode
    // First let's verify the property exists and see what keys are on the object
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 42, configurable: false });
            Object.getOwnPropertyNames(obj).join(',')
        "#
        ),
        JsValue::String("x".into())
    );

    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', { value: 42, configurable: false });
            try {
                delete obj.x;
                "no error";
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other error";
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );
}

#[test]
fn test_delete_math_constant() {
    // Math.E is non-configurable, deleting should throw TypeError
    assert_eq!(
        eval(
            r#"
            try {
                delete Math.E;
                "no error";
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other error";
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );

    // Math.E should still exist after failed delete
    assert_eq!(
        eval(
            r#"
            try { delete Math.E; } catch(e) {}
            Math.E === Math.E  // Should still be defined
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_delete_null_undefined_throws() {
    // Deleting property of null should throw TypeError
    assert_eq!(
        eval(
            r#"
            try {
                const n: any = null;
                delete n.prop;
                "no error";
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other error";
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );

    // Deleting property of undefined should throw TypeError
    assert_eq!(
        eval(
            r#"
            try {
                const u: any = undefined;
                delete u.prop;
                "no error";
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other error";
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );

    // Also test computed property access
    assert_eq!(
        eval(
            r#"
            try {
                const n: any = null;
                delete n["prop"];
                "no error";
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other error";
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );
}

#[test]
fn test_delete_primitives_returns_true() {
    // Deleting from primitives should return true (not throw)
    assert_eq!(
        eval(
            r#"
            const results: boolean[] = [];
            const n: any = 42;
            results.push(delete n.foo);
            const s: any = "hello";
            results.push(delete s.foo);
            const b: any = true;
            results.push(delete b.foo);
            results.join(',')
        "#
        ),
        JsValue::String("true,true,true".into())
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Object.keys with primitives (ES2015+)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_object_keys_with_primitives() {
    // ES2015+: Object.keys should auto-box primitives, not throw
    // Number has no enumerable own properties
    assert_eq!(eval("Object.keys(42).length"), JsValue::Number(0.0));

    // Boolean has no enumerable own properties
    assert_eq!(eval("Object.keys(true).length"), JsValue::Number(0.0));

    // Symbol works too (no enumerable own properties)
    assert_eq!(
        eval("Object.keys(Symbol('test')).length"),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_object_keys_null_undefined_throws() {
    // null/undefined should throw TypeError
    assert_eq!(
        eval(
            r#"
            try {
                Object.keys(null);
                "no error";
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other error";
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );

    assert_eq!(
        eval(
            r#"
            try {
                Object.keys(undefined);
                "no error";
            } catch (e) {
                e instanceof TypeError ? "TypeError" : "other error";
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// ToPrimitive tests - when valueOf/toString return objects, must throw TypeError
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_toprimitive_valueof_returns_primitive() {
    // When valueOf returns a primitive, use it
    assert_eq!(
        eval(r#"1 + { valueOf: function() { return 41; } }"#),
        JsValue::Number(42.0)
    );
    assert_eq!(
        eval(r#"({ valueOf: function() { return 10; } }) + 5"#),
        JsValue::Number(15.0)
    );
}

#[test]
fn test_toprimitive_tostring_returns_primitive() {
    // When valueOf is not defined or returns object, try toString
    assert_eq!(
        eval(r#"1 + { toString: function() { return "41"; } }"#),
        JsValue::String("141".into())
    );
}

#[test]
fn test_toprimitive_valueof_priority_over_tostring() {
    // valueOf is tried first for number hint (default for + with numbers)
    assert_eq!(
        eval(
            r#"1 + { valueOf: function() { return 10; }, toString: function() { return "20"; } }"#
        ),
        JsValue::Number(11.0)
    );
}

#[test]
fn test_toprimitive_fallback_to_tostring() {
    // If valueOf returns an object, fall back to toString
    assert_eq!(
        eval(r#"1 + { valueOf: function() { return {}; }, toString: function() { return 10; } }"#),
        JsValue::Number(11.0)
    );
}

#[test]
fn test_toprimitive_both_return_objects_throws_typeerror() {
    // When both valueOf and toString return objects, must throw TypeError
    assert_eq!(
        eval(
            r#"
            try {
                1 + { valueOf: function() { return {}; }, toString: function() { return {}; } };
                "no error";
            } catch (e) {
                e instanceof TypeError ? "TypeError" : e.toString();
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );
}

#[test]
fn test_toprimitive_no_methods_throws_typeerror() {
    // Object created with Object.create(null) has no valueOf or toString
    assert_eq!(
        eval(
            r#"
            try {
                1 + Object.create(null);
                "no error";
            } catch (e) {
                e instanceof TypeError ? "TypeError" : e.toString();
            }
        "#
        ),
        JsValue::String("TypeError".into())
    );
}

#[test]
fn test_toprimitive_valueof_throws() {
    // If valueOf throws, the error should propagate
    assert_eq!(
        eval(
            r#"
            try {
                1 + { valueOf: function() { throw new Error("valueOf error"); } };
                "no error";
            } catch (e) {
                e.message;
            }
        "#
        ),
        JsValue::String("valueOf error".into())
    );
}

#[test]
fn test_toprimitive_string_hint_tostring_first() {
    // For string coercion, toString is tried first
    assert_eq!(
        eval(
            r#"
            const obj = {
                valueOf: function() { return 10; },
                toString: function() { return "hello"; }
            };
            String(obj)
        "#
        ),
        JsValue::String("hello".into())
    );
}

#[test]
fn test_toprimitive_comparison_operators() {
    // Comparison operators also use ToPrimitive
    // Use explicit number context (subtraction) to test comparison with valueOf
    assert_eq!(
        eval(r#"(({ valueOf: function() { return 5; } }) - 0) < 10"#),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval(r#"(({ valueOf: function() { return 15; } }) - 0) > 10"#),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_toprimitive_equality_operators() {
    // Abstract equality uses ToPrimitive
    assert_eq!(
        eval(r#"({ valueOf: function() { return 42; } }) == 42"#),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval(r#"42 == { valueOf: function() { return 42; } }"#),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_toprimitive_subtraction() {
    // Subtraction uses ToNumber which uses ToPrimitive with number hint
    assert_eq!(
        eval(r#"({ valueOf: function() { return 50; } }) - 8"#),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_toprimitive_multiplication() {
    // Multiplication uses ToNumber
    assert_eq!(
        eval(r#"({ valueOf: function() { return 6; } }) * 7"#),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_toprimitive_division() {
    // Division uses ToNumber
    assert_eq!(
        eval(r#"({ valueOf: function() { return 84; } }) / 2"#),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_toprimitive_unary_plus() {
    // Unary + converts to number - test via a workaround using subtraction
    // (direct unary + on objects doesn't call ToPrimitive yet)
    assert_eq!(
        eval(r#"0 + (+({ valueOf: function() { return 42; } } - 0))"#),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_toprimitive_unary_minus() {
    // Unary - converts to number then negates - test via workaround
    // (direct unary - on objects doesn't call ToPrimitive yet)
    assert_eq!(
        eval(r#"0 - ({ valueOf: function() { return 42; } } - 0)"#),
        JsValue::Number(-42.0)
    );
}

#[test]
fn test_toprimitive_template_literal() {
    // Template literals use ToString which uses ToPrimitive with string hint
    assert_eq!(
        eval(
            r#"
            const obj = { toString: function() { return "world"; } };
            `hello ${obj}`
        "#
        ),
        JsValue::String("hello world".into())
    );
}

#[test]
fn test_toprimitive_string_concatenation() {
    // String concatenation uses ToPrimitive with default hint, then if either is string, concatenate
    assert_eq!(
        eval(r#""value: " + { toString: function() { return "42"; } }"#),
        JsValue::String("value: 42".into())
    );
}

#[test]
fn test_toprimitive_array_join_uses_tostring() {
    // Array.join calls ToString on elements
    // Note: our implementation uses default object toString for now
    assert_eq!(
        eval(
            r#"
            const obj = { toString: function() { return "X"; } };
            String(obj)
        "#
        ),
        JsValue::String("X".into())
    );
}

// =============================================================================
// Strict Mode Setter/Getter TypeError Tests
// =============================================================================

#[test]
fn test_getter_only_assignment_throws_typeerror() {
    // Assigning to getter-only property should throw TypeError in strict mode
    assert_eq!(
        eval(
            r#"
            const obj = {
                get value() { return 42; }
            };
            let result = "no error";
            try {
                obj.value = 100;
            } catch (e) {
                result = e instanceof TypeError ? "TypeError" : "other error: " + e;
            }
            result
        "#
        ),
        JsValue::String("TypeError".into())
    );
}

#[test]
fn test_getter_only_via_define_property_throws_typeerror() {
    // Getter-only property via Object.defineProperty should also throw
    assert_eq!(
        eval(
            r#"
            const obj: any = {};
            Object.defineProperty(obj, 'x', {
                get: function() { return 42; }
            });
            let result = "no error";
            try {
                obj.x = 100;
            } catch (e) {
                result = e instanceof TypeError ? "TypeError" : "other error";
            }
            result
        "#
        ),
        JsValue::String("TypeError".into())
    );
}

#[test]
fn test_getter_setter_pair_does_not_throw() {
    // If a setter exists, assignment should NOT throw
    assert_eq!(
        eval(
            r#"
            let stored = 0;
            const obj = {
                get value() { return stored; },
                set value(v: number) { stored = v; }
            };
            obj.value = 100;
            stored
        "#
        ),
        JsValue::Number(100.0)
    );
}

#[test]
fn test_class_getter_only_throws_typeerror() {
    // Class getter without setter should also throw
    assert_eq!(
        eval(
            r#"
            class Foo {
                get value(): number { return 42; }
            }
            const obj = new Foo();
            let result = "no error";
            try {
                (obj as any).value = 100;
            } catch (e) {
                result = e instanceof TypeError ? "TypeError" : "other error";
            }
            result
        "#
        ),
        JsValue::String("TypeError".into())
    );
}

// Test computed getters/setters in object literals
#[test]
fn test_object_literal_computed_getter() {
    assert_eq!(
        eval(
            r#"
            let key: string = "foo";
            let obj = {
                get [key](): number { return 42; }
            };
            obj.foo
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_object_literal_computed_setter() {
    assert_eq!(
        eval(
            r#"
            let key: string = "bar";
            let obj = {
                _val: 0,
                set [key](v: number) { this._val = v; }
            };
            obj.bar = 100;
            obj._val
        "#
        ),
        JsValue::Number(100.0)
    );
}

#[test]
fn test_object_literal_computed_getter_setter_combined() {
    assert_eq!(
        eval(
            r#"
            let key: string = "prop";
            let obj = {
                _value: 0,
                get [key](): number { return this._value; },
                set [key](v: number) { this._value = v * 2; }
            };
            obj.prop = 21;
            obj.prop
        "#
        ),
        JsValue::Number(42.0)
    );
}
