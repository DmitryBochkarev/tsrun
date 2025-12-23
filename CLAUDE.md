# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TypeScript interpreter written in Rust for config/manifest generation with support for ES modules and async/await. Types are parsed but stripped at runtime (not type-checked). The interpreter uses a **register-based bytecode VM** for execution.

## Quick Reference

### Build & Test Commands

```bash
cargo build                              # Build the project
timeout 30 cargo test                    # Run all tests (always use timeout!)
timeout 30 cargo test --test interpreter # Run interpreter integration tests
timeout 30 cargo test test_name          # Run specific test
timeout 30 cargo test -- --nocapture     # Show test output
```

### Key Files

| File/Directory | Purpose |
|----------------|---------|
| `src/lib.rs` | Public API - `Runtime` struct |
| `src/lexer.rs` | Tokenizer |
| `src/parser.rs` | Recursive descent + Pratt parsing |
| `src/ast.rs` | AST node types |
| `src/value.rs` | Runtime values, object model, GC |
| `src/compiler/` | Bytecode compiler |
| `src/interpreter/` | VM and builtins |
| `tests/interpreter/` | Integration tests by feature |

## Development Rules

- **Always use the Edit tool** - never shell commands like `echo >>` to modify files
- **Use TypeScript annotations in tests** - types are stripped at runtime but tests should use proper syntax
- **No tech debt** - fix failing tests immediately, no TODO/FIXME for known bugs
- **Use TDD** - if a test fails because a feature isn't implemented, implement the feature
- **Never change failing test cases** - write simpler tests to verify current scope, keep original as goal
- **Fix pre-existing bugs** - write a test, fix it, then continue with your feature
- **Proper fixes over workarounds** - make architectural changes if needed
- **Debug via tests** - use `cargo test test_name -- --nocapture` with `console.log()`, not ad-hoc scripts

### TDD Workflow

1. **Verify parser support** - write parser test first, implement if needed
2. Write failing interpreter test
3. Implement minimal code to pass
4. Refactor while keeping tests green
5. Run `cargo test && cargo fmt && cargo clippy` before committing

## Code Safety

### Zero-Panic Policy

These patterns are **denied** via Clippy lints:

| Pattern | Alternative |
|---------|-------------|
| `.unwrap()` | `.ok_or_else()`, `if let`, `match` |
| `.expect()` | `.ok_or_else()` with descriptive error |
| `[index]` | `.get(index)` with error handling |
| `panic!()` | `Err(JsError::...)` |
| `unreachable!()` | `Err(JsError::internal_error(...))` |
| `todo!()` | Implement the feature or return error |
| `&str[start..end]` | `.get(start..end)` for safe slicing |

Test code is exempt via `clippy.toml`.

### Safe Access Patterns

```rust
// Function arguments
let first = args.first().cloned().unwrap_or(JsValue::Undefined);
let second = args.get(1).cloned().unwrap_or(JsValue::Undefined);

// Array/vector access
let elem = elements.get(i).ok_or_else(|| JsError::internal_error("index out of bounds"))?;
let rest = args.get(i..).unwrap_or_default().to_vec();

// String slicing
let slice = s.get(start..end).unwrap_or("");

// Option unwrapping
let value = opt.ok_or_else(|| JsError::internal_error("expected value"))?;
```

### Clone Conventions

Use `.cheap_clone()` for O(1) reference-counted clones:

| Type | Clone Cost | Method |
|------|-----------|--------|
| `Gc<JsObject>` | Cheap | `.cheap_clone()` |
| `JsString` | Cheap | `.cheap_clone()` |
| `Rc<T>` | Cheap | `.cheap_clone()` |
| `String`, `Vec<T>`, AST | Expensive | `.clone()` with comment |

## GC & Guards

### Overview

The `Guarded` struct wraps a `JsValue` with a `Guard` that keeps objects alive during GC:

```rust
pub struct Guarded {
    pub value: JsValue,
    pub guard: Option<Guard<JsObject>>,
}
```

The VM maintains a `register_guard` that keeps all register values alive. When returning from the VM, values are wrapped in `Guarded`.

### Object Creation API

Caller provides guard, method allocates from it:

```rust
let guard = self.heap.create_guard();
let obj = self.create_object(&guard);           // With prototype
let raw = self.create_object_raw(&guard);       // Without prototype
let arr = self.create_array_from(&guard, elements);
let func = self.create_native_fn(&guard, "name", native_fn, arity);

// Multiple objects can share one guard
let guard = self.heap.create_guard();
let obj1 = self.create_object(&guard);
let obj2 = self.create_object(&guard);
```

### Critical GC Rules

**1. Guard before allocate** - GC runs BEFORE allocation when threshold is reached:
```rust
// CORRECT
let guard = interp.heap.create_guard();
interp.guard_value_with(&guard, &input_value);  // Guard input FIRST
let obj = interp.create_object(&guard);         // Then allocate

// WRONG - input_value may be collected during allocation!
let obj = interp.create_object(&guard);
```

**2. Return Guarded when returning objects**:
```rust
// CORRECT
pub fn some_builtin(...) -> Result<Guarded, JsError> {
    let guard = interp.heap.create_guard();
    let arr = interp.create_array(&guard, elements);
    Ok(Guarded { value: JsValue::Object(arr), guard: Some(guard) })
}

// WRONG - guard dropped, object may be collected!
pub fn some_builtin(...) -> Result<JsValue, JsError> { ... }
```

