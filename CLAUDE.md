# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A minimal TypeScript runtime in Rust designed for embedding in applications. The primary use case is configuration files where users benefit from IDE autocompletion, type checking, and error highlighting.

**Key characteristics:**
- **TypeScript-native** - Supports enums, interfaces, type annotations, and generics (types are stripped at runtime, not checked)
- **Minimal footprint** - No Node.js dependency, designed for embedding
- **Register-based bytecode VM** - Efficient execution with ES modules and async/await

## Quick Reference

### Build & Test Commands

```bash
cargo build                              # Build the project
cargo build --features c-api             # Build with C FFI support
cargo build --release --features c-api   # Release build with C FFI (creates libtsrun.so)
timeout 30 cargo test                    # Run all tests (always use timeout!)
timeout 30 cargo test --test interpreter # Run interpreter integration tests
timeout 30 cargo test test_name          # Run specific test
timeout 30 cargo test -- --nocapture     # Show test output
```

### Key Files

| File/Directory | Purpose |
|----------------|---------|
| `src/lib.rs` | Public API - `Interpreter`, `InterpreterConfig` |
| `src/api.rs` | High-level API for stepping execution |
| `src/lexer.rs` | Tokenizer |
| `src/parser.rs` | Recursive descent + Pratt parsing |
| `src/ast.rs` | AST node types |
| `src/value.rs` | Runtime values, object model |
| `src/gc.rs` | Garbage collector, Guard system, Heap |
| `src/error.rs` | JsError types |
| `src/compiler/` | Bytecode compiler |
| `src/interpreter/` | VM and builtins |
| `src/ffi/` | C FFI module (feature-gated: `c-api`) |
| `src/wasm/mod.rs` | WASM API (feature-gated: `wasm`) |
| `tests/interpreter/` | Integration tests by feature |
| `examples/c-embedding/` | C API usage examples |
| `examples/wasm-playground/` | WASM playground source |

## Development Rules

- **Always use the Edit tool** - never shell commands like `echo >>` to modify files
- **Use haiku agents for bulk changes** - for repetitive edits across multiple files (renames, pattern replacements), spawn Task agents with `model: "haiku"` instead of using sed/awk
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
| `JsValue` | Enum: Undefined, Null, Boolean, Number, String, Object, Symbol |
| `Gc<JsObject>` | GC-managed object pointer |
| `JsString` | `Rc<str>` reference-counted string |
| `JsSymbol` | Symbol primitive with description |
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
- `bytecode.rs` - Bytecode instruction definitions (Op enum)
- `builder.rs` - Bytecode builder with register allocation
- `hoist.rs` - Variable hoisting

**Interpreter** (`src/interpreter/`):
- `mod.rs` - Main interpreter, environment management
- `bytecode_vm.rs` - Register-based bytecode VM execution engine

**Builtins** (`src/interpreter/builtins/`):
- `array.rs`, `string.rs`, `number.rs`, `object.rs` - Core types
- `function.rs`, `math.rs`, `json.rs`, `date.rs` - Standard objects
- `regexp.rs`, `map.rs`, `set.rs`, `error.rs` - Other builtins
- `promise.rs`, `generator.rs` - Async primitives
- `proxy.rs` - Proxy and Reflect objects
- `symbol.rs`, `boolean.rs`, `console.rs` - Additional builtins
- `global.rs` - Global functions (parseInt, parseFloat, etc.)

**C FFI** (`src/ffi/`, feature-gated):
- `mod.rs` - Types, result structs, utility functions
- `context.rs` - Context lifecycle and step-based execution
- `value.rs` - Value creation, inspection, object/array operations
- `native.rs` - Native C function callback system
- `module.rs` - Module system (provide_module, exports)
- `order.rs` - Async order system (pending orders, fulfillment)

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
| `tests/compiler.rs` | Compiler integration tests |
| `tests/parser.rs` | Parser integration tests |
| `tests/lexer.rs` | Lexer integration tests |
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

### TypeScript Features

