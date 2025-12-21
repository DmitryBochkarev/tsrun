//! Class feature tests: private fields, methods, etc.
//!
//! NOTE: Class declarations are currently not fully implemented in the interpreter.
//! The execute_class_declaration function is a stub. These tests document the
//! desired behavior for when classes are fully implemented.

use super::eval;
use typescript_eval::JsValue;

// Basic class test to verify class functionality
#[test]
fn test_class_basic() {
    assert_eq!(
        eval(
            r#"
            class Counter {
                count: number = 0;
                increment(): void {
                    this.count = this.count + 1;
                }
                getCount(): number {
                    return this.count;
                }
            }
            const c: Counter = new Counter();
            c.increment();
            c.getCount()
        "#
        ),
        JsValue::Number(1.0)
    );
}

// Test static methods
#[test]
fn test_class_static_method() {
    assert_eq!(
        eval(
            r#"
            class MathHelper {
                static double(x: number): number {
                    return x * 2;
                }
            }
            MathHelper.double(5)
        "#
        ),
        JsValue::Number(10.0)
    );
}

// Test static fields
#[test]
fn test_class_static_field() {
    assert_eq!(
        eval(
            r#"
            class Config {
                static version: string = "1.0";
            }
            Config.version
        "#
        ),
        JsValue::from("1.0")
    );
}

// Test class inheritance
#[test]
fn test_class_extends() {
    assert_eq!(
        eval(
            r#"
            class Animal {
                name: string = "unknown";
                speak(): string {
                    return "...";
                }
            }
            class Dog extends Animal {
                speak(): string {
                    return "woof";
                }
            }
            const d: Dog = new Dog();
            d.speak()
        "#
        ),
        JsValue::from("woof")
    );
}

// Test super() call in constructor
#[test]
fn test_class_super_call() {
    assert_eq!(
        eval(
            r#"
            class Animal {
                name: string;
                constructor(name: string) {
                    this.name = name;
                }
            }
            class Dog extends Animal {
                breed: string;
                constructor(name: string, breed: string) {
                    super(name);
                    this.breed = breed;
                }
            }
            const d: Dog = new Dog("Rex", "German Shepherd");
            d.name + " is a " + d.breed
        "#
        ),
        JsValue::from("Rex is a German Shepherd")
    );
}

// Test super.method() call
#[test]
fn test_class_super_method() {
    assert_eq!(
        eval(
            r#"
            class Animal {
                speak(): string {
                    return "generic sound";
                }
            }
            class Dog extends Animal {
                speak(): string {
                    return super.speak() + " and woof";
                }
            }
            const d: Dog = new Dog();
            d.speak()
        "#
        ),
        JsValue::from("generic sound and woof")
    );
}

