# Guard System Refactoring Plan

## Overview

This document describes the guard-related issues found in the codebase and the plan to fix them. The guard system keeps GC-managed objects alive during operations that might trigger garbage collection.

## Background: How Guards Work

```rust
pub struct Guarded {
    pub value: JsValue,
    pub guard: Option<Guard<JsObject>>,
}
```

- `Guard<JsObject>` anchors objects to roots, preventing GC collection
- Guards must be held until ownership is transferred (stored in register, property, etc.)
- Creating a guard BEFORE allocation is critical - GC runs at allocation time

### Guard Types

| Guard | Purpose | Lifetime |
|-------|---------|----------|
| `root_guard` | Permanent objects (builtins, prototypes) | Forever |
| `register_guard` | Values in VM registers | Per call frame |
| Temporary guards | Short-lived allocations in functions | Function scope |

**Important:** `register_guard` is internal to the VM and should NOT be passed to other functions. Functions that need a guard should either:
1. Accept a guard parameter (caller creates temporary guard)
2. Create their own temporary guard via `heap.create_guard()`

## Issues Found

### Issue 1: `to_object` Creates Dropped Guards (BUG)

**Location:** `src/interpreter/mod.rs:2797-2845`

**Problem:** The `to_object` function creates wrapper objects (Boolean, Number, String, Symbol) with local guards that are immediately dropped when the function returns.

```rust
pub fn to_object(&mut self, value: JsValue) -> Result<Gc<JsObject>, JsError> {
    match value {
        JsValue::Boolean(b) => {
            let guard = self.heap.create_guard();  // Guard created
            let gc_obj = guard.alloc();
            // ... setup object ...
            Ok(gc_obj)  // Guard DROPPED here - object vulnerable to GC!
        }
        // Same pattern for Number, String, Symbol
    }
}
```

**Impact:** If any caller of `to_object` triggers an allocation before storing the returned object, GC could collect it.

**Callers at risk:**
- `src/interpreter/builtins/object.rs:175` - `object_keys`
- `src/interpreter/builtins/object.rs:792` - `object_freeze`
- `src/interpreter/builtins/object.rs:861` - `object_seal`
- `src/interpreter/builtins/object.rs:886` - `object_is_frozen`
- `src/interpreter/builtins/object.rs:1271` - `object_get_own_property_descriptors`

**Fix:** Change signature to accept a guard parameter:

```rust
pub fn to_object(&mut self, guard: &Guard<JsObject>, value: JsValue) -> Result<Gc<JsObject>, JsError>
```

### Issue 2: `get_property_value` Drops Getter Result Guard (POTENTIAL BUG)

**Location:** `src/interpreter/bytecode_vm.rs:5344-5454`

**Problem:** When a property has a getter, `get_property_value` calls the getter and discards the guard:

```rust
fn get_property_value(&self, interp: &mut Interpreter, obj: &JsValue, key: &JsValue) -> Result<JsValue, JsError> {
    // ...
    if let Some(getter) = prop.getter() {
        let result = interp.call_function(...)?;
        Ok(result.value)  // Guard dropped! Value could be GC'd before stored in register
    }
}
```

**Impact:** If a getter returns a newly allocated object and subsequent code allocates before the value is stored, the object could be collected. In practice, callers immediately store in a register:

```rust
let result = self.get_property_value(interp, obj_val, key_val)?;
self.set_reg(dst, result);  // Safe IF no allocation between these lines
```

**Callers:**
- `src/interpreter/bytecode_vm.rs:2200` - `Op::GetProperty`
- `src/interpreter/bytecode_vm.rs:2211` - `Op::GetPropertyConst`
- `src/interpreter/bytecode_vm.rs:2496` - `Op::CallMethod`
- `src/interpreter/bytecode_vm.rs:4005` - `Op::GetSuperProperty`
- `src/interpreter/bytecode_vm.rs:4016` - `Op::GetSuperPropertyConst`

**Fix:** Return `Guarded` instead of `JsValue`:

```rust
fn get_property_value(&self, interp: &mut Interpreter, obj: &JsValue, key: &JsValue) -> Result<Guarded, JsError>
```

Update callers to hold the guard until value is stored in register.

### Issue 4: `create_native_function` Should Accept Guard

**Location:** `src/interpreter/mod.rs:1760-1785`

**Current State:** Uses `root_guard` internally, hiding this dependency.

**Problem:** The function accesses `self.root_guard` directly. For consistency and explicitness, callers should pass the guard explicitly.

**Fix:** Change signature to accept guard:

```rust
pub fn create_native_function(
    &mut self,
    guard: &Guard<JsObject>,
    name: &str,
    func: NativeFn,
    arity: usize,
) -> Gc<JsObject>
```

Callers (all during initialization) will pass `&self.root_guard`.

### Issue 5: `register_species_getter` Should Accept Guard

**Location:** `src/interpreter/mod.rs:1850-1867`

**Current State:** Uses `root_guard` internally.

**Problem:** Same as Issue 4 - hidden dependency on `root_guard`.

**Fix:** Change signature to accept guard:

```rust
pub fn register_species_getter(&mut self, guard: &Guard<JsObject>, constructor: &Gc<JsObject>)
```

Callers will pass `&interp.root_guard`.

## Detailed Fix Plan

### Phase 1: Fix `to_object` (Critical) ✅ COMPLETED