**Supported:**
- Type annotations, interfaces, type aliases → parsed but stripped at runtime
- `enum` declarations → compile to object literals with reverse mappings
- Generic functions and classes → type parameters parsed and stripped
- Type assertions (`x as T`, `<T>x`) → evaluate to just the expression
- Parameter properties (`constructor(public x: number)`) → desugared to assignments
- Optional parameters (`x?: number`) and default values

**Also supported:**
- Decorators (class, method, property, parameter)
- Namespaces
- `eval()` for dynamic code evaluation

**Not supported:**
- Type checking (no type errors at runtime)

## C FFI

The interpreter can be embedded in C/C++ applications via a C API (feature-gated behind `c-api`).

### Building

```bash
cargo build --release --features c-api
# Output: target/release/libtsrun.so (Linux), .dylib (macOS), .dll (Windows)
```

### Key Concepts

- **Opaque handles**: `TsRunContext*`, `TsRunValue*` - all types are opaque pointers
- **Step-based execution**: `tsrun_prepare()` → `tsrun_run()` loop handling `NeedImports`/`Suspended`
- **Native callbacks**: C functions callable from JS via `tsrun_native_function()`
- **Order system**: Async operations via `tsrun_create_pending_order()` + `tsrun_fulfill_orders()`

### Examples

See `examples/c-embedding/` for working examples:
- `basic.c` - Value creation, objects, arrays, JSON
- `native_functions.c` - C callbacks, stateful functions, error handling
- `module_loading.c` - ES module loading with virtual filesystem
- `async_orders.c` - Async operations via pending orders

### Header

The C header is at `examples/c-embedding/tsrun.h`. Key functions:

```c
// Lifecycle
TsRunContext* tsrun_new(void);
void tsrun_free(TsRunContext* ctx);

// Execution
TsRunResult tsrun_prepare(TsRunContext* ctx, const char* code, const char* path);
TsRunStepResult tsrun_run(TsRunContext* ctx);

// Values
TsRunValue* tsrun_number(TsRunContext* ctx, double n);
TsRunValue* tsrun_string(TsRunContext* ctx, const char* s);
TsRunValueResult tsrun_get(TsRunContext* ctx, TsRunValue* obj, const char* key);

// Native functions
TsRunValueResult tsrun_native_function(TsRunContext* ctx, const char* name,
                                        TsRunNativeFn func, size_t arity, void* userdata);

// Async orders
TsRunValueResult tsrun_create_pending_order(TsRunContext* ctx, TsRunValue* payload,
                                             TsRunOrderId* order_id_out);
TsRunResult tsrun_fulfill_orders(TsRunContext* ctx, const TsRunOrderResponse* responses,
                                  size_t count);
```

## WASM Playground

The interpreter compiles to WebAssembly for browser-based execution with a step-based API.

### Building

```bash
cd examples/wasm-playground
./build.sh              # Build WASM module and copy to site/playground/pkg
./build.sh --test       # Build and run e2e tests
```

### Files

| Location | Purpose |
|----------|---------|
| `examples/wasm-playground/` | **Source** - playground HTML, JS, and build scripts |
| `examples/wasm-playground/pkg/` | Built WASM output |
| `site/playground/` | **Copy** - synced by build.sh |
| `src/wasm/mod.rs` | WASM API (TsRunner, step-based execution) |
| `src/platform/wasm_impl.rs` | WASM-specific platform code |

### Step-Based API

The WASM module exposes a step-based execution API where JavaScript controls the execution loop:

