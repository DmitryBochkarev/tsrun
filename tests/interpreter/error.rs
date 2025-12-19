//! Error-related tests

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_reference_error_message_format() {
    use super::eval_result;

    // ReferenceError message should not have duplicate "is not defined"
    let err = eval_result("undefinedVariable").unwrap_err();
    let err_debug = format!("{:?}", err);
    println!("Error debug: {}", err_debug);

    // Should contain the variable name
    assert!(
        err_debug.contains("undefinedVariable"),
        "Error should contain variable name"
    );

    // Should NOT contain duplicate text
    assert!(
        !err_debug.contains("is not defined is not defined"),
        "Should NOT have duplicate 'is not defined'"
    );
}

#[test]
fn test_error_constructor() {
    assert_eq!(eval("new Error('oops').message"), JsValue::from("oops"));
    assert_eq!(eval("new Error('oops').name"), JsValue::from("Error"));
}

#[test]
fn test_typeerror() {
    assert_eq!(
        eval("new TypeError('bad type').name"),
        JsValue::from("TypeError")
    );
    assert_eq!(
        eval("new TypeError('bad type').message"),
        JsValue::from("bad type")
    );
}

#[test]
fn test_rangeerror() {
    assert_eq!(
        eval("new RangeError('out of range').name"),
        JsValue::from("RangeError")
    );
    assert_eq!(
        eval("new RangeError('out of range').message"),
        JsValue::from("out of range")
    );
}

// Error.prototype.toString tests
#[test]
fn test_error_tostring_basic() {
    // Standard format: "ErrorName: message"
    assert_eq!(
        eval("new Error('something went wrong').toString()"),
        JsValue::from("Error: something went wrong")
    );
}

#[test]
fn test_error_tostring_no_message() {
    // When message is empty, just return name
    assert_eq!(eval("new Error().toString()"), JsValue::from("Error"));
}

#[test]
fn test_error_tostring_typeerror() {
    assert_eq!(
        eval("new TypeError('invalid argument').toString()"),
        JsValue::from("TypeError: invalid argument")
    );
}

#[test]
fn test_error_tostring_referenceerror() {
    assert_eq!(
        eval("new ReferenceError('x is not defined').toString()"),
        JsValue::from("ReferenceError: x is not defined")
    );
}

#[test]
fn test_error_tostring_custom() {
    // Custom name and message
    assert_eq!(
        eval(
            r#"
            const e = new Error('oops');
            e.name = 'CustomError';
            e.toString()
        "#
        ),
        JsValue::from("CustomError: oops")
    );
}

// Stack trace tests
#[test]
fn test_error_stack_exists() {
    // Error objects should have a stack property
    assert_eq!(
        eval("typeof new Error('test').stack"),
        JsValue::from("string")
    );
}

