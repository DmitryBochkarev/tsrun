# Arena-Based Environment Storage Plan

## Problem

Reference cycles between closures and environments cause memory leaks:

```
Environment.bindings → JsValue::Object → JsFunction.closure (Rc<Environment>) → Environment
```

With `Rc<Environment>`, short-lived closures in loops accumulate memory:
- 10,000 loop iterations with closures: 16MB leaked during execution
- Same loop without closures: 456 bytes

## Solution - IMPLEMENTED

Replace `Rc<Environment>` with arena-based storage using indices (`EnvId`) and
reference counting for capture tracking.

### Results

After implementation:
- With closures: **6.2 MB** (was 22.5 MB)
- Without closures: **5.7 MB**
- Memory difference reduced from 17 MB to 0.5 MB

## Implementation Details

### Environment Structure

```rust
pub struct Environment {
    pub bindings: HashMap<JsString, Binding>,
    pub outer: Option<EnvId>,
    pub capture_count: usize,  // Number of closures capturing this env
}
```

### Key Methods

```rust
impl EnvironmentArena {
    /// Increment capture count when closure is created
    pub fn increment_capture(&mut self, id: EnvId);

    /// Decrement capture count when closure is dropped
    pub fn decrement_capture(&mut self, id: EnvId) -> bool;

    /// Try to free an environment (handles self-referential closures)
    pub fn try_free(&mut self, id: EnvId) -> bool;

    /// Set binding with automatic capture decrement for old function values
    pub fn set_binding(&mut self, id: EnvId, name: &JsString, value: JsValue) -> Result<(), JsError>;

    /// Free environment with automatic capture decrement for function bindings
    pub fn free(&mut self, id: EnvId);
}
```

### Self-Referential Closure Handling

The key insight is handling closures that capture their own environment:

```javascript
for (let i = 0; i < 10000; i++) {
    let fn = () => i;  // fn is in iter_env and captures iter_env
    sum = sum + fn();
}
```

`try_free` counts "self-captures" (closures in the environment that capture that
same environment) and only blocks freeing for external captures:

```rust
pub fn try_free(&mut self, id: EnvId) -> bool {
    let self_captures = /* count bindings that captured id */;
    let capture_count = self.get(id).map(|e| e.capture_count).unwrap_or(0);

    if capture_count > self_captures {
        return false; // External captures exist
    }
    self.free(id);
    true
}
```

## Checklist - All Complete

### 1. value.rs - Environment Types ✅

- [x] Add `EnvId` type
- [x] Add `EnvironmentArena` with alloc/free/get/set methods
- [x] Modify `Environment`:
  - Change `bindings: Rc<RefCell<HashMap<...>>>` → `bindings: HashMap<...>`
  - Change `outer: Option<Rc<Environment>>` → `outer: Option<EnvId>`
  - Change `captured: bool` → `capture_count: usize`
- [x] Modify `InterpretedFunction`:
  - Change `closure: Rc<Environment>` → `closure: EnvId`
- [x] Modify `GeneratorState`:
  - Change `closure: Rc<Environment>` → `closure: EnvId`
- [x] Add `closure_env()` helper to JsValue and JsFunction

### 2. interpreter/mod.rs - Interpreter ✅

- [x] Add `env_arena: EnvironmentArena` field
- [x] Change `env: Rc<Environment>` → `env: EnvId`
- [x] Update `Interpreter::new()` to create arena
- [x] Replace all `mark_captured` → `increment_capture`
- [x] Add environment freeing when scopes exit (blocks, functions, loops)
- [x] Handle self-referential closures in `try_free`
- [x] Decrement captures when function values are overwritten

### 3. Testing ✅

```bash
# Memory usage with closures
/usr/bin/time -v ./target/debug/typescript-eval-runner examples/loop_closures.ts
# Result: 6.2 MB (was 22.5 MB)

# Memory usage without closures
/usr/bin/time -v ./target/debug/typescript-eval-runner examples/loop_no_closures.ts
# Result: 5.7 MB

# No leaks
valgrind --leak-check=full ./target/debug/typescript-eval-runner examples/loop_closures.ts
# Result: 0 bytes lost
```