```javascript
import init, { TsRunner, STEP_CONTINUE, STEP_COMPLETE, STEP_ERROR, STEP_SUSPENDED } from './pkg/tsrun.js';

await init();
const runner = new TsRunner();

// Load constants (they're functions that return values)
const StepStatus = {
    CONTINUE: STEP_CONTINUE(),
    COMPLETE: STEP_COMPLETE(),
    NEED_IMPORTS: STEP_NEED_IMPORTS(),
    SUSPENDED: STEP_SUSPENDED(),
    DONE: STEP_DONE(),
    ERROR: STEP_ERROR()
};

// Prepare code
const prepResult = runner.prepare(code, 'script.ts');
if (prepResult.status === StepStatus.ERROR) {
    console.error(prepResult.error);
    return;
}

// Main execution loop
while (true) {
    const result = runner.step();

    // Display console output from this step
    for (const entry of result.console_output) {
        console.log(`[${entry.level}] ${entry.message}`);
    }

    switch (result.status) {
        case StepStatus.CONTINUE:
            continue;
        case StepStatus.COMPLETE:
            console.log('Result:', result.value);
            return;
        case StepStatus.DONE:
            return;
        case StepStatus.ERROR:
            console.error(result.error);
            return;
        case StepStatus.NEED_IMPORTS:
            console.error('Imports:', runner.get_import_requests());
            return;
        case StepStatus.SUSPENDED:
            // Handle async orders (see below)
            const orders = runner.get_pending_orders();
            const responses = await handleOrders(orders);
            runner.fulfill_orders(responses);
            continue;
    }
}
```

### Status Constants

| Function | Value | Meaning |
|----------|-------|---------|
| `STEP_CONTINUE()` | 0 | More to execute, call step() again |
| `STEP_COMPLETE()` | 1 | Finished with a value in `result.value` |
| `STEP_NEED_IMPORTS()` | 2 | Waiting for modules (call `get_import_requests()`) |
| `STEP_SUSPENDED()` | 3 | Waiting for orders (call `get_pending_orders()`) |
| `STEP_DONE()` | 4 | Finished, no return value |
| `STEP_ERROR()` | 5 | Error in `result.error` |

### Async Order System

For async operations, TypeScript code uses `order` from the `tsrun:host` module:

```typescript
import { order } from "tsrun:host";

function fetch(url: string): Promise<any> {
    return order({ type: "fetch", url });
}

const data = await fetch("/api/users");
```

JavaScript handles these orders and fulfills them:

```javascript
async function handleOrders(orders) {
    const responses = [];
    for (const order of orders) {
        const { id, payload } = order;
        // payload contains { type: "fetch", url: "..." }

        // Simulate async operation with setTimeout
        await new Promise(r => setTimeout(r, 100));

        // Mock response based on payload
        const result = { data: "mock" };
        responses.push({ id, result });
    }
    return responses;
}
```

### TsRunner Methods

| Method | Description |
|--------|-------------|
| `new TsRunner()` | Create new runner instance |
| `prepare(code, filename)` | Compile code, returns WasmStepResult |
| `step()` | Execute one step, returns WasmStepResult |
| `get_pending_orders()` | Get orders when Suspended (returns `[{id, payload}]`) |
| `get_import_requests()` | Get module specifiers when NeedImports |
| `fulfill_orders(responses)` | Provide order responses (`[{id, result?, error?}]`) |

### WasmStepResult Properties

| Property | Type | Description |
|----------|------|-------------|
| `status` | number | StepStatus enum value |
| `value` | string? | Result value (for Complete status) |
| `error` | string? | Error message (for Error status) |
| `console_output` | ConsoleEntry[] | Console output from this step |

### Notes

- Uses `wasm-pack` with `--target web`
- Feature-gated: builds with `--features wasm` and `--no-default-features`
- Imports in builtins must use `crate::prelude::Box` (not `std::boxed::Box`) for `no_std` compatibility
- Each `TsRunner` instance is independent (no shared state)

## Implementation Status

**TypeScript Features:** enums, interfaces, type annotations, generics, type assertions, parameter properties, optional parameters, decorators, namespaces.

**Language Features:** variables, functions, closures, control flow, classes with inheritance/static blocks, destructuring, spread, template literals, all operators, generators, async/await, Promises, eval().

**Built-in Objects:** Array, String, Object, Number, Math, JSON, Map, Set, WeakMap, WeakSet, Date, RegExp, Function, Error types, Symbol, Proxy, Reflect, console.

**Embedding:** Rust API, C FFI with native callbacks, module loading, async order system, and WASM support for browser execution.
