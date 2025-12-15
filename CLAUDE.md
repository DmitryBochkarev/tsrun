# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TypeScript interpreter written in Rust for config/manifest generation with support for ES modules and async/await. Types are parsed but stripped at runtime (not type-checked). The interpreter uses an explicit evaluation stack for true state capture, enabling suspension at import/await points.

## Build and Test Commands

```bash
cargo build                    # Build the project
cargo test                     # Run all tests
cargo test --test interpreter  # Run only interpreter integration tests
cargo test lexer               # Run only lexer tests
cargo test parser              # Run only parser tests
cargo test -- --nocapture      # Show test output
```

### Running Specific Tests

```bash
# Run tests matching a pattern
cargo test test_name           # Run tests containing "test_name"
cargo test string_match        # Run all string_match* tests
cargo test test_tdz            # Run all TDZ-related tests

# Run tests in a specific module
cargo test string::            # Run all tests in string module
cargo test array::             # Run all tests in array module

# Run a single specific test with output
cargo test test_string_match_basic -- --nocapture

# Run tests and show all output (including passing tests)
cargo test -- --nocapture --show-output
```

### Test Organization

Tests are located in:
- `tests/interpreter/` - Integration tests organized by feature (array.rs, string.rs, etc.)
- `src/parser.rs` (bottom) - Parser unit tests
- `src/value.rs` (bottom) - Value type unit tests

## Important Rules

- **Always use the proper Edit tool to modify files** - never use shell commands like `echo >>` to modify files
- Prefer small, focused edits over large rewrites
- **Always add TypeScript type annotations in test code** - even though types are stripped at runtime, tests should use proper TypeScript syntax
- **No tech debt**: Fix failing tests immediately before moving on. Do not leave TODO/FIXME comments for known bugs - implement the fix or ask for clarification
- **Use TDD**: If a test fails because a feature is not implemented, implement the feature first rather than deleting or modifying the test to work around the limitation
- **Never change failing test cases** - if a test fails because a syntax/feature is not yet supported, write additional simpler tests to verify the current implementation scope, but keep the original test as a goal to implement the missing feature
- **Debugging tests** - write debug tests in test files and run with `cargo test test_name -- --nocapture` to see output. Do not use heredoc/echo commands in bash to run code.

## Zero-Panic Policy

This codebase enforces a **zero-panic policy** via Clippy lints. The following patterns are **denied** in production code:

| Pattern | Alternative |
|---------|-------------|
| `.unwrap()` | Use `.ok_or_else()`, `if let`, or `match` |
| `.expect()` | Use `.ok_or_else()` with descriptive error |
| `[index]` | Use `.get(index)` with error handling |
| `panic!()` | Return `Err(JsError::...)` |
| `unreachable!()` | Return `Err(JsError::internal_error(...))` |
| `todo!()` | Implement the feature or return error |
| `unimplemented!()` | Return `Err(JsError::type_error(...))` |
| `&str[start..end]` | Use `.get(start..end)` for safe slicing |

### Clippy Configuration

The lints are configured in `Cargo.toml`:
```toml
[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
indexing_slicing = "deny"
panic = "deny"
unreachable = "deny"
todo = "deny"
unimplemented = "deny"
string_slice = "deny"
```

Test code is exempt via `clippy.toml`:
```toml
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-indexing-slicing-in-tests = true
allow-panic-in-tests = true
```

### Safe Access Patterns

**For function arguments:**
```rust
// Instead of: args[0]
let first = args.first().cloned().unwrap_or(JsValue::Undefined);

// Instead of: args[1]
let second = args.get(1).cloned().unwrap_or(JsValue::Undefined);
```

**For array/vector access:**
```rust
// Instead of: elements[i]
let elem = elements.get(i).ok_or_else(|| JsError::internal_error("index out of bounds"))?;

// For slicing: args[i..]
let rest = args.get(i..).unwrap_or_default().to_vec();
```

**For string slicing:**
```rust
// Instead of: s[start..end]
let slice = s.get(start..end).unwrap_or("");
```

**For Option unwrapping:**
```rust
// Instead of: opt.unwrap()
let value = opt.ok_or_else(|| JsError::internal_error("expected value"))?;
```

### Guarded Destructuring Rule

The `Guarded` struct (from `interpreter/mod.rs`) must **ALWAYS** be accessed through destructuring:

```rust
// CORRECT: Always use destructuring to keep guard alive
let Guarded { value, guard: _guard } = self.evaluate_expression(expr)?;
// _guard keeps the object alive until end of scope
// Now use `value` safely

// WRONG: Never access .value directly (drops guard prematurely!)
let val = self.evaluate_expression(expr)?.value;  // BUG: GC may collect the object!
```

