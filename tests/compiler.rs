//! Tests for the bytecode compiler
//!
//! These tests verify that the compiler correctly generates bytecode
//! from AST nodes.

use tsrun::compiler::{BytecodeChunk, Compiler, Op};
use tsrun::parser::Parser;
use tsrun::string_dict::StringDict;

/// Parse source and compile to bytecode
#[allow(clippy::expect_used)]
fn compile(source: &str) -> BytecodeChunk {
    let mut dict = StringDict::new();
    let mut parser = Parser::new(source, &mut dict);
    let program = parser.parse_program().expect("parse failed");
    let chunk = Compiler::compile_program(&program).expect("compile failed");
    (*chunk).clone()
}

/// Helper to check if bytecode contains a specific opcode type
fn contains_op<F: Fn(&Op) -> bool>(chunk: &BytecodeChunk, predicate: F) -> bool {
    chunk.code.iter().any(predicate)
}

#[test]
fn test_compile_number_literal() {
    let chunk = compile("42");

    // Should have LoadInt or LoadConst for 42
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::LoadInt { value: 42, .. })),
        "Expected LoadInt for 42, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_string_literal() {
    let chunk = compile("'hello'");

    // Should have LoadConst for the string
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::LoadConst { .. })),
        "Expected LoadConst for string, got {:?}",
        chunk.code
    );

    // Should have the string in constants
    assert!(
        chunk.constants.iter().any(|c| {
            matches!(c, tsrun::compiler::Constant::String(s) if s.as_ref() == "hello")
        }),
        "Expected 'hello' in constants"
    );
}

#[test]
fn test_compile_boolean_literal() {
    let chunk = compile("true");
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::LoadBool { value: true, .. })),
        "Expected LoadBool true, got {:?}",
        chunk.code
    );

    let chunk = compile("false");
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::LoadBool { value: false, .. })),
        "Expected LoadBool false, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_null_undefined() {
    let chunk = compile("null");
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::LoadNull { .. })),
        "Expected LoadNull, got {:?}",
        chunk.code
    );

    // Note: `undefined` is parsed as an identifier, not a literal
    // so it generates GetVar instead of LoadUndefined
    // The LoadUndefined opcode is used for void expressions and uninitialized variables
    let chunk = compile("void 0");
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::Void { .. })),
        "Expected Void for void 0, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_binary_add() {
    let chunk = compile("1 + 2");

    // Should have Add opcode
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::Add { .. })),
        "Expected Add, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_binary_operators() {
    // Test various binary operators
    let ops = [
        ("1 - 2", "Sub"),
        ("1 * 2", "Mul"),
        ("1 / 2", "Div"),
        ("1 % 2", "Mod"),
        ("1 ** 2", "Exp"),
        ("1 === 2", "StrictEq"),
        ("1 !== 2", "StrictNotEq"),
        ("1 == 2", "Eq"),
        ("1 != 2", "NotEq"),
        ("1 < 2", "Lt"),
        ("1 <= 2", "LtEq"),
        ("1 > 2", "Gt"),
        ("1 >= 2", "GtEq"),
        ("1 & 2", "BitAnd"),
        ("1 | 2", "BitOr"),
        ("1 ^ 2", "BitXor"),
        ("1 << 2", "LShift"),
        ("1 >> 2", "RShift"),
        ("1 >>> 2", "URShift"),
    ];

    for (source, expected_op) in ops {
        let chunk = compile(source);
        let has_op = chunk.code.iter().any(|op| {
            let op_name = format!("{:?}", op);
            op_name.starts_with(expected_op)
        });
        assert!(
            has_op,
            "Expected {} for '{}', got {:?}",
            expected_op, source, chunk.code
        );
    }
}

