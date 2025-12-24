# Guard-Passing Issues: Detailed Refactoring Plan

## Overview

The GC system uses `Guard<JsObject>` to keep objects alive as roots. When `JsValue::Object` values are stored in data structures or passed through error paths without an associated guard, they can be collected prematurely, causing "X is not a function", undefined properties, or other subtle bugs.

This document catalogs all locations where guards need to be added or passed through call chains.

---

## Problem Categories

### 1. JsError Variants Holding Unguarded JsValue

**Files:** `src/error.rs`

The `JsError` enum has two variants that store `JsValue` without a guard:

```rust
// src/error.rs:88-95
ThrownValue { value: crate::value::JsValue },  // Line 90
GeneratorYield { value: crate::value::JsValue },  // Line 95
```

**Risk:** If a GC cycle runs while the error is propagating up the call stack, object values inside these errors can be collected.

**Current Mitigation:** `materialize_thrown_error()` in `src/interpreter/mod.rs:721-750` converts `ThrownValue` to `RuntimeError` with string data before returning from the interpreter. However, within the interpreter loop, these values remain unguarded.

**Solution:**

**Use Guarded wrapper:**
   ```rust
   ThrownValue { guarded: Guarded },
   ```
   - Pro: Consistent with existing patterns
   - Con: Same Clone issue

**Affected Locations:**
- `src/interpreter/bytecode_vm.rs:2760` - `Op::Throw`
- `src/interpreter/bytecode_vm.rs:2813` - `PendingCompletion::Throw` rethrow
- `src/interpreter/bytecode_vm.rs:2837` - `Op::Rethrow`
- `src/interpreter/mod.rs:2106` - Generator exception propagation
- `src/interpreter/builtins/generator.rs:303,309` - Generator throw
- `src/interpreter/builtins/promise.rs:555` - Promise rejection

---

### 2. OpResult Variants Holding Unguarded Values

**File:** `src/interpreter/bytecode_vm.rs:5619-5661`

```rust
enum OpResult {
    // These are fine - use Guarded:
    Halt(Guarded),
    Suspend { promise: Guarded, ... },
    Yield { value: Guarded, ... },
    YieldStar { iterable: Guarded, ... },

    // PROBLEM - unguarded JsValues:
    Call {
        callee: JsValue,      // Line 5642
        this_value: JsValue,  // Line 5643
        args: Vec<JsValue>,   // Line 5644
        new_target: JsValue,  // Line 5646
        ...
    },
    Construct {
        callee: JsValue,      // Line 5653
        this_value: JsValue,  // Line 5654
        args: Vec<JsValue>,   // Line 5655
        new_target: JsValue,  // Line 5657
        new_obj: Gc<JsObject>, // Line 5659 - Gc itself, but caller needs guard
        ...
    },
}
```

**Risk:** Between returning `OpResult::Call/Construct` and the trampoline handling it, a GC cycle could collect the callee, this_value, or args.

**Analysis:** Looking at the code flow:
1. `execute_op` returns `OpResult::Call`
2. Main loop in `run()` matches the result
3. Trampoline pushes frame and continues

The `register_guard` is alive during this entire flow, so values in registers are protected. However, the values are cloned out of registers into `OpResult`, creating new `Gc` pointers without incrementing guard counts.

**Solution:** Add a single `Guard` field to protect all object values:
```rust
Call {
    callee: JsValue,
    this_value: JsValue,
    args: Vec<JsValue>,
    return_register: Register,
    new_target: JsValue,
    is_super_call: bool,
    guard: Guard<JsObject>,  // Single guard protecting all values
},
Construct {
    callee: JsValue,
    this_value: JsValue,
    args: Vec<JsValue>,
    return_register: Register,
    new_target: JsValue,
    new_obj: Gc<JsObject>,
    guard: Guard<JsObject>,  // Single guard protecting all values
},
```

Before returning, guard all object values with the same guard:
```rust
let guard = interp.heap.create_guard();
callee.guard_by(&guard);
this_value.guard_by(&guard);
for arg in &args {
    arg.guard_by(&guard);
}
new_target.guard_by(&guard);
guard.guard(new_obj.cheap_clone());  // For Construct only
Ok(OpResult::Call { callee, this_value, args, ..., guard })
```

