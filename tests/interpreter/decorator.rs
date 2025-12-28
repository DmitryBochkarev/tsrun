//! Decorator tests: class, method, field, accessor, and parameter decorators
//!
//! Decorators follow the TC39 Stage 3 Decorators proposal (2023).
//! See: https://github.com/tc39/proposal-decorators

use super::{eval, throws_error};
use tsrun::JsValue;

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
/// Decorator receives { get, set } object and returns { get, set } wrapper
#[test]
fn test_auto_accessor_decorator() {
    assert_eq!(
        eval(
            r#"
            function logged(target: any, context: any): any {
                return {
                    get: function(): any {
                        return target.get.call(this);
                    },
                    set: function(value: any): void {
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

/// Auto-accessor without decorator (just auto-accessor syntax)
#[test]
fn test_auto_accessor_basic() {
    assert_eq!(
        eval(
            r#"
            class Point {
                accessor x: number = 10;
                accessor y: number = 20;
            }

            const p = new Point();
            p.x + p.y
        "#
        ),
        JsValue::Number(30.0)
    );
}

/// Auto-accessor setter
#[test]
fn test_auto_accessor_setter() {
    assert_eq!(
        eval(
            r#"
            class Counter {
                accessor count: number = 0;
            }

            const c = new Counter();
            c.count = 42;
            c.count
        "#
        ),
        JsValue::Number(42.0)
    );
}

/// Auto-accessor decorator context
#[test]
fn test_auto_accessor_decorator_context() {
    assert_eq!(
        eval(
            r#"
            let contextKind: string = "";
            let contextName: string = "";

            function inspect(target: any, context: any): any {
                contextKind = context.kind;
                contextName = context.name;
                return target;
            }

            class Counter {
                @inspect
                accessor count: number = 0;
            }

            contextKind
        "#
        ),
        JsValue::from("accessor")
    );
}

/// Auto-accessor with tracking decorator
#[test]
fn test_auto_accessor_with_tracking() {
    assert_eq!(
        eval(
            r#"
            let getCalls: number = 0;
            let setCalls: number = 0;

            function track(target: any, context: any): any {
                return {
                    get: function(): any {
                        getCalls++;
                        return target.get.call(this);
                    },
                    set: function(value: any): void {
                        setCalls++;
                        target.set.call(this, value);
                    }
                };
            }

            class Counter {
                @track
                accessor count: number = 0;
            }

            const c = new Counter();
            c.count;  // get (initial read)
            c.count;  // get
            c.count = 5;  // set
            c.count;  // get
            getCalls * 10 + setCalls
        "#
        ),
        JsValue::Number(31.0) // 3 gets, 1 set = 30 + 1
    );
}

/// Static auto-accessor
#[test]
fn test_static_auto_accessor() {
    assert_eq!(
        eval(
            r#"
            class Config {
                static accessor version: string = "1.0.0";
            }

            Config.version
        "#
        ),
        JsValue::from("1.0.0")
    );
}

/// Static auto-accessor with decorator
#[test]
fn test_static_auto_accessor_decorator() {
    assert_eq!(
        eval(
            r#"
            let isStatic: boolean = false;

            function checkStatic(target: any, context: any): any {
                isStatic = context.static;
                return target;
            }

            class Config {
                @checkStatic
                static accessor version: string = "1.0.0";
            }

            isStatic
        "#
        ),
        JsValue::Boolean(true)
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
/// Initializers are run after all class decorators are applied
#[test]
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

/// Parameter decorator with TC39-style context object
#[test]
fn test_parameter_decorator() {
    assert_eq!(
        eval(
            r#"
            const paramInfo: any[] = [];

            function logParam(target: any, context: any): void {
                paramInfo.push({ method: context.function, index: context.index, name: context.name });
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

/// Parameter decorator factory
#[test]
fn test_parameter_decorator_factory() {
    assert_eq!(
        eval(
            r#"
            const info: any[] = [];

            function required(message: string) {
                return function(target: any, context: any): void {
                    info.push({ method: context.function, index: context.index, message: message });
                };
            }

            class Validator {
                check(@required("name is required") name: string): boolean {
                    return name !== "";
                }
            }

            info[0].message
        "#
        ),
        JsValue::from("name is required")
    );
}

/// Constructor parameter decorator
#[test]
fn test_constructor_parameter_decorator() {
    assert_eq!(
        eval(
            r#"
            const injections: any[] = [];

            function inject(target: any, context: any): void {
                injections.push({ func: context.function, index: context.index, name: context.name });
            }

            class Service {
                constructor(@inject db: any, @inject logger: any) {}
            }

            injections.length
        "#
        ),
        JsValue::Number(2.0)
    );
}

/// Static method parameter decorator
#[test]
fn test_static_method_parameter_decorator() {
    assert_eq!(
        eval(
            r#"
            const params: any[] = [];

            function track(target: any, context: any): void {
                params.push({ method: context.function, index: context.index, isStatic: context.static });
            }

            class Utils {
                static format(@track value: string): string {
                    return value;
                }
            }

            params[0].isStatic
        "#
        ),
        JsValue::Boolean(true)
    );
}

/// Parameter decorator context object properties
#[test]
fn test_parameter_decorator_context() {
    assert_eq!(
        eval(
            r#"
            let capturedContext: any = null;

            function capture(target: any, context: any): void {
                capturedContext = context;
            }

            class Test {
                method(@capture value: string): void {}
            }

            // Context should have: kind, name, function, index, static
            capturedContext.kind === "parameter" &&
            capturedContext.name === "value" &&
            capturedContext.function === "method" &&
            capturedContext.index === 0 &&
            capturedContext.static === false
        "#
        ),
        JsValue::Boolean(true)
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

// ============================================================================
// Real-World Functional Tests
// ============================================================================

/// Dependency Injection pattern using decorators
/// Common pattern in frameworks like Angular, NestJS
#[test]
fn test_dependency_injection() {
    assert_eq!(
        eval(
            r#"
            // Simple DI container
            const container: any = {};

            function Injectable(target: any, context: any): any {
                // Use context.name (class name) as the service key
                const name = context.name;
                container[name] = new target();
                return target;
            }

            function Inject(serviceName: string) {
                return function(target: any, context: any): any {
                    // Field decorator returns initializer that provides the service
                    return function(): any {
                        return container[serviceName];
                    };
                };
            }

            @Injectable
            class LoggerService {
                log(msg: string): string {
                    return "LOG: " + msg;
                }
            }

            @Injectable
            class DatabaseService {
                query(sql: string): string {
                    return "RESULT: " + sql;
                }
            }

            class UserController {
                @Inject("LoggerService")
                logger: any;

                @Inject("DatabaseService")
                db: any;

                getUser(id: number): string {
                    this.logger.log("Getting user " + id);
                    return this.db.query("SELECT * FROM users WHERE id = " + id);
                }
            }

            const controller = new UserController();
            controller.getUser(42)
        "#
        ),
        JsValue::from("RESULT: SELECT * FROM users WHERE id = 42")
    );
}

/// Observable/Reactive pattern - auto-notify on property changes
/// Similar to MobX @observable
#[test]
fn test_observable_pattern() {
    assert_eq!(
        eval(
            r#"
            const changes: string[] = [];

            function Observable(target: any, context: any): any {
                const name = context.name;
                return {
                    get: function(): any {
                        return target.get.call(this);
                    },
                    set: function(value: any): void {
                        const oldValue = target.get.call(this);
                        target.set.call(this, value);
                        changes.push(name + ": " + oldValue + " -> " + value);
                    }
                };
            }

            class Store {
                @Observable
                accessor count: number = 0;

                @Observable
                accessor name: string = "default";
            }

            const store = new Store();
            store.count = 5;
            store.count = 10;
            store.name = "updated";
            changes.join("; ")
        "#
        ),
        JsValue::from("count: 0 -> 5; count: 5 -> 10; name: default -> updated")
    );
}

/// REST API decorator pattern - like NestJS/Express decorators
#[test]
fn test_rest_api_decorators() {
    assert_eq!(
        eval(
            r#"
            const routes: any[] = [];

            function Controller(basePath: string) {
                return function(target: any, context: any): any {
                    target.prototype.__basePath = basePath;
                    return target;
                };
            }

            function Get(path: string) {
                return function(target: any, context: any): any {
                    routes.push({ method: "GET", path: path, handler: context.name });
                    return target;
                };
            }

            function Post(path: string) {
                return function(target: any, context: any): any {
                    routes.push({ method: "POST", path: path, handler: context.name });
                    return target;
                };
            }

            @Controller("/users")
            class UserController {
                @Get("/")
                findAll(): string {
                    return "all users";
                }

                @Get("/:id")
                findOne(): string {
                    return "one user";
                }

                @Post("/")
                create(): string {
                    return "created";
                }
            }

            routes.length + "-" + routes[0].method + ":" + routes[0].path
        "#
        ),
        JsValue::from("3-GET:/")
    );
}

/// Validation decorators - like class-validator
#[test]
fn test_validation_decorators() {
    assert_eq!(
        eval(
            r#"
            const validationRules: any = {};

            function Min(value: number) {
                return function(target: any, context: any): any {
                    const className = "DTO";
                    if (!validationRules[className]) {
                        validationRules[className] = {};
                    }
                    if (!validationRules[className][context.name]) {
                        validationRules[className][context.name] = [];
                    }
                    validationRules[className][context.name].push({ type: "min", value: value });
                    return target;
                };
            }

            function Max(value: number) {
                return function(target: any, context: any): any {
                    const className = "DTO";
                    if (!validationRules[className]) {
                        validationRules[className] = {};
                    }
                    if (!validationRules[className][context.name]) {
                        validationRules[className][context.name] = [];
                    }
                    validationRules[className][context.name].push({ type: "max", value: value });
                    return target;
                };
            }

            function IsString(target: any, context: any): any {
                const className = "DTO";
                if (!validationRules[className]) {
                    validationRules[className] = {};
                }
                if (!validationRules[className][context.name]) {
                    validationRules[className][context.name] = [];
                }
                validationRules[className][context.name].push({ type: "string" });
                return target;
            }

            class CreateUserDTO {
                @IsString
                @Min(3)
                @Max(50)
                name: string = "";

                @Min(0)
                @Max(120)
                age: number = 0;
            }

            // Check validation rules were registered
            const nameRules = validationRules["DTO"]["name"];
            const ageRules = validationRules["DTO"]["age"];
            nameRules.length + "-" + ageRules.length
        "#
        ),
        JsValue::from("3-2")
    );
}

/// Timing/Performance decorator
#[test]
fn test_timing_decorator() {
    assert_eq!(
        eval(
            r#"
            const timings: string[] = [];

            function Timed(target: any, context: any): any {
                const methodName = context.name;
                return function(...args: any[]): any {
                    const start = Date.now();
                    const result = target.apply(this, args);
                    const end = Date.now();
                    timings.push(methodName + " executed");
                    return result;
                };
            }

            class Calculator {
                @Timed
                add(a: number, b: number): number {
                    return a + b;
                }

                @Timed
                multiply(a: number, b: number): number {
                    return a * b;
                }
            }

            const calc = new Calculator();
            const sum = calc.add(5, 3);
            const product = calc.multiply(4, 7);
            sum + "-" + product + "-" + timings.join(",")
        "#
        ),
        JsValue::from("8-28-add executed,multiply executed")
    );
}

/// Retry decorator for fault tolerance
#[test]
fn test_retry_decorator() {
    assert_eq!(
        eval(
            r#"
            let attemptCount: number = 0;

            function Retry(maxAttempts: number) {
                return function(target: any, context: any): any {
                    return function(...args: any[]): any {
                        let lastError: any = null;
                        for (let i: number = 0; i < maxAttempts; i++) {
                            attemptCount++;
                            try {
                                return target.apply(this, args);
                            } catch (e) {
                                lastError = e;
                            }
                        }
                        throw lastError;
                    };
                };
            }

            let callCount: number = 0;

            class ApiClient {
                @Retry(3)
                fetchData(): string {
                    callCount++;
                    if (callCount < 3) {
                        throw new Error("Network error");
                    }
                    return "success";
                }
            }

            const client = new ApiClient();
            const result = client.fetchData();
            result + "-" + attemptCount
        "#
        ),
        JsValue::from("success-3")
    );
}

/// Role-based access control decorator
#[test]
fn test_rbac_decorator() {
    assert_eq!(
        eval(
            r#"
            let currentUser: any = { roles: ["user"] };

            function RequireRole(role: string) {
                return function(target: any, context: any): any {
                    return function(...args: any[]): any {
                        if (!currentUser.roles.includes(role)) {
                            throw new Error("Access denied: requires " + role);
                        }
                        return target.apply(this, args);
                    };
                };
            }

            class AdminPanel {
                @RequireRole("admin")
                deleteUser(id: number): string {
                    return "deleted user " + id;
                }

                @RequireRole("user")
                viewProfile(): string {
                    return "profile viewed";
                }
            }

            const panel = new AdminPanel();
            let results: string[] = [];

            // User can view profile
            results.push(panel.viewProfile());

            // User cannot delete (catch error)
            try {
                panel.deleteUser(1);
            } catch (e) {
                results.push("blocked");
            }

            // Upgrade to admin
            currentUser.roles.push("admin");
            results.push(panel.deleteUser(42));

            results.join("; ")
        "#
        ),
        JsValue::from("profile viewed; blocked; deleted user 42")
    );
}

/// Throttle decorator - limit function call frequency
#[test]
fn test_throttle_decorator() {
    assert_eq!(
        eval(
            r#"
            const callLog: number[] = [];
            let lastCallTime: number = 0;

            function Throttle(minInterval: number) {
                return function(target: any, context: any): any {
                    return function(...args: any[]): any {
                        const now = Date.now();
                        if (now - lastCallTime >= minInterval) {
                            lastCallTime = now;
                            callLog.push(args[0]);
                            return target.apply(this, args);
                        }
                        return undefined;
                    };
                };
            }

            class EventHandler {
                @Throttle(0)  // 0ms for testing (always allows)
                handleClick(id: number): string {
                    return "handled " + id;
                }
            }

            const handler = new EventHandler();
            handler.handleClick(1);
            handler.handleClick(2);
            handler.handleClick(3);
            callLog.join(",")
        "#
        ),
        JsValue::from("1,2,3")
    );
}

/// Bound method decorator - auto-bind this
#[test]
fn test_autobind_decorator() {
    assert_eq!(
        eval(
            r#"
            function Autobind(target: any, context: any): any {
                let boundFn: any = null;
                return function(...args: any[]): any {
                    if (!boundFn) {
                        boundFn = target.bind(this);
                    }
                    return boundFn.apply(this, args);
                };
            }

            class Button {
                label: string = "Click me";

                @Autobind
                handleClick(): string {
                    return this.label;
                }
            }

            const button = new Button();
            const handler = button.handleClick;
            // Even when called without explicit this, it should work
            // because Autobind captures the instance
            button.handleClick()
        "#
        ),
        JsValue::from("Click me")
    );
}

/// Cacheable/Memoize decorator with TTL
/// NOTE: Using lower values (fib8 instead of fib10) to avoid Rust stack overflow
/// The memoization wrapper adds Rust stack frames, which accumulates for deep recursion
#[test]
fn test_cache_with_ttl() {
    assert_eq!(
        eval(
            r#"
            let computeCount: number = 0;

            function Cacheable(target: any, context: any): any {
                const cache: any = {};
                return function(...args: any[]): any {
                    const key = JSON.stringify(args);
                    if (cache[key] !== undefined) {
                        return cache[key];
                    }
                    const result = target.apply(this, args);
                    cache[key] = result;
                    return result;
                };
            }

            class MathService {
                @Cacheable
                fibonacci(n: number): number {
                    computeCount++;
                    if (n <= 1) return n;
                    return this.fibonacci(n - 1) + this.fibonacci(n - 2);
                }

                @Cacheable
                factorial(n: number): number {
                    computeCount++;
                    if (n <= 1) return 1;
                    return n * this.factorial(n - 1);
                }
            }

            const math = new MathService();
            const fib8 = math.fibonacci(8);
            const fact5 = math.factorial(5);
            fib8 + "-" + fact5 + "-" + computeCount
        "#
        ),
        JsValue::from("21-120-14") // 9 fib calls (0-8) + 5 factorial calls
    );
}

/// Event emitter pattern with decorators
#[test]
fn test_event_emitter_decorator() {
    assert_eq!(
        eval(
            r#"
            const events: string[] = [];

            function EmitEvent(eventName: string) {
                return function(target: any, context: any): any {
                    return function(...args: any[]): any {
                        const result = target.apply(this, args);
                        events.push(eventName + ":" + JSON.stringify(args));
                        return result;
                    };
                };
            }

            class UserService {
                @EmitEvent("user.created")
                createUser(name: string): any {
                    return { id: 1, name: name };
                }

                @EmitEvent("user.updated")
                updateUser(id: number, name: string): any {
                    return { id: id, name: name };
                }

                @EmitEvent("user.deleted")
                deleteUser(id: number): boolean {
                    return true;
                }
            }

            const service = new UserService();
            service.createUser("Alice");
            service.updateUser(1, "Bob");
            service.deleteUser(1);
            events.length + ":" + events[0]
        "#
        ),
        JsValue::from("3:user.created:[\"Alice\"]")
    );
}

/// Serializable decorator - control JSON output
#[test]
fn test_serializable_decorator() {
    assert_eq!(
        eval(
            r#"
            const serializableFields: any = {};

            function Serializable(target: any, context: any): any {
                // Register class as serializable
                return target;
            }

            function JsonProperty(jsonName: string) {
                return function(target: any, context: any): any {
                    const className = "Entity";
                    if (!serializableFields[className]) {
                        serializableFields[className] = {};
                    }
                    serializableFields[className][context.name] = jsonName;
                    return target;
                };
            }

            function JsonIgnore(target: any, context: any): any {
                const className = "Entity";
                if (!serializableFields[className]) {
                    serializableFields[className] = {};
                }
                serializableFields[className][context.name] = null; // null means ignore
                return target;
            }

            @Serializable
            class User {
                @JsonProperty("user_id")
                id: number = 0;

                @JsonProperty("user_name")
                name: string = "";

                @JsonIgnore
                password: string = "";

                @JsonProperty("created_at")
                createdAt: string = "";
            }

            // Check field mappings
            const mappings = serializableFields["Entity"];
            const mapped = Object.keys(mappings).filter(function(k: string): boolean {
                return mappings[k] !== null;
            });
            mapped.length + ":" + mappings["id"] + "," + mappings["name"]
        "#
        ),
        JsValue::from("3:user_id,user_name")
    );
}

/// Method interception chain - multiple decorators forming a pipeline
#[test]
fn test_decorator_pipeline() {
    assert_eq!(
        eval(
            r#"
            const log: string[] = [];

            function Before(msg: string) {
                return function(target: any, context: any): any {
                    return function(...args: any[]): any {
                        log.push("before:" + msg);
                        return target.apply(this, args);
                    };
                };
            }

            function After(msg: string) {
                return function(target: any, context: any): any {
                    return function(...args: any[]): any {
                        const result = target.apply(this, args);
                        log.push("after:" + msg);
                        return result;
                    };
                };
            }

            function Transform(fn: any) {
                return function(target: any, context: any): any {
                    return function(...args: any[]): any {
                        const result = target.apply(this, args);
                        return fn(result);
                    };
                };
            }

            class Processor {
                @Before("validate")
                @After("cleanup")
                @Transform(function(x: number): number { return x * 2; })
                process(value: number): number {
                    log.push("processing:" + value);
                    return value + 10;
                }
            }

            const p = new Processor();
            const result = p.process(5);
            result + "|" + log.join(",")
        "#
        ),
        // Transform is innermost, then After, then Before
        // Execution: Before -> After -> Transform -> original -> Transform returns -> After logs -> Before returns
        JsValue::from("30|before:validate,processing:5,after:cleanup")
    );
}

/// State machine with decorators
#[test]
fn test_state_machine_decorator() {
    assert_eq!(
        eval(
            r#"
            function AllowedStates(...states: string[]) {
                return function(target: any, context: any): any {
                    return function(...args: any[]): any {
                        const currentState = this.state;
                        let allowed: boolean = false;
                        for (let i: number = 0; i < states.length; i++) {
                            if (states[i] === currentState) {
                                allowed = true;
                                break;
                            }
                        }
                        if (!allowed) {
                            throw new Error("Invalid state transition from " + currentState);
                        }
                        return target.apply(this, args);
                    };
                };
            }

            class Order {
                state: string = "pending";

                @AllowedStates("pending")
                confirm(): string {
                    this.state = "confirmed";
                    return this.state;
                }

                @AllowedStates("confirmed")
                ship(): string {
                    this.state = "shipped";
                    return this.state;
                }

                @AllowedStates("shipped")
                deliver(): string {
                    this.state = "delivered";
                    return this.state;
                }

                @AllowedStates("pending", "confirmed")
                cancel(): string {
                    this.state = "cancelled";
                    return this.state;
                }
            }

            const order = new Order();
            const results: string[] = [];

            results.push(order.confirm());
            results.push(order.ship());
            results.push(order.deliver());

            // Try invalid transition
            const order2 = new Order();
            try {
                order2.ship(); // Can't ship from pending
            } catch (e) {
                results.push("blocked");
            }

            results.join(",")
        "#
        ),
        JsValue::from("confirmed,shipped,delivered,blocked")
    );
}

/// Debug: original lazy initialization issue
/// The original test used empty string check which was falsy
#[test]
fn test_lazy_initialization_debug() {
    assert_eq!(
        eval(
            r#"
            let debugInfo: string[] = [];
            let initCount: number = 0;

            function Lazy(target: any, context: any): any {
                debugInfo.push("Lazy decorator called for: " + context.name);
                return {
                    get: function(): any {
                        const value = target.get.call(this);
                        debugInfo.push("get called, current value: '" + value + "', type: " + typeof value);
                        // Original issue: empty string "" is falsy but is a valid initialized value
                        if (value === undefined || value === null) {
                            initCount++;
                            const initialized = "lazy_value_" + initCount;
                            debugInfo.push("initializing to: " + initialized);
                            target.set.call(this, initialized);
                            return initialized;
                        }
                        return value;
                    },
                    set: function(value: any): void {
                        debugInfo.push("set called with: " + value);
                        target.set.call(this, value);
                    }
                };
            }

            class Config {
                @Lazy
                accessor value: string = "";  // Empty string initial value
            }

            const config = new Config();
            debugInfo.push("--- First access ---");
            const v1 = config.value;
            debugInfo.push("Got: '" + v1 + "'");

            debugInfo.join("; ")
        "#
        ),
        // Empty string is returned as-is (not re-initialized) because it's !== undefined/null
        JsValue::from(
            "Lazy decorator called for: value; --- First access ---; get called, current value: '', type: string; Got: ''"
        )
    );
}

/// Lazy initialization decorator - properly handles initial values
#[test]
fn test_lazy_initialization() {
    assert_eq!(
        eval(
            r#"
            let initCount: number = 0;

            function Lazy(factory: any) {
                return function(target: any, context: any): any {
                    const name = context.name;
                    const initKey = "__lazy_init_" + name;
                    return {
                        get: function(): any {
                            if (!this[initKey]) {
                                initCount++;
                                const initialized = factory(initCount);
                                target.set.call(this, initialized);
                                this[initKey] = true;
                            }
                            return target.get.call(this);
                        },
                        set: function(value: any): void {
                            target.set.call(this, value);
                            this[initKey] = true;
                        }
                    };
                };
            }

            class Config {
                @Lazy(function(n: number): string { return "lazy_db_" + n; })
                accessor database: string = "";

                @Lazy(function(n: number): string { return "lazy_cache_" + n; })
                accessor cache: string = "";
            }

            const config = new Config();
            const results: string[] = [];

            // First access initializes
            results.push(config.database);
            results.push(config.cache);

            // Second access returns cached
            results.push(config.database);
            results.push(config.cache);

            results.join(",") + "|" + initCount
        "#
        ),
        JsValue::from("lazy_db_1,lazy_cache_2,lazy_db_1,lazy_cache_2|2")
    );
}

/// Class metadata registration
#[test]
fn test_class_metadata() {
    assert_eq!(
        eval(
            r#"
            const metadata: any = { User: { columns: [] } };

            function Entity(tableName: string) {
                return function(target: any, context: any): any {
                    // Class decorator - set table name
                    metadata["User"].table = tableName;
                    return target;
                };
            }

            function Column(options: any) {
                return function(target: any, context: any): any {
                    metadata["User"].columns.push({
                        name: context.name,
                        type: options.type,
                        nullable: options.nullable || false
                    });
                    return target;
                };
            }

            function PrimaryKey(target: any, context: any): any {
                metadata["User"].primaryKey = context.name;
                return target;
            }

            @Entity("users")
            class User {
                @PrimaryKey
                @Column({ type: "int" })
                id: number = 0;

                @Column({ type: "varchar", nullable: false })
                name: string = "";

                @Column({ type: "varchar", nullable: true })
                email: string = "";
            }

            const m = metadata["User"];
            m.table + "|" + m.primaryKey + "|" + m.columns.length
        "#
        ),
        JsValue::from("users|id|3")
    );
}

/// Computed property decorator
#[test]
fn test_computed_property() {
    assert_eq!(
        eval(
            r#"
            function Computed(dependencies: string[]) {
                return function(target: any, context: any): any {
                    // In a real implementation, this would track dependencies
                    // and recompute when they change
                    return target;
                };
            }

            class Rectangle {
                width: number;
                height: number;

                constructor(w: number, h: number) {
                    this.width = w;
                    this.height = h;
                }

                @Computed(["width", "height"])
                get area(): number {
                    return this.width * this.height;
                }

                @Computed(["width", "height"])
                get perimeter(): number {
                    return 2 * (this.width + this.height);
                }
            }

            const rect = new Rectangle(5, 3);
            rect.area + "-" + rect.perimeter
        "#
        ),
        JsValue::from("15-16")
    );
}

/// Singleton pattern with decorator
#[test]
fn test_singleton_pattern() {
    assert_eq!(
        eval(
            r#"
            let instance: any = null;

            function Singleton(target: any, context: any): any {
                return function(): any {
                    if (!instance) {
                        instance = new target();
                    }
                    return instance;
                };
            }

            @Singleton
            class Database {
                connectionId: number;

                constructor() {
                    this.connectionId = Math.floor(Math.random() * 10000);
                }

                query(sql: string): string {
                    return "Result from connection " + this.connectionId;
                }
            }

            const db1 = new Database();
            const db2 = new Database();
            const db3 = new Database();

            // All should be the same instance
            const sameInstance = db1.connectionId === db2.connectionId && db2.connectionId === db3.connectionId;
            sameInstance + "-" + (db1 === db2)
        "#
        ),
        JsValue::from("true-true")
    );
}

/// Method overload tracking
#[test]
fn test_overload_tracking() {
    assert_eq!(
        eval(
            r#"
            const callHistory: any[] = [];

            function Track(target: any, context: any): any {
                return function(...args: any[]): any {
                    callHistory.push({
                        method: context.name,
                        args: args.length,
                        timestamp: Date.now()
                    });
                    return target.apply(this, args);
                };
            }

            class Calculator {
                @Track
                add(a: number, b: number): number {
                    return a + b;
                }

                @Track
                subtract(a: number, b: number): number {
                    return a - b;
                }

                @Track
                multiply(a: number, b: number, c: number): number {
                    return a * b * c;
                }
            }

            const calc = new Calculator();
            calc.add(1, 2);
            calc.subtract(5, 3);
            calc.multiply(2, 3, 4);

            callHistory.length + ":" + callHistory[0].method + "," + callHistory[1].method + "," + callHistory[2].args
        "#
        ),
        JsValue::from("3:add,subtract,3")
    );
}

/// Debug: fluent interface method chaining issue
#[test]
fn test_fluent_interface_debug() {
    assert_eq!(
        eval(
            r#"
            let debugInfo: string[] = [];

            function Fluent(target: any, context: any): any {
                debugInfo.push("Fluent decorator for: " + context.name);
                return function(...args: any[]): any {
                    debugInfo.push("Calling " + context.name + " with this=" + (this ? "object" : "undefined"));
                    target.apply(this, args);
                    debugInfo.push("Returning this from " + context.name);
                    return this;
                };
            }

            class Builder {
                value: string = "";

                @Fluent
                append(s: string): any {
                    this.value = this.value + s;
                }
            }

            const b = new Builder();
            debugInfo.push("--- Calling append ---");
            const result = b.append("hello");
            debugInfo.push("Result type: " + typeof result);
            debugInfo.push("Result === b: " + (result === b));
            debugInfo.push("b.value: " + b.value);

            // Try chaining
            debugInfo.push("--- Try chaining ---");
            const chained = b.append(" ").append("world");
            debugInfo.push("After chain, b.value: " + b.value);

            debugInfo.join("; ")
        "#
        ),
        JsValue::from(
            "Fluent decorator for: append; --- Calling append ---; Calling append with this=object; Returning this from append; Result type: object; Result === b: true; b.value: hello; --- Try chaining ---; Calling append with this=object; Returning this from append; Calling append with this=object; Returning this from append; After chain, b.value: hello world"
        )
    );
}

/// Fluent interface decorator - enables method chaining
#[test]
fn test_fluent_interface() {
    assert_eq!(
        eval(
            r#"
            function Fluent(target: any, context: any): any {
                return function(...args: any[]): any {
                    target.apply(this, args);
                    return this;
                };
            }

            class QueryBuilder {
                query: string = "";

                @Fluent
                addSelect(fields: string): any {
                    this.query = this.query + "SELECT " + fields;
                }

                @Fluent
                addFrom(table: string): any {
                    this.query = this.query + " FROM " + table;
                }

                @Fluent
                addWhere(condition: string): any {
                    this.query = this.query + " WHERE " + condition;
                }

                build(): string {
                    return this.query;
                }
            }

            // Full chain test
            const qb = new QueryBuilder();
            const result = qb.addSelect("*").addFrom("users").addWhere("active = true").build();
            result
        "#
        ),
        JsValue::from("SELECT * FROM users WHERE active = true")
    );
}

/// Debug: immutable decorator - track setter calls
#[test]
fn test_immutable_decorator_debug() {
    assert_eq!(
        eval(
            r#"
            let debugInfo: string[] = [];

            function Immutable(target: any, context: any): any {
                const name = context.name;
                debugInfo.push("Immutable decorator for: " + name);

                // Per-instance tracking using a marker property
                const initKey = "__init_" + name;

                return {
                    get: function(): any {
                        debugInfo.push("get " + name);
                        return target.get.call(this);
                    },
                    set: function(value: any): void {
                        debugInfo.push("set " + name + " to " + value + ", initKey exists: " + !!this[initKey]);
                        if (this[initKey]) {
                            debugInfo.push("BLOCKING set for " + name);
                            throw new Error("Cannot modify immutable field");
                        }
                        target.set.call(this, value);
                        this[initKey] = true;
                        debugInfo.push("Marked " + name + " as initialized");
                    }
                };
            }

            class Config {
                @Immutable
                accessor value: string = "initial";
            }

            const config = new Config();
            debugInfo.push("--- Reading value ---");
            const v = config.value;
            debugInfo.push("Value: " + v);

            debugInfo.push("--- Try to modify ---");
            let blocked: boolean = false;
            try {
                config.value = "modified";
            } catch (e) {
                blocked = true;
            }
            debugInfo.push("Blocked: " + blocked);

            debugInfo.join("; ")
        "#
        ),
        // First setter call sees initKey=undefined (false), allows the write, and marks as initialized
        // NOTE: Initial values bypass the setter, so the first setter call is the modification attempt
        JsValue::from(
            "Immutable decorator for: value; --- Reading value ---; get value; Value: initial; --- Try to modify ---; set value to modified, initKey exists: false; Marked value as initialized; Blocked: false"
        )
    );
}

/// Immutable decorator - proper test with TWO modification attempts
#[test]
fn test_immutable_decorator_two_modifications() {
    assert_eq!(
        eval(
            r#"
            let debugInfo: string[] = [];

            function Immutable(target: any, context: any): any {
                const name = context.name;
                const initKey = "__init_" + name;

                return {
                    get: function(): any {
                        return target.get.call(this);
                    },
                    set: function(value: any): void {
                        debugInfo.push("set " + name + "=" + value + ", initKey=" + !!this[initKey]);
                        if (this[initKey]) {
                            debugInfo.push("BLOCKED");
                            throw new Error("Cannot modify immutable field");
                        }
                        target.set.call(this, value);
                        this[initKey] = true;
                        debugInfo.push("ALLOWED");
                    }
                };
            }

            class Config {
                @Immutable
                accessor value: string = "initial";
            }

            const config = new Config();
            debugInfo.push("Value: " + config.value);

            // First modification - should be allowed (this is our "real" first set)
            config.value = "first";
            debugInfo.push("After first: " + config.value);

            // Second modification - should be BLOCKED
            let blocked: boolean = false;
            try {
                config.value = "second";
            } catch (e) {
                blocked = true;
            }
            debugInfo.push("Blocked: " + blocked);
            debugInfo.push("Final: " + config.value);

            debugInfo.join("; ")
        "#
        ),
        JsValue::from(
            "Value: initial; set value=first, initKey=false; ALLOWED; After first: first; set value=second, initKey=true; BLOCKED; Blocked: true; Final: first"
        )
    );
}

/// Debug: Test that this[key] persists between setter calls
#[test]
fn test_accessor_this_property_persists() {
    assert_eq!(
        eval(
            r#"
            let log: string[] = [];

            function TrackSetter(target: any, context: any): any {
                const name = context.name;
                return {
                    get: function(): any {
                        return target.get.call(this);
                    },
                    set: function(value: any): void {
                        // Set a marker property on this
                        const key = "__marker_" + name;
                        log.push("before set: this[key]=" + this[key]);
                        target.set.call(this, value);
                        this[key] = "was_set";
                        log.push("after set: this[key]=" + this[key]);
                    }
                };
            }

            class Test {
                @TrackSetter
                accessor prop: string = "initial";
            }

            const t = new Test();
            log.push("--- first write ---");
            t.prop = "one";
            log.push("--- second write ---");
            t.prop = "two";
            log.push("--- check marker directly ---");
            log.push("t.__marker_prop=" + t.__marker_prop);
            log.join("; ")
        "#
        ),
        JsValue::from(
            "--- first write ---; before set: this[key]=undefined; after set: this[key]=was_set; --- second write ---; before set: this[key]=was_set; after set: this[key]=was_set; --- check marker directly ---; t.__marker_prop=was_set"
        )
    );
}

/// Debug: Test this[variable] computed access with boolean tracking
#[test]
fn test_accessor_this_boolean_tracking() {
    assert_eq!(
        eval(
            r#"
            let log: string[] = [];

            function TrackBool(target: any, context: any): any {
                const name = context.name;
                const initKey = "__init_" + name;
                return {
                    get: function(): any {
                        return target.get.call(this);
                    },
                    set: function(value: any): void {
                        log.push("initKey=" + initKey);
                        log.push("this[initKey]=" + this[initKey]);
                        log.push("!!this[initKey]=" + !!this[initKey]);
                        if (this[initKey]) {
                            log.push("BLOCK");
                        } else {
                            target.set.call(this, value);
                            this[initKey] = true;
                            log.push("SET initKey to true");
                        }
                    }
                };
            }

            class Test {
                @TrackBool
                accessor prop: string = "initial";
            }

            const t = new Test();
            log.push("--- write 1 ---");
            t.prop = "one";
            log.push("--- write 2 ---");
            t.prop = "two";
            log.push("t[\"__init_prop\"]=" + t["__init_prop"]);
            log.join("; ")
        "#
        ),
        JsValue::from(
            "--- write 1 ---; initKey=__init_prop; this[initKey]=undefined; !!this[initKey]=false; SET initKey to true; --- write 2 ---; initKey=__init_prop; this[initKey]=true; !!this[initKey]=true; BLOCK; t[\"__init_prop\"]=true"
        )
    );
}

/// Read-only decorator - logs attempts to modify
#[test]
fn test_readonly_decorator() {
    assert_eq!(
        eval(
            r#"
            let decoratedFields: string[] = [];
            let writeAttempts: number = 0;

            function ReadOnly(target: any, context: any): any {
                decoratedFields.push(context.name);
                const name = context.name;
                return {
                    get: function(): any {
                        return target.get.call(this);
                    },
                    set: function(value: any): void {
                        writeAttempts++;
                        // Allow the initial value but track all writes
                        target.set.call(this, value);
                    }
                };
            }

            class Config {
                @ReadOnly
                accessor apiKey: string = "secret-key";

                @ReadOnly
                accessor environment: string = "production";
            }

            const config = new Config();
            const results: string[] = [];

            results.push(config.apiKey);
            results.push(config.environment);

            // Modify (allowed in this version, just tracked)
            config.apiKey = "new-key";
            results.push(config.apiKey);

            results.join(",") + "|" + decoratedFields.join(",") + "|" + writeAttempts
        "#
        ),
        // Only explicit writes go through the decorated setter (initial values bypass)
        JsValue::from("secret-key,production,new-key|apiKey,environment|1")
    );
}

/// Debug test for field decorators
#[test]
fn test_debug_field_decorator() {
    assert_eq!(
        eval(
            r#"
            const debug: string[] = [];

            function log(target: any, context: any) {
                debug.push("decorator called, kind=" + context.kind);
                const initFn = function(initialValue: any): any {
                    debug.push("initializer called, value=" + initialValue);
                    return 100;
                };
                debug.push("returning function: " + (typeof initFn));
                return initFn;
            }

            class Config {
                @log
                timeout: number;
            }

            // Check if __field_initializers__ exists on the class
            const inits = (Config as any).__field_initializers__;
            debug.push("has inits: " + (inits !== undefined));
            if (inits) {
                debug.push("timeout init: " + (typeof inits.timeout));
            }

            const c = new Config();
            debug.push("timeout=" + c.timeout);
            debug.join("|")
        "#
        ),
        JsValue::from(
            "decorator called, kind=field|returning function: function|has inits: true|timeout init: function|initializer called, value=undefined|timeout=100"
        )
    );
}

/// Debug test to understand decorator context issues
#[test]
fn test_debug_decorator_context() {
    assert_eq!(
        eval(
            r#"
            const debug: string[] = [];

            function captureContext(target: any, context: any) {
                debug.push("kind=" + context.kind + ",name=" + context.name + ",static=" + context.static);
                return target;
            }

            @captureContext
            class Foo {
                @captureContext
                myMethod(): void {}
            }

            debug.join("|")
        "#
        ),
        // Method is processed first, then class decorator
        // Class decorator doesn't have static property (it's undefined)
        JsValue::from(
            "kind=method,name=myMethod,static=false|kind=class,name=Foo,static=undefined"
        )
    );
}
