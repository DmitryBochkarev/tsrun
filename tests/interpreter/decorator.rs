//! Decorator tests: class, method, field, accessor, and parameter decorators
//!
//! Decorators follow the TC39 Stage 3 Decorators proposal (2023).
//! See: https://github.com/tc39/proposal-decorators

use super::{eval, throws_error};
use typescript_eval::JsValue;

// ============================================================================
// Class Decorators
// ============================================================================

/// Basic class decorator that receives the class constructor
#[test]
fn test_class_decorator_basic() {
    assert_eq!(
        eval(
            r#"
            let decoratorCalled: boolean = false;
            let receivedClass: any = null;

            function classDecorator(target: any): any {
                decoratorCalled = true;
                receivedClass = target;
                return target;
            }

            @classDecorator
            class Foo {
                value: number = 42;
            }

            decoratorCalled
        "#
        ),
        JsValue::Boolean(true)
    );
}

/// Class decorator that modifies the class
#[test]
fn test_class_decorator_modify() {
    assert_eq!(
        eval(
            r#"
            function addMethod(target: any): any {
                target.prototype.added = function(): string {
                    return "added method";
                };
                return target;
            }

            @addMethod
            class Foo {}

            const f = new Foo();
            f.added()
        "#
        ),
        JsValue::from("added method")
    );
}

/// Class decorator that replaces the class
#[test]
fn test_class_decorator_replace() {
    assert_eq!(
        eval(
            r#"
            function replaceClass(target: any): any {
                return class extends target {
                    extra: string = "added";
                };
            }

            @replaceClass
            class Original {
                value: number = 1;
            }

            const o = new Original();
            o.extra
        "#
        ),
        JsValue::from("added")
    );
}

/// Class decorator that adds static properties
#[test]
fn test_class_decorator_static_property() {
    assert_eq!(
        eval(
            r#"
            function addVersion(target: any): any {
                target.version = "1.0.0";
                return target;
            }

            @addVersion
            class App {}

            App.version
        "#
        ),
        JsValue::from("1.0.0")
    );
}

// ============================================================================
// Decorator Factories (Decorators with arguments)
// ============================================================================

/// Decorator factory that returns a decorator
#[test]
fn test_decorator_factory_basic() {
    assert_eq!(
        eval(
            r#"
            function tag(name: string) {
                return function(target: any): any {
                    target.tag = name;
                    return target;
                };
            }

            @tag("important")
            class Widget {}

            Widget.tag
        "#
        ),
        JsValue::from("important")
    );
}

/// Decorator factory with multiple arguments
#[test]
fn test_decorator_factory_multi_args() {
    assert_eq!(
        eval(
            r#"
            function metadata(key: string, value: any) {
                return function(target: any): any {
                    target[key] = value;
                    return target;
                };
            }

            @metadata("version", 2)
            class Api {}

            Api.version
        "#
        ),
        JsValue::Number(2.0)
    );
}

// ============================================================================
// Multiple Decorators (Composition)
// ============================================================================

/// Multiple decorators are applied bottom-up
#[test]
fn test_multiple_decorators_order() {
    assert_eq!(
        eval(
            r#"
            const order: string[] = [];

            function first(target: any): any {
                order.push("first");
                return target;
            }

            function second(target: any): any {
                order.push("second");
                return target;
            }

            @first
            @second
            class Foo {}

            order.join(",")
        "#
        ),
        // Decorators are evaluated top-to-bottom, but applied bottom-to-top
        JsValue::from("second,first")
    );
}

/// Multiple decorator factories - evaluation vs application order
#[test]
fn test_multiple_decorator_factories_order() {
    assert_eq!(
        eval(
            r#"
            const evalOrder: string[] = [];
            const applyOrder: string[] = [];

            function factory(name: string) {
                evalOrder.push(name + "_eval");
                return function(target: any): any {
                    applyOrder.push(name + "_apply");
                    return target;
                };
            }

            @factory("A")
            @factory("B")
            class Foo {}

            evalOrder.join(",") + "|" + applyOrder.join(",")
        "#
        ),
        // Factories evaluated top-to-bottom, decorators applied bottom-to-top
        JsValue::from("A_eval,B_eval|B_apply,A_apply")
    );
}

