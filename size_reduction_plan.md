# Binary Size Reduction Plan

This document outlines code changes to further reduce the binary size of tsrun beyond the Cargo.toml optimizations already applied.

## Current State (After Tier 2)

| Metric | Before | After Tier 2 | Reduction |
|--------|--------|--------------|-----------|
| Binary size | 4.8 MB | 1.6 MB | 67% |
| .text section | 2.2 MB | 750 KB | 66% |

### Current Size Breakdown

| Component | Size | Notes |
|-----------|------|-------|
| tsrun | 252 KB | Parser, interpreter, builtins |
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
<tsrun::ast::Expression as core::clone::Clone>::clone  ~30 KB total (multiple monomorphizations)
<tsrun::ast::Statement as core::clone::Clone>::clone   ~12 KB total
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

**Rationale**: Splitting into `tsrun-parser`, `tsrun-runtime`, `tsrun-builtins` allows users to depend only on what they need.

**Structure**:
```
tsrun/
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
2. Verify binary size: `ls -lh target/release/tsrun`
3. Run examples: `cargo run --release --bin tsrun -- examples/*.ts`
4. Benchmark if performance-sensitive: `cargo bench`

---

## Internal Code Optimizations (No External Dependencies)

These optimizations focus on reducing code size within the tsrun crate itself, without changing external dependencies.

### Current Internal Size Breakdown

Top functions by binary size (from cargo-bloat):

| Function | Size | Notes |
|----------|------|-------|
| `Interpreter::new` | 12.6 KB | Builtin prototype registration |
| `Parser::parse_member_expression` | 10.3 KB | Complex parser logic |
| `Interpreter::evaluate` | 8.4 KB | Main expression evaluator |
| `Parser::parse_class_declaration` | 5.2 KB | Class parsing |
| `create_class_constructor` | 4.2 KB | Class instantiation |
| `Lexer::next_token` | 3.8 KB | Tokenizer |
| `execute_statement` | 3.7 KB | Statement executor |
| `call_function` | 3.4 KB | Function calls |
| `string_to_lower_case` | 2.9 KB | Unicode case conversion |
| `string_to_upper_case` | 2.4 KB | Unicode case conversion |

Builtin file sizes:
- `array.rs`: 2160 lines
- `string.rs`: 1292 lines
- `date.rs`: 960 lines
- `object.rs`: 887 lines
- `promise.rs`: 844 lines
- `math.rs`: 610 lines

---

### A. Shared Builtin Registration Function

**Estimated savings**: 3-5 KB

**Problem**: `Interpreter::new()` is 12.6 KB because it repeats similar code to register each builtin method (~100 times).

**Current pattern** (repeated ~100 times):
```rust
let push_fn = create_function(JsFunction::Native(NativeFunction {
    name: "push".to_string(),
    func: array_push,
    arity: 1,
}));
p.set_property(PropertyKey::from("push"), JsValue::Object(push_fn));
```

**Proposed solution** - shared runtime function:
```rust
/// Register a native method on a prototype object
fn register_method(
    proto: &mut JsObject,
    name: &str,
    func: fn(&mut Interpreter, JsValue, &[JsValue]) -> Result<JsValue, JsError>,
    arity: usize,
) {
    let f = create_function(JsFunction::Native(NativeFunction {
        name: name.to_string(),
        func,
        arity,
    }));
    proto.set_property(PropertyKey::from(name), JsValue::Object(f));
}

// Usage - single function call per method instead of 5 lines:
let mut p = proto.borrow_mut();
register_method(&mut p, "push", array_push, 1);
register_method(&mut p, "pop", array_pop, 0);
register_method(&mut p, "map", array_map, 1);
// ...
```

This reduces binary size because the registration logic is compiled once, not inlined 100 times.

**Files to modify**:
- `src/value.rs` (add `register_method` helper)
- `src/interpreter/builtins/array.rs`
- `src/interpreter/builtins/string.rs`
- `src/interpreter/builtins/object.rs`
- `src/interpreter/builtins/math.rs`
- All other builtin files

---

### B. Consolidate Array Iterator Methods

**Estimated savings**: 2-4 KB

**Problem**: These methods share 80% identical code:
- `array_map` (45 lines)
- `array_filter` (48 lines)
- `array_forEach` (42 lines)
- `array_find` (46 lines)
- `array_findIndex` (46 lines)
- `array_some` (48 lines)
- `array_every` (48 lines)
- `array_findLast` (48 lines)
- `array_findLastIndex` (48 lines)

**Common boilerplate**:
```rust
let JsValue::Object(arr) = this.clone() else {
    return Err(JsError::type_error("...called on non-object"));
};
let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
if !callback.is_callable() {
    return Err(JsError::type_error("...callback is not a function"));
}
let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);
let length = { /* get length */ };
for i in 0..length {
    let elem = arr.borrow().get_property(&PropertyKey::Index(i))...;
    let result = interp.call_function(callback.clone(), this_arg.clone(), &[...]);
    // Only this part differs per method
}
```

**Proposed solution**:
```rust
enum IteratorBehavior {
    Map,           // collect transformed values
    Filter,        // collect values where predicate is true
    ForEach,       // just iterate, return undefined
    Find,          // return first match
    FindIndex,     // return first match index
    Some,          // return true if any match
    Every,         // return true if all match
    FindLast,      // return last match (reverse)
    FindLastIndex, // return last match index (reverse)
}

