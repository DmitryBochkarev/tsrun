//! Test for cycle memory leak

use typescript_eval::{Runtime, RuntimeResult};

/// Get baseline object count (builtins only, no user code)
fn get_baseline_live_count() -> usize {
    let runtime = Runtime::new();
    runtime.collect();
    runtime.gc_stats().live_objects
}

#[test]
fn test_cycle_leak_detailed() {
    let baseline = get_baseline_live_count();
    println!("Baseline live count: {}", baseline);

    // Create many cycles and measure memory after full GC
    let source = r#"
        let sum = 0;
        for (let i = 0; i < 1000; i++) {
            const a = { id: i, other: null };
            const b = { id: i + 1, other: null };
            a.other = b;
            b.other = a;
            sum = sum + a.id + b.id;
        }
        sum
    "#;

    let mut runtime = Runtime::new();
    runtime.set_gc_threshold(50); // Trigger GC frequently
    let result = match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    };

    // Force final GC
    runtime.collect();
    let stats = runtime.gc_stats();

    println!("Result: {:?}", result);
    println!(
        "After cycles: total={}, pooled={}, live={}",
        stats.total_objects, stats.pooled_objects, stats.live_objects
    );

    // If cycles were properly collected, live count should be close to baseline
    // 1000 iterations * 2 objects per cycle = 2000 objects potentially leaked
    let overhead = stats.live_objects.saturating_sub(baseline);
    println!("Overhead over baseline: {}", overhead);

    // The test: if cycles are leaking, we'll have 2000+ extra objects
    assert!(
        overhead < 200,
        "Too many objects retained after GC: {} over baseline (cycles may be leaking)",
        overhead
    );
}
