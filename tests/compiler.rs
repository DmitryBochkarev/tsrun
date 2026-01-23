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

#[test]
fn test_register_usage_array_of_objects() {
    // Array literal with multiple object elements - each object needs temp registers
    let chunk = compile(
        r#"
        const arr = [
            { id: 1, name: "a" },
            { id: 2, name: "b" },
            { id: 3, name: "c" },
            { id: 4, name: "d" },
            { id: 5, name: "e" }
        ];
    "#,
    );

    // Register count should be reasonable, not exploding
    println!(
        "Array of 5 objects register count: {}",
        chunk.register_count
    );
    assert!(
        chunk.register_count < 50,
        "Register count {} is unexpectedly high for array of 5 objects",
        chunk.register_count
    );
}

#[test]
fn test_register_usage_large_array_of_objects() {
    // Larger array to check if registers accumulate
    let chunk = compile(
        r#"
        const arr = [
            { id: 1, name: "a" }, { id: 2, name: "b" }, { id: 3, name: "c" },
            { id: 4, name: "d" }, { id: 5, name: "e" }, { id: 6, name: "f" },
            { id: 7, name: "g" }, { id: 8, name: "h" }, { id: 9, name: "i" },
            { id: 10, name: "j" }, { id: 11, name: "k" }, { id: 12, name: "l" },
            { id: 13, name: "m" }, { id: 14, name: "n" }, { id: 15, name: "o" },
            { id: 16, name: "p" }, { id: 17, name: "q" }, { id: 18, name: "r" },
            { id: 19, name: "s" }, { id: 20, name: "t" }
        ];
    "#,
    );

    println!(
        "Array of 20 objects register count: {}",
        chunk.register_count
    );
    // If registers are freed properly, count should still be low
    // If there's a leak, this would approach 20*3 = 60+ registers
    assert!(
        chunk.register_count < 50,
        "Register count {} suggests register leak in array compilation",
        chunk.register_count
    );
}

#[test]
fn test_register_reuse_in_array_elements() {
    // Registers for each array element should be reused after pushing to array
    // With proper reuse, a 100-element array should NOT need 100+ registers
    let mut elements = String::new();
    for i in 0..100 {
        if i > 0 {
            elements.push_str(", ");
        }
        elements.push_str(&format!("{{ id: {}, name: \"item{}\" }}", i, i));
    }
    let code = format!("const arr = [{}];", elements);

    let result = std::panic::catch_unwind(|| compile(&code));
    match result {
        Ok(chunk) => {
            println!(
                "Array of 100 objects register count: {}",
                chunk.register_count
            );
            // With proper register reuse, we should need far fewer than 100 registers
            assert!(
                chunk.register_count < 50,
                "Register count {} for 100-element array suggests registers aren't being reused",
                chunk.register_count
            );
        }
        Err(_) => {
            panic!("Compilation failed - likely hit 255 register limit, confirming register leak");
        }
    }
}

#[test]
fn test_register_usage_many_statements() {
    // Test many statements at module level (similar to collections/main.ts pattern)
    let chunk = compile(
        r#"
        const map = new Map();
        map.set("one", 1);
        map.set("two", 2);
        map.set("three", 3);
        console.log("Map size:", map.size);
        console.log("get('two'):", map.get("two"));
        console.log("has('three'):", map.has("three"));
        console.log("has('four'):", map.has("four"));
        const fruitPrices = new Map([
            ["apple", 1.5],
            ["banana", 0.75],
            ["orange", 2.0],
            ["grape", 3.25]
        ]);
        console.log("Fruit prices map:");
        fruitPrices.forEach((price, fruit) => {
            console.log("  " + fruit + ": $" + price);
        });
    "#,
    );

    println!("Many statements register count: {}", chunk.register_count);
    assert!(
        chunk.register_count < 100,
        "Register count {} too high for module-level statements",
        chunk.register_count
    );
}

#[test]
fn test_register_usage_many_variables() {
    // Each module-level variable might consume a register if not freed
    let mut code = String::new();
    for i in 0..100 {
        code.push_str(&format!("const v{} = {};\n", i, i));
    }
    code.push_str("v99");

    let result = std::panic::catch_unwind(|| compile(&code));
    match result {
        Ok(chunk) => {
            println!(
                "100 module-level variables register count: {}",
                chunk.register_count
            );
            // Variables should NOT each consume a permanent register
            // They're stored in the environment, not registers
            assert!(
                chunk.register_count < 50,
                "Register count {} suggests variables are using permanent registers",
                chunk.register_count
            );
        }
        Err(_) => {
            panic!("Compilation failed - hit register limit with just 100 variables");
        }
    }
}

#[test]
fn test_register_usage_many_functions() {
    // Many function declarations at module level
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!(
            "function func{}(x: number): number {{ return x + {}; }}\n",
            i, i
        ));
    }
    code.push_str("func49(1)");

    let result = std::panic::catch_unwind(|| compile(&code));
    match result {
        Ok(chunk) => {
            println!("50 functions register count: {}", chunk.register_count);
            assert!(
                chunk.register_count < 100,
                "Register count {} too high for 50 function declarations",
                chunk.register_count
            );
        }
        Err(e) => {
            panic!("Compilation failed with 50 functions: {:?}", e);
        }
    }
}

#[test]
fn test_register_usage_graph_like_module() {
    // Simulate the graph.ts pattern - many exported functions
    let chunk = compile(
        r#"
        function createGraph() { return { nodes: new Map() }; }
        function addNode(graph, node) { if (!graph.nodes.has(node)) { graph.nodes.set(node, new Set()); } }
        function addEdge(graph, from, to) { addNode(graph, from); addNode(graph, to); graph.nodes.get(from).add(to); }
        function hasEdge(graph, from, to) { const n = graph.nodes.get(from); return n !== undefined && n.has(to); }
        function getNeighbors(graph, node) { return graph.nodes.get(node) || new Set(); }
        function removeEdge(graph, from, to) { const n = graph.nodes.get(from); if (n) { n.delete(to); } }
        function bfs(graph, start) {
            const visited = new Set();
            const result = [];
            const queue = [start];
            while (queue.length > 0) {
                const current = queue.shift();
                if (visited.has(current)) continue;
                visited.add(current);
                result.push(current);
                const neighbors = getNeighbors(graph, current);
                for (const neighbor of neighbors) {
                    if (!visited.has(neighbor)) queue.push(neighbor);
                }
            }
            return result;
        }
        function dfs(graph, start) {
            const visited = new Set();
            const result = [];
            function visit(node) {
                if (visited.has(node)) return;
                visited.add(node);
                result.push(node);
                for (const neighbor of getNeighbors(graph, node)) visit(neighbor);
            }
            visit(start);
            return result;
        }
        const g = createGraph();
        addEdge(g, "A", "B");
        bfs(g, "A");
    "#,
    );

    println!("Graph-like module register count: {}", chunk.register_count);
    assert!(
        chunk.register_count < 150,
        "Register count {} too high for graph-like module",
        chunk.register_count
    );
}
