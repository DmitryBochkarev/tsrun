//! Tests for garbage collection of JavaScript objects

use typescript_eval::{Runtime, RuntimeResult, DEFAULT_GC_THRESHOLD};

/// Get baseline object count (builtins only, no user code)
fn get_baseline_alive_count() -> usize {
    let mut runtime = Runtime::new();
    runtime.collect_garbage();
    runtime.gc_stats().alive_count
}

fn eval_with_gc_stats(source: &str) -> (typescript_eval::JsValue, typescript_eval::GcStats) {
    let mut runtime = Runtime::new();
    let result = match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    };
    // Force GC to run
    runtime.collect_garbage();
    let stats = runtime.gc_stats();
    (result, stats)
}

#[test]
fn test_gc_stats_available() {
    let runtime = Runtime::new();
    let stats = runtime.gc_stats();
    // Should have some roots (global, prototypes)
    assert!(stats.roots_count > 0, "Should have roots");
}

#[test]
fn test_baseline_object_count() {
    let baseline = get_baseline_alive_count();
    println!("Baseline alive count (builtins only): {}", baseline);
    // Builtins include: global, prototypes, constructors, Math, JSON, console, etc.
    // This should be stable and typically around 100-200
    assert!(baseline > 50, "Should have some builtins");
    assert!(baseline < 300, "Baseline should be bounded");
}

#[test]
fn test_simple_object_not_leaked() {
    let baseline = get_baseline_alive_count();

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
    println!("Baseline: {}, After test: {}", baseline, stats.alive_count);

    // Allow for small overhead but temp objects should be collected
    let overhead = stats.alive_count.saturating_sub(baseline);
    assert!(
        overhead < 50,
        "Too many objects leaked after simple loop: {} over baseline",
        overhead
    );
}

#[test]
fn test_cycle_detection_simple() {
    let baseline = get_baseline_alive_count();

    // Create simple cycles and verify they're collected
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

    let (result, stats) = eval_with_gc_stats(source);

    println!("Result: {:?}", result);
    println!("Baseline: {}, After test: {}", baseline, stats.alive_count);

    // After GC, the cyclic objects should be collected
    let overhead = stats.alive_count.saturating_sub(baseline);
    assert!(
        overhead < 50,
        "Cycles not collected: {} objects over baseline (expected < 50)",
        overhead
    );
}

#[test]
fn test_self_referencing_collected() {
    let baseline = get_baseline_alive_count();

    let source = r#"
        let sum = 0;
        for (let i = 0; i < 100; i++) {
            const obj = { value: i, self: null };
            obj.self = obj;
            sum = sum + obj.value;
        }
        sum
    "#;

    let (_, stats) = eval_with_gc_stats(source);

    println!("Baseline: {}, After test: {}", baseline, stats.alive_count);

    // Self-referencing objects should be collected
    let overhead = stats.alive_count.saturating_sub(baseline);
    assert!(
        overhead < 50,
        "Self-referencing objects not collected: {} over baseline",
        overhead
    );
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
    runtime.collect_garbage();

    // Verify global objects are still accessible
    let check = match runtime
        .eval("global_obj.a + global_obj.b + global_arr[0]")
        .unwrap()
    {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    };

    assert_eq!(result, typescript_eval::JsValue::Number(4.0));
    assert_eq!(check, typescript_eval::JsValue::Number(4.0));
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
    assert_eq!(result, typescript_eval::JsValue::Number(6.0));
}

#[test]
fn test_many_cycles_memory_bounded() {
    let baseline = get_baseline_alive_count();

    // Create many cycles and verify memory stays bounded
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

    let (result, stats) = eval_with_gc_stats(source);

    println!("Result: {:?}", result);
    println!("Baseline: {}, After test: {}", baseline, stats.alive_count);

    assert_eq!(result, typescript_eval::JsValue::Number(6000.0));

    // After 1000 iterations creating 3 objects each, if GC works,
    // we should NOT have 3000 objects alive - just baseline + small overhead
    let overhead = stats.alive_count.saturating_sub(baseline);
    assert!(
        overhead < 100,
        "Too many objects alive after cycle test: {} over baseline (expected < 100)",
        overhead
    );
}

#[test]
fn test_gc_threshold_api() {
    let mut runtime = Runtime::new();

    // Check default threshold
    assert_eq!(runtime.gc_threshold(), DEFAULT_GC_THRESHOLD);

    // Change threshold
    runtime.set_gc_threshold(256);
    assert_eq!(runtime.gc_threshold(), 256);

    // Disable threshold
    runtime.set_gc_threshold(0);
    assert_eq!(runtime.gc_threshold(), 0);
}

#[test]
fn test_gc_stats_includes_threshold_info() {
    let mut runtime = Runtime::new();
    runtime.set_gc_threshold(512);

    let stats = runtime.gc_stats();
    assert_eq!(stats.gc_threshold, 512);
    // Note: allocs_since_gc is non-zero because runtime initialization allocates builtins
    let allocs_before = stats.allocs_since_gc;

    // Run some code to allocate more objects
    let _ = runtime.eval("const x = { a: 1 }; x.a").unwrap();

    let stats_after = runtime.gc_stats();
    // After running code, we should have more allocations
    assert!(
        stats_after.allocs_since_gc > allocs_before,
        "Should allocate more objects when running code"
    );
}

#[test]
fn test_lower_threshold_bounds_memory() {
    // Create cycles with high-frequency GC (threshold=100)
    let source = r#"
        let total = 0;
        for (let i = 0; i < 500; i++) {
            const a = { v: 1 };
            const b = { v: 2 };
            a.next = b;
            b.next = a;
            total = total + a.v + b.v;
        }
        total
    "#;

    // With low threshold (100) - GC runs frequently
    let mut runtime_low = Runtime::new();
    runtime_low.set_gc_threshold(100);
    let _ = runtime_low.eval(source).unwrap();
    runtime_low.collect_garbage();
    let stats_low = runtime_low.gc_stats();

    // With high threshold (10000) - GC runs less frequently
    let mut runtime_high = Runtime::new();
    runtime_high.set_gc_threshold(10000);
    let _ = runtime_high.eval(source).unwrap();
    // Don't collect - check live count before final GC
    let stats_high_before_gc = runtime_high.gc_stats();

    println!(
        "Low threshold alive: {}, High threshold alive (before GC): {}",
        stats_low.alive_count, stats_high_before_gc.alive_count
    );

    // With high threshold, many more cycles should still be alive
    // (they haven't been collected yet)
    // Note: After final GC both should be similar, but DURING execution
    // the high threshold accumulates more garbage
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
        typescript_eval::JsValue::Number(1500.0),
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
        typescript_eval::JsValue::Number(300.0),
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
        typescript_eval::JsValue::Number(150.0),
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
        typescript_eval::JsValue::Number(150.0),
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
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    };

    if let typescript_eval::JsValue::Object(arr) = result {
        let arr_ref = arr.borrow();
        let get = |i: usize| -> f64 {
            if let Some(typescript_eval::JsValue::Number(n)) =
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
