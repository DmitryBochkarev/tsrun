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
        eval(r#"
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
        "#),
        JsValue::Number(1.0)
    );
}

// Test static methods
#[test]
fn test_class_static_method() {
    assert_eq!(
        eval(r#"
            class MathHelper {
                static double(x: number): number {
                    return x * 2;
                }
            }
            MathHelper.double(5)
        "#),
        JsValue::Number(10.0)
    );
}

// Test static fields
#[test]
fn test_class_static_field() {
    assert_eq!(
        eval(r#"
            class Config {
                static version: string = "1.0";
            }
            Config.version
        "#),
        JsValue::from("1.0")
    );
}

// Test class inheritance
#[test]
fn test_class_extends() {
    assert_eq!(
        eval(r#"
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
        "#),
        JsValue::from("woof")
    );
}

// Test super() call in constructor
#[test]
fn test_class_super_call() {
    assert_eq!(
        eval(r#"
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
        "#),
        JsValue::from("Rex is a German Shepherd")
    );
}

// Test super.method() call
#[test]
fn test_class_super_method() {
    assert_eq!(
        eval(r#"
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
        "#),
        JsValue::from("generic sound and woof")
    );
}

// Class expression tests
#[test]
fn test_class_expression() {
    assert_eq!(
        eval(r#"
            const Foo = class {
                value: number = 10;
                getValue(): number {
                    return this.value;
                }
            };
            const f = new Foo();
            f.getValue()
        "#),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_class_expression_named() {
    assert_eq!(
        eval(r#"
            const MyClass = class Named {
                name: string = "test";
                getName(): string {
                    return this.name;
                }
            };
            const m = new MyClass();
            m.getName()
        "#),
        JsValue::from("test")
    );
}

// Private field tests
#[test]
fn test_private_field_basic() {
    assert_eq!(
        eval(r#"
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
        "#),
        JsValue::Number(2.0)
    );
}

#[test]
fn test_private_field_initial_value() {
    assert_eq!(
        eval(r#"
            class Box {
                #value: number = 42;
                getValue(): number {
                    return this.#value;
                }
            }
            const b: Box = new Box();
            b.getValue()
        "#),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_private_method() {
    assert_eq!(
        eval(r#"
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
        "#),
        JsValue::Number(10.0)
    );
}

// Test getter/setter in classes
#[test]
fn test_class_getter_setter() {
    assert_eq!(
        eval(r#"
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
        "#),
        JsValue::Number(10.0)
    );
}

// Test getter-only property
#[test]
fn test_class_getter_only() {
    assert_eq!(
        eval(r#"
            class Box {
                #value: number = 42;

                get value(): number {
                    return this.#value;
                }
            }
            const b = new Box();
            b.value
        "#),
        JsValue::Number(42.0)
    );
}

// Test static getter/setter
#[test]
fn test_class_static_getter_setter() {
    // Simple test with public static field first
    assert_eq!(
        eval(r#"
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
        "#),
        JsValue::Number(5.0)
    );
}
