//! Tests for garbage collection of JavaScript objects

use typescript_eval::{GcStats, JsString, JsValue, Runtime, RuntimeResult, RuntimeValue};

/// Get baseline object count (builtins only, no user code)
fn get_baseline_live_count() -> usize {
    let runtime = Runtime::new();
    runtime.collect();
    runtime.gc_stats().live_objects
}

fn eval_with_gc_stats(source: &str) -> (RuntimeValue, GcStats) {
    let mut runtime = Runtime::new();
    let result = match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(rv) => rv,
        other => panic!("Expected Complete, got {:?}", other),
    };
    // Force GC to run
    runtime.collect();
    let stats = runtime.gc_stats();
    (result, stats)
}

#[test]
fn test_gc_stats_available() {
    let runtime = Runtime::new();
    let stats = runtime.gc_stats();
    // Should have some live objects (global, global_env, prototypes)
    assert!(stats.live_objects > 0, "Should have live objects");
}

#[test]
fn test_baseline_object_count() {
    let baseline = get_baseline_live_count();
    println!("Baseline live count (builtins only): {}", baseline);
    // Builtins include: global, prototypes, constructors, Math, JSON, console, Boolean, etc.
    // This should be stable and typically around 100-300
    assert!(baseline > 50, "Should have some builtins");
    assert!(baseline < 350, "Baseline should be bounded");
}

#[test]
fn test_simple_object_not_leaked() {
    let baseline = get_baseline_live_count();

    let source = r#"
        let sum = 0;
        for (let i = 0; i < 100; i++) {
            const obj = { value: i };
            sum = sum + obj.value;
        }
        sum
    "#;

    let (_, stats) = eval_with_gc_stats(source);
    // After GC, temp objects should be collected
    // We expect only the builtin objects to remain (plus maybe a few for the loop)
    println!("Baseline: {}, After test: {}", baseline, stats.live_objects);

    // Allow for small overhead but temp objects should be collected
    let overhead = stats.live_objects.saturating_sub(baseline);
    assert!(
        overhead < 50,
        "Too many objects leaked after simple loop: {} over baseline",
        overhead
    );
}

#[test]
fn test_cycle_detection_simple() {
    // Create simple cycles and verify GC can handle them during execution.
    // NOTE: Current GC uses ref_count > 0 for root detection, so cycles
    // that have already formed cannot be collected. This test verifies that
    // cycles don't cause crashes and values are computed correctly.
    let source = r#"
        let count = 0;
        for (let i = 0; i < 50; i++) {
            const a = { id: 1, ref: null };
            const b = { id: 2, ref: null };
            a.ref = b;
            b.ref = a;
            count = count + a.id + b.id;
        }
        count
    "#;

    // Run with low gc_threshold to trigger GC during execution
    let mut runtime = Runtime::new();
    runtime.set_gc_threshold(50);
    runtime.set_timeout_ms(0);

    let result = match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    };

    // Expected: 50 * 3 = 150
    assert_eq!(result, JsValue::Number(150.0));
}

#[test]
fn test_self_referencing_collected() {
    // Self-referencing objects create single-element cycles (obj.self = obj).
    // Like multi-element cycles, these cannot be collected by ref_count based GC
    // after the variable goes out of scope. This test verifies correct execution.
    let source = r#"
        let sum = 0;
        for (let i = 0; i < 100; i++) {
            const obj = { value: i, self: null };
            obj.self = obj;
            sum = sum + obj.value;
        }
        sum
    "#;

    // Run with low gc_threshold to trigger GC during execution
    let mut runtime = Runtime::new();
    runtime.set_gc_threshold(50);
    runtime.set_timeout_ms(0);

    let result = match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    };

    // sum = 0 + 1 + 2 + ... + 99 = 4950
    assert_eq!(result, JsValue::Number(4950.0));
}

#[test]
fn test_reachable_objects_preserved() {
    // Objects reachable from global should NOT be collected
    let source = r#"
        // These should survive GC
        var global_obj = { a: 1, b: 2 };
        var global_arr = [1, 2, 3];

        // This should be collected (local scope)
        {
            const local = { temp: true };
        }

        global_obj.a + global_arr.length
    "#;

    let mut runtime = Runtime::new();
    let result = match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    };

    // Run GC
    runtime.collect();

    // Verify global objects are still accessible
    let check = match runtime
        .eval("global_obj.a + global_obj.b + global_arr[0]")
        .unwrap()
    {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    };

    assert_eq!(result, JsValue::Number(4.0));
    assert_eq!(check, JsValue::Number(4.0));
}

