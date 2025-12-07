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

Implemented: variables (let/const/var), functions (declarations, expressions, arrows), closures, control flow (if/for/while/switch/try-catch), classes with inheritance, object/array literals, destructuring, spread operator, template literals, most operators.

Not yet implemented: module resolution/loading, generators, Map/Set, Date, RegExp, most Array/String prototype methods.

See design.md for the complete feature checklist.
