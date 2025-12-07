# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TypeScript interpreter written in Rust for synchronous config/manifest generation. Types are parsed but stripped at runtime (not type-checked). The project is in early development (Milestone 1 complete: basic expressions).

## Build and Test Commands

```bash
cargo build           # Build the project
cargo test            # Run all tests
cargo test lexer      # Run only lexer tests
cargo test parser     # Run only parser tests
cargo test -- --nocapture  # Show test output
```

## Development Workflow

Use TDD (Test-Driven Development) for new features:
1. Write a failing test that demonstrates the desired behavior
2. Implement the minimal code to make the test pass
3. Refactor if needed while keeping tests green

### Implementing Built-in Methods

When adding new built-in methods (Array, String, Object, etc.), follow this pattern:

#### 1. Write Tests First
Add tests in `interpreter.rs` in the `mod tests` section:
```rust
#[test]
fn test_array_mymethod() {
    assert_eq!(eval("[1,2,3].myMethod()"), JsValue::Number(expected));
}
```

#### 2. Register the Method on Prototype
In `Interpreter::new()`, find the appropriate prototype setup section and add:
```rust
let mymethod_fn = create_function(JsFunction::Native(NativeFunction {
    name: "myMethod".to_string(),
    func: array_my_method,  // native function reference
    arity: 1,               // number of expected arguments
}));
proto.set_property(PropertyKey::from("myMethod"), JsValue::Object(mymethod_fn));
```

#### 3. Implement the Native Function
Add the implementation near other methods of the same type:
```rust
fn array_my_method(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error("Array.prototype.myMethod called on non-object"));
    };
    // Implementation here
    Ok(result)
}
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
- **interpreter.rs**: Statement execution and expression evaluation
- **error.rs**: Error types (`JsError`) with source locations

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
- `Array`: isArray, from, of, push, pop, shift, unshift, slice, splice, concat, join, reverse, sort, indexOf, lastIndexOf, includes, find, findIndex, filter, map, forEach, reduce, reduceRight, every, some, flat, flatMap, fill, copyWithin, at
- `String`: fromCharCode, charAt, charCodeAt, indexOf, includes, startsWith, endsWith, slice, substring, toLowerCase, toUpperCase, trim, trimStart, trimEnd, split, repeat, replace, padStart, padEnd, concat
- `Object`: keys, values, entries, assign, hasOwnProperty, toString, valueOf
- `Number`: isNaN, isFinite, isInteger, isSafeInteger, parseInt, parseFloat, toFixed, toString, constants
- `Math`: abs, floor, ceil, round, trunc, sign, min, max, pow, sqrt, log, exp, sin, cos, tan, random, PI, E, etc.
- `JSON`: stringify, parse
- Global: parseInt, parseFloat, isNaN, isFinite, console.log

**Not yet implemented:** module resolution/loading, generators, Map/Set, Date, RegExp, Promise.

See design.md for the complete feature checklist.
