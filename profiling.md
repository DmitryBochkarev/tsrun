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

## Criterion Benchmarks

The project includes criterion-based microbenchmarks for the lexer and parser.

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run only lexer benchmarks
cargo bench --bench lexer

# Run only parser benchmarks
cargo bench --bench parser

# Run specific benchmark group
cargo bench --bench lexer -- lexer/throughput
cargo bench --bench parser -- parser/individual

# Quick benchmark run (fewer samples)
cargo bench -- --quick
```

### Benchmark Groups

**Lexer benchmarks** (`benches/lexer.rs`):
| Group | What it measures |
|-------|------------------|
| `lexer/individual` | Tokenization of specific code patterns (strings, operators, classes, etc.) |
| `lexer/throughput` | Throughput at different source sizes (1KB to 500KB) |
| `lexer/token_types` | Performance of tokenizing specific token types |

**Parser benchmarks** (`benches/parser.rs`):
| Group | What it measures |
|-------|------------------|
| `parser/individual` | Parsing specific code patterns |
| `parser/throughput` | Throughput at different source sizes |
| `parser/expression_depth` | Binary expression tree parsing at different depths |
| `parser/statements` | Parsing many statements (lets, functions, classes) |
| `parser/string_interning` | String dictionary performance with repeated vs unique identifiers |

### Viewing Results

Criterion generates HTML reports in `target/criterion/`:

```bash
# Open the report index
firefox target/criterion/report/index.html
```

### Comparing Against Baseline

```bash
# Save current as baseline
cargo bench -- --save-baseline main

# Make changes, then compare
cargo bench -- --baseline main
```

## Dedicated Profiling Binaries

For detailed profiling with `perf` or `flamegraph`, use the dedicated profiling binaries:

### Lexer Profiling

```bash
# Build with profiling profile
cargo build --profile profiling --bin profile_lexer

# Run with custom size and iterations
# Usage: profile_lexer [size_bytes] [iterations]
./target/profiling/profile_lexer 500000 50

# Profile with perf
perf record -g ./target/profiling/profile_lexer 500000 50
perf report --stdio --sort=symbol --no-children | head -50

# Generate flamegraph
cargo flamegraph --profile profiling --bin profile_lexer -o lexer-flamegraph.svg -- 500000 50
```

### Parser Profiling

```bash
# Build with profiling profile
cargo build --profile profiling --bin profile_parser

# Run with custom size and iterations
# Usage: profile_parser [size_bytes] [iterations]
./target/profiling/profile_parser 500000 10

# Profile with perf
perf record -g ./target/profiling/profile_parser 500000 10
perf report --stdio --sort=symbol --no-children | head -50

# Generate flamegraph
cargo flamegraph --profile profiling --bin profile_parser -o parser-flamegraph.svg -- 500000 10
```

### Example Output

```
$ ./target/profiling/profile_lexer 500000 50
Generating 488KB source...
Source size: 500202 bytes
Running 50 iterations of lexer...
Done in 319.13ms
Total tokens: 6211350
Throughput: 78.37 MB/s

$ ./target/profiling/profile_parser 500000 10
Generating 488KB source...
Source size: 500137 bytes
Running 10 iterations of parser...
Done in 217.46ms
Total statements: 22530
Throughput: 23.00 MB/s
```

## Lexer/Parser Optimization Guide

### Common Lexer Hotspots

| Symbol | Typical % | What it does | Optimization hints |
|--------|-----------|--------------|-------------------|
| `Lexer::next_token` | 15-20% | Main tokenization loop | Core work, hard to optimize |
| `__memcmp` / `str::eq` | 10-15% | Keyword matching | Use perfect hash or trie |
| `String::push` | 5-10% | Building identifier/string tokens | Pre-allocate capacity |
| `Lexer::advance` | 5-10% | Character iteration | Core work |
| `drop_in_place<Token>` | 5-10% | Token cleanup | Reduce token allocations |

### Common Parser Hotspots

| Symbol | Typical % | What it does | Optimization hints |
|--------|-----------|--------------|-------------------|
| `Parser::advance` | 5-10% | Token advancement | Core work |
| `Parser::parse_*` | varies | Parse functions | Reduce backtracking |
| `Rc::new` / `drop<Rc>` | 5-10% | AST node allocation | Use arena allocator |
| `malloc` / `free` | 5-10% | Memory allocation | Reduce allocations |
| `Hash::hash` | 2-5% | String interning | Already using FxHashMap |

### Red Flags in Profiler Output

Watch for these patterns that indicate problems:

1. **`Lexer::restore` > 1%**: The checkpoint/restore mechanism should be O(1). If it shows up, the lexer is recreating iterators inefficiently.

2. **`Peekable::peek` > 5%** in parser: Excessive lookahead or backtracking.

3. **`clone` > 10%**: Too much cloning. Use `Rc` or references.

4. **`CharIndices::next` > 10%** outside lexer: Iterator being recreated instead of reused.

## Hyperfine for Quick Comparisons

For quick A/B comparisons between commits:

```bash
# Install hyperfine
cargo install hyperfine

# Compare two versions
git stash
cargo build --release
cp target/release/typescript-eval-runner /tmp/baseline

git stash pop
cargo build --release

hyperfine \
    '/tmp/baseline examples/memory-management/stress-test.ts' \
    './target/release/typescript-eval-runner examples/memory-management/stress-test.ts' \
    --warmup 3
```