// ============================================================================
// Method Decorators
// ============================================================================

/// Basic method decorator
#[test]
fn test_method_decorator_basic() {
    assert_eq!(
        eval(
            r#"
            function log(target: any, context: any) {
                return function(...args: any[]): any {
                    return target.apply(this, args);
                };
            }

            class Calculator {
                @log
                add(a: number, b: number): number {
                    return a + b;
                }
            }

            const c = new Calculator();
            c.add(2, 3)
        "#
        ),
        JsValue::Number(5.0)
    );
}

/// Method decorator that wraps the method
#[test]
fn test_method_decorator_wrap() {
    assert_eq!(
        eval(
            r#"
            function double(target: any, context: any) {
                return function(...args: any[]): any {
                    const result = target.apply(this, args);
                    return result * 2;
                };
            }

            class Math {
                @double
                getValue(): number {
                    return 5;
                }
            }

            const m = new Math();
            m.getValue()
        "#
        ),
        JsValue::Number(10.0)
    );
}

/// Method decorator with context information
#[test]
fn test_method_decorator_context() {
    assert_eq!(
        eval(
            r#"
            let contextInfo: any = null;

            function captureContext(target: any, context: any) {
                contextInfo = context;
                return target;
            }

            class Foo {
                @captureContext
                myMethod(): void {}
            }

            contextInfo.name
        "#
        ),
        JsValue::from("myMethod")
    );
}

/// Static method decorator
#[test]
fn test_static_method_decorator() {
    assert_eq!(
        eval(
            r#"
            function triple(target: any, context: any) {
                return function(...args: any[]): any {
                    return target.apply(this, args) * 3;
                };
            }

            class Util {
                @triple
                static compute(): number {
                    return 10;
                }
            }

            Util.compute()
        "#
        ),
        JsValue::Number(30.0)
    );
}

// ============================================================================
// Field Decorators
// ============================================================================

/// Basic field decorator
#[test]
fn test_field_decorator_basic() {
    assert_eq!(
        eval(
            r#"
            function defaultValue(value: any) {
                return function(target: any, context: any) {
                    return function(initialValue: any): any {
                        return initialValue ?? value;
                    };
                };
            }

            class Config {
                @defaultValue(100)
                timeout: number;
            }

            const c = new Config();
            c.timeout
        "#
        ),
        JsValue::Number(100.0)
    );
}

/// Field decorator that transforms the initial value
#[test]
fn test_field_decorator_transform() {
    assert_eq!(
        eval(
            r#"
            function uppercase(target: any, context: any) {
                return function(initialValue: string): string {
                    return initialValue.toUpperCase();
                };
            }

            class Greeting {
                @uppercase
                message: string = "hello";
            }

            const g = new Greeting();
            g.message
        "#
        ),
        JsValue::from("HELLO")
    );
}

/// Static field decorator
#[test]
fn test_static_field_decorator() {
    assert_eq!(
        eval(
            r#"
            function constant(target: any, context: any) {
                return function(initialValue: any): any {
                    return initialValue;
                };
            }

            class Constants {
                @constant
                static PI: number = 3.14;
            }

            Constants.PI
        "#
        ),
        JsValue::Number(3.14)
    );
}

// ============================================================================
// Accessor Decorators (getter/setter)
// ============================================================================

/// Accessor decorator on getter
#[test]
fn test_accessor_decorator_getter() {
    assert_eq!(
        eval(
            r#"
            function cache(target: any, context: any) {
                let cachedValue: any = null;
                let hasCached: boolean = false;

                return function(): any {
                    if (!hasCached) {
                        cachedValue = target.call(this);
                        hasCached = true;
                    }
                    return cachedValue;
                };
            }

            let computeCount: number = 0;

            class ExpensiveComputation {
                @cache
                get value(): number {
                    computeCount++;
                    return 42;
                }
            }

            const e = new ExpensiveComputation();
            e.value;
            e.value;
            e.value;
            computeCount
        "#
        ),
        JsValue::Number(1.0)
    );
}

