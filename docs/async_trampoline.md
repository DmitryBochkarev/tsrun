# Async Function Trampoline Migration Plan

## Overview

Currently, async functions (`JsFunction::BytecodeAsync`) fall back to recursive `call_function_with_new_target` calls. This document outlines a plan to eliminate this recursion by handling async functions directly in the trampoline.

## Current Implementation

### How Async Functions Work Today

When an async function is called:

1. `setup_trampoline_call` matches `BytecodeAsync` and calls:
   ```rust
   JsFunction::BytecodeAsync(bc_func) => {
       let result = interp.call_function_with_new_target(callee, this_value, &args, new_target)?;
       self.set_reg(return_register, result.value);
       Ok(())
   }
   ```

2. `call_function_with_new_target` dispatches to `call_bytecode_async_function`:
   ```rust
   fn call_bytecode_async_function(&mut self, bc_func, this_value, args) -> Result<Guarded, JsError> {
       // Execute the function body synchronously (RECURSIVE!)
       let body_result = self.call_bytecode_function(bc_func, this_value, args);

       // Wrap result in Promise (fulfilled or rejected)
       match body_result {
           Ok(guarded) => {
               // Promise assimilation: if result is Promise, return it
               // Otherwise: create fulfilled promise
           }
           Err(e) => {
               // Create rejected promise with error
           }
       }
   }
   ```

3. `call_bytecode_function` creates a new `BytecodeVM` and calls `vm.run()` - this is the **recursion point**

### The Problem

Each async function call creates a new Rust stack frame through `vm.run()`. Deep call chains of async functions can cause stack overflow.

Example problematic pattern:
```javascript
async function a() { return await b(); }
async function b() { return await c(); }
async function c() { return await d(); }
// ... deep nesting
```

### How Generators Avoid This

Generators don't have this problem because calling a generator function **doesn't run the body**:

```rust
JsFunction::BytecodeGenerator(bc_func) => {
    // Just create generator object - NO vm.run() call
    let state = BytecodeGeneratorState { chunk, closure, args, ... };
    let gen_obj = create_bytecode_generator_object(interp, state);
    self.set_reg(return_register, JsValue::Object(gen_obj));
    Ok(())
}
```

The body runs later when `.next()` is called - and that happens through `resume_bytecode_generator`, not recursion.

## Proposed Solution

### Key Insight

Async functions can be viewed as **generators that return Promises**:

1. Create a Promise immediately when called
2. Run the body using the trampoline (like regular bytecode functions)
3. When body completes or throws, resolve/reject the Promise
4. If `await` is hit on a pending Promise, suspend and resume later

### Implementation Strategy

#### Step 1: Add `AsyncFrame` to `TrampolineFrame`

Extend `TrampolineFrame` to track async-specific state:

```rust
pub struct TrampolineFrame {
    // ... existing fields ...

    /// For async calls: the Promise to resolve/reject when function completes
    pub async_promise: Option<Gc<JsObject>>,
    /// For async calls: the resolve function
    pub async_resolve: Option<Gc<JsObject>>,
    /// For async calls: the reject function
    pub async_reject: Option<Gc<JsObject>>,
}
```

#### Step 2: Add `setup_trampoline_async_call`

Create a new method that:

1. Creates a pending Promise with resolve/reject functions
2. Pushes a trampoline frame with the async state
3. Sets up the bytecode to run (like regular bytecode call)
4. Returns the Promise immediately

```rust
fn setup_trampoline_async_call(
    &mut self,
    interp: &mut Interpreter,
    bc_func: BytecodeFunction,
    this_value: JsValue,
    args: Vec<JsValue>,
    return_register: Register,
) -> Result<(), JsError> {
    // Create a pending Promise with resolvers
    let (promise, resolve, reject) = interp.create_promise_with_resolvers()?;

    // Push trampoline frame with async state
    self.push_trampoline_frame_and_call_bytecode_async(
        interp,
        bc_func,
        this_value,
        &args,
        return_register,
        promise.cheap_clone(),
        resolve,
        reject,
    )?;

    // Store the Promise in return_register immediately
    // (Caller sees the Promise, body runs via trampoline)
    self.set_reg(return_register, JsValue::Object(promise));

    Ok(())
}
```

#### Step 3: Handle Async Completion in `restore_from_trampoline_frame`

When an async function's trampoline frame completes:

```rust
fn restore_from_trampoline_frame(&mut self, interp: &mut Interpreter, frame: TrampolineFrame, return_value: JsValue) {
    // Check if this was an async call
    if let (Some(resolve), Some(_reject)) = (frame.async_resolve, frame.async_reject) {
        // Resolve the promise with the return value
        // Handle promise assimilation if return_value is a Promise
        if is_promise(&return_value) {
            // Chain to the returned promise
            chain_promise(interp, return_value, resolve, reject);
        } else {
            // Resolve directly
            call_resolve(interp, resolve, return_value);
        }
        // Don't restore to previous frame - just return
        return;
    }

    // ... existing restoration logic ...
}
```

#### Step 4: Handle Async Errors

When an error occurs in an async function:

```rust
// In the error handling path of run():
if let Some(frame) = self.trampoline_stack.last() {
    if let Some(reject) = &frame.async_reject {
        // Reject the promise instead of propagating error
        call_reject(interp, reject.clone(), error.to_value());
        self.trampoline_stack.pop();
        continue; // Resume outer frame
    }
}
```

#### Step 5: Handle `await` on Pending Promises

Currently `Op::Await` returns `OpResult::Suspend` when hitting a pending Promise. We need to:

1. Save the Promise's resolve/reject to the current async frame
2. Register a callback on the awaited Promise to resume execution
3. Pop up to the next non-async frame (or complete if none)

```rust
Op::Await { dst, promise } => {
    // ... existing pending promise detection ...

    PromiseStatus::Pending => {
        // For async functions in trampoline, we need to:
        // 1. Register a .then() callback on the awaited promise
        // 2. When it resolves, resume this async function
        // 3. Continue the outer trampoline loop

        if let Some(async_frame) = self.find_current_async_frame() {
            // Register continuation callback
            register_await_continuation(
                interp,
                awaited_promise,
                async_frame,
                dst, // Register to receive resolved value
            );

            // Return to continue outer frame
            return Ok(OpResult::AwaitPending);
        }

        // Top-level await - suspend as before
        return Ok(OpResult::Suspend { ... });
    }
}
```

### New OpResult Variant

Add a new variant for async await:

```rust
pub enum OpResult {
    Continue,
    Return(Guarded),
    Call { ... },
    Construct { ... },
    Suspend { ... },
    Yield { ... },
    YieldStar { ... },

    /// Async function is awaiting a pending Promise
    /// The Promise's .then() has been registered to resume execution
    AwaitPending,
}
```

## Implementation Phases

### Phase A: Foundation (Low Risk)

1. Add `async_promise`, `async_resolve`, `async_reject` fields to `TrampolineFrame`
2. Add helper `create_promise_with_resolvers` if not exists
3. Write tests for async functions with deep call chains

### Phase B: Simple Async (Medium Risk)

1. Implement `push_trampoline_frame_and_call_bytecode_async`
2. Modify `restore_from_trampoline_frame` for async completion
3. Test async functions that don't use `await`

### Phase C: Await Handling (High Complexity)

1. Add `OpResult::AwaitPending` variant
2. Implement `register_await_continuation` to attach `.then()` callbacks
3. Implement resumption logic when awaited Promise resolves
4. Handle error propagation through awaited Promises

### Phase D: Edge Cases

1. Nested async calls (async calling async)
2. `await` in try/catch/finally
3. Promise.all/race with async functions
4. Async function returning a Promise (assimilation)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_async_trampoline_simple() {
    // Async function that returns immediately
    assert_promise_resolves("(async () => 42)()", 42);
}

#[test]
fn test_async_trampoline_await_resolved() {
    // Await on already-resolved promise
    assert_promise_resolves("(async () => await Promise.resolve(42))()", 42);
}

#[test]
fn test_async_trampoline_deep_call_chain() {
    // Many nested async calls - should not stack overflow
    let code = r#"
        async function chain(n) {
            if (n <= 0) return 0;
            return await chain(n - 1) + 1;
        }
        chain(100)
    "#;
    assert_promise_resolves(code, 100);
}

#[test]
fn test_async_trampoline_error_propagation() {
    // Errors should reject the promise
    assert_promise_rejects("(async () => { throw 'error'; })()", "error");
}

#[test]
fn test_async_trampoline_await_rejection() {
    // Await on rejected promise should propagate
    assert_promise_rejects("(async () => await Promise.reject('oops'))()", "oops");
}
```

### Stress Tests

```rust
#[test]
fn test_async_deep_recursion_no_stack_overflow() {
    let mut runtime = Runtime::new();
    runtime.set_max_call_depth(1000);

    let result = runtime.eval_simple(r#"
        async function fib(n) {
            if (n <= 1) return n;
            return await fib(n-1) + await fib(n-2);
        }
        fib(15)  // Would overflow without trampoline
    "#);

    // Should complete without stack overflow
    assert_promise_resolves_to(result, 610.0);
}
```

## Complexity Analysis

| Component | Complexity | Risk | Notes |
|-----------|------------|------|-------|
| TrampolineFrame fields | Low | Low | Just adding fields |
| Promise creation | Low | Low | Already have Promise support |
| Async frame pushing | Medium | Medium | Similar to existing bytecode pushing |
| Async completion handling | Medium | Medium | Resolve/reject logic |
| Await continuation | High | High | Complex callback registration |
| Error propagation | High | High | Must handle all error paths |
| Nested async | High | High | Multiple active async frames |

## Alternative Approaches Considered

### 1. Convert Async to Generators

Transform async functions to generators internally:
- `async function f() { await x; return y; }` becomes
- `function* f() { yield x; return y; }` with Promise wrapper

**Pros**: Reuses existing generator suspension
**Cons**: Significant compiler changes, harder to debug

### 2. Stackless Coroutines

Implement full stackless execution with explicit continuation passing.

**Pros**: Most general solution
**Cons**: Massive refactor, changes all code paths

### 3. Current Approach: Trampoline Extension

Extend the existing trampoline pattern to handle async state.

**Pros**: Incremental, builds on working pattern
**Cons**: Adds complexity to TrampolineFrame

## Recommendation

Proceed with the **Trampoline Extension** approach (Option 3):

1. It's an incremental improvement over the current working system
2. Reuses the proven trampoline pattern from regular calls
3. Can be implemented in phases with testing at each step
4. Doesn't require compiler changes

Start with **Phase A** (foundation) and **Phase B** (simple async without await), which provide value even without full await support. Phase C (await handling) can be tackled once the foundation is solid.

## Files to Modify

| File | Changes |
|------|---------|
| `src/interpreter/bytecode_vm.rs` | TrampolineFrame, run() loop, await handling |
| `src/interpreter/mod.rs` | Remove `call_bytecode_async_function` eventually |
| `src/interpreter/builtins/promise.rs` | Add `create_promise_with_resolvers` helper |
| `tests/interpreter/async_await.rs` | Add deep recursion tests |

## Open Questions

1. **Await in try/finally**: How to handle `await` inside a try block with finally?
   - The finally must run even if await throws
   - May need to track pending finally blocks in async frame

2. **Multiple awaits**: Async function with multiple awaits needs to resume correctly
   - IP is saved in trampoline frame, should resume at next instruction

3. **Async calling sync calling async**: Mixed call chains
   - Sync functions use regular trampoline
   - Async functions add Promise wrapping layer

4. **Top-level await**: Currently returns `VmSuspension`
   - Should continue to work for module-level await
   - Trampoline async is for function-level

## Success Criteria

1. All existing async tests pass
2. Deep async recursion (100+ levels) doesn't stack overflow
3. Error propagation works correctly
4. Performance doesn't regress significantly
5. Code is maintainable and well-documented
