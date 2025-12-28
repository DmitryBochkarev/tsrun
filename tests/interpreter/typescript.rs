//! Comprehensive TypeScript-specific tests
//!
//! This module tests TypeScript-specific syntax and features to ensure
//! the runtime correctly parses and executes valid TypeScript code.
//! Types are parsed but stripped at runtime (not type-checked).

use super::eval;
use tsrun::JsValue;

// ============================================================================
// Type Annotations - Basic Types
// ============================================================================

#[test]
fn test_type_annotation_number() {
    assert_eq!(
        eval(
            r#"
            let x: number = 42;
            x
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_type_annotation_string() {
    assert_eq!(
        eval(
            r#"
            let s: string = "hello";
            s
        "#
        ),
        JsValue::from("hello")
    );
}

#[test]
fn test_type_annotation_boolean() {
    assert_eq!(
        eval(
            r#"
            let b: boolean = true;
            b
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_type_annotation_any() {
    assert_eq!(
        eval(
            r#"
            let a: any = 123;
            a = "now a string";
            a
        "#
        ),
        JsValue::from("now a string")
    );
}

#[test]
fn test_type_annotation_unknown() {
    assert_eq!(
        eval(
            r#"
            let u: unknown = 42;
            u
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_type_annotation_void() {
    assert_eq!(
        eval(
            r#"
            function logMessage(msg: string): void {
                // Side effect only
            }
            logMessage("hello");
            undefined
        "#
        ),
        JsValue::Undefined
    );
}

#[test]
fn test_type_annotation_null() {
    assert_eq!(
        eval(
            r#"
            let n: null = null;
            n
        "#
        ),
        JsValue::Null
    );
}

#[test]
fn test_type_annotation_undefined() {
    assert_eq!(
        eval(
            r#"
            let u: undefined = undefined;
            u
        "#
        ),
        JsValue::Undefined
    );
}

// ============================================================================
// Type Annotations - Arrays and Objects
// ============================================================================

#[test]
fn test_type_annotation_array() {
    assert_eq!(
        eval(
            r#"
            let arr: number[] = [1, 2, 3];
            arr.length
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_type_annotation_array_generic() {
    assert_eq!(
        eval(
            r#"
            let arr: Array<string> = ["a", "b", "c"];
            arr.join("-")
        "#
        ),
        JsValue::from("a-b-c")
    );
}

#[test]
fn test_type_annotation_object() {
    assert_eq!(
        eval(
            r#"
            let obj: { name: string; age: number } = { name: "Alice", age: 30 };
            obj.name + " is " + obj.age
        "#
        ),
        JsValue::from("Alice is 30")
    );
}

#[test]
fn test_type_annotation_tuple() {
    assert_eq!(
        eval(
            r#"
            let tuple: [string, number] = ["hello", 42];
            tuple[0] + tuple[1]
        "#
        ),
        JsValue::from("hello42")
    );
}

// ============================================================================
// Type Annotations - Functions
// ============================================================================

#[test]
fn test_function_parameter_types() {
    assert_eq!(
        eval(
            r#"
            function add(a: number, b: number): number {
                return a + b;
            }
            add(10, 20)
        "#
        ),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_arrow_function_types() {
    assert_eq!(
        eval(
            r#"
            const multiply = (x: number, y: number): number => x * y;
            multiply(6, 7)
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_optional_parameters() {
    assert_eq!(
        eval(
            r#"
            function greet(name: string, greeting?: string): string {
                return (greeting || "Hello") + ", " + name;
            }
            greet("World")
        "#
        ),
        JsValue::from("Hello, World")
    );
}

#[test]
fn test_default_parameters_with_types() {
    assert_eq!(
        eval(
            r#"
            function greet(name: string, greeting: string = "Hi"): string {
                return greeting + ", " + name;
            }
            greet("TypeScript")
        "#
        ),
        JsValue::from("Hi, TypeScript")
    );
}

#[test]
fn test_rest_parameters_with_types() {
    assert_eq!(
        eval(
            r#"
            function sum(...numbers: number[]): number {
                return numbers.reduce((a, b) => a + b, 0);
            }
            sum(1, 2, 3, 4, 5)
        "#
        ),
        JsValue::Number(15.0)
    );
}

#[test]
fn test_function_type_expression() {
    assert_eq!(
        eval(
            r#"
            let fn: (x: number) => number = (x) => x * 2;
            fn(21)
        "#
        ),
        JsValue::Number(42.0)
    );
}

// ============================================================================
// Type Assertions
// ============================================================================

#[test]
fn test_as_assertion() {
    assert_eq!(
        eval(
            r#"
            let value: any = "hello";
            let len = (value as string).length;
            len
        "#
        ),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_angle_bracket_assertion() {
    assert_eq!(
        eval(
            r#"
            let value: any = 42;
            let num = <number>value;
            num + 8
        "#
        ),
        JsValue::Number(50.0)
    );
}

#[test]
fn test_non_null_assertion() {
    assert_eq!(
        eval(
            r#"
            let value: string | null = "hello";
            value!.length
        "#
        ),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_const_assertion() {
    assert_eq!(
        eval(
            r#"
            const arr = [1, 2, 3] as const;
            arr[0] + arr[1] + arr[2]
        "#
        ),
        JsValue::Number(6.0)
    );
}

// ============================================================================
// Interfaces
// ============================================================================

#[test]
fn test_interface_basic() {
    assert_eq!(
        eval(
            r#"
            interface Person {
                name: string;
                age: number;
            }

            let person: Person = { name: "Bob", age: 25 };
            person.name
        "#
        ),
        JsValue::from("Bob")
    );
}

#[test]
fn test_interface_optional_properties() {
    assert_eq!(
        eval(
            r#"
            interface Config {
                host: string;
                port?: number;
            }

            let config: Config = { host: "localhost" };
            config.host
        "#
        ),
        JsValue::from("localhost")
    );
}

#[test]
fn test_interface_readonly() {
    assert_eq!(
        eval(
            r#"
            interface Point {
                readonly x: number;
                readonly y: number;
            }

            let p: Point = { x: 10, y: 20 };
            p.x + p.y
        "#
        ),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_interface_method_signature() {
    assert_eq!(
        eval(
            r#"
            interface Calculator {
                add(a: number, b: number): number;
            }

            let calc: Calculator = {
                add(a: number, b: number): number {
                    return a + b;
                }
            };
            calc.add(5, 3)
        "#
        ),
        JsValue::Number(8.0)
    );
}

#[test]
fn test_interface_extends() {
    assert_eq!(
        eval(
            r#"
            interface Animal {
                name: string;
            }

            interface Dog extends Animal {
                breed: string;
            }

            let dog: Dog = { name: "Rex", breed: "German Shepherd" };
            dog.name + " - " + dog.breed
        "#
        ),
        JsValue::from("Rex - German Shepherd")
    );
}

#[test]
fn test_interface_index_signature() {
    assert_eq!(
        eval(
            r#"
            interface StringMap {
                [key: string]: string;
            }

            let map: StringMap = {};
            map["hello"] = "world";
            map["hello"]
        "#
        ),
        JsValue::from("world")
    );
}

// ============================================================================
// Type Aliases
// ============================================================================

#[test]
fn test_type_alias_simple() {
    assert_eq!(
        eval(
            r#"
            type ID = number;
            let userId: ID = 12345;
            userId
        "#
        ),
        JsValue::Number(12345.0)
    );
}

#[test]
fn test_type_alias_union() {
    assert_eq!(
        eval(
            r#"
            type StringOrNumber = string | number;
            let value: StringOrNumber = "hello";
            value
        "#
        ),
        JsValue::from("hello")
    );
}

#[test]
fn test_type_alias_intersection() {
    assert_eq!(
        eval(
            r#"
            type Named = { name: string };
            type Aged = { age: number };
            type Person = Named & Aged;

            let person: Person = { name: "Alice", age: 30 };
            person.age
        "#
        ),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_type_alias_object() {
    assert_eq!(
        eval(
            r#"
            type Point = {
                x: number;
                y: number;
            };

            let p: Point = { x: 5, y: 10 };
            p.x * p.y
        "#
        ),
        JsValue::Number(50.0)
    );
}

// ============================================================================
// Generics
// ============================================================================

#[test]
fn test_generic_function() {
    assert_eq!(
        eval(
            r#"
            function identity<T>(arg: T): T {
                return arg;
            }
            identity<number>(42)
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_generic_function_inference() {
    assert_eq!(
        eval(
            r#"
            function identity<T>(arg: T): T {
                return arg;
            }
            identity("hello")
        "#
        ),
        JsValue::from("hello")
    );
}

#[test]
fn test_generic_interface() {
    assert_eq!(
        eval(
            r#"
            interface Container<T> {
                value: T;
            }

            let box: Container<number> = { value: 42 };
            box.value
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_generic_constraint() {
    assert_eq!(
        eval(
            r#"
            interface HasLength {
                length: number;
            }

            function getLength<T extends HasLength>(arg: T): number {
                return arg.length;
            }
            getLength("hello")
        "#
        ),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_generic_multiple_types() {
    assert_eq!(
        eval(
            r#"
            function pair<T, U>(first: T, second: U): [T, U] {
                return [first, second];
            }
            let p = pair<string, number>("age", 25);
            p[0] + ": " + p[1]
        "#
        ),
        JsValue::from("age: 25")
    );
}

// ============================================================================
// Classes with TypeScript Features
// ============================================================================

#[test]
fn test_class_property_types() {
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

            let p = new Point(3, 4);
            p.x + p.y
        "#
        ),
        JsValue::Number(7.0)
    );
}

#[test]
fn test_class_access_modifiers() {
    assert_eq!(
        eval(
            r#"
            class Person {
                public name: string;
                private age: number;
                protected status: string;

                constructor(name: string, age: number) {
                    this.name = name;
                    this.age = age;
                    this.status = "active";
                }

                getAge(): number {
                    return this.age;
                }
            }

            let p = new Person("Alice", 30);
            p.getAge()
        "#
        ),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_class_readonly_property() {
    assert_eq!(
        eval(
            r#"
            class Config {
                readonly apiUrl: string;

                constructor(url: string) {
                    this.apiUrl = url;
                }
            }

            let config = new Config("https://api.example.com");
            config.apiUrl
        "#
        ),
        JsValue::from("https://api.example.com")
    );
}

#[test]
fn test_class_parameter_properties() {
    assert_eq!(
        eval(
            r#"
            class Point {
                constructor(public x: number, public y: number) {}
            }

            let p = new Point(10, 20);
            p.x + p.y
        "#
        ),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_class_static_types() {
    assert_eq!(
        eval(
            r#"
            class MathUtils {
                static PI: number = 3.14159;

                static double(x: number): number {
                    return x * 2;
                }
            }

            MathUtils.double(21) + MathUtils.PI
        "#
        ),
        JsValue::Number(45.14159)
    );
}

#[test]
fn test_class_implements_interface() {
    assert_eq!(
        eval(
            r#"
            interface Greetable {
                greet(): string;
            }

            class Person implements Greetable {
                constructor(public name: string) {}

                greet(): string {
                    return "Hello, " + this.name;
                }
            }

            let p = new Person("World");
            p.greet()
        "#
        ),
        JsValue::from("Hello, World")
    );
}

#[test]
fn test_class_extends_with_types() {
    assert_eq!(
        eval(
            r#"
            class Animal {
                constructor(public name: string) {}

                speak(): string {
                    return this.name + " makes a sound";
                }
            }

            class Dog extends Animal {
                constructor(name: string, public breed: string) {
                    super(name);
                }

                speak(): string {
                    return this.name + " barks";
                }
            }

            let dog = new Dog("Rex", "German Shepherd");
            dog.speak()
        "#
        ),
        JsValue::from("Rex barks")
    );
}

#[test]
fn test_abstract_class() {
    assert_eq!(
        eval(
            r#"
            abstract class Shape {
                abstract getArea(): number;

                describe(): string {
                    return "This shape has area " + this.getArea();
                }
            }

            class Rectangle extends Shape {
                constructor(public width: number, public height: number) {
                    super();
                }

                getArea(): number {
                    return this.width * this.height;
                }
            }

            let rect = new Rectangle(5, 10);
            rect.getArea()
        "#
        ),
        JsValue::Number(50.0)
    );
}

// ============================================================================
// Enums (TypeScript-specific)
// ============================================================================

#[test]
fn test_numeric_enum() {
    assert_eq!(
        eval(
            r#"
            enum Direction {
                Up,
                Down,
                Left,
                Right
            }
            Direction.Down
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_string_enum() {
    assert_eq!(
        eval(
            r#"
            enum Color {
                Red = "RED",
                Green = "GREEN",
                Blue = "BLUE"
            }
            Color.Green
        "#
        ),
        JsValue::from("GREEN")
    );
}

#[test]
fn test_enum_with_explicit_values() {
    assert_eq!(
        eval(
            r#"
            enum HttpStatus {
                OK = 200,
                NotFound = 404,
                InternalError = 500
            }
            HttpStatus.NotFound
        "#
        ),
        JsValue::Number(404.0)
    );
}

#[test]
fn test_const_enum() {
    assert_eq!(
        eval(
            r#"
            const enum Size {
                Small = 1,
                Medium = 2,
                Large = 3
            }
            Size.Medium
        "#
        ),
        JsValue::Number(2.0)
    );
}

// ============================================================================
// Namespaces (TypeScript-specific)
// ============================================================================

#[test]
fn test_namespace_basic() {
    assert_eq!(
        eval(
            r#"
            namespace MathUtils {
                export function add(a: number, b: number): number {
                    return a + b;
                }

                export const PI: number = 3.14159;
            }

            MathUtils.add(10, 20) + MathUtils.PI
        "#
        ),
        JsValue::Number(33.14159)
    );
}

#[test]
fn test_nested_namespace() {
    assert_eq!(
        eval(
            r#"
            namespace Outer {
                export namespace Inner {
                    export function getValue(): number {
                        return 42;
                    }
                }
            }

            Outer.Inner.getValue()
        "#
        ),
        JsValue::Number(42.0)
    );
}

// ============================================================================
// Advanced TypeScript Features
// ============================================================================

#[test]
fn test_literal_types() {
    assert_eq!(
        eval(
            r#"
            type YesNo = "yes" | "no";
            let answer: YesNo = "yes";
            answer
        "#
        ),
        JsValue::from("yes")
    );
}

#[test]
fn test_template_literal_types() {
    // Template literal types are a compile-time feature
    // At runtime, we just use the string
    assert_eq!(
        eval(
            r#"
            type Greeting = `Hello ${string}`;
            let msg: Greeting = "Hello World";
            msg
        "#
        ),
        JsValue::from("Hello World")
    );
}

#[test]
fn test_typeof_in_type_position() {
    assert_eq!(
        eval(
            r#"
            let original = { x: 10, y: 20 };
            let copy: typeof original = { x: 30, y: 40 };
            copy.x + copy.y
        "#
        ),
        JsValue::Number(70.0)
    );
}

#[test]
fn test_keyof() {
    assert_eq!(
        eval(
            r#"
            interface Person {
                name: string;
                age: number;
            }

            type PersonKeys = keyof Person;
            // At runtime, keyof is stripped - we just work with values
            let key: PersonKeys = "name";
            key
        "#
        ),
        JsValue::from("name")
    );
}

#[test]
fn test_mapped_types() {
    assert_eq!(
        eval(
            r#"
            type Readonly<T> = {
                readonly [P in keyof T]: T[P];
            };

            interface Point {
                x: number;
                y: number;
            }

            let p: Readonly<Point> = { x: 5, y: 10 };
            p.x + p.y
        "#
        ),
        JsValue::Number(15.0)
    );
}

#[test]
fn test_conditional_types() {
    assert_eq!(
        eval(
            r#"
            type IsString<T> = T extends string ? true : false;

            // At runtime, conditional types are stripped
            let result: IsString<string> = true;
            result
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_infer_rest_params() {
    // Test parsing rest parameters with array type in function type
    assert_eq!(
        eval(
            r#"
            type Fn = (...args: any[]) => void;
            42
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_infer_simple_func_return() {
    // Test simple function type return
    assert_eq!(
        eval(
            r#"
            type Fn = () => number;
            42
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_infer_func_return_infer() {
    // Test function type with infer in return
    assert_eq!(
        eval(
            r#"
            type Fn = () => infer R;
            42
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_infer_conditional_with_func() {
    // Test conditional type with function type on the left of extends
    assert_eq!(
        eval(
            r#"
            type ReturnType<T> = T extends () => infer R ? R : never;
            42
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_infer_keyword() {
    assert_eq!(
        eval(
            r#"
            type ReturnType<T> = T extends (...args: any[]) => infer R ? R : never;

            function getNumber(): number {
                return 42;
            }

            // At runtime, type annotations are stripped
            let result: ReturnType<typeof getNumber> = getNumber();
            result
        "#
        ),
        JsValue::Number(42.0)
    );
}

// ============================================================================
// Type Guards and Narrowing
// ============================================================================

#[test]
fn test_typeof_guard() {
    assert_eq!(
        eval(
            r#"
            function processValue(value: string | number): string {
                if (typeof value === "string") {
                    return value.toUpperCase();
                } else {
                    return value.toString();
                }
            }
            processValue("hello")
        "#
        ),
        JsValue::from("HELLO")
    );
}

#[test]
fn test_instanceof_guard() {
    assert_eq!(
        eval(
            r#"
            class Dog {
                bark(): string { return "woof"; }
            }

            class Cat {
                meow(): string { return "meow"; }
            }

            function makeSound(animal: Dog | Cat): string {
                if (animal instanceof Dog) {
                    return animal.bark();
                } else {
                    return animal.meow();
                }
            }

            makeSound(new Dog())
        "#
        ),
        JsValue::from("woof")
    );
}

#[test]
fn test_in_operator_guard() {
    assert_eq!(
        eval(
            r#"
            interface Fish {
                swim: () => void;
            }

            interface Bird {
                fly: () => void;
            }

            function move(animal: Fish | Bird): string {
                if ("swim" in animal) {
                    return "swimming";
                } else {
                    return "flying";
                }
            }

            let fish: Fish = { swim: () => {} };
            move(fish)
        "#
        ),
        JsValue::from("swimming")
    );
}

#[test]
fn test_user_defined_type_guard() {
    assert_eq!(
        eval(
            r#"
            interface Cat {
                meow(): string;
            }

            function isCat(pet: any): pet is Cat {
                return pet && typeof pet.meow === "function";
            }

            let animal: any = {
                meow(): string { return "meow"; }
            };

            if (isCat(animal)) {
                animal.meow()
            } else {
                "not a cat"
            }
        "#
        ),
        JsValue::from("meow")
    );
}

// ============================================================================
// Async/Await with Types
// ============================================================================

#[test]
fn test_async_function_return_type() {
    assert_eq!(
        eval(
            r#"
            let result: number = 0;

            async function fetchData(): Promise<number> {
                return 42;
            }

            async function main(): Promise<number> {
                const value: number = await fetchData();
                return value;
            }

            main().then(function(x: number) {
                result = x;
            });
            result
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_async_arrow_with_types() {
    assert_eq!(
        eval(
            r#"
            let result: string = "";

            const getData = async (): Promise<string> => {
                return "hello";
            };

            getData().then(function(x: string) {
                result = x;
            });
            result
        "#
        ),
        JsValue::from("hello")
    );
}

// ============================================================================
// Decorators with Types
// ============================================================================

#[test]
fn test_class_decorator_with_types() {
    assert_eq!(
        eval(
            r#"
            function sealed(constructor: Function): void {
                Object.seal(constructor);
                Object.seal(constructor.prototype);
            }

            @sealed
            class Greeter {
                greeting: string;

                constructor(message: string) {
                    this.greeting = message;
                }

                greet(): string {
                    return "Hello, " + this.greeting;
                }
            }

            let g = new Greeter("world");
            g.greet()
        "#
        ),
        JsValue::from("Hello, world")
    );
}

#[test]
fn test_method_decorator_with_types() {
    // Uses modern decorator protocol (target, context) not legacy (target, key, descriptor)
    assert_eq!(
        eval(
            r#"
            function log(target: any, context: any): any {
                return function(...args: any[]): any {
                    const result = target.apply(this, args);
                    return result;
                };
            }

            class Calculator {
                @log
                add(a: number, b: number): number {
                    return a + b;
                }
            }

            let calc: Calculator = new Calculator();
            calc.add(5, 3)
        "#
        ),
        JsValue::Number(8.0)
    );
}

// ============================================================================
// Complex TypeScript Patterns
// ============================================================================

#[test]
fn test_discriminated_union() {
    assert_eq!(
        eval(
            r#"
            interface Square {
                kind: "square";
                size: number;
            }

            interface Rectangle {
                kind: "rectangle";
                width: number;
                height: number;
            }

            type Shape = Square | Rectangle;

            function area(s: Shape): number {
                switch (s.kind) {
                    case "square":
                        return s.size * s.size;
                    case "rectangle":
                        return s.width * s.height;
                }
            }

            let square: Square = { kind: "square", size: 5 };
            area(square)
        "#
        ),
        JsValue::Number(25.0)
    );
}

#[test]
fn test_generic_class() {
    assert_eq!(
        eval(
            r#"
            class Stack<T> {
                private items: T[] = [];

                push(item: T): void {
                    this.items.push(item);
                }

                pop(): T | undefined {
                    return this.items.pop();
                }

                peek(): T | undefined {
                    return this.items[this.items.length - 1];
                }

                size(): number {
                    return this.items.length;
                }
            }

            let stack = new Stack<number>();
            stack.push(1);
            stack.push(2);
            stack.push(3);
            stack.pop()
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_utility_types_partial() {
    assert_eq!(
        eval(
            r#"
            interface Todo {
                title: string;
                description: string;
                completed: boolean;
            }

            // Partial<T> makes all properties optional - this is a type-level construct
            function updateTodo(todo: Todo, update: Partial<Todo>): Todo {
                return { ...todo, ...update };
            }

            let todo: Todo = { title: "Learn TS", description: "Study TypeScript", completed: false };
            let updated = updateTodo(todo, { completed: true });
            updated.completed
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_builder_pattern_with_types() {
    assert_eq!(
        eval(
            r#"
            class RequestBuilder {
                private url: string = "";
                private method: string = "GET";
                private headers: { [key: string]: string } = {};

                setUrl(url: string): RequestBuilder {
                    this.url = url;
                    return this;
                }

                setMethod(method: string): RequestBuilder {
                    this.method = method;
                    return this;
                }

                addHeader(key: string, value: string): RequestBuilder {
                    this.headers[key] = value;
                    return this;
                }

                getUrl(): string {
                    return this.url;
                }

                getMethod(): string {
                    return this.method;
                }
            }

            let request = new RequestBuilder()
                .setUrl("https://api.example.com")
                .setMethod("POST")
                .addHeader("Content-Type", "application/json");

            request.getMethod() + " " + request.getUrl()
        "#
        ),
        JsValue::from("POST https://api.example.com")
    );
}

#[test]
fn test_factory_pattern_with_types() {
    assert_eq!(
        eval(
            r#"
            interface Product {
                name: string;
                price: number;
            }

            interface ProductFactory {
                createProduct(name: string, price: number): Product;
            }

            class DefaultProductFactory implements ProductFactory {
                createProduct(name: string, price: number): Product {
                    return { name, price };
                }
            }

            let factory: ProductFactory = new DefaultProductFactory();
            let product = factory.createProduct("Widget", 29.99);
            product.name + ": $" + product.price
        "#
        ),
        JsValue::from("Widget: $29.99")
    );
}

#[test]
fn test_mixin_simple_class_return() {
    // Simple test: function returning a class expression
    assert_eq!(
        eval(
            r#"
            function createClass() {
                return class {
                    getValue(): number { return 42; }
                };
            }
            const MyClass = createClass();
            let obj = new MyClass();
            obj.getValue()
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_mixin_class_extends() {
    // Test: function returning a class that extends a base
    assert_eq!(
        eval(
            r#"
            class Base {
                baseValue(): number { return 10; }
            }
            function createExtended(B: any) {
                return class extends B {
                    extendedValue(): number { return 20; }
                };
            }
            const Extended = createExtended(Base);
            let obj = new Extended();
            obj.baseValue()
        "#
        ),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_mixin_debug_direct() {
    // Test: basic User with param properties works
    assert_eq!(
        eval(
            r#"
            class User {
                constructor(public name: string) {}
            }
            let user = new User("Alice");
            user.name
        "#
        ),
        JsValue::from("Alice")
    );
}

#[test]
fn test_mixin_debug_simple_extend() {
    // Test: named class extends User
    assert_eq!(
        eval(
            r#"
            class User {
                constructor(public name: string) {}
            }
            class ExtUser extends User {
                constructor(name: string) {
                    super(name);
                }
            }
            let user = new ExtUser("Alice");
            user.name
        "#
        ),
        JsValue::from("Alice")
    );
}

#[test]
fn test_mixin_debug_implicit_super() {
    // Test: implicit super call forwards arguments
    assert_eq!(
        eval(
            r#"
            class Base {
                constructor(public value: number) {}
            }
            class Sub extends Base {
            }
            let obj = new Sub(42);
            obj.value
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_mixin_with_param_properties() {
    // Test: class with parameter properties used with mixin
    assert_eq!(
        eval(
            r#"
            class User {
                constructor(public name: string) {}
            }
            function wrap(B: any) {
                return class extends B {};
            }
            const WrappedUser = wrap(User);
            let user = new WrappedUser("Alice");
            user.name
        "#
        ),
        JsValue::from("Alice")
    );
}

#[test]
fn test_mixin_pattern() {
    assert_eq!(
        eval(
            r#"
            // Mixin: adds timestamping capability
            function Timestamped<TBase extends new (...args: any[]) => any>(Base: TBase) {
                return class extends Base {
                    timestamp: Date = new Date();
                };
            }

            class User {
                constructor(public name: string) {}
            }

            const TimestampedUser = Timestamped(User);
            let user = new TimestampedUser("Alice");
            user.name
        "#
        ),
        JsValue::from("Alice")
    );
}

// ============================================================================
// Error Handling with Types
// ============================================================================

#[test]
fn test_typed_error_handling() {
    assert_eq!(
        eval(
            r#"
            class ValidationError extends Error {
                constructor(public field: string, message: string) {
                    super(message);
                    this.name = "ValidationError";
                }
            }

            function validateAge(age: number): void {
                if (age < 0) {
                    throw new ValidationError("age", "Age cannot be negative");
                }
            }

            try {
                validateAge(-5);
            } catch (e) {
                if (e instanceof ValidationError) {
                    e.field + ": " + e.message
                } else {
                    "Unknown error"
                }
            }
        "#
        ),
        JsValue::from("age: Age cannot be negative")
    );
}

// ============================================================================
// Real-world TypeScript Patterns
// ============================================================================

#[test]
fn test_event_emitter_pattern() {
    assert_eq!(
        eval(
            r#"
            type EventHandler<T> = (data: T) => void;

            class EventEmitter<Events extends Record<string, any>> {
                private handlers: { [K in keyof Events]?: EventHandler<Events[K]>[] } = {};

                on<K extends keyof Events>(event: K, handler: EventHandler<Events[K]>): void {
                    if (!this.handlers[event]) {
                        this.handlers[event] = [];
                    }
                    this.handlers[event]!.push(handler);
                }

                emit<K extends keyof Events>(event: K, data: Events[K]): void {
                    const eventHandlers = this.handlers[event];
                    if (eventHandlers) {
                        for (const handler of eventHandlers) {
                            handler(data);
                        }
                    }
                }
            }

            interface MyEvents {
                message: string;
                count: number;
            }

            let result: string = "";
            const emitter = new EventEmitter<MyEvents>();

            emitter.on("message", (msg: string) => {
                result = msg;
            });

            emitter.emit("message", "Hello TypeScript!");
            result
        "#
        ),
        JsValue::from("Hello TypeScript!")
    );
}

#[test]
fn test_repository_pattern() {
    assert_eq!(
        eval(
            r#"
            interface Entity {
                id: number;
            }

            interface Repository<T extends Entity> {
                findById(id: number): T | undefined;
                save(entity: T): void;
                findAll(): T[];
            }

            interface User extends Entity {
                name: string;
                email: string;
            }

            class InMemoryUserRepository implements Repository<User> {
                private users: User[] = [];

                findById(id: number): User | undefined {
                    return this.users.find((u: User) => u.id === id);
                }

                save(user: User): void {
                    const existing = this.findById(user.id);
                    if (existing) {
                        const index = this.users.indexOf(existing);
                        this.users[index] = user;
                    } else {
                        this.users.push(user);
                    }
                }

                findAll(): User[] {
                    return this.users;
                }
            }

            let repo: Repository<User> = new InMemoryUserRepository();
            repo.save({ id: 1, name: "Alice", email: "alice@example.com" });
            repo.save({ id: 2, name: "Bob", email: "bob@example.com" });

            let user = repo.findById(1);
            user ? user.name : "not found"
        "#
        ),
        JsValue::from("Alice")
    );
}

#[test]
fn test_state_machine_pattern() {
    assert_eq!(
        eval(
            r#"
            type State = "idle" | "loading" | "success" | "error";

            interface StateMachine {
                state: State;
                data: any;
                error: string | null;
            }

            function createMachine(): StateMachine {
                return { state: "idle", data: null, error: null };
            }

            function transition(machine: StateMachine, action: string, payload?: any): StateMachine {
                switch (machine.state) {
                    case "idle":
                        if (action === "FETCH") {
                            return { ...machine, state: "loading" };
                        }
                        break;
                    case "loading":
                        if (action === "SUCCESS") {
                            return { ...machine, state: "success", data: payload };
                        }
                        if (action === "ERROR") {
                            return { ...machine, state: "error", error: payload };
                        }
                        break;
                }
                return machine;
            }

            let machine = createMachine();
            machine = transition(machine, "FETCH");
            machine = transition(machine, "SUCCESS", { items: [1, 2, 3] });
            machine.state
        "#
        ),
        JsValue::from("success")
    );
}

// ============================================================================
// Module-like Patterns (without actual ES modules)
// ============================================================================

#[test]
fn test_revealing_module_pattern() {
    assert_eq!(
        eval(
            r#"
            const Calculator = (function() {
                // Private members
                let result: number = 0;

                function add(x: number): void {
                    result += x;
                }

                function subtract(x: number): void {
                    result -= x;
                }

                function getResult(): number {
                    return result;
                }

                function reset(): void {
                    result = 0;
                }

                // Public API
                return {
                    add,
                    subtract,
                    getResult,
                    reset
                };
            })();

            Calculator.add(10);
            Calculator.add(5);
            Calculator.subtract(3);
            Calculator.getResult()
        "#
        ),
        JsValue::Number(12.0)
    );
}

// ============================================================================
// Complex Data Transformations with Types
// ============================================================================

#[test]
fn test_data_transform_simple_map() {
    // Simple test of map with arrow function and type annotations
    assert_eq!(
        eval(
            r#"
            let arr: number[] = [1, 2, 3];
            let result: number[] = arr.map((x: number) => x * 2);
            result[1]
        "#
        ),
        JsValue::Number(4.0)
    );
}

#[test]
fn test_data_transform_object_map() {
    // Map that transforms objects
    assert_eq!(
        eval(
            r#"
            interface User {
                name: string;
            }
            let users: User[] = [{ name: "Alice" }, { name: "Bob" }];
            let names: string[] = users.map((u: User) => u.name);
            names[0]
        "#
        ),
        JsValue::from("Alice")
    );
}

#[test]
fn test_data_transform_with_function_call() {
    // Test map with arrow that calls another function
    assert_eq!(
        eval(
            r#"
            interface User {
                name: string;
            }
            function transform(u: User): string {
                return u.name.toUpperCase();
            }
            let users: User[] = [{ name: "Alice" }];
            let result: string[] = users.map((u: User) => transform(u));
            result[0]
        "#
        ),
        JsValue::from("ALICE")
    );
}

#[test]
fn test_data_transform_closure() {
    // Test map with closure capturing outer variable
    assert_eq!(
        eval(
            r#"
            interface User {
                age: number;
            }
            let year: number = 2024;
            let users: User[] = [{ age: 30 }];
            let result: number[] = users.map((u: User) => year - u.age);
            result[0]
        "#
        ),
        JsValue::Number(1994.0)
    );
}

#[test]
fn test_data_transform_two_args() {
    // Test closure capturing function parameter and passing to another function
    assert_eq!(
        eval(
            r#"
            interface User {
                name: string;
            }
            function transform(u: User, prefix: string): string {
                return prefix + u.name;
            }
            function process(users: User[], prefix: string): string[] {
                return users.map((u: User) => transform(u, prefix));
            }
            let users: User[] = [{ name: "Alice" }];
            let result = process(users, "Hello ");
            result[0]
        "#
        ),
        JsValue::from("Hello Alice")
    );
}

#[test]
fn test_data_transform_return_object_js() {
    // Test transform that returns an object - without type annotations to verify it's not TS-specific
    assert_eq!(
        eval(
            r#"
            function transform(input) {
                return { doubled: input.value * 2 };
            }
            function process(inputs) {
                return inputs.map(function(i) { return transform(i); });
            }
            let inputs = [{ value: 5 }];
            let result = process(inputs);
            result[0].doubled
        "#
        ),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_data_transform_return_object() {
    // Test transform that returns an object
    assert_eq!(
        eval(
            r#"
            interface Input {
                value: number;
            }
            interface Output {
                doubled: number;
            }
            function transform(input: Input): Output {
                return { doubled: input.value * 2 };
            }
            function process(inputs: Input[]): Output[] {
                return inputs.map((i: Input) => transform(i));
            }
            let inputs: Input[] = [{ value: 5 }];
            let result = process(inputs);
            result[0].doubled
        "#
        ),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_data_transformation_pipeline() {
    assert_eq!(
        eval(
            r#"
            interface RawUser {
                firstName: string;
                lastName: string;
                birthYear: number;
            }

            interface ProcessedUser {
                fullName: string;
                age: number;
            }

            function transformUser(raw: RawUser, currentYear: number): ProcessedUser {
                return {
                    fullName: raw.firstName + " " + raw.lastName,
                    age: currentYear - raw.birthYear
                };
            }

            function processUsers(users: RawUser[], currentYear: number): ProcessedUser[] {
                return users.map((u: RawUser) => transformUser(u, currentYear));
            }

            let rawUsers: RawUser[] = [
                { firstName: "Alice", lastName: "Smith", birthYear: 1990 },
                { firstName: "Bob", lastName: "Jones", birthYear: 1985 }
            ];

            let processed = processUsers(rawUsers, 2024);
            processed[0].fullName + " is " + processed[0].age
        "#
        ),
        JsValue::from("Alice Smith is 34")
    );
}

#[test]
fn test_recursive_type() {
    assert_eq!(
        eval(
            r#"
            interface TreeNode<T> {
                value: T;
                children: TreeNode<T>[];
            }

            function sumTree(node: TreeNode<number>): number {
                let sum = node.value;
                for (const child of node.children) {
                    sum += sumTree(child);
                }
                return sum;
            }

            let tree: TreeNode<number> = {
                value: 1,
                children: [
                    { value: 2, children: [] },
                    { value: 3, children: [
                        { value: 4, children: [] }
                    ]}
                ]
            };

            sumTree(tree)
        "#
        ),
        JsValue::Number(10.0)
    );
}
