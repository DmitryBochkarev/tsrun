//! Function-related tests

use super::eval;
use typescript_eval::value::JsString;
use typescript_eval::JsValue;

#[test]
fn test_function_name_property() {
    // Named function declaration
    assert_eq!(
        eval("function foo() {} foo.name"),
        JsValue::String(JsString::from("foo"))
    );
    // Named function expression
    assert_eq!(
        eval("const f = function bar() {}; f.name"),
        JsValue::String(JsString::from("bar"))
    );
    // Built-in function
    assert_eq!(
        eval("[].push.name"),
        JsValue::String(JsString::from("push"))
    );
}

#[test]
fn test_function_name_from_assignment() {
    // Anonymous function expression assigned to variable gets the variable name
    assert_eq!(
        eval("const myFunc = function() {}; myFunc.name"),
        JsValue::String(JsString::from("myFunc"))
    );
    // Arrow function
    assert_eq!(
        eval("const arrow = () => {}; arrow.name"),
        JsValue::String(JsString::from("arrow"))
    );
}

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

// Test that call stack depth limit works
#[test]
fn test_call_stack_depth_limit() {
    // Create a runtime with very low depth limit (10) to test without stack overflow
    let mut runtime = typescript_eval::Runtime::new();
    runtime.set_max_call_depth(10);

    // Test that recursion to depth 5 works (well under limit)
    let result = runtime.eval_simple(
        r#"
        function countDown(n) {
            if (n <= 0) return 0;
            return 1 + countDown(n - 1);
        }
        countDown(5)
    "#,
    );
    assert!(result.is_ok(), "countDown(5) should work");
    assert_eq!(result.unwrap(), typescript_eval::JsValue::Number(5.0));

    // Test that depth 15 should fail (over 10 limit)
    let mut runtime2 = typescript_eval::Runtime::new();
    runtime2.set_max_call_depth(10);
    let result2 = runtime2.eval_simple(
        r#"
        function countDown(n) {
            if (n <= 0) return 0;
            return 1 + countDown(n - 1);
        }
        countDown(15)
    "#,
    );

    assert!(
        result2.is_err(),
        "Deep recursion should error: {:?}",
        result2
    );
    let err = format!("{:?}", result2.unwrap_err());
    assert!(
        err.contains("Maximum call stack size exceeded"),
        "Error should mention stack size: {}",
        err
    );
}

// Test infinite recursion is caught
#[test]
fn test_infinite_recursion_caught() {
    // Use a low depth limit to ensure we catch it without Rust stack overflow
    let mut runtime = typescript_eval::Runtime::new();
    runtime.set_max_call_depth(10);

    let result = runtime.eval_simple(
        r#"
        function infinite() {
            return infinite();
        }
        infinite()
    "#,
    );

    assert!(result.is_err(), "Infinite recursion should error");
    let err = format!("{:?}", result.unwrap_err());
    assert!(
        err.contains("Maximum call stack size exceeded"),
        "Error should mention stack size: {}",
        err
    );
}

// ============================================================
// Array callback depth tests (for trampoline verification)
// ============================================================

/// Test that array callbacks don't blow the stack
/// This tests the current behavior - callbacks go through interp.call_function
/// which adds Rust stack frames. With trampoline migration, this would use
/// less Rust stack.
#[test]
fn test_array_map_callback_depth() {
    // Basic map with callback - should work fine
    let mut runtime = typescript_eval::Runtime::new();
    runtime.set_max_call_depth(100);

    let result = runtime.eval_simple(
        r#"
        [1, 2, 3].map(function(x) { return x * 2; })
    "#,
    );
    assert!(result.is_ok(), "Simple map should work: {:?}", result);
}

/// Test nested array callbacks - map inside map
#[test]
fn test_nested_array_callbacks() {
    let mut runtime = typescript_eval::Runtime::new();
    runtime.set_max_call_depth(100);

    let result = runtime.eval_simple(
        r#"
        const arr = [[1, 2], [3, 4]];
        arr.map(function(inner) {
            return inner.map(function(x) { return x * 2; });
        })
    "#,
    );
    assert!(result.is_ok(), "Nested map should work: {:?}", result);
}