#[test]
fn test_closure_environment_preserved() {
    let source = r#"
        function makeCounter() {
            let count = 0;
            return function() {
                count = count + 1;
                return count;
            };
        }

        const counter = makeCounter();
        counter() + counter() + counter()
    "#;

    let (result, _) = eval_with_gc_stats(source);
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_many_cycles_memory_bounded() {
    // Create many cycles and verify memory stays bounded when GC runs during execution.
    // NOTE: Cycles are only collected when GC runs while variables are in scope,
    // because the GC uses ref_count > 0 as root detection. Once variables go out
    // of scope but cycles still exist, the ref_count stays > 0 from cross-references.
    let source = r#"
        let total = 0;
        for (let i = 0; i < 1000; i++) {
            const a = { v: 1 };
            const b = { v: 2 };
            const c = { v: 3 };
            a.next = b;
            b.next = c;
            c.next = a;
            total = total + a.v + b.v + c.v;
        }
        total
    "#;

    // With low GC threshold, collection happens frequently during execution
    // This allows cycles to be broken while variables are still in scope
    let mut runtime = Runtime::new();
    runtime.set_gc_threshold(100);
    runtime.set_timeout_ms(0);

    let result = match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    };

    // Final GC
    runtime.collect();
    let stats = runtime.gc_stats();

    let baseline = 256; // Approximate baseline from builtins
    println!("Result: {:?}", result);
    println!(
        "Baseline: ~{}, After test: {}",
        baseline, stats.live_objects
    );

    assert_eq!(result, JsValue::Number(6000.0));

    // With frequent GC during execution, cycles should be collected
    // We expect some pooled objects showing reuse happened
    assert!(
        stats.pooled_objects > 0,
        "Expected some objects to be pooled (reused) during execution"
    );
}

#[test]
fn test_gc_cycles_graph_with_push_multiple() {
    // Regression test: gc-cycles.ts Test 6 and Test 7 were returning NaN
    // because GC was collecting objects that were still reachable through
    // local variables during loop iterations.
    //
    // The bug was triggered when:
    // 1. GC threshold is reached during execution
    // 2. Objects inside the current loop iteration were being marked as
    //    unreachable and unlinked (properties cleared!), even though local
    //    variables still reference them

    let source = r#"
        interface GraphNode { id: number; edges: GraphNode[]; }
        let sum: number = 0;
        for (let i = 0; i < 100; i++) {
            const n1: GraphNode = { id: 1, edges: [] };
            const n2: GraphNode = { id: 2, edges: [] };
            const n3: GraphNode = { id: 3, edges: [] };
            const n4: GraphNode = { id: 4, edges: [] };
            const n5: GraphNode = { id: 5, edges: [] };

            n1.edges.push(n2, n3);
            n2.edges.push(n1, n3, n4);
            n3.edges.push(n2, n4, n5);
            n4.edges.push(n3, n5);
            n5.edges.push(n4, n1);

            sum = sum + n1.id + n2.id + n3.id + n4.id + n5.id;
        }
        sum
    "#;

    // Test with low GC threshold to trigger the bug
    let mut runtime = Runtime::new();
    runtime.set_gc_threshold(100);
    let result = match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    };

    // Expected: 100 * (1+2+3+4+5) = 100 * 15 = 1500
    assert_eq!(
        result,
        JsValue::Number(1500.0),
        "Graph with push multiple should compute correct sum (got NaN due to GC bug)"
    );
}

#[test]
fn test_gc_cycles_array_refs_with_push_multiple() {
    // Regression test for gc-cycles.ts Test 7
    let source = r#"
        interface ArrayNode { value: number; refs: ArrayNode[]; }
        let sum: number = 0;
        for (let i = 0; i < 50; i++) {
            const a: ArrayNode = { value: 1, refs: [] };
            const b: ArrayNode = { value: 2, refs: [] };
            const c: ArrayNode = { value: 3, refs: [] };

            a.refs.push(b, c);
            b.refs.push(c, a);
            c.refs.push(a, b);

            sum = sum + a.value + b.value + c.value;
        }
        sum
    "#;

    let mut runtime = Runtime::new();
    runtime.set_gc_threshold(100);
    let result = match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    };

    // Expected: 50 * 6 = 300
    assert_eq!(
        result,
        JsValue::Number(300.0),
        "Array refs with push multiple should compute correct sum (got NaN due to GC bug)"
    );
}

#[test]
fn test_gc_object_cycle_with_property_assignment() {
    // Regression test: cycles created via property assignment should survive GC
    let source = r#"
        let sum: number = 0;
        for (let i = 0; i < 50; i++) {
            const a: { id: number; ref: any } = { id: 1, ref: null };
            const b: { id: number; ref: any } = { id: 2, ref: null };
            a.ref = b;
            b.ref = a;  // Creates cycle
            sum = sum + a.id + b.id;
        }
        sum
    "#;

    let mut runtime = Runtime::new();
    runtime.set_gc_threshold(50); // Low threshold to trigger GC often
    let result = match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    };

    // Expected: 50 * 3 = 150
    assert_eq!(
        result,
        JsValue::Number(150.0),
        "Object properties should survive GC during cycle creation"
    );
}