**Why this matters:** The guard keeps newly created objects alive in the GC. If you drop the guard before you're done using the value, the garbage collector may reclaim the object (and its prototype chain), causing "is not a function" errors or other GC-related bugs.

**Pattern for conditional evaluation:**
```rust
let (val, _guard) = if some_condition {
    let Guarded { value, guard } = self.evaluate_expression(expr)?;
    (value, guard)
} else {
    (JsValue::Undefined, None)
};
```

## Development Workflow

Use TDD (Test-Driven Development) for new features:
1. **Verify parser support first** - Before implementing an interpreter feature, write a parser test to ensure the syntax is correctly parsed. If parsing fails, implement parser support first.
2. Write a failing interpreter test that demonstrates the desired behavior
3. Implement the minimal code to make the test pass
4. Refactor if needed while keeping tests green
5. **Run quality checks** - After implementing each feature, run:
   ```bash
   cargo test && cargo fmt && cargo clippy
   ```
   Fix any test failures, formatting issues, or clippy warnings before committing.

### Parser Testing Before Implementation

When implementing a new language feature (e.g., private fields, class methods, etc.):

1. **Write parser tests** in `src/parser.rs` tests section - include both JavaScript (no types) and TypeScript (with types) variants:
```rust
#[test]
fn test_parse_class_method() {
    // JavaScript style (no types)
    let source = "class Foo { bar() { return 1; } }";
    parse(source).expect("should parse JS class");

    // TypeScript style (with types)
    let source_ts = "class Foo { bar(): number { return 1; } }";
    parse(source_ts).expect("should parse TS class");
}

#[test]
fn test_parse_private_field() {
    // JavaScript style
    let source = "class Foo { #bar = 1; }";
    parse(source).expect("should parse JS private field");

    // TypeScript style
    let source_ts = "class Foo { #bar: number = 1; }";
    parse(source_ts).expect("should parse TS private field");
}
```

2. **Run the parser test** to verify parsing works:
```bash
cargo test test_parse_private_field -- --nocapture
```

3. **Only then** proceed to interpreter implementation and tests.

**Note:** Always test BOTH JavaScript and TypeScript syntax variants. Types should be parsed but stripped at runtime.

### Implementing Built-in Methods

When adding new built-in methods (Array, String, Object, etc.), follow this pattern:

#### 1. Write Tests First
Add tests in the appropriate file under `tests/interpreter/` (e.g., `tests/interpreter/array.rs`):
```rust
#[test]
fn test_array_mymethod() {
    assert_eq!(eval("[1,2,3].myMethod()"), JsValue::Number(expected));
}
```

#### 2. Add the Native Function Implementation
Add the implementation in the appropriate builtins file (e.g., `interpreter/builtins/array.rs`):
```rust
pub fn array_my_method(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error("Array.prototype.myMethod called on non-object"));
    };
    // Implementation here
    Ok(result)
}
```

#### 3. Register the Method on Prototype
In the same builtins file, add the method registration in the `create_*_prototype()` function:
```rust
let mymethod_fn = create_function(JsFunction::Native(NativeFunction {
    name: "myMethod".to_string(),
    func: array_my_method,
    arity: 1,
}));
p.set_property(PropertyKey::from("myMethod"), JsValue::Object(mymethod_fn));
```

#### 4. Update design.md
Mark the feature as implemented: `- [x] \`Array.prototype.myMethod()\``

#### 5. Commit
After tests pass, commit with descriptive message.

### Native Function Signature
All native functions follow this signature:
```rust
fn name(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError>
```
- `interp`: Interpreter instance (for calling other functions, creating arrays, etc.)
- `this`: The receiver object (e.g., the array for array methods)
- `args`: Function arguments as a vector

### Prototype Chain
- Objects fall back to `object_prototype` for methods like `hasOwnProperty`, `toString`
- Arrays have `array_prototype` with all array methods
- Strings use `string_prototype` (looked up in `evaluate_member`)
- Numbers use `number_prototype` (looked up in `evaluate_member`)

### Common Patterns

**Getting array length:**
```rust
let length = match &arr.borrow().exotic {
    ExoticObject::Array { length } => *length,
    _ => return Err(JsError::type_error("Not an array")),
};
```

**Updating array length (must update both):**
```rust
if let ExoticObject::Array { ref mut length } = arr_ref.exotic {
    *length = new_length;
}
arr_ref.set_property(PropertyKey::from("length"), JsValue::Number(new_length as f64));
```

**Calling a callback function:**
```rust
let result = interp.call_function(
    callback.clone(),
    this_arg.clone(),
    vec![elem, JsValue::Number(index as f64), this.clone()],
)?;
```

**Creating a new array with results:**
```rust
Ok(JsValue::Object(interp.create_array(elements)))
```

## Architecture

The interpreter follows a pipeline with support for suspension:

```
Source → Lexer → Parser → AST → Interpreter → RuntimeResult
                                                   │
                              ┌────────────────────┼────────────────────┐
                              ▼                    ▼                    ▼
                         Complete            ImportAwaited        AsyncAwaited
                          (value)            (slot, spec)        (slot, promise)
```

### State Machine Execution Model

The interpreter uses an **explicit evaluation stack** instead of Rust's call stack. This enables:
- **True state capture**: Save exact position, resume without re-execution
- **Suspension at imports**: Return to host to load modules
- **Suspension at await**: Return to host to resolve promises

The stack-based execution model is now fully implemented.

**Key Types:**
```rust
pub enum RuntimeResult {
    Complete(JsValue),                              // Finished
    ImportAwaited { slot: PendingSlot, specifier }, // Need module
    AsyncAwaited { slot: PendingSlot, promise },    // Need promise resolution
}
```

**Host Loop Pattern:**
```rust
let mut result = runtime.eval(source)?;
loop {
    match result {
        RuntimeResult::Complete(value) => break value,
        RuntimeResult::ImportAwaited { slot, specifier } => {
            let module = load_module(&specifier)?;
            slot.set_success(module);
        }
        RuntimeResult::AsyncAwaited { slot, .. } => {
            let value = resolve_async()?;
            slot.set_success(value);
        }
    }
    result = runtime.continue_eval()?;
}
```

### Module Structure