// Class expression tests
#[test]
fn test_class_expression() {
    assert_eq!(
        eval(
            r#"
            const Foo = class {
                value: number = 10;
                getValue(): number {
                    return this.value;
                }
            };
            const f = new Foo();
            f.getValue()
        "#
        ),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_class_expression_named() {
    assert_eq!(
        eval(
            r#"
            const MyClass = class Named {
                name: string = "test";
                getName(): string {
                    return this.name;
                }
            };
            const m = new MyClass();
            m.getName()
        "#
        ),
        JsValue::from("test")
    );
}

// Private field tests
#[test]
fn test_private_field_basic() {
    assert_eq!(
        eval(
            r#"
            class Counter {
                #count: number = 0;
                increment(): void {
                    this.#count = this.#count + 1;
                }
                getCount(): number {
                    return this.#count;
                }
            }
            const c: Counter = new Counter();
            c.increment();
            c.increment();
            c.getCount()
        "#
        ),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_private_field_initial_value() {
    assert_eq!(
        eval(
            r#"
            class Box {
                #value: number = 42;
                getValue(): number {
                    return this.#value;
                }
            }
            const b: Box = new Box();
            b.getValue()
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_private_method() {
    assert_eq!(
        eval(
            r#"
            class Calculator {
                #double(n: number): number {
                    return n * 2;
                }
                compute(n: number): number {
                    return this.#double(n);
                }
            }
            const calc: Calculator = new Calculator();
            calc.compute(5)
        "#
        ),
        JsValue::Number(10.0)
    );
}

// Test getter/setter in classes
#[test]
fn test_class_getter_setter() {
    assert_eq!(
        eval(
            r#"
            class Circle {
                #radius: number = 1;

                get radius(): number {
                    return this.#radius;
                }

                set radius(value: number) {
                    this.#radius = value;
                }

                get diameter(): number {
                    return this.#radius * 2;
                }
            }
            const c = new Circle();
            c.radius = 5;
            c.diameter
        "#
        ),
        JsValue::Number(10.0)
    );
}

// Test getter-only property
#[test]
fn test_class_getter_only() {
    assert_eq!(
        eval(
            r#"
            class Box {
                #value: number = 42;

                get value(): number {
                    return this.#value;
                }
            }
            const b = new Box();
            b.value
        "#
        ),
        JsValue::Number(42.0)
    );
}

// Test accessing getter via prototype (test262 style)
#[test]
fn test_class_getter_via_prototype() {
    // This is how test262 tests class getters - accessing via prototype
    assert_eq!(
        eval(
            r#"
            var C = class {
                get foo() { return 'get string'; }
            };
            C.prototype['foo']
        "#
        ),
        JsValue::String("get string".into())
    );
}

// Test accessing getter via prototype with numeric key
#[test]
fn test_class_getter_numeric_key_via_prototype() {
    // Binary literal 0b10 = 2
    assert_eq!(
        eval(
            r#"
            var C = class {
                get 2() { return 'get string'; }
            };
            C.prototype['2']
        "#
        ),
        JsValue::String("get string".into())
    );
}

// Test getter with leading decimal numeric key (.1 = 0.1)
#[test]
fn test_class_getter_leading_decimal_key() {
    // .1 should be parsed as 0.1 and stored with key "0.1"
    assert_eq!(
        eval(
            r#"
            var C = class {
                get .1() { return 'get string'; }
            };
            C.prototype['0.1']
        "#
        ),
        JsValue::String("get string".into())
    );
}

// Test getter with non-canonical numeric key (0.0000001 = 1e-7)
#[test]
fn test_class_getter_non_canonical_key() {
    assert_eq!(
        eval(
            r#"
            var C = class {
                get 0.0000001() { return 'get string'; }
            };
            C.prototype['1e-7']
        "#
        ),
        JsValue::String("get string".into())
    );
}

// Test getter with computed key
#[test]
fn test_class_getter_computed_key() {
    assert_eq!(
        eval(
            r#"
            var key = 'myProp';
            var C = class {
                get [key]() { return 'computed get'; }
            };
            C.prototype['myProp']
        "#
        ),
        JsValue::String("computed get".into())
    );
}

// Test setter via prototype
#[test]
fn test_class_setter_via_prototype() {
    assert_eq!(
        eval(
            r#"
            var stringSet;
            var C = class {
                get foo() { return 'get string'; }
                set foo(param) { stringSet = param; }
            };
            C.prototype['foo'] = 'set string';
            stringSet
        "#
        ),
        JsValue::String("set string".into())
    );
}

// Test static getter/setter
#[test]
fn test_class_static_getter_setter() {
    // Simple test with public static field first
    assert_eq!(
        eval(
            r#"
            class Config {
                static _count: number = 0;

                static get count(): number {
                    return Config._count;
                }

                static set count(value: number) {
                    Config._count = value;
                }
            }
            Config.count = 5;
            Config.count
        "#
        ),
        JsValue::Number(5.0)
    );
}

// Test static initialization block
#[test]
fn test_static_initialization_block() {
    assert_eq!(
        eval(
            r#"
            class Config {
                static initialized: boolean = false;
                static value: number = 0;

                static {
                    Config.initialized = true;
                    Config.value = 42;
                }
            }
            Config.value
        "#
        ),
        JsValue::Number(42.0)
    );
}

// Test static block with complex initialization
#[test]
fn test_static_block_complex() {
    assert_eq!(
        eval(
            r#"
            class Counter {
                static count: number = 0;
                static doubled: number = 0;

                static {
                    Counter.count = 5;
                    Counter.doubled = Counter.count * 2;
                }
            }
            Counter.doubled
        "#
        ),
        JsValue::Number(10.0)
    );
}

// Test static private field
#[test]
fn test_static_private_field() {
    assert_eq!(
        eval(
            r#"
            class Counter {
                static #count: number = 0;

                static increment(): void {
                    Counter.#count++;
                }

                static getCount(): number {
                    return Counter.#count;
                }
            }
            Counter.increment();
            Counter.increment();
            Counter.getCount()
        "#
        ),
        JsValue::Number(2.0)
    );
}

// Test static private field in instance method
#[test]
fn test_static_private_field_in_instance() {
    assert_eq!(
        eval(
            r#"
            class Entity {
                static #nextId: number = 1;
                #id: number;

                constructor() {
                    this.#id = Entity.#nextId++;
                }

                getId(): number {
                    return this.#id;
                }
            }
            const e1 = new Entity();
            const e2 = new Entity();
            e1.getId() + e2.getId()
        "#
        ),
        JsValue::Number(3.0) // 1 + 2
    );
}

// Test spread in new expression
#[test]
fn test_spread_in_new() {
    assert_eq!(
        eval(
            r#"
            class Point {
                x: number;
                y: number;
                constructor(x: number, y: number) {
                    this.x = x;
                    this.y = y;
                }
            }
            const args: number[] = [10, 20];
            const p = new Point(...args);
            p.x + p.y
        "#
        ),
        JsValue::Number(30.0)
    );
}

// Test spread in new with mixed args
#[test]
fn test_spread_in_new_mixed_args() {
    assert_eq!(
        eval(
            r#"
            class Triple {
                a: number;
                b: number;
                c: number;
                constructor(a: number, b: number, c: number) {
                    this.a = a;
                    this.b = b;
                    this.c = c;
                }
            }
            const rest: number[] = [2, 3];
            const t = new Triple(1, ...rest);
            t.a * 100 + t.b * 10 + t.c
        "#
        ),
        JsValue::Number(123.0)
    );
}

// Test spread in new with multiple spreads
#[test]
fn test_spread_in_new_multiple() {
    assert_eq!(
        eval(
            r#"
            class Sum {
                total: number;
                constructor(a: number, b: number, c: number, d: number) {
                    this.total = a + b + c + d;
                }
            }
            const first: number[] = [1, 2];
            const second: number[] = [3, 4];
            const s = new Sum(...first, ...second);
            s.total
        "#
        ),
        JsValue::Number(10.0)
    );
}

// ============================================================================
// SUPER KEYWORD COMPREHENSIVE TEST SUITE
// ============================================================================

// Test super.property access in instance methods
#[test]
fn test_super_property_in_method() {
    assert_eq!(
        eval(
            r#"
            class B {
                get x() { return 2; }
            }
            class C extends B {
                method() {
                    return super.x;
                }
            }
            new C().method()
        "#
        ),
        JsValue::Number(2.0)
    );
}

// Test super.method() call in instance methods
#[test]
fn test_super_method_in_method() {
    assert_eq!(
        eval(
            r#"
            class B {
                method() { return 1; }
            }
            class C extends B {
                method() {
                    return super.method();
                }
            }
            new C().method()
        "#
        ),
        JsValue::Number(1.0)
    );
}

// Test super in getter
#[test]
fn test_super_in_getter() {
    assert_eq!(
        eval(
            r#"
            class B {
                method() { return 1; }
                get x() { return 2; }
            }
            class C extends B {
                get y() {
                    return super.x + super.method();
                }
            }
            new C().y
        "#
        ),
        JsValue::Number(3.0)
    );
}

// Test super in setter
#[test]
fn test_super_in_setter() {
    assert_eq!(
        eval(
            r#"
            class B {
                get x() { return 10; }
            }
            class C extends B {
                result: number = 0;
                set y(v: number) {
                    this.result = v + super.x;
                }
            }
            const c = new C();
            c.y = 5;
            c.result
        "#
        ),
        JsValue::Number(15.0)
    );
}

// Test super in static methods
#[test]
fn test_super_in_static_method() {
    assert_eq!(
        eval(
            r#"
            class B {
                static method() { return 1; }
                static get x() { return 2; }
            }
            class C extends B {
                static method() {
                    return super.x + super.method();
                }
            }
            C.method()
        "#
        ),
        JsValue::Number(3.0)
    );
}

// Test super in static getter
#[test]
fn test_super_in_static_getter() {
    assert_eq!(
        eval(
            r#"
            class B {
                static get value() { return 42; }
            }
            class C extends B {
                static get doubled() {
                    return super.value * 2;
                }
            }
            C.doubled
        "#
        ),
        JsValue::Number(84.0)
    );
}

// Test super in static setter
#[test]
fn test_super_in_static_setter() {
    assert_eq!(
        eval(
            r#"
            class B {
                static get base() { return 10; }
            }
            class C extends B {
                static result: number = 0;
                static set value(v: number) {
                    C.result = v + super.base;
                }
            }
            C.value = 5;
            C.result
        "#
        ),
        JsValue::Number(15.0)
    );
}

// Test super with computed property access
#[test]
fn test_super_computed_property() {
    assert_eq!(
        eval(
            r#"
            class B {
                getValue() { return 100; }
            }
            class C extends B {
                test() {
                    const name = "getValue";
                    return super[name]();
                }
            }
            new C().test()
        "#
        ),
        JsValue::Number(100.0)
    );
}

// Test super in nested function (should refer to class's super)
#[test]
fn test_super_in_arrow_inside_method() {
    assert_eq!(
        eval(
            r#"
            class B {
                value() { return 5; }
            }
            class C extends B {
                test() {
                    const arrow = () => super.value();
                    return arrow();
                }
            }
            new C().test()
        "#
        ),
        JsValue::Number(5.0)
    );
}

// Test super with deep inheritance chain - 2 levels
#[test]
fn test_super_method_two_levels() {
    assert_eq!(
        eval(
            r#"
            class A {
                value() { return "A"; }
            }
            class B extends A {
                value() { return super.value() + "B"; }
            }
            new B().value()
        "#
        ),
        JsValue::from("AB")
    );
}

// Debug test for static super
#[test]
fn test_super_static_debug() {
    // Super simple static method test - without super first
    assert_eq!(
        eval(
            r#"
            class B {
                static method() { return 42; }
            }
            // Test that B.method works
            B.method()
        "#
        ),
        JsValue::Number(42.0)
    );

    // Now test with super
    assert_eq!(
        eval(
            r#"
            class B {
                static method() { return 42; }
            }
            class C extends B {
                static method() {
                    console.log("C.method called");
                    console.log("B.method exists:", typeof B.method);
                    console.log("super.method is:", typeof super.method);
                    return super.method();
                }
            }
            C.method()
        "#
        ),
        JsValue::Number(42.0)
    );
}

// Debug test to check call depth
#[test]
fn test_super_call_depth_debug() {
    // Simple test: just one class with no super
    assert_eq!(
        eval(
            r#"
            class A {
                value() { return "A"; }
            }
            new A().value()
        "#
        ),
        JsValue::from("A")
    );
}

// Test 2-level with super
#[test]
fn test_super_two_levels_debug() {
    // 2-level with super.value() call
    assert_eq!(
        eval(
            r#"
            class A {
                value() { return "A"; }
            }
            class B extends A {
                value() { return super.value() + "B"; }
            }
            new B().value()
        "#
        ),
        JsValue::from("AB")
    );
}

// Test 3-level with console.log to see what's happening
#[test]
fn test_super_three_levels_debug() {
    // 3-level with console.log
    assert_eq!(
        eval(
            r#"
            class A {
                value() {
                    console.log("A.value called");
                    return "A";
                }
            }
            class B extends A {
                value() {
                    console.log("B.value called");
                    const a = super.value();
                    console.log("B.value got:", a);
                    return a + "B";
                }
            }
            class C extends B {
                value() {
                    console.log("C.value called");
                    const b = super.value();
                    console.log("C.value got:", b);
                    return b + "C";
                }
            }
            new C().value()
        "#
        ),
        JsValue::from("ABC")
    );
}

// Test super with deep inheritance chain - 3 levels
// Uses trampoline to avoid Rust stack overflow
#[test]
fn test_super_deep_inheritance() {
    assert_eq!(
        eval(
            r#"
            class A {
                value() { return "A"; }
            }
            class B extends A {
                value() { return super.value() + "B"; }
            }
            class C extends B {
                value() { return super.value() + "C"; }
            }
            new C().value()
        "#
        ),
        JsValue::from("ABC")
    );
}

// Test super() call in constructor with method call after
#[test]
fn test_super_call_then_super_property() {
    assert_eq!(
        eval(
            r#"
            class B {
                name: string;
                constructor(name: string) {
                    this.name = name;
                }
                greet() { return "Hello, " + this.name; }
            }
            class C extends B {
                constructor(name: string) {
                    super(name);
                }
                greeting() {
                    return super.greet() + "!";
                }
            }
            new C("World").greeting()
        "#
        ),
        JsValue::from("Hello, World!")
    );
}

// Test super property access to a regular property (not getter)
#[test]
fn test_super_regular_property() {
    assert_eq!(
        eval(
            r#"
            class B {
                value: number = 42;
            }
            class C extends B {
                getValue() {
                    return super.value;
                }
            }
            new C().getValue()
        "#
        ),
        JsValue::Undefined // Instance properties aren't on prototype
    );
}

// Test super with prototype property
#[test]
fn test_super_prototype_property() {
    assert_eq!(
        eval(
            r#"
            class B {}
            B.prototype.value = 42;
            class C extends B {
                getValue() {
                    return super.value;
                }
            }
            new C().getValue()
        "#
        ),
        JsValue::Number(42.0)
    );
}

// Test super in constructor before super() - should error
#[test]
fn test_super_property_before_super_call() {
    // Accessing super.x before super() should work (it's super() that must come first for `this`)
    assert_eq!(
        eval(
            r#"
            class B {
                static getValue() { return 10; }
            }
            class C extends B {
                result: number;
                constructor() {
                    const v = B.getValue(); // Can access parent's static before super()
                    super();
                    this.result = v;
                }
            }
            new C().result
        "#
        ),
        JsValue::Number(10.0)
    );
}

// Test super.method with different this binding
#[test]
fn test_super_method_this_binding() {
    assert_eq!(
        eval(
            r#"
            class B {
                name: string = "parent";
                getName() { return this.name; }
            }
            class C extends B {
                name: string = "child";
                getParentName() {
                    return super.getName();
                }
            }
            new C().getParentName()
        "#
        ),
        JsValue::from("child") // super.method() uses child's `this`
    );
}

// Test super with Symbol property
#[test]
fn test_super_symbol_property() {
    assert_eq!(
        eval(
            r#"
            const sym = Symbol("test");
            class B {}
            B.prototype[sym] = "symbol value";
            class C extends B {
                getValue() {
                    return super[sym];
                }
            }
            new C().getValue()
        "#
        ),
        JsValue::from("symbol value")
    );
}

// Test super in async method
#[test]
fn test_super_in_async_method() {
    assert_eq!(
        eval(
            r#"
            class B {
                getValue() { return Promise.resolve(42); }
            }
            class C extends B {
                async getAsyncValue() {
                    return await super.getValue();
                }
            }
            await new C().getAsyncValue()
        "#
        ),
        JsValue::Number(42.0)
    );
}

// Test super in generator method
#[test]
fn test_super_in_generator_method() {
    assert_eq!(
        eval(
            r#"
            class B {
                getValue() { return 10; }
            }
            class C extends B {
                *gen() {
                    yield super.getValue();
                    yield super.getValue() * 2;
                }
            }
            const g = new C().gen();
            g.next().value + g.next().value
        "#
        ),
        JsValue::Number(30.0)
    );
}

// Test multiple super accesses in same method
#[test]
fn test_multiple_super_accesses() {
    assert_eq!(
        eval(
            r#"
            class B {
                a() { return 1; }
                b() { return 2; }
                c() { return 3; }
            }
            class C extends B {
                sum() {
                    return super.a() + super.b() + super.c();
                }
            }
            new C().sum()
        "#
        ),
        JsValue::Number(6.0)
    );
}

// Test super assignment (super.x = value)
#[test]
fn test_super_property_assignment() {
    assert_eq!(
        eval(
            r#"
            class B {
                value: number = 0;
            }
            class C extends B {
                setValue(v: number) {
                    super.value = v; // Sets on the instance, not prototype
                }
                getValue() {
                    return this.value;
                }
            }
            const c = new C();
            c.setValue(42);
            c.getValue()
        "#
        ),
        JsValue::Number(42.0)
    );
}

// Test super.x in constructor (after super() call)
#[test]
fn test_super_property_in_constructor() {
    assert_eq!(
        eval(
            r#"
            class B {}
            B.prototype.x = 42;
            class C extends B {
                result: number;
                constructor() {
                    super();
                    this.result = super.x;
                }
            }
            new C().result
        "#
        ),
        JsValue::Number(42.0)
    );
}

// Test Object.getPrototypeOf with super
#[test]
fn test_super_matches_prototype() {
    assert_eq!(
        eval(
            r#"
            class B {
                test() { return "B"; }
            }
            class C extends B {
                checkSuper() {
                    // super should resolve to B.prototype for method lookup
                    return super.test() === B.prototype.test.call(this);
                }
            }
            new C().checkSuper()
        "#
        ),
        JsValue::Boolean(true)
    );
}