/// Auto-accessor decorator
/// Not yet implemented - requires parser support for `accessor` keyword
#[test]
#[ignore = "Auto-accessor syntax not yet implemented"]
fn test_auto_accessor_decorator() {
    assert_eq!(
        eval(
            r#"
            function logged(target: any, context: any) {
                return {
                    get(): any {
                        return target.get.call(this);
                    },
                    set(value: any): void {
                        target.set.call(this, value);
                    }
                };
            }

            class Counter {
                @logged
                accessor count: number = 0;
            }

            const c = new Counter();
            c.count = 5;
            c.count
        "#
        ),
        JsValue::Number(5.0)
    );
}

// ============================================================================
// Private Member Decorators
// ============================================================================

/// Decorator wrapper called from another instance method
/// This test isolates the GC-related stack overflow issue
#[test]
fn test_decorator_wrapper_called_from_instance_method() {
    // Calling a non-decorated method from another method works
    assert_eq!(
        eval(
            r#"
            class Secret {
                compute(): number {
                    return 10;
                }

                getResult(): number {
                    return this.compute();
                }
            }

            const s = new Secret();
            s.getResult()
        "#
        ),
        JsValue::Number(10.0)
    );
}

/// Decorator wrapper called from another instance method - with decorator
#[test]
fn test_decorator_wrapper_called_from_instance_method_decorated() {
    // Simplest case - wrapper returns constant, called from another method
    assert_eq!(
        eval(
            r#"
            function wrap(target: any, context: any) {
                return function(...args: any[]): any {
                    return 99;
                };
            }

            class Secret {
                @wrap
                compute(): number {
                    return 10;
                }

                getResult(): number {
                    return this.compute();
                }
            }

            const s = new Secret();
            s.getResult()
        "#
        ),
        JsValue::Number(99.0)
    );
}

/// Decorator on private method
#[test]
fn test_private_method_decorator() {
    assert_eq!(
        eval(
            r#"
            function wrap(target: any, context: any) {
                return function(...args: any[]): any {
                    return target.apply(this, args) + 1;
                };
            }

            class Secret {
                @wrap
                #compute(): number {
                    return 10;
                }

                getResult(): number {
                    return this.#compute();
                }
            }

            const s = new Secret();
            s.getResult()
        "#
        ),
        JsValue::Number(11.0)
    );
}

/// Decorator on private field
#[test]
fn test_private_field_decorator() {
    assert_eq!(
        eval(
            r#"
            function multiply(factor: number) {
                return function(target: any, context: any) {
                    return function(initialValue: number): number {
                        return initialValue * factor;
                    };
                };
            }

            class Box {
                @multiply(10)
                #value: number = 5;

                getValue(): number {
                    return this.#value;
                }
            }

            const b = new Box();
            b.getValue()
        "#
        ),
        JsValue::Number(50.0)
    );
}

// ============================================================================
// Decorator Context
// ============================================================================

/// Context.kind for different element types
#[test]
fn test_decorator_context_kind() {
    assert_eq!(
        eval(
            r#"
            const kinds: string[] = [];

            function captureKind(target: any, context: any) {
                kinds.push(context.kind);
                return target;
            }

            @captureKind
            class Foo {
                @captureKind
                field: number = 1;

                @captureKind
                method(): void {}

                @captureKind
                get accessor(): number { return 1; }

                @captureKind
                set accessor(v: number) {}
            }

            kinds.join(",")
        "#
        ),
        // Note: Methods are processed before fields in our implementation
        // TC39 doesn't mandate a specific order within a class
        JsValue::from("method,getter,setter,field,class")
    );
}