#[test]
fn test_error_stack_contains_error_name() {
    // Stack should start with error type and message
    assert_eq!(
        eval("new Error('test message').stack.includes('Error: test message')"),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_error_stack_in_function() {
    // Stack should include function names
    assert_eq!(
        eval(
            r#"
            function foo(): Error {
                return new Error('in foo');
            }
            foo().stack.includes('foo')
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_urierror() {
    assert_eq!(
        eval("new URIError('invalid URI').name"),
        JsValue::from("URIError")
    );
    assert_eq!(
        eval("new URIError('invalid URI').message"),
        JsValue::from("invalid URI")
    );
    assert_eq!(
        eval("new URIError('malformed').toString()"),
        JsValue::from("URIError: malformed")
    );
}

#[test]
fn test_evalerror() {
    assert_eq!(
        eval("new EvalError('eval failed').name"),
        JsValue::from("EvalError")
    );
    assert_eq!(
        eval("new EvalError('eval failed').message"),
        JsValue::from("eval failed")
    );
    assert_eq!(
        eval("new EvalError('bad eval').toString()"),
        JsValue::from("EvalError: bad eval")
    );
}

#[test]
fn test_catch_variable_assignment() {
    // Catch variable should be assignable to outer scope variable
    assert_eq!(
        eval(
            r#"
            let caught: Error | undefined;
            try {
                throw new Error("test error");
            } catch (e) {
                caught = e;
            }
            caught.message
        "#
        ),
        JsValue::from("test error")
    );
}

#[test]
fn test_catch_with_instanceof() {
    // Catch with instanceof check and assignment
    assert_eq!(
        eval(
            r#"
            let caught: Error | undefined;
            try {
                throw new Error("test error");
            } catch (e) {
                if (e instanceof Error) {
                    caught = e;
                }
            }
            caught.message
        "#
        ),
        JsValue::from("test error")
    );
}

#[test]
fn test_if_block_assigns_outer_variable() {
    // Simpler test: assignment inside if block should work
    assert_eq!(
        eval(
            r#"
            let x: number = 0;
            if (true) {
                x = 42;
            }
            x
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_if_block_assigns_outer_variable_with_condition() {
    // Test with non-trivial condition
    assert_eq!(
        eval(
            r#"
            let x: number = 0;
            const cond = true;
            if (cond) {
                x = 42;
            }
            x
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_catch_then_if_nested() {
    // Test if block inside catch block
    assert_eq!(
        eval(
            r#"
            let x: number = 0;
            try {
                throw new Error("test");
            } catch (e) {
                if (true) {
                    x = 42;
                }
            }
            x
        "#
        ),
        JsValue::Number(42.0)
    );
}

#[test]
fn test_instanceof_error_basic() {
    // Test instanceof Error returns true
    assert_eq!(
        eval(
            r#"
            let err: Error = new Error("test");
            err instanceof Error
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_instanceof_catch_variable() {
    // Test instanceof on catch variable
    assert_eq!(
        eval(
            r#"
            let result: boolean = false;
            try {
                throw new Error("test");
            } catch (e) {
                result = e instanceof Error;
            }
            result
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_instanceof_catch_variable_condition() {
    // Test using instanceof in an if condition inside catch
    assert_eq!(
        eval(
            r#"
            let isError: boolean = false;
            try {
                throw new Error("test");
            } catch (e) {
                if (e instanceof Error) {
                    isError = true;
                }
            }
            isError
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_array_push_in_catch_if() {
    // Test pushing to array inside if block inside catch block
    assert_eq!(
        eval(
            r#"
            const errors: string[] = [];
            try {
                throw new Error("test error");
            } catch (e) {
                if (e instanceof Error) {
                    errors.push(e.message);
                }
            }
            errors.length
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_for_loop_inside_function_catch() {
    // Test for loop inside catch inside function
    assert_eq!(
        eval(
            r#"
            function validate(): string[] {
                const errors: string[] = [];
                const validators = [() => { throw new Error("e1"); }, () => { throw new Error("e2"); }];
                for (const v of validators) {
                    try {
                        v();
                    } catch (e) {
                        if (e instanceof Error) {
                            errors.push(e.message);
                        }
                    }
                }
                return errors;
            }
            validate().join(",")
        "#
        ),
        JsValue::from("e1,e2")
    );
}

#[test]
fn test_nested_for_loops_with_catch() {
    // Test nested for loops with try-catch - similar to validateObject
    assert_eq!(
        eval(
            r#"
            function validate(): string[] {
                const errors: string[] = [];
                const rules = [
                    { validators: [() => { throw new Error("e1"); }] },
                    { validators: [() => { throw new Error("e2"); }] }
                ];
                for (const rule of rules) {
                    for (const validator of rule.validators) {
                        try {
                            validator();
                        } catch (e) {
                            if (e instanceof Error) {
                                errors.push(e.message);
                            }
                        }
                    }
                }
                return errors;
            }
            validate().join(",")
        "#
        ),
        JsValue::from("e1,e2")
    );
}

#[test]
fn test_top_level_catch_with_array_push() {
    // Test that array push works in catch block at top level
    assert_eq!(
        eval(
            r#"
            const errors: string[] = [];
            try { throw new Error("e1"); } catch (e) { if (e instanceof Error) errors.push(e.message); }
            try { throw new Error("e2"); } catch (e) { if (e instanceof Error) errors.push(e.message); }
            errors.join(",")
        "#
        ),
        JsValue::from("e1,e2")
    );
}

#[test]
fn test_catch_instanceof_assign_outer_var_in_loop() {
    // Outer var assigned inside catch with instanceof inside a loop
    assert_eq!(
        eval(
            r#"
            function retry(): string {
                let errorMsg: string = "default";
                for (let i = 0; i < 3; i++) {
                    try {
                        throw new Error("attempt " + i);
                    } catch (e) {
                        if (e instanceof Error) {
                            errorMsg = e.message;
                        }
                    }
                }
                return errorMsg;
            }
            retry()
        "#
        ),
        JsValue::from("attempt 2")
    );
}

#[test]
fn test_closure_arg_with_catch_instanceof_simplified() {
    // Simplest reproduction: pass a closure that throws to a function with try-catch-instanceof
    assert_eq!(
        eval(
            r#"
            function callWithCatch(fn: () => void): string {
                let msg: string = "none";
                try {
                    fn();
                } catch (e) {
                    if (e instanceof Error) {
                        msg = e.message;
                    }
                }
                return msg;
            }

            callWithCatch(() => { throw new Error("test"); })
        "#
        ),
        JsValue::from("test")
    );
}

#[test]
fn test_closure_arg_with_catch_no_instanceof() {
    // Same but without instanceof - does it work?
    assert_eq!(
        eval(
            r#"
            function callWithCatch(fn: () => void): string {
                let msg: string = "none";
                try {
                    fn();
                } catch (e) {
                    msg = "caught";
                }
                return msg;
            }

            callWithCatch(() => { throw new Error("test"); })
        "#
        ),
        JsValue::from("caught")
    );
}

#[test]
fn test_closure_arg_with_catch_direct_assign() {
    // Minimal: direct assignment in catch after closure throws
    assert_eq!(
        eval(
            r#"
            function test(fn: () => void): string {
                let x: string = "before";
                try {
                    fn();
                } catch (e) {
                    x = "after";
                }
                return x;
            }

            test(() => { throw "error"; })
        "#
        ),
        JsValue::from("after")
    );
}

#[test]
fn test_named_function_throw_in_try_catch() {
    // Does it work with a named function that throws?
    assert_eq!(
        eval(
            r#"
            function thrower(): void {
                throw "error";
            }

            function test(): string {
                let x: string = "before";
                try {
                    thrower();
                } catch (e) {
                    x = "after";
                }
                return x;
            }

            test()
        "#
        ),
        JsValue::from("after")
    );
}

#[test]
fn test_closure_arg_with_catch_typeof() {
    // Same but with typeof instead of instanceof
    assert_eq!(
        eval(
            r#"
            function callWithCatch(fn: () => void): string {
                let msg: string = "none";
                try {
                    fn();
                } catch (e) {
                    if (typeof e === "object") {
                        msg = "object caught";
                    }
                }
                return msg;
            }

            callWithCatch(() => { throw new Error("test"); })
        "#
        ),
        JsValue::from("object caught")
    );
}

#[test]
fn test_custom_error_class_inherits_message() {
    // Custom error class should properly inherit message from Error
    assert_eq!(
        eval(
            r#"
            class CustomError extends Error {
                code: number;
                constructor(message: string, code: number) {
                    super(message);
                    this.name = "CustomError";
                    this.code = code;
                }
            }

            const err = new CustomError("test message", 42);
            err.message
        "#
        ),
        JsValue::from("test message")
    );
}

#[test]
fn test_custom_error_class_tostring() {
    // Custom error class toString should work
    assert_eq!(
        eval(
            r#"
            class CustomError extends Error {
                constructor(message: string) {
                    super(message);
                    this.name = "CustomError";
                }
            }

            new CustomError("my error").toString()
        "#
        ),
        JsValue::from("CustomError: my error")
    );
}

#[test]
fn test_error_instanceof_hierarchy() {
    // TypeError is not RangeError
    assert_eq!(
        eval("new TypeError('test') instanceof RangeError"),
        JsValue::Boolean(false)
    );
    // RangeError is not TypeError
    assert_eq!(
        eval("new RangeError('test') instanceof TypeError"),
        JsValue::Boolean(false)
    );
    // Both are Error instances
    assert_eq!(
        eval("new TypeError('test') instanceof Error"),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval("new RangeError('test') instanceof Error"),
        JsValue::Boolean(true)
    );
}

// Tests for runtime errors being proper objects (not strings)
#[test]
fn test_runtime_typeerror_is_object() {
    // Calling null as function should throw TypeError that is an object
    assert_eq!(
        eval(
            r#"
            let caught: any;
            try {
                (null as any)();
            } catch (e) {
                caught = e;
            }
            typeof caught
        "#
        ),
        JsValue::from("object")
    );
}

#[test]
fn test_runtime_typeerror_instanceof_error() {
    // Runtime TypeError should be instanceof Error
    assert_eq!(
        eval(
            r#"
            let isError: boolean = false;
            try {
                (null as any)();
            } catch (e) {
                isError = e instanceof Error;
            }
            isError
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_runtime_typeerror_instanceof_typeerror() {
    // Runtime TypeError should be instanceof TypeError
    assert_eq!(
        eval(
            r#"
            let isTypeError: boolean = false;
            try {
                (null as any)();
            } catch (e) {
                isTypeError = e instanceof TypeError;
            }
            isTypeError
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_runtime_typeerror_has_name() {
    // Runtime TypeError should have name property
    assert_eq!(
        eval(
            r#"
            let name: string = "";
            try {
                (null as any)();
            } catch (e) {
                name = e.name;
            }
            name
        "#
        ),
        JsValue::from("TypeError")
    );
}

#[test]
fn test_runtime_typeerror_has_message() {
    // Runtime TypeError should have a message property
    assert_eq!(
        eval(
            r#"
            let hasMessage: boolean = false;
            try {
                (null as any)();
            } catch (e) {
                hasMessage = typeof e.message === "string" && e.message.length > 0;
            }
            hasMessage
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_runtime_referenceerror_is_object() {
    // ReferenceError for undefined variable should be an object
    assert_eq!(
        eval(
            r#"
            let caught: any;
            try {
                undefinedVariable;
            } catch (e) {
                caught = e;
            }
            typeof caught
        "#
        ),
        JsValue::from("object")
    );
}

#[test]
fn test_runtime_referenceerror_instanceof_error() {
    // Runtime ReferenceError should be instanceof Error
    assert_eq!(
        eval(
            r#"
            let isError: boolean = false;
            try {
                undefinedVariable;
            } catch (e) {
                isError = e instanceof Error;
            }
            isError
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_runtime_referenceerror_instanceof_referenceerror() {
    // Runtime ReferenceError should be instanceof ReferenceError
    assert_eq!(
        eval(
            r#"
            let isReferenceError: boolean = false;
            try {
                undefinedVariable;
            } catch (e) {
                isReferenceError = e instanceof ReferenceError;
            }
            isReferenceError
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_runtime_referenceerror_has_name() {
    // Runtime ReferenceError should have name property
    assert_eq!(
        eval(
            r#"
            let name: string = "";
            try {
                undefinedVariable;
            } catch (e) {
                name = e.name;
            }
            name
        "#
        ),
        JsValue::from("ReferenceError")
    );
}

#[test]
fn test_runtime_rangeerror_is_object() {
    // RangeError from toFixed should be an object
    assert_eq!(
        eval(
            r#"
            let caught: any;
            try {
                (1.5).toFixed(-1);
            } catch (e) {
                caught = e;
            }
            typeof caught
        "#
        ),
        JsValue::from("object")
    );
}

#[test]
fn test_runtime_rangeerror_instanceof_rangeerror() {
    // Runtime RangeError should be instanceof RangeError
    assert_eq!(
        eval(
            r#"
            let isRangeError: boolean = false;
            try {
                (1.5).toFixed(-1);
            } catch (e) {
                isRangeError = e instanceof RangeError;
            }
            isRangeError
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_runtime_error_tostring() {
    // Runtime errors should have working toString()
    assert_eq!(
        eval(
            r#"
            let str: string = "";
            try {
                (null as any)();
            } catch (e) {
                str = e.toString();
            }
            str.startsWith("TypeError:")
        "#
        ),
        JsValue::Boolean(true)
    );
}
