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