fn array_iterate_with_predicate(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
    method_name: &str,
    behavior: IteratorBehavior,
) -> Result<JsValue, JsError> {
    // Single implementation handling all cases
}
```

**Files to modify**:
- `src/interpreter/builtins/array.rs`

---

### C. Consolidate Math Single-Argument Functions

**Estimated savings**: 1-2 KB

**Problem**: 20+ math functions have identical structure:
```rust
pub fn math_sin(_interp: &mut Interpreter, _this: JsValue, args: &[JsValue]) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.sin()))
}
pub fn math_cos(_interp: &mut Interpreter, _this: JsValue, args: &[JsValue]) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.cos()))
}
// tan, asin, acos, atan, sinh, cosh, tanh, asinh, acosh, atanh, sqrt, log, exp, ...
```

**Proposed solution** - generic wrapper:
```rust
fn math_unary_op(args: &[JsValue], op: fn(f64) -> f64) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(op(n)))
}

pub fn math_sin(_: &mut Interpreter, _: JsValue, args: &[JsValue]) -> Result<JsValue, JsError> {
    math_unary_op(args, f64::sin)
}
```

**Note**: Using a macro here would NOT reduce binary size - macros expand at compile time. Only a shared runtime function with dispatch reduces code.

**Files to modify**:
- `src/interpreter/builtins/math.rs`

---

### D. Reduce String Method Duplication

**Estimated savings**: 1-2 KB

**Problem**: `string_to_lower_case` (2.9 KB) and `string_to_upper_case` (2.4 KB) are nearly identical. Many string methods share similar "get string from this, do operation, return new string" pattern.

**Current**:
```rust
pub fn string_to_lower_case(_: &mut Interpreter, this: JsValue, _: &[JsValue]) -> Result<JsValue, JsError> {
    let s = get_string_value(&this)?;
    Ok(JsValue::String(JsString::from(s.to_lowercase())))
}
pub fn string_to_upper_case(_: &mut Interpreter, this: JsValue, _: &[JsValue]) -> Result<JsValue, JsError> {
    let s = get_string_value(&this)?;
    Ok(JsValue::String(JsString::from(s.to_uppercase())))
}
```

**Proposed solution**:
```rust
fn string_transform(this: &JsValue, transform: impl Fn(&str) -> String) -> Result<JsValue, JsError> {
    let s = get_string_value(this)?;
    Ok(JsValue::String(JsString::from(transform(&s))))
}

