//! Namespace declaration tests

use super::eval;
use tsrun::JsValue;

// Basic namespace tests
#[test]
fn test_namespace_basic() {
    // Namespace creates an object with exported members
    assert_eq!(
        eval(
            r#"
            namespace MyNamespace {
                export const x = 42;
            }
            MyNamespace.x
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_namespace_function() {
    assert_eq!(
        eval(
            r#"
            namespace Utils {
                export function double(n: number): number {
                    return n * 2;
                }
            }
            Utils.double(21)
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_namespace_multiple_exports() {
    assert_eq!(
        eval(
            r#"
            namespace Math2 {
                export const BASE = 3;
                export function square(x: number): number {
                    return x * x;
                }
            }
            Math2.BASE + Math2.square(2)
        "#
        ),
        JsValue::Number(7.0)
    );
}

#[test]
fn test_namespace_private_member() {
    // Non-exported members should not be accessible outside
    assert_eq!(
        eval(
            r#"
            namespace NS {
                const secret = 100;
                export const public_value = secret + 1;
            }
            NS.public_value
        "#
        ),
        JsValue::Number(101.0)
    );
}

#[test]
fn test_namespace_nested() {
    // Nested namespaces
    assert_eq!(
        eval(
            r#"
            namespace Outer {
                export namespace Inner {
                    export const value = 42;
                }
            }
            Outer.Inner.value
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_namespace_class_typeof() {
    // Debug: check what type the class is
    assert_eq!(
        eval(
            r#"
            namespace Models {
                export class Person {
                    name: string;
                    constructor(name: string) {
                        this.name = name;
                    }
                }
            }
            typeof Models.Person
        "#
        ),
        JsValue::from("function")
    );
}

#[test]
fn test_namespace_class_inside() {
    // Test instantiation inside the namespace
    assert_eq!(
        eval(
            r#"
            namespace Models {
                export class Person {
                    name: string;
                    constructor(name: string) {
                        this.name = name;
                    }
                }
                const p = new Person("Inside");
                export const testName = p.name;
            }
            Models.testName
        "#
        ),
        JsValue::from("Inside")
    );
}

#[test]
fn test_namespace_class_has_prototype() {
    // Check if the exported class has a prototype
    assert_eq!(
        eval(
            r#"
            namespace Models {
                export class Person {
                    name: string;
                    constructor(name: string) {
                        this.name = name;
                    }
                }
            }
            Models.Person.prototype !== undefined
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_namespace_class_call() {
    // Test with regular function first (should work)
    assert_eq!(
        eval(
            r#"
            namespace Models {
                export function create(): string {
                    return "created";
                }
            }
            Models.create()
        "#
        ),
        JsValue::from("created")
    );
}

#[test]
fn test_namespace_class_simple() {
    // Test simple class instantiation
    assert_eq!(
        eval(
            r#"
            namespace Models {
                export class Simple {
                    constructor() {
                    }
                }
            }
            const p = new Models.Simple();
            p !== undefined
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_with_class() {
    // Test that a plain object with a class works
    assert_eq!(
        eval(
            r#"
            class Simple {
                constructor() {
                }
            }
            const Models = { Simple };
            const p = new Models.Simple();
            p !== undefined
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_class_extracted() {
    // Test extracting the class and instantiating
    assert_eq!(
        eval(
            r#"
            class Simple {
                constructor() {
                }
            }
            const Models = { Simple };
            const Ctor = Models.Simple;
            const p = new Ctor();
            p !== undefined
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_function_via_member_new() {
    // Test that a constructor function through object member works with new
    assert_eq!(
        eval(
            r#"
            function Simple() {
                this.value = 42;
            }
            const Models = { Simple };
            const p = new Models.Simple();
            p.value
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_object_function_explicit_new() {
    // Test with explicit assignment instead of shorthand
    assert_eq!(
        eval(
            r#"
            function Simple() {
                this.value = 42;
            }
            const Models = {};
            Models.Simple = Simple;
            const p = new Models.Simple();
            p.value
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_object_function_identity() {
    // Test that Models.Simple === Simple
    assert_eq!(
        eval(
            r#"
            function Simple() {
                this.value = 42;
            }
            const Models = {};
            Models.Simple = Simple;
            Models.Simple === Simple
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_object_function_is_callable() {
    // Test that typeof Models.Simple is "function"
    assert_eq!(
        eval(
            r#"
            function Simple() {
                this.value = 42;
            }
            const Models = {};
            Models.Simple = Simple;
            typeof Models.Simple
        "#
        ),
        JsValue::from("function")
    );
}

#[test]
fn test_namespace_class() {
    assert_eq!(
        eval(
            r#"
            namespace Models {
                export class Person {
                    name: string;
                    constructor(name: string) {
                        this.name = name;
                    }
                }
            }
            const p = new Models.Person("Alice");
            p.name
        "#
        ),
        JsValue::from("Alice")
    );
}

#[test]
fn test_namespace_merged() {
    // Same namespace declared multiple times should merge
    assert_eq!(
        eval(
            r#"
            namespace NS {
                export const a = 1;
            }
            namespace NS {
                export const b = 2;
            }
            NS.a + NS.b
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_namespace_exported_const() {
    // Test that exported const in namespace is accessible
    assert_eq!(
        eval(
            r#"
            namespace Geometry {
                export const PI: number = 3.14159;
            }
            Geometry.PI
        "#
        ),
        JsValue::Number(3.14159)
    );
}

#[test]
fn test_export_namespace_const() {
    // Test that export namespace makes the namespace object exportable
    assert_eq!(
        eval(
            r#"
            export namespace Geometry {
                export const PI: number = 3.14159;
            }
            Geometry.PI
        "#
        ),
        JsValue::Number(3.14159)
    );
}
