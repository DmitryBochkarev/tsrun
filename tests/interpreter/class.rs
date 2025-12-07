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

// Private field tests
#[test]
#[ignore] // TODO: Class declarations not implemented - execute_class_declaration is a stub
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
#[ignore] // TODO: Implement private fields
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
#[ignore] // TODO: Implement private fields
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