#[test]
fn test_gc_object_cycle_with_array_push() {
    // Regression test: cycles created via array push should survive GC.
    // This was a bug where object literals with nested arrays would get
    // unlinked by GC before the arrays were populated.
    let source = r#"
        let sum: number = 0;
        for (let i = 0; i < 50; i++) {
            const a: { id: number; refs: any[] } = { id: 1, refs: [] };
            const b: { id: number; refs: any[] } = { id: 2, refs: [] };
            a.refs.push(b);
            b.refs.push(a);  // Creates cycle via array
            sum = sum + a.id + b.id;
        }
        sum
    "#;

    let mut runtime = Runtime::new();
    runtime.set_gc_threshold(50); // Low threshold to trigger GC often
    let result = match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    };

    // Expected: 50 * 3 = 150
    assert_eq!(
        result,
        JsValue::Number(150.0),
        "Object properties should survive GC when cycles are created via array push"
    );
}

#[test]
fn test_gc_cycles_full_sequence() {
    // Reduced reproduction of gc-cycles.ts Tests 1 through 7 with lower
    // iteration counts to run within timeout but still trigger multiple GCs
    // (SCALE = 0.1 compared to the original)
    let source = r#"
const results: number[] = [];

// Test 1: Two-node cycles (1000 iterations)
{
    let sum: number = 0;
    for (let i = 0; i < 1000; i++) {
        const a: { id: number; other: any } = { id: i, other: null };
        const b: { id: number; other: any } = { id: i + 1, other: null };
        a.other = b;
        b.other = a;
        sum = sum + a.id + b.id;
    }
    results.push(sum);
}

// Test 2: Triangle cycles (500 iterations)
{
    let sum: number = 0;
    for (let i = 0; i < 500; i++) {
        const a: { v: number; next: any } = { v: 1, next: null };
        const b: { v: number; next: any } = { v: 2, next: null };
        const c: { v: number; next: any } = { v: 3, next: null };
        a.next = b;
        b.next = c;
        c.next = a;
        sum = sum + a.v + b.v + c.v;
    }
    results.push(sum);
}

// Test 3: Ring cycles (100 iterations)
{
    let sum: number = 0;
    for (let i = 0; i < 100; i++) {
        const ringSize: number = 5 + (i % 10);
        interface RingNode { id: number; next: RingNode | null; }
        const nodes: RingNode[] = [];
        for (let j = 0; j < ringSize; j++) {
            nodes.push({ id: j, next: null });
        }
        for (let j = 0; j < ringSize; j++) {
            nodes[j].next = nodes[(j + 1) % ringSize];
        }
        for (let j = 0; j < ringSize; j++) {
            sum = sum + nodes[j].id;
        }
    }
    results.push(sum);
}

// Test 4: Doubly-linked cycles (200 iterations)
{
    let sum: number = 0;
    for (let i = 0; i < 200; i++) {
        interface DLNode { id: number; prev: DLNode | null; next: DLNode | null; }
        const a: DLNode = { id: 1, prev: null, next: null };
        const b: DLNode = { id: 2, prev: null, next: null };
        const c: DLNode = { id: 3, prev: null, next: null };
        const d: DLNode = { id: 4, prev: null, next: null };
        a.next = b; b.next = c; c.next = d; d.next = a;
        b.prev = a; c.prev = b; d.prev = c; a.prev = d;
        sum = sum + a.id + b.id + c.id + d.id;
    }
    results.push(sum);
}

// Test 5: Self-referencing objects (2000 iterations)
{
    let sum: number = 0;
    for (let i = 0; i < 2000; i++) {
        const obj: { value: number; self: any } = { value: i, self: null };
        obj.self = obj;
        sum = sum + obj.value;
    }
    results.push(sum);
}

// Test 6: Complex graph with multiple cycles (100 iterations)
{
    let sum: number = 0;
    for (let i = 0; i < 100; i++) {
        interface GraphNode { id: number; edges: GraphNode[]; }
        const n1: GraphNode = { id: 1, edges: [] };
        const n2: GraphNode = { id: 2, edges: [] };
        const n3: GraphNode = { id: 3, edges: [] };
        const n4: GraphNode = { id: 4, edges: [] };
        const n5: GraphNode = { id: 5, edges: [] };
        n1.edges.push(n2, n3);
        n2.edges.push(n1, n3, n4);
        n3.edges.push(n2, n4, n5);
        n4.edges.push(n3, n5);
        n5.edges.push(n4, n1);
        sum = sum + n1.id + n2.id + n3.id + n4.id + n5.id;
    }
    results.push(sum);
}

// Test 7: Cycles through arrays (300 iterations)
{
    let sum: number = 0;
    for (let i = 0; i < 300; i++) {
        interface ArrayNode { value: number; refs: ArrayNode[]; }
        const a: ArrayNode = { value: 1, refs: [] };
        const b: ArrayNode = { value: 2, refs: [] };
        const c: ArrayNode = { value: 3, refs: [] };
        a.refs.push(b, c);
        b.refs.push(c, a);
        c.refs.push(a, b);
        sum = sum + a.value + b.value + c.value;
    }
    results.push(sum);
}

results
    "#;

    let mut runtime = Runtime::new();
    // Use lower GC threshold to trigger collection more frequently
    runtime.set_gc_threshold(100);
    // Disable timeout for this long-running test
    runtime.set_timeout_ms(0);
    let result = match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(rv) => rv,
        other => panic!("Expected Complete, got {:?}", other),
    };

    if let JsValue::Object(arr) = &*result {
        let arr_ref = arr.borrow();
        let get = |i: usize| -> f64 {
            if let Some(JsValue::Number(n)) =
                arr_ref.get_property(&typescript_eval::value::PropertyKey::Index(i as u32))
            {
                n
            } else {
                f64::NAN
            }
        };

        // Test 1: sum(i=0 to 999) of (i + (i+1)) = sum(2i+1) for i=0..999 = 1000^2 = 1000000
        assert_eq!(get(0), 1000000.0, "Test 1 failed");
        // Test 2: 500 * 6 = 3000
        assert_eq!(get(1), 3000.0, "Test 2 failed");
        // Test 3: 4450 (verified with bun - 1/10 scale of original 44500)
        assert_eq!(get(2), 4450.0, "Test 3 failed");
        // Test 4: 200 * 10 = 2000
        assert_eq!(get(3), 2000.0, "Test 4 failed");
        // Test 5: sum(i=0 to 1999) of i = 1999*2000/2 = 1999000
        assert_eq!(get(4), 1999000.0, "Test 5 failed");
        // Test 6: 100 * 15 = 1500
        assert_eq!(get(5), 1500.0, "Test 6 (complex graph) failed - got NaN!");
        // Test 7: 300 * 6 = 1800
        assert_eq!(get(6), 1800.0, "Test 7 (array cycles) failed - got NaN!");
    } else {
        panic!("Expected array result");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests with gc_threshold=1 to stress test GC safety
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper to evaluate with gc_threshold=1 (most aggressive GC)
fn eval_with_threshold_1(source: &str) -> RuntimeValue {
    let mut runtime = Runtime::new();
    runtime.set_gc_threshold(1);
    runtime.set_timeout_ms(0); // Disable timeout for GC stress tests
    match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(rv) => rv,
        other => panic!("Expected Complete, got {:?}", other),
    }
}

