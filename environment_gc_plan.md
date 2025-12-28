# Environment GC Integration Plan

## Profiling Results

### How Profiling Was Done

```bash
cargo build --profile=profiling
valgrind --leak-check=full ./target/profiling/tsrun examples/memory-management/gc-cycles.ts
```

### Valgrind Output Summary

```
HEAP SUMMARY:
    in use at exit: 69,323,884 bytes in 225,140 blocks
  total heap usage: 924,966 allocs, 699,826 frees, 162,878,481 bytes allocated

LEAK SUMMARY:
   definitely lost: 9,171,388 bytes in 44,992 blocks
   indirectly lost: 60,107,732 bytes in 179,969 blocks
     possibly lost: 44,308 bytes in 178 blocks
   still reachable: 456 bytes in 1 blocks

ERROR SUMMARY: 103 errors from 103 contexts
```

### Key Leak Sources

The largest leak (50.7MB direct + indirect) comes from object allocation:

```
50,711,012 (6,008,352 direct, 44,702,660 indirect) bytes in 35,764 blocks are definitely lost
   at tsrun::gc::Space<T>::alloc (gc.rs:220)
   at tsrun::value::create_object (value.rs:1441)
   at tsrun::interpreter::Interpreter::create_object (mod.rs:506)
   at tsrun::interpreter::Interpreter::evaluate (mod.rs:3111)
```

Objects are allocated in the GC space but never collected because:
1. They form cycles (mutual references)
2. Environment bindings hold strong `Rc` references outside of GC tracking
3. When environments are freed, cyclic objects still have `strong_count > 1`

## Problem Statement

Valgrind reports significant memory leaks (~69MB definitely lost) when running GC cycle tests. The root cause is that the `EnvironmentArena` holds `JsValue`s (which contain `Gc<JsObject>` references) outside of the GC's knowledge.

### Current Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Interpreter                             │
├─────────────────────────────────────────────────────────────┤
│  gc_space: Space<JsObject>    │    env_arena: EnvironmentArena │
│  ┌─────────────────────────┐  │    ┌──────────────────────────┐ │
│  │ Gc<JsObject> refs       │  │    │ Vec<Environment>         │ │
│  │  - prototypes (rooted)  │  │    │  - bindings: HashMap     │ │
│  │  - global object        │  │    │    - JsValue (contains   │ │
│  │  - user objects         │  │    │      Gc<JsObject>!)      │ │
│  └─────────────────────────┘  │    │  - outer: Option<EnvId>  │ │
│         ↑                     │    └──────────────────────────┘ │
│         │ traced              │               ↑                 │
│         │                     │               │ NOT traced      │
│  roots ─┘                     │    ───────────┘                 │
└─────────────────────────────────────────────────────────────┘
```

### Why Memory Leaks Occur

1. **GC only traces from roots**: The GC marks objects reachable from `roots` (global object + prototypes)
2. **Environments hold strong references**: `Binding.value: JsValue` can contain `JsObjectRef` (which is `Gc<JsObject>`)
3. **Environments are not traced**: The `EnvironmentArena` is completely separate from the GC system
4. **Cycles keep objects alive**: When objects form cycles (e.g., `a.other = b; b.other = a`), the `Rc` strong count > 1
5. **GC can't break cycles**: The `break_cycles()` function only unlinks objects with `strong_count == 1`, but cyclic objects have higher counts due to mutual references

### Example Leak Scenario

```typescript
for (let i = 0; i < 10000; i++) {
    const a = { other: null };
    const b = { other: null };
    a.other = b;
    b.other = a;
    // Loop ends - 'a' and 'b' go out of scope
}
```

What happens:
1. Each iteration creates objects `a` and `b` in `gc_space`
2. The loop block environment stores bindings `a -> JsObject` and `b -> JsObject`
3. The cycle `a.other = b; b.other = a` creates mutual strong `Rc` references
4. When the loop iteration ends, `try_free()` clears the environment bindings
5. BUT: The `Rc` strong counts are still > 1 due to the cycle
6. GC marks from roots, doesn't find `a` or `b` (they're not reachable from global)
7. `break_cycles()` sees `strong_count > 1` for both, doesn't unlink them
8. Objects are removed from GC tracking but `Rc` never drops to 0 → **MEMORY LEAK**

## Solution: Make Environment a GC-Managed Type

Store environments as `JsObject` instances with a new `ExoticObject::Environment` variant.

```rust
pub enum ExoticObject {
    // ... existing variants ...
    Environment(EnvironmentData),
}