**3. Guard scope in collect-then-store loops**:
```rust
// CORRECT - guards at outer scope
let mut all_guards: Vec<Guard<JsObject>> = Vec::new();
let mut methods: Vec<(String, Gc<JsObject>)> = Vec::new();

for item in items {
    let (func, guard) = create_function(...)?;
    if let Some(g) = guard { all_guards.push(g); }
    methods.push((name, func));
}
// Store methods - guards still alive
for (name, func) in methods {
    prototype.borrow_mut().set_property(name, JsValue::Object(func));
}

// WRONG - guards dropped each iteration, funcs may be GC'd before storage
```

**4. Never allocate temporary objects from root_guard** - they'll never be collected (memory leak).

### Aggressive Test Defaults

Tests use aggressive settings to catch bugs early:
- `GC_THRESHOLD=1` - GC on every allocation
- `MAX_CALL_DEPTH=50` - Low recursion limit

Common GC bugs caught: "X is not a function", missing array elements, undefined properties.

## Architecture

### Pipeline

```
Source → Lexer → Parser → AST → Compiler → Bytecode → BytecodeVM → RuntimeResult
                                                                         │
                                              ┌──────────────────────────┼──────────────────────────┐
                                              ▼                          ▼                          ▼
                                         Complete                   NeedImports                 Suspended
```

### Register-Based VM

The VM uses registers instead of a stack:
- Fewer instructions (no push/pop overhead)
- Better cache locality
- State capture for suspension at await/yield

### Key Types

| Type | Description |
|------|-------------|
| `JsValue` | Enum: Undefined, Null, Boolean, Number, String, Object |
| `Gc<JsObject>` | GC-managed object pointer |
| `JsString` | `Rc<str>` reference-counted string |
| `Op` | Bytecode instruction (100+ variants) |
| `BytecodeChunk` | Compiled function with instructions + constants |
| `Register` | Virtual register index (u8, 0-255 per frame) |

### Runtime Result

```rust
pub enum RuntimeResult {
    Complete(RuntimeValue),              // Finished
    NeedImports(Vec<ImportRequest>),     // Need modules loaded
    Suspended { pending, cancelled },    // Waiting for orders
}
```

### Module Structure

**Compiler** (`src/compiler/`):
- `compile_stmt.rs` / `compile_expr.rs` - Statement/expression compilation
- `compile_pattern.rs` - Destructuring patterns
- `builder.rs` - Bytecode builder with register allocation
- `hoist.rs` - Variable hoisting

**Builtins** (`src/interpreter/builtins/`):
- `array.rs`, `string.rs`, `number.rs`, `object.rs` - Core types
- `function.rs`, `math.rs`, `json.rs`, `date.rs` - Standard objects
- `regexp.rs`, `map.rs`, `set.rs`, `error.rs` - Other builtins
- `global.rs` - Global functions (parseInt, parseFloat, etc.)

## Implementation Patterns

### Adding Built-in Methods

1. **Write test** in `tests/interpreter/<type>.rs`:
```rust
#[test]
fn test_array_mymethod() {
    assert_eq!(eval("[1,2,3].myMethod()"), JsValue::Number(expected));
}
```

2. **Implement** in `src/interpreter/builtins/<type>.rs`:
```rust
pub fn array_my_method(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error("Array.prototype.myMethod called on non-object"));
    };
    // Implementation
    Ok(result)
}
```

3. **Register** in `create_*_prototype()`:
```rust
let fn_obj = create_native_fn(&guard, "myMethod", array_my_method, 1);
p.set_property(PropertyKey::from("myMethod"), JsValue::Object(fn_obj));
```

4. **Update** design.md checklist

### Common Patterns

```rust
// Get array length
let length = match &arr.borrow().exotic {
    ExoticObject::Array { length } => *length,
    _ => return Err(JsError::type_error("Not an array")),
};

// Update array length (must update both!)
if let ExoticObject::Array { ref mut length } = arr_ref.exotic {
    *length = new_length;
}
arr_ref.set_property(PropertyKey::from("length"), JsValue::Number(new_length as f64));

// Call a callback
let result = interp.call_function(
    callback.clone(),
    this_arg.clone(),
    vec![elem, JsValue::Number(index as f64), this.clone()],
)?;
```

### Prototype Chain

- Objects → `object_prototype` (hasOwnProperty, toString)
- Arrays → `array_prototype` → `object_prototype`
- Strings → `string_prototype` (looked up in evaluate_member)
- Numbers → `number_prototype` (looked up in evaluate_member)

## Testing

### Test Organization

| Location | Contents |
|----------|----------|
| `tests/interpreter/*.rs` | Integration tests by feature |
| `src/parser.rs` (bottom) | Parser unit tests |
| `src/value.rs` (bottom) | Value type unit tests |

Each test file uses the shared `eval()` helper:
```rust
use super::eval;
assert_eq!(eval("1 + 2"), JsValue::Number(3.0));
```

### Test262 Conformance

```bash
git submodule update --init --depth 1
cargo build --release --bin test262-runner
./target/release/test262-runner --strict-only language/types
```

The interpreter runs all code in strict mode - use `--strict-only` for meaningful results.

### TypeScript Handling

- Type annotations, interfaces, type aliases → parsed but no-op at runtime
- `enum` declarations → compile to object literals
- Type assertions (`x as T`, `<T>x`) → evaluate to just the expression

## Implementation Status

**1550+ passing tests**

**Language Features:** variables, functions, closures, control flow, classes with inheritance/static blocks, destructuring, spread, template literals, all operators, generators, async/await, Promises.

**Built-in Objects:** Array, String, Object, Number, Math, JSON, Map, Set, WeakMap, WeakSet, Date, RegExp, Function, Error types, Symbol, Proxy, Reflect, console.

**Not yet implemented:** ES Modules (import/export), for-await-of, private class members (#fields), BigInt, some decorator edge cases.

See design.md for complete feature checklist, profiling.md for performance notes.
