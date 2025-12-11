# GC Integration Plan for typescript-eval

This document details the plan for integrating the cycle-breaking garbage collector from `src/gc.rs` into the TypeScript interpreter.

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Current Memory Model](#current-memory-model)
3. [Reference Cycle Analysis](#reference-cycle-analysis)
4. [Integration Strategy](#integration-strategy)
5. [Phase 1: Core Object GC](#phase-1-core-object-gc)
6. [Phase 2: Promise and Generator Integration](#phase-2-promise-and-generator-integration)
7. [API Changes](#api-changes)
8. [Migration Guide](#migration-guide)
9. [Testing Strategy](#testing-strategy)
10. [Performance Considerations](#performance-considerations)
11. [Appendix: File Change Summary](#appendix-file-change-summary)

---

## Executive Summary

### Goal

Integrate the mark-and-sweep garbage collector (`src/gc.rs`) to automatically detect and break reference cycles in JavaScript objects, preventing memory leaks in long-running programs.

### Current State

| Component | Implementation | Cycle Risk | GC Need |
|-----------|---------------|------------|---------|
| Environments | Arena-based (`EnvId`) | None | No |
| JS Objects | `Rc<RefCell<JsObject>>` | High | **Yes** |
| Promises | `Rc<RefCell<PromiseState>>` | High | **Yes** |
| Generators | `Rc<RefCell<GeneratorState>>` | Medium | Optional |

### Approach

Two-phase integration:
1. **Phase 1**: GC for `JsObject` (highest impact, most cycles)
2. **Phase 2**: Extend to Promise/Generator state (completeness)

### Estimated Effort

- Phase 1: ~800-1000 lines changed across 20+ files
- Phase 2: ~300-400 additional lines
- Testing: ~200-300 lines of new tests

---

## Current Memory Model

### Object References

All JavaScript objects use reference-counted pointers:

```rust
// src/value.rs:744
pub type JsObjectRef = Rc<RefCell<JsObject>>;
```

Objects are created via helper functions:

```rust
pub fn create_object() -> JsObjectRef {
    Rc::new(RefCell::new(JsObject::new()))
}

pub fn create_array(elements: Vec<JsValue>) -> JsObjectRef {
    // ... builds array object
    Rc::new(RefCell::new(obj))
}

pub fn create_function(func: JsFunction) -> JsObjectRef {
    // ... builds function object
    Rc::new(RefCell::new(obj))
}
```

### Environment Management (Already Solved)

The codebase uses an arena-based approach for environments to prevent closure cycles:

```rust
// src/value.rs:15-37
pub struct EnvId(pub usize);  // Index, not Rc

pub struct EnvironmentArena {
    envs: Vec<Environment>,
    free_list: Vec<usize>,
}
```

This approach successfully prevents closure → environment → closure cycles.

### Object Structure

```rust
pub struct JsObject {
    pub prototype: Option<JsObjectRef>,           // Can form cycles
    pub extensible: bool,
    pub frozen: bool,
    pub sealed: bool,
    pub null_prototype: bool,
    pub properties: FxHashMap<PropertyKey, Property>,  // Contains JsValues
    pub exotic: ExoticObject,                     // May contain Rc types
}

pub struct Property {
    pub value: JsValue,                           // Can be Object(JsObjectRef)
    pub writable: bool,
    pub enumerable: bool,
    pub configurable: bool,
    pub getter: Option<JsObjectRef>,              // Can form cycles
    pub setter: Option<JsObjectRef>,              // Can form cycles
}
```

---

## Reference Cycle Analysis

### Cycle Type 1: Object Property Cycles (Most Common)

```javascript
// JavaScript code that creates cycles
const a = { name: 'a' };
const b = { name: 'b', ref: a };
a.ref = b;  // Creates cycle: a -> b -> a
```

**In Rust:**
```
JsObject(a).properties["ref"] = JsValue::Object(Rc to b)
JsObject(b).properties["ref"] = JsValue::Object(Rc to a)
// Reference count never reaches 0
```

**Frequency**: Very common in real JavaScript code (parent/child relationships, graphs, caches)

### Cycle Type 2: Promise Handler Cycles

```javascript
const p1 = new Promise(resolve => { /* ... */ });
const p2 = p1.then(value => value);
// p2's handler references p1, p1's handlers include p2
```

**In Rust:**
```rust
pub struct PromiseHandler {
    pub on_fulfilled: Option<JsValue>,    // Function referencing closure
    pub on_rejected: Option<JsValue>,     // Function referencing closure
    pub result_promise: JsObjectRef,      // Direct Rc to another promise
}
```

**Cycle formation:**
```
Promise A → PromiseState → handlers → PromiseHandler
    ↓                                        ↓
    └─────────────────────────────── result_promise → Promise B
                                                          ↓
Promise B → PromiseState → handlers → PromiseHandler → Promise A
```

**Frequency**: Common in async code with promise chains

### Cycle Type 3: Prototype Cycles (Rare)

```javascript
// Unusual but possible
const proto1 = {};
const proto2 = Object.create(proto1);
Object.setPrototypeOf(proto1, proto2);  // Circular prototype chain
```

**Frequency**: Rare, usually a bug

### Cycle Type 4: Accessor Property Cycles

```javascript
const obj = {};
Object.defineProperty(obj, 'self', {
    get() { return this; },  // Getter closure captures obj
});
```

**In Rust:**
```rust
Property {
    getter: Some(JsObjectRef),  // Function with closure referencing obj
    // ...
}
```

**Frequency**: Moderate in complex objects

### Cycle Type 5: Map/Set Value Cycles

```javascript
const map = new Map();
map.set('self', map);  // Map contains reference to itself
```

**In Rust:**
```rust
ExoticObject::Map { entries: Vec<(JsValue, JsValue)> }
// entries can contain Object(Rc) pointing back to the map
```

**Frequency**: Occasional

---

## Integration Strategy

### Why the GC in gc.rs Fits

The `gc.rs` implementation provides:

1. **`Gc<T>` smart pointer**: Drop-in replacement for `Rc<RefCell<T>>`
2. **`Space<T>` manager**: Tracks all allocated objects
3. **`Traceable` trait**: Defines how to traverse object graphs
4. **Mark-and-sweep collection**: Identifies unreachable cycles
5. **Cycle breaking via `unlink()`**: Clears references in dead objects

### Integration Approach

```
Before:
┌─────────────────────────────────────────────────────────┐
│ Interpreter                                             │
│   global: Rc<RefCell<JsObject>>                        │
│   object_prototype: Rc<RefCell<JsObject>>              │
│   ...                                                   │
└─────────────────────────────────────────────────────────┘

After:
┌─────────────────────────────────────────────────────────┐
│ Interpreter                                             │
│   gc_space: Space<GcJsObject>     ←── All objects here │
│   global: Gc<GcJsObject>          ←── Rooted           │
│   object_prototype: Gc<GcJsObject> ←── Rooted          │
│   ...                                                   │
└─────────────────────────────────────────────────────────┘
```

---

## Phase 1: Core Object GC

### Step 1.1: Add GC Module to Library

**File: `src/lib.rs`**

```rust
// Add to existing module declarations
pub mod gc;
pub use gc::{Gc, Space, Traceable, Tracer, GcBox};
```

### Step 1.2: Create GC-Aware Object Wrapper

**File: `src/value.rs`**

Create a new wrapper type that implements `Traceable`:

```rust
use crate::gc::{Gc, Space, Traceable, Tracer};

/// GC-managed JavaScript object wrapper
pub struct GcJsObject {
    inner: RefCell<JsObject>,
}

impl GcJsObject {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(JsObject::new()),
        }
    }

    pub fn with_object(obj: JsObject) -> Self {
        Self {
            inner: RefCell::new(obj),
        }
    }

    pub fn borrow(&self) -> std::cell::Ref<'_, JsObject> {
        self.inner.borrow()
    }

    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, JsObject> {
        self.inner.borrow_mut()
    }

    pub fn try_borrow(&self) -> Result<std::cell::Ref<'_, JsObject>, std::cell::BorrowError> {
        self.inner.try_borrow()
    }

    pub fn try_borrow_mut(&self) -> Result<std::cell::RefMut<'_, JsObject>, std::cell::BorrowMutError> {
        self.inner.try_borrow_mut()
    }
}

// Update the type alias
pub type JsObjectRef = Gc<GcJsObject>;
```

### Step 1.3: Implement Traceable for GcJsObject

**File: `src/value.rs`**

```rust
impl Traceable for GcJsObject {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        // Try to borrow - if we can't, object is being modified elsewhere
        let Ok(obj) = self.inner.try_borrow() else {
            return;
        };

        // Trace prototype reference
        if let Some(ref proto) = obj.prototype {
            tracer.trace(proto);
        }

        // Trace all property values
        for prop in obj.properties.values() {
            Self::trace_property(prop, tracer);
        }

        // Trace exotic object data
        Self::trace_exotic(&obj.exotic, tracer);
    }

    fn unlink(&mut self) {
        let Ok(mut obj) = self.inner.try_borrow_mut() else {
            return;
        };

        // Clear prototype
        obj.prototype = None;

        // Clear all properties
        obj.properties.clear();

        // Clear exotic data
        obj.exotic = ExoticObject::Ordinary;
    }
}

impl GcJsObject {
    fn trace_property(prop: &Property, tracer: &mut Tracer<'_>) {
        Self::trace_jsvalue(&prop.value, tracer);

        if let Some(ref getter) = prop.getter {
            tracer.trace(getter);
        }
        if let Some(ref setter) = prop.setter {
            tracer.trace(setter);
        }
    }

    fn trace_jsvalue(value: &JsValue, tracer: &mut Tracer<'_>) {
        if let JsValue::Object(ref obj_ref) = value {
            tracer.trace(obj_ref);
        }
    }

    fn trace_exotic(exotic: &ExoticObject, tracer: &mut Tracer<'_>) {
        match exotic {
            ExoticObject::Ordinary => {}

            ExoticObject::Array { .. } => {
                // Array elements are stored as properties, already traced
            }

            ExoticObject::Function(func) => {
                Self::trace_function(func, tracer);
            }

            ExoticObject::Map { entries } => {
                for (key, value) in entries {
                    Self::trace_jsvalue(key, tracer);
                    Self::trace_jsvalue(value, tracer);
                }
            }

            ExoticObject::Set { entries } => {
                for value in entries {
                    Self::trace_jsvalue(value, tracer);
                }
            }

            ExoticObject::Date { .. } => {}

            ExoticObject::RegExp { .. } => {}

            ExoticObject::Generator(state) => {
                let state = state.borrow();
                for arg in &state.args {
                    Self::trace_jsvalue(arg, tracer);
                }
                Self::trace_jsvalue(&state.sent_value, tracer);
            }

            ExoticObject::Promise(state) => {
                let state = state.borrow();
                if let Some(ref result) = state.result {
                    Self::trace_jsvalue(result, tracer);
                }
                for handler in &state.handlers {
                    if let Some(ref on_fulfilled) = handler.on_fulfilled {
                        Self::trace_jsvalue(on_fulfilled, tracer);
                    }
                    if let Some(ref on_rejected) = handler.on_rejected {
                        Self::trace_jsvalue(on_rejected, tracer);
                    }
                    tracer.trace(&handler.result_promise);
                }
            }
        }
    }

    fn trace_function(func: &JsFunction, tracer: &mut Tracer<'_>) {
        match func {
            JsFunction::Interpreted(_) => {
                // Closure environment is tracked via EnvId (arena), not Gc
            }
            JsFunction::Native(_) => {}
            JsFunction::Bound(bound) => {
                tracer.trace(&bound.target);
                Self::trace_jsvalue(&bound.this_arg, tracer);
                for arg in &bound.bound_args {
                    Self::trace_jsvalue(arg, tracer);
                }
            }
            JsFunction::PromiseResolve(promise) => {
                tracer.trace(promise);
            }
            JsFunction::PromiseReject(promise) => {
                tracer.trace(promise);
            }
        }
    }
}
```

### Step 1.4: Add Space to Interpreter

**File: `src/interpreter/mod.rs`**

```rust
use crate::gc::Space;
use crate::value::GcJsObject;

pub struct Interpreter {
    /// GC space managing all JavaScript objects
    pub gc_space: Space<GcJsObject>,

    /// Global object (rooted)
    pub global: JsObjectRef,

    /// Arena storing all environments (avoids Rc cycles)
    pub env_arena: EnvironmentArena,

    /// Current environment ID
    pub env: EnvId,

    /// Object.prototype for all objects (rooted)
    pub object_prototype: JsObjectRef,

    /// Array.prototype for all array instances (rooted)
    pub array_prototype: JsObjectRef,

    // ... rest of existing fields unchanged
}
```

### Step 1.5: Update Interpreter::new()

**File: `src/interpreter/mod.rs`**

```rust
impl Interpreter {
    pub fn new() -> Self {
        let mut gc_space = Space::with_capacity(4096);  // Reasonable initial capacity
        let mut env_arena = EnvironmentArena::new();
        let env = env_arena.global_id();

        // Create global object and root it
        let global = gc_space.alloc(GcJsObject::new());
        gc_space.add_root(&global);

        // Add basic global values to environment
        env_arena.define(env, "undefined".to_string(), JsValue::Undefined, false);
        env_arena.define(env, "NaN".to_string(), JsValue::Number(f64::NAN), false);
        env_arena.define(env, "Infinity".to_string(), JsValue::Number(f64::INFINITY), false);

        // Create prototypes - need to pass gc_space to builtin creators
        let object_prototype = create_object_prototype_gc(&mut gc_space);
        gc_space.add_root(&object_prototype);

        let array_prototype = create_array_prototype_gc(&mut gc_space);
        gc_space.add_root(&array_prototype);

        let string_prototype = create_string_prototype_gc(&mut gc_space);
        gc_space.add_root(&string_prototype);

        // ... continue for all prototypes

        // Set up prototype chains
        {
            let mut arr_proto = array_prototype.borrow_mut();
            arr_proto.prototype = Some(object_prototype.cheap_clone());
        }

        // ... continue prototype chain setup

        Self {
            gc_space,
            global,
            env_arena,
            env,
            object_prototype,
            array_prototype,
            // ... rest of fields
        }
    }
}
```

### Step 1.6: Update Object Creation Helpers

**File: `src/value.rs`**

Replace standalone functions with methods that take `&mut Space`:

```rust
// Old API (to be removed)
pub fn create_object() -> JsObjectRef {
    Rc::new(RefCell::new(JsObject::new()))
}

// New API
pub fn create_object_gc(space: &mut Space<GcJsObject>) -> JsObjectRef {
    space.alloc(GcJsObject::new())
}

pub fn create_object_with_prototype_gc(
    space: &mut Space<GcJsObject>,
    prototype: JsObjectRef,
) -> JsObjectRef {
    space.alloc(GcJsObject::with_object(JsObject::with_prototype(prototype)))
}

pub fn create_array_gc(
    space: &mut Space<GcJsObject>,
    elements: Vec<JsValue>,
) -> JsObjectRef {
    let length = elements.len() as u32;
    let mut obj = JsObject::new();
    obj.exotic = ExoticObject::Array { length };

    for (i, elem) in elements.into_iter().enumerate() {
        obj.set_property(PropertyKey::Index(i as u32), elem);
    }
    obj.properties.insert(
        PropertyKey::String(JsString::from("length")),
        Property::data_with_attrs(JsValue::Number(length as f64), true, false, false),
    );

    space.alloc(GcJsObject::with_object(obj))
}

pub fn create_function_gc(
    space: &mut Space<GcJsObject>,
    func: JsFunction,
) -> JsObjectRef {
    let name = func.name().map(|s| s.to_string());
    let mut obj = JsObject::new();
    obj.exotic = ExoticObject::Function(func);

    if let Some(name) = name {
        obj.set_property(
            PropertyKey::String(JsString::from("name")),
            JsValue::String(JsString::from(name)),
        );
    }

    space.alloc(GcJsObject::with_object(obj))
}
```

### Step 1.7: Update Interpreter Methods

**File: `src/interpreter/mod.rs`**

```rust
impl Interpreter {
    /// Create an array with elements
    pub fn create_array(&mut self, elements: Vec<JsValue>) -> JsObjectRef {
        let arr = create_array_gc(&mut self.gc_space, elements);
        arr.borrow_mut().prototype = Some(self.array_prototype.cheap_clone());
        arr
    }

    /// Create a new object with Object.prototype
    pub fn create_plain_object(&mut self) -> JsObjectRef {
        let obj = create_object_gc(&mut self.gc_space);
        obj.borrow_mut().prototype = Some(self.object_prototype.cheap_clone());
        obj
    }

    /// Create a function object
    pub fn create_function_object(&mut self, func: JsFunction) -> JsObjectRef {
        let obj = create_function_gc(&mut self.gc_space, func);
        obj.borrow_mut().prototype = Some(self.function_prototype.cheap_clone());
        obj
    }

    /// Run garbage collection
    pub fn collect_garbage(&mut self) {
        self.gc_space.collect();
    }

    /// Get GC statistics
    pub fn gc_stats(&self) -> GcStats {
        GcStats {
            alive_count: self.gc_space.alive_count(),
            tracked_count: self.gc_space.tracked_count(),
            roots_count: self.gc_space.roots_count(),
            free_count: self.gc_space.free_count(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GcStats {
    pub alive_count: usize,
    pub tracked_count: usize,
    pub roots_count: usize,
    pub free_count: usize,
}
```

### Step 1.8: Update Builtins to Use GC Space

**File: `src/interpreter/builtins/array.rs` (example)**

```rust
// Before
pub fn create_array_prototype() -> JsObjectRef {
    let p = create_object();
    // ...
    p
}

// After
pub fn create_array_prototype_gc(space: &mut Space<GcJsObject>) -> JsObjectRef {
    let p = create_object_gc(space);
    // ... register methods (functions also need GC allocation)
    p
}

// For methods that create objects, they need interpreter access
pub fn array_map(
    interp: &mut Interpreter,
    this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    // ... existing logic

    // Instead of: create_array(results)
    // Use:
    let result = interp.create_array(results);
    Ok(JsValue::Object(result))
}
```

### Step 1.9: Automatic Collection

The GC automatically triggers collection when needed. In `Space::alloc`, collection runs when the free list is exhausted:

```rust
// Already implemented in gc.rs:453-457
fn prepare_for_alloc(&mut self) {
    if self.free_list.is_empty() {
        self.collect();
    }
}
```

No additional triggers are needed. The interpreter can optionally expose manual collection for debugging:

```rust
impl Interpreter {
    /// Manually trigger garbage collection (for debugging/testing)
    pub fn collect_garbage(&mut self) {
        self.gc_space.collect();
    }
}
```

---

## Phase 2: Promise and Generator Integration

### Option A: Inline State (Recommended)

Move Promise/Generator state directly into `ExoticObject`:

```rust
pub enum ExoticObject {
    // ... existing variants

    // Change from Rc<RefCell<PromiseState>> to inline
    Promise {
        status: PromiseStatus,
        result: Option<JsValue>,
        handlers: Vec<PromiseHandler>,
    },

    // Change from Rc<RefCell<GeneratorState>> to inline
    Generator {
        body: Rc<BlockStatement>,
        params: Rc<[FunctionParam]>,
        args: Vec<JsValue>,
        closure: EnvId,
        state: GeneratorStatus,
        stmt_index: usize,
        sent_value: JsValue,
        name: Option<JsString>,
    },
}
```

**Pros:**
- Simpler implementation
- No separate allocation needed
- Automatically traced via `GcJsObject`

**Cons:**
- Larger `ExoticObject` enum size
- Some code changes for access patterns

### Option B: Separate GC Space

Keep separate `Rc<RefCell<T>>` but add a second space:

```rust
pub struct Interpreter {
    pub gc_space: Space<GcJsObject>,
    pub promise_space: Space<GcPromiseState>,  // Additional space
    // ...
}
```

**Pros:**
- Minimal changes to existing code
- State types remain separate

**Cons:**
- More complex
- Two spaces to manage
- Cross-space references need special handling

### Recommended: Option A (Inline)

The inline approach is simpler and more efficient. The `PromiseHandler.result_promise` already uses `JsObjectRef`, so it will automatically be GC-tracked once `JsObjectRef` is `Gc<GcJsObject>`.

---

## API Changes

### Breaking Changes

1. **`JsObjectRef` type change**
   ```rust
   // Before
   pub type JsObjectRef = Rc<RefCell<JsObject>>;

   // After
   pub type JsObjectRef = Gc<GcJsObject>;
   ```

2. **Object creation requires `&mut Space` or `&mut Interpreter`**
   ```rust
   // Before
   let obj = create_object();

   // After
   let obj = interp.create_plain_object();
   // or
   let obj = create_object_gc(&mut space);
   ```

3. **Borrow access through wrapper**
   ```rust
   // Before
   obj.borrow().prototype

   // After (same API, different underlying type)
   obj.borrow().prototype  // GcJsObject implements borrow()
   ```

### New APIs

```rust
// Interpreter methods
impl Interpreter {
    pub fn collect_garbage(&mut self);
    pub fn gc_stats(&self) -> GcStats;
}

// Optional: Expose gc() to JavaScript
// globalThis.gc() - triggers manual collection
```

---

## Migration Guide

### Step-by-Step Migration

1. **Add gc module to lib.rs**
   ```rust
   pub mod gc;
   ```

2. **Create GcJsObject wrapper in value.rs**
   - Add `GcJsObject` struct
   - Implement `Traceable`
   - Update `JsObjectRef` type alias

3. **Update Interpreter struct**
   - Add `gc_space: Space<GcJsObject>`
   - Update `new()` to use GC allocation

4. **Update object creation helpers**
   - Create `*_gc` versions of `create_object`, `create_array`, `create_function`
   - Add Interpreter methods for common patterns

5. **Update builtins**
   - Each `create_*_prototype` function needs space parameter
   - Methods that create objects need interpreter access

6. **Update expression evaluation**
   - Object literals, array literals, function expressions
   - All use interpreter's allocation methods

7. **Add collection triggers**
   - After top-level statements
   - When allocation pressure is high

8. **Testing**
   - Verify existing tests pass
   - Add cycle-specific tests

### Files to Modify (In Order)

| Order | File | Changes |
|-------|------|---------|
| 1 | `src/lib.rs` | Add `pub mod gc;` |
| 2 | `src/value.rs` | Add `GcJsObject`, `Traceable` impl, update type alias |
| 3 | `src/interpreter/mod.rs` | Add `gc_space`, update `new()`, add helper methods |
| 4 | `src/interpreter/builtins/object.rs` | Update to use GC space |
| 5 | `src/interpreter/builtins/array.rs` | Update to use GC space |
| 6 | `src/interpreter/builtins/function.rs` | Update to use GC space |
| 7 | `src/interpreter/builtins/string.rs` | Update to use GC space |
| 8 | `src/interpreter/builtins/number.rs` | Update to use GC space |
| 9 | `src/interpreter/builtins/map.rs` | Update to use GC space |
| 10 | `src/interpreter/builtins/set.rs` | Update to use GC space |
| 11 | `src/interpreter/builtins/date.rs` | Update to use GC space |
| 12 | `src/interpreter/builtins/regexp.rs` | Update to use GC space |
| 13 | `src/interpreter/builtins/error.rs` | Update to use GC space |
| 14 | `src/interpreter/builtins/promise.rs` | Update to use GC space |
| 15 | `src/interpreter/builtins/symbol.rs` | Update to use GC space |
| 16 | `src/interpreter/builtins/json.rs` | Update to use GC space |
| 17 | `src/interpreter/builtins/math.rs` | Update to use GC space |
| 18 | `src/interpreter/builtins/console.rs` | Update to use GC space |
| 19 | `src/interpreter/builtins/global.rs` | Update to use GC space |
| 20 | `src/interpreter/builtins/mod.rs` | Re-export updated functions |

---

## Testing Strategy

### Unit Tests for GC

**File: `src/gc.rs` (existing tests)**

The gc.rs already contains tests for:
- Simple allocation
- Cycle cleanup
- Rooted objects preserved
- Self-reference cycles
- Mixed scenarios

### Integration Tests

**File: `tests/interpreter/gc.rs` (new)**

```rust
use typescript_eval::{Runtime, JsValue};

fn eval(code: &str) -> JsValue {
    Runtime::new().eval(code).unwrap()
}

#[test]
fn test_object_cycle_collected() {
    let mut runtime = Runtime::new();

    // Create cycle
    runtime.eval(r#"
        const a = { name: 'a' };
        const b = { name: 'b', ref: a };
        a.ref = b;
    "#).unwrap();

    let before = runtime.interpreter().gc_stats().alive_count;

    // Clear references
    runtime.eval(r#"
        a = undefined;
        b = undefined;
    "#).unwrap();

    // Force collection
    runtime.interpreter_mut().collect_garbage();

    let after = runtime.interpreter().gc_stats().alive_count;

    // Cycle should be broken and objects collected
    assert!(after < before);
}

#[test]
fn test_rooted_objects_survive() {
    let mut runtime = Runtime::new();

    runtime.eval(r#"
        globalThis.keeper = { value: 42 };
    "#).unwrap();

    let before = runtime.interpreter().gc_stats().alive_count;
    runtime.interpreter_mut().collect_garbage();
    let after = runtime.interpreter().gc_stats().alive_count;

    // Rooted object should survive
    assert_eq!(before, after);

    let result = runtime.eval("keeper.value").unwrap();
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_promise_cycle_collected() {
    let mut runtime = Runtime::new();

    runtime.eval(r#"
        let p1, p2;
        p1 = new Promise(r => { p2 = Promise.resolve().then(() => p1); });
    "#).unwrap();

    let before = runtime.interpreter().gc_stats().alive_count;

    runtime.eval(r#"
        p1 = undefined;
        p2 = undefined;
    "#).unwrap();

    runtime.interpreter_mut().collect_garbage();

    let after = runtime.interpreter().gc_stats().alive_count;
    assert!(after < before);
}

#[test]
fn test_gc_stats() {
    let mut runtime = Runtime::new();

    let stats = runtime.interpreter().gc_stats();
    assert!(stats.roots_count > 0);  // Prototypes are rooted
    assert!(stats.alive_count >= stats.roots_count);
}
```

### Stress Tests

```rust
#[test]
fn test_many_cycles() {
    let mut runtime = Runtime::new();

    // Create 1000 cycles
    for i in 0..1000 {
        runtime.eval(&format!(r#"
            const a{i} = {{}};
            const b{i} = {{ ref: a{i} }};
            a{i}.ref = b{i};
        "#, i = i)).unwrap();
    }

    let before = runtime.interpreter().gc_stats().alive_count;

    // Clear all references
    for i in 0..1000 {
        runtime.eval(&format!("a{i} = undefined; b{i} = undefined;", i = i)).unwrap();
    }

    runtime.interpreter_mut().collect_garbage();

    let after = runtime.interpreter().gc_stats().alive_count;

    // Should collect ~2000 objects (the cycle pairs)
    assert!(before - after >= 1900);
}

#[test]
fn test_deep_prototype_chain() {
    let mut runtime = Runtime::new();

    runtime.eval(r#"
        let obj = {};
        for (let i = 0; i < 100; i++) {
            obj = Object.create(obj);
        }
        globalThis.deepObj = obj;
    "#).unwrap();

    runtime.interpreter_mut().collect_garbage();

    // Entire chain should survive (reachable from root)
    let result = runtime.eval(r#"
        let count = 0;
        let p = deepObj;
        while (p !== null) {
            count++;
            p = Object.getPrototypeOf(p);
        }
        count
    "#).unwrap();

    assert_eq!(result, JsValue::Number(102.0));  // 100 + Object.prototype + null
}
```

---

## Performance Considerations

### Allocation Overhead

| Operation | Before (Rc) | After (Gc) | Difference |
|-----------|-------------|------------|------------|
| Allocate object | ~50ns | ~80ns | +60% |
| Clone reference | ~5ns | ~8ns | +60% |
| Access via borrow | ~2ns | ~2ns | Same |

**Note**: These are estimated; actual performance depends on Space implementation details.

### Collection Pause Times

Mark-and-sweep collection pauses all execution:
- **1,000 objects**: ~0.1ms
- **10,000 objects**: ~1ms
- **100,000 objects**: ~10ms

### Recommendations

1. **Tune initial capacity**: Start with reasonable size to avoid early resizing
   ```rust
   Space::with_capacity(4096)
   ```

2. **Automatic collection**: The GC triggers collection automatically when the free list is exhausted during allocation - no manual tuning needed.

3. **Consider incremental collection** (future enhancement):
   - Mark phase spread across multiple steps
   - Reduces max pause time

4. **Monitor GC stats** in production:
   ```rust
   if cfg!(debug_assertions) {
       eprintln!("GC: alive={}, tracked={}", stats.alive_count, stats.tracked_count);
   }
   ```

---

## Appendix: File Change Summary

### New Files

| File | Purpose |
|------|---------|
| `tests/interpreter/gc.rs` | GC integration tests |

### Modified Files

| File | Lines Changed (Est.) | Changes |
|------|---------------------|---------|
| `src/lib.rs` | +2 | Add gc module |
| `src/value.rs` | +150 | GcJsObject, Traceable impl |
| `src/interpreter/mod.rs` | +100 | Space field, helper methods |
| `src/interpreter/builtins/mod.rs` | +20 | Re-exports |
| `src/interpreter/builtins/object.rs` | +30 | GC-aware creation |
| `src/interpreter/builtins/array.rs` | +40 | GC-aware creation |
| `src/interpreter/builtins/function.rs` | +30 | GC-aware creation |
| `src/interpreter/builtins/string.rs` | +30 | GC-aware creation |
| `src/interpreter/builtins/number.rs` | +20 | GC-aware creation |
| `src/interpreter/builtins/map.rs` | +20 | GC-aware creation |
| `src/interpreter/builtins/set.rs` | +20 | GC-aware creation |
| `src/interpreter/builtins/date.rs` | +20 | GC-aware creation |
| `src/interpreter/builtins/regexp.rs` | +20 | GC-aware creation |
| `src/interpreter/builtins/error.rs` | +30 | GC-aware creation |
| `src/interpreter/builtins/promise.rs` | +40 | GC-aware creation |
| `src/interpreter/builtins/symbol.rs` | +20 | GC-aware creation |
| `src/interpreter/builtins/json.rs` | +10 | GC-aware creation |
| `src/interpreter/builtins/math.rs` | +10 | GC-aware creation |
| `src/interpreter/builtins/console.rs` | +10 | GC-aware creation |
| `src/interpreter/builtins/global.rs` | +20 | GC-aware creation |

**Total Estimated Changes**: ~800-1000 lines

### Dependency Changes

None required - gc.rs uses only std library types.

---

## Conclusion

Integrating the GC from `gc.rs` will:

1. **Eliminate memory leaks** from object reference cycles
2. **Handle Promise chains** that currently leak
3. **Maintain compatibility** with existing code (same borrow API)
4. **Add minimal overhead** (~60% allocation cost, periodic collection pauses)

The arena-based environment system already prevents closure cycles, so this GC integration focuses on the remaining cycle sources: object properties, Promise handlers, and accessor properties.

The recommended approach is **Phase 1** (core object GC) with **inline Promise/Generator state** (Option A from Phase 2) for simplicity and efficiency.
