# Proper Use of Guards in the GC System

This document describes common guard-related errors and how to fix them. Guards are the primary mechanism for keeping GC-managed objects alive during execution.

## Overview

The garbage collector uses a mark-and-sweep algorithm with guard-based root tracking:

1. **Guard** - A root anchor that keeps objects alive. Objects added to a guard's roots are protected from collection.
2. **Gc<T>** - A reference-counted pointer to a GC-managed object. Cloning increments ref_count, dropping decrements it.
3. **Guarded** - A `JsValue` paired with an optional `Guard` that keeps any contained object alive.

### The Golden Rule

> **Every `JsValue::Object` that needs to survive a potential GC point must be protected by a guard.**

GC can run during ANY allocation. If you have a `JsValue::Object` that isn't in a guard and an allocation triggers GC, your object may be collected.

---

## Common Errors and Fixes

### Error 1: Storing `JsValue` Without a Guard

**Problem:**
```rust
pub struct SavedState {
    pub registers: Vec<JsValue>,  // ← DANGER: Raw JsValues, no guard!
}

fn save_state(&self) -> SavedState {
    SavedState {
        registers: self.registers.clone(),  // Objects can be collected!
    }
}
```

Between saving and restoring state, any allocation could trigger GC and collect objects in `registers`.

**Fix:** Include a guard that protects all objects:

```rust
pub struct SavedState {
    pub registers: Vec<JsValue>,
    pub guard: Guard<JsObject>,  // Keeps objects alive
}

fn save_state(&self, heap: &Heap<JsObject>) -> SavedState {
    let guard = heap.create_guard();
    // Guard all objects BEFORE storing
    for val in &self.registers {
        if let JsValue::Object(obj) = val {
            guard.guard(obj.cheap_clone());
        }
    }
    SavedState {
        registers: self.registers.clone(),
        guard,
    }
}
```

### Error 2: Creating Objects Before Guarding Inputs

**Problem:**
```rust
pub fn create_wrapper(interp: &mut Interpreter, value: JsValue) -> Gc<JsObject> {
    let guard = interp.heap.create_guard();
    let wrapper = interp.create_object(&guard);  // ← GC may run here!
    // If value was the only reference to an object, it may have been collected!
    wrapper.borrow_mut().set_property(PropertyKey::from("value"), value);
    wrapper
}  // ← guard dropped here, wrapper can be collected before caller uses it!
```

Two problems here:
1. When `create_object` allocates, GC may run. If `value` contains an object that has no other references, it gets collected.
2. Creating a guard inside the function and returning the object is an **antipattern** - the guard is dropped when the function returns!

**Fix:** Pass the guard from the caller:

```rust
// CORRECT: Caller controls lifetime via guard parameter
pub fn create_wrapper(
    interp: &mut Interpreter, 
    guard: &Guard<JsObject>,  // ← Caller provides guard
    value: JsValue,
) -> Gc<JsObject> {
    // Guard input FIRST (before any allocation)
    if let JsValue::Object(obj) = &value {
        guard.guard(obj.cheap_clone());
    }
    // NOW safe to allocate
    let wrapper = interp.create_object(guard);
    wrapper.borrow_mut().set_property(PropertyKey::from("value"), value);
    wrapper
}

// Usage:
let guard = interp.heap.create_guard();
let wrapper = create_wrapper(interp, &guard, some_value);
// guard keeps wrapper alive until this scope ends
```

**Alternative:** If you must create a guard inside the function, return `Guarded`:

```rust
pub fn create_wrapper(interp: &mut Interpreter, value: JsValue) -> Guarded {
    let guard = interp.heap.create_guard();
    if let JsValue::Object(obj) = &value {
        guard.guard(obj.cheap_clone());
    }
    let wrapper = interp.create_object(&guard);
    wrapper.borrow_mut().set_property(PropertyKey::from("value"), value);
    Guarded::with_guard(JsValue::Object(wrapper), guard)  // Guard survives!
}
```

### Error 3: Dropping Guard Before Value is Stored

**Problem:**
```rust
fn create_array(interp: &mut Interpreter, elements: Vec<JsValue>) -> JsValue {
    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, elements);
    JsValue::Object(arr)
}  // guard dropped here, arr may be collected before caller uses it!
```

The guard is dropped at the end of the function. If the caller does ANY allocation before storing the result, GC may collect `arr`.

**Fix (preferred):** Accept guard from caller:

```rust
fn create_array(
    interp: &mut Interpreter, 
    guard: &Guard<JsObject>,  // ← Caller controls lifetime
    elements: Vec<JsValue>,
) -> Gc<JsObject> {
    interp.create_array_from(guard, elements)
}

// Usage:
let guard = interp.heap.create_guard();
let arr = create_array(interp, &guard, elements);
// guard keeps arr alive
```

**Fix (alternative):** Return `Guarded` when caller doesn't have a guard:

```rust
fn create_array(interp: &mut Interpreter, elements: Vec<JsValue>) -> Guarded {
    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, elements);
    Guarded {
        value: JsValue::Object(arr),
        guard: Some(guard),  // Guard survives until caller handles it
    }
}
```

**When to use which:**
- Pass `&Guard` when the caller already has a guard (common in loops, multi-object creation)
- Return `Guarded` for public APIs or when caller doesn't manage guards directly

### Error 4: Loop Guards at Wrong Scope

**Problem:**
```rust
let mut results: Vec<Gc<JsObject>> = Vec::new();

for item in items {
    let guard = interp.heap.create_guard();  // ← Guard created in loop
    let obj = interp.create_object(&guard);
    // ... populate obj ...
    results.push(obj);
}  // guard dropped at end of each iteration!

// By this point, objects in `results` may have been collected and reused!
for obj in results {
    // May see corrupted data from object reuse
}
```

**Fix:** Collect guards at outer scope:

```rust
let mut results: Vec<Gc<JsObject>> = Vec::new();
let mut guards: Vec<Guard<JsObject>> = Vec::new();  // ← OUTER scope

for item in items {
    let guard = interp.heap.create_guard();
    let obj = interp.create_object(&guard);
    // ... populate obj ...
    results.push(obj);
    guards.push(guard);  // Keep guard alive
}

// Now safe to use results
for obj in &results {
    // Use obj safely
}

// Guards can be dropped after objects are stored somewhere permanent
drop(guards);
```

**Even Better:** Use a single guard for all objects:

```rust
let guard = interp.heap.create_guard();  // One guard for all
let mut results: Vec<Gc<JsObject>> = Vec::new();

for item in items {
    let obj = interp.create_object(&guard);  // Same guard
    // ... populate obj ...
    results.push(obj);
}

// All objects stay alive until guard is dropped
```

### Error 5: Extracting Value from Guarded and Dropping Guard

**Problem:**
```rust
let result = some_function(interp)?;  // Returns Guarded
let value = result.value;  // Extract JsValue
// result (and its guard) dropped here!

do_something_that_allocates(interp);  // GC may collect value!
use_value(value);  // May be using collected object
```

**Fix:** Keep the Guarded alive, or use destructuring:

```rust
// Option 1: Keep Guarded alive
let result = some_function(interp)?;
do_something_that_allocates(interp);
use_value(&result.value);

// Option 2: Destructure to keep guard in scope
let Guarded { value, guard: _guard } = some_function(interp)?;
// _guard keeps value alive
do_something_that_allocates(interp);
use_value(&value);
```

### Error 6: Saving State Without Guard for Resumption

**Problem (in generators/async):**
```rust
struct GeneratorState {
    saved_registers: Vec<JsValue>,  // ← No guard!
    // ...
}

// On yield
state.saved_registers = vm.registers.clone();
// Generator suspends...
// Later, GC runs and collects objects in saved_registers
// On resume, using corrupted/collected objects!
```

**Fix:** Add a guard field to the saved state:

```rust
struct GeneratorState {
    saved_registers: Vec<JsValue>,
    saved_registers_guard: Option<Guard<JsObject>>,  // ← Keeps them alive!
}

// On yield
let guard = heap.create_guard();
for val in &vm.registers {
    if let JsValue::Object(obj) = val {
        guard.guard(obj.cheap_clone());
    }
}
state.saved_registers = vm.registers.clone();
state.saved_registers_guard = Some(guard);
```

### Error 7: Not Guarding Values When Restoring State

**Problem:**
```rust
fn restore_state(&mut self, state: SavedState, guard: Guard<JsObject>) {
    self.registers = state.registers;  // Objects in registers aren't guarded!
    self.register_guard = guard;  // Empty guard!
    // ...
}
```

The `guard` passed in is freshly created and empty. The objects in `state.registers` are NOT in it.

**Fix:** Guard all values when restoring:

```rust
fn restore_state(&mut self, state: SavedState, guard: Guard<JsObject>) {
    // Guard all existing objects in the saved state
    for val in &state.registers {
        if let JsValue::Object(obj) = val {
            guard.guard(obj.cheap_clone());
        }
    }
    self.registers = state.registers;
    self.register_guard = guard;
}
```

### Error 8: Using `Option<Guard>` and Forgetting to Check

**Problem:**
```rust
struct VM {
    register_guard: Option<Guard<JsObject>>,
}

fn set_reg(&mut self, r: usize, value: JsValue) {
    if let JsValue::Object(obj) = &value {
        self.register_guard.guard(obj.clone());  // ← Won't compile / runtime error
    }
    self.registers[r] = value;
}
```

**Fix:** Always check the Option:

```rust
fn set_reg(&mut self, r: usize, value: JsValue) {
    if let Some(ref guard) = self.register_guard {
        if let JsValue::Object(obj) = &value {
            guard.guard(obj.cheap_clone());
        }
        // Also unguard the old value
        if let Some(slot) = self.registers.get(r) {
            if let JsValue::Object(old_obj) = slot {
                guard.unguard(old_obj);
            }
        }
    }
    if let Some(slot) = self.registers.get_mut(r) {
        *slot = value;
    }
}
```

---

## Guard Lifecycle Patterns

### Pattern 1: Short-lived Computation

Use when creating objects that will be immediately stored elsewhere:

```rust
fn compute_result(interp: &mut Interpreter) -> Guarded {
    let guard = interp.heap.create_guard();
    
    let obj = interp.create_object(&guard);
    obj.borrow_mut().set_property(key, value);
    
    Guarded::with_guard(JsValue::Object(obj), guard)
}
```

### Pattern 2: Long-lived State (Prototypes, Global Objects)

Use the `root_guard` for objects that live for the entire interpreter lifetime:

```rust
// In Interpreter::new()
let root_guard = heap.create_guard();

let object_prototype = root_guard.alloc();
let array_prototype = root_guard.alloc();
// These stay alive as long as the interpreter exists
```

### Pattern 3: Scoped Execution

For function calls or block scopes:

```rust
fn call_function(interp: &mut Interpreter, ...) -> Result<Guarded, JsError> {
    // Create environment with guard
    let (func_env, func_guard) = create_environment_unrooted(&interp.heap, parent);
    
    // Push guard to env_guards stack
    interp.push_env_guard(func_guard);
    
    // Execute function body
    let result = execute_body(interp, ...);
    
    // ALWAYS pop guard, even on error
    interp.pop_env_guard();
    
    result
}
```

### Pattern 4: Collecting Results

When building arrays/objects from computed values:

```rust
fn collect_values(interp: &mut Interpreter, items: &[Expr]) -> Result<Guarded, JsError> {
    let guard = interp.heap.create_guard();
    let mut results = Vec::with_capacity(items.len());
    
    for item in items {
        let Guarded { value, guard: item_guard } = interp.evaluate(item)?;
        
        // Guard the value we're keeping
        if let JsValue::Object(obj) = &value {
            guard.guard(obj.cheap_clone());
        }
        results.push(value);
        
        // item_guard can be dropped now - we've guarded what we need
    }
    
    let arr = interp.create_array_from(&guard, results);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}
```

### Pattern 5: Propagating Guards

When returning a derived value from an object, guard the source object. This ensures:
1. The source object stays alive during property access
2. During GC mark phase, all objects reachable from the guarded object are also marked

```rust
fn get_property(interp: &mut Interpreter, obj: &JsValue, key: &str) 
    -> Result<Guarded, JsError> 
{
    // First, guard the object we're reading from
    let obj_guard = interp.guard_value(obj);
    
    let property_value = match obj {
        JsValue::Object(o) => {
            o.borrow()
                .get_property(key)
                .unwrap_or(JsValue::Undefined)
        }
        _ => return Err(JsError::type_error("Not an object")),
    };
    
    // Propagate the guard - keeps source object alive
    // Property values that are objects have their own Gc ref_count from cloning,
    // but guarding the parent ensures the whole object graph stays consistent
    Ok(Guarded { value: property_value, guard: obj_guard })
}
```

**Note:** When `get_property` returns a `JsValue::Object`, it clones the `Gc<JsObject>`, which increments the ref_count. This means the property object itself won't be collected even without a separate guard. However, propagating the parent's guard is still good practice for consistency and ensures the parent remains accessible if needed.

---

## Symptoms of Guard Bugs

When guards are misused, you may see:

1. **"X is not a function"** - Prototype chain was collected, method lookup fails
2. **Wrong values returned** - Object was collected and slot reused by a different object
3. **Undefined where object expected** - Object reference now points to reset/pooled object
4. **Infinite recursion** - Method A was overwritten by Method B due to slot reuse
5. **Intermittent failures** - GC timing is non-deterministic; works sometimes, fails others

### Debugging Tips

1. **Set `GC_THRESHOLD=1`** to force GC on every allocation (catches bugs faster)
2. **Add `console.log` in tests** to inspect values at different points
3. **Check if issue appears/disappears** when changing GC_THRESHOLD
4. **Trace object IDs** to see if they change unexpectedly

---

## Quick Reference

| Situation | Solution |
|-----------|----------|
| Creating temporary objects | Use local guard, return `Guarded` |
| Storing objects for later | Include guard alongside the stored values |
| Function arguments are objects | Guard inputs before any allocation |
| Returning derived values | Propagate the input's guard |
| Loop creating multiple objects | Use single guard outside loop, or collect guards |
| Async/generator suspension | Store guard in saved state |
| Long-lived objects | Use `root_guard` |
| Restoring saved state | Guard all values when restoring |