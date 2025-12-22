# AST Interpreter Cleanup Plan

This document outlines the plan to remove legacy AST interpreter code from `src/interpreter/mod.rs` and complete the migration to the bytecode VM.

## Background

The interpreter originally used an AST-walking approach where expressions and statements were evaluated directly from the parsed AST. This has been replaced with a register-based bytecode VM for better performance and cleaner architecture.

However, significant amounts of legacy AST evaluation code remain in `src/interpreter/mod.rs`. This code is mostly dead but still has a few active paths that need to be migrated before cleanup.

## Current State

### Legacy `evaluate_*` Methods (Lines ~4010-7200)

These methods form the core of the old AST interpreter:

| Method | Line | Purpose |
|--------|------|---------|
| `evaluate_expression` | 4010 | Main expression dispatcher |
| `evaluate_new` | 4232 | `new` expression handling |
| `evaluate_template_literal` | 4399 | Template literal evaluation |
| `evaluate_tagged_template` | 4420 | Tagged template evaluation |
| `evaluate_literal` | 4471 | Literal value conversion |
| `evaluate_binary` | 4486 | Binary operators |
| `evaluate_unary` | 4836 | Unary operators |
| `evaluate_delete` | 4878 | `delete` operator |
| `evaluate_logical` | 4983 | `&&`, `||`, `??` operators |
| `evaluate_conditional` | 5011 | Ternary `?:` operator |
| `evaluate_assignment` | 5024 | Assignment expressions |
| `evaluate_update` | 5493 | `++`/`--` operators |
| `evaluate_sequence` | 5566 | Comma operator |
| `evaluate_callee_with_this` | 5596 | Method call this-binding |
| `evaluate_call` | 5686 | Function calls |
| `evaluate_member` | 6797 | Property access |
| `evaluate_super_member` | 6939 | `super.x` access |
| `evaluate_array` | 7004 | Array literals |
| `evaluate_object` | 7038 | Object literals |
| `evaluate_decorators` | 3615 | Decorator evaluation |

### Legacy Class Creation Methods (Lines ~2909-3533)

| Method | Line | Purpose |
|--------|------|---------|
| `create_class_constructor` | 2909 | AST-based class creation |
| `create_class_from_expression` | 3516 | Class expression wrapper |

### Pattern Binding Methods (Lines ~2451-2700)

| Method | Line | Purpose |
|--------|------|---------|
| `bind_pattern` | 2451 | Pattern binding (uses `evaluate_expression` for defaults) |
| `bind_array_pattern` | 2574 | Array destructuring |
| `assign_pattern` | 5350 | Pattern assignment (uses `evaluate_expression`) |

### Variable Hoisting (Lines ~7202-7350)

| Method | Line | Purpose |
|--------|------|---------|
| `hoist_var_declarations` | 7202 | Var hoisting |
| `hoist_var_in_statement` | 7208 | Per-statement hoisting |

## Active Paths to Legacy Code

### 1. `JsFunction::Interpreted` Execution

**Location:** `call_function_with_new_target` (line 5983)

When calling an `InterpretedFunction`, the code:
1. Uses `bind_pattern` for parameter binding (line 6121, 6127)
2. `bind_pattern` calls `evaluate_expression` for default parameter values (line 2564)
3. Uses `hoist_var_declarations` for var hoisting (line 6134)

**Creation Sites:**
- `create_interpreted_function` (line 1533) - used by:
  - `create_class_constructor` (lines 3036, 3194, 3344) - AST class creation
  - `evaluate_expression` for Function expressions (lines 4049, 4069)
  - `builtins/function.rs` Function constructor (line 196)

### 2. Direct Eval in AST Path

**Location:** `execute_direct_eval` (line 5917)

Called from `evaluate_call` (line 5708) when detecting direct `eval()` calls. Uses `evaluate_expression` to evaluate the eval argument (line 5921).

Note: The bytecode VM has `Op::DirectEval` which handles this case separately.

## Migration Plan

### Phase 1: Migrate Function Constructor to Bytecode

**Goal:** Make `new Function("code")` create `JsFunction::Bytecode` instead of `JsFunction::Interpreted`.

**Changes:**

1. **Modify `function_constructor_fn` in `builtins/function.rs`:**
   ```rust
   // Instead of:
   let func_obj = interp.create_interpreted_function(...);

   // Do:
   let chunk = Compiler::compile_function_body_direct(&params, &body, ...)?;
   let bc_func = BytecodeFunction {
       chunk: Rc::new(chunk),
       closure: interp.global_env.clone(),
       captured_this: None,
   };
   let func_obj = interp.create_bytecode_function(&guard, bc_func);
   ```

2. **Add helper method `Interpreter::create_bytecode_function_from_ast`** to compile and wrap AST in bytecode function.

**Impact:** Eliminates the primary user of `JsFunction::Interpreted`.

### Phase 2: Remove `JsFunction::Interpreted` Handling from Call Path

**Goal:** Remove the `JsFunction::Interpreted` branch from function calling.

**Changes:**

1. **Simplify `call_function_with_new_target`:**
   - Remove the entire `JsFunction::Interpreted(interp) => { ... }` branch (lines 5983-6289)
   - Keep only bytecode, native, bound, generator, and async generator paths

2. **Update bytecode VM:**
   - Remove `JsFunction::Interpreted` branch (line 767-774)
   - Return error for interpreted functions (should never happen)

