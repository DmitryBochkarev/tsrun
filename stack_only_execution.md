# Stack-Only Execution Migration Plan

This document contains atomic, actionable items for removing the recursive (non-stack) execution model and keeping only the stack-based (`execute_resumable`) implementation.

## Overview

**Current State**: Two execution paths exist:
1. `Interpreter::execute()` / `Runtime::eval()` - Recursive, uses Rust call stack
2. `Interpreter::execute_resumable()` / `Runtime::eval_resumable()` - Stack-based, supports suspension

**Target State**: Single stack-based execution model with simplified API.

---

## Phase 1: Create Convenience Wrapper

Before removing the recursive implementation, add a convenience method that wraps `eval_resumable` for simple cases.

### 1.1 Add `eval_simple()` to Runtime (lib.rs)

Add a new method that calls `eval_resumable()` and extracts the value for `Complete` cases:

```rust
/// Evaluate TypeScript source code, expecting immediate completion.
///
/// This is a convenience method for code that doesn't use imports or async.
/// Returns an error if execution suspends (ImportAwaited/AsyncAwaited).
pub fn eval_simple(&mut self, source: &str) -> Result<JsValue, JsError> {
    match self.eval_resumable(source)? {
        RuntimeResult::Complete(value) => Ok(value),
        RuntimeResult::ImportAwaited { specifier, .. } => {
            Err(JsError::type_error(&format!(
                "Execution suspended for import '{}' - use eval_resumable() for code with imports",
                specifier
            )))
        }
        RuntimeResult::AsyncAwaited { .. } => {
            Err(JsError::type_error(
                "Execution suspended for async - use eval_resumable() for async code"
            ))
        }
    }
}
```

**Files**: `src/lib.rs`
**Test**: Add test in lib.rs that verifies `eval_simple("1 + 2")` returns `JsValue::Number(3.0)`

---

## Phase 2: Migrate Test Infrastructure

### 2.1 Update test helper in `tests/interpreter/main.rs`

Change the `eval()` helper from using `Interpreter::execute()` to using `Runtime`:

**Before:**
```rust
pub fn eval(source: &str) -> JsValue {
    let mut parser = Parser::new(source);
    let program = parser.parse_program().unwrap();
    let mut interp = Interpreter::new();
    interp.execute(&program).unwrap()
}
```

**After:**
```rust
pub fn eval(source: &str) -> JsValue {
    let mut runtime = Runtime::new();
    match runtime.eval_resumable(source).unwrap() {
        RuntimeResult::Complete(value) => value,
        other => panic!("Expected Complete, got {:?}", other),
    }
}
```

**Files**: `tests/interpreter/main.rs`
**Imports to add**: `use typescript_eval::{Runtime, RuntimeResult};`
**Imports to remove**: `use typescript_eval::parser::Parser;`, `use typescript_eval::Interpreter;`

### 2.2 Update `eval_result()` helper

**Before:**
```rust
pub fn eval_result(source: &str) -> Result<JsValue, JsError> {
    let mut parser = Parser::new(source);
    let program = parser.parse_program().unwrap();
    let mut interp = Interpreter::new();
    interp.execute(&program)
}
```

**After:**
```rust
pub fn eval_result(source: &str) -> Result<JsValue, JsError> {
    let mut runtime = Runtime::new();
    match runtime.eval_resumable(source)? {
        RuntimeResult::Complete(value) => Ok(value),
        other => panic!("Expected Complete, got {:?}", other),
    }
}
```

**Files**: `tests/interpreter/main.rs`

### 2.3 Run all tests to verify helpers work

```bash
cargo test --test interpreter
```

All ~800+ tests should pass with no changes to individual test files.

---

## Phase 3: Migrate exports.rs Tests

The `tests/interpreter/exports.rs` file uses `Runtime::eval()` directly (25 calls).

### 3.1 Create helper function in exports.rs

Add at the top of the file:

```rust
fn run_eval(runtime: &mut Runtime, source: &str) {
    match runtime.eval_resumable(source).unwrap() {
        RuntimeResult::Complete(_) => {}
        other => panic!("Expected Complete, got {:?}", other),
    }
}
```

**Files**: `tests/interpreter/exports.rs`
**Imports to add**: `use typescript_eval::RuntimeResult;`

### 3.2 Replace `runtime.eval(source).unwrap()` with `run_eval(&mut runtime, source)`

Find and replace all 25 occurrences:
- Line 31, 80, 95, 113, 130, 149, 167, 184, 201, 223, 240, 266, 297, 316, 333, 357, 401, 438, 455, 472, 487, 524, 609, 674, 712

**Files**: `tests/interpreter/exports.rs`

### 3.3 Run exports tests

```bash
cargo test --test interpreter exports
```

---

## Phase 4: Update lib.rs Doctests and Tests

### 4.1 Update module-level doctest (lib.rs lines 5-11)

**Before:**
```rust
//! let result = runtime.eval("1 + 2 * 3").unwrap();
```

**After:**
```rust
//! let result = match runtime.eval_resumable("1 + 2 * 3").unwrap() {
//!     typescript_eval::RuntimeResult::Complete(v) => v,
//!     _ => panic!("unexpected"),
//! };
```