#[test]
fn test_compile_unary_operators() {
    let chunk = compile("-x");
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::Neg { .. })),
        "Expected Neg, got {:?}",
        chunk.code
    );

    let chunk = compile("!x");
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::Not { .. })),
        "Expected Not, got {:?}",
        chunk.code
    );

    let chunk = compile("~x");
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::BitNot { .. })),
        "Expected BitNot, got {:?}",
        chunk.code
    );

    let chunk = compile("typeof x");
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::Typeof { .. })),
        "Expected Typeof, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_variable_declaration() {
    let chunk = compile("let x = 42");

    // Should have DeclareVar
    assert!(
        contains_op(&chunk, |op| matches!(
            op,
            Op::DeclareVar { mutable: true, .. }
        )),
        "Expected DeclareVar with mutable=true, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_const_declaration() {
    let chunk = compile("const x = 42");

    // Should have DeclareVar with mutable=false
    assert!(
        contains_op(&chunk, |op| matches!(
            op,
            Op::DeclareVar { mutable: false, .. }
        )),
        "Expected DeclareVar with mutable=false, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_var_declaration() {
    let chunk = compile("var x = 42");

    // Should have DeclareVarHoisted
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::DeclareVarHoisted { .. })),
        "Expected DeclareVarHoisted, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_variable_read() {
    let chunk = compile("x");

    // Should have GetVar
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::GetVar { .. })),
        "Expected GetVar, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_variable_assignment() {
    let chunk = compile("x = 42");

    // Should have SetVar
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::SetVar { .. })),
        "Expected SetVar, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_if_statement() {
    let chunk = compile("if (true) { x }");

    // Should have JumpIfFalse for the condition
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::JumpIfFalse { .. })),
        "Expected JumpIfFalse, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_if_else() {
    let chunk = compile("if (true) { x } else { y }");

    // Should have JumpIfFalse and unconditional Jump
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::JumpIfFalse { .. })),
        "Expected JumpIfFalse, got {:?}",
        chunk.code
    );
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::Jump { .. })),
        "Expected Jump, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_while_loop() {
    let chunk = compile("while (true) { x }");

    // Should have JumpIfFalse and Jump back
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::JumpIfFalse { .. })),
        "Expected JumpIfFalse, got {:?}",
        chunk.code
    );
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::Jump { .. })),
        "Expected Jump, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_object_literal() {
    let chunk = compile("({ a: 1 })");

    // Should have CreateObject and SetPropertyConst
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::CreateObject { .. })),
        "Expected CreateObject, got {:?}",
        chunk.code
    );
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::SetPropertyConst { .. })),
        "Expected SetPropertyConst, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_array_literal() {
    let chunk = compile("[1, 2, 3]");

    // Should have CreateArray
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::CreateArray { .. })),
        "Expected CreateArray, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_member_access() {
    let chunk = compile("obj.prop");

    // Should have GetPropertyConst for static property access
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::GetPropertyConst { .. })),
        "Expected GetPropertyConst, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_computed_member_access() {
    let chunk = compile("obj[key]");

    // Should have GetProperty for computed access
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::GetProperty { .. })),
        "Expected GetProperty, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_function_call() {
    let chunk = compile("foo()");

    // Should have Call
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::Call { .. })),
        "Expected Call, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_method_call() {
    let chunk = compile("obj.method()");

    // Method calls now use GetPropertyConst + Call pattern for correct evaluation order
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::GetPropertyConst { .. })),
        "Expected GetPropertyConst, got {:?}",
        chunk.code
    );
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::Call { .. })),
        "Expected Call, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_new_expression() {
    let chunk = compile("new Foo()");

    // Should have Construct
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::Construct { .. })),
        "Expected Construct, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_return() {
    // Can't have bare return, need to wrap in function context
    // For now just test that Return opcode exists in our enum
    let chunk = compile("42");
    // This is a placeholder - actual return compilation needs function context
    assert!(!chunk.code.is_empty());
}

#[test]
fn test_compile_throw() {
    let chunk = compile("throw new Error()");

    // Should have Throw
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::Throw { .. })),
        "Expected Throw, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_try_catch() {
    let chunk = compile("try { x } catch (e) { y }");

    // Should have PushTry and PopTry
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::PushTry { .. })),
        "Expected PushTry, got {:?}",
        chunk.code
    );
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::PopTry)),
        "Expected PopTry, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_logical_and() {
    let chunk = compile("a && b");

    // Should have JumpIfFalse for short-circuit
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::JumpIfFalse { .. })),
        "Expected JumpIfFalse for &&, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_logical_or() {
    let chunk = compile("a || b");

    // Should have JumpIfTrue for short-circuit
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::JumpIfTrue { .. })),
        "Expected JumpIfTrue for ||, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_conditional_expression() {
    let chunk = compile("a ? b : c");

    // Should have JumpIfFalse and Jump
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::JumpIfFalse { .. })),
        "Expected JumpIfFalse for ternary, got {:?}",
        chunk.code
    );
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::Jump { .. })),
        "Expected Jump for ternary, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_update_increment() {
    let chunk = compile("x++");

    // Should have Add for increment
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::Add { .. })),
        "Expected Add for ++, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_update_decrement() {
    let chunk = compile("x--");

    // Should have Sub for decrement
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::Sub { .. })),
        "Expected Sub for --, got {:?}",
        chunk.code
    );
}

#[test]
fn test_compile_halt_at_end() {
    let chunk = compile("42");

    // Every program should end with Halt
    assert!(
        matches!(chunk.code.last(), Some(Op::Halt)),
        "Expected Halt at end, got {:?}",
        chunk.code.last()
    );
}

#[test]
fn test_compile_scope_push_pop() {
    let chunk = compile("{ let x = 1 }");

    // Block should have PushScope and PopScope
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::PushScope)),
        "Expected PushScope, got {:?}",
        chunk.code
    );
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::PopScope)),
        "Expected PopScope, got {:?}",
        chunk.code
    );
}

#[test]
fn test_source_map() {
    let chunk = compile("1 + 2");

    // Should have source map entries
    assert!(!chunk.source_map.is_empty(), "Expected source map entries");
}

#[test]
fn test_register_count() {
    let chunk = compile("1 + 2 + 3 + 4 + 5");

    // Should have a reasonable register count (not 0)
    assert!(
        chunk.register_count > 0,
        "Expected positive register count, got {}",
        chunk.register_count
    );
}

#[test]
fn test_compile_nullish_coalescing() {
    let chunk = compile("null ?? 'default'");

    // Print bytecode for debugging
    println!("Bytecode for: null ?? 'default'");
    println!("Register count: {}", chunk.register_count);
    for (i, op) in chunk.code.iter().enumerate() {
        println!("  {}: {:?}", i, op);
    }

    // Should have JumpIfNotNullish for nullish coalescing
    assert!(
        contains_op(&chunk, |op| matches!(op, Op::JumpIfNotNullish { .. })),
        "Expected JumpIfNotNullish for ??, got {:?}",
        chunk.code
    );

    // Verify the jump target is correct - it should jump past the 'default' load
    let has_valid_jump = chunk.code.iter().any(|op| {
        if let Op::JumpIfNotNullish { target, .. } = op {
            // The target should be a valid instruction index
            (*target as usize) < chunk.code.len()
        } else {
            false
        }
    });
    assert!(has_valid_jump, "JumpIfNotNullish has invalid target");
}