pub struct EnvironmentData {
    pub bindings: FxHashMap<JsString, Binding>,
    pub outer: Option<JsObjectRef>,  // Parent env is also a JsObject
}
```

**Why this works:**
- Everything in one `Space<JsObject>` - unified tracing
- Environments participate in cycle detection naturally
- `outer` reference becomes a GC pointer that can be traced/unlinked
- Existing `Traceable` impl for `JsObject` traces everything

**Trade-offs:**
- Need to update all environment access patterns
- `InterpretedFunction.closure` becomes `JsObjectRef` instead of `EnvId`
- More complex environment lookup (need to borrow, check exotic type)
- Performance overhead of going through `Gc<JsObject>` for env access

## Implementation Plan

#### Phase 1: Add Environment to ExoticObject

1. Add new types:
```rust
// In value.rs
pub struct EnvironmentData {
    pub bindings: FxHashMap<JsString, Binding>,
    pub outer: Option<JsObjectRef>,
}

pub enum ExoticObject {
    // ... existing ...
    Environment(EnvironmentData),
}
```

2. Update `Traceable` impl for `JsObject`:
```rust
fn trace(&self, tracer: &mut Tracer<'_>) {
    // ... existing tracing ...

    // Trace environment data
    if let ExoticObject::Environment(env) = &self.exotic {
        for binding in env.bindings.values() {
            trace_jsvalue(&binding.value, tracer);
        }
        if let Some(ref outer) = env.outer {
            tracer.trace(outer);
        }
    }
}

fn unlink(&mut self) {
    // ... existing unlinking ...

    // Clear environment data
    if let ExoticObject::Environment(ref mut env) = &mut self.exotic {
        env.bindings.clear();
        env.outer = None;
    }
}
```

#### Phase 2: Create EnvRef Type and Helper Functions

```rust
// Type alias for clarity
pub type EnvRef = JsObjectRef;

// Helper to create environment objects
pub fn create_environment(
    space: &mut Space<JsObject>,
    outer: Option<EnvRef>,
) -> EnvRef {
    let mut obj = JsObject::new();
    obj.exotic = ExoticObject::Environment(EnvironmentData {
        bindings: FxHashMap::default(),
        outer,
    });
    space.alloc(obj)
}

// Helper to access environment data
impl JsObject {
    pub fn as_environment(&self) -> Option<&EnvironmentData> {
        match &self.exotic {
            ExoticObject::Environment(data) => Some(data),
            _ => None,
        }
    }

    pub fn as_environment_mut(&mut self) -> Option<&mut EnvironmentData> {
        match &mut self.exotic {
            ExoticObject::Environment(data) => Some(data),
            _ => None,
        }
    }
}
```

#### Phase 3: Update InterpretedFunction

```rust
pub struct InterpretedFunction {
    pub name: Option<JsString>,
    pub params: Rc<[FunctionParam]>,
    pub body: Rc<FunctionBody>,
    pub closure: EnvRef,  // Changed from EnvId
    pub source_location: Span,
    pub generator: bool,
    pub async_: bool,
}
```

Update `JsFunction::closure_env()` to return `Option<&EnvRef>`.

#### Phase 4: Update Interpreter

1. Remove `EnvironmentArena` field
2. Replace `env: EnvId` with `env: EnvRef`
3. Create global environment as `EnvRef` and root it
4. Update all environment operations:
   - `env_arena.alloc(outer)` → `create_environment(&mut self.gc_space, outer)`
   - `env_arena.get(id)` → `env.borrow().as_environment()`
   - `env_arena.define(...)` → modify environment data directly
   - `env_arena.try_free(id)` → just drop the `EnvRef` (GC handles cleanup)

5. Remove capture counting (no longer needed - GC traces everything)

#### Phase 5: Testing and Validation

1. Run existing test suite - all tests should pass
2. Re-run valgrind with gc-cycles.ts - leaks should be eliminated
3. Run performance benchmarks to measure overhead
4. Test closure-heavy code for correctness

### Migration Notes

#### Breaking Changes
- `EnvId` type removed (replaced by `EnvRef = JsObjectRef`)
- `EnvironmentArena` removed
- `InterpretedFunction.closure` type changes

#### Performance Considerations
- Environment access now goes through `Gc<JsObject>` (one indirection)
- `borrow()` and `borrow_mut()` calls for every environment operation
- Offset by removal of capture counting overhead
- GC now needs to trace more objects (environments)

#### Invariants to Maintain
- Global environment must be rooted
- Current environment must always be valid during execution
- Outer chain must not have cycles (enforced by construction)

## Testing Strategy

### Unit Tests
- Environment creation and binding operations
- Closure capture and execution
- Outer chain traversal

### Integration Tests
- All existing interpreter tests
- New cycle collection tests

### Memory Tests (valgrind)
- gc-cycles.ts with various SCALE values
- Memory should stay bounded regardless of iteration count

### Performance Tests
- Benchmark closure-heavy workloads
- Measure environment access overhead
- Compare before/after GC threshold behavior