pub fn string_to_lower_case(_: &mut Interpreter, this: JsValue, _: &[JsValue]) -> Result<JsValue, JsError> {
    string_transform(&this, |s| s.to_lowercase())
}
pub fn string_to_upper_case(_: &mut Interpreter, this: JsValue, _: &[JsValue]) -> Result<JsValue, JsError> {
    string_transform(&this, |s| s.to_uppercase())
}
pub fn string_trim(_: &mut Interpreter, this: JsValue, _: &[JsValue]) -> Result<JsValue, JsError> {
    string_transform(&this, |s| s.trim().to_string())
}
```

**Files to modify**:
- `src/interpreter/builtins/string.rs`

---

### E. Simplify Unicode Case Conversion

**Estimated savings**: 2-3 KB (if unicode-normalization can be made optional)

**Problem**: `string_to_lower_case` and `string_to_upper_case` are large because they pull in `unicode_normalization` crate for proper Unicode handling.

**Options**:
1. Make unicode-normalization optional (fallback to ASCII-only)
2. Use simpler case conversion for common cases

**Implementation**:
```toml
[features]
default = ["unicode"]
unicode = ["unicode-normalization"]
```

```rust
#[cfg(feature = "unicode")]
fn to_lowercase(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    s.nfc().collect::<String>().to_lowercase()
}

#[cfg(not(feature = "unicode"))]
fn to_lowercase(s: &str) -> String {
    s.to_lowercase()  // ASCII-only
}
```

**Files to modify**:
- `Cargo.toml`
- `src/interpreter/builtins/string.rs`

---

### F. Parser Refactoring

**Estimated savings**: 3-5 KB

**Problem**: `parse_member_expression` (10.3 KB) and `parse_class_declaration` (5.2 KB) are very large due to handling many cases inline.

**Proposed solution**: Extract helper functions for repeated patterns.

```rust
// Current: inline handling of computed vs non-computed property access
// Proposed: extract to helper
fn parse_property_access(&mut self, object: Expression) -> Result<Expression, ParseError> {
    // Handle both computed ([expr]) and non-computed (.ident) access
}

// Current: inline handling of method, getter, setter, field
// Proposed: extract per-member-type handlers
fn parse_class_method(&mut self, ...) -> Result<ClassMember, ParseError> { ... }
fn parse_class_field(&mut self, ...) -> Result<ClassMember, ParseError> { ... }
fn parse_class_getter(&mut self, ...) -> Result<ClassMember, ParseError> { ... }
```

**Files to modify**:
- `src/parser.rs`

---

## Summary of Internal Optimizations

| Change | Savings | Effort | Risk |
|--------|---------|--------|------|
| A. Shared builtin registration | 3-5 KB | Low | Low |
| B. Array iterator consolidation | 2-4 KB | Medium | Low |
| C. Math function consolidation | 1-2 KB | Low | Low |
| D. String method consolidation | 1-2 KB | Low | Low |
| E. Optional unicode | 2-3 KB | Medium | Medium |
| F. Parser refactoring | 3-5 KB | High | Medium |
| **Total potential** | **12-21 KB** | | |

**Important**: Macros do NOT reduce binary size - they expand at compile time. Only consolidating code into shared runtime functions with dispatch (enums, function pointers, or trait objects) reduces binary size.

## Recommended Order for Internal Changes

1. **Shared builtin registration** (A) - Lowest effort, immediate gains (~100 call sites)
2. **Array iterator consolidation** (B) - Best ROI, consolidates 9 similar functions
3. **Math function consolidation** (C) - Easy, consolidates 20+ functions into one dispatcher
4. **String method consolidation** (D) - Easy, safe refactoring
5. **Parser refactoring** (F) - Higher effort, requires careful testing
6. **Optional unicode** (E) - Only if users need minimal builds
