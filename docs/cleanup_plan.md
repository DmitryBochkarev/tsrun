# AST Interpreter Cleanup Plan - COMPLETED

This document outlined the plan to remove legacy AST interpreter code from `src/interpreter/mod.rs` and complete the migration to the bytecode VM.

## Status: COMPLETE ✓

All phases have been successfully completed. The AST interpreter code has been fully removed and the bytecode VM is now the sole execution path.

## Summary of Changes

### Completed Phases

1. **Phase 1: Migrate Function Constructor to Bytecode** ✓
   - `new Function("code")` now creates `JsFunction::Bytecode` instead of `JsFunction::Interpreted`

2. **Phase 2: Remove JsFunction::Interpreted from Call Path** ✓
   - Removed the `JsFunction::Interpreted` branch from `call_function_with_new_target`
   - Simplified bytecode VM to only handle bytecode, native, bound, generator, and async paths

3. **Phase 3: Remove AST Class Creation** ✓
   - Removed `create_class_constructor` method
   - Removed `create_class_from_expression` method
   - Removed `evaluate_decorators` method

4. **Phase 4: Remove AST Expression Evaluation** ✓
   - Removed `evaluate_expression` and all `evaluate_*` methods (~3,500 lines)
   - Removed helper methods like `evaluate_callee_with_this`, `execute_direct_eval`

5. **Phase 5: Remove Pattern Binding** ✓
   - Removed `bind_pattern` method
   - Removed `bind_array_pattern` method
   - Removed `assign_pattern` method

6. **Phase 6: Remove Variable Hoisting** ✓
   - Removed `hoist_var_declarations` method
   - Removed `hoist_var_in_statement` method
   - Removed `hoist_pattern_names` method
   - Removed `env_has_own_binding` method
   - Bytecode compiler handles hoisting via `compiler/hoist.rs`

7. **Phase 7: Remove InterpretedFunction Type** ✓
   - Removed `InterpretedFunction` struct from `value.rs`
   - Removed `JsFunction::Interpreted` variant
   - Removed `create_interpreted_function` method

### Code Reduction

| Category | Lines Removed |
|----------|---------------|
| `evaluate_*` methods | ~3,500 |
| Class creation methods | ~600 |
| Pattern binding methods | ~250 |
| Variable hoisting | ~150 |
| `InterpretedFunction` handling | ~200 |
| **Total** | **~4,700 lines** |

### Result

- `mod.rs` reduced from ~8,000 lines to ~3,400 lines
- All 2000+ tests pass
- Bytecode VM is the sole execution path
- No `evaluate_*` methods remain
- No `JsFunction::Interpreted` references remain

## Architecture After Cleanup

```
Source → Lexer → Parser → AST → Compiler → Bytecode → BytecodeVM → Result
```

The interpreter now has a clean pipeline:
1. **Parser** produces AST
2. **Compiler** (`src/compiler/`) converts AST to bytecode
3. **BytecodeVM** (`src/interpreter/bytecode_vm.rs`) executes bytecode

All runtime execution goes through the bytecode VM. The interpreter module (`mod.rs`) now only contains:
- Runtime initialization and built-in setup
- Environment management
- Module resolution
- Helper functions for built-ins
- No AST evaluation code
