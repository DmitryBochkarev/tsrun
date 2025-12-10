# Profiling typescript-eval

This document describes how to profile the typescript-eval interpreter to identify performance bottlenecks.

## Build Profiles

The project has three build profiles:

| Profile | Command | Use Case |
|---------|---------|----------|
| `dev` | `cargo build` | Development, debugging |
| `release` | `cargo build --release` | Production, benchmarks |
| `profiling` | `cargo build --profile profiling` | Performance analysis with symbols |

The `profiling` profile inherits from `release` but keeps debug symbols and doesn't strip the binary, which is essential for meaningful profiler output.

## Quick Performance Check

Use `/usr/bin/time -v` for a quick overview of execution time and memory usage:

```bash
# Build release binary
cargo build --release

# Run with timing
/usr/bin/time -v ./target/release/typescript-eval-runner examples/memory-management/stress-test.ts
```

Key metrics to watch:
- **User time (seconds)**: CPU time spent in user mode
- **Maximum resident set size (kbytes)**: Peak memory usage
- **Minor page faults**: Memory allocation activity

## Profiling with perf

### Basic CPU Profiling

```bash
# Build with profiling profile (includes debug symbols)
cargo build --profile profiling

# Record performance data
perf record -g ./target/profiling/typescript-eval-runner examples/memory-management/stress-test.ts

# View flat profile (top functions by self time)
perf report --stdio --sort=symbol --no-children | head -50

# View call graph
perf report --stdio --sort=symbol | head -80
```

### Hardware Performance Counters

```bash
# Get detailed CPU statistics
perf stat -e cycles,instructions,cache-references,cache-misses \
    ./target/release/typescript-eval-runner examples/memory-management/stress-test.ts
```

Key metrics:
- **insn per cycle (IPC)**: Higher is better (max ~4-6 on modern CPUs)
- **cache-misses %**: Lower is better (<5% is good)

## Generating Flamegraphs

Flamegraphs provide a visual representation of where time is spent:

```bash
# Install flamegraph tool (one-time)
cargo install flamegraph

# Generate flamegraph (requires profiling build for good symbols)
cargo flamegraph --profile profiling --bin typescript-eval-runner \
    -o flamegraph.svg -- examples/memory-management/stress-test.ts

# Open in browser
firefox flamegraph.svg
```

## Interpreting Profiler Output

### Example perf report output

```
# Overhead  Symbol
# ........  .......................................
    19.27%  typescript_eval::interpreter::Interpreter::evaluate_member
     9.15%  <alloc::rc::Rc<T,A> as core::hash::Hash>::hash
     5.84%  typescript_eval::interpreter::Interpreter::evaluate
     5.54%  typescript_eval::value::JsObject::get_property_descriptor
     5.03%  typescript_eval::interpreter::Interpreter::execute_for_labeled
     4.29%  hashbrown::raw::RawTable<T,A>::find
```

### Common hotspots and their meanings

| Symbol | What it does | Optimization hints |
|--------|--------------|-------------------|
| `evaluate_member` | Property access (`obj.prop`) | Reduce property lookups, cache results |
| `Rc::hash` / `Hash::hash` | HashMap key hashing | Use faster hasher (FxHashMap) |
| `RawTable::find` | HashMap lookup | Reduce map operations, use faster hasher |
| `evaluate` | Expression evaluation | Core interpreter work, hard to optimize |
| `get_property_descriptor` | Prototype chain lookup | Flatten prototype chains |
| `execute_for_labeled` | For loop execution | Core interpreter work |
| `malloc` / `cfree` | Memory allocation | Reduce allocations, use arena allocators |
| `clone` | Value cloning | Use `Rc` for shared ownership |

## Benchmarking Best Practices

### Consistent benchmarking environment

```bash
# Disable CPU frequency scaling (requires root)
sudo cpupower frequency-set --governor performance

# Run multiple times and take the median
for i in {1..5}; do
    /usr/bin/time -f "%e" ./target/release/typescript-eval-runner \
        examples/memory-management/stress-test.ts 2>&1 | tail -1
done
```

### Comparing before/after changes

```bash
# Save baseline
git stash  # or checkout baseline commit
cargo build --release
/usr/bin/time -v ./target/release/typescript-eval-runner \
    examples/memory-management/stress-test.ts 2>&1 | tee baseline.txt

# Apply changes
git stash pop  # or checkout new commit
cargo build --release
/usr/bin/time -v ./target/release/typescript-eval-runner \
    examples/memory-management/stress-test.ts 2>&1 | tee optimized.txt

# Compare
diff baseline.txt optimized.txt
```

## Test Files for Profiling

The `examples/memory-management/` directory contains good profiling targets:

| File | What it tests |
|------|---------------|
| `stress-test.ts` | Object creation, property access, closures |
| `main.ts` | Comprehensive test (imports all other tests) |
| `object-churn.ts` | Heavy object allocation/deallocation |
| `closure-lifetime.ts` | Closure creation and capture |
| `circular-refs.ts` | Circular reference handling |
| `scope-cleanup.ts` | Scope/environment management |

## Memory Profiling with Valgrind

```bash
# Build debug binary (more accurate but slower)
cargo build

# Check for memory leaks
valgrind --leak-check=full \
    ./target/debug/typescript-eval-runner examples/memory-management/main.ts

# Profile memory allocation patterns
valgrind --tool=massif \
    ./target/debug/typescript-eval-runner examples/memory-management/stress-test.ts

# View massif output
ms_print massif.out.*
```

## Profiling Specific Operations

### Profile only parsing

```bash
# Create a test that only parses (no execution)
cat > /tmp/parse-only.rs << 'EOF'
use typescript_eval::parser::parse;
use std::fs;

fn main() {
    let source = fs::read_to_string("examples/memory-management/stress-test.ts").unwrap();
    for _ in 0..1000 {
        let _ = parse(&source);
    }
}
EOF
```

### Profile with specific workload

```bash
# Create a focused test case
cat > /tmp/test.ts << 'EOF'
// Test specific feature
for (let i = 0; i < 100000; i++) {
    const obj = { x: i, y: i * 2 };
    const sum = obj.x + obj.y;
}
EOF

cargo build --profile profiling
perf record -g ./target/profiling/typescript-eval-runner /tmp/test.ts
perf report --stdio
```

## Optimization Checklist

When profiling reveals a bottleneck, consider:

1. **Hashing overhead (>10%)**
   - Switch from `std::HashMap` to `rustc_hash::FxHashMap`
   - Pre-compute hash values for frequently used keys

2. **String operations (>5%)**
   - Use `Rc<str>` (JsString) instead of `String`
   - Avoid `to_string()` in hot paths
   - Use direct comparison methods like `eq_str()`

3. **Memory allocation (>5%)**
   - Use object pools or arenas
   - Reduce cloning with `Rc` shared ownership
   - Pre-allocate vectors with known capacity

4. **Property lookup (>10%)**
   - Cache prototype chain lookups
   - Use inline caches for repeated property access
   - Consider shape/hidden class optimization

## Continuous Performance Monitoring

Add performance tests to CI:

```bash
# In CI script
cargo build --release
BASELINE=0.15  # seconds
ACTUAL=$(/usr/bin/time -f "%e" ./target/release/typescript-eval-runner \
    examples/memory-management/stress-test.ts 2>&1)

if (( $(echo "$ACTUAL > $BASELINE * 1.2" | bc -l) )); then
    echo "Performance regression: ${ACTUAL}s > ${BASELINE}s * 1.2"
    exit 1
fi
```
