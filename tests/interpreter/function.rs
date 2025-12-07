//! Function-related tests

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_function() {
    assert_eq!(eval("function add(a, b) { return a + b; } add(2, 3)"), JsValue::Number(5.0));
}

#[test]
fn test_this_binding() {
    // Test that 'this' is properly bound in method calls
    assert_eq!(eval("let obj = {x: 42, getX: function() { return this.x; }}; obj.getX()"), JsValue::Number(42.0));
}

#[test]
fn test_function_call() {
    assert_eq!(eval("function greet() { return 'Hello ' + this.name; } greet.call({name: 'World'})"), JsValue::from("Hello World"));
    assert_eq!(eval("function add(a, b) { return a + b; } add.call(null, 2, 3)"), JsValue::Number(5.0));
}

#[test]
fn test_function_apply() {
    assert_eq!(eval("function greet() { return 'Hello ' + this.name; } greet.apply({name: 'World'})"), JsValue::from("Hello World"));
    assert_eq!(eval("function add(a, b) { return a + b; } add.apply(null, [2, 3])"), JsValue::Number(5.0));
}

#[test]
fn test_function_bind() {
    assert_eq!(eval("function greet() { return 'Hello ' + this.name; } const boundGreet = greet.bind({name: 'World'}); boundGreet()"), JsValue::from("Hello World"));
    assert_eq!(eval("function add(a, b) { return a + b; } const add5 = add.bind(null, 5); add5(3)"), JsValue::Number(8.0));
}

#[test]
fn test_arrow_function() {
    assert_eq!(eval("const add = (a, b) => a + b; add(2, 3)"), JsValue::Number(5.0));
}

// Tests for the `arguments` object
#[test]
fn test_arguments_length() {
    assert_eq!(eval("function f() { return arguments.length; } f(1, 2, 3)"), JsValue::Number(3.0));
    assert_eq!(eval("function f() { return arguments.length; } f()"), JsValue::Number(0.0));
}

#[test]
fn test_arguments_access() {
    assert_eq!(eval("function f() { return arguments[0]; } f(42)"), JsValue::Number(42.0));
    assert_eq!(eval("function f() { return arguments[1]; } f(1, 2, 3)"), JsValue::Number(2.0));
    assert_eq!(eval("function f() { return arguments[2]; } f('a', 'b', 'c')"), JsValue::from("c"));
}

#[test]
fn test_arguments_out_of_bounds() {
    assert_eq!(eval("function f() { return arguments[5]; } f(1, 2)"), JsValue::Undefined);
}

#[test]
fn test_arguments_with_named_params() {
    // arguments should still contain all args even when named params exist
    assert_eq!(eval("function f(a, b) { return arguments.length; } f(1, 2, 3, 4)"), JsValue::Number(4.0));
    assert_eq!(eval("function f(a, b) { return arguments[2]; } f(1, 2, 3)"), JsValue::Number(3.0));
}

#[test]
fn test_arguments_in_nested_function() {
    // Each function has its own arguments object
    assert_eq!(
        eval("function outer() { function inner() { return arguments[0]; } return inner(42); } outer(99)"),
        JsValue::Number(42.0)
    );
}