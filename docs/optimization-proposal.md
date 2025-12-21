# Performance Optimization Proposal

Based on profiling the interpreter with `perf` on compute-intensive workloads, here are the top hotspots and proposed optimizations.

## Profiling Summary

### VM Execution (compute-intensive.ts)

| Symbol | Overhead | Description |
|--------|----------|-------------|
| `BytecodeVM::run` | 13.5% | Main bytecode loop |
| `Vec::extend_with` | 7.4% | Register file allocation |
| `Interpreter::env_get` | 5.3-6.3% | Variable lookup |
| `BytecodeVM::execute_op` | 5.5% | Opcode dispatch |
| `Guard::alloc` | 5.5% | GC guard allocation |
| `BytecodeVM::set_reg` | 4.6% | Register writes with guard/unguard |
| `HashMap::insert` | 3.0% | Environment binding creation |
| `push_trampoline_frame_and_call_bytecode` | 2.8% | Function call setup |
| `hash_one` / `Hasher::write_str` | 2.2% | Hashing for lookups |

### Lexer (profile_lexer)

| Symbol | Overhead | Description |
|--------|----------|-------------|
| `__memcmp_evex_movbe` | 13.2% | Keyword matching (string comparison) |
| `Lexer::advance` | 8.5% | Character iteration |
| `String::push` | 6.3% | Building tokens |
| `str::eq` | 5.5% | String equality |
| `cfree` | 5.1% | Token deallocation |
| `Lexer::peek` | 5.4% | Lookahead |
| `CharIndices::next` | 5.1% | Iterator |
| `StringDict::get_or_insert` | 2.9% | String interning |

### Parser (profile_parser)

| Symbol | Overhead | Description |
|--------|----------|-------------|
| `Lexer::next_token` | 16.0% | Tokenization |
| `Parser::advance` | 11.0% | Token advancement |
| `__memcmp_evex_movbe` | 5.9% | String comparison |
| `parse_member_expression` | 4.2% | Member access parsing |
| `parse_unary_expression` | 3.6% | Unary parsing |
| `parse_binary_expression` | 3.6% | Binary parsing |
| `malloc/_int_malloc` | 2.3% | Memory allocation |
| `cfree/_int_free` | 3.9% | Memory deallocation |

---

## Optimization Proposals

### Priority 1: Register Allocation Overhead (High Impact, Medium Effort)

**Problem:** `vec![JsValue::Undefined; register_count]` allocates and initializes a new Vec for every function call (~7.4% overhead in `extend_with`).

**Current code (bytecode_vm.rs:1061):**
```rust
let mut new_registers = vec![JsValue::Undefined; register_count.max(1)];
```

**Proposed solution:** Use a register pool with pre-allocated frames.

```rust
struct RegisterPool {
    frames: Vec<Vec<JsValue>>,
    size_hint: usize,
}

impl RegisterPool {
    fn acquire(&mut self, size: usize) -> Vec<JsValue> {
        // Try to find an existing frame of sufficient size
        if let Some(pos) = self.frames.iter().position(|f| f.capacity() >= size) {
            let mut frame = self.frames.swap_remove(pos);
            frame.clear();
            frame.resize(size, JsValue::Undefined);
            return frame;
        }
        // Allocate new frame
        vec![JsValue::Undefined; size.max(self.size_hint)]
    }

    fn release(&mut self, mut frame: Vec<JsValue>) {
        frame.clear();
        self.frames.push(frame);
    }
}
```

**Expected impact:** 5-7% improvement on function-heavy code.

---

### Priority 2: set_reg Guard Overhead (Medium Impact, Medium Effort)

**Problem:** Every register write guards new objects and unguards old ones (~4.6% overhead), even for primitive values.

**Current code (bytecode_vm.rs:323-342):**
```rust
pub fn set_reg(&mut self, r: Register, value: JsValue) {
    if let JsValue::Object(obj) = &value {
        self.register_guard.guard(obj.clone());
    }
    if let JsValue::Object(obj) = &slot {
        self.register_guard.unguard(obj);
    }
    *slot = value;
}
```

**Proposed solutions:**

A. **Lazy guarding:** Only guard objects when suspending or crossing function boundaries.

B. **Batch guarding:** Defer guard updates until end of basic block.

C. **Split register types:** Track which registers hold objects separately.

```rust
// Option C: Type-aware registers
struct TypedRegisters {
    values: Vec<JsValue>,
    object_mask: BitVec,  // Track which registers contain objects
}
```

**Expected impact:** 3-5% improvement.

---

### Priority 3: Environment Lookup (Medium Impact, High Effort)

**Problem:** `env_get` walks the scope chain on every variable access (~5-6% overhead), even for local variables.

