# Binary Size Reduction Plan

This document outlines code changes to further reduce the binary size of typescript-eval beyond the Cargo.toml optimizations already applied.

## Current State (After Tier 2)

| Metric | Before | After Tier 2 | Reduction |
|--------|--------|--------------|-----------|
| Binary size | 4.8 MB | 1.6 MB | 67% |
| .text section | 2.2 MB | 750 KB | 66% |

### Current Size Breakdown

| Component | Size | Notes |
|-----------|------|-------|
| typescript_eval | 252 KB | Parser, interpreter, builtins |
| std | 273 KB | Rust standard library |
| regex_syntax | 72 KB | Regex parsing |
| regex_automata | 60 KB | Regex execution |
| chrono | 19 KB | Date handling |
| serde_json | 13 KB | JSON parsing |

---

## Proposed Code Changes

### 1. Make RegExp Support Optional

**Estimated savings**: 130-150 KB

**Rationale**: Many config/manifest use cases don't need RegExp. Making it optional allows smaller binaries for simple use cases.

**Implementation**:

```toml
# Cargo.toml
[features]
default = ["regexp"]
regexp = ["regex"]

[dependencies]
regex = { version = "1.10", default-features = false, features = ["std", "unicode-perl", "unicode-case"], optional = true }
```

```rust
// src/interpreter/builtins/mod.rs
#[cfg(feature = "regexp")]
pub mod regexp;

// src/interpreter/mod.rs
#[cfg(feature = "regexp")]
use builtins::regexp::*;

impl Interpreter {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "regexp")]
            regexp_prototype: create_regexp_prototype(),
            #[cfg(feature = "regexp")]
            regexp_constructor: create_regexp_constructor(&regexp_prototype),
            // ...
        }
    }
}
```

**Files to modify**:
- `Cargo.toml`
- `src/lib.rs`
- `src/interpreter/mod.rs`
- `src/interpreter/builtins/mod.rs`
- `src/interpreter/builtins/string.rs` (conditional regex in replace/split)
- `src/value.rs` (ExoticObject::RegExp variant)
- `src/parser.rs` (regex literal parsing)

---

### 2. Make Date Support Optional

**Estimated savings**: 20-30 KB (chrono removal)

**Rationale**: Date operations aren't needed for all config generation scenarios.

**Implementation**:

```toml
# Cargo.toml
[features]
default = ["regexp", "date"]
date = ["chrono"]

[dependencies]
chrono = { version = "0.4", default-features = false, features = ["clock", "std"], optional = true }
```

**Files to modify**:
- `Cargo.toml`
- `src/interpreter/mod.rs`
- `src/interpreter/builtins/mod.rs`
- `src/interpreter/builtins/date.rs`

---

### 3. Use Rc<T> for Large AST Nodes

**Estimated savings**: 10-20 KB (reduced Clone impls)

**Rationale**: The `Expression` and `Statement` enums are cloned frequently. Using `Rc` makes clones O(1) and reduces generated code for deep Clone implementations.

**Current problem** (from cargo-bloat):
```
<typescript_eval::ast::Expression as core::clone::Clone>::clone  ~30 KB total (multiple monomorphizations)
<typescript_eval::ast::Statement as core::clone::Clone>::clone   ~12 KB total
```

**Implementation**:

```rust
// src/ast.rs
use std::rc::Rc;

// Wrap large recursive types
pub type ExpressionRef = Rc<Expression>;
pub type StatementRef = Rc<Statement>;

// Update struct fields that hold Expression/Statement
pub struct BinaryExpression {
    pub left: ExpressionRef,   // was: Box<Expression>
    pub right: ExpressionRef,  // was: Box<Expression>
    pub operator: BinaryOperator,
}
```

**Trade-offs**:
- Pro: Cheaper clones, smaller binary
- Con: Slightly more indirection, needs careful lifetime management

**Files to modify**:
- `src/ast.rs` (all Box<Expression> and Box<Statement>)
- `src/parser.rs` (wrap expressions in Rc::new())
- `src/interpreter/mod.rs` (dereference Rc where needed)

---

### 4. Consolidate Similar Builtin Implementations

**Estimated savings**: 5-10 KB

