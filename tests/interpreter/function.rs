//! Function-related tests

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_function() {
    assert_eq!(
        eval("function add(a: number, b: number): number { return a + b; } add(2, 3)"),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_this_binding() {
    // Test that 'this' is properly bound in method calls
    assert_eq!(
        eval(
            "let obj = {x: 42 as number, getX: function(): number { return this.x; }}; obj.getX()"
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_call() {
    assert_eq!(
        eval(
            "function greet(): string { return 'Hello ' + this.name; } greet.call({name: 'World'})"
        ),
        JsValue::from("Hello World")
    );
    assert_eq!(
        eval("function add(a: number, b: number): number { return a + b; } add.call(null, 2, 3)"),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_function_apply() {
    assert_eq!(eval("function greet(): string { return 'Hello ' + this.name; } greet.apply({name: 'World'})"), JsValue::from("Hello World"));
    assert_eq!(
        eval(
            "function add(a: number, b: number): number { return a + b; } add.apply(null, [2, 3])"
        ),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_function_bind() {
    assert_eq!(eval("function greet(): string { return 'Hello ' + this.name; } const boundGreet: Function = greet.bind({name: 'World'}); boundGreet()"), JsValue::from("Hello World"));
    assert_eq!(eval("function add(a: number, b: number): number { return a + b; } const add5: Function = add.bind(null, 5); add5(3)"), JsValue::Number(8.0));
}

#[test]
fn test_arrow_function() {
    assert_eq!(
        eval("const add: (a: number, b: number) => number = (a, b) => a + b; add(2, 3)"),
        JsValue::Number(5.0)
    );
}

// Tests for the `arguments` object
#[test]
fn test_arguments_length() {
    assert_eq!(
        eval("function f(): number { return arguments.length; } f(1, 2, 3)"),
        JsValue::Number(3.0)
    );
    assert_eq!(
        eval("function f(): number { return arguments.length; } f()"),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_arguments_access() {
    assert_eq!(
        eval("function f(): number { return arguments[0]; } f(42)"),
        JsValue::Number(42.0)
    );
    assert_eq!(
        eval("function f(): number { return arguments[1]; } f(1, 2, 3)"),
        JsValue::Number(2.0)
    );
    assert_eq!(
        eval("function f(): string { return arguments[2]; } f('a', 'b', 'c')"),
        JsValue::from("c")
    );
}

#[test]
fn test_arguments_out_of_bounds() {
    assert_eq!(
        eval("function f(): any { return arguments[5]; } f(1, 2)"),
        JsValue::Undefined
    );
}

#[test]
fn test_arguments_with_named_params() {
    // arguments should still contain all args even when named params exist
    assert_eq!(
        eval("function f(a: number, b: number): number { return arguments.length; } f(1, 2, 3, 4)"),
        JsValue::Number(4.0)
    );
    assert_eq!(
        eval("function f(a: number, b: number): number { return arguments[2]; } f(1, 2, 3)"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_arguments_in_nested_function() {
    // Each function has its own arguments object
    assert_eq!(
        eval("function outer(): number { function inner(): number { return arguments[0]; } return inner(42); } outer(99)"),
        JsValue::Number(42.0)
    );
}

// Tests for destructuring in function parameters
#[test]
fn test_destructuring_object_param() {
    assert_eq!(
        eval("function f({ x, y }: { x: number; y: number }): number { return x + y; } f({ x: 1, y: 2 })"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_destructuring_object_param_with_default() {
    assert_eq!(
        eval("function f({ x, y = 10 }: { x: number; y?: number }): number { return x + y; } f({ x: 1 })"),
        JsValue::Number(11.0)
    );
}

#[test]
fn test_destructuring_array_param() {
    assert_eq!(
        eval("function f([a, b]: number[]): number { return a + b; } f([3, 4])"),
        JsValue::Number(7.0)
    );
}

#[test]
fn test_destructuring_array_param_with_rest() {
    assert_eq!(
        eval("function f([first, ...rest]: number[]): number { return rest.length; } f([1, 2, 3, 4])"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_destructuring_nested_param() {
    assert_eq!(
        eval("function f({ person: { name } }: { person: { name: string } }): string { return name; } f({ person: { name: 'John' } })"),
        JsValue::from("John")
    );
}

#[test]
fn test_arrow_destructuring_param() {
    assert_eq!(
        eval("const f: (obj: { x: number }) => number = ({ x }) => x * 2; f({ x: 5 })"),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_arrow_with_return_type_annotation() {
    // Arrow function with typed parameters and return type annotation
    assert_eq!(
        eval(
            r#"
            const filterByCategory = (items: any[], category: string): any[] =>
                items.filter(p => p.category === category);
            const products = [
                { id: 1, category: "X" },
                { id: 2, category: "Y" },
            ];
            filterByCategory(products, "X").length
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_arrow_array_destructuring_param() {
    // Array destructuring pattern in arrow function parameter
    assert_eq!(
        eval(
            r#"
            const arr = [[1, 2], [3, 4]];
            arr.map(([a, b]) => a + b).reduce((sum, x) => sum + x, 0)
        "#
        ),
        JsValue::Number(10.0)
    );
}