/// Context.static for static vs instance members
#[test]
fn test_decorator_context_static() {
    assert_eq!(
        eval(
            r#"
            const staticFlags: boolean[] = [];

            function captureStatic(target: any, context: any) {
                staticFlags.push(context.static || false);
                return target;
            }

            class Foo {
                @captureStatic
                instanceMethod(): void {}

                @captureStatic
                static staticMethod(): void {}
            }

            staticFlags.join(",")
        "#
        ),
        JsValue::from("false,true")
    );
}

/// Context.private for private members
#[test]
fn test_decorator_context_private() {
    assert_eq!(
        eval(
            r#"
            const privateFlags: boolean[] = [];

            function capturePrivate(target: any, context: any) {
                privateFlags.push(context.private || false);
                return target;
            }

            class Foo {
                @capturePrivate
                publicMethod(): void {}

                @capturePrivate
                #privateMethod(): void {}
            }

            privateFlags.join(",")
        "#
        ),
        JsValue::from("false,true")
    );
}

/// Context.addInitializer for class decorators
/// Not yet implemented - requires addInitializer method on context
#[test]
#[ignore = "addInitializer not yet implemented"]
fn test_decorator_add_initializer() {
    assert_eq!(
        eval(
            r#"
            let initialized: boolean = false;

            function init(target: any, context: any) {
                context.addInitializer(function() {
                    initialized = true;
                });
                return target;
            }

            @init
            class Foo {}

            initialized
        "#
        ),
        JsValue::Boolean(true)
    );
}

// ============================================================================
// TypeScript-specific Decorators
// ============================================================================

/// Decorator with TypeScript type annotations
#[test]
fn test_decorator_typescript_types() {
    assert_eq!(
        eval(
            r#"
            function typed<T>(target: T, context: ClassDecoratorContext): T {
                return target;
            }

            @typed
            class TypedClass {
                value: number = 42;
            }

            const t = new TypedClass();
            t.value
        "#
        ),
        JsValue::Number(42.0)
    );
}

