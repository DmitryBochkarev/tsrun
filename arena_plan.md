# Arena-Based Environment Storage Plan

## Problem

Reference cycles between closures and environments cause memory leaks:

```
Environment.bindings → JsValue::Object → JsFunction.closure (Rc<Environment>) → Environment
```

With `Rc<Environment>`, short-lived closures in loops accumulate memory:
- 10,000 loop iterations with closures: 16MB leaked during execution
- Same loop without closures: 456 bytes

## Solution

Replace `Rc<Environment>` with arena-based storage using indices (`EnvId`).

## New Types

```rust
/// Index into environment arena
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvId(usize);

/// Arena owning all environments
pub struct EnvironmentArena {
    envs: Vec<Environment>,
    free_list: Vec<usize>,  // Reusable slots
}

/// Environment now uses EnvId instead of Rc
pub struct Environment {
    pub bindings: HashMap<JsString, Binding>,  // No longer Rc<RefCell<...>>
    pub outer: Option<EnvId>,                  // Was: Option<Rc<Environment>>
}
```

## Changes Required

### 1. value.rs - Environment Types

- [x] Add `EnvId` type
- [x] Add `EnvironmentArena` with alloc/free/get/set methods
- [ ] Modify `Environment`:
  - Change `bindings: Rc<RefCell<HashMap<...>>>` → `bindings: HashMap<...>`
  - Change `outer: Option<Rc<Environment>>` → `outer: Option<EnvId>`
  - Remove `Environment::get`, `set`, `has` methods (move to arena)
- [ ] Modify `InterpretedFunction`:
  - Change `closure: Rc<Environment>` → `closure: EnvId`
- [ ] Modify `GeneratorState`:
  - Change `closure: Rc<Environment>` → `closure: EnvId`

### 2. interpreter/mod.rs - Interpreter

- [ ] Add `env_arena: EnvironmentArena` field
- [ ] Change `env: Rc<Environment>` → `env: EnvId`
- [ ] Update `Interpreter::new()` to create arena
- [ ] Replace all `self.env.define(...)` → `self.env_arena.define(self.env, ...)`
- [ ] Replace all `self.env.get(...)` → `self.env_arena.get_binding(self.env, ...)`
- [ ] Replace all `self.env.set(...)` → `self.env_arena.set_binding(self.env, ...)`
- [ ] Replace all `Rc::new(Environment::child(...))` → `self.env_arena.alloc(Some(self.env))`
- [ ] Replace all `Rc::new(Environment::with_outer(...))` → `self.env_arena.alloc(Some(...))`
- [ ] Add environment freeing when scopes exit (blocks, functions, loops)
- [ ] Remove `Drop` implementation (no longer needed)

### 3. Key Call Sites to Update

```rust
// Before:
self.env = Rc::new(Environment::child(&self.env));
// After:
self.env = self.env_arena.alloc(Some(self.env));

// Before:
self.env.define(name, value, mutable);
// After:
self.env_arena.define(self.env, name, value, mutable);

// Before:
self.env.get(&name)?
// After:
self.env_arena.get_binding(self.env, &name)?
```

### 4. Environment Lifetime Management

When a scope exits, we need to free its environment:

```rust
// Block scope
fn execute_block(&mut self, block: &BlockStatement) -> Result<Completion, JsError> {
    let prev_env = self.env;
    self.env = self.env_arena.alloc(Some(prev_env));

    let result = self.execute_statements(&block.body);

    let block_env = self.env;
    self.env = prev_env;
    self.env_arena.free(block_env);  // NEW: free the block's environment

    result
}
```

**Important**: Only free environments that are NOT captured by closures.

### 5. Closure Capture Tracking (Option C: Scope-based with capture flag)

When a function is created, it captures `self.env` as its closure. We must NOT free that environment until the function is dropped.

**Chosen approach: Scope-based tracking with capture flag**

Each environment has a `captured: bool` flag:
- When a closure is created, mark its captured environment (and ancestors) as captured
- When exiting a scope, only free the environment if `captured == false`
- Captured environments stay alive until Interpreter drops

```rust
pub struct Environment {
    pub bindings: HashMap<JsString, Binding>,
    pub outer: Option<EnvId>,
    pub captured: bool,  // True if a closure references this env
}

impl EnvironmentArena {
    /// Mark an environment (and its ancestors) as captured by a closure
    pub fn mark_captured(&mut self, id: EnvId) {
        let mut current = Some(id);
        while let Some(env_id) = current {
            if let Some(env) = self.get_mut(env_id) {
                if env.captured {
                    break;  // Already captured, ancestors must be too
                }
                env.captured = true;
                current = env.outer;
            } else {
                break;
            }
        }
    }

    /// Try to free an environment. Only succeeds if not captured.
    pub fn try_free(&mut self, id: EnvId) -> bool {
        if id == EnvId::GLOBAL {
            return false;
        }
        if let Some(env) = self.get(id) {
            if env.captured {
                return false;  // Can't free, it's captured
            }
        }
        self.free(id);
        true
    }
}
```

**Usage pattern:**

```rust
// When creating a function/closure:
let func = InterpretedFunction {
    closure: self.env,
    // ...
};
self.env_arena.mark_captured(self.env);

// When exiting a block scope:
fn execute_block(&mut self, block: &BlockStatement) -> Result<Completion, JsError> {
    let prev_env = self.env;
    self.env = self.env_arena.alloc(Some(prev_env));

    let result = self.execute_statements(&block.body);

    let block_env = self.env;
    self.env = prev_env;
    self.env_arena.try_free(block_env);  // Only frees if not captured

    result
}
```

**Advantages:**
- Simple flag per environment
- No reference counting needed
- Captured environments stay alive (correct semantics)
- Non-captured environments are freed immediately (good memory usage)

**Trade-off:**
- Captured environments are never freed during execution
- For pathological cases (creating many closures), memory grows
- But normal code (loops with temporary closures that escape) works well

## Testing

```bash
# Should show no leaks at exit
valgrind --leak-check=full ./target/debug/typescript-eval-runner examples/algorithms/main.ts

# Should show stable memory during execution (no accumulation)
cat > /tmp/loop_closures.ts << 'EOF'
let sum = 0;
for (let i = 0; i < 10000; i++) {
    let fn = () => i;
    sum = sum + fn();
}
sum
EOF
valgrind ./target/debug/typescript-eval-runner /tmp/loop_closures.ts
```

## Migration Strategy

1. Add new types (`EnvId`, `EnvironmentArena`) - DONE
2. Add arena to Interpreter, keep old `Rc<Environment>` working
3. Migrate one file at a time, running tests after each
4. Remove old `Rc<Environment>` code
5. Remove `Drop` implementation
6. Verify with valgrind