- **lib.rs**: Public API - `Runtime` struct with `eval()` method
- **lexer.rs**: Tokenizer with span tracking for error reporting
- **parser.rs**: Recursive descent + Pratt parsing for expressions
- **ast.rs**: All AST node types (statements, expressions, patterns, types)
- **value.rs**: Runtime values (`JsValue` enum), object model, environments
- **interpreter/mod.rs**: Statement execution and expression evaluation
- **interpreter/builtins/**: Built-in function implementations (split by type)
- **error.rs**: Error types (`JsError`) with source locations
- **tests/interpreter/**: Integration tests organized by feature

### Builtins Module Structure

Each builtin type has its own file in `interpreter/builtins/`:

| File | Contents |
|------|----------|
| `array.rs` | `create_array_prototype()`, `create_array_constructor()`, array methods |
| `string.rs` | `create_string_prototype()`, `create_string_constructor()`, string methods |
| `number.rs` | `create_number_prototype()`, `create_number_constructor()`, number methods |
| `object.rs` | `create_object_prototype()`, `create_object_constructor()`, object methods |
| `function.rs` | `create_function_prototype()`, call/apply/bind |
| `math.rs` | `create_math_object()`, math functions and constants |
| `json.rs` | `create_json_object()`, stringify/parse |
| `console.rs` | `create_console_object()`, log/error/warn/info/debug |
| `date.rs` | `create_date_prototype()`, `create_date_constructor()`, date methods |
| `regexp.rs` | `create_regexp_prototype()`, `create_regexp_constructor()`, test/exec |
| `map.rs` | `create_map_prototype()`, `create_map_constructor()`, map methods |
| `set.rs` | `create_set_prototype()`, `create_set_constructor()`, set methods |
| `error.rs` | `create_error_constructors()`, Error/TypeError/etc. |
| `global.rs` | `register_global_functions()`, parseInt/parseFloat/isNaN/etc. |

### Test Structure

Integration tests are located in `tests/interpreter/` and organized by feature:

| File | Contents |
|------|----------|
| `main.rs` | Entry point, declares modules, shared `eval()` helper |
| `array.rs` | Array method tests (push, pop, map, filter, etc.) |
| `basics.rs` | Basic language features (arithmetic, variables, conditionals) |
| `console.rs` | Console methods (log, error, warn, info, debug) |
| `date.rs` | Date object tests |
| `error.rs` | Error constructor tests |
| `function.rs` | Function features (call, apply, bind, arrows) |
| `global.rs` | Global functions (parseInt, parseFloat, isNaN, etc.) |
| `map.rs` | Map object tests |
| `math.rs` | Math object tests |
| `number.rs` | Number object tests |
| `object.rs` | Object method tests |
| `regexp.rs` | RegExp tests |
| `set.rs` | Set object tests |
| `string.rs` | String method tests |

Each test file uses a shared `eval()` helper from `main.rs`:
```rust
use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_example() {
    assert_eq!(eval("1 + 2"), JsValue::Number(3.0));
}
```

### Key Types

- `JsValue`: Enum with `Undefined`, `Null`, `Boolean(bool)`, `Number(f64)`, `String(JsString)`, `Object(JsObjectRef)`
- `JsObjectRef`: `Rc<RefCell<JsObject>>` - shared mutable reference to objects (cheap clone)
- `JsString`: `Rc<str>` - reference-counted string (cheap clone)
- `PendingSlot`: Slot for async/import resolution (cheap clone)
- `Completion`: Control flow enum (`Normal`, `Return`, `Break`, `Continue`)

### Clone Conventions (CheapClone Trait)

The codebase distinguishes between cheap (O(1), reference-counted) and expensive clones using the `CheapClone` trait.

**Cheap clones - use `.cheap_clone()`:**
```rust
// JsObjectRef (Rc<RefCell<JsObject>>)
arr.borrow_mut().prototype = Some(self.array_prototype.cheap_clone());

// JsString (Rc<str>)
let s = js_string.cheap_clone();

// PendingSlot (contains Rc)
self.pending_slot = Some(slot.cheap_clone());
```

**Expensive clones - add comment explaining why:**
```rust
// AST clone - needed to release borrow before execution
state.body.clone(),

// Environment clone - needed to restore after execution
let saved_env = self.env.clone();

// Vec<JsValue> clone - needed for bound function args
let mut full_args = bound_data.bound_args.clone();

// String clone - env.define takes ownership
self.env.define(id.name.clone(), value, mutable);
```

**Type classification:**
| Type | Clone Cost | Notes |
|------|-----------|-------|
| `JsObjectRef` | Cheap | Use `.cheap_clone()` |
| `JsString` | Cheap | Use `.cheap_clone()` |
| `PendingSlot` | Cheap | Use `.cheap_clone()` |
| `Rc<T>` | Cheap | Use `.cheap_clone()` |
| `JsValue` | Varies | May contain Rc types or expensive variants |
| `Environment` | Expensive | Contains `Box<Environment>` chain |
| `String`, `Vec<T>` | Expensive | Heap allocations |
| AST types | Expensive | Deep structure clones |

### TypeScript Handling

- Type annotations, interfaces, and type aliases are parsed but become no-ops at runtime
- `enum` declarations compile to object literals
- Type assertions (`x as T`, `<T>x`) and non-null assertions (`x!`) evaluate to just the expression

## Current Implementation Status

**Language Features:** variables (let/const/var), functions (declarations, expressions, arrows), closures, control flow (if/for/while/switch/try-catch), classes with inheritance, object/array literals, destructuring, spread operator, template literals, most operators.

**Built-in Objects:**
- `Array`: isArray, from, of, push, pop, shift, unshift, slice, splice, concat, join, reverse, sort, indexOf, lastIndexOf, includes, find, findIndex, findLast, findLastIndex, filter, map, forEach, reduce, reduceRight, every, some, flat, flatMap, fill, copyWithin, at, toReversed, toSorted, toSpliced, with
- `String`: fromCharCode, charAt, charCodeAt, at, indexOf, lastIndexOf, includes, startsWith, endsWith, slice, substring, toLowerCase, toUpperCase, trim, trimStart, trimEnd, split, repeat, replace, replaceAll, padStart, padEnd, concat
- `Object`: keys, values, entries, assign, fromEntries, hasOwn, create, freeze, isFrozen, seal, isSealed, hasOwnProperty, toString, valueOf
- `Number`: isNaN, isFinite, isInteger, isSafeInteger, parseInt, parseFloat, toFixed, toString, toPrecision, toExponential, constants
- `Math`: abs, floor, ceil, round, trunc, sign, min, max, pow, sqrt, cbrt, hypot, log, log10, log2, log1p, exp, expm1, sin, cos, tan, asin, acos, atan, atan2, sinh, cosh, tanh, asinh, acosh, atanh, random, PI, E, LN2, LN10, LOG2E, LOG10E, SQRT2, SQRT1_2
- `JSON`: stringify, parse
- `Map`: get, set, has, delete, clear, forEach, size
- `Set`: add, has, delete, clear, forEach, size
- `Date`: now, UTC, parse, getTime, getFullYear, getMonth, getDate, getDay, getHours, getMinutes, getSeconds, getMilliseconds, toISOString, toJSON, valueOf
- `RegExp`: test, exec, source, flags, global, ignoreCase, multiline
- `Function`: call, apply, bind
- `Error`: Error, TypeError, ReferenceError, SyntaxError, RangeError, URIError, EvalError
- `Symbol`: Symbol(), Symbol.for(), Symbol.keyFor(), well-known symbols (iterator, toStringTag, hasInstance)
- Global: parseInt, parseFloat, isNaN, isFinite, encodeURI, decodeURI, encodeURIComponent, decodeURIComponent, console.log/error/warn/info/debug
- Generators: function*, yield, yield*
- Namespace declarations with export and merging
- `Promise`: new, resolve, reject, then, catch, finally, all, race, allSettled, any
- Async/await: async functions, async arrow functions, await expressions
- Dynamic `import()`: import() expression returning Promise

**Not yet implemented:**
- ES Modules (import/export resolution - parsing only)
- WeakMap/WeakSet

See design.md for the complete feature checklist.