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