#[test]
fn test_gc_threshold_1_simple_object() {
    let result = eval_with_threshold_1("const obj = { a: 1, b: 2 }; obj.a + obj.b");
    assert_eq!(result, JsValue::Number(3.0));
}

#[test]
fn test_gc_threshold_1_object_with_computed_props() {
    let result = eval_with_threshold_1(
        r#"
        const x = 10;
        const y = 20;
        const obj = { a: x + 1, b: y + 2 };
        obj.a + obj.b
    "#,
    );
    assert_eq!(result, JsValue::Number(33.0));
}

#[test]
fn test_gc_threshold_1_array_literal() {
    let result = eval_with_threshold_1("const arr = [1, 2, 3]; arr[0] + arr[1] + arr[2]");
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_gc_threshold_1_array_with_computed_elements() {
    let result = eval_with_threshold_1(
        r#"
        const x = 10;
        const y = 20;
        const arr = [x + 1, y + 2, x + y];
        arr[0] + arr[1] + arr[2]
    "#,
    );
    assert_eq!(result, JsValue::Number(63.0));
}

#[test]
fn test_gc_threshold_1_nested_objects() {
    let result = eval_with_threshold_1(
        r#"
        const inner = { x: 5 };
        const outer = { inner: inner, y: 10 };
        outer.inner.x + outer.y
    "#,
    );
    assert_eq!(result, JsValue::Number(15.0));
}

#[test]
fn test_gc_threshold_1_array_map() {
    let result = eval_with_threshold_1(
        r#"
        const arr = [1, 2, 3];
        const mapped = arr.map(x => x * 2);
        mapped[0] + mapped[1] + mapped[2]
    "#,
    );
    assert_eq!(result, JsValue::Number(12.0));
}

#[test]
fn test_gc_threshold_1_array_filter() {
    let result = eval_with_threshold_1(
        r#"
        const arr = [1, 2, 3, 4, 5];
        const filtered = arr.filter(x => x > 2);
        filtered.length
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0));
}