### Phase 3: Remove AST Class Creation

**Goal:** Remove `create_class_constructor` and `create_class_from_expression`.

**Verification:**
- Confirm bytecode compiler handles all class features (decorators, computed keys, etc.)
- Run test suite to verify no regressions

**Changes:**

1. **Remove methods:**
   - `create_class_constructor` (lines 2909-3515)
   - `create_class_from_expression` (lines 3516-3534)

2. **Remove decorator AST evaluation:**
   - `evaluate_decorators` (lines 3615-3627)
   - Decorator context creation methods

### Phase 4: Remove AST Expression Evaluation

**Goal:** Remove all `evaluate_*` methods.

**Changes:**

1. **Remove the main dispatcher:**
   - `evaluate_expression` (lines 4010-4230)

2. **Remove all expression evaluators:**
   - `evaluate_new` through `evaluate_object` (~3000 lines)

3. **Remove helper methods:**
   - `evaluate_callee_with_this`
   - `execute_direct_eval` (bytecode uses `Op::DirectEval`)
   - `unwrap_parenthesized` (if only used by AST path)

### Phase 5: Remove Pattern Binding with AST Evaluation

**Goal:** Remove `bind_pattern` and `assign_pattern` that use `evaluate_expression`.

**Analysis:**
- `bind_pattern` is used for parameter binding with default values
- Bytecode VM handles parameter binding via registers and `Op::*` instructions
- Need to verify all call sites are migrated to bytecode

**Changes:**

1. **Remove methods:**
   - `bind_pattern` (lines 2451-2571)
   - `bind_array_pattern` (lines 2574-2610)
   - `assign_pattern` (lines 5350-5490)

### Phase 6: Remove Variable Hoisting

**Goal:** Remove AST-based variable hoisting.

**Verification:**
- Bytecode compiler handles hoisting during compilation (see `compiler/hoist.rs`)

**Changes:**

1. **Remove methods:**
   - `hoist_var_declarations` (lines 7202-7206)
   - `hoist_var_in_statement` (lines 7208-7350)

### Phase 7: Remove InterpretedFunction Type

**Goal:** Remove `JsFunction::Interpreted` variant entirely.

**Changes:**

1. **In `value.rs`:**
   - Remove `InterpretedFunction` struct (lines 2734-2747)
   - Remove `JsFunction::Interpreted` variant (line 2633)
   - Update all pattern matches

2. **In `interpreter/mod.rs`:**
   - Remove `create_interpreted_function` method (lines 1533-1559)

3. **Update all files that reference `JsFunction::Interpreted`:**
   - `src/value.rs` (10+ locations)
   - `src/interpreter/mod.rs` (4 locations)
   - `src/interpreter/bytecode_vm.rs` (2 locations)

## Estimated Code Removal

| Category | Approximate Lines |
|----------|------------------|
| `evaluate_*` methods | ~3,500 |
| Class creation methods | ~600 |
| Pattern binding methods | ~250 |
| Variable hoisting | ~150 |
| `InterpretedFunction` handling | ~200 |
| **Total** | **~4,700 lines** |

## Testing Strategy

1. **Run full test suite after each phase:**
   ```bash
   timeout 120 cargo test
   ```

2. **Run with aggressive GC to catch lifetime bugs:**
   ```bash
   GC_THRESHOLD=1 timeout 120 cargo test
   ```

3. **Specific test files to watch:**
   - `tests/interpreter/function.rs` - Function constructor tests
   - `tests/interpreter/class.rs` - Class tests (if exists)
   - Any tests using decorators

4. **Test262 conformance:**
   ```bash
   ./target/release/test262-runner --strict-only language/expressions
   ./target/release/test262-runner --strict-only language/statements/class
   ```

## Risks and Mitigations

### Risk 1: Missing Bytecode Coverage

Some edge cases might only be handled by AST interpreter.

**Mitigation:** Run extensive tests before removing each method. Check for any test failures that indicate missing bytecode support.

### Risk 2: Function Constructor Semantics

The `Function` constructor has specific scoping rules (always global scope).

**Mitigation:** Carefully review `builtins/function.rs` and ensure bytecode version preserves semantics.

### Risk 3: Default Parameter Values

Default parameters need expression evaluation at call time.

**Mitigation:** Verify bytecode handles default parameters correctly via `Op::LoadDefaultArg` or similar.

## Success Criteria

1. All existing tests pass
2. No `evaluate_*` methods remain in `mod.rs`
3. No `JsFunction::Interpreted` variant exists
4. `mod.rs` reduced by ~4,500 lines
5. Bytecode VM is the sole execution path

## Order of Operations

```
Phase 1: Migrate Function constructor
    └── Verify: cargo test builtins::function

Phase 2: Remove Interpreted from call path
    └── Verify: cargo test

Phase 3: Remove AST class creation
    └── Verify: cargo test class (if decorator tests exist)

Phase 4: Remove evaluate_* methods
    └── Verify: cargo test

Phase 5: Remove pattern binding
    └── Verify: cargo test

Phase 6: Remove variable hoisting
    └── Verify: cargo test

Phase 7: Remove InterpretedFunction type
    └── Verify: cargo test && cargo build
```

## Notes

- Each phase should be a separate commit for easy rollback
- Run `cargo clippy` after each phase to catch dead code warnings
- The bytecode VM (`bytecode_vm.rs`) should not need significant changes
- Focus on `mod.rs` as the primary cleanup target