---

### 3. PendingCompletion Holding Unguarded Values

**File:** `src/interpreter/bytecode_vm.rs:114-125`

```rust
pub enum PendingCompletion {
    Return(JsValue),  // Line 116 - UNGUARDED
    Throw(JsValue),   // Line 118 - UNGUARDED
    Break { target: usize, try_depth: usize },
    Continue { target: usize, try_depth: usize },
}
```

**Usage:** Stored in `self.pending_completion` while executing finally blocks.

**Risk:** If finally block triggers GC, return/throw values could be collected.

**Current Protection:** The VM's `register_guard` should have these values, but they're cloned from registers.

**Solution:** Use `Guarded` type:
```rust
pub enum PendingCompletion {
    Return(Guarded),
    Throw(Guarded),
    Break { target: usize, try_depth: usize },
    Continue { target: usize, try_depth: usize },
}
```

**Affected Locations:**
- `src/interpreter/bytecode_vm.rs:5509` - `PendingCompletion::Return`
- `src/interpreter/bytecode_vm.rs:2806-2813` - Match on `PendingCompletion`

---

### 4. VM State Fields Holding Unguarded Values

**File:** `src/interpreter/bytecode_vm.rs`

```rust
pub struct BytecodeVm {
    this_value: JsValue,                    // Line 179
    exception_value: Option<JsValue>,       // Line 181 - PROBLEM
    new_target: JsValue,                    // Line 187
    ...
}
```

**Analysis:**
- `this_value` and `new_target` are set at VM creation and typically guarded by caller
- `exception_value` is set dynamically when exceptions are caught

**Current Code (INCORRECT):**
```rust
// Line 1470-1473: When setting exception_value
if let JsValue::Object(obj) = &exc_val {
    self.register_guard.guard(obj.cheap_clone());  // WRONG: register_guard is for registers only!
}
self.exception_value = Some(exc_val);
```

**Problem:** `register_guard` should only be used for register values, not for exception storage.

**Solution:** Change to use `Guarded` type:
```rust
pub struct BytecodeVm {
    exception_value: Option<Guarded>,  // Guarded bundles value + guard
    ...
}
```

**In TrampolineFrame:**
```rust
pub struct TrampolineFrame {
    pub exception_value: Option<Guarded>,
    ...
}
```

**Verdict:** Needs fix - exception_value must use `Guarded`, not bare `JsValue` with register_guard.

---

### 5. GeneratorYield/YieldStar Structs

**File:** `src/interpreter/bytecode_vm.rs:32-48`

```rust
pub struct GeneratorYield {
    pub value: JsValue,           // Line 34 - UNGUARDED
    pub resume_register: Register,
    pub state: BytecodeGeneratorState,
}

pub struct GeneratorYieldStar {
    pub iterable: JsValue,        // Line 44 - UNGUARDED
    pub resume_register: Register,
    pub state: BytecodeGeneratorState,
}
```

**Usage:** Returned from `VmResult::Yield(GeneratorYield)` when generator yields.

**Risk:** Between yielding and resuming, the yielded value could be collected.

**Current Flow:**
1. VM returns `VmResult::Yield(GeneratorYield { value, ... })`
2. Interpreter wraps value in iterator result object
3. Caller receives the wrapped value

**Solution:** Use `Guarded` type:
```rust
pub struct GeneratorYield {
    pub value: Guarded,  // Changed from JsValue
    pub resume_register: Register,
    pub state: BytecodeGeneratorState,
}

pub struct GeneratorYieldStar {
    pub iterable: Guarded,  // Changed from JsValue
    pub resume_register: Register,
    pub state: BytecodeGeneratorState,
}
```

---

### 6. Functions That Should Accept/Pass Guards

These functions create objects or handle values that need guarding:

| Location | Function | Issue |
|----------|----------|-------|
| `src/interpreter/mod.rs:1563` | `create_native_fn` | `name: &str` should be `JsString` |
| `src/interpreter/mod.rs:1759` | `create_native_function` | Should accept guard |
| `src/interpreter/mod.rs:1849` | `register_species_getter` | Should accept Guard |
| `src/interpreter/mod.rs:1870` | `guard_value` | Marked for removal |
| `src/interpreter/mod.rs:2844` | `call_function` | Should receive guard for args |
| `src/interpreter/mod.rs:3236` | Generator creation | Should pass guard |
| `src/interpreter/builtins/generator.rs:332` | `create_bytecode_generator_object` | Should accept guard |
| `src/interpreter/builtins/proxy.rs:991` | `proxy_apply` | Should accept guard |

---

## Implementation Plan

### Phase 1: Guard OpResult Values (High Priority)

**Goal:** Ensure values in `OpResult::Call` and `OpResult::Construct` stay alive.

1. Add `guard: Guard<JsObject>` field to `OpResult::Call`
2. Add `guard: Guard<JsObject>` field to `OpResult::Construct`
3. Before returning, create guard and call `.guard_by(&guard)` on all object values
4. Update all match sites to include the guard field (guard is dropped when variant is consumed)

**Files to modify:**
- `src/interpreter/bytecode_vm.rs`

**Estimated scope:** ~50 lines changed

---

### Phase 2: Guard PendingCompletion Values

**Goal:** Ensure return/throw values survive finally block execution.

1. Change `PendingCompletion::Return(JsValue)` to `Return(Guarded)`
2. Change `PendingCompletion::Throw(JsValue)` to `Throw(Guarded)`
3. Update creation sites to wrap values in `Guarded`
4. Update match sites to extract `.value` from Guarded

**Files to modify:**
- `src/interpreter/bytecode_vm.rs`

**Estimated scope:** ~30 lines changed

---

### Phase 3: Guard Generator Yield Values

**Goal:** Ensure yielded values survive between yield and resume.

1. Change `GeneratorYield::value` from `JsValue` to `Guarded`
2. Change `GeneratorYieldStar::iterable` from `JsValue` to `Guarded`
3. Wrap values in `Guarded` when constructing these structs
4. Update consumers to extract `.value` from Guarded

**Files to modify:**
- `src/interpreter/bytecode_vm.rs`
- `src/interpreter/mod.rs` (generator handling)

**Estimated scope:** ~40 lines changed

---

### Phase 4: Fix VM exception_value Guarding

**Goal:** Replace incorrect `register_guard` usage for exception_value with `Guarded` type.

**Changes to `BytecodeVm`:**
```rust
pub struct BytecodeVm {
    exception_value: Option<Guarded>,  // Changed from Option<JsValue>
    ...
}
```

**Changes to `TrampolineFrame`:**
```rust
pub struct TrampolineFrame {
    pub exception_value: Option<Guarded>,  // Changed from Option<JsValue>
    ...
}
```

**Locations to update:**
1. `BytecodeVm` struct definition - change `exception_value` type
2. `TrampolineFrame` struct definition - change `exception_value` type
3. All places that set `exception_value` - create `Guarded` instead of using register_guard
4. All places that read `exception_value` - extract `.value` from Guarded
5. All places that save/restore trampoline frames - already handled by type change

**Files to modify:**
- `src/interpreter/bytecode_vm.rs`

**Estimated scope:** ~40 lines changed

---

### Phase 5: Use Guarded in JsError Variants

**Goal:** Change `JsError::ThrownValue` and `JsError::GeneratorYield` to use `Guarded`.

**Changes to `src/error.rs`:**
```rust
// Before:
ThrownValue { value: JsValue },
GeneratorYield { value: JsValue },

// After:
ThrownValue { guarded: Guarded },
GeneratorYield { guarded: Guarded },
```

**Locations to update:**
1. `src/error.rs` - Change variant definitions
2. `src/interpreter/bytecode_vm.rs:2760` - `Op::Throw` - create Guarded
3. `src/interpreter/bytecode_vm.rs:2837` - `Op::Rethrow` - create Guarded
4. `src/interpreter/mod.rs:721-750` - `materialize_thrown_error` - extract from Guarded
5. `src/interpreter/mod.rs:2106` - Generator exception propagation
6. `src/interpreter/builtins/generator.rs:303,309` - Generator throw
7. `src/interpreter/builtins/promise.rs:555` - Promise rejection
8. All match sites on these error variants