**Files**: `src/lib.rs`

### 4.2 Update `call_function` doctest (lib.rs lines 165-172)

**Before:**
```rust
/// runtime.eval("export function add...").unwrap();
```

**After:**
```rust
/// match runtime.eval_resumable("export function add...").unwrap() {
///     typescript_eval::RuntimeResult::Complete(_) => {}
///     _ => panic!("unexpected"),
/// };
```

**Files**: `src/lib.rs`

### 4.3 Update `test_basic_arithmetic` unit test (lib.rs lines 239-244)

**Before:**
```rust
let result = runtime.eval("1 + 2 * 3").unwrap();
```

**After:**
```rust
let result = match runtime.eval_resumable("1 + 2 * 3").unwrap() {
    RuntimeResult::Complete(v) => v,
    _ => panic!("Expected Complete"),
};
```

**Files**: `src/lib.rs`

### 4.4 Run lib tests

```bash
cargo test --lib
```

---

## Phase 5: Remove Old API

### 5.1 Remove `Runtime::eval()` method (lib.rs lines 109-114)

Delete:
```rust
/// Evaluate TypeScript source code and return the result
pub fn eval(&mut self, source: &str) -> Result<JsValue, JsError> {
    let mut parser = parser::Parser::new(source);
    let program = parser.parse_program()?;
    self.interpreter.execute(&program)
}
```

**Files**: `src/lib.rs`

### 5.2 Remove `Interpreter::execute()` method (interpreter/mod.rs lines 465-486)

Delete:
```rust
/// Execute a program
pub fn execute(&mut self, program: &Program) -> Result<JsValue, JsError> {
    // Hoist var declarations at global scope
    self.hoist_var_declarations(&program.body);

    let mut result = JsValue::Undefined;

    for stmt in &program.body {
        match self.execute_statement(stmt)? {
            Completion::Normal(val) => result = val,
            Completion::Return(val) => return Ok(val),
            Completion::Break(_) => {
                return Err(JsError::syntax_error("Illegal break statement", 0, 0));
            }
            Completion::Continue(_) => {
                return Err(JsError::syntax_error("Illegal continue statement", 0, 0));
            }
        }
    }

    Ok(result)
}
```

**Files**: `src/interpreter/mod.rs`

### 5.3 Run all tests

```bash
cargo test
```

---

## Phase 6: Rename Methods (Optional)

Consider renaming for cleaner API:

### 6.1 Rename `eval_resumable()` to `eval()`

**Files**: `src/lib.rs`
**Impact**: Update all callers in tests

### 6.2 Rename `execute_resumable()` to `execute()`

**Files**: `src/interpreter/mod.rs`
**Impact**: Update caller in `lib.rs`

### 6.3 Run all tests

```bash
cargo test
```

---

## Phase 7: Update Documentation

### 7.1 Update CLAUDE.md

Update the "Host Loop Pattern" example (around line 285):

**Before:**
```rust
let mut result = runtime.eval(source)?;
```

**After:**
```rust
let mut result = runtime.eval_resumable(source)?;
```

**Files**: `CLAUDE.md`

### 7.2 Update CLAUDE.md Architecture section

Update to reflect single execution model.

**Files**: `CLAUDE.md`

### 7.3 Review state_machine.md

Most examples already use `eval_resumable`. Verify consistency.

**Files**: `state_machine.md`

---

## Phase 8: Clean Up Unused Code (Optional)

After removing `execute()`, check if any code is now unreachable.

### 8.1 Run clippy

```bash
cargo clippy
```

Look for dead_code warnings.

### 8.2 Remove any orphaned helper methods

If `execute_statement()` or `evaluate()` are only called from the stack-based path and could be simplified, consider refactoring. (Likely not needed - these are shared.)

---

## Verification Checklist

After completing all phases:

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (all ~800+ tests)
- [ ] `cargo test --doc` passes (doctests)
- [ ] `cargo clippy` has no new warnings
- [ ] No references to `execute()` or `eval()` (the old simple version) remain
- [ ] `RuntimeResult` is the only return type for program execution

---

## Summary of Changes by File

| File | Changes |
|------|---------|
| `src/lib.rs` | Remove `eval()`, add `eval_simple()`, update doctests |
| `src/interpreter/mod.rs` | Remove `execute()` method |
| `tests/interpreter/main.rs` | Update `eval()`, `eval_result()` helpers to use Runtime |
| `tests/interpreter/exports.rs` | Replace `runtime.eval()` with helper using `eval_resumable()` |
| `CLAUDE.md` | Update examples |
| `state_machine.md` | Verify examples (likely no changes) |

---

## Rollback Plan

If issues arise, each phase can be reverted independently:

1. Git revert the phase commit
2. Run `cargo test` to verify restoration
3. Investigate the issue before re-attempting

---

## Estimated Scope

- **Files modified**: 5-6
- **Lines removed**: ~30 (execute method + eval method)
- **Lines added**: ~40 (new helpers, updated test code)
- **Tests affected**: All tests use the helpers, but test logic unchanged
