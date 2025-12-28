# Profiling tsrun

This document describes how to profile the tsrun interpreter to identify performance bottlenecks.

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
/usr/bin/time -v ./target/release/tsrun examples/memory-management/gc-no-cycles.ts
```

Example output:
```
User time (seconds): 0.09
Maximum resident set size (kbytes): 4284
Minor (reclaiming a frame) page faults: 344
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
perf record -g ./target/profiling/tsrun examples/memory-management/gc-no-cycles.ts

# View flat profile (top functions by self time)
perf report --stdio --sort=symbol --no-children | head -50

# View call graph
perf report --stdio --sort=symbol | head -80
```

### Hardware Performance Counters

```bash
# Get detailed CPU statistics
perf stat -e cycles,instructions,cache-references,cache-misses \
    ./target/profiling/tsrun examples/memory-management/gc-no-cycles.ts
```

Example output:
```
   492,057,481      cycles:u
 1,544,622,706      instructions:u                   #    3.14  insn per cycle
    19,850,244      cache-references:u
        79,108      cache-misses:u                   #    0.40% of all cache refs
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
cargo flamegraph --profile profiling --bin tsrun \
    -o flamegraph.svg -- examples/memory-management/gc-no-cycles.ts

# Open in browser
firefox flamegraph.svg
```

## Interpreting Profiler Output

### Example perf report output

From profiling `gc-no-cycles.ts` (50K iterations, 150K objects):

```
# Overhead  Symbol
# ........  .......................................
    20.82%  tsrun::interpreter::bytecode_vm::BytecodeVM::run
     8.79%  tsrun::interpreter::bytecode_vm::BytecodeVM::execute_op
     8.67%  tsrun::interpreter::Interpreter::env_get
     5.10%  tsrun::gc::Guard<T>::alloc
     4.41%  tsrun::value::PropertyStorage::iter
     4.12%  tsrun::interpreter::bytecode_vm::BytecodeVM::set_reg
     3.58%  core::ptr::drop_in_place<Rc<RefCell<Space>>>
     2.83%  <PropertyStorageIter as Iterator>::next
     2.38%  alloc::vec::Vec<T,A>::push
     2.32%  alloc::rc::Weak<T,A>::upgrade
```

### Common hotspots and their meanings

| Symbol | What it does | Optimization hints |
|--------|--------------|-------------------|
| `BytecodeVM::run` | Main VM execution loop | Core work, hard to optimize |
| `BytecodeVM::execute_op` | Opcode dispatch | Core work |
| `env_get` | Variable lookup | Reduce scope chain depth |
| `Guard::alloc` | GC object allocation | Reduce allocations, reuse objects |
| `PropertyStorage::iter` | Property iteration | Reduce property access in loops |
| `set_reg` | VM register writes | Core work |
| `drop_in_place<Rc>` | Reference counting cleanup | Reduce Rc usage in hot paths |
| `Weak::upgrade` | Weak reference upgrade | GC bookkeeping |
| `malloc` / `cfree` | Memory allocation | Reduce allocations, use arena allocators |
| `clone` | Value cloning | Use `Rc` for shared ownership |

## Benchmarking Best Practices

### Consistent benchmarking environment

```bash
# Disable CPU frequency scaling (requires root)
sudo cpupower frequency-set --governor performance

# Run multiple times and take the median
for i in {1..5}; do
    /usr/bin/time -f "%e" ./target/release/tsrun \
        examples/memory-management/gc-no-cycles.ts 2>&1 | tail -1
done
```

### Comparing before/after changes

```bash
# Save baseline
git stash  # or checkout baseline commit
cargo build --release
/usr/bin/time -v ./target/release/tsrun \
    examples/memory-management/gc-no-cycles.ts 2>&1 | tee baseline.txt

# Apply changes
git stash pop  # or checkout new commit
cargo build --release
/usr/bin/time -v ./target/release/tsrun \
    examples/memory-management/gc-no-cycles.ts 2>&1 | tee optimized.txt

# Compare
diff baseline.txt optimized.txt
```

## Test Files for Profiling

The `examples/memory-management/` directory contains good profiling targets:

| File | What it tests |
|------|---------------|
| `gc-no-cycles.ts` | Baseline: 150K simple objects without cycles (recommended starting point) |
| `gc-scale-test.ts` | Same object count WITH circular references (compare to baseline) |
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
    ./target/debug/tsrun examples/memory-management/gc-no-cycles.ts

# Profile memory allocation patterns
valgrind --tool=massif \
    ./target/debug/tsrun examples/memory-management/gc-no-cycles.ts

# View massif output
ms_print massif.out.*

# Clean up
rm -f massif.out.*
```

### Example massif output

From profiling `gc-no-cycles.ts`:

```
    KB
343.0^#
     |#:::::::::::@:::@:::::::@:::::::@::::::@::::::@::::::@
   0 +----------------------------------------------------------------------->
```

**Summary:**
| Metric | Value |
|--------|-------|
| Peak heap usage | 343 KB |
| Total allocations | 357,032 |
| Total bytes allocated | 57 MB |
| Memory leaks | 0 bytes |

**Allocation breakdown at peak:**
| % | Source |
|---|--------|
| 91% | `regex_automata` - Lexer regex compilation (one-time startup) |
| 4% | `Rc<Expression>` - AST nodes |
| 3% | `JsString` - String interning |
| 1% | `Guard::alloc` - GC object allocation |
| 1% | Parser vectors |

The flat memory profile confirms GC is working - 150K objects created but peak stays at 343 KB.

## Profiling Specific Operations

### Profile only parsing

```bash
# Create a test that only parses (no execution)
cat > /tmp/parse-only.rs << 'EOF'
use tsrun::parser::parse;
use std::fs;

fn main() {
    let source = fs::read_to_string("examples/memory-management/gc-no-cycles.ts").unwrap();
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
perf record -g ./target/profiling/tsrun /tmp/test.ts
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
BASELINE=0.10  # seconds (gc-no-cycles.ts baseline)
ACTUAL=$(/usr/bin/time -f "%e" ./target/release/tsrun \
    examples/memory-management/gc-no-cycles.ts 2>&1)

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
cp target/release/tsrun /tmp/baseline

git stash pop
cargo build --release

hyperfine \
    '/tmp/baseline examples/memory-management/gc-no-cycles.ts' \
    './target/release/tsrun examples/memory-management/gc-no-cycles.ts' \
    --warmup 3
```
