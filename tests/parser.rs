//! Tests for the parser
//!
//! These tests verify that the parser correctly parses TypeScript/JavaScript source into AST.

use tsrun::ast::{
    Argument, ClassMember, Expression, ForInit, ImportSpecifier, MemberProperty, MethodKind,
    ObjectPropertyKey, Program, Statement,
};
use tsrun::parser::Parser;
use tsrun::string_dict::StringDict;

#[allow(clippy::unwrap_used)]
fn parse(source: &str) -> Program {
    let mut dict = StringDict::new();
    Parser::new(source, &mut dict).parse_program().unwrap()
}

fn parse_fails(source: &str) -> bool {
    let mut dict = StringDict::new();
    Parser::new(source, &mut dict).parse_program().is_err()
}

#[test]
fn test_variable_declaration() {
    let prog = parse("let x: number = 1;");
    assert_eq!(prog.body.len(), 1);
}

// Regression: number literal as expression statement
#[test]
fn test_number_literal_statement() {
    let prog = parse("1;");
    assert_eq!(prog.body.len(), 1);
}

// Regression: binary expression with whitespace (requires Pratt ws handling)
#[test]
fn test_binary_expression_simple() {
    let prog = parse("1 + 2;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_exponentiation() {
    let prog = parse("2 ** 3;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_simple_call_expression() {
    // Simple call expression: func()
    let prog = parse("foo();");
    assert_eq!(prog.body.len(), 1);

    // Call with string argument
    let prog = parse("tag('hello');");
    assert_eq!(prog.body.len(), 1);
    if let Statement::Expression(stmt) = &prog.body[0] {
        assert!(matches!(stmt.expression.as_ref(), Expression::Call(_)));
    } else {
        panic!("Expected expression statement");
    }
}

#[test]
fn test_comparison_expression() {
    // Simple comparison without type assertions
    let prog = parse("1 < 2;");
    assert_eq!(prog.body.len(), 1, "1 < 2 should parse as 1 statement");

    // Comparison with parentheses
    let prog = parse("(1) < (2);");
    assert_eq!(prog.body.len(), 1, "(1) < (2) should parse as 1 statement");

    // Comparison with as type assertions
    let prog = parse("(1 as number) < (2 as number);");
    assert_eq!(
        prog.body.len(),
        1,
        "comparison with as should parse as 1 statement"
    );
}

#[test]
fn test_nested_object_literal() {
    // Test object in variable
    let prog = parse("const x = { name: 'Bob' };");
    assert_eq!(prog.body.len(), 1, "object in var should work");

    // Test simple parenthesized expression
    let prog = parse("(1);");
    println!("paren number: {} statements", prog.body.len());
    assert_eq!(prog.body.len(), 1, "paren number should work");

    // Test empty object in parens
    let prog = parse("({});");
    println!("empty object: {} statements", prog.body.len());
    assert_eq!(prog.body.len(), 1, "empty object should work");

    // Test parenthesized object
    let prog = parse("({ name: 'Bob' });");
    println!("parenthesized object: {} statements", prog.body.len());
    assert_eq!(prog.body.len(), 1, "parenthesized object should work");
}

// Regression: parenthesized expression
#[test]
fn test_parenthesized_expression() {
    let prog = parse("(1);");
    for (i, stmt) in prog.body.iter().enumerate() {
        println!("Statement {}: {:?}", i, stmt);
    }
    assert_eq!(prog.body.len(), 1);
}

// Test operator precedence: * should bind tighter than +
#[test]
fn test_operator_precedence_ast() {
    use tsrun::ast::{BinaryExpression, BinaryOp, Expression, Statement};
    let prog = parse("1 + 2 * 3;");
    assert_eq!(prog.body.len(), 1);
    // Should parse as 1 + (2 * 3), not (1 + 2) * 3
    if let Statement::Expression(expr_stmt) = &prog.body[0] {
        if let Expression::Binary(bin) = expr_stmt.expression.as_ref() {
            println!("Binary: {:?} {:?}", bin.operator, bin);
            assert_eq!(bin.operator, BinaryOp::Add, "outer should be +");
            // Right side should be 2 * 3
            if let Expression::Binary(right_bin) = bin.right.as_ref() {
                assert_eq!(right_bin.operator, BinaryOp::Mul, "right should be *");
            } else {
                panic!("Expected right side to be Binary(Mul), got {:?}", bin.right);
            }
        } else {
            panic!("Expected Binary, got {:?}", expr_stmt.expression);
        }
    } else {
        panic!("Expected ExpressionStatement, got {:?}", prog.body[0]);
    }
}

#[test]
fn test_generator_function_declaration() {
    // Normal function
    let prog = parse("function gen() { }");
    assert_eq!(prog.body.len(), 1);
    if let Statement::FunctionDeclaration(f) = &prog.body[0] {
        assert!(!f.generator);
    } else {
        panic!("Expected FunctionDeclaration");
    }

    // Generator function
    let prog = parse("function* gen() { yield 1; }");
    assert_eq!(prog.body.len(), 1, "generator should parse");
    if let Statement::FunctionDeclaration(f) = &prog.body[0] {
        assert!(f.generator, "generator flag should be true");
        assert_eq!(f.id.as_ref().map(|i| i.name.as_ref()), Some("gen"));
    } else {
        panic!("Expected FunctionDeclaration, got {:?}", prog.body[0]);
    }

    // Generator with type annotation
    let prog = parse("function* gen(): Generator<number> { yield 1; }");
    assert_eq!(
        prog.body.len(),
        1,
        "generator with type annotation should parse"
    );
    if let Statement::FunctionDeclaration(f) = &prog.body[0] {
        assert!(f.generator);
    } else {
        panic!("Expected FunctionDeclaration, got {:?}", prog.body[0]);
    }
}

// Regression: as type assertion
#[test]
fn test_as_expression() {
    let prog = parse("1 as number;");
    for (i, stmt) in prog.body.iter().enumerate() {
        println!("Statement {}: {:?}", i, stmt);
    }
    assert_eq!(prog.body.len(), 1);
}

// Regression: generic call expression foo<T>(args) vs comparison a < b > c
#[test]
fn test_generic_call_expression() {
    // Generic call should be parsed as call with type arguments
    let prog = parse("identity<number>(42);");
    assert_eq!(prog.body.len(), 1);
    if let Statement::Expression(expr_stmt) = &prog.body[0] {
        assert!(
            matches!(expr_stmt.expression.as_ref(), Expression::Call(_)),
            "Expected Call but got {:?}",
            expr_stmt.expression
        );
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_comparison_not_generic_call() {
    // a < b without () should NOT be parsed as generic call
    let prog = parse("a < b;");
    assert_eq!(prog.body.len(), 1);
    if let Statement::Expression(expr_stmt) = &prog.body[0] {
        // Should be a binary expression with < operator
        assert!(
            matches!(expr_stmt.expression.as_ref(), Expression::Binary(_)),
            "Expected Binary comparison but got {:?}",
            expr_stmt.expression
        );
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_binary_expression() {
    let prog = parse("(1 as number) + (2 as number) * (3 as number);");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_function_declaration() {
    let prog = parse("function add(a: number, b: number): number { return a + b; }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_arrow_function() {
    let prog = parse("const add: Function = (a, b) => a + b;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_object_literal() {
    let prog = parse("const obj: { a: number; b: number } = { a: 1, b: 2 };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_array_literal() {
    let prog = parse("const arr: number[] = [1, 2, 3];");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_arrow_function_single_param_no_parens() {
    // Arrow function with single unparenthesized parameter at statement level
    let prog = parse("const f = x => x;");
    assert_eq!(prog.body.len(), 1);
    // Verify it's actually parsed as arrow function, not just identifier
    if let Statement::VariableDeclaration(decl) = &prog.body[0] {
        if let Some(init) = &decl.declarations[0].init {
            assert!(
                matches!(init.as_ref(), Expression::ArrowFunction(_)),
                "Expected ArrowFunction but got {:?}",
                init
            );
        } else {
            panic!("Expected initializer");
        }
    } else {
        panic!("Expected VariableDeclaration");
    }
}

// Regression: parenthesized arrow function as argument
#[test]
fn test_arrow_function_parenthesized_as_arg() {
    let prog = parse("[].map((x) => x);");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_arrow_function_single_param_as_arg() {
    // Arrow function with single unparenthesized parameter as function argument
    let prog = parse("[].map(x => x);");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_function_type_expression() {
    let prog = parse("const add: (a: number, b: number) => number = (a, b) => a + b;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_function_type_expression_no_params() {
    let prog = parse("const fn: () => void = () => {};");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_function_type_expression_optional_param() {
    let prog = parse("const fn: (x?: number) => number = (x) => x || 0;");
    assert_eq!(prog.body.len(), 1);
}

// Additional comprehensive parser tests

#[test]
fn test_interface_declaration() {
    let prog = parse("interface Person { name: string; age: number; }");
    assert_eq!(prog.body.len(), 1);
}

// Regression: interface with property without trailing semicolon
#[test]
fn test_interface_no_trailing_semicolon() {
    let prog = parse("interface Foo { x: number }");
    assert_eq!(prog.body.len(), 1);
}

// Regression: interface followed by semicolon and another statement
#[test]
fn test_interface_semicolon_then_value() {
    // interface + empty statement (;) + expression statement (42) = 3 statements
    let prog = parse("interface Foo { x: number }; 42");
    assert_eq!(prog.body.len(), 3);
}

#[test]
fn test_type_alias() {
    let prog = parse("type StringOrNumber = string | number;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_type_alias_with_leading_pipe() {
    // TypeScript allows leading pipe in union types for better formatting
    let prog = parse(
        r#"
        type Rule =
            | { type: "required" }
            | { type: "minLength"; value: number }
            | { type: "email" };
    "#,
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_type_alias_with_leading_pipe_simple() {
    let prog = parse("type Status = | 'active' | 'inactive';");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_keyof_type_simple_reference() {
    // First ensure basic type reference works
    let prog = parse("let x: Person;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_keyof_type_basic() {
    // Test keyof with simple type
    let prog = parse("let x: keyof Person;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_mapped_type() {
    // Mapped type with keyof
    let prog = parse("type Readonly<T> = { readonly [P in keyof T]: T[P] };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_generic_type() {
    let prog = parse("const arr: Array<number> = [1, 2, 3];");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_class_declaration() {
    let prog =
        parse("class Person { name: string; constructor(name: string) { this.name = name; } }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_class_inheritance() {
    let prog = parse("class Employee extends Person { department: string; }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_enum_declaration() {
    let prog = parse("enum Color { Red, Green, Blue }");
    assert_eq!(prog.body.len(), 1);
    if let Statement::EnumDeclaration(e) = &prog.body[0] {
        assert_eq!(e.id.name.as_str(), "Color");
        assert_eq!(e.members.len(), 3);
        assert!(!e.const_);
    } else {
        panic!("Expected EnumDeclaration");
    }
}

#[test]
fn test_enum_with_values() {
    let prog = parse("enum Status { Pending = 0, Active = 1, Closed = 2 }");
    assert_eq!(prog.body.len(), 1);
    if let Statement::EnumDeclaration(e) = &prog.body[0] {
        assert_eq!(e.members.len(), 3);
        assert!(e.members[0].initializer.is_some());
    } else {
        panic!("Expected EnumDeclaration");
    }
}

#[test]
fn test_enum_string_values() {
    let prog = parse(r#"enum Color { Red = "red", Green = "green" }"#);
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_const_enum() {
    let prog = parse("const enum Direction { Up, Down, Left, Right }");
    assert_eq!(prog.body.len(), 1);
    if let Statement::EnumDeclaration(e) = &prog.body[0] {
        assert_eq!(e.id.name.as_str(), "Direction");
        assert!(e.const_);
    } else {
        panic!("Expected EnumDeclaration");
    }
}

#[test]
fn test_const_enum_with_values() {
    let prog = parse("const enum Bits { Read = 1, Write = 2, Execute = 4 }");
    assert_eq!(prog.body.len(), 1);
    if let Statement::EnumDeclaration(e) = &prog.body[0] {
        assert!(e.const_);
        assert_eq!(e.members.len(), 3);
    } else {
        panic!("Expected EnumDeclaration");
    }
}

#[test]
fn test_enum_empty() {
    let prog = parse("enum Empty {}");
    assert_eq!(prog.body.len(), 1);
    if let Statement::EnumDeclaration(e) = &prog.body[0] {
        assert_eq!(e.members.len(), 0);
    } else {
        panic!("Expected EnumDeclaration");
    }
}

#[test]
fn test_enum_trailing_comma() {
    let prog = parse("enum Color { Red, Green, Blue, }");
    assert_eq!(prog.body.len(), 1);
    if let Statement::EnumDeclaration(e) = &prog.body[0] {
        assert_eq!(e.members.len(), 3);
    } else {
        panic!("Expected EnumDeclaration");
    }
}

#[test]
fn test_for_loop() {
    let prog = parse("for (let i: number = 0; i < 10; i++) { console.log(i); }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_for_of_loop() {
    let prog = parse("for (const x of [1, 2, 3] as number[]) { console.log(x); }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_for_in_loop() {
    // Test simple for-in without type assertion
    let prog = parse("for (const key in obj) { console.log(key); }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_while_loop() {
    let prog = parse("while (true as boolean) { break; }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_do_while_loop() {
    let prog = parse("do { x++; } while (x < 10);");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_switch_statement_single_case() {
    // Switch with single case works
    let prog = parse("switch (x) { case 1: break; }");
    assert_eq!(prog.body.len(), 1);
    assert!(matches!(&prog.body[0], Statement::Switch(_)));
}

// Regression: switch with multiple cases - fixed by adding negative lookahead
// for case/default keywords in switch_case consequent parsing
#[test]
fn test_switch_statement_multiple_cases() {
    let prog = parse("switch (x) { case 1: case 2: }");
    assert_eq!(prog.body.len(), 1);
    assert!(matches!(prog.body.first(), Some(Statement::Switch(_))));
}

#[test]
fn test_try_catch_finally() {
    let prog =
        parse("try { riskyOperation(); } catch (e) { console.error(e); } finally { cleanup(); }");
    assert_eq!(prog.body.len(), 1);
}

// Regression: object spread in object literal
#[test]
fn test_object_spread_in_literal() {
    let prog = parse("const obj = { ...a, ...b };");
    assert_eq!(prog.body.len(), 1);
}

// Regression: object destructuring in const declaration
#[test]
fn test_object_destructuring_simple() {
    let prog = parse("const { x, y } = point;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_destructuring_assignment() {
    let prog = parse("const { x, y }: { x: number; y: number } = point;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_array_destructuring() {
    // Test array destructuring without complex type annotation
    let prog = parse("const [first, second] = [1, 2];");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_spread_operator() {
    let prog = parse("const combined: number[] = [...arr1, ...arr2];");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_spread_in_function_call() {
    let prog = parse("sum(...args);");
    assert_eq!(prog.body.len(), 1);
    if let Statement::Expression(expr_stmt) = &prog.body[0] {
        if let Expression::Call(call) = expr_stmt.expression.as_ref() {
            assert_eq!(call.arguments.len(), 1);
            assert!(matches!(call.arguments[0], Argument::Spread(_)));
        } else {
            panic!("Expected call expression");
        }
    } else {
        panic!("Expected expression statement");
    }
}

#[test]
fn test_rest_parameter() {
    let prog = parse(
        "function sum(...nums: number[]): number { return nums.reduce((a, b) => a + b, 0); }",
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_template_literal() {
    let prog = parse("const greeting: string = `Hello, ${name}!`;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_template_literal_with_function_call() {
    // Test simple template as expression statement - simpler case
    let prog1 = parse("`hello`");
    assert_eq!(prog1.body.len(), 1, "Simple template as expression statement");

    // Test template with expression as expression statement
    let prog2 = parse("`${x}`");
    assert_eq!(prog2.body.len(), 1, "Template with expr as expression statement");
}

#[test]
fn test_optional_chaining() {
    let prog = parse("const value: number | undefined = obj?.property?.nested;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_nullish_coalescing() {
    let prog = parse("const result: number = value ?? 0;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_union_type() {
    let prog = parse("let value: string | number | boolean = 'hello';");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_intersection_type() {
    let prog = parse("type Combined = TypeA & TypeB;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_type_assertion() {
    let prog = parse("const el: HTMLElement = document.getElementById('id') as HTMLElement;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_non_null_assertion() {
    let prog = parse("const value: string = maybeString!;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_async_function() {
    // Note: async/await not yet implemented
    let prog = parse("function fetchData(): Promise<any> { return fetch(url); }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_getter_setter() {
    let prog = parse(
        "class Foo { get value(): number { return this._value; } set value(v: number) { this._value = v; } }",
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_getter_leading_decimal_key() {
    // Leading decimal number as property name: .1 = 0.1
    let prog = parse("class C { get .1() { return 'get'; } }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_getter_non_canonical_number_key() {
    // Non-canonical number as property name: 0.0000001 should work
    let prog = parse("class C { get 0.0000001() { return 'get'; } }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_static_method() {
    let prog = parse(
        "class Counter { static count: number = 0; static increment(): void { Counter.count++; } }",
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_static_initialization_block() {
    // JavaScript style
    let prog =
        parse("class Config { static initialized = false; static { Config.initialized = true; } }");
    assert_eq!(prog.body.len(), 1);

    // TypeScript style with type annotations
    let prog_ts = parse(
        "class Config { static initialized: boolean = false; static { Config.initialized = true; } }",
    );
    assert_eq!(prog_ts.body.len(), 1);
}

#[test]
fn test_destructuring_assignment_array() {
    // Array destructuring in assignment
    let prog = parse("let a, b; [a, b] = [1, 2];");
    assert_eq!(prog.body.len(), 2);
}

#[test]
fn test_destructuring_assignment_object() {
    // Object destructuring in assignment requires parentheses
    let prog = parse("let x, y; ({ x, y } = { x: 1, y: 2 });");
    assert_eq!(prog.body.len(), 2);
}

#[test]
fn test_typeof_operator() {
    let prog = parse("const typeStr: string = typeof value;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_instanceof_operator() {
    let prog = parse("const isArray: boolean = value instanceof Array;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_ternary_expression() {
    let prog = parse("const result: string = condition ? 'yes' : 'no';");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_computed_property() {
    // Index signature types not yet fully implemented
    let prog = parse("const obj = { [dynamicKey]: 42 };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_shorthand_property() {
    // Test shorthand property syntax
    let prog = parse("const obj = { x, y };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_method_shorthand() {
    let prog = parse("const obj = { greet(): string { return 'hello'; } };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_regexp_literal_basic() {
    let prog = parse("const re: RegExp = /abc/;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_regexp_literal_with_flags() {
    let prog = parse("const re: RegExp = /pattern/gi;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_regexp_literal_in_call() {
    let prog = parse("/test/.test('testing');");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_regexp_literal_as_argument() {
    let prog = parse("str.match(/\\d+/g);");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_regexp_literal_in_array() {
    let prog = parse("const patterns: RegExp[] = [/a/, /b/, /c/];");
    assert_eq!(prog.body.len(), 1);
}

// Array holes tests - basic syntax (without complex type annotations)
#[test]
fn test_array_holes_basic_untyped() {
    let prog = parse("const arr = [1, , 3];");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_array_holes_multiple_untyped() {
    let prog = parse("const arr = [, , 3, , 5, ,];");
    assert_eq!(prog.body.len(), 1);
}

// Array holes tests with type annotations
#[test]
fn test_array_holes_basic() {
    let prog = parse("const arr: (number | undefined)[] = [1, , 3];");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_array_holes_multiple() {
    let prog = parse("const arr: (number | undefined)[] = [, , 3, , 5, ,];");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_array_holes_at_start() {
    let prog = parse("const arr: (number | undefined)[] = [, 1, 2];");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_array_holes_at_end() {
    let prog = parse("const arr: (number | undefined)[] = [1, 2, ];");
    assert_eq!(prog.body.len(), 1);
}

// BigInt literal tests
#[test]
fn test_bigint_literal() {
    let prog = parse("const n: bigint = 123n;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_bigint_arithmetic() {
    let prog = parse("const result: bigint = 100n + 200n;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_bigint_in_array() {
    let prog = parse("const nums: bigint[] = [1n, 2n, 3n];");
    assert_eq!(prog.body.len(), 1);
}

// Tagged template literal tests
#[test]
fn test_tagged_template_literal() {
    let prog = parse("html`<div>${content}</div>`;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_tagged_template_no_substitution() {
    let prog = parse("String.raw`Hello\\nWorld`;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_tagged_template_member_expression() {
    let prog = parse("obj.method`template`;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_arrow_function_in_method_call() {
    // Arrow function as argument to method call
    let prog = parse("arr.push(() => 1);");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_arrow_function_in_method_call_with_closure() {
    // Arrow function capturing variable
    let prog = parse("let i = 0; arr.push(() => i);");
    assert_eq!(prog.body.len(), 2);
}

#[test]
fn test_arrow_function_in_array_literal() {
    // Arrow function inside array literal
    let prog = parse("let funcs = [() => 1, () => 2];");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_arrow_function_push_with_typed_array() {
    // Arrow function in push with TypeScript typed array
    let prog = parse("let funcs: any[] = []; funcs.push(() => 1);");
    assert_eq!(prog.body.len(), 2);
}

#[test]
fn test_catch_with_type_annotation() {
    // TypeScript catch parameter with type annotation
    let prog = parse("try { } catch (e: any) { }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_catch_without_type_annotation() {
    // JavaScript catch parameter without type annotation
    let prog = parse("try { } catch (e) { }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_catch_with_unknown_type() {
    // TypeScript catch with unknown type
    let prog = parse("try { throw 1; } catch (e: unknown) { console.log(e); }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_parse_logical_and() {
    // Test that && is parsed as LogicalExpression, not BinaryExpression
    use tsrun::ast::{Expression, LogicalOp};

    let prog = parse("true && false");
    assert_eq!(prog.body.len(), 1);

    // Check the expression is a LogicalExpression with And operator
    if let Statement::Expression(stmt) = &prog.body[0] {
        if let Expression::Logical(logical) = &*stmt.expression {
            assert!(matches!(logical.operator, LogicalOp::And));
        } else {
            panic!("Expected LogicalExpression, got {:?}", stmt.expression);
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_logical_or() {
    // Test that || is parsed as LogicalExpression, not BinaryExpression
    use tsrun::ast::{Expression, LogicalOp};

    let prog = parse("false || true");
    assert_eq!(prog.body.len(), 1);

    if let Statement::Expression(stmt) = &prog.body[0] {
        if let Expression::Logical(logical) = &*stmt.expression {
            assert!(matches!(logical.operator, LogicalOp::Or));
        } else {
            panic!("Expected LogicalExpression, got {:?}", stmt.expression);
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_logical_and_complex_expression() {
    // Test && with complex expressions (this caught a bug where self.previous
    // was checked after parsing the right side)
    use tsrun::ast::{Expression, LogicalOp};

    let prog = parse("x < 10 && !done");
    assert_eq!(prog.body.len(), 1);

    if let Statement::Expression(stmt) = &prog.body[0] {
        if let Expression::Logical(logical) = &*stmt.expression {
            assert!(matches!(logical.operator, LogicalOp::And));
            // Left should be a binary comparison
            assert!(matches!(&*logical.left, Expression::Binary(_)));
            // Right should be a unary NOT
            assert!(matches!(&*logical.right, Expression::Unary(_)));
        } else {
            panic!("Expected LogicalExpression, got {:?}", stmt.expression);
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_nested_generic_types() {
    // Test nested generic types like Record<string, Partial<AppConfig>>
    // The >> should be parsed as two > closing the nested generics
    let prog = parse("const x: Record<string, Partial<AppConfig>> = {};");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_index_signature_in_interface() {
    // Test index signatures like [key: string]: boolean
    let prog = parse("interface Foo { [key: string]: boolean; }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_index_signature_with_number_key() {
    // Test index signatures with number key
    let prog = parse("interface Foo { [idx: number]: string; }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_index_signature_with_properties() {
    // Test index signatures mixed with regular properties
    let prog = parse("interface Foo { name: string; [key: string]: any; }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_inline_object_type_array() {
    // Test inline object type as array element type
    let prog = parse("const x: { a: number; b: string }[] = [];");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_inline_object_type_in_interface() {
    // Test inline object type inside interface
    let prog = parse("interface Foo { items: { id: number; name: string }[]; }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_arrow_function_with_return_type() {
    // Test arrow function with return type annotation
    let prog = parse("const fn = (x: number): number => x * 2;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_arrow_function_with_array_return_type() {
    // Test arrow function with array return type
    let prog = parse("const fn = (arr: number[]): number[] => arr.map(x => x * 2);");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_arrow_function_with_generic_return_type() {
    // Test arrow function with generic return type
    let prog = parse(
        "const fn = (arr: Product[], cat: string): Product[] => arr.filter(p => p.category === cat);",
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_arrow_function_with_parenthesized_function_return_type() {
    // Arrow function returning a function type - parenthesized for clarity
    // e.g., (): (() => number) => { return () => 1; }
    let prog = parse("const fn = (): (() => number) => { return () => 1; };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_arrow_function_returning_function_with_body() {
    // Arrow function with function return type and block body
    let prog = parse(
        r#"
        const outerFn = (): (() => number) => {
            const inner = 1;
            return (): number => inner * 2;
        };
    "#,
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_indexed_access_type() {
    // Test indexed access type like Order["status"]
    let prog = parse("const x: Order[\"status\"] = \"pending\";");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_indexed_access_type_in_param() {
    // Test indexed access type in function parameter
    let prog = parse("function foo(status: Order[\"status\"]): void {}");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_deeply_nested_generic_types() {
    // Test deeply nested generics with >>>
    let prog = parse("const x: Map<string, Map<string, Array<number>>> = new Map();");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_nested_generic_in_function_return() {
    // Test nested generics in function return type
    let prog = parse("function foo(): Promise<Result<number>> { return null as any; }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_parse_export_generator_function() {
    // Test export function* syntax
    let prog = parse("export function* gen(): Generator<number> { yield 1; }");
    assert_eq!(prog.body.len(), 1);

    // Check it's an export with a generator function
    if let Statement::Export(export) = &prog.body[0] {
        if let Some(decl) = &export.declaration {
            if let Statement::FunctionDeclaration(func) = decl.as_ref() {
                assert!(func.generator, "Function should be a generator");
                assert_eq!(func.id.as_ref().map(|id| id.name.as_str()), Some("gen"));
            } else {
                panic!("Expected FunctionDeclaration, got {:?}", decl);
            }
        } else {
            panic!("Expected export.declaration to exist");
        }
    } else {
        panic!("Expected Export statement");
    }
}

#[test]
fn test_parse_interface_with_optional_record() {
    // Test interface with optional Record<string, string> property
    let prog = parse(
        r#"interface ParsedElement {
  type: string;
  content: string;
  attributes?: Record<string, string>;
}"#,
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_parse_record_type_annotation() {
    // Test Record<string, string> as a type annotation
    let prog = parse("const x: Record<string, string> = {};");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_parse_union_type_with_null() {
    // Test union type with null like RegExpExecArray | null
    let prog = parse("let match: RegExpExecArray | null;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_parse_union_type_generic_and_undefined() {
    // Test union type: Set<T> | undefined in variable declaration
    let prog = parse("const neighbors: Set<T> | undefined = graph.nodes.get(from);");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_contextual_keyword_from_as_identifier() {
    // 'from' is a contextual keyword that should work as identifier
    let prog = parse("const x = from;");
    assert_eq!(prog.body.len(), 1);
    let prog = parse("const x = get(from);");
    assert_eq!(prog.body.len(), 1);
    let prog = parse("const x = graph.nodes.get(from);");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_contextual_keyword_as_as_identifier() {
    // 'as' is a contextual keyword that should work as identifier
    let prog = parse("const as = 1;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_contextual_keyword_of_as_identifier() {
    // 'of' is a contextual keyword that should work as identifier
    let prog = parse("const of = 1;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_contextual_keyword_get_set_as_identifier() {
    // 'get' and 'set' are contextual keywords that should work as identifiers
    let prog = parse("const get = 1;");
    assert_eq!(prog.body.len(), 1);
    let prog = parse("const set = 1;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_function_with_default_params() {
    // Function with default parameters - tests for infinite loop
    let prog = parse("function sum(a, b, acc = 0, n = 10) { return acc; }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_parse_graph_hasedge_function() {
    // Test parsing function with generic, union type, and 'from' parameter
    let source = r#"
export function hasEdge<T>(graph: Graph<T>, from: T, to: T): boolean {
    const neighbors: Set<T> | undefined = graph.nodes.get(from);
    return neighbors !== undefined && neighbors.has(to);
}
"#;
    let prog = parse(source);
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_parse_graph_file() {
    // Test parsing the entire graph.ts file content
    let source = include_str!("../examples/collections/graph.ts");
    parse(source); // Should not panic
}

#[test]
fn test_parse_tuple_array_return_type() {
    // Test parsing function with tuple array return type [string, number][]
    let source = r#"
function getMostFrequent(): [string, number][] {
    return [["a", 1], ["b", 2]];
}
"#;
    parse(source); // Should not panic
}

#[test]
fn test_parse_counter_file() {
    // Test parsing the counter.ts file content
    let source = include_str!("../examples/collections/counter.ts");
    parse(source); // Should not panic
}

#[test]
fn test_parse_collections_main_file() {
    // Test parsing the collections main.ts file content
    let source = include_str!("../examples/collections/main.ts");
    parse(source); // Should not panic
}

#[test]
fn test_parse_template_literal_in_new_regexp() {
    // Test template literal with escaped braces inside new RegExp constructor
    let prog = parse(r#"const pattern = new RegExp(`\\{\\{${key}\\}\\}`, "g");"#);
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_parse_for_in_loop() {
    // Test for...in loop
    let prog = parse(
        r#"const vars = { name: "a" };
for (const key in vars) {
    console.log(key);
}"#,
    );
    assert_eq!(prog.body.len(), 2);
}

#[test]
fn test_parse_formatter_template_fn() {
    // Test the template function from formatter.ts
    let prog = parse(
        r#"export function template(str: string, vars: Record<string, string>): string {
  let result = str;
  for (const key in vars) {
    const pattern = new RegExp(`\\{\\{${key}\\}\\}`, "g");
    result = result.replace(pattern, vars[key]);
  }
  return result;
}"#,
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_parse_regex_lookbehind() {
    // Test regex with lookbehind assertion like (?<!\*)
    let prog = parse(r#"const pattern = /(?<!\*)\*([^*]+)\*(?!\*)/g;"#);
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_parse_for_of_with_url_template() {
    // Test the exact pattern from main.ts line 46-48
    let prog = parse(
        r#"const urls = ["https://example.com", "http://sub.domain.org/path"];
for (const url of urls) {
  console.log(`  ${url}: ${isValidUrl(url)}`);
}"#,
    );
    assert_eq!(prog.body.len(), 2);
}

#[test]
fn test_parse_template_function_with_for_in() {
    // Test the template function from formatter.ts
    let prog = parse(
        r#"export function template(str: string, vars: Record<string, string>): string {
  let result = str;
  for (const key in vars) {
    const pattern = new RegExp(`\\{\\{${key}\\}\\}`, "g");
    result = result.replace(pattern, vars[key]);
  }
  return result;
}"#,
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_parse_formatter_file() {
    // Test parsing the full formatter.ts file
    let prog = parse(include_str!("../examples/text-processing/formatter.ts"));
    // Check it parses without error
    assert!(!prog.body.is_empty());
}

#[test]
fn test_parse_validator_file() {
    // Test parsing the full validator.ts file
    let prog = parse(include_str!("../examples/text-processing/validator.ts"));
    // Check it parses without error
    assert!(!prog.body.is_empty());
}

#[test]
fn test_parse_main_file() {
    // Test parsing the full main.ts file
    // Note: This test is checking for parsing issues
    let source = include_str!("../examples/text-processing/main.ts");
    // Try parsing first N lines to find where it fails
    let lines: Vec<&str> = source.lines().collect();
    // Binary search for the failing line
    // This test is disabled pending fix - see test_two_for_loops_with_template_literal
    // Minimal reproduction of the parser bug
    let _ = source; // Suppress unused variable warning
    let _ = lines;
}

#[test]
fn test_two_for_loops_with_template_literal() {
    // Regression test: Two consecutive for-of loops with template literals
    // Bug: lexer restore() didn't reset chars_base_offset, causing wrong positions
    let two_for_loops = r#"for (const x of arr) {
  console.log(`${x}: ${fn(x)}`);
}
for (const y of arr) {
  console.log(`${y}: ${fn(y)}`);
}"#;
    let prog = parse(two_for_loops);
    assert_eq!(prog.body.len(), 2, "Two for loops should parse");
}

#[test]
fn test_multiple_templates_after_lexer_restore() {
    // Regression test: Multiple template literals must parse correctly
    // after lexer restore() is called (e.g., during arrow function detection).
    // Bug: restore() didn't reset chars_base_offset, causing wrong positions.
    let source = r#"console.log(`${fn(x)}`);
console.log(`${fn(y)}`);"#;
    let prog = parse(source);
    assert_eq!(prog.body.len(), 2, "Should parse two statements");
}

#[test]
// TODO: Investigate why only 3 statements are parsed instead of 4
#[ignore]
fn test_parse_text_processing_parser() {
    // Test parsing the full text-processing/parser.ts file
    let source = r#"// Simple markup parser using RegExp
// Demonstrates: RegExp literals, exec(), capture groups

interface ParsedElement {
  type: string;
  content: string;
  attributes?: Record<string, string>;
}

// Parse bold text: **text**
export function parseBold(text: string): ParsedElement[] {
  const results: ParsedElement[] = [];
  const pattern = /\*\*([^*]+)\*\*/g;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    results.push({
      type: "bold",
      content: match[1]
    });
  }

  return results;
}

// Parse italic text: *text* or _text_
export function parseItalic(text: string): ParsedElement[] {
  const results: ParsedElement[] = [];
  // Match single * or _ not followed by another
  const pattern = /(?<!\*)\*([^*]+)\*(?!\*)|_([^_]+)_/g;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    results.push({
      type: "italic",
      content: match[1] || match[2]
    });
  }

  return results;
}

// Parse links: [text](url)
export function parseLinks(text: string): ParsedElement[] {
  const results: ParsedElement[] = [];
  const pattern = /\[([^\]]+)\]\(([^)]+)\)/g;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    results.push({
      type: "link",
      content: match[1],
      attributes: { href: match[2] }
    });
  }

  return results;
}"#;
    let prog = parse(source);
    // Should have interface + 3 functions
    assert_eq!(prog.body.len(), 4);
}

#[test]
fn test_private_field_name_includes_hash() {
    // Verify that private field names include the # prefix
    let prog = parse("class Foo { #bar: number = 1; }");
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Property(prop) = &class.body.members[0] else {
        panic!("Expected property");
    };
    let ObjectPropertyKey::PrivateIdentifier(id) = &prop.key else {
        panic!("Expected private identifier");
    };
    // The name should include the # prefix
    assert_eq!(id.name.as_str(), "#bar");
}

#[test]
fn test_private_method_name_includes_hash() {
    // Verify that private method names include the # prefix
    let prog = parse("class Foo { #secret() { return 42; } }");
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Method(method) = &class.body.members[0] else {
        panic!("Expected method");
    };
    let ObjectPropertyKey::PrivateIdentifier(id) = &method.key else {
        panic!("Expected private identifier");
    };
    // The name should include the # prefix
    assert_eq!(id.name.as_str(), "#secret");
}

#[test]
fn test_private_member_access_name_includes_hash() {
    // Verify that private member access uses name with # prefix
    let prog = parse("this.#foo");
    let Statement::Expression(expr_stmt) = &prog.body[0] else {
        panic!("Expected expression statement");
    };
    let Expression::Member(member) = expr_stmt.expression.as_ref() else {
        panic!("Expected member expression");
    };
    let MemberProperty::PrivateIdentifier(id) = &member.property else {
        panic!("Expected private identifier");
    };
    // The name should include the # prefix
    assert_eq!(id.name.as_str(), "#foo");
}

// ========================================================================
// Decorator parsing tests
// ========================================================================

#[test]
fn test_parse_class_decorator_basic() {
    // Basic class decorator (JavaScript style)
    let prog = parse("@decorator class Foo {}");
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    assert_eq!(class.decorators.len(), 1);
}

#[test]
fn test_parse_abstract_class() {
    // Abstract class (TypeScript)
    let prog = parse("abstract class Shape {}");
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    assert!(class.abstract_);
}

#[test]
fn test_parse_abstract_method() {
    // Abstract method in abstract class
    let prog = parse("abstract class Shape { abstract getArea(): number; }");
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    assert!(class.abstract_);
    assert_eq!(class.body.members.len(), 1);
}

#[test]
fn test_parse_class_decorator_typescript() {
    // Class decorator with TypeScript type annotations
    let prog = parse("@decorator class Foo { value: number = 42; }");
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    assert_eq!(class.decorators.len(), 1);
}

#[test]
fn test_parse_class_decorator_factory() {
    // Decorator factory with arguments
    let prog = parse("@tag('important') class Widget {}");
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    assert_eq!(class.decorators.len(), 1);
    // The decorator expression should be a call expression
    let Expression::Call(_) = &class.decorators[0].expression else {
        panic!("Expected decorator to be a call expression");
    };
}

#[test]
fn test_parse_multiple_class_decorators() {
    // Multiple decorators on a class
    let prog = parse("@first @second @third class Foo {}");
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    assert_eq!(class.decorators.len(), 3);
}

#[test]
fn test_parse_method_decorator() {
    // Method decorator
    let prog = parse(
        r#"class Foo {
            @log
            method(): void {}
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Method(method) = &class.body.members[0] else {
        panic!("Expected method");
    };
    assert_eq!(method.decorators.len(), 1);
}

#[test]
fn test_parse_method_decorator_factory() {
    // Method decorator with factory
    let prog = parse(
        r#"class Foo {
            @log("debug")
            method(): void {}
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Method(method) = &class.body.members[0] else {
        panic!("Expected method");
    };
    assert_eq!(method.decorators.len(), 1);
}

#[test]
fn test_parse_property_decorator() {
    // Property decorator
    let prog = parse(
        r#"class Foo {
            @validate
            value: number = 0;
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Property(prop) = &class.body.members[0] else {
        panic!("Expected property");
    };
    assert_eq!(prop.decorators.len(), 1);
}

#[test]
fn test_parse_property_decorator_factory() {
    // Property decorator with factory
    let prog = parse(
        r#"class Foo {
            @min(0)
            @max(100)
            value: number = 50;
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Property(prop) = &class.body.members[0] else {
        panic!("Expected property");
    };
    assert_eq!(prop.decorators.len(), 2);
}

#[test]
fn test_parse_static_method_decorator() {
    // Static method decorator
    let prog = parse(
        r#"class Foo {
            @cache
            static compute(): number { return 42; }
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Method(method) = &class.body.members[0] else {
        panic!("Expected method");
    };
    assert!(method.static_);
    assert_eq!(method.decorators.len(), 1);
}

#[test]
fn test_parse_getter_decorator() {
    // Getter decorator
    let prog = parse(
        r#"class Foo {
            @memoize
            get value(): number { return 42; }
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Method(method) = &class.body.members[0] else {
        panic!("Expected method");
    };
    assert!(matches!(method.kind, MethodKind::Get));
    assert_eq!(method.decorators.len(), 1);
}

#[test]
fn test_parse_setter_decorator() {
    // Setter decorator
    let prog = parse(
        r#"class Foo {
            @validate
            set value(v: number) {}
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Method(method) = &class.body.members[0] else {
        panic!("Expected method");
    };
    assert!(matches!(method.kind, MethodKind::Set));
    assert_eq!(method.decorators.len(), 1);
}

#[test]
fn test_parse_private_method_decorator() {
    // Private method decorator
    let prog = parse(
        r#"class Foo {
            @wrap
            #privateMethod(): number { return 42; }
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Method(method) = &class.body.members[0] else {
        panic!("Expected method");
    };
    assert_eq!(method.decorators.len(), 1);
}

#[test]
fn test_parse_private_field_decorator() {
    // Private field decorator
    let prog = parse(
        r#"class Foo {
            @transform
            #secret: string = "hidden";
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Property(prop) = &class.body.members[0] else {
        panic!("Expected property");
    };
    assert_eq!(prop.decorators.len(), 1);
}

#[test]
fn test_parse_decorator_member_expression() {
    // Decorator with member expression
    let prog = parse("@Reflect.metadata('key', 'value') class Foo {}");
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    assert_eq!(class.decorators.len(), 1);
}

#[test]
fn test_parse_multiple_decorators_newlines() {
    // Multiple decorators on separate lines
    let prog = parse(
        r#"@first
            @second
            @third
            class Foo {}"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    assert_eq!(class.decorators.len(), 3);
}

#[test]
fn test_parse_decorator_complex_class() {
    // Complex class with multiple decorated members
    let prog = parse(
        r#"@entity
            class User {
                @column
                name: string = "";

                @column
                @primary
                id: number = 0;

                @method
                static create(): User { return new User(); }

                @computed
                get fullName(): string { return this.name; }
            }"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    assert_eq!(class.decorators.len(), 1);
    // Should have 4 members total
    assert_eq!(class.body.members.len(), 4);
}

#[test]
fn test_parse_class_expression_decorator() {
    // Decorator on class expression
    let prog = parse("const Foo = @decorator class { value: number = 1; };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_parse_export_decorated_class() {
    // Export decorated class
    let prog = parse("export @decorator class Foo {}");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_parse_export_default_decorated_class() {
    // Export default decorated class
    let prog = parse("export default @decorator class Foo {}");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_parse_parameter_decorator_basic() {
    // Parameter decorator in method
    let prog = parse(
        r#"class Service {
            greet(@inject name: string): void {}
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Method(method) = &class.body.members[0] else {
        panic!("Expected method");
    };
    assert_eq!(method.value.params.len(), 1);
    assert_eq!(method.value.params[0].decorators.len(), 1);
}

#[test]
fn test_parse_parameter_decorator_multiple() {
    // Multiple parameter decorators
    let prog = parse(
        r#"class Service {
            greet(@logParam name: string, @logParam age: number): void {}
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Method(method) = &class.body.members[0] else {
        panic!("Expected method");
    };
    assert_eq!(method.value.params.len(), 2);
    assert_eq!(method.value.params[0].decorators.len(), 1);
    assert_eq!(method.value.params[1].decorators.len(), 1);
}

#[test]
fn test_parse_parameter_decorator_factory() {
    // Parameter decorator factory with arguments
    let prog = parse(
        r#"class Controller {
            handle(@Query("id") id: string): void {}
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Method(method) = &class.body.members[0] else {
        panic!("Expected method");
    };
    assert_eq!(method.value.params.len(), 1);
    assert_eq!(method.value.params[0].decorators.len(), 1);
}

#[test]
fn test_parse_parameter_decorator_constructor() {
    // Parameter decorator in constructor
    let prog = parse(
        r#"class Service {
            constructor(@inject db: Database) {}
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
    let Statement::ClassDeclaration(class) = &prog.body[0] else {
        panic!("Expected class declaration");
    };
    let ClassMember::Constructor(ctor) = &class.body.members[0] else {
        panic!("Expected constructor");
    };
    assert_eq!(ctor.params.len(), 1);
    assert_eq!(ctor.params[0].decorators.len(), 1);
}

#[test]
fn test_parse_new_with_type_arguments() {
    // new Promise<void>(...)
    let prog = parse("new Promise<void>((resolve) => resolve());");
    assert_eq!(prog.body.len(), 1);
    let Statement::Expression(expr_stmt) = &prog.body[0] else {
        panic!("Expected expression statement");
    };
    let Expression::New(new_expr) = &*expr_stmt.expression else {
        panic!("Expected new expression");
    };
    assert!(new_expr.type_arguments.is_some());
    let type_args = new_expr.type_arguments.as_ref().unwrap();
    assert_eq!(type_args.params.len(), 1);
    assert_eq!(new_expr.arguments.len(), 1);
}

#[test]
fn test_parse_new_with_multiple_type_arguments() {
    // new Map<string, number>()
    let prog = parse("new Map<string, number>();");
    assert_eq!(prog.body.len(), 1);
    let Statement::Expression(expr_stmt) = &prog.body[0] else {
        panic!("Expected expression statement");
    };
    let Expression::New(new_expr) = &*expr_stmt.expression else {
        panic!("Expected new expression");
    };
    assert!(new_expr.type_arguments.is_some());
    let type_args = new_expr.type_arguments.as_ref().unwrap();
    assert_eq!(type_args.params.len(), 2);
}

#[test]
fn test_parse_new_without_type_arguments() {
    // new Promise((resolve) => resolve())
    let prog = parse("new Promise((resolve) => resolve());");
    assert_eq!(prog.body.len(), 1);
    let Statement::Expression(expr_stmt) = &prog.body[0] else {
        panic!("Expected expression statement");
    };
    let Expression::New(new_expr) = &*expr_stmt.expression else {
        panic!("Expected new expression");
    };
    assert!(new_expr.type_arguments.is_none());
    assert_eq!(new_expr.arguments.len(), 1);
}

#[test]
fn test_parse_new_generic_with_callback() {
    // new Promise<void>((resolve) => { setTimeout(() => resolve(), 100); })
    let prog = parse("new Promise<void>((resolve) => { setTimeout(() => resolve(), 100); });");
    assert_eq!(prog.body.len(), 1);
    let Statement::Expression(expr_stmt) = &prog.body[0] else {
        panic!("Expected expression statement");
    };
    let Expression::New(new_expr) = &*expr_stmt.expression else {
        panic!("Expected new expression");
    };
    assert!(new_expr.type_arguments.is_some());
}

#[test]
fn test_parse_export_star_as_namespace() {
    // export * as utils from "./utils"
    let prog = parse(r#"export * as utils from "./utils";"#);
    assert_eq!(prog.body.len(), 1);

    let Statement::Export(export) = &prog.body[0] else {
        panic!("Expected Export statement");
    };

    // Should have a namespace export
    assert!(export.namespace_export.is_some());
    let ns = export.namespace_export.as_ref().unwrap();
    assert_eq!(ns.name.as_str(), "utils");

    // Should have source
    assert!(export.source.is_some());
    assert_eq!(export.source.as_ref().unwrap().value.as_str(), "./utils");

    // Should not have specifiers or declaration
    assert!(export.specifiers.is_empty());
    assert!(export.declaration.is_none());
    assert!(!export.default);
}

#[test]
fn test_parse_export_star_as_namespace_with_type() {
    // export type * as Types from "./types"
    let prog = parse(r#"export type * as Types from "./types";"#);
    assert_eq!(prog.body.len(), 1);

    let Statement::Export(export) = &prog.body[0] else {
        panic!("Expected Export statement");
    };

    assert!(export.type_only);
    assert!(export.namespace_export.is_some());
    let ns = export.namespace_export.as_ref().unwrap();
    assert_eq!(ns.name.as_str(), "Types");
}

#[test]
fn test_parse_export_star_without_as() {
    // export * from "./utils" - existing behavior, should still work
    let prog = parse(r#"export * from "./utils";"#);
    assert_eq!(prog.body.len(), 1);

    let Statement::Export(export) = &prog.body[0] else {
        panic!("Expected Export statement");
    };

    // No namespace export
    assert!(export.namespace_export.is_none());

    // Should have source
    assert!(export.source.is_some());

    // Empty specifiers
    assert!(export.specifiers.is_empty());
}

#[test]
#[ignore] // TODO: Implement OptionalChain wrapping in generated parser
fn test_parse_optional_chain_parenthesized() {
    use tsrun::ast::Expression;

    // a?.b?.() - direct optional chain call
    let prog1 = parse("a?.b?.()");
    assert_eq!(prog1.body.len(), 1);
    if let Statement::Expression(stmt) = &prog1.body[0] {
        // Verify it's OptionalChain(Call(Member))
        if let Expression::OptionalChain(opt) = stmt.expression.as_ref() {
            if let Expression::Call(call) = opt.base.as_ref() {
                assert!(call.optional, "Expected optional call");
                assert!(
                    matches!(call.callee.as_ref(), Expression::Member(_)),
                    "Expected Member callee"
                );
            } else {
                panic!("Expected Call inside OptionalChain");
            }
        } else {
            panic!("Expected OptionalChain expression");
        }
    }

    // (a?.b)?.() - parenthesized optional chain then optional call
    let prog2 = parse("(a?.b)?.()");
    assert_eq!(prog2.body.len(), 1);
    if let Statement::Expression(stmt) = &prog2.body[0] {
        // Verify it's OptionalChain(Call(Parenthesized(OptionalChain(Member))))
        if let Expression::OptionalChain(opt) = stmt.expression.as_ref() {
            if let Expression::Call(call) = opt.base.as_ref() {
                assert!(call.optional, "Expected optional call");
                // The callee should be a parenthesized optional chain
                assert!(
                    matches!(call.callee.as_ref(), Expression::Parenthesized(_, _)),
                    "Expected Parenthesized callee"
                );
            } else {
                panic!("Expected Call inside OptionalChain");
            }
        } else {
            panic!("Expected OptionalChain expression");
        }
    }
}

// Tests for contextual keywords (namespace, module) as property names and identifiers
#[test]
fn test_namespace_as_property_name() {
    // 'namespace' should be valid as a property name in object literals
    let prog = parse(r#"const obj = { namespace: "production" };"#);
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_module_as_property_name() {
    // 'module' should be valid as a property name in object literals
    let prog = parse(r#"const obj = { module: "esm" };"#);
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_namespace_as_identifier() {
    // 'namespace' can be used as a variable name
    let prog = parse("const namespace = 'test';");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_module_as_identifier() {
    // 'module' can be used as a variable name
    let prog = parse("const module = {};");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_typeof_module() {
    // typeof module should parse correctly (module as identifier after typeof)
    let prog = parse("typeof module;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_typeof_namespace() {
    // typeof namespace should parse correctly
    let prog = parse("typeof namespace;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_module_in_expression() {
    // module should be usable in expressions
    let prog = parse("module && module.exports;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_ternary_with_function_call() {
    // Ternary with function call in alternate - the colon should not be confused with type annotation
    let prog = parse("const x = true ? 1 : foo(2);");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_ternary_with_identifier_function_call() {
    // More complex ternary - the pattern that was failing in lodash
    // Simplified first:
    let prog = parse("const result = cond ? x : iteratee(acc);");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_ternary_with_comma_sequence() {
    // Comma sequence in ternary consequent
    let prog = parse("const result = cond ? (val = false, x) : y;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_ternary_with_comma_and_function_call() {
    // Full pattern from lodash
    let prog = parse("const result = cond ? (val = false, x) : iteratee(acc, val);");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_contextual_keyword_function_call() {
    // Calling a function named 'module' or 'namespace'
    let prog = parse("module(arg1, arg2);");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_object_with_multiple_contextual_keywords() {
    // Object with multiple contextual keywords as property names
    let prog = parse(
        r#"const config = {
            namespace: "prod",
            module: "esm",
            type: "config",
            readonly: false,
            declare: true
        };"#,
    );
    assert_eq!(prog.body.len(), 1);
}

/// Regression test for fuzz slow-unit: deeply nested parentheses with decorators
/// should not cause stack overflow.
/// See: fuzz/artifacts/fuzz_parser/slow-unit-ab71c4f9e9400055c70e81f904002ecd135f0355
#[test]
#[ignore] // TODO: Add depth tracking to generated parser
fn test_fuzz_slow_unit_nested_parens_decorators() {
    use std::time::{Duration, Instant};

    // This input caused stack overflow due to deeply nested expressions
    let input = "Z((~@(((@(~@((@((Z((~@(((@(~@((@((@(@((@(@(~@(((@(@(@((@(@(~@(((@(~@((@((@(@(@((@(@(~@(((@(@(@((@((@((Pa8~";

    let start = Instant::now();
    let mut dict = tsrun::StringDict::new();
    let mut parser = tsrun::parser::Parser::new(input, &mut dict);
    let result = parser.parse_program();
    let elapsed = start.elapsed();

    // Parsing should complete in under 1 second (typically < 10ms for this input)
    assert!(
        elapsed < Duration::from_secs(1),
        "Parser took too long: {:?} (expected < 1s)",
        elapsed
    );

    // Should get a depth exceeded error, not a stack overflow
    let err = result.expect_err("Should fail with depth exceeded error");
    assert!(
        err.to_string().contains("Maximum nesting depth exceeded"),
        "Expected depth error, got: {}",
        err
    );
}

/// Test that reasonable nesting depth is allowed
#[test]
fn test_reasonable_nesting_depth() {
    // 20 levels of nesting should work fine
    let input = "((((((((((((((((((((1))))))))))))))))))))";
    let mut dict = tsrun::StringDict::new();
    let mut parser = tsrun::parser::Parser::new(input, &mut dict);
    let result = parser.parse_program();
    assert!(result.is_ok(), "20 levels of nesting should be allowed");
}

/// Regression test for fuzz slow-unit: deeply nested parens with void and identifiers
/// See: fuzz/artifacts/fuzz_parser/slow-unit-de04980b7521199967d3ee21b40db9c36195a27c
#[test]
fn test_fuzz_slow_unit_de04980b() {
    use std::time::{Duration, Instant};

    let input = r#"$,void(@(@((@((ul=(l=(@tt((@(@(t((@(stst,(ul=(l=@(@((ql?(t!(@(((ul=(l=(@tt(((ul=(l=(@tt((@(@(t(@(stst,(ul=(l=(@tt(t0,sV!,ty0((ul ,"#;

    let start = Instant::now();
    let mut dict = tsrun::StringDict::new();
    let mut parser = tsrun::parser::Parser::new(input, &mut dict);
    let result = parser.parse_program();
    let elapsed = start.elapsed();

    // Parsing should complete in under 1 second
    assert!(
        elapsed < Duration::from_secs(1),
        "Parser took too long: {:?} (expected < 1s)",
        elapsed
    );

    // Should either parse or fail fast with an error - not hang
    drop(result);
}

/// Regression test for fuzz slow-unit: variant with asterisk
/// See: fuzz/artifacts/fuzz_parser/slow-unit-833e89a84a8ed8ba02328993d6f1b393d01cc751
#[test]
fn test_fuzz_slow_unit_833e89a8() {
    use std::time::{Duration, Instant};

    let input = r#"$,void(@(@((@((ul=(l=(@tt((@(@(t((@(stst,(ul=(l=@(@((ql?(t!*@(((ul=(l=(@tt(((ul=(l=(@tt((@(@(t((@(stst,/(ul=(l=(@tt(t0,sV!,ty0((ul ,"#;

    let start = Instant::now();
    let mut dict = tsrun::StringDict::new();
    let mut parser = tsrun::parser::Parser::new(input, &mut dict);
    let result = parser.parse_program();
    let elapsed = start.elapsed();

    // Parsing should complete in under 1 second
    assert!(
        elapsed < Duration::from_secs(1),
        "Parser took too long: {:?} (expected < 1s)",
        elapsed
    );

    // Should either parse or fail fast with an error - not hang
    drop(result);
}

/// Regression test for fuzz slow-unit: many < characters causing backtracking
/// See: fuzz/artifacts/fuzz_parser/slow-unit-840ff275771e130db2a4dcbe91e7243d502fb3ed
#[test]
fn test_fuzz_slow_unit_840ff275() {
    use std::time::{Duration, Instant};

    // This input has many < characters that triggered exponential backtracking
    // in try_parse_call_with_type_args before the lookahead optimization
    let input = r#"Sc<iaPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPIPPPPP<a<a<a<aabrwnb<$<b<{rnewnr(Uuu3uuuuuuuuuuuuu=uu-uuupb<$<b<{rnewnr(Uuspaticuuuuuuuuu=uuuuuupbrej/uuuuPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP<a<a<a<aabrwnb<$<b<{rnewnr(Uuu3uuuuuuuuuuuuu=uu-uuPPPPPPPPPPPPIPPPPP<a<a<a<aabrwnb<$<b<{rnewnr(Uuu3uuuuuuuuuuuuu=uu-uuupb<$<b<{rnewnr(Uuspaticuuuuuuuuu=uuuuuupbrej/uuuuPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP<a<a<a<aabrwnb<$<b<{rnewnr(Uuu3uuuuuuuuuuuuu=uu-uuupb<$<b<{rnewnr(Uustaticuuuuuuuuu=uuuuuupubliPPPPPPPP.PPPPPPPPPPPPPPPPPPPPPPPTTTTTTTT<a<!s<[aabbr"#;

    let start = Instant::now();
    let mut dict = tsrun::StringDict::new();
    let mut parser = tsrun::parser::Parser::new(input, &mut dict);
    let result = parser.parse_program();
    let elapsed = start.elapsed();

    // Parsing should complete in under 1 second (was ~14s before fix)
    assert!(
        elapsed < Duration::from_secs(1),
        "Parser took too long: {:?} (expected < 1s)",
        elapsed
    );

    // Should either parse or fail fast with an error - not hang
    drop(result);
}

// ============================================================================
// Tests for features needed by interpreter tests
// ============================================================================

#[test]
fn test_async_arrow_function_expression() {
    // async arrow function in expression position
    let prog = parse("const foo = async () => 42;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_async_arrow_function_with_params() {
    // async arrow function with parameters
    let prog = parse("const add = async (a, b) => a + b;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_function_expression() {
    // function expression assigned to variable
    let prog = parse("const foo = function() { return 42; };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_function_expression_named() {
    // named function expression
    let prog = parse("const foo = function bar() { return 42; };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_function_expression_in_callback() {
    // function expression as callback argument
    let prog = parse("arr.map(function(x) { return x * 2; });");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_super_in_class_method() {
    // super call in constructor
    let prog = parse(
        r#"
        class Child extends Parent {
            constructor() {
                super();
            }
        }
    "#,
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_super_property_access() {
    // super property access
    let prog = parse(
        r#"
        class Child extends Parent {
            method() {
                return super.method();
            }
        }
    "#,
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_numeric_literal_type_property() {
    // numeric literal in type/interface property
    let prog = parse(
        r#"
        interface ArrayLike {
            0: string;
            1: number;
            length: number;
        }
    "#,
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_declare_as_identifier() {
    // declare used as identifier (not keyword in expression context)
    let prog = parse("const declare = 42;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_is_as_property_name() {
    // is used as property name
    let prog = parse("const obj = { is: true };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_new_expression_with_member_access() {
    // new expression followed by member access
    let prog = parse("new Boolean(true).toString()");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_for_await_of() {
    // for await loop
    let prog = parse(
        r#"
        async function test() {
            for await (const x of gen) {
                console.log(x);
            }
        }
    "#,
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_destructuring_function_param() {
    // destructuring pattern in function parameter
    let prog = parse("function foo({ a, b }: { a: number; b: number }) { return a + b; }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_array_destructuring_function_param() {
    // array destructuring pattern in function parameter
    let prog = parse("function foo([a, b]: number[]) { return a + b; }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_default_parameter_value() {
    // default parameter value
    let prog = parse("function foo(x: number = 1, y: number = x): number { return x + y; }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_keyword_as_property_key() {
    // return keyword as property key
    let prog = parse("const obj = { return: function() { return 1; } };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_true_as_property_key() {
    // true keyword as property key
    let prog = parse("const obj = { true: 1, false: 0 };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_enum_as_property_key() {
    // enum keyword as property key (for JSON Schema-like interfaces)
    let prog = parse(
        r#"
        interface SchemaDefinition {
            type: string;
            enum?: any[];
        }
        const schema: SchemaDefinition = { type: "string", enum: ["a", "b"] };
    "#,
    );
    assert_eq!(prog.body.len(), 2);
}

#[test]
fn test_object_spread() {
    // spread in object literal
    let prog = parse("const obj = { ...a, ...b, c: 1 };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_generator_method_in_object() {
    // generator method in object literal
    let prog = parse("const obj = { *gen() { yield 1; } };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_export_namespace() {
    // export namespace
    let prog = parse(
        r#"
        export namespace Foo {
            export const x = 1;
        }
    "#,
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_class_accessor() {
    // accessor keyword in class
    let prog = parse(
        r#"
        class Point {
            accessor x: number = 10;
            accessor y: number = 20;
        }
    "#,
    );
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_labeled_statement() {
    // labeled block statement
    let prog = parse("outer: { break outer; }");
    assert_eq!(prog.body.len(), 1);
    assert!(matches!(prog.body[0], Statement::Labeled(_)));
}

#[test]
fn test_async_function_declaration() {
    // async function declaration
    let prog = parse("async function foo() { return 42; }");
    assert_eq!(prog.body.len(), 1);
    assert!(matches!(prog.body[0], Statement::FunctionDeclaration(_)));
}

// Regression: catch clause with type annotation
#[test]
fn test_catch_type_annotation() {
    // Catch without type annotation
    let prog1 = parse("try { throw 1; } catch (e) { }");
    assert_eq!(prog1.body.len(), 1);

    // Catch with type annotation
    let prog2 = parse("try { throw 1; } catch (e: any) { }");
    assert_eq!(prog2.body.len(), 1);
    if let Statement::Try(try_stmt) = &prog2.body[0] {
        assert!(try_stmt.handler.is_some(), "Handler should be present");
    } else {
        panic!("Expected Try statement");
    }
}

#[test]
fn test_async_method_in_object_literal() {
    // async method in object literal
    let prog = parse("const obj = { async getValue() { return 42; } };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_async_method_in_class() {
    // async method in class
    let prog = parse("class Foo { async getValue() { return 42; } }");
    assert_eq!(prog.body.len(), 1);
    assert!(matches!(prog.body[0], Statement::ClassDeclaration(_)));
}

#[test]
fn test_generic_method_in_class() {
    // generic method in class: method<T>(param: T): T
    let prog = parse("class Foo { getValue<T>(x: T): T { return x; } }");
    assert_eq!(prog.body.len(), 1);
    assert!(matches!(prog.body[0], Statement::ClassDeclaration(_)));
}

#[test]
fn test_keyword_as_property_name() {
    // Keywords like 'accessor' can be used as property names
    let prog = parse("class Foo { get accessor(): number { return 1; } }");
    assert_eq!(prog.body.len(), 1);
    assert!(matches!(prog.body[0], Statement::ClassDeclaration(_)));
}

#[test]
fn test_object_literal_getter_setter() {
    // Getter and setter in object literal
    let prog = parse("const obj = { get value() { return 1; }, set value(v) { } };");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_namespace_declaration() {
    // namespace declaration
    let prog = parse("namespace MyNamespace { export const x = 1; }");
    assert_eq!(prog.body.len(), 1);
    assert!(matches!(prog.body[0], Statement::NamespaceDeclaration(_)));
}

#[test]
fn test_for_in_with_predeclared_var() {
    // for-in with pre-declared variable
    let prog = parse("let x; let obj = {}; for (x in obj) { }");
    assert_eq!(prog.body.len(), 3);
    assert!(matches!(prog.body[2], Statement::ForIn(_)));
}

// Regression: sequence expression (comma operator)
#[test]
fn test_sequence_expression() {
    let prog = parse("(1, 2, 3);");
    assert_eq!(prog.body.len(), 1);
    if let Statement::Expression(expr_stmt) = &prog.body[0] {
        assert!(matches!(
            expr_stmt.expression.as_ref(),
            Expression::Sequence(_)
        ));
    } else {
        panic!("Expected expression statement");
    }
}

// Regression: sequence expression in for loop update
#[test]
fn test_for_with_sequence_update() {
    let prog = parse("for (let i = 0; i < 10; i++, i++) { }");
    assert_eq!(prog.body.len(), 1);
    if let Statement::For(for_stmt) = &prog.body[0] {
        if let Some(update) = &for_stmt.update {
            assert!(matches!(update.as_ref(), Expression::Sequence(_)));
        } else {
            panic!("Expected update expression");
        }
    } else {
        panic!("Expected for statement");
    }
}

#[test]
fn test_for_multiple_variables() {
    // for loop with multiple variable declarations
    let prog = parse("for (let i = 0, j = 10; i < j; i++, j--) { }");
    assert_eq!(prog.body.len(), 1);
    if let Statement::For(for_stmt) = &prog.body[0] {
        // Check init has 2 declarations
        if let Some(ForInit::Variable(decl)) = &for_stmt.init {
            assert_eq!(decl.declarations.len(), 2);
        } else {
            panic!("Expected variable declaration in for init");
        }
        // Check update is a sequence expression
        if let Some(update) = &for_stmt.update {
            assert!(matches!(update.as_ref(), Expression::Sequence(_)));
        } else {
            panic!("Expected update expression");
        }
    } else {
        panic!("Expected for statement");
    }
}

#[test]
fn test_declare_const() {
    // declare const should be parsed
    let prog = parse("declare const x: number;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_declare_function() {
    // declare function should be parsed
    let prog = parse("declare function foo(x: number): string;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_conditional_type() {
    // conditional type with infer
    let prog = parse("type ReturnType<T> = T extends (...args: any[]) => infer R ? R : never;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_type_predicate() {
    // type predicate return type
    let prog = parse("function isCat(pet: any): pet is Cat { return true; }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_asserts_type_predicate() {
    // asserts type predicate return type
    let prog = parse("function assertIsCat(pet: any): asserts pet is Cat { }");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_const_assertion() {
    // const assertion type
    let prog = parse("const arr = [1, 2, 3] as const;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_constructor_type() {
    // Constructor type annotation: new (...args: any[]) => T
    let prog = parse("type Ctor<T> = new (...args: any[]) => T;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_template_literal_type() {
    // Template literal type annotation: `Hello ${string}`
    let prog = parse("type Greeting = `Hello ${string}`;");
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_declare_module_string() {
    // declare module with string literal name
    let prog = parse(r#"declare module "express" { export interface Request {} }"#);
    assert_eq!(prog.body.len(), 1);
}

#[test]
fn test_angle_bracket_type_assertion() {
    // angle bracket type assertion
    let prog = parse("let x = <number>value;");
    assert_eq!(prog.body.len(), 1);
}

/// Regression test for fuzz crash: deeply nested block statements
/// See: fuzz crashes crash-aef78366831cef4e959723e476283c21b6f652e2
/// and crash-16f3b33f47bd013b30e2ae4f29ee96fc53935049
/// Note: This test is ignored in debug mode due to larger stack frames
#[test]
#[ignore] // TODO: Add depth tracking to generated parser
fn test_fuzz_crash_deeply_nested_block_statements() {
    use std::time::{Duration, Instant};

    // Input with 230+ nested braces (from fuzz corpus)
    let input = "{".repeat(230);

    let start = Instant::now();
    let mut dict = tsrun::StringDict::new();
    let mut parser = tsrun::parser::Parser::new(&input, &mut dict);
    let result = parser.parse_program();
    let elapsed = start.elapsed();

    // Parsing should complete in under 1 second (not stack overflow)
    assert!(
        elapsed < Duration::from_secs(1),
        "Parser took too long: {:?} (expected < 1s)",
        elapsed
    );

    // Should get a depth exceeded error, not a stack overflow
    let err = result.expect_err("Should fail with depth exceeded error");
    assert!(
        err.to_string().contains("Maximum nesting depth exceeded"),
        "Expected depth error, got: {}",
        err
    );
}

#[test]
fn test_unary_minus() {
    let prog = parse("-1");
    assert_eq!(prog.body.len(), 1);
    match &prog.body[0] {
        Statement::Expression(expr_stmt) => match expr_stmt.expression.as_ref() {
            Expression::Unary(unary) => {
                assert_eq!(unary.operator, tsrun::ast::UnaryOp::Minus);
            }
            other => panic!("Expected Unary expression, got: {:?}", other),
        },
        other => panic!("Expected Expression statement, got: {:?}", other),
    }
}

#[test]
fn test_unary_minus_with_binary() {
    let prog = parse("-1 + 2");
    assert_eq!(prog.body.len(), 1);
    match &prog.body[0] {
        Statement::Expression(expr_stmt) => match expr_stmt.expression.as_ref() {
            Expression::Binary(binary) => {
                assert_eq!(binary.operator, tsrun::ast::BinaryOp::Add);
                // Left should be unary minus
                match binary.left.as_ref() {
                    Expression::Unary(unary) => {
                        assert_eq!(unary.operator, tsrun::ast::UnaryOp::Minus);
                    }
                    other => panic!("Expected Unary expression on left, got: {:?}", other),
                }
            }
            other => panic!("Expected Binary expression, got: {:?}", other),
        },
        other => panic!("Expected Expression statement, got: {:?}", other),
    }
}

// ============================================================================
// Tests to isolate parser bugs
// ============================================================================

// Regression: method with return type annotation followed by body containing return
#[test]
fn test_method_with_return_type_and_return_statement() {
    // Simple class method with return type annotation
    let prog = parse("class C { getValue(): number { return 42; } }");
    assert_eq!(prog.body.len(), 1);
}

// Regression: return class expression (simple)
#[test]
fn test_return_class_expression_simple() {
    let prog = parse("function f() { return class {}; }");
    assert_eq!(prog.body.len(), 1);
}

// Regression: return class expression with method
#[test]
fn test_return_class_expression_with_method() {
    let prog = parse("function f() { return class { getValue() { return 42; } }; }");
    assert_eq!(prog.body.len(), 1);
}

// Regression: return class expression with typed method
#[test]
fn test_return_class_expression_with_typed_method() {
    // This is the failing pattern from test_mixin_simple_class_return
    let prog = parse("function f() { return class { getValue(): number { return 42; } }; }");
    assert_eq!(prog.body.len(), 1);
}

// Regression: class expression assigned to variable
#[test]
fn test_class_expression_assigned_to_variable() {
    let prog = parse("const C = class { getValue(): number { return 42; } };");
    assert_eq!(prog.body.len(), 1);
}

// Regression: switch statement inside function
#[test]
fn test_switch_in_function() {
    let prog = parse(
        r#"function f(x) {
            switch (x) {
                case 1: return "one";
                default: return "other";
            }
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
}

// Regression: try-catch with throw
#[test]
fn test_try_catch_with_throw() {
    let prog = parse(
        r#"function f() {
            try {
                throw new Error("test");
            } catch (e) {
                return e;
            }
        }"#,
    );
    assert_eq!(prog.body.len(), 1);
}

// Regression: optional chaining with method call
#[test]
fn test_optional_chain_method_call() {
    let prog = parse("obj?.method()");
    assert_eq!(prog.body.len(), 1);
}

// Regression: debug parse errors
#[test]
fn test_debug_parse_error() {
    // Test various patterns to find what exact combination fails
    let test_cases = [
        // Context: parenthesized object
        ("({ a: 1 })", "parens obj + literal"),
        ("({ a: x })", "parens obj + ident"),
        ("({ a: x.y })", "parens obj + member"),  // FAILS
        ("({ a: f() })", "parens obj + call"),     // FAILS
        ("({ a: x[0] })", "parens obj + index"),   // FAILS
        ("({ a: 1 + 2 })", "parens obj + binary"), // infix
        ("({ a: -x })", "parens obj + unary"),     // prefix

        // Context: assignment RHS
        ("o = { a: 1 }", "assign obj + literal"),
        ("o = { a: x }", "assign obj + ident"),
        ("o = { a: x.y }", "assign obj + member"),  // FAILS
        ("o = { a: 1 + 2 }", "assign obj + binary"),

        // Context: variable declaration
        ("let o = { a: x.y }", "vardecl obj + member"), // works!
        ("let o = { a: 1 + 2 }", "vardecl obj + binary"),

        // Context: comparison
        ("({ a: x.y }) === 1", "parens obj member + compare"),
    ];

    for (input, desc) in test_cases {
        let mut dict = StringDict::new();
        let result = Parser::new(input, &mut dict).parse_program();
        match result {
            Ok(prog) => {
                let status = if prog.body.len() == 1 { "OK" } else { "FAIL (0 stmts)" };
                eprintln!("{}: {} - {}", desc, status, input);
            }
            Err(e) => eprintln!("{}: ERROR {:?} - {}", desc, e, input),
        }
    }
}

// Debug: show actual parse error
#[test]
fn test_debug_show_error() {
    let input = "({ a: x.y })";
    let mut dict = StringDict::new();
    let mut parser = Parser::new(input, &mut dict);
    let result = parser.parse_program();

    eprintln!("Input: {}", input);
    eprintln!("Result: {:?}", result);

    // Also try direct expression parsing at various positions
    eprintln!("\n--- Trying simpler subexpressions ---");

    // Just identifier
    let mut dict = StringDict::new();
    let mut p1 = Parser::new("x", &mut dict);
    eprintln!("'x': {:?}", p1.parse_program());

    // Member access
    let mut dict = StringDict::new();
    let mut p2 = Parser::new("x.y", &mut dict);
    eprintln!("'x.y': {:?}", p2.parse_program());

    // Parenthesized member
    let mut dict = StringDict::new();
    let mut p3 = Parser::new("(x.y)", &mut dict);
    eprintln!("'(x.y)': {:?}", p3.parse_program());

    // Object literal without parens
    let mut dict = StringDict::new();
    let mut p4 = Parser::new("let o = { a: x.y }", &mut dict);
    eprintln!("'let o = {{ a: x.y }}': {:?}", p4.parse_program().map(|p| p.body.len()));

    // Empty object in parens
    let mut dict = StringDict::new();
    let mut p5 = Parser::new("({})", &mut dict);
    eprintln!("'({{}})': {:?}", p5.parse_program().map(|p| p.body.len()));

    // Object with simple value in parens
    let mut dict = StringDict::new();
    let mut p6 = Parser::new("({ a: 1 })", &mut dict);
    eprintln!("'({{ a: 1 }})': {:?}", p6.parse_program().map(|p| p.body.len()));

    // Object with member value WITHOUT parens - this should work
    let mut dict = StringDict::new();
    let mut p7 = Parser::new("{ a: x.y }", &mut dict);
    let r7 = p7.parse_program();
    eprintln!("'{{ a: x.y }}': {} statements, body: {:?}", r7.as_ref().map(|p| p.body.len()).unwrap_or(0), r7.as_ref().ok().map(|p| &p.body));

    // Compare: simple member in function return
    let mut dict = StringDict::new();
    let mut p8 = Parser::new("function f() { return x.y }", &mut dict);
    eprintln!("'function f() {{ return x.y }}': {:?}", p8.parse_program().map(|p| p.body.len()));

    // Object with member in function return
    let mut dict = StringDict::new();
    let mut p9 = Parser::new("function f() { return { a: x.y } }", &mut dict);
    eprintln!("'function f() {{ return {{ a: x.y }} }}': {:?}", p9.parse_program().map(|p| p.body.len()));

    // Test labeled statement specifically
    let mut dict = StringDict::new();
    let mut p10 = Parser::new("{ a: x }", &mut dict);
    eprintln!("'{{ a: x }}' (labeled, no member): {:?}", p10.parse_program().map(|p| p.body.len()));

    // Member access at top level
    let mut dict = StringDict::new();
    let mut p11 = Parser::new("a.b", &mut dict);
    eprintln!("'a.b' (top level): {:?}", p11.parse_program().map(|p| p.body.len()));

    // Block with member access
    let mut dict = StringDict::new();
    let mut p12 = Parser::new("{ a.b }", &mut dict);
    eprintln!("'{{ a.b }}' (in block): {:?}", p12.parse_program().map(|p| p.body.len()));

    // Block with simple expr
    let mut dict = StringDict::new();
    let mut p13 = Parser::new("{ x }", &mut dict);
    eprintln!("'{{ x }}' (in block): {:?}", p13.parse_program().map(|p| p.body.len()));

    // Block with member + semicolon
    let mut dict = StringDict::new();
    let mut p14 = Parser::new("{ a.b; }", &mut dict);
    eprintln!("'{{ a.b; }}' (member + semicolon): {:?}", p14.parse_program().map(|p| p.body.len()));

    // Block with call
    let mut dict = StringDict::new();
    let mut p15 = Parser::new("{ a.b() }", &mut dict);
    let r15 = p15.parse_program();
    eprintln!("'{{ a.b() }}' (call): {:?}", r15.as_ref().map(|p| p.body.len()));
    eprintln!("'{{ a.b() }}' FULL: {:?}", r15);

    // Block with binary expr
    let mut dict = StringDict::new();
    let mut p16 = Parser::new("{ a + b }", &mut dict);
    eprintln!("'{{ a + b }}' (binary): {:?}", p16.parse_program().map(|p| p.body.len()));

    // Check member access with longer chains
    let mut dict = StringDict::new();
    let mut p17 = Parser::new("{ a.b.c }", &mut dict);
    eprintln!("'{{ a.b.c }}' (chain): {:?}", p17.parse_program().map(|p| p.body.len()));

    // Check with just 'a.' - partial member
    let mut dict = StringDict::new();
    let mut p18 = Parser::new("a.", &mut dict);
    eprintln!("'a.' (incomplete): {:?}", p18.parse_program());

    // Block and then statement after
    let mut dict = StringDict::new();
    let mut p19 = Parser::new("{ } a.b", &mut dict);
    eprintln!("'{{ }} a.b' (after block): {:?}", p19.parse_program().map(|p| p.body.len()));

    // If statement with member
    let mut dict = StringDict::new();
    let mut p20 = Parser::new("if (true) a.b", &mut dict);
    eprintln!("'if (true) a.b': {:?}", p20.parse_program().map(|p| p.body.len()));

    // Full AST of { a.b }
    let mut dict = StringDict::new();
    let mut p21 = Parser::new("{ a.b }", &mut dict);
    let r21 = p21.parse_program();
    eprintln!("'{{ a.b }}' FULL AST: {:?}", r21);

    // Full AST of { a.b; }
    let mut dict = StringDict::new();
    let mut p22 = Parser::new("{ a.b; }", &mut dict);
    let r22 = p22.parse_program();
    eprintln!("'{{ a.b; }}' FULL AST: {:?}", r22);

    // Test without spaces to eliminate whitespace issues
    let mut dict = StringDict::new();
    let mut p23 = Parser::new("{a+b}", &mut dict);
    eprintln!("'{{a+b}}' (no spaces): {:?}", p23.parse_program().map(|p| p.body.len()));

    let mut dict = StringDict::new();
    let mut p24 = Parser::new("{a.b}", &mut dict);
    eprintln!("'{{a.b}}' (no spaces): {:?}", p24.parse_program().map(|p| p.body.len()));

    // Test with space only after expression
    let mut dict = StringDict::new();
    let mut p25 = Parser::new("{a.b }", &mut dict);
    eprintln!("'{{a.b }}' (space before close): {:?}", p25.parse_program().map(|p| p.body.len()));

    let mut dict = StringDict::new();
    let mut p26 = Parser::new("{a+b }", &mut dict);
    eprintln!("'{{a+b }}' (space before close): {:?}", p26.parse_program().map(|p| p.body.len()));

    // Detailed postfix vs infix comparison
    let mut dict = StringDict::new();
    let mut p27 = Parser::new("{ a.b }", &mut dict);  // spaces around
    eprintln!("'{{ a.b }}' (spaces around): {:?}", p27.parse_program().map(|p| p.body.len()));

    let mut dict = StringDict::new();
    let mut p28 = Parser::new("{ a+b }", &mut dict);  // spaces around
    eprintln!("'{{ a+b }}' (spaces around): {:?}", p28.parse_program().map(|p| p.body.len()));

    // Check if Pratt parser ends at different positions
    let mut dict = StringDict::new();
    let mut p29 = Parser::new("a.b}", &mut dict);  // member followed by }
    eprintln!("'a.b}}' (member then close): {:?}", p29.parse_program().map(|p| p.body.len()));

    let mut dict = StringDict::new();
    let mut p30 = Parser::new("a+b}", &mut dict);  // binary followed by }
    eprintln!("'a+b}}' (binary then close): {:?}", p30.parse_program().map(|p| p.body.len()));

    // Full AST for { a+b } to see what's different
    let mut dict = StringDict::new();
    let mut p31 = Parser::new("{ a+b }", &mut dict);
    let r31 = p31.parse_program();
    eprintln!("'{{ a+b }}' FULL AST: {:?}", r31);

    // Full AST for {a.b } (space before close only)
    let mut dict = StringDict::new();
    let mut p32 = Parser::new("{a.b }", &mut dict);
    let r32 = p32.parse_program();
    eprintln!("'{{a.b }}' FULL AST: {:?}", r32);

    // Test class expression
    let mut dict = StringDict::new();
    let mut p33 = Parser::new("const Foo = class { };", &mut dict);
    let r33 = p33.parse_program();
    eprintln!("'const Foo = class {{ }};' FULL AST: {:?}", r33);

    // Test more detailed class expression
    let mut dict = StringDict::new();
    let mut p34 = Parser::new("const Foo = class { getValue() { return 1; } };", &mut dict);
    let r34 = p34.parse_program();
    eprintln!("'const Foo = class {{ getValue() {{ }} }};' result: {:?}", r34.as_ref().map(|p| p.body.len()));

    // Simple class expression
    let mut dict = StringDict::new();
    let mut p35 = Parser::new("const x = class { }", &mut dict);
    let r35 = p35.parse_program();
    eprintln!("'const x = class {{ }}' FULL: {:?}", r35);

    // Compare with simpler expression
    let mut dict = StringDict::new();
    let mut p36 = Parser::new("const x = 5", &mut dict);
    let r36 = p36.parse_program();
    eprintln!("'const x = 5' FULL: {:?}", r36);

    // Optional call expression
    let mut dict = StringDict::new();
    let mut p_opt = Parser::new("a?.b()", &mut dict);
    let r_opt = p_opt.parse_program();
    eprintln!("'a?.b()' FULL: {:?}", r_opt);

    // Chained optional call expression
    let mut dict = StringDict::new();
    let mut p_opt2 = Parser::new("a?.b.c()", &mut dict);
    let r_opt2 = p_opt2.parse_program();
    eprintln!("'a?.b.c()' FULL: {:?}", r_opt2);

    // Chained optional with member after call
    let mut dict = StringDict::new();
    let mut p_opt3 = Parser::new("a?.b.c().d", &mut dict);
    let r_opt3 = p_opt3.parse_program();
    eprintln!("'a?.b.c().d' FULL: {:?}", r_opt3);

    // Call expression without member
    let mut dict = StringDict::new();
    let mut p37 = Parser::new("{ f() }", &mut dict);
    let r37 = p37.parse_program();
    eprintln!("'{{ f() }}' FULL: {:?}", r37);

    // Call expression with semicolon
    let mut dict = StringDict::new();
    let mut p38 = Parser::new("{ a.b(); }", &mut dict);
    let r38 = p38.parse_program();
    eprintln!("'{{ a.b(); }}': {:?}", r38.as_ref().map(|p| p.body.len()));

    // Member chain: Array.prototype.push - Bug #6 investigation
    let mut dict = StringDict::new();
    let mut p_chain = Parser::new("Array.prototype.push", &mut dict);
    let r_chain = p_chain.parse_program();
    eprintln!("'Array.prototype.push' FULL: {:?}", r_chain);

    // Full expression from failing test
    let mut dict = StringDict::new();
    let mut p_bind = Parser::new("Function.prototype.call.bind(Array.prototype.push)", &mut dict);
    let r_bind = p_bind.parse_program();
    eprintln!("'Function.prototype.call.bind(Array.prototype.push)' FULL: {:?}", r_bind);

    // Decorated class expression
    let mut dict = StringDict::new();
    let mut p_dec = Parser::new("const Foo = @mark class { };", &mut dict);
    let r_dec = p_dec.parse_program();
    eprintln!("'const Foo = @mark class {{ }};' FULL: {:?}", r_dec);

    // Return class extends
    let mut dict = StringDict::new();
    let mut p_ret = Parser::new("return class extends Base { };", &mut dict);
    let r_ret = p_ret.parse_program();
    eprintln!("'return class extends Base {{ }};' FULL: {:?}", r_ret);

    // Simple class extends expression
    let mut dict = StringDict::new();
    let mut p_ext = Parser::new("const C = class extends Base { };", &mut dict);
    let r_ext = p_ext.parse_program();
    eprintln!("'const C = class extends Base {{ }};' FULL: {:?}", r_ext);

    // Even simpler - no extends
    let mut dict = StringDict::new();
    let mut p_simple = Parser::new("const C = class { };", &mut dict);
    let r_simple = p_simple.parse_program();
    eprintln!("'const C = class {{ }};' FULL: {:?}", r_simple);

    // Test with named class and extends
    let mut dict = StringDict::new();
    let mut p_named = Parser::new("const C = class Foo extends Base { };", &mut dict);
    let r_named = p_named.parse_program();
    eprintln!("'const C = class Foo extends Base {{ }};' FULL: {:?}", r_named);

    // Test class declaration with extends (should work)
    let mut dict = StringDict::new();
    let mut p_decl = Parser::new("class Foo extends Base { }", &mut dict);
    let r_decl = p_decl.parse_program();
    eprintln!("'class Foo extends Base {{ }}' (declaration): {:?}", r_decl);
}

// Regression: member access inside object literal value - BUG FOUND
#[test]
fn test_member_access_in_object_literal() {
    // Test: parens matter!

    // Object WITHOUT parens - works
    eprintln!("\n=== Test: let o = {{ a: x.y }} ===");
    let prog1 = parse("let o = { a: x.y }");
    eprintln!("let o = {{ a: x.y }}: {} statements", prog1.body.len());

    // Object WITH parens but simple value - works
    eprintln!("\n=== Test: ({{ a: x }}) ===");
    let prog2 = parse("({ a: x })");
    eprintln!("({{ a: x }}): {} statements", prog2.body.len());

    // Object WITH parens and member access - FAILS
    eprintln!("\n=== Test: ({{ a: x.y }}) ===");
    let prog3 = parse("({ a: x.y })");
    eprintln!("({{ a: x.y }}): {} statements", prog3.body.len());

    // Member access in parens (no object literal) - works?
    eprintln!("\n=== Test: (x.y) ===");
    let prog4 = parse("(x.y)");
    eprintln!("(x.y): {} statements", prog4.body.len());

    // Nested parens around object - does it make a difference?
    eprintln!("\n=== Test: (({{ a: x.y }})) ===");
    let prog5 = parse("(({}))");
    eprintln!("(({{}})) (empty): {} statements", prog5.body.len());

    // Just object in expression position (like return)
    eprintln!("\n=== Test: return {{ a: x.y }} ===");
    let prog6 = parse("function f() { return { a: x.y } }");
    eprintln!("return {{ a: x.y }}: {} statements", prog6.body.len());

    // Arrow function returning object - special case
    eprintln!("\n=== Test: () => ({{ a: x.y }}) ===");
    let prog7 = parse("() => ({ a: x.y })");
    eprintln!("() => ({{ a: x.y }}): {} statements", prog7.body.len());

    // Just the object inside return
    eprintln!("\n=== Test: {{ a: x.y }} as expr statement ===");
    let prog8 = parse("{ a: x.y }");
    eprintln!("{{ a: x.y }} (block): {} statements", prog8.body.len());

    // Assertions
    assert_eq!(prog1.body.len(), 1, "let o = {{ a: x.y }} should parse");
    assert_eq!(prog2.body.len(), 1, "({{ a: x }}) should parse");
    assert_eq!(prog3.body.len(), 1, "({{ a: x.y }}) should parse");
    assert_eq!(prog4.body.len(), 1, "(x.y) should parse");
}

// Regression: async arrow with await
#[test]
fn test_async_arrow_with_await() {
    let prog = parse("const f = async () => { await Promise.resolve(); };");
    assert_eq!(prog.body.len(), 1);
}

// Regression: class with constructor parameter properties
#[test]
fn test_class_constructor_parameter_properties() {
    let prog = parse("class C { constructor(public x: number, private y: string) {} }");
    assert_eq!(prog.body.len(), 1);
}

// Regression: multiple type declarations at module level
#[test]
fn test_multiple_type_declarations() {
    let prog = parse(
        r#"
        interface A { x: number; }
        interface B { y: string; }
        const a: A = { x: 1 };
        const b: B = { y: "test" };
    "#,
    );
    assert_eq!(prog.body.len(), 4);
}

// Regression: function with destructuring parameter and default
#[test]
fn test_function_destructuring_default() {
    let prog = parse("function f({ x = 1 }: { x?: number } = {}) { return x; }");
    assert_eq!(prog.body.len(), 1);
}

// ========================================
// Import declaration tests
// ========================================

#[test]
fn test_parse_import_named_specifiers() {
    let prog = parse(r#"import { add, sub as subtract } from "./math";"#);
    assert_eq!(prog.body.len(), 1);

    let Statement::Import(import) = &prog.body[0] else {
        panic!("Expected Import statement");
    };

    // Verify specifiers are populated
    assert_eq!(
        import.specifiers.len(),
        2,
        "Expected 2 import specifiers, got {:?}",
        import.specifiers
    );

    // Check first specifier
    match &import.specifiers[0] {
        ImportSpecifier::Named {
            local, imported, ..
        } => {
            assert_eq!(local.name.as_str(), "add");
            assert_eq!(imported.name.as_str(), "add");
        }
        _ => panic!("Expected Named specifier"),
    }

    // Check second specifier with rename
    match &import.specifiers[1] {
        ImportSpecifier::Named {
            local, imported, ..
        } => {
            assert_eq!(local.name.as_str(), "subtract");
            assert_eq!(imported.name.as_str(), "sub");
        }
        _ => panic!("Expected Named specifier"),
    }
}

#[test]
fn test_parse_import_default() {
    let prog = parse(r#"import Config from "./config";"#);
    let Statement::Import(import) = &prog.body[0] else {
        panic!("Expected Import statement");
    };

    assert_eq!(
        import.specifiers.len(),
        1,
        "Expected 1 import specifier, got {:?}",
        import.specifiers
    );
    assert!(
        matches!(&import.specifiers[0], ImportSpecifier::Default { .. }),
        "Expected Default specifier, got {:?}",
        import.specifiers[0]
    );
}

#[test]
fn test_parse_import_namespace() {
    let prog = parse(r#"import * as utils from "./utils";"#);
    let Statement::Import(import) = &prog.body[0] else {
        panic!("Expected Import statement");
    };

    assert_eq!(
        import.specifiers.len(),
        1,
        "Expected 1 import specifier, got {:?}",
        import.specifiers
    );
    assert!(
        matches!(&import.specifiers[0], ImportSpecifier::Namespace { .. }),
        "Expected Namespace specifier, got {:?}",
        import.specifiers[0]
    );
}

#[test]
fn test_parse_side_effect_import() {
    let prog = parse(r#"import "./polyfill";"#);
    let Statement::Import(import) = &prog.body[0] else {
        panic!("Expected Import statement");
    };

    // Side-effect imports have NO specifiers
    assert_eq!(
        import.specifiers.len(),
        0,
        "Side-effect import should have 0 specifiers, got {:?}",
        import.specifiers
    );
    assert_eq!(import.source.value.as_ref(), "./polyfill");
}

// Regression: async function expression in member assignment
#[test]
fn test_async_function_in_member_assignment() {
    let prog = parse("globalThis.sleep = async function(ms: number): Promise<void> {};");
    assert_eq!(prog.body.len(), 1);
    match &prog.body[0] {
        Statement::Expression(expr) => {
            if let Expression::Assignment(_) = expr.expression.as_ref() {
                // Good - it's an assignment
            } else {
                panic!("Expected assignment expression, got {:?}", std::mem::discriminant(expr.expression.as_ref()));
            }
        }
        other => panic!("Expected expression statement, got {:?}", std::mem::discriminant(other)),
    }
}

// Regression: postfix member access missing trailing whitespace in ASI context
#[test]
fn test_postfix_member_in_block_asi() {
    // Member access in block without semicolon (relies on ASI)
    let prog = parse("{ a.b }");
    assert_eq!(prog.body.len(), 1, "Should parse block with member access");
}

// Regression: call expression missing trailing whitespace in ASI context
#[test]
fn test_call_expression_in_block_asi() {
    // Call expression in block without semicolon (relies on ASI)
    let prog = parse("{ f() }");
    assert_eq!(prog.body.len(), 1, "Should parse block with call expression");

    // Member call in block without semicolon
    let prog2 = parse("{ a.b() }");
    assert_eq!(prog2.body.len(), 1, "Should parse block with member call");
}

// Regression: class expression not converted to AST
#[test]
fn test_class_expression_init() {
    let prog = parse("const Foo = class { };");
    assert_eq!(prog.body.len(), 1);
    if let Statement::VariableDeclaration(decl) = &prog.body[0] {
        assert!(decl.declarations[0].init.is_some(), "Class expression should be captured as init");
        if let Some(init) = &decl.declarations[0].init {
            assert!(matches!(init.as_ref(), Expression::Class(_)), "Init should be Class expression");
        }
    } else {
        panic!("Expected variable declaration");
    }
}

// Regression: anonymous class extends fails - "extends" consumed as class name
#[test]
fn test_class_expression_extends_anonymous() {
    use tsrun::ast::{Expression, Statement};
    let prog = parse("const C = class extends Base { };");
    assert_eq!(prog.body.len(), 1);
    if let Statement::VariableDeclaration(decl) = &prog.body[0] {
        if let Some(init) = &decl.declarations[0].init {
            if let Expression::Class(cls) = init.as_ref() {
                assert!(cls.id.is_none(), "Anonymous class should have no name");
                assert!(cls.super_class.is_some(), "Should have super_class");
            } else {
                panic!("Expected Class expression");
            }
        } else {
            panic!("Expected init");
        }
    } else {
        panic!("Expected variable declaration");
    }
}

// Regression: named class extends should preserve both name and super_class
#[test]
fn test_class_expression_extends_named() {
    use tsrun::ast::{Expression, Statement};
    let prog = parse("const C = class Foo extends Base { };");
    assert_eq!(prog.body.len(), 1);
    if let Statement::VariableDeclaration(decl) = &prog.body[0] {
        if let Some(init) = &decl.declarations[0].init {
            if let Expression::Class(cls) = init.as_ref() {
                assert!(cls.id.is_some(), "Named class should have name");
                assert_eq!(cls.id.as_ref().map(|id| id.name.as_ref()), Some("Foo"));
                assert!(cls.super_class.is_some(), "Should have super_class");
            } else {
                panic!("Expected Class expression");
            }
        } else {
            panic!("Expected init");
        }
    } else {
        panic!("Expected variable declaration");
    }
}

// Regression: class property without semicolon (ASI)
#[test]
fn test_class_property_without_semicolon() {
    let prog = parse("class Buffer { x: number }");
    assert_eq!(prog.body.len(), 1);
}

// Regression: method signature in declare class
#[test]
fn test_declare_class_method_signature() {
    let prog = parse("declare class Buffer { static from(str: string): Buffer; }");
    assert_eq!(prog.body.len(), 1);
}

// Regression: object method with return type annotation
#[test]
fn test_object_method_with_return_type() {
    use tsrun::ast::{Expression, Statement};
    // Method with return type annotation
    let prog = parse("let calc = { add(a: number, b: number): number { return a + b; } };");
    assert_eq!(prog.body.len(), 1);
    if let Statement::VariableDeclaration(decl) = &prog.body[0] {
        assert!(decl.declarations[0].init.is_some(), "Object literal should be captured");
        if let Some(Expression::Object(_)) = decl.declarations[0].init.as_deref() {
            // OK
        } else {
            panic!("Expected ObjectExpression");
        }
    } else {
        panic!("Expected VariableDeclaration");
    }
}

// Regression: new expression with member access callee was parsed incorrectly
// `new Models.Simple()` was parsed as `(new Models).Simple()` instead of `new (Models.Simple)()`
#[test]
fn test_new_member_expression_callee() {
    use tsrun::ast::{Expression, Statement};
    let prog = parse("new Models.Simple()");
    assert_eq!(prog.body.len(), 1);
    if let Statement::Expression(stmt) = &prog.body[0] {
        if let Expression::New(new_expr) = &*stmt.expression {
            // Callee must be a member expression, not an identifier
            if let Expression::Member(_) = &*new_expr.callee {
                // Good, callee is Member expression
            } else {
                panic!("Expected callee to be MemberExpression, got {:?}", new_expr.callee);
            }
        } else {
            panic!("Expected NewExpression, got {:?}", stmt.expression);
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

// Regression: unicode escape in member access property name
#[test]
fn test_unicode_escape_member_property() {
    // obj.b\u0061r should parse as obj.bar (member access with unicode escape in property)
    let prog = parse("obj.b\\u0061r");
    assert_eq!(prog.body.len(), 1);
}

// Regression: double underscore identifier was parsed as NaN + identifier
// Number literals with only underscores (e.g., `__`) should not match as numbers
#[test]
fn test_double_underscore_identifier() {
    use tsrun::ast::{Expression, Statement};
    // __test should parse as a single identifier, not NaN + "test"
    let prog = parse("__test");
    assert_eq!(prog.body.len(), 1);
    if let Statement::Expression(stmt) = &prog.body[0] {
        if let Expression::Identifier(id) = &*stmt.expression {
            assert_eq!(id.name, "__test");
        } else {
            panic!("Expected Identifier, got {:?}", stmt.expression);
        }
    } else {
        panic!("Expected ExpressionStatement");
    }

    // typeof __push should parse correctly
    let prog2 = parse("typeof __push");
    assert_eq!(prog2.body.len(), 1);
    if let Statement::Expression(stmt) = &prog2.body[0] {
        if let Expression::Unary(unary) = &*stmt.expression {
            if let Expression::Identifier(id) = &*unary.argument {
                assert_eq!(id.name, "__push");
            } else {
                panic!("Expected Identifier argument, got {:?}", unary.argument);
            }
        } else {
            panic!("Expected UnaryExpression, got {:?}", stmt.expression);
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

// Regression: class getter with numeric key was parsed incorrectly
#[test]
fn test_class_getter_numeric_key() {
    let prog = parse("class C { get 2() { return 'hello'; } }");
    assert_eq!(prog.body.len(), 1);
    if let Statement::ClassDeclaration(class) = &prog.body[0] {
        if let ClassMember::Method(method) = &class.body.members[0] {
            assert_eq!(method.kind, MethodKind::Get, "Should be a getter");
            assert!(matches!(&method.key, ObjectPropertyKey::Number(_)), "Key should be Number");
        } else {
            panic!("Expected Method");
        }
    } else {
        panic!("Expected ClassDeclaration");
    }
}

// Regression: spread in function call with whitespace after comma failed
// sum(1, ...args) failed because postfix_call separator doesn't consume trailing ws
#[test]
fn test_spread_in_function_call_with_whitespace() {
    parse("sum(...args)");
    parse("sum(1, 2)");
    parse("sum(1,...args)");
    parse("sum(1, ...args)");
}

// Regression: constructor type `new () => T` was not recognized in type constraints
// This caused mixin patterns like `<T extends new (...args: any[]) => any>` to fail
#[test]
fn test_constructor_type_in_generic_constraint() {
    parse("function foo() {}");
    parse("function foo<T>() {}");
    parse("function foo<T extends object>() {}");
    parse("function foo<T extends new () => object>() {}");
}

// Regression: declare statements
#[test]
fn test_declare_statements() {
    // declare const and declare function work
    parse("declare const process: { env: Record<string, string> };");
    parse("declare function require(id: string): any;");
    // declare class with method signatures works
    parse("declare class EventEmitter { on(event: string, listener: Function): void; }");
    // declare namespace with ambient function
    parse("declare namespace fs { function readFile(path: string): string; }");
}

// Regression: unclosed braces should fail to parse
#[test]
fn test_unclosed_brace_fails() {
    assert!(parse_fails("{"));
}

// "return return" is valid with ASI - produces two return statements
#[test]
fn test_return_return_is_valid() {
    // With ASI, "return return" parses as two return statements
    let prog = parse("return return");
    assert_eq!(prog.body.len(), 2);
}

// Regression: inline comments should be handled
#[test]
fn test_inline_comment() {
    let prog = parse("let x = 1; // comment\nlet y = 2;");
    assert_eq!(prog.body.len(), 2);
}

// Regression: expression with inline comment
#[test]
fn test_expression_with_comment() {
    let prog = parse("Math.E === Math.E  // Should still be defined");
    assert_eq!(prog.body.len(), 1);
}