**Files to modify:**
- `src/error.rs`
- `src/interpreter/bytecode_vm.rs`
- `src/interpreter/mod.rs`
- `src/interpreter/builtins/generator.rs`
- `src/interpreter/builtins/promise.rs`
- `src/bin/test262-runner.rs` (error matching)

**Note:** This makes `JsError` non-Clone. Need to audit all `.clone()` calls on JsError.

**Estimated scope:** ~60 lines changed

---

### Phase 6: Function Signature Updates

**Goal:** Make guard passing explicit in key APIs.

1. `create_bytecode_generator_object`: Accept `&Guard<JsObject>`
2. `proxy_apply`: Accept guard parameter
3. Remove deprecated `guard_value` function
4. Consider updating `call_function` signature

**Files to modify:**
- `src/interpreter/mod.rs`
- `src/interpreter/builtins/generator.rs`
- `src/interpreter/builtins/proxy.rs`
- All callers of these functions

**Estimated scope:** ~100 lines changed (many call sites)

---

## Testing Strategy

### Existing Tests
Run with aggressive GC settings:
```bash
GC_THRESHOLD=1 timeout 60 cargo test
```

### New Tests to Add
1. Test throwing object, triggering GC, catching it
2. Test generator yield with object, trigger GC, resume
3. Test finally block with return object, trigger GC, complete return
4. Test deep call chain with object args, trigger GC mid-call

### Manual Verification
After each phase, run the test262 suite:
```bash
./target/release/test262-runner --strict-only language/statements
```

---

## Risk Assessment

| Phase | Risk Level | Justification |
|-------|------------|---------------|
| 1 | Low | Contained to bytecode_vm.rs, clear pattern |
| 2 | Low | Same file, similar pattern |
| 3 | Low | Clear ownership, localized changes |
| 4 | Low | Same file, well-defined scope |
| 5 | Medium | Multiple files, makes JsError non-Clone |
| 6 | Medium-High | Many call sites, API changes |

---

## Dependencies

- Phases 1-4 are independent and can be done in any order
- Phase 5 (JsError changes) is independent but has broader impact
- Phase 6 depends on understanding patterns from earlier phases
- All phases should pass existing tests before proceeding

### Suggested Order

1. **Phase 4** (exception_value) - fixes existing incorrect register_guard usage
2. **Phases 1-3** (OpResult, PendingCompletion, GeneratorYield) - similar patterns
3. **Phase 5** (JsError) - higher impact, needs careful Clone audit
4. **Phase 6** (Function signatures) - can be done incrementally

---

## Notes

### Critical Constraint: register_guard is ONLY for Registers

`BytecodeVm::register_guard` must only be used to guard values stored in VM registers. It should NOT be used for:
- Exception values
- Values in `OpResult` variants
- Values in `PendingCompletion`
- Thrown values in errors
- Any other non-register storage

Each value that escapes registers or needs to survive across operations must have its **own dedicated guard**.

### JsError with Guarded

Using `Guarded` in JsError variants:
```rust
ThrownValue { guarded: Guarded },
```

This makes `JsError` non-Clone because `Guard` is not Clone. This is acceptable because:
1. Errors typically propagate up without cloning
2. If cloning is needed, extract the value first
3. The `materialize_thrown_error` function already converts to string-based errors at boundaries

### Guard Ownership Pattern

Use `Guarded::from_value` to create a guarded value - it creates its own guard from the heap:
```rust
let guarded = Guarded::from_value(value, &interp.heap);
```

This is the preferred method for returning values from the VM.

Example for exception handling:
```rust
self.exception_value = Some(Guarded::from_value(exc_val, &interp.heap));
```

Available methods:
- `Guarded::from_value(value, &heap)` - create Guarded with new guard from heap (preferred)
- `Guarded::with_guard(value, guard)` - create Guarded with existing guard (caller must guard objects first)
- `Guarded::unguarded(value)` - create Guarded with no guard (for primitives)
- `JsValue::guard_by(&guard)` - register object with existing guard (no-op for primitives)