**Rationale**: Many array methods have similar patterns (filter, map, find, some, every). Using a shared implementation with callbacks reduces code duplication.

**Current duplication**:
```rust
// array_filter, array_map, array_find, array_some, array_every
// All iterate over array elements and call a callback
```

**Implementation**:

```rust
// src/interpreter/builtins/array.rs

enum ArrayIteratorOp {
    Filter,
    Map,
    Find,
    FindIndex,
    Some,
    Every,
}

fn array_iterate_with_callback(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
    op: ArrayIteratorOp,
) -> Result<JsValue, JsError> {
    // Shared implementation
}

pub fn array_filter(interp: &mut Interpreter, this: JsValue, args: &[JsValue]) -> Result<JsValue, JsError> {
    array_iterate_with_callback(interp, this, args, ArrayIteratorOp::Filter)
}
```

**Files to modify**:
- `src/interpreter/builtins/array.rs`

---

### 5. Use regex-lite Instead of regex

**Estimated savings**: 80-100 KB

**Rationale**: `regex-lite` is a smaller, simpler regex implementation. It lacks some advanced features but may be sufficient for typical JS regex usage.

**Trade-offs**:
- Pro: ~100 KB smaller
- Con: Slower, lacks some features (lookahead, backreferences)

**Implementation**:

```toml
# Cargo.toml
[features]
default = ["regexp"]
regexp = ["regex-lite"]
regexp-full = ["regex"]  # For users needing full regex

[dependencies]
regex-lite = { version = "0.1", optional = true }
regex = { version = "1.10", ..., optional = true }
```

**Files to modify**:
- `Cargo.toml`
- `src/interpreter/builtins/regexp.rs` (API is mostly compatible)
- `src/interpreter/builtins/string.rs`

---

### 6. Lazy Initialization of Builtins

**Estimated savings**: Compile-time only (faster startup)

**Rationale**: Not all scripts use all builtins. Lazy initialization could reduce startup time and potentially allow dead code elimination.

**Implementation**:

```rust
// src/interpreter/mod.rs
use std::cell::OnceCell;

pub struct Interpreter {
    array_prototype: OnceCell<JsObjectRef>,
    // ...
}

impl Interpreter {
    fn get_array_prototype(&self) -> &JsObjectRef {
        self.array_prototype.get_or_init(|| create_array_prototype())
    }
}
```

**Trade-offs**:
- Pro: Faster startup for simple scripts
- Con: First access is slower, more complex code

---

### 7. Split Library into Separate Crates

**Estimated savings**: Build-time optimization, better dead code elimination

**Rationale**: Splitting into `typescript-eval-parser`, `typescript-eval-runtime`, `typescript-eval-builtins` allows users to depend only on what they need.

**Structure**:
```
typescript-eval/
├── crates/
│   ├── parser/      # Lexer, Parser, AST
│   ├── runtime/     # Interpreter core, values
│   └── builtins/    # Built-in objects (Array, String, etc.)
└── src/
    └── lib.rs       # Re-exports everything
```

---

## Priority Matrix

| Change | Effort | Impact | Risk | Priority |
|--------|--------|--------|------|----------|
| 1. Optional RegExp | Medium | High (130KB) | Low | **High** |
| 2. Optional Date | Low | Medium (25KB) | Low | **High** |
| 3. Rc for AST | High | Medium (15KB) | Medium | Medium |
| 4. Consolidate builtins | Medium | Low (8KB) | Low | Medium |
| 5. regex-lite | Low | High (90KB) | Medium | Medium |
| 6. Lazy builtins | Medium | Low | Low | Low |
| 7. Split crates | High | Variable | Medium | Low |

## Recommended Implementation Order

1. **Optional RegExp + Date** (Low risk, high impact)
2. **regex-lite option** (Easy to add as alternative)
3. **Consolidate array builtins** (Clean refactoring)
4. **Rc for AST** (Requires careful testing)

## Testing Requirements

After each change:
1. Run full test suite: `cargo test`
2. Verify binary size: `ls -lh target/release/typescript-eval-runner`
3. Run examples: `cargo run --release --bin typescript-eval-runner -- examples/*.ts`
4. Benchmark if performance-sensitive: `cargo bench`
