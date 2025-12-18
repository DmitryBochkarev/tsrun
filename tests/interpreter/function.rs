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
fn test_destructuring_object_param_with_rest() {
    // Rest pattern in function parameter
    assert_eq!(
        eval(
            r#"
            function f({ id, ...rest }: { id: number; name: string; age: number }): string {
                return id + "-" + rest.name + "-" + rest.age;
            }
            f({ id: 1, name: "Bob", age: 30 })
        "#
        ),
        JsValue::from("1-Bob-30")
    );
}

#[test]
fn test_arrow_destructuring_param_with_rest() {
    // Rest pattern in arrow function parameter
    assert_eq!(
        eval(
            r#"
            const extract = ({ type, ...data }: { type: string; x: number; y: number }) => data.x + data.y;
            extract({ type: "point", x: 10, y: 20 })
        "#
        ),
        JsValue::Number(30.0)
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

#[test]
fn test_recursive_fibonacci() {
    let result = eval(
        r#"
        function fibRecursive(n: number): number {
            if (n <= 1) return n;
            return fibRecursive(n - 1) + fibRecursive(n - 2);
        }
        fibRecursive(10)
    "#,
    );
    assert_eq!(result, JsValue::Number(55.0));
}

#[test]
fn test_memoized_closure() {
    // Test closure-based memoization pattern
    let result = eval(
        r#"
        function createMemoizedFib(): (n: number) => number {
            const cache: { [key: number]: number } = {};
            return function fib(n: number): number {
                if (n in cache) return cache[n];
                if (n <= 1) return n;
                const result = fib(n - 1) + fib(n - 2);
                cache[n] = result;
                return result;
            };
        }
        const fib = createMemoizedFib();
        fib(10)
    "#,
    );
    assert_eq!(result, JsValue::Number(55.0));
}

#[test]
fn test_function_returning_array() {
    // Test function that returns an array with array methods
    let result = eval(
        r#"
        function getNumbers(): number[] {
            const arr: number[] = [];
            for (let i = 0; i < 5; i++) {
                arr.push(i);
            }
            return arr;
        }
        const nums = getNumbers();
        nums.map(x => x * 2).join(",")
    "#,
    );
    assert_eq!(result, JsValue::String("0,2,4,6,8".into()));
}

#[test]
fn test_fibonacci_iterative() {
    let result = eval(
        r#"
        function fibIterative(n: number): number {
            if (n <= 1) return n;
            let prev = 0;
            let curr = 1;
            for (let i = 2; i <= n; i++) {
                const next = prev + curr;
                prev = curr;
                curr = next;
            }
            return curr;
        }
        fibIterative(10)
    "#,
    );
    assert_eq!(result, JsValue::Number(55.0));
}