**Current approach:** Linear scope chain walk with HashMap lookup at each level.

**Proposed solutions:**

A. **Lexical depth tracking (compile-time):** Compiler records the scope depth and slot index for each variable. VM uses direct indexing.

```rust
enum VarLoc {
    Local(Register),           // Already in register
    Closure { depth: u16, slot: u16 },  // Closure variable
    Global(ConstantIndex),     // Global name
}

// Bytecode becomes:
Op::GetClosure { dst, depth, slot }  // Direct access, no hash lookup
```

B. **Flat closure conversion:** Convert free variables to explicit closure slots during compilation.

**Expected impact:** 4-6% improvement on closure-heavy code.

---

### Priority 4: Keyword Matching in Lexer (Medium Impact, Low Effort)

**Problem:** 13.2% of lexer time in `memcmp` for keyword matching.

**Current approach:** Linear search or hash lookup with string comparison.

**Proposed solutions:**

A. **Perfect hash function:** Use `phf` crate for O(1) keyword lookup.

```rust
static KEYWORDS: phf::Map<&'static str, TokenKind> = phf_map! {
    "if" => TokenKind::If,
    "else" => TokenKind::Else,
    // ...
};
```

B. **Length-prefixed dispatch:** First dispatch on string length, then compare.

```rust
fn match_keyword(s: &str) -> Option<TokenKind> {
    match s.len() {
        2 => match s { "if" => Some(If), "in" => Some(In), "do" => Some(Do), _ => None },
        3 => match s { "for" => Some(For), "let" => Some(Let), "var" => Some(Var), _ => None },
        // ...
    }
}
```

**Expected impact:** 5-8% improvement in lexer throughput.

---

### Priority 5: Token Allocation (Low Impact, Medium Effort)

**Problem:** 5% overhead from token allocation/deallocation (`String::push`, `cfree`).

**Proposed solutions:**

A. **Pre-allocated token buffers:** Reuse token string storage.

B. **Interning during lexing:** Use StringDict for all identifier/string tokens immediately.

C. **Arena allocation:** Use a bump allocator for tokens within a single file.

**Expected impact:** 3-5% improvement in lexer throughput.

---

### Priority 6: Trampoline Call Setup (Low Impact, High Effort)

**Problem:** 2.8% overhead in function call setup (saving/restoring VM state).

**Current approach:** Every call pushes a full TrampolineFrame with all VM state.

**Proposed solutions:**

A. **Inline caching for hot calls:** Cache compiled callees for polymorphic call sites.

B. **Tail call optimization:** Avoid frame creation for tail calls.

C. **Caller-save registers:** Only save registers that are live across the call.

**Expected impact:** 2-3% improvement.

---

## Implementation Priority Matrix

| Optimization | Impact | Effort | Priority |
|-------------|--------|--------|----------|
| Register pool | 5-7% | Medium | **P1** |
| Keyword perfect hash | 5-8% | Low | **P1** |
| set_reg lazy guarding | 3-5% | Medium | **P2** |
| Environment lookup | 4-6% | High | **P2** |
| Token allocation | 3-5% | Medium | **P3** |
| Trampoline optimization | 2-3% | High | **P3** |

---

## Quick Wins (Immediate Implementation)

### 1. Add `#[inline]` to hot paths ✅ DONE

Added `#[inline]` to the following hot path functions in `bytecode_vm.rs`:
- `get_reg` (was already present)
- `set_reg` ✅
- `fetch` ✅
- `get_constant` (was already present)
- `get_string_constant` ✅

### 2. Use `FxHashMap` for environments (if not already)

Check if environment bindings use `FxHashMap` instead of `std::HashMap`.

### 3. Pre-size vectors with known capacity

Already done in many places, verify all allocation sites.

---

## Benchmarking Commands

```bash
# Quick timing comparison
hyperfine \
    './target/release/typescript-eval-runner examples/profiling/compute-intensive.ts' \
    --warmup 3

# Profile specific operations
perf record -g ./target/profiling/typescript-eval-runner examples/profiling/compute-intensive.ts
perf report --stdio --sort=symbol --no-children | head -50

# Lexer throughput
./target/profiling/profile_lexer 500000 50

# Parser throughput
./target/profiling/profile_parser 500000 10
```

---

## Metrics to Track

| Metric | Current | Target |
|--------|---------|--------|
| Lexer throughput | ~65-73 MB/s | 100+ MB/s |
| Parser throughput | ~20 MB/s | 35+ MB/s |
| compute-intensive.ts | 242 ms | 180 ms |
| stress-test.ts | 128 ms | 100 ms |
| Fibonacci(30) | 2298 ms | 1500 ms |