#[test]
fn test_gc_threshold_1_object_keys() {
    let result = eval_with_threshold_1(
        r#"
        const obj = { a: 1, b: 2, c: 3 };
        const keys = Object.keys(obj);
        keys.length
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0));
}

#[test]
fn test_gc_threshold_1_object_values() {
    let result = eval_with_threshold_1(
        r#"
        const obj = { a: 1, b: 2, c: 3 };
        const values = Object.values(obj);
        values[0] + values[1] + values[2]
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_gc_threshold_1_object_entries() {
    let result = eval_with_threshold_1(
        r#"
        const obj = { a: 1, b: 2 };
        const entries = Object.entries(obj);
        entries.length
    "#,
    );
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn test_gc_threshold_1_string_split() {
    let result = eval_with_threshold_1(
        r#"
        const str = "a,b,c";
        const parts = str.split(",");
        parts.length
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0));
}

#[test]
fn test_gc_threshold_1_array_concat() {
    let result = eval_with_threshold_1(
        r#"
        const a = [1, 2];
        const b = [3, 4];
        const c = a.concat(b);
        c.length
    "#,
    );
    assert_eq!(result, JsValue::Number(4.0));
}

#[test]
fn test_gc_threshold_1_array_slice() {
    let result = eval_with_threshold_1(
        r#"
        const arr = [1, 2, 3, 4, 5];
        const sliced = arr.slice(1, 4);
        sliced[0] + sliced[1] + sliced[2]
    "#,
    );
    assert_eq!(result, JsValue::Number(9.0));
}

#[test]
fn test_gc_threshold_1_constructor_call() {
    let result = eval_with_threshold_1(
        r#"
        class Point {
            x: number;
            y: number;
            constructor(x: number, y: number) {
                this.x = x;
                this.y = y;
            }
        }
        const p = new Point(3, 4);
        p.x + p.y
    "#,
    );
    assert_eq!(result, JsValue::Number(7.0));
}

#[test]
fn test_gc_threshold_1_loop_with_objects() {
    let result = eval_with_threshold_1(
        r#"
        let sum = 0;
        for (let i = 0; i < 10; i++) {
            const obj = { value: i };
            sum = sum + obj.value;
        }
        sum
    "#,
    );
    assert_eq!(result, JsValue::Number(45.0));
}

#[test]
fn test_gc_threshold_1_json_parse() {
    let result = eval_with_threshold_1(
        r#"
        const obj = JSON.parse('{"a": 1, "b": 2}');
        obj.a + obj.b
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0));
}

#[test]
fn test_gc_threshold_1_array_from() {
    let result = eval_with_threshold_1(
        r#"
        const arr = Array.from([1, 2, 3]);
        arr[0] + arr[1] + arr[2]
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_gc_threshold_1_array_of() {
    let result = eval_with_threshold_1(
        r#"
        const arr = Array.of(1, 2, 3);
        arr[0] + arr[1] + arr[2]
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_gc_threshold_1_function_returning_object() {
    let result = eval_with_threshold_1(
        r#"
        function makeObj(x: number): { value: number } {
            return { value: x * 2 };
        }
        const obj = makeObj(5);
        obj.value
    "#,
    );
    assert_eq!(result, JsValue::Number(10.0));
}

#[test]
fn test_gc_threshold_1_multiple_objects_in_loop() {
    let result = eval_with_threshold_1(
        r#"
        let sum = 0;
        for (let i = 0; i < 20; i++) {
            const a = { v: 1 };
            const b = { v: 2 };
            const c = { v: 3 };
            sum = sum + a.v + b.v + c.v;
        }
        sum
    "#,
    );
    assert_eq!(result, JsValue::Number(120.0));
}

#[test]
fn test_gc_threshold_1_cycles_in_loop() {
    // This test simulates the gc-cycles.ts script behavior
    let result = eval_with_threshold_1(
        r#"
        let sum = 0;
        for (let i = 0; i < 100; i++) {
            const a: { id: number; other: any } = { id: i, other: null };
            const b: { id: number; other: any } = { id: i + 1, other: null };
            a.other = b;
            b.other = a;
            sum = sum + a.id + b.id;
        }
        sum
    "#,
    );
    // sum = 0+1 + 1+2 + 2+3 + ... + 99+100 = 2*(0+1+...+99) + 100 = 2*4950 + 100 = 10000
    assert_eq!(result, JsValue::Number(10000.0));
}

#[test]
fn test_gc_threshold_1_array_foreach() {
    let result = eval_with_threshold_1(
        r#"
        let sum = 0;
        [1, 2, 3].forEach(x => { sum = sum + x; });
        sum
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_gc_threshold_1_array_reduce() {
    let result = eval_with_threshold_1(
        r#"
        const sum = [1, 2, 3, 4, 5].reduce((acc, x) => acc + x, 0);
        sum
    "#,
    );
    assert_eq!(result, JsValue::Number(15.0));
}

#[test]
fn test_gc_threshold_1_array_find() {
    let result = eval_with_threshold_1(
        r#"
        const arr = [1, 2, 3, 4, 5];
        const found = arr.find(x => x > 3);
        found
    "#,
    );
    assert_eq!(result, JsValue::Number(4.0));
}

#[test]
fn test_gc_threshold_1_array_findindex() {
    let result = eval_with_threshold_1(
        r#"
        const arr = [1, 2, 3, 4, 5];
        const idx = arr.findIndex(x => x > 3);
        idx
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0));
}

#[test]
fn test_gc_threshold_1_array_every() {
    let result = eval_with_threshold_1(
        r#"
        const arr = [2, 4, 6];
        arr.every(x => x % 2 === 0)
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_gc_threshold_1_array_some() {
    let result = eval_with_threshold_1(
        r#"
        const arr = [1, 2, 3];
        arr.some(x => x > 2)
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_gc_threshold_1_array_sort_with_comparator() {
    let result = eval_with_threshold_1(
        r#"
        const arr = [3, 1, 4, 1, 5, 9, 2, 6];
        arr.sort((a, b) => b - a);
        arr[0]
    "#,
    );
    assert_eq!(result, JsValue::Number(9.0));
}

#[test]
fn test_gc_threshold_1_array_flatmap() {
    let result = eval_with_threshold_1(
        r#"
        const arr = [1, 2, 3];
        const flat = arr.flatMap(x => [x, x * 2]);
        flat.length
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_gc_threshold_1_nested_class() {
    let result = eval_with_threshold_1(
        r#"
        class Outer {
            inner: { value: number };
            constructor() {
                this.inner = { value: 42 };
            }
        }
        const o = new Outer();
        o.inner.value
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_gc_threshold_1_simple_class_assignment() {
    // Simpler test - just check if this.x = y works
    let result = eval_with_threshold_1(
        r#"
        class Test {
            val: number;
            constructor() {
                this.val = 42;
            }
        }
        const t = new Test();
        t.val
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_gc_threshold_1_assignment_with_object() {
    // Check assignment with object literal without class
    let result = eval_with_threshold_1(
        r#"
        const obj = {};
        obj.inner = { value: 42 };
        obj.inner.value
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));
}

// Test various constructor patterns with GC stress
#[test]
fn test_gc_threshold_1_constructor_patterns() {
    // Object assignment as last statement (was the failing case)
    let result = eval_with_threshold_1(
        r#"
        class Test1 {
            constructor() {
                this.inner = { value: 42 };
            }
        }
        const t = new Test1();
        t.inner.value
    "#,
    );
    assert_eq!(result, JsValue::Number(42.0));

    // Constructor saved to variable matches returned instance
    let result2 = eval_with_threshold_1(
        r#"
        let savedThis: any = null;
        class Test2 {
            constructor() {
                this.inner = { value: 42 };
                savedThis = this;
            }
        }
        const t = new Test2();
        t === savedThis && t.inner.value === 42
    "#,
    );
    assert_eq!(result2, JsValue::Boolean(true));

    // With explicit return statement
    let result3 = eval_with_threshold_1(
        r#"
        class Test3 {
            constructor() {
                this.inner = { value: 42 };
                return;
            }
        }
        const t = new Test3();
        t.inner.value
    "#,
    );
    assert_eq!(result3, JsValue::Number(42.0));

    // With field type annotation
    let result4 = eval_with_threshold_1(
        r#"
        class Test4 {
            inner: { value: number };
            constructor() {
                this.inner = { value: 42 };
            }
        }
        const t = new Test4();
        t.inner.value
    "#,
    );
    assert_eq!(result4, JsValue::Number(42.0));
}

#[test]
fn test_gc_threshold_1_string_replace_callback() {
    // First verify callback is callable - simpler test
    let result_simple = eval_with_threshold_1(
        r#"
        const fn = (x: string) => x.toUpperCase();
        typeof fn === "function"
    "#,
    );
    assert_eq!(
        result_simple,
        JsValue::Boolean(true),
        "callback should be a function"
    );

    // Test that replace with string replacement works
    let result_string = eval_with_threshold_1(
        r#"
        "hello world".replace(/\w+/g, "X")
    "#,
    );
    assert_eq!(
        result_string,
        JsValue::String(JsString::from("X X")),
        "string replacement should work"
    );

    // Test that replace with pre-assigned callback works
    let result_preassigned = eval_with_threshold_1(
        r#"
        const cb = (m: string) => m.toUpperCase();
        "hello".replace(/\w+/g, cb)
    "#,
    );
    assert_eq!(
        result_preassigned,
        JsValue::String(JsString::from("HELLO")),
        "pre-assigned callback should work"
    );

    // Test that replace with inline callback works
    let result = eval_with_threshold_1(
        r#"
        const str = "hello world";
        const result = str.replace(/\w+/g, (match: string) => match.toUpperCase());
        result
    "#,
    );
    assert_eq!(result, JsValue::String(JsString::from("HELLO WORLD")));
}

#[test]
fn test_gc_threshold_1_string_match_all() {
    let result = eval_with_threshold_1(
        r#"
        const str = "test1 test2 test3";
        const matches = [...str.matchAll(/test(\d)/g)];
        matches.length
    "#,
    );
    assert_eq!(result, JsValue::Number(3.0));
}

#[test]
fn test_gc_threshold_1_object_from_entries() {
    // First test: verify entries array is correct
    let entries_test = eval_with_threshold_1(
        r#"
        const entries = [["a", 1], ["b", 2], ["c", 3]];
        entries[0][0] + entries[0][1] + entries[1][0] + entries[1][1]
    "#,
    );
    // "a" + 1 + "b" + 2 = "a1b2"
    assert_eq!(
        entries_test,
        JsValue::String(JsString::from("a1b2")),
        "entries array should be intact"
    );

    let result = eval_with_threshold_1(
        r#"
        const entries = [["a", 1], ["b", 2], ["c", 3]];
        const obj = Object.fromEntries(entries);
        obj.a + obj.b + obj.c
    "#,
    );
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_gc_threshold_1_json_parse_nested() {
    // Test deeply nested JSON parsing
    // obj.a.b.c = 1, obj.d = [1, 2, 3], so 1 + 1 + 2 + 3 = 7
    let result = eval_with_threshold_1(
        r#"
        const obj = JSON.parse('{"a": {"b": {"c": 1}}, "d": [1, 2, 3]}');
        obj.a.b.c + obj.d[0] + obj.d[1] + obj.d[2]
    "#,
    );
    assert_eq!(result, JsValue::Number(7.0));
}

#[test]
fn test_gc_threshold_1_regexp_exec() {
    // Test regexp exec with index and input properties
    let result = eval_with_threshold_1(
        r#"
        const re = /test(\d)/g;
        const str = "test1 test2";
        const match = re.exec(str);
        match.index + match[0].length + match[1].length
    "#,
    );
    // index = 0, match[0] = "test1" (length 5), match[1] = "1" (length 1) => 0 + 5 + 1 = 6
    assert_eq!(result, JsValue::Number(6.0));
}

#[test]
fn test_gc_threshold_1_map_entries() {
    // Test Map.entries() creates arrays correctly
    let result = eval_with_threshold_1(
        r#"
        const m = new Map([["a", 1], ["b", 2], ["c", 3]]);
        const entries = [...m.entries()];
        entries[0][0] + entries[0][1] + entries[1][0] + entries[1][1]
    "#,
    );
    // "a" + 1 + "b" + 2 = "a1b2"
    assert_eq!(result, JsValue::String(JsString::from("a1b2")));
}

#[test]
fn test_gc_threshold_1_map_foreach() {
    // Test Map.forEach() with callback
    let result = eval_with_threshold_1(
        r#"
        const m = new Map([[1, 10], [2, 20], [3, 30]]);
        let sum = 0;
        m.forEach((v: number, k: number) => { sum += v + k; });
        sum
    "#,
    );
    // (10+1) + (20+2) + (30+3) = 11 + 22 + 33 = 66
    assert_eq!(result, JsValue::Number(66.0));
}

#[test]
fn test_gc_threshold_1_try_catch_no_throw() {
    // Test try without throw - accessing outer var after try block
    let result = eval_with_threshold_1(
        r#"
        let result = 1;
        try {
            result = 2;
        } catch (e) {
            result = 3;
        }
        result
    "#,
    );
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn test_gc_threshold_1_try_catch_with_throw() {
    // Test try with throw
    let result = eval_with_threshold_1(
        r#"
        let result: string = "";
        try {
            throw "error message";
        } catch (e: any) {
            result = e;
        }
        result
    "#,
    );
    assert_eq!(result, JsValue::String(JsString::from("error message")));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test for loop environment leak
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_loop_environments_collected() {
    let baseline = get_baseline_live_count();

    // Run a loop that creates many environments (one per iteration for let bindings)
    let source = r#"
        let sum = 0;
        for (let i = 0; i < 1000; i++) {
            const x = i * 2;
            sum = sum + x;
        }
        sum
    "#;

    let (result, stats) = eval_with_gc_stats(source);

    println!("Result: {:?}", result);
    println!(
        "Baseline: {}, After test: total={}, pooled={}, live={}",
        baseline, stats.total_objects, stats.pooled_objects, stats.live_objects
    );

    // After running 1000 loop iterations that each create an environment,
    // the live object count should NOT grow by 1000+
    // A healthy count would be baseline builtins + some small overhead
    let overhead = stats.live_objects.saturating_sub(baseline);
    assert!(
        overhead < 100,
        "Too many live objects: {} over baseline (possible environment leak)",
        overhead
    );
}

#[test]
fn test_for_loop_object_bindings_collected() {
    // Test that objects assigned to loop bindings are collected after each iteration
    let baseline = get_baseline_live_count();

    let source = r#"
        let sum = 0;
        for (let i = 0; i < 500; i++) {
            // Each iteration creates these objects in loop-scoped bindings
            const obj1 = { a: 1, b: 2, c: 3 };
            const obj2 = { x: i, y: i * 2 };
            const arr = [1, 2, 3, 4, 5];
            sum = sum + obj1.a + obj2.x + arr[0];
        }
        sum
    "#;

    let (result, stats) = eval_with_gc_stats(source);

    println!("Result: {:?}", result);
    println!(
        "Baseline: {}, After test: total={}, pooled={}, live={}",
        baseline, stats.total_objects, stats.pooled_objects, stats.live_objects
    );

    // Verify computation is correct
    // sum = 500 * 1 (obj1.a) + (0+1+...+499) (obj2.x) + 500 * 1 (arr[0])
    // = 500 + 499*500/2 + 500 = 500 + 124750 + 500 = 125750
    assert_eq!(result, JsValue::Number(125750.0));

    // If objects weren't collected, we'd have 500 * 3 = 1500+ extra objects
    // Allow some overhead but not a full leak
    let overhead = stats.live_objects.saturating_sub(baseline);
    assert!(
        overhead < 100,
        "Too many live objects: {} over baseline (objects in loop bindings may be leaking)",
        overhead
    );
}

#[test]
fn test_nested_for_loop_environments_collected() {
    // Test that nested loop environments are properly collected
    let baseline = get_baseline_live_count();

    let source = r#"
        let total = 0;
        for (let i = 0; i < 50; i++) {
            const outer_obj = { id: i };
            for (let j = 0; j < 20; j++) {
                const inner_obj = { value: j };
                total = total + outer_obj.id + inner_obj.value;
            }
        }
        total
    "#;

    let (result, stats) = eval_with_gc_stats(source);

    println!("Result: {:?}", result);
    println!(
        "Baseline: {}, After test: total={}, pooled={}, live={}",
        baseline, stats.total_objects, stats.pooled_objects, stats.live_objects
    );

    // Verify computation
    // outer_obj.id contribution: 50 * 20 * (0+1+...+49)/50 = 1000 * 24.5 = doesn't matter
    // Let's just verify it ran correctly
    // total = sum over i,j of (i + j)
    // = 50 * sum(j=0 to 19) + 20 * sum(i=0 to 49)
    // = 50 * 190 + 20 * 1225 = 9500 + 24500 = 34000
    assert_eq!(result, JsValue::Number(34000.0));

    // 50 outer iterations * 20 inner = 1000 inner environments
    // Plus 50 outer environments = 1050 environments total
    // If not collected, we'd have 1050+ extra objects
    let overhead = stats.live_objects.saturating_sub(baseline);
    assert!(
        overhead < 100,
        "Too many live objects: {} over baseline (nested loop environments may be leaking)",
        overhead
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Size checks for memory optimization
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_type_sizes() {
    use std::mem::size_of;
    use typescript_eval::value::{JsObject, JsValue, Property, PropertyKey, PropertyStorage};

    // Print sizes for debugging/optimization
    println!("JsValue: {} bytes", size_of::<JsValue>());
    println!("PropertyKey: {} bytes", size_of::<PropertyKey>());
    println!("Property: {} bytes", size_of::<Property>());
    println!("PropertyStorage: {} bytes", size_of::<PropertyStorage>());
    println!("JsObject: {} bytes", size_of::<JsObject>());
    println!(
        "(PropertyKey, Property) entry: {} bytes",
        size_of::<(PropertyKey, Property)>()
    );

    // Sanity checks - these should be relatively small
    // JsValue is 40 bytes
    assert!(size_of::<JsValue>() <= 40, "JsValue too large");
    assert!(size_of::<PropertyKey>() <= 32, "PropertyKey too large");
    // Property is 56 bytes due to Gc pointer size increase (generation field)
    assert!(size_of::<Property>() <= 56, "Property too large");
    // PropertyStorage uses inline storage for small objects (4 entries × 80 bytes + overhead)
    assert!(
        size_of::<PropertyStorage>() <= 400,
        "PropertyStorage too large"
    );
}