/// Test array callback that itself recurses
/// This is the stress test for stack depth
#[test]
fn test_array_callback_with_recursion() {
    let mut runtime = typescript_eval::Runtime::new();
    runtime.set_max_call_depth(50);

    // The callback itself has recursion depth 5
    // forEach has 3 elements, each calling a recursive function
    let result = runtime.eval_simple(
        r#"
        let sum = 0;
        function countDown(n) {
            if (n <= 0) return 0;
            return 1 + countDown(n - 1);
        }
        [1, 2, 3].forEach(function(x) {
            sum += countDown(3);
        });
        sum
    "#,
    );
    assert!(
        result.is_ok(),
        "forEach with recursive callback should work: {:?}",
        result
    );
    assert_eq!(result.unwrap(), typescript_eval::JsValue::Number(9.0));
}

// ============================================================
// Function constructor tests
// ============================================================

#[test]
fn test_function_constructor_no_args() {
    // new Function(body) - no parameters, just body
    assert_eq!(
        eval("const f = new Function('return 42'); f()"),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_one_arg() {
    // new Function(param, body)
    assert_eq!(
        eval("const f = new Function('x', 'return x * 2'); f(21)"),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_two_args() {
    // new Function(param1, param2, body)
    assert_eq!(
        eval("const f = new Function('a', 'b', 'return a + b'); f(10, 32)"),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_three_args() {
    // new Function(param1, param2, param3, body)
    assert_eq!(
        eval("const f = new Function('a', 'b', 'c', 'return a + b + c'); f(10, 20, 12)"),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_comma_separated_params() {
    // Parameters can be comma-separated in a single string
    assert_eq!(
        eval("const f = new Function('a, b', 'return a + b'); f(10, 32)"),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_mixed_params() {
    // Mix of single params and comma-separated
    assert_eq!(
        eval("const f = new Function('a', 'b, c', 'return a + b + c'); f(10, 20, 12)"),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_no_body() {
    // Empty body returns undefined
    assert_eq!(eval("const f = new Function(''); f()"), JsValue::Undefined);
}

#[test]
fn test_function_constructor_empty_no_args() {
    // No arguments at all - should create a function with no params and empty body
    assert_eq!(eval("const f = new Function(); f()"), JsValue::Undefined);
}

#[test]
fn test_function_constructor_returns_string() {
    assert_eq!(
        eval("const f = new Function('return \"hello\"'); f()"),
        JsValue::from("hello")
    );
}

#[test]
fn test_function_constructor_multiple_statements() {
    // Body can have multiple statements
    assert_eq!(
        eval("const f = new Function('x', 'let y = x * 2; return y + 1'); f(20)"),
        JsValue::Number(41.0)
    );
}

#[test]
fn test_function_constructor_global_scope() {
    // Function constructor creates functions in global scope
    // They should NOT have access to local variables
    // Accessing undefined variable throws ReferenceError, so we use try-catch
    assert_eq!(
        eval(
            r#"
            const outer = 100;
            function test(): string {
                const inner = 50;
                const f = new Function('try { return inner; } catch(e) { return "not accessible"; }');
                return f();
            }
            test()
        "#
        ),
        JsValue::from("not accessible")
    );
}

#[test]
fn test_function_constructor_access_global() {
    // But they CAN access global variables
    assert_eq!(
        eval(
            r#"
            var globalVar = 42;
            const f = new Function('return globalVar');
            f()
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_this_binding() {
    // Function constructor creates functions that respect this binding
    assert_eq!(
        eval(
            r#"
            const f = new Function('return this.value');
            const obj = { value: 42 };
            f.call(obj)
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_call_without_new() {
    // Function() without new should work the same as new Function()
    assert_eq!(
        eval("const f = Function('x', 'return x + 1'); f(41)"),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_with_array_operations() {
    // Create a function that uses array methods
    assert_eq!(
        eval(
            r#"
            const mapper = new Function('arr', 'return arr.map(x => x * 2)');
            const result = mapper([1, 2, 3]);
            result.join(',')
        "#
        ),
        JsValue::from("2,4,6")
    );
}

#[test]
fn test_function_constructor_with_object() {
    // Create a function that returns an object
    assert_eq!(
        eval(
            r#"
            const makeObj = new Function('x', 'y', 'return { sum: x + y, product: x * y }');
            const obj = makeObj(3, 4);
            obj.sum + obj.product
        "#
        ),
        JsValue::Number(19.0) // 7 + 12
    );
}

#[test]
fn test_function_constructor_recursive() {
    // Create a recursive function using Function constructor
    assert_eq!(
        eval(
            r#"
            var factorial = new Function('n', 'return n <= 1 ? 1 : n * factorial(n - 1)');
            factorial(5)
        "#
        ),
        JsValue::Number(120.0)
    );
}

#[test]
fn test_function_constructor_closure_in_body() {
    // Body can create closures
    assert_eq!(
        eval(
            r#"
            const makeAdder = new Function('x', 'return function(y) { return x + y; }');
            const add10 = makeAdder(10);
            add10(32)
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_with_default_params() {
    // Parameters with default values in the parameter string
    assert_eq!(
        eval("const f = new Function('x = 10', 'return x * 2'); f()"),
        JsValue::Number(20.0)
    );
    assert_eq!(
        eval("const f = new Function('x = 10', 'return x * 2'); f(5)"),
        JsValue::Number(10.0)
    );
}

#[test]
fn test_function_constructor_rest_params() {
    // Rest parameters
    assert_eq!(
        eval(
            r#"
            const f = new Function('...args', 'return args.length');
            f(1, 2, 3, 4, 5)
        "#
        ),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_function_constructor_rest_params_sum() {
    assert_eq!(
        eval(
            r#"
            const sum = new Function('...nums', 'return nums.reduce((a, b) => a + b, 0)');
            sum(1, 2, 3, 4, 5)
        "#
        ),
        JsValue::Number(15.0)
    );
}

#[test]
fn test_function_constructor_destructuring_param() {
    // Destructuring in parameters
    assert_eq!(
        eval(
            r#"
            const f = new Function('{x, y}', 'return x + y');
            f({x: 10, y: 32})
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_array_destructuring() {
    assert_eq!(
        eval(
            r#"
            const f = new Function('[a, b]', 'return a * b');
            f([6, 7])
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_length_property() {
    // Function.length should be the number of formal parameters
    assert_eq!(
        eval("const f = new Function('a', 'b', 'c', 'return 0'); f.length"),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_function_constructor_name_property() {
    // Function created by constructor should have name "anonymous"
    assert_eq!(
        eval("const f = new Function('return 0'); f.name"),
        JsValue::from("anonymous")
    );
}

#[test]
fn test_function_constructor_is_callable() {
    assert_eq!(
        eval("typeof new Function('return 1')"),
        JsValue::from("function")
    );
}

#[test]
fn test_function_constructor_bind() {
    // Bound function created from Function constructor
    assert_eq!(
        eval(
            r#"
            const f = new Function('a', 'b', 'return a + b');
            const bound = f.bind(null, 10);
            bound(32)
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_apply() {
    assert_eq!(
        eval(
            r#"
            const f = new Function('a', 'b', 'return a + b');
            f.apply(null, [10, 32])
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_call() {
    assert_eq!(
        eval(
            r#"
            const f = new Function('a', 'b', 'return a + b');
            f.call(null, 10, 32)
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_constructor_syntax_error() {
    // Invalid syntax should throw SyntaxError
    use super::throws_error;
    assert!(throws_error(
        "const f = new Function('return return');",
        "SyntaxError"
    ));
}

#[test]
fn test_function_constructor_whitespace_in_params() {
    // Whitespace around parameter names should be trimmed
    assert_eq!(
        eval("const f = new Function('  x  ', '  y  ', 'return x + y'); f(10, 32)"),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_prototype_constructor() {
    // Function.prototype.constructor should be Function
    assert_eq!(
        eval("Function.prototype.constructor === Function"),
        JsValue::Boolean(true)
    );
}

// ============================================================
// Function.prototype methods on proxied functions
// ============================================================

#[test]
fn test_function_has_call_method() {
    // Regular functions should have access to call method
    assert_eq!(
        eval(
            r#"
            let fn = function() { return this.value; };
            typeof fn.call
        "#
        ),
        JsValue::from("function")
    );
}

#[test]
fn test_proxied_function_has_call_method() {
    // Proxied functions should have access to call method via prototype chain
    assert_eq!(
        eval(
            r#"
            let fn = function() { return this.value; };
            let p = new Proxy(fn, {});
            typeof p.call
        "#
        ),
        JsValue::from("function")
    );
}

#[test]
fn test_proxied_function_call_method_stored() {
    // Store the call method in a variable, then invoke it
    assert_eq!(
        eval(
            r#"
            let fn = function() { return this.value; };
            let p = new Proxy(fn, {});
            let callMethod = p.call;
            let obj = { value: 42 };
            callMethod.call(fn, obj)
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_function_call_on_function_value() {
    // Get function from proxy, use call on it with proxy as this
    assert_eq!(
        eval(
            r#"
            let fn = function() { return this.value; };
            let p = new Proxy(fn, {});
            let obj = { value: 42 };
            Function.prototype.call.call(p, obj)
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_proxied_function_call_method() {
    // Proxied functions should have access to Function.prototype.call
    assert_eq!(
        eval(
            r#"
            let fn = function() { return this.value; };
            let p = new Proxy(fn, {});
            let obj = { value: 42 };
            p.call(obj)
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_proxied_function_apply_method() {
    // Proxied functions should have access to Function.prototype.apply
    assert_eq!(
        eval(
            r#"
            let fn = function(a: number, b: number) { return a + b; };
            let p = new Proxy(fn, {});
            p.apply(null, [10, 32])
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_proxied_function_bind_method() {
    // Proxied functions should have access to Function.prototype.bind
    assert_eq!(
        eval(
            r#"
            let fn = function() { return this.value; };
            let p = new Proxy(fn, {});
            let bound = p.bind({ value: 42 });
            bound()
        "#
        ),
        JsValue::Number(42.0)
    );
}

// ============================================================
// Spread in new expressions
// ============================================================

#[test]
fn test_spread_in_new_basic() {
    // Basic spread in new expression
    assert_eq!(
        eval(
            r#"
            function Point(x: number, y: number) {
                this.x = x;
                this.y = y;
            }
            let args: number[] = [10, 20];
            let p = new Point(...args);
            p.x + p.y
        "#
        ),
        JsValue::Number(30.0)
    );
}

#[test]
fn test_spread_in_new_with_regular_args() {
    // Spread combined with regular arguments
    assert_eq!(
        eval(
            r#"
            function Triple(a: number, b: number, c: number) {
                this.sum = a + b + c;
            }
            let rest: number[] = [2, 3];
            let t = new Triple(1, ...rest);
            t.sum
        "#
        ),
        JsValue::Number(6.0)
    );
}

#[test]
fn test_spread_in_new_multiple_spreads() {
    // Multiple spreads in new expression
    assert_eq!(
        eval(
            r#"
            function Sum(a: number, b: number, c: number, d: number) {
                this.total = a + b + c + d;
            }
            let arr1: number[] = [1, 2];
            let arr2: number[] = [3, 4];
            let s = new Sum(...arr1, ...arr2);
            s.total
        "#
        ),
        JsValue::Number(10.0)
    );
}

// ============================================================
// Function name and length property tests (Test262 conformance)
// ============================================================

#[test]
fn test_function_declaration_name() {
    // Function declarations should have a name property
    assert_eq!(eval(r#"function foo() {} foo.name"#), JsValue::from("foo"));
}

#[test]
fn test_function_declaration_length() {
    // Function.length should be the number of formal parameters
    assert_eq!(
        eval(r#"function foo(a: number, b: number, c: number) {} foo.length"#),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_function_declaration_length_no_params() {
    assert_eq!(
        eval(r#"function foo() {} foo.length"#),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_function_expression_name() {
    // Named function expression should have the name
    assert_eq!(
        eval(r#"const f = function bar() {}; f.name"#),
        JsValue::from("bar")
    );
}

// NOTE: ES6+ name inference for anonymous functions assigned to variables
// is not yet implemented. These functions should have the variable name
// but currently have empty strings.
// - Anonymous function expressions: `const f = function() {}` -> f.name = "f"  (not working)
// - Arrow functions: `const f = () => {}` -> f.name = "f"  (not working)
// - Anonymous classes: `const C = class {}` -> C.name = "C"  (not working)

#[test]
fn test_arrow_function_length() {
    assert_eq!(
        eval(r#"const f = (a: number, b: number) => a + b; f.length"#),
        JsValue::Number(2.0)
    );
}

// NOTE: Method shorthand name inference is not yet implemented
// `{ myMethod() {} }` should give myMethod.name = "myMethod" but currently is ""

#[test]
fn test_class_constructor_name() {
    // Class constructor should have the class name
    assert_eq!(
        eval(r#"class MyClass {} MyClass.name"#),
        JsValue::from("MyClass")
    );
}

#[test]
fn test_class_expression_name() {
    // Named class expression should have the class name
    assert_eq!(
        eval(r#"const C = class MyClass {}; C.name"#),
        JsValue::from("MyClass")
    );
}

// test_class_expression_anonymous_name is commented out -
// see NOTE above about ES6+ name inference for anonymous classes

// ============================================================
// Function.call/apply `this` binding tests (strict mode)
// ============================================================

// In strict mode, `this` is passed through exactly as provided.
// Primitives remain primitives, null/undefined remain as-is.

#[test]
fn test_call_this_null_strict() {
    // In strict mode, this should be null
    assert_eq!(
        eval(r#"function f() { return this; } f.call(null)"#),
        JsValue::Null
    );
}

#[test]
fn test_call_this_undefined_strict() {
    // In strict mode, this should be undefined
    assert_eq!(
        eval(r#"function f() { return this; } f.call(undefined)"#),
        JsValue::Undefined
    );
}

#[test]
fn test_call_this_primitive_number() {
    // In strict mode, primitive this should stay primitive
    assert_eq!(
        eval(r#"function f() { return this; } f.call(42)"#),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_call_this_primitive_string() {
    assert_eq!(
        eval(r#"function f() { return this; } f.call("hello")"#),
        JsValue::from("hello")
    );
}

#[test]
fn test_call_this_primitive_boolean() {
    assert_eq!(
        eval(r#"function f() { return this; } f.call(true)"#),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_apply_this_null_strict() {
    assert_eq!(
        eval(r#"function f() { return this; } f.apply(null)"#),
        JsValue::Null
    );
}

#[test]
fn test_apply_this_primitive_number() {
    assert_eq!(
        eval(r#"function f() { return this; } f.apply(42)"#),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_call_with_args_this_preserved() {
    // this should be preserved while args work correctly
    assert_eq!(
        eval(r#"function f(a: number, b: number) { return this + a + b; } f.call(100, 20, 3)"#),
        JsValue::Number(123.0)
    );
}

#[test]
fn test_apply_with_args_this_preserved() {
    assert_eq!(
        eval(r#"function f(a: number, b: number) { return this + a + b; } f.apply(100, [20, 3])"#),
        JsValue::Number(123.0)
    );
}

// =============================================================================
// Call Expression Argument Evaluation Order Tests
// =============================================================================

#[test]
fn test_call_args_evaluated_before_callable_check() {
    // Arguments should be evaluated before checking if the callee is callable
    // This matches ECMAScript spec behavior
    assert_eq!(
        eval(
            r#"
            var fooCalled = false;
            function foo() { fooCalled = true; }
            var o = {};
            try {
                o.bar(foo());
            } catch (e) {}
            fooCalled
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_call_args_side_effects_on_undefined() {
    // Side effects in arguments should occur even when calling undefined
    assert_eq!(
        eval(
            r#"
            var count = 0;
            function increment() { count++; return count; }
            try {
                undefined(increment(), increment());
            } catch (e) {}
            count
        "#
        ),
        JsValue::Number(2.0)
    );
}

// ============================================================
// Built-in function length/name property tests (Test262 conformance)
// ============================================================

#[test]
fn test_builtin_function_has_length() {
    // Built-in functions should have a length property
    assert_eq!(
        eval(r#"JSON.parse.hasOwnProperty("length")"#),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_builtin_function_length_value() {
    // JSON.parse has 2 parameters (text, reviver)
    assert_eq!(eval(r#"JSON.parse.length"#), JsValue::Number(2.0));
}

#[test]
fn test_builtin_function_has_name() {
    // Built-in functions should have a name property
    assert_eq!(
        eval(r#"JSON.parse.hasOwnProperty("name")"#),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_builtin_function_name_value() {
    assert_eq!(eval(r#"JSON.parse.name"#), JsValue::from("parse"));
}

#[test]
fn test_array_push_has_length() {
    assert_eq!(
        eval(r#"[].push.hasOwnProperty("length")"#),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_array_push_length_value() {
    // Array.prototype.push has 1 parameter (but can accept multiple)
    assert_eq!(eval(r#"[].push.length"#), JsValue::Number(1.0));
}

#[test]
fn test_function_prototype_call_has_length() {
    assert_eq!(
        eval(r#"Function.prototype.call.hasOwnProperty("length")"#),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_function_prototype_call_length() {
    // Function.prototype.call has 1 parameter (thisArg)
    assert_eq!(
        eval(r#"Function.prototype.call.length"#),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_call_args_not_evaluated_when_callee_throws() {
    // When evaluating the callee throws (e.g., undefined.prop),
    // arguments should NOT be evaluated
    assert_eq!(
        eval(
            r#"
            var fooCalled = false;
            function foo() { fooCalled = true; }
            var o = {};
            try {
                o.bar.gar(foo());  // o.bar is undefined, .gar throws
            } catch (e) {}
            fooCalled
        "#
        ),
        JsValue::Boolean(false)
    );
}

// ============================================================
// Function.prototype Tests (ES5/ES6 conformance)
// ============================================================

#[test]
fn test_function_prototype_is_function() {
    // The Function prototype object is itself a Function object
    // Object.prototype.toString.call(Function.prototype) should return "[object Function]"
    assert_eq!(
        eval(r#"Object.prototype.toString.call(Function.prototype)"#),
        JsValue::from("[object Function]")
    );
}

#[test]
fn test_function_prototype_typeof() {
    // typeof Function.prototype should be "function"
    assert_eq!(
        eval(r#"typeof Function.prototype"#),
        JsValue::from("function")
    );
}

#[test]
fn test_function_prototype_callable() {
    // Function.prototype should be callable and return undefined
    assert_eq!(eval(r#"Function.prototype()"#), JsValue::Undefined);
}

#[test]
fn test_function_prototype_callable_with_args() {
    // Function.prototype accepts any arguments and returns undefined
    assert_eq!(
        eval(r#"Function.prototype(1, 2, 3, "test", {})"#),
        JsValue::Undefined
    );
}

#[test]
fn test_function_prototype_callable_with_this() {
    // Function.prototype can be called with any this value
    assert_eq!(
        eval(r#"Function.prototype.call({value: 42})"#),
        JsValue::Undefined
    );
}

#[test]
fn test_function_prototype_length() {
    // Function.prototype.length should be 0
    assert_eq!(eval(r#"Function.prototype.length"#), JsValue::Number(0.0));
}

#[test]
fn test_function_prototype_name() {
    // Function.prototype.name should be empty string
    assert_eq!(eval(r#"Function.prototype.name"#), JsValue::from(""));
}

#[test]
fn test_function_prototype_has_call() {
    // Function.prototype should have call method
    assert_eq!(
        eval(r#"typeof Function.prototype.call"#),
        JsValue::from("function")
    );
}

#[test]
fn test_function_prototype_has_apply() {
    // Function.prototype should have apply method
    assert_eq!(
        eval(r#"typeof Function.prototype.apply"#),
        JsValue::from("function")
    );
}

#[test]
fn test_function_prototype_has_bind() {
    // Function.prototype should have bind method
    assert_eq!(
        eval(r#"typeof Function.prototype.bind"#),
        JsValue::from("function")
    );
}

#[test]
fn test_function_inherits_from_function_prototype() {
    // Regular functions should inherit from Function.prototype
    assert_eq!(
        eval(r#"function foo() {} Object.getPrototypeOf(foo) === Function.prototype"#),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_arrow_inherits_from_function_prototype() {
    // Arrow functions should inherit from Function.prototype
    assert_eq!(
        eval(r#"const f = () => {}; Object.getPrototypeOf(f) === Function.prototype"#),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_function_prototype_inherits_from_object_prototype() {
    // Function.prototype should inherit from Object.prototype
    assert_eq!(
        eval(r#"Object.getPrototypeOf(Function.prototype) === Object.prototype"#),
        JsValue::Boolean(true)
    );
}

// ============================================================
// Symbol.hasInstance tests
// ============================================================

#[test]
fn test_function_has_symbol_hasinstance() {
    // Function.prototype should have Symbol.hasInstance
    assert_eq!(
        eval(r#"typeof Function.prototype[Symbol.hasInstance]"#),
        JsValue::from("function")
    );
}

#[test]
fn test_instanceof_uses_symbol_hasinstance() {
    // instanceof should use Symbol.hasInstance
    assert_eq!(
        eval(
            r#"
            function Foo() {}
            const obj = new Foo();
            obj instanceof Foo
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_symbol_hasinstance_direct_call() {
    // Can call Symbol.hasInstance directly
    assert_eq!(
        eval(
            r#"
            function Foo() {}
            const obj = new Foo();
            Foo[Symbol.hasInstance](obj)
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_symbol_hasinstance_non_instance() {
    // Symbol.hasInstance returns false for non-instances
    assert_eq!(
        eval(
            r#"
            function Foo() {}
            function Bar() {}
            const obj = new Bar();
            Foo[Symbol.hasInstance](obj)
        "#
        ),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_symbol_hasinstance_primitive() {
    // Symbol.hasInstance returns false for primitives
    assert_eq!(
        eval(r#"Function.prototype[Symbol.hasInstance].call(Object, 42)"#),
        JsValue::Boolean(false)
    );
}

// Tests for function name inference in destructuring patterns
#[test]
fn test_function_name_in_destructuring_default_array() {
    // Arrow function in array destructuring default should get the binding name
    assert_eq!(
        eval(
            r#"
            let [arrow = () => {}] = [];
            arrow.name
        "#
        ),
        JsValue::String(JsString::from("arrow"))
    );
}

#[test]
fn test_function_name_in_destructuring_default_object() {
    // Arrow function in object destructuring default should get the binding name
    assert_eq!(
        eval(
            r#"
            function test({fn = () => {}}) {
                return fn.name;
            }
            test({})
        "#
        ),
        JsValue::String(JsString::from("fn"))
    );
}

#[test]
fn test_function_name_in_destructuring_default_function_expr() {
    // Function expression in destructuring default should get the binding name
    assert_eq!(
        eval(
            r#"
            function test([func = function() {}]) {
                return func.name;
            }
            test([])
        "#
        ),
        JsValue::String(JsString::from("func"))
    );
}

// Function property descriptor tests
#[test]
fn test_function_name_descriptor() {
    // name property should be: { writable: false, enumerable: false, configurable: true }
    assert_eq!(
        eval(
            r#"
            const f = function foo(a: number, b: number): number { return a + b; };
            const desc = Object.getOwnPropertyDescriptor(f, 'name');
            desc.writable
        "#
        ),
        JsValue::Boolean(false)
    );

    assert_eq!(
        eval(
            r#"
            const f = function foo(a: number, b: number): number { return a + b; };
            const desc = Object.getOwnPropertyDescriptor(f, 'name');
            desc.enumerable
        "#
        ),
        JsValue::Boolean(false)
    );

    assert_eq!(
        eval(
            r#"
            const f = function foo(a: number, b: number): number { return a + b; };
            const desc = Object.getOwnPropertyDescriptor(f, 'name');
            desc.configurable
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_function_length_descriptor() {
    // length property should be: { writable: false, enumerable: false, configurable: true }
    assert_eq!(
        eval(
            r#"
            const f = function foo(a: number, b: number): number { return a + b; };
            const desc = Object.getOwnPropertyDescriptor(f, 'length');
            desc.writable
        "#
        ),
        JsValue::Boolean(false)
    );

    assert_eq!(
        eval(
            r#"
            const f = function foo(a: number, b: number): number { return a + b; };
            const desc = Object.getOwnPropertyDescriptor(f, 'length');
            desc.enumerable
        "#
        ),
        JsValue::Boolean(false)
    );

    assert_eq!(
        eval(
            r#"
            const f = function foo(a: number, b: number): number { return a + b; };
            const desc = Object.getOwnPropertyDescriptor(f, 'length');
            desc.configurable
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_builtin_function_name_descriptor() {
    // Built-in function name should also have correct descriptors
    assert_eq!(
        eval(
            r#"
            const desc = Object.getOwnPropertyDescriptor([].push, 'name');
            [desc.writable, desc.enumerable, desc.configurable].join(',')
        "#
        ),
        JsValue::from("false,false,true")
    );
}

#[test]
fn test_builtin_function_length_descriptor() {
    // Built-in function length should also have correct descriptors
    assert_eq!(
        eval(
            r#"
            const desc = Object.getOwnPropertyDescriptor([].push, 'length');
            [desc.writable, desc.enumerable, desc.configurable].join(',')
        "#
        ),
        JsValue::from("false,false,true")
    );
}

#[test]
fn test_arrow_function_name_descriptor() {
    // Arrow function name should have correct descriptors
    assert_eq!(
        eval(
            r#"
            const f = (a: number) => a * 2;
            const desc = Object.getOwnPropertyDescriptor(f, 'name');
            [desc.writable, desc.enumerable, desc.configurable].join(',')
        "#
        ),
        JsValue::from("false,false,true")
    );
}

#[test]
fn test_arrow_function_length_descriptor() {
    // Arrow function length should have correct descriptors
    assert_eq!(
        eval(
            r#"
            const f = (a: number, b: number) => a + b;
            const desc = Object.getOwnPropertyDescriptor(f, 'length');
            [desc.writable, desc.enumerable, desc.configurable].join(',')
        "#
        ),
        JsValue::from("false,false,true")
    );
}

// ============================================================
// Function.prototype.call.bind() tests (Test262 propertyHelper pattern)
// ============================================================

#[test]
fn test_call_bind_basic() {
    // Function.prototype.call.bind() creates a function that calls the original
    // with a specific receiver
    assert_eq!(
        eval(
            r#"
            var __push = Function.prototype.call.bind(Array.prototype.push);
            var arr = [1, 2];
            __push(arr, 3);
            arr.length
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_call_bind_with_array() {
    // Verify the pushed value is correct
    assert_eq!(
        eval(
            r#"
            var __push = Function.prototype.call.bind(Array.prototype.push);
            var arr = [1, 2];
            __push(arr, 3);
            arr[2]
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_call_bind_hasownproperty() {
    // Pattern used in test262: Function.prototype.call.bind(Object.prototype.hasOwnProperty)
    assert_eq!(
        eval(
            r#"
            var __hasOwnProperty = Function.prototype.call.bind(Object.prototype.hasOwnProperty);
            var obj = { foo: 1 };
            __hasOwnProperty(obj, "foo")
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_call_bind_hasownproperty_false() {
    assert_eq!(
        eval(
            r#"
            var __hasOwnProperty = Function.prototype.call.bind(Object.prototype.hasOwnProperty);
            var obj = { foo: 1 };
            __hasOwnProperty(obj, "bar")
        "#
        ),
        JsValue::Boolean(false)
    );
}

#[test]
fn test_call_bind_join() {
    // Pattern: Function.prototype.call.bind(Array.prototype.join)
    assert_eq!(
        eval(
            r#"
            var __join = Function.prototype.call.bind(Array.prototype.join);
            var arr = [1, 2, 3];
            __join(arr, "-")
        "#
        ),
        JsValue::from("1-2-3")
    );
}

#[test]
fn test_call_bind_tostring() {
    // Pattern: Function.prototype.call.bind(Object.prototype.toString)
    assert_eq!(
        eval(
            r#"
            var __toString = Function.prototype.call.bind(Object.prototype.toString);
            __toString({})
        "#
        ),
        JsValue::from("[object Object]")
    );
}

#[test]
fn test_call_bind_get_own_property_descriptor() {
    // Pattern: Function.prototype.call.bind on Object.getOwnPropertyDescriptor
    assert_eq!(
        eval(
            r#"
            var __getOwnPropertyDescriptor = Function.prototype.call.bind(Object.getOwnPropertyDescriptor);
            var obj = { foo: 1 };
            var desc = __getOwnPropertyDescriptor(Object, obj, "foo");
            desc.value
        "#
        ),
        JsValue::Number(1.0)
    );
}