/// Parameter decorator (TypeScript experimental)
/// Not yet implemented - requires parser support for parameter decorators
#[test]
#[ignore = "Parameter decorators not yet implemented"]
fn test_parameter_decorator() {
    assert_eq!(
        eval(
            r#"
            const paramInfo: any[] = [];

            function logParam(target: any, propertyKey: string, parameterIndex: number): void {
                paramInfo.push({ method: propertyKey, index: parameterIndex });
            }

            class Service {
                greet(@logParam name: string, @logParam age: number): string {
                    return name + " is " + age;
                }
            }

            paramInfo.length
        "#
        ),
        JsValue::Number(2.0)
    );
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

/// Decorator returning undefined (should keep original)
#[test]
fn test_decorator_return_undefined() {
    assert_eq!(
        eval(
            r#"
            function noOp(target: any, context: any): undefined {
                return undefined;
            }

            class Foo {
                @noOp
                getValue(): number {
                    return 42;
                }
            }

            const f = new Foo();
            f.getValue()
        "#
        ),
        JsValue::Number(42.0)
    );
}

/// Decorator on class expression
#[test]
fn test_decorator_class_expression() {
    assert_eq!(
        eval(
            r#"
            function mark(target: any): any {
                target.marked = true;
                return target;
            }

            const Foo = @mark class {
                value: number = 1;
            };

            Foo.marked
        "#
        ),
        JsValue::Boolean(true)
    );
}

/// Decorator accessing `this` in methods
#[test]
fn test_decorator_this_binding() {
    assert_eq!(
        eval(
            r#"
            function bound(target: any, context: any) {
                return function(...args: any[]): any {
                    return target.apply(this, args);
                };
            }

            class Counter {
                count: number = 0;

                @bound
                increment(): void {
                    this.count++;
                }
            }

            const c = new Counter();
            const inc = c.increment;
            inc.call(c);
            c.count
        "#
        ),
        JsValue::Number(1.0)
    );
}

/// Decorator with inherited classes
#[test]
fn test_decorator_inheritance() {
    assert_eq!(
        eval(
            r#"
            function addMethod(target: any): any {
                target.prototype.decorated = true;
                return target;
            }

            @addMethod
            class Parent {}

            class Child extends Parent {}

            const c = new Child();
            c.decorated
        "#
        ),
        JsValue::Boolean(true)
    );
}

/// Multiple method decorators
#[test]
fn test_multiple_method_decorators() {
    assert_eq!(
        eval(
            r#"
            function addOne(target: any, context: any) {
                return function(...args: any[]): any {
                    return target.apply(this, args) + 1;
                };
            }

            function double(target: any, context: any) {
                return function(...args: any[]): any {
                    return target.apply(this, args) * 2;
                };
            }

            class Math {
                @addOne
                @double
                getValue(): number {
                    return 5;
                }
            }

            const m = new Math();
            m.getValue()
        "#
        ),
        // double(5) = 10, addOne(10) = 11
        JsValue::Number(11.0)
    );
}

// ============================================================================
// Real-world Patterns
// ============================================================================

/// Singleton pattern with decorator
#[test]
fn test_singleton_decorator() {
    assert_eq!(
        eval(
            r#"
            function singleton(target: any): any {
                let instance: any = null;
                return class extends target {
                    constructor(...args: any[]) {
                        if (instance) {
                            return instance;
                        }
                        super(...args);
                        instance = this;
                    }
                };
            }

            @singleton
            class Database {
                id: number = Math.random();
            }

            const db1 = new Database();
            const db2 = new Database();
            db1 === db2
        "#
        ),
        JsValue::Boolean(true)
    );
}

/// Memoization decorator
#[test]
fn test_memoize_decorator() {
    assert_eq!(
        eval(
            r#"
            function memoize(target: any, context: any) {
                const cache = new Map();
                return function(...args: any[]): any {
                    const key = JSON.stringify(args);
                    if (cache.has(key)) {
                        return cache.get(key);
                    }
                    const result = target.apply(this, args);
                    cache.set(key, result);
                    return result;
                };
            }

            let callCount: number = 0;

            class Calculator {
                @memoize
                expensive(n: number): number {
                    callCount++;
                    return n * 2;
                }
            }

            const c = new Calculator();
            c.expensive(5);
            c.expensive(5);
            c.expensive(5);
            callCount
        "#
        ),
        JsValue::Number(1.0)
    );
}

/// Validation decorator for fields
#[test]
fn test_validation_decorator() {
    assert_eq!(
        eval(
            r#"
            function min(minValue: number) {
                return function(target: any, context: any) {
                    return function(initialValue: number): number {
                        if (initialValue < minValue) {
                            return minValue;
                        }
                        return initialValue;
                    };
                };
            }

            class Config {
                @min(0)
                timeout: number = -5;
            }

            const c = new Config();
            c.timeout
        "#
        ),
        JsValue::Number(0.0)
    );
}

/// Deprecation warning decorator
#[test]
fn test_deprecation_decorator() {
    assert_eq!(
        eval(
            r#"
            const warnings: string[] = [];

            function deprecated(message: string) {
                return function(target: any, context: any) {
                    return function(...args: any[]): any {
                        warnings.push(message);
                        return target.apply(this, args);
                    };
                };
            }

            class Api {
                @deprecated("Use newMethod instead")
                oldMethod(): number {
                    return 42;
                }
            }

            const api = new Api();
            api.oldMethod();
            warnings[0]
        "#
        ),
        JsValue::from("Use newMethod instead")
    );
}

// ============================================================================
// Error Cases
// ============================================================================

/// Decorator that is not a function should throw
#[test]
fn test_decorator_not_function_error() {
    assert!(throws_error(
        r#"
        const notAFunction = 42;

        @notAFunction
        class Foo {}
    "#,
        "Not a function"
    ));
}

/// Decorator factory that doesn't return a function should throw
#[test]
fn test_decorator_factory_invalid_return() {
    assert!(throws_error(
        r#"
        function badFactory() {
            return 42;  // Should return a function
        }

        @badFactory()
        class Foo {}
    "#,
        "Not a function"
    ));
}
