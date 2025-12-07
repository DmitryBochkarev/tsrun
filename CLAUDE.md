# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TypeScript interpreter written in Rust for synchronous config/manifest generation. Types are parsed but stripped at runtime (not type-checked). The project is in early development (Milestone 1 complete: basic expressions).

## Build and Test Commands

```bash
cargo build                    # Build the project
cargo test                     # Run all tests
cargo test --test interpreter  # Run only interpreter integration tests
cargo test lexer               # Run only lexer tests
cargo test parser              # Run only parser tests
cargo test -- --nocapture      # Show test output
```

## Development Workflow

Use TDD (Test-Driven Development) for new features:
1. Write a failing test that demonstrates the desired behavior
2. Implement the minimal code to make the test pass
3. Refactor if needed while keeping tests green

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

The interpreter follows a classic pipeline:

```
Source → Lexer → Parser → AST → Interpreter → JsValue
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
- `JsObjectRef`: `Rc<RefCell<JsObject>>` - shared mutable reference to objects
- `Completion`: Control flow enum (`Normal`, `Return`, `Break`, `Continue`)

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
- `Error`: Error, TypeError, ReferenceError, SyntaxError, RangeError
- Global: parseInt, parseFloat, isNaN, isFinite, encodeURI, decodeURI, encodeURIComponent, decodeURIComponent, console.log/error/warn/info/debug

**Not yet implemented:** module resolution/loading, generators, Promise, WeakMap/WeakSet.

See design.md for the complete feature checklist.