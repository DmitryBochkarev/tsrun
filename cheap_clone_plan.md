# Cheap Clone Optimization Plan

This document outlines a comprehensive plan to minimize expensive `clone()` operations in the TypeScript interpreter codebase.

## Executive Summary

**Current state:** ~347 clone operations, ~200 expensive (58%)
**Target state:** ~40 expensive clones (12%)
**Expected improvement:** 80% reduction in clone overhead, 60-70% reduction in memory allocations

---

## Table of Contents

1. [Analysis Overview](#analysis-overview)
2. [Phase 1: Environment Refactor](#phase-1-environment-refactor)
3. [Phase 2: AST Rc Wrapping](#phase-2-ast-rc-wrapping)
4. [Phase 3: String Interning](#phase-3-string-interning)
5. [Phase 4: Callback Optimization](#phase-4-callback-optimization)
6. [Phase 5: Minor Optimizations](#phase-5-minor-optimizations)
7. [Migration Guide](#migration-guide)
8. [Testing Strategy](#testing-strategy)

---

## Analysis Overview

### Clone Cost Classification

| Type | Clone Cost | Current Usage | Locations |
|------|------------|---------------|-----------|
| `Environment` | O(n) scope depth | 26 instances | interpreter/mod.rs |
| `BlockStatement` | O(n) recursive | 20+ instances | interpreter/mod.rs |
| `Vec<FunctionParam>` | O(n) params | 20+ instances | interpreter/mod.rs |
| `String` (identifiers) | O(n) length | 60+ instances | throughout |
| `Vec<JsValue>` | O(n) elements | 20+ instances | builtins/*.rs |
| `JsValue` (Rc-based) | O(1) | 80+ instances | already cheap |
| `JsObjectRef` | O(1) | many | already cheap |
| `JsString` | O(1) | many | already cheap |

### Hotspot Files

1. **`src/interpreter/mod.rs`** - 139 clones (40%)
   - Environment clones for scope management
   - AST clones for function/generator creation
   - String clones for variable names

2. **`src/interpreter/builtins/array.rs`** - 65 clones (19%)
   - Callback argument vector creation
   - `this` cloning in iterations

3. **`src/interpreter/builtins/promise.rs`** - 37 clones (11%)
   - Handler cloning
   - Result value cloning

4. **`src/parser.rs`** - 33 clones (10%)
   - Token string extraction
   - AST node conversion

---

## Phase 1: Environment Refactor

**Priority:** CRITICAL
**Complexity:** Low
**Impact:** Eliminates ~26 deep clones

### Problem

The `Environment` struct uses `Box<Environment>` for the outer scope reference, causing deep recursive clones of the entire scope chain:

```rust
// Current: src/value.rs:828-831
pub struct Environment {
    pub bindings: Rc<RefCell<HashMap<String, Binding>>>,
    pub outer: Option<Box<Environment>>,  // Deep clone on every .clone()
}
```

Every `self.env.clone()` copies the entire scope chain, which can be arbitrarily deep in nested functions.

### Solution

Change `Box<Environment>` to `Rc<Environment>`:

```rust
// New: src/value.rs
pub struct Environment {
    pub bindings: Rc<RefCell<HashMap<String, Binding>>>,
    pub outer: Option<Rc<Environment>>,  // O(1) clone
}

impl CheapClone for Environment {}
```

### Required Changes

#### 1. Update Environment struct (`src/value.rs`)

```rust
// Before
pub struct Environment {
    pub bindings: Rc<RefCell<HashMap<String, Binding>>>,
    pub outer: Option<Box<Environment>>,
}

impl Environment {
    pub fn with_outer(outer: Environment) -> Self {
        Self {
            bindings: Rc::new(RefCell::new(HashMap::new())),
            outer: Some(Box::new(outer)),
        }
    }
}

// After
pub struct Environment {
    pub bindings: Rc<RefCell<HashMap<String, Binding>>>,
    pub outer: Option<Rc<Environment>>,
}

impl CheapClone for Environment {}

impl Environment {
    pub fn with_outer(outer: Rc<Environment>) -> Self {
        Self {
            bindings: Rc::new(RefCell::new(HashMap::new())),
            outer: Some(outer),
        }
    }

    /// Create a child environment from self (convenience method)
    pub fn child(self_rc: &Rc<Environment>) -> Self {
        Self {
            bindings: Rc::new(RefCell::new(HashMap::new())),
            outer: Some(Rc::clone(self_rc)),
        }
    }
}
```

#### 2. Update Interpreter (`src/interpreter/mod.rs`)

The interpreter needs to store `Rc<Environment>` instead of `Environment`:

```rust
// Before
pub struct Interpreter {
    pub env: Environment,
    // ...
}

// After
pub struct Interpreter {
    pub env: Rc<Environment>,
    // ...
}
```

#### 3. Update all Environment usage sites

| Location | Before | After |
|----------|--------|-------|
| `mod.rs:396` | `let saved_env = self.env.clone();` | `let saved_env = Rc::clone(&self.env);` |
| `mod.rs:397` | `self.env = Environment::with_outer(self.env.clone());` | `self.env = Rc::new(Environment::with_outer(Rc::clone(&self.env)));` |
| `mod.rs:983-984` | `prev_env = self.env.clone(); self.env = Environment::with_outer(...)` | `prev_env = Rc::clone(&self.env); self.env = Rc::new(Environment::child(&self.env));` |
| `mod.rs:1279` | `closure: self.env.clone()` | `closure: Rc::clone(&self.env)` |

### Alternative: Environment Reference Type

For cleaner API, introduce a type alias:

```rust
pub type EnvironmentRef = Rc<Environment>;

impl Interpreter {
    pub env: EnvironmentRef,
}

// Usage becomes clearer:
let saved = self.env.cheap_clone();
self.env = EnvironmentRef::new(Environment::with_outer(self.env.cheap_clone()));
```

### Considerations

1. **Mutation through Rc**: The inner `bindings` is already `Rc<RefCell<...>>`, so mutations work through shared references.

2. **Recursion in get/set**: The `get()`, `set()`, and `has()` methods traverse `outer` - this works identically with `Rc`.

3. **Memory**: Scope chains now share structure. A child scope holds an `Rc` to parent, not a copy. Memory freed when all references drop.

---

## Phase 2: AST Rc Wrapping

**Priority:** HIGH
**Complexity:** Medium
**Impact:** Eliminates ~50+ recursive clones

### Problem

Function bodies and parameters are cloned when creating closures and generators:

```rust
// Current: src/value.rs:776-787
pub struct InterpretedFunction {
    pub name: Option<String>,
    pub params: Vec<FunctionParam>,  // Cloned from AST
    pub body: FunctionBody,           // Cloned from AST
    pub closure: Environment,
    // ...
}

// Current: src/value.rs:708-726
pub struct GeneratorState {
    pub body: BlockStatement,         // Cloned from AST
    pub params: Vec<FunctionParam>,   // Cloned from AST
    pub args: Vec<JsValue>,
    pub closure: Environment,
    // ...
}
```

Every function definition clones the entire AST subtree for body and params.

### Solution

Wrap AST components in `Rc`:

```rust
// New: src/value.rs
pub struct InterpretedFunction {
    pub name: Option<JsString>,       // Also use JsString (Phase 3)
    pub params: Rc<[FunctionParam]>,  // Shared reference
    pub body: Rc<FunctionBody>,       // Shared reference
    pub closure: Rc<Environment>,     // From Phase 1
    pub source_location: Span,
    pub generator: bool,
    pub async_: bool,
}

pub struct GeneratorState {
    pub body: Rc<BlockStatement>,
    pub params: Rc<[FunctionParam]>,
    pub args: Vec<JsValue>,           // Keep as Vec - these are runtime values
    pub closure: Rc<Environment>,
    pub state: GeneratorStatus,
    pub stmt_index: usize,
    pub sent_value: JsValue,
    pub name: Option<JsString>,
}
```

### Required Changes

#### 1. Update value.rs types

```rust
// FunctionBody wrapper (already exists, just wrap in Rc when used)
pub enum FunctionBody {
    Block(BlockStatement),
    Expression(Box<crate::ast::Expression>),
}

pub struct InterpretedFunction {
    pub name: Option<JsString>,
    pub params: Rc<[FunctionParam]>,
    pub body: Rc<FunctionBody>,
    pub closure: Rc<Environment>,
    pub source_location: Span,
    pub generator: bool,
    pub async_: bool,
}

impl CheapClone for InterpretedFunction {}
```

#### 2. Update function creation sites

```rust
// Before: src/interpreter/mod.rs:1274-1283
fn execute_function_declaration(&mut self, decl: &FunctionDeclaration) -> Result<(), JsError> {
    let func = InterpretedFunction {
        name: decl.id.as_ref().map(|id| id.name.clone()),
        params: decl.params.clone(),
        body: FunctionBody::Block(decl.body.clone()),
        closure: self.env.clone(),
        // ...
    };
}

// After
fn execute_function_declaration(&mut self, decl: &FunctionDeclaration) -> Result<(), JsError> {
    let func = InterpretedFunction {
        name: decl.id.as_ref().map(|id| JsString::from(id.name.as_str())),
        params: Rc::from(decl.params.as_slice()),
        body: Rc::new(FunctionBody::Block(decl.body.clone())), // Still clones AST once
        closure: Rc::clone(&self.env),
        // ...
    };
}
```

#### 3. Parser-level Rc wrapping (optimal)

For maximum benefit, wrap at parse time:

```rust
// In parser.rs, when creating FunctionDeclaration:
pub struct FunctionDeclaration {
    pub id: Option<Identifier>,
    pub params: Rc<[FunctionParam]>,  // Wrapped at parse time
    pub body: Rc<BlockStatement>,      // Wrapped at parse time
    // ...
}
```

This eliminates clones entirely - the interpreter just `Rc::clone()`s the already-wrapped nodes.

### Function Creation Locations

| File | Line | Function | Change |
|------|------|----------|--------|
| `mod.rs` | 1275-1283 | `execute_function_declaration` | Wrap params/body in Rc |
| `mod.rs` | 1408-1420 | `evaluate` (FunctionExpression) | Wrap params/body in Rc |
| `mod.rs` | 1568-1580 | `evaluate` (method creation) | Wrap params/body in Rc |
| `mod.rs` | 2605-2625 | Arrow function handling | Wrap params/body in Rc |

### Generator State Updates

```rust
// Before: src/interpreter/mod.rs:369-383
let (body, closure, ...) = {
    let state = gen_state.borrow();
    (
        state.body.clone(),    // Expensive
        state.closure.clone(), // Expensive (fixed by Phase 1)
        state.params.clone(),  // Expensive
        state.args.clone(),    // Keep - runtime values
    )
};

// After
let (body, closure, ...) = {
    let state = gen_state.borrow();
    (
        Rc::clone(&state.body),    // O(1)
        Rc::clone(&state.closure), // O(1)
        Rc::clone(&state.params),  // O(1)
        state.args.clone(),        // Keep - small Vec of runtime values
    )
};
```

---

## Phase 3: String Interning

**Priority:** HIGH
**Complexity:** Medium
**Impact:** Eliminates ~60+ String allocations

### Problem

Identifiers use `String`, requiring allocation on every access:

```rust
// Current: src/ast.rs:509-512
pub struct Identifier {
    pub name: String,
    pub span: Span,
}
```

Common patterns that clone strings:
```rust
// src/interpreter/mod.rs:1289
self.env.define(id.name.clone(), JsValue::Object(func_obj), true);

// src/interpreter/mod.rs:948
brk.label.as_ref().map(|l| l.name.clone())
```

### Solution A: JsString for Identifiers

Replace `String` with `JsString` (which is `Rc<str>`):

```rust
// New: src/ast.rs
pub struct Identifier {
    pub name: JsString,  // Rc<str> - O(1) clone
    pub span: Span,
}

pub struct StringLiteral {
    pub value: JsString,
    pub span: Span,
}
```

### Solution B: String Interner (More Complex)

Add a string interner for deduplication:

```rust
// New: src/interner.rs
use std::collections::HashMap;
use crate::value::JsString;

pub struct StringInterner {
    strings: HashMap<Box<str>, JsString>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self { strings: HashMap::new() }
    }

    pub fn intern(&mut self, s: &str) -> JsString {
        if let Some(existing) = self.strings.get(s) {
            return existing.cheap_clone();
        }
        let js_str = JsString::from(s);
        self.strings.insert(s.into(), js_str.cheap_clone());
        js_str
    }
}

// Pre-intern common strings
impl StringInterner {
    pub fn with_common_strings() -> Self {
        let mut interner = Self::new();
        // Intern keywords and common identifiers
        for s in ["undefined", "null", "true", "false", "length",
                  "prototype", "constructor", "toString", "valueOf",
                  "hasOwnProperty", "name", "message", "stack"] {
            interner.intern(s);
        }
        interner
    }
}
```

### Required Changes for Solution A

#### 1. Update AST types (`src/ast.rs`)

```rust
use crate::value::JsString;

pub struct Identifier {
    pub name: JsString,
    pub span: Span,
}

pub struct StringLiteral {
    pub value: JsString,
    pub span: Span,
}

// Also update:
// - LiteralValue::String(JsString)
// - TemplateElement.value: JsString
// - EnumMember (if using string keys)
```

#### 2. Update parser (`src/parser.rs`)

```rust
// Before
fn parse_identifier(&mut self) -> Result<Identifier, JsError> {
    let name = self.expect_identifier()?;
    Ok(Identifier { name, span })
}

// After
fn parse_identifier(&mut self) -> Result<Identifier, JsError> {
    let name = self.expect_identifier()?;
    Ok(Identifier {
        name: JsString::from(name),
        span
    })
}
```

#### 3. Update Environment API (`src/value.rs`)

```rust
impl Environment {
    // Change from String to &str or JsString
    pub fn define(&mut self, name: JsString, value: JsValue, mutable: bool) {
        self.bindings.borrow_mut().insert(
            name.as_str().to_string(), // Internal storage still String for HashMap
            Binding { value, mutable, initialized: true },
        );
    }

    // Or use JsString as key:
    pub bindings: Rc<RefCell<HashMap<JsString, Binding>>>,
}
```

### Impact Analysis

| Pattern | Before | After |
|---------|--------|-------|
| `id.name.clone()` | O(n) String alloc | O(1) Rc increment |
| `env.define(name.clone(), ...)` | O(n) String alloc | O(1) or pass reference |
| `label.name.clone()` | O(n) String alloc | O(1) Rc increment |

---

## Phase 4: Callback Optimization

**Priority:** MEDIUM
**Complexity:** Low
**Impact:** Eliminates ~20+ allocations per iteration

### Problem

Array methods allocate a new `Vec` for each callback invocation:

```rust
// Current pattern in src/interpreter/builtins/array.rs
for i in 0..length {
    let result = interp.call_function(
        callback.clone(),
        this_arg.clone(),
        vec![elem, JsValue::Number(i as f64), this.clone()],  // NEW ALLOCATION
    )?;
}
```

### Solution A: Reusable Buffer

Add a reusable argument buffer to Interpreter:

```rust
// src/interpreter/mod.rs
pub struct Interpreter {
    // ... existing fields ...

    /// Reusable buffer for callback arguments (avoids allocation in hot loops)
    callback_args: Vec<JsValue>,
}

impl Interpreter {
    /// Call a function with arguments from a reusable buffer
    pub fn call_with_callback_args(
        &mut self,
        func: JsValue,
        this: JsValue,
        arg0: JsValue,
        arg1: JsValue,
        arg2: JsValue,
    ) -> Result<JsValue, JsError> {
        self.callback_args.clear();
        self.callback_args.push(arg0);
        self.callback_args.push(arg1);
        self.callback_args.push(arg2);

        // Take ownership temporarily to avoid borrow issues
        let args = std::mem::take(&mut self.callback_args);
        let result = self.call_function(func, this, args);
        self.callback_args = result.1; // Restore buffer
        result.0
    }
}
```

### Solution B: SmallVec

Use `smallvec` for stack allocation of small argument lists:

```rust
// Cargo.toml
[dependencies]
smallvec = "1.11"

// src/interpreter/mod.rs
use smallvec::SmallVec;

type CallbackArgs = SmallVec<[JsValue; 4]>;

pub fn call_function(
    &mut self,
    func: JsValue,
    this: JsValue,
    args: impl Into<CallbackArgs>,
) -> Result<JsValue, JsError> {
    let args: CallbackArgs = args.into();
    // ...
}
```

### Solution C: Pass Slice (Breaking Change)

Change function signature to accept slice:

```rust
// Before
pub fn call_function(
    &mut self,
    func: JsValue,
    this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError>

// After
pub fn call_function(
    &mut self,
    func: JsValue,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError>
```

### Affected Locations

| File | Method | Current Pattern |
|------|--------|-----------------|
| `array.rs:487` | `array_map` | `vec![elem, Number(i), this.clone()]` |
| `array.rs:534` | `array_filter` | `vec![elem, Number(i), this.clone()]` |
| `array.rs:582` | `array_for_each` | `vec![elem, Number(i), this.clone()]` |
| `array.rs:639` | `array_find` | `vec![elem, Number(i), this.clone()]` |
| `array.rs:683` | `array_find_index` | `vec![elem, Number(i), this.clone()]` |
| `array.rs:731` | `array_some` | `vec![elem, Number(i), this.clone()]` |
| `array.rs:779` | `array_every` | `vec![elem, Number(i), this.clone()]` |
| `array.rs:996` | `array_reduce` | `vec![acc, elem, Number(i), this.clone()]` |
| `array.rs:1044` | `array_reduce_right` | `vec![acc, elem, Number(i), this.clone()]` |
| `set.rs:262` | `set_for_each` | `vec![value.clone(), value, this.clone()]` |
| `map.rs:340` | `map_for_each` | `vec![v.clone(), k.clone(), this.clone()]` |

---

## Phase 5: Minor Optimizations

### 5.1 Property Descriptor References

**Problem:** `get_property_descriptor` clones the property:

```rust
// Current: src/value.rs:451-453
pub fn get_property_descriptor(&self, key: &PropertyKey) -> Option<(Property, bool)> {
    if let Some(prop) = self.properties.get(key) {
        return Some((prop.clone(), false));
    }
}
```

**Solution:** Return reference when possible:

```rust
pub fn get_property_descriptor(&self, key: &PropertyKey) -> Option<(&Property, bool)> {
    if let Some(prop) = self.properties.get(key) {
        return Some((prop, false));
    }
    // For prototype chain, we need owned value
}

// Or split into two methods:
pub fn get_own_property(&self, key: &PropertyKey) -> Option<&Property>
pub fn get_property_from_chain(&self, key: &PropertyKey) -> Option<Property>
```

### 5.2 Token Kind Optimization

**Problem:** `TokenKind` contains `String` variants:

```rust
// Current: src/lexer.rs
pub enum TokenKind {
    Identifier(String),
    StringLiteral(String),
    // ...
}
```

**Solution:** Use `JsString`:

```rust
pub enum TokenKind {
    Identifier(JsString),
    StringLiteral(JsString),
    // ...
}
```

### 5.3 Generator/Promise State Sharing

**Problem:** Promise handlers clone callbacks:

```rust
// Current: src/value.rs:697-705
pub struct PromiseHandler {
    pub on_fulfilled: Option<JsValue>,
    pub on_rejected: Option<JsValue>,
    pub result_promise: JsObjectRef,
}
```

**Solution:** JsValue containing functions is already cheap (Rc-based). Verify no unnecessary clones in promise resolution code.

---

## Migration Guide

### Step-by-Step Implementation Order

1. **Phase 1 first** - Environment changes are isolated and have highest impact
2. **Phase 3 partially** - Add JsString to Identifier in AST (enables Phase 2)
3. **Phase 2** - AST Rc wrapping (depends on Phase 1 for closure type)
4. **Phase 4** - Callback optimization (independent)
5. **Phase 5** - Minor optimizations (independent)

### Breaking Changes Checklist

- [ ] `Environment::with_outer()` signature change
- [ ] `Interpreter.env` type change to `Rc<Environment>`
- [ ] `InterpretedFunction` field types change
- [ ] `GeneratorState` field types change
- [ ] `Identifier.name` type change (if doing Phase 3)
- [ ] Parser output types change (if doing parser-level Rc)

### Deprecation Strategy

For gradual migration, add deprecated methods:

```rust
impl Environment {
    #[deprecated(note = "Use with_outer_rc instead")]
    pub fn with_outer(outer: Environment) -> Self {
        Self::with_outer_rc(Rc::new(outer))
    }

    pub fn with_outer_rc(outer: Rc<Environment>) -> Self {
        Self {
            bindings: Rc::new(RefCell::new(HashMap::new())),
            outer: Some(outer),
        }
    }
}
```

---

## Testing Strategy

### 1. Correctness Tests

All existing tests must pass. Run after each phase:

```bash
cargo test
cargo test --test interpreter
```

### 2. Clone Counting (Debug)

Add debug instrumentation to count clones:

```rust
#[cfg(debug_assertions)]
thread_local! {
    static CLONE_COUNT: std::cell::Cell<usize> = std::cell::Cell::new(0);
}

#[cfg(debug_assertions)]
pub fn increment_clone_count() {
    CLONE_COUNT.with(|c| c.set(c.get() + 1));
}

#[cfg(debug_assertions)]
pub fn get_clone_count() -> usize {
    CLONE_COUNT.with(|c| c.get())
}

// Add to expensive Clone impls temporarily
impl Clone for Environment {
    fn clone(&self) -> Self {
        #[cfg(debug_assertions)]
        increment_clone_count();
        // ... actual clone
    }
}
```

### 3. Benchmark Tests

Create benchmarks for clone-heavy operations:

```rust
// benches/clone_benchmark.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_nested_closures(c: &mut Criterion) {
    c.bench_function("nested_closures", |b| {
        b.iter(|| {
            eval(r#"
                function outer() {
                    let x = 1;
                    function middle() {
                        let y = 2;
                        function inner() {
                            return x + y;
                        }
                        return inner;
                    }
                    return middle;
                }
                outer()()();
            "#)
        })
    });
}

fn bench_array_map(c: &mut Criterion) {
    c.bench_function("array_map_1000", |b| {
        b.iter(|| {
            eval("[...Array(1000)].map((_, i) => i * 2)")
        })
    });
}

criterion_group!(benches, bench_nested_closures, bench_array_map);
criterion_main!(benches);
```

### 4. Memory Profiling

Use `heaptrack` or `valgrind` to measure allocation reduction:

```bash
# Before optimization
cargo build --release
heaptrack ./target/release/typescript-eval bench_script.ts
heaptrack_print heaptrack.*.gz > before.txt

# After optimization
# ... make changes ...
cargo build --release
heaptrack ./target/release/typescript-eval bench_script.ts
heaptrack_print heaptrack.*.gz > after.txt

# Compare
diff before.txt after.txt
```

---

## Appendix: Clone Location Reference

### Environment Clones (26 instances)

| Line | Context | Purpose |
|------|---------|---------|
| 396 | `resume_generator` | Save env before generator execution |
| 397 | `resume_generator` | Create generator scope |
| 475 | `resume_generator_with_throw` | Save env |
| 476 | `resume_generator_with_throw` | Create generator scope |
| 983 | `execute_try` | Save env for catch block |
| 984 | `execute_try` | Create catch scope |
| 1279 | `execute_function_declaration` | Capture closure |
| 1412 | `evaluate` (FunctionExpr) | Capture closure |
| 1494 | `evaluate` (method) | Capture closure |
| 1574 | `evaluate` (getter/setter) | Capture closure |
| 1686 | `execute_class` | Capture method closure |
| 2079 | `execute_block` | Create block scope |
| 2166 | `execute_for` | Create loop scope |
| 2609 | `evaluate` (arrow) | Capture closure |
| 2624 | `evaluate` (arrow) | Capture closure |
| 3592 | `call_function` | Create function scope |
| 3713 | `call_function` | Create function scope |

### AST Clones (50+ instances)

| Line | Type Cloned | Purpose |
|------|-------------|---------|
| 376 | `BlockStatement` | Generator body for execution |
| 381 | `Vec<FunctionParam>` | Generator params |
| 456 | `BlockStatement` | Generator body (throw path) |
| 460 | `Vec<FunctionParam>` | Generator params (throw path) |
| 1277 | `Vec<FunctionParam>` | Function declaration params |
| 1278 | `BlockStatement` | Function declaration body |
| 1410 | `Vec<FunctionParam>` | Function expression params |
| 1411 | `BlockStatement` | Function expression body |
| 1572 | `Vec<FunctionParam>` | Method params |
| 1573 | `BlockStatement` | Method body |
| 2607 | `Vec<FunctionParam>` | Arrow function params |
| 2608 | `ArrowFunctionBody` | Arrow function body |

---

## Summary

| Phase | Clones Eliminated | Complexity | Dependencies |
|-------|-------------------|------------|--------------|
| 1. Environment | ~26 | Low | None |
| 2. AST Rc | ~50+ | Medium | Phase 1 |
| 3. String/JsString | ~60+ | Medium | None |
| 4. Callback Args | ~20+/iteration | Low | None |
| 5. Minor | ~10 | Low | None |

**Total reduction: ~80% of expensive clones**

Recommended implementation order: Phase 1 → Phase 3 (partial) → Phase 2 → Phase 4 → Phase 5