Run benchmarks before/after each optimization to measure actual impact.

## Baseline Commands

```bash
# Baseline measurements (run with warmup)
hyperfine './target/release/typescript-eval-runner examples/profiling/compute-intensive.ts' --warmup 3
hyperfine './target/release/typescript-eval-runner examples/profiling/fibonacci.ts' --warmup 3
hyperfine './target/release/typescript-eval-runner examples/memory-management/stress-test.ts' --warmup 3

# Lexer/parser profiling
./target/profiling/profile_lexer 500000 50
./target/profiling/profile_parser 500000 10
```

---

## Memory Profiling Analysis

### Peak Memory Usage

| Test | Peak Heap | Useful Heap |
|------|-----------|-------------|
| stress-test.ts | ~400 KB | ~385 KB |
| compute-intensive.ts | ~2 MB | ~2 MB |

Memory usage is well-controlled with effective GC.

### Allocation Hotspots (DHAT Analysis)

**Fibonacci(30) - 1 GB total allocations:**

| Rank | Bytes | Allocs | Source |
|------|-------|--------|--------|
| 1 | 469 MB | 5.8T | HashMap allocation (environment bindings) |
| 2 | 344 MB | 1.6T | `vec![JsValue::Undefined; n]` (register files) |
| 3 | 51 MB | 4.6T | `Rc<GuardInner>` (GC guards) |
| 4 | 51 MB | 4.5T | `Rc<GuardInner>` (GC guards) |
| 5 | 34 MB | 39B | `Vec::with_capacity` (register files) |
| 6 | 34 MB | 4.3B | `to_vec()` (cloning args) |
| 7 | 34 MB | 1.6T | `to_vec()` (cloning args) |

**Key Finding:** The interpreter allocates **~1 GB** for Fibonacci(30), with:
- **~469 MB (45%)** for HashMap (environment bindings per call)
- **~412 MB (40%)** for register file allocation
- **~102 MB (10%)** for GC guard Rc allocations

### Memory Optimization Proposals

#### M1: Environment Binding Pool (High Impact)

**Problem:** Every function call creates a new `FxHashMap` for bindings (~469 MB for fib(30)).

**Proposed solution:** Use a slab allocator or object pool for environment objects.

```rust
struct EnvironmentPool {
    free_list: Vec<Gc<JsObject>>,
}

impl EnvironmentPool {
    fn acquire(&mut self, guard: &Guard<JsObject>) -> Gc<JsObject> {
        if let Some(env) = self.free_list.pop() {
            // Clear and reuse
            env.borrow_mut().clear_bindings();
            return env;
        }
        guard.alloc()  // Allocate new if pool empty
    }

    fn release(&mut self, env: Gc<JsObject>) {
        self.free_list.push(env);
    }
}
```

**Expected impact:** 30-40% reduction in allocations for function-heavy code.

---

#### M2: Register File Reuse (High Impact)

**Problem:** Each call allocates `vec![JsValue::Undefined; register_count]` (~412 MB for fib(30)).

Already proposed in CPU optimization section. This is confirmed as a major memory issue.

**Expected impact:** 30-40% reduction in allocations.

---

#### M3: Reduce Guard Allocations (Medium Impact)

**Problem:** ~102 MB spent on `Rc<GuardInner>` allocations.

**Proposed solutions:**

A. **Guard pooling:** Reuse guard objects instead of allocating new ones.

B. **Inline guards:** For short-lived operations, use stack-allocated guards.

```rust
// Current: heap-allocated guard
let guard = heap.create_guard();

// Proposed: stack guard for short operations
let guard = heap.stack_guard();  // No allocation
```

**Expected impact:** 10% reduction in allocations.

---

#### M4: Avoid Argument Cloning (Medium Impact)

**Problem:** `to_vec()` clones argument vectors (~68 MB for fib(30)).

**Current code pattern:**
```rust
fn call_function(args: Vec<JsValue>) {
    // args already owned, but we clone internally
    let processed = args.clone();
}
```

**Proposed:** Pass by reference where possible, or take ownership explicitly.

**Expected impact:** 5-10% reduction in allocations.

---

### Memory Commands

```bash
# Heap profiling with massif
valgrind --tool=massif ./target/debug/typescript-eval-runner examples/profiling/fibonacci.ts
ms_print massif.out.*

# Allocation site profiling with DHAT
valgrind --tool=dhat ./target/debug/typescript-eval-runner examples/profiling/compute-intensive.ts
# Then open dh_view.html and load dhat.out.*

# Quick memory stats
/usr/bin/time -v ./target/release/typescript-eval-runner examples/profiling/fibonacci.ts
```
