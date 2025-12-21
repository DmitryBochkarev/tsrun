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

1. **Generators/async still recursive** - `JsFunction::BytecodeGenerator`, `BytecodeAsync`, `BytecodeAsyncGenerator` fall back to recursive calls.

2. **Decorator calls still recursive** - Complex multi-step logic in decorator opcodes.

3. **Native function callbacks still recursive** - Array.map, Array.forEach, etc.

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

### Phase 3: Migrate Decorator Call Sites

**Problem**: Decorators call user functions multiple times within `Op::ApplyClassDecorator`, `Op::ApplyMethodDecorator`, etc.

**Current code pattern**:
```rust
// In ApplyMethodDecorator:
let result = interp.call_function(decorator_val, JsValue::Undefined, vec![method_val, ctx])?;
```

**Solution**: These are more complex because they have multi-step logic. Options:
1. Keep recursive for now (decorators rarely cause deep recursion)
2. Add multi-call continuation state in trampoline frame

**Recommendation**: Keep recursive for Phase 3, revisit if needed.

### Phase 4: Migrate Native Function Callbacks

**Problem**: Many native functions call user callbacks, e.g., `Array.prototype.map`, `Array.prototype.forEach`, `Promise.then`.

**Current code pattern** (in builtins/array.rs):
```rust
for (i, elem) in elements.iter().enumerate() {
    let result = interp.call_function(callback.clone(), this_arg.clone(), vec![...])?;
    // process result
}
```

**Solution**: These are tricky because:
1. They're in a loop with state between calls
2. They need the result of each call before continuing

**Options**:
1. **Keep recursive** - Native callbacks are usually shallow
2. **Iterator-based trampoline** - Add `OpResult::NativeCallback` that saves loop state
3. **Compile to bytecode** - Make native functions return bytecode that the VM executes

**Recommendation**: Keep recursive for now. The main recursion problem is JS→JS calls, not JS→Native→JS.

### Phase 5: Migrate Generator/Async Functions

**Problem**: Generator and async functions need special setup (create generator object, wrap in Promise) before running bytecode.

**Current handling**:
```rust
JsFunction::BytecodeGenerator(_) | JsFunction::BytecodeAsync(_) | ... => {
    // Falls back to interp.call_function_with_new_target()
}
```

**Solution**: Split into two parts:
1. Setup (create generator/promise) - done immediately
2. Running the body - use trampoline

For generators:
```rust
JsFunction::BytecodeGenerator(bc_func) => {
    // Create generator object (doesn't run body yet)
    let gen = interp.create_generator_object(bc_func, this_value, args)?;
    self.set_reg(return_register, JsValue::Object(gen));
    Ok(())
}
```

For async:
```rust
JsFunction::BytecodeAsync(bc_func) => {
    // Create promise
    let (promise, resolve, reject) = interp.create_promise_with_resolvers()?;
    // Push trampoline frame that will resolve/reject promise on completion
    self.push_async_trampoline_frame(...)?;
    Ok(())
}
```

**Files to change**: `src/interpreter/bytecode_vm.rs`, `src/interpreter/mod.rs`

### Phase 6: Migrate Proxy Calls

**Problem**: Proxy traps call user-provided handler functions recursively.

**Current code** (in builtins/proxy.rs):
```rust
pub fn proxy_apply(...) -> Result<Guarded, JsError> {
    // ...
    interp.call_function(apply_trap, handler_val, vec![target_val, this_val, args_array])?
}
```

**Solution**: Proxy calls are already rare and shallow. Keep recursive.

## Implementation Order

1. **Phase 1** ✅ COMPLETED: Fix depth counting bug
2. **Phase 2** ✅ COMPLETED: Migrate Construct opcodes
3. **Phase 5** (Medium priority): Migrate async functions (if async-in-async causes issues)
4. **Phase 3, 4, 6** (Low priority): Keep recursive unless problems arise

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
