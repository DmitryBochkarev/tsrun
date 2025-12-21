# Trampoline Migration Plan

## Overview

The bytecode VM currently uses Rust recursion for JavaScript function calls. Each JS function call creates a new `vm.run()` call on the Rust stack, which can cause stack overflow on deep call chains (even 3-level inheritance with super calls).

This document outlines the plan to migrate all recursive call sites to use a trampoline pattern, where instead of recursively calling into the VM, we save state and iterate in a single loop.

## Current State

### What's Been Implemented

1. **TrampolineFrame struct** - Saves VM state when making a call:
   - `ip`, `chunk`, `registers`, `this_value`
   - `vm_call_stack`, `try_stack`, `exception_value`
   - `saved_env_stack`, `arguments`, `new_target`
   - `pending_completion`, `return_register`
   - `saved_interp_env`, `register_guard`
   - `construct_new_obj` - For construct calls, the new object to fall back to
   - `is_async` - For async function calls, wrap result in Promise when returning

2. **OpResult::Call variant** - Signals a function call request:
   ```rust
   OpResult::Call {
       callee: JsValue,
       this_value: JsValue,
       args: Vec<JsValue>,
       return_register: Register,
       new_target: JsValue,
   }
   ```

3. **OpResult::Construct variant** - Signals a construct call request:
   ```rust
   OpResult::Construct {
       callee: JsValue,
       this_value: JsValue,
       args: Vec<JsValue>,
       return_register: Register,
       new_target: JsValue,
       new_obj: Gc<JsObject>,  // Already created new object
   }
   ```

4. **Trampoline handling in run()** - The main loop handles `OpResult::Call` and `OpResult::Construct`:
   - Calls `setup_trampoline_call()` or `setup_trampoline_construct()` to dispatch
   - For `JsFunction::Bytecode`, pushes frame and switches context
   - For other function types, falls back to recursive `interp.call_function()`
   - For construct, if constructor doesn't return object, uses the `new_obj` from frame

5. **Super resolution fix** - `__super_target__` is now bound in environment and looked up from there first.

6. **Depth counting fix** - Uses only `trampoline_stack.len()` for depth check, not double-counting.

### Current Issues

1. **Native function callbacks still recursive** - Array.map, Array.forEach, etc. (HIGH IMPACT)

2. **Decorator calls still recursive** - Complex multi-step logic in decorator opcodes. (LOW IMPACT)

3. **Proxy trap calls still recursive** - All proxy traps call user handlers. (MEDIUM IMPACT)

## Migration Plan

### Phase 1: Fix Depth Counting ✅ COMPLETED

**Problem**: Depth is double-counted because we push to both `interp.call_stack` (for stack traces) and `trampoline_stack`.

**Solution**: Use only `trampoline_stack.len()` for depth check:

```rust
// In setup_trampoline_call:
let total_depth = self.trampoline_stack.len();
if interp.max_call_depth > 0 && total_depth >= interp.max_call_depth {
    return Err(JsError::range_error(...));
}
```

**Status**: Implemented in commit 6b79e92.

### Phase 2: Migrate Construct Opcodes ✅ COMPLETED

**Problem**: `Op::Construct` and `Op::ConstructSpread` create a `this` object, then recursively call the constructor.

**Solution**: Added `OpResult::Construct` variant and `TrampolineFrame.construct_new_obj` field:

```rust
OpResult::Construct {
    callee: JsValue,
    this_value: JsValue,
    args: Vec<JsValue>,
    return_register: Register,
    new_target: JsValue,
    new_obj: Gc<JsObject>,  // Already created new object
}
```

The implementation:
1. `Op::Construct` and `Op::ConstructSpread` return `OpResult::Construct`
2. `run()` loop handles `OpResult::Construct` by calling `setup_trampoline_construct()`
3. `setup_trampoline_construct()` dispatches to bytecode or falls back to recursive call
4. `push_trampoline_frame_and_call_bytecode_construct()` stores `new_obj` in frame
5. `restore_from_trampoline_frame()` checks if constructor returned object; if not, uses `new_obj`

**Status**: Implemented. All 1796 tests pass.

### Phase 3: Migrate Native Function Callbacks (HIGH PRIORITY)

**Problem**: Many native functions call user callbacks, e.g., `Array.prototype.map`, `Array.prototype.forEach`, `Promise.then`.

**Impact Analysis**: These are the highest-impact recursive calls because:
- They execute in loops (e.g., `arr.map(fn)` calls `fn` for every element)
- Callback can itself call array methods, creating deep recursion
- Sort comparators are called O(n log n) times

**Call sites identified** (in `src/interpreter/builtins/`):
- `array.rs`: map, filter, forEach, reduce, reduceRight, every, some, find, findIndex, findLast, findLastIndex, sort, flatMap, toSorted
- `set.rs`: forEach
- `map.rs`: forEach
- `promise.rs`: then/catch callbacks
- `string.rs`: replace with function replacer

**Current code pattern** (in builtins/array.rs):
```rust
for (i, elem) in elements.iter().enumerate() {
    let result = interp.call_function(callback.clone(), this_arg.clone(), vec![...])?;
    // process result
}
```

**Solution Options**:

**Option A: State Machine Pattern** (Original plan)
1. Native function yields back to trampoline when it needs to call a callback
2. Trampoline invokes callback via normal call mechanism
3. Native function resumes with callback result
4. Requires: `OpResult::NativeCallback` variant, continuation state per builtin

**Option B: Inline Bytecode Generation**
1. Instead of Rust loops, generate synthetic bytecode for array iteration
2. Push bytecode onto trampoline stack
3. Native becomes thin wrapper that generates bytecode
4. Requires: bytecode generation in builtins, more complex than A

**Option C: Callback Invoker with VM Context**
1. Pass reference to current VM to native functions
2. Native uses VM's trampoline directly for callbacks
3. Requires: change all native function signatures (large refactor)

**Complexity Assessment** (2025-01-21):
- All options require significant architectural changes
- The current `interp.call_function()` creates a new BytecodeVM for each call
- This adds Rust stack frames even when JS depth is within limits
- The current implementation works for typical use cases (JS depth limit prevents runaway)
- Deep recursion *through* native callbacks (e.g., `arr.map(x => recurse(x))`) still uses Rust stack

**Current Status**: Deferred. The trampoline pattern for regular function calls (Phase 1, 2, 5) handles most stack-overflow scenarios. Native callbacks add stack depth but are bounded by JS depth limits. A full solution requires major refactoring that may not be worth the complexity.

**Workaround**: Users hitting stack issues with native callbacks can:
1. Reduce call depth with iterative solutions
2. Increase Rust stack size (if environment allows)
3. Break up array operations into smaller chunks

### Phase 4: Migrate Proxy Calls (MEDIUM PRIORITY)

**Problem**: Proxy traps call user-provided handler functions recursively.

**Impact Analysis**: Medium impact because:
- Proxies are less common than array operations
- Each property access/set on a proxy triggers a trap
- Nested proxies multiply the recursion

**Call sites identified** (in `src/interpreter/builtins/proxy.rs`):
- `get` trap
- `set` trap
- `has` trap
- `deleteProperty` trap
- `apply` trap (function call on proxy)
- `construct` trap
- `ownKeys`, `getOwnPropertyDescriptor`, `defineProperty`, etc.

**Current code** (in builtins/proxy.rs):
```rust
pub fn proxy_apply(...) -> Result<Guarded, JsError> {
    // ...
    interp.call_function(apply_trap, handler_val, vec![target_val, this_val, args_array])?
}
```

**Solution**: Similar to Phase 3 - proxy trap calls return `OpResult::ProxyTrap` with continuation state. The trampoline invokes the trap handler, then resumes proxy logic with the result.

**Complexity**: Medium - fewer state variations than array methods.

### Phase 5: Migrate Generator/Async Functions ✅ COMPLETED

**Problem**: Generator and async functions need special setup (create generator object, wrap in Promise) before running bytecode.

**Solution**: Split by function type:

1. **Generators (`BytecodeGenerator`)** ✅: Create generator object directly without recursion.
   When a generator function is called, it just creates a generator object that captures
   the bytecode, closure, and arguments. The body doesn't run until `.next()` is called.

2. **Async Generators (`BytecodeAsyncGenerator`)** ✅: Same as generators - create object directly.
   The body runs when `.next()` is called, returning Promises.

3. **Async Functions (`BytecodeAsync`)** ✅: Now use trampoline with `is_async` flag.
   The `TrampolineFrame.is_async` field tracks that this frame is for an async function.
   When the frame is popped:
   - On success: result is wrapped in a fulfilled Promise
   - On error: error is converted to a rejected Promise (no propagation)
   - Promise assimilation: if result is already a Promise, it's returned directly

**Implementation**:
```rust
JsFunction::BytecodeGenerator(bc_func) => {
    // Create generator state with args/this/closure
    let state = BytecodeGeneratorState { chunk, closure, args, this_value, ... };
    let gen_obj = create_bytecode_generator_object(interp, state);
    self.set_reg(return_register, JsValue::Object(gen_obj));
    Ok(())
}

JsFunction::BytecodeAsyncGenerator(bc_func) => {
    // Same as above, but with is_async: true
    let state = BytecodeGeneratorState { ..., is_async: true, ... };
    let gen_obj = create_bytecode_generator_object(interp, state);
    self.set_reg(return_register, JsValue::Object(gen_obj));
    Ok(())
}

JsFunction::BytecodeAsync(bc_func) => {
    // Use trampoline with is_async flag - result wrapped in Promise on return
    self.push_trampoline_frame_and_call_bytecode(
        interp, func_obj, bc_func, this_value, &args, return_register, new_target,
        true, // is_async - wrap result in Promise
    )?;
    Ok(())
}
```

**Status**: All generator and async function types now handled via trampoline without recursion.

**Files changed**: `src/interpreter/bytecode_vm.rs`

### Phase 6: Migrate Decorator Call Sites (LOW PRIORITY)

**Problem**: Decorators call user functions multiple times within `Op::ApplyClassDecorator`, `Op::ApplyMethodDecorator`, etc.

**Impact Analysis**: Low impact because:
- Decorators run once at class definition time (not in loops)
- Decorator functions typically don't cause deep recursion themselves
- The recursion depth from decorators is bounded by the number of decorators on a class

**Call sites identified** (in `src/interpreter/bytecode_vm.rs`):
- `Op::ApplyClassDecorator`: Single call per decorator
- `Op::ApplyMethodDecorator`: Single call per decorator
- `Op::ApplyFieldDecorator`: Single call per decorator
- `Op::ApplyParameterDecorator`: Single call per decorator
- `Op::ApplyFieldInitializer`: Single call per field
- `Op::RunClassInitializers`: Loop over initializer callbacks

**Current code pattern**:
```rust
// In ApplyMethodDecorator:
let result = interp.call_function(decorator_val, JsValue::Undefined, vec![method_val, ctx])?;
```

**Solution**: Add `OpResult::DecoratorCall` variant that returns to the trampoline loop, then resumes decorator logic with the result. Each decorator opcode becomes a state machine that can be suspended/resumed.

**Complexity**: Medium - each decorator opcode has simple control flow but needs state preservation.

**Note**: This phase can be deferred indefinitely since decorators don't cause the stack overflow issues that motivate the trampoline pattern.

## Implementation Order (Revised 2025-01-21)

**Completed:**
1. **Phase 1** ✅: Fix depth counting bug
2. **Phase 2** ✅: Migrate Construct opcodes
3. **Phase 5** ✅: All generators and async functions now use trampoline

**Deferred (requires major architectural changes):**
4. **Phase 3** (DEFERRED): Migrate native function callbacks
   - Requires coroutine-like state machines for all affected builtins
   - Current workaround: JS depth limit prevents runaway recursion
5. **Phase 4** (DEFERRED): Migrate proxy calls
   - Similar complexity to Phase 3
6. **Phase 6** (LOW PRIORITY): Migrate decorator call sites
   - Low impact, can be skipped entirely

**Summary**: The core trampoline pattern is complete for bytecode function calls. The remaining phases would eliminate Rust stack growth for native callbacks but require substantial refactoring that may not be worth the effort given the JS depth limit workaround.

## Testing Strategy

After each phase:
1. Run full test suite: `timeout 60 cargo test`
2. Run specific tests:
   - `cargo test test_super_deep_inheritance` - Multi-level super calls
   - `cargo test test_call_stack_depth_limit` - Depth limiting
   - `cargo test test_infinite_recursion_caught` - Recursion detection
3. Run stress test with high recursion:
   ```rust
   #[test]
   fn test_deep_recursion() {
       let mut runtime = Runtime::new();
       runtime.set_max_call_depth(1000);
       let result = runtime.eval_simple(r#"
           function fib(n) {
               if (n <= 1) return n;
               return fib(n-1) + fib(n-2);
           }
           fib(20)
       "#);
       assert_eq!(result.unwrap(), JsValue::Number(6765.0));
   }
   ```

## Architecture Notes

### Why Trampoline vs Stackless VM

A fully stackless VM would require:
- Continuation-passing style for all operations
- Explicit stack for expression evaluation
- Much larger refactor

Trampoline is simpler:
- Only function calls need modification
- Expression evaluation stays the same
- Incremental migration possible

### Guard/GC Considerations

When saving a trampoline frame:
1. The frame's `register_guard` keeps its values alive
2. The new frame gets a fresh guard
3. On restore, we swap guards back

Important: Don't drop the old guard until the frame is popped!

### Error Propagation

When an error occurs in a trampolined call:
1. Check for exception handler in current frame
2. If none, pop trampoline frame and check its handlers
3. Repeat until handler found or stack exhausted
4. Restore environment at each pop

Current implementation handles this in the `Err(e)` branch of the run() loop.

## Files Reference

- `src/interpreter/bytecode_vm.rs` - Main trampoline implementation
- `src/interpreter/mod.rs` - Interpreter with `call_function` methods
- `src/value.rs` - `TrampolineFrame`, `OpResult` definitions
- `tests/interpreter/class.rs` - Super inheritance tests
- `tests/interpreter/function.rs` - Depth limit tests