**Files modified:**
- `src/interpreter/mod.rs`
- `src/interpreter/builtins/object.rs`
- `src/value.rs` (added `ExoticObject::Symbol` variant)
- `src/interpreter/builtins/global.rs` (updated structuredClone for Symbol)
- `src/interpreter/builtins/json.rs` (updated JSON serialization for Symbol)

**Changes made:**

1. Changed `to_object` signature to return `Guarded` instead of `Gc<JsObject>`
2. Added `ExoticObject::Symbol(Box<JsSymbol>)` variant to properly wrap Symbol values
3. Updated all callers to extract the object from `Guarded.value` while keeping the guard alive:
   - `object_keys`
   - `object_get_own_property_descriptor`
   - `object_get_own_property_names`
   - `object_get_own_property_symbols`
   - `object_get_own_property_descriptors`
4. Updated exhaustive matches for the new `ExoticObject::Symbol` variant

### Phase 3: Fix `get_property_value` ✅ COMPLETED

**Files modified:**
- `src/interpreter/bytecode_vm.rs`

**Changes made:**

1. Changed `get_property_value` to return `Guarded` instead of `JsValue`
2. For proxy_get and getter calls, the Guarded is now properly propagated
3. For primitive value lookups, returns `Guarded::unguarded(value)`
4. Updated all callers:
   - `Op::GetProperty`
   - `Op::GetPropertyConst`
   - `Op::CallMethod` (with proper guard transfer)
   - `Op::SuperGet`
   - `Op::SuperGetConst`

### Phase 4 & 5: `create_native_function`, `register_method`, `register_species_getter` ⏭️ SKIPPED

**Decision:** These functions were NOT changed to accept explicit guard parameters.

**Rationale:**
1. All ~200 call sites are during initialization and correctly use `root_guard`
2. Changing the signatures would require updating every call site
3. Borrow checker issues: `interp.method(&interp.root_guard, ...)` creates conflicting borrows
4. This was a design improvement for explicitness, not a bug fix

**Current implementation:** These functions use `self.root_guard` internally, which is correct
for builtin initialization. Functions are permanently rooted and never collected.

**Documentation updated:** Added doc comments explaining that `root_guard` is used internally.

### Phase 6: Add Tests

**Files to modify:**
- `tests/interpreter/gc.rs` (create if needed)

**Tests to add:**

1. Test `to_object` with primitives followed by allocations under `GC_THRESHOLD=1`
2. Test getters returning new objects with immediate subsequent allocations
3. Test `call_function` with object arguments under aggressive GC

## Testing Strategy

1. Run existing test suite with aggressive GC (`GC_THRESHOLD=1`)
2. Add specific tests for `to_object` with primitives followed by allocations
3. Add tests for getters that return new objects followed by operations
4. Add tests for `call_function` with object arguments under aggressive GC

## Order of Implementation

1. **Phase 1** - `to_object` ✅ COMPLETED
2. **Phase 2** - `call_function` (already returns Guarded, no change needed)
3. **Phase 3** - `get_property_value` ✅ COMPLETED
4. **Phase 4** - `create_native_function` ⏭️ SKIPPED (uses root_guard correctly)
5. **Phase 5** - `register_species_getter` ⏭️ SKIPPED (uses root_guard correctly)
6. **Phase 6** - Add tests (pending)

## Files Changed Summary

| File | Changes |
|------|---------|
| `src/interpreter/mod.rs` | `to_object`, `call_function`, `call_function_with_new_target`, `create_native_function`, `register_species_getter`, `register_method` |
| `src/interpreter/bytecode_vm.rs` | `get_property_value`, all `call_function` call sites |
| `src/interpreter/builtins/array.rs` | `create_native_function`, `register_species_getter`, `call_function` calls |
| `src/interpreter/builtins/object.rs` | `to_object` callers, `create_native_function` |
| `src/interpreter/builtins/string.rs` | `create_native_function` |
| `src/interpreter/builtins/number.rs` | `create_native_function` |
| `src/interpreter/builtins/boolean.rs` | `create_native_function` |
| `src/interpreter/builtins/function.rs` | `create_native_function` |
| `src/interpreter/builtins/date.rs` | `create_native_function` |
| `src/interpreter/builtins/regexp.rs` | `create_native_function`, `register_species_getter` |
| `src/interpreter/builtins/map.rs` | `create_native_function`, `register_species_getter`, `call_function` calls |
| `src/interpreter/builtins/set.rs` | `create_native_function`, `register_species_getter`, `call_function` calls |
| `src/interpreter/builtins/promise.rs` | `create_native_function`, `register_species_getter` |
| `src/interpreter/builtins/symbol.rs` | `create_native_function` |
| `src/interpreter/builtins/error.rs` | `create_native_function` |
| `src/interpreter/builtins/global.rs` | `create_native_function` |
| `src/interpreter/builtins/proxy.rs` | `create_native_function`, `call_function` calls |
| `src/interpreter/builtins/generator.rs` | `create_native_function` |
| `tests/interpreter/gc.rs` | New test file |

## Estimated Impact

- Large refactor touching most builtin files
- 77 call sites to update for `call_function` (11 files)
- 43 call sites to update for `create_native_function` (17 files)
- 5 call sites for `register_species_getter`
- 5 call sites for `to_object`
- 5 call sites for `get_property_value`

Total: ~135 call sites across ~20 files
