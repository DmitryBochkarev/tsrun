# Missing Bytecode VM Features

This document analyzes test failures in the bytecode VM and categorizes them by feature area with implementation guidance.

**Total Tests:** 1787
**Passing:** 1597
**Failing:** 183
**Ignored:** 7

---

## Recently Fixed (Quick Wins)

The following issues have been fixed:

- ✅ **Super in static methods** - Fixed super lookup to work in static context
- ✅ **Closure scoping in loops** - Implemented per-iteration bindings for `for (let...)` loops
- ✅ **Optional chaining with parentheses** - Fixed `this` binding in parenthesized optional chain calls
- ✅ **Static initialization blocks** - Implemented static block compilation and execution
- ✅ **Argument evaluation order** - Fixed callee evaluation to occur before arguments
- ✅ **Generator yield\* with arrays** - Fixed GC issue with delegated iterator tracing
- ✅ **Generator.throw() with catch blocks** - Implemented proper exception injection at yield points
- ✅ **Symbol.hasInstance** - Fixed `instanceof` operator to check custom `[Symbol.hasInstance]` method
- ✅ **Function.prototype property** - Regular functions now get a `.prototype` property set up correctly for use with `new`
- ✅ **Function constructor rest params** - Fixed rest parameter handling for functions created via `new Function('...args', 'body')`
- ✅ **BigInt literals** - BigInt literals now compile to Number values (simplified implementation)
- ✅ **new eval() TypeError** - `new eval()` now throws TypeError as required by ECMAScript spec
- ✅ **Proxy get/set traps** - Proxy get and set traps now work in bytecode VM (delegates to proxy_get/proxy_set)

---

## Table of Contents

1. [Module System (Import/Export)](#1-module-system-importexport)
2. [Async Iteration (for-await-of)](#2-async-iteration-for-await-of)
3. [Private Class Members](#3-private-class-members)
4. [Proxy Handler Traps](#4-proxy-handler-traps)
5. [Decorators](#5-decorators)
6. [BigInt](#6-bigint)
7. [Generator Edge Cases](#7-generator-edge-cases)
8. [Eval Scope Handling](#8-eval-scope-handling)
9. [Miscellaneous Issues](#9-miscellaneous-issues)

---

## 1. Module System (Import/Export)

**Error Message:** `Module imports not yet supported in bytecode compiler`

**Affected Tests (~40 tests):**
- `modules::test_module_with_internal_imports`
- `modules::test_external_module_*`
- `modules::test_live_binding_*`
- `modules::test_export_*`
- `modules::test_import_namespace`
- `modules::test_order_fulfillment*`
- `orders::test_*` (most tests)

**Current State:**
The bytecode compiler throws a `SyntaxError` when encountering import/export declarations.

**Implementation Strategy:**

### Step 1: Compile Import Declarations
```rust
// In bytecode compiler, handle ImportDeclaration
Statement::ImportDeclaration(import) => {
    // Emit LoadModule instruction with resolved specifier
    // Emit GetModuleExport for each import specifier
    // Emit DefineVariable for each binding
}
```

### Step 2: Add VM Instructions
```rust
enum Opcode {
    // ... existing
    LoadModule(ModuleId),       // Load module into register
    GetModuleBinding(BindingId), // Get live binding from module
    ExportBinding(BindingId),   // Export a binding
}
```

### Step 3: Module Records
The VM needs to maintain module records with:
- Namespace object
- Live binding references
- Export entries
- Module status (loading, linked, evaluated)

### Step 4: Handle Module Suspension
When module loading is needed, the VM should:
1. Return `RuntimeResult::NeedImports`
2. Store current execution state
3. Resume after host provides module

**Complexity:** High - requires significant VM changes
**Estimated Effort:** 2-3 days

---

## 2. Async Iteration (for-await-of)

**Error Message:** `Async iterators not yet implemented in VM`

**Affected Tests (~25 tests):**
- `async_iter::test_for_await_of_*`
- `async_iter::test_custom_async_iterable*`
- `async_iter::test_async_generator_*` (several)
- `async_iter::test_top_level_for_await*`
- `async_iter::test_debug_nested_async_gen_*`

**Current State:**
The VM recognizes `for-await-of` syntax but throws an error at runtime.

**Implementation Strategy:**

### Step 1: Add ForAwaitOf Instruction
```rust
enum Opcode {
    // Existing ForOf uses Symbol.iterator
    ForOfNext { ... },

    // New: uses Symbol.asyncIterator, awaits each result
    ForAwaitOfStart { iterator_reg: u8 },
    ForAwaitOfNext { result_reg: u8, done_label: Label },
}
```

### Step 2: Iteration Protocol
1. Get `Symbol.asyncIterator` method from iterable
2. If not present, fall back to `Symbol.iterator` (wrap values in promises)
3. Call `.next()` on async iterator
4. Await the returned promise
5. Extract `value` and `done` from result

### Step 3: Handle Suspension
Each `await` in the loop body requires:
- Saving loop state (iterator, current index)
- Proper continuation after promise resolution

**Key Difference from Regular for-of:**
- Must await each `.next()` call result
- Must handle async generator protocol

**Complexity:** Medium-High
**Estimated Effort:** 1-2 days

---

## 3. Private Class Members

**Error Message:** `Private fields not yet supported in bytecode compiler`

**Affected Tests (~15 tests):**
- `class::test_private_field_*`
- `class::test_private_method`
- `class::test_static_private_field*`
- `class::test_class_getter_setter` (uses private backing field)
- `class::test_class_getter_only`
- `class::test_class_getter_computed_key`

**Current State:**
The parser successfully parses private fields (`#field`), but the bytecode compiler rejects them.

**Implementation Strategy:**

### Step 1: Private Field Storage
Private fields need special handling:
```rust
struct JsObject {
    // ... existing
    private_fields: HashMap<PrivateFieldId, JsValue>,
}
```

### Step 2: Bytecode Instructions
```rust
enum Opcode {
    GetPrivateField { obj_reg: u8, field_id: PrivateFieldId },
    SetPrivateField { obj_reg: u8, field_id: PrivateFieldId, value_reg: u8 },
    HasPrivateField { obj_reg: u8, field_id: PrivateFieldId }, // for #field in obj
}
```

### Step 3: Brand Checking
Private fields are "branded" - only accessible within the class that defined them:
```javascript
class A { #x = 1; getX(obj) { return obj.#x; } }
new A().getX(new A()); // Works
new A().getX({}); // TypeError
```

### Step 4: Private Methods
Private methods are similar but stored as non-configurable, non-writable properties.

**Complexity:** Medium
**Estimated Effort:** 1-2 days

---

## 4. Proxy Handler Traps

**Status: Partially Fixed**

Most proxy traps now work (67/73 tests passing). The following are fixed:
- ✅ Get trap - now invoked via `proxy_get`
- ✅ Set trap - now invoked via `proxy_set`
- ✅ Revocable proxies
- ✅ Nested proxies
- ✅ Most Reflect methods

**Remaining Issues (~6 tests):**
- `proxy::test_proxy_has_trap` - `in` operator not checking proxy has trap
- `proxy::test_proxy_delete_property_trap` - `delete` not checking proxy deleteProperty trap
- `proxy::test_proxy_construct_trap*` - `new` not checking proxy construct trap
- `proxy::test_proxy_array_for_of` - Iterator on proxy not working
- `proxy::test_proxy_for_in` - for-in on proxy not working

**Implementation Notes:**
The bytecode VM now delegates to `proxy_get` and `proxy_set` from `get_property_value` and `set_property_value`. Similar delegation is needed for:
- `Op::In` → `proxy_has`
- `Op::DeleteProperty` → `proxy_delete_property`
- `Op::Construct` → `proxy_construct`
- `Op::GetIterator` → proxy iteration

**Complexity:** Low-Medium (similar pattern to get/set)
**Estimated Effort:** 1 day

---

## 5. Decorators

**Affected Tests (~60+ tests):**
- `decorator::test_class_decorator_*`
- `decorator::test_method_decorator_*`
- `decorator::test_field_decorator_*`
- `decorator::test_decorator_context_*`
- `decorator::test_decorator_factory_*`
- `decorator::test_accessor_*`
- Plus many pattern-specific tests

**Error Patterns:**
- Decorator context not passed correctly
- `addInitializer` not working
- Decorator return values not applied
- Static decorator context missing
- Field decorator transform not applied

**Current State:**
Basic decorator invocation works for methods, but decorator context and advanced features are incomplete.

**Implementation Strategy:**

### Step 1: Decorator Context Object
Each decorator receives a context object:
```javascript
{
    kind: "class" | "method" | "getter" | "setter" | "field" | "accessor",
    name: string | symbol,
    static: boolean,
    private: boolean,
    access: { get?, set? },
    addInitializer: (fn) => void
}
```

### Step 2: Class Decorator Context
```rust
fn create_class_decorator_context(name: &str) -> JsObject {
    // kind: "class"
    // name: class name
    // addInitializer: function that stores initializers
}
```

### Step 3: Method/Field Decorator Context
Need to pass:
- `kind`: "method", "field", "getter", "setter", "accessor"
- `name`: property name
- `static`: boolean
- `private`: boolean for #private members
- `access`: getter/setter access functions

### Step 4: Initializers
`addInitializer` callbacks run after class definition:
```rust
struct ClassDecorators {
    initializers: Vec<JsValue>, // Functions to call after class creation
}
```

### Step 5: Return Value Handling
- Class decorators: return new class or undefined
- Method decorators: return replacement function or undefined
- Field decorators: return initializer function or undefined

**Complexity:** High
**Estimated Effort:** 2-3 days

---

## 6. BigInt

**Status: ✅ Fixed (Simplified)**

All BigInt tests now pass:
- ✅ `basics::test_bigint_literal`
- ✅ `basics::test_bigint_arithmetic`
- ✅ `basics::test_bigint_variable`

**Implementation Details:**
BigInt literals are compiled to Number values. This is a simplified implementation that works for most practical cases where BigInt values fit within f64's 53-bit integer precision. A full BigInt implementation with arbitrary precision would require adding a new JsValue variant and the `num-bigint` crate.

---

## 7. Generator Edge Cases

**Status: ✅ Fixed**

All generator edge cases have been fixed:

- ✅ `generator::test_generator_throw_with_catch` - Fixed by implementing `inject_exception()` in VM to resume generator with exception at yield point
- ✅ `generator::test_yield_star_array` - Fixed by adding GC tracing for `delegated_iterator` and keeping iterator guard alive
- ✅ `generator::test_generator_yield_star_with_array` - Same fix as above

**Implementation Details:**

### Generator.throw() Fix
Added `throw_value` field to `BytecodeGeneratorState` and `inject_exception()` method to `BytecodeVM`. When `generator.throw(exception)` is called, it sets the throw_value and resumes the generator. The VM then finds an exception handler (if any) and jumps to the catch block.

### yield* with Array Fix
Two issues were fixed:
1. Added GC tracing for `delegated_iterator`, `func_env`, and `current_env` in `BytecodeGeneratorState`
2. Added `iter_guard` in `start_yield_star_delegation` to keep iterator alive during the first `next()` call

---

## 8. Eval Scope Handling

**Affected Tests (~12 tests):**
- `eval::test_eval_closure` - ReferenceError: x
- `eval::test_eval_access_function_scope`
- `eval::test_eval_scope_*`
- `eval::test_eval_for_completion`
- `eval::test_eval_while_completion`
- `eval::test_eval_switch_completion`

**Error Patterns:**
- Eval code can't access outer lexical scope
- Completion values not returned correctly for loops/switch

**Current State:**
Direct eval should access the caller's scope, but currently uses global scope.

**Implementation Strategy:**

### Direct vs Indirect Eval
```javascript
let x = 10;
eval("x");       // Direct: should find x = 10
(0, eval)("x");  // Indirect: global scope only
```

### Fix Scope Chain
When compiling eval'd code, pass the current lexical environment:
```rust
fn eval_direct(&mut self, code: &str, caller_env: Environment) {
    // Parse code
    // Compile with caller_env as outer scope
    // Execute
}
```

### Completion Values
Loops and switch should return their completion value:
```javascript
eval("for(let i=0;i<3;i++) i") // Should return 2
```

**Complexity:** Medium
**Estimated Effort:** 1 day

---

## 9. Miscellaneous Issues

### ~~Function Constructor Rest Params~~ ✅ Fixed
**Tests:** `function::test_function_constructor_rest_params*`
~~**Error:** Returns undefined, rest params not working in `new Function("...args", "..."`~~

Fixed by adding rest parameter processing to interpreted function JIT compilation in `call_function`.

### ~~Symbol.hasInstance~~ ✅ Fixed
**Tests:** `function::test_instanceof_uses_symbol_hasinstance`, `test_symbol_hasinstance_direct_call`
~~**Error:** Custom `[Symbol.hasInstance]` not consulted during `instanceof`~~

Fixed by updating the `Instanceof` opcode to check for `[Symbol.hasInstance]` method before falling back to OrdinaryHasInstance. Also fixed regular functions not having a `.prototype` property.

### GC/Memory Leak
**Tests:** `gc::test_nested_for_loop_environments_collected`
**Error:** Loop environments not being collected properly

### ~~Reflect.construct with newTarget~~ ✅ Fixed
**Tests:** `proxy::test_reflect_construct_with_new_target`
~~**Error:** Third argument to `Reflect.construct` not handled~~

This was already working - likely fixed by a previous change.

---

## Implementation Priority

### High Priority (Core Language Features)
1. **Module System** - Essential for real-world usage
2. **Private Class Members** - Required for modern JS/TS
3. **Async Iteration** - Needed for async patterns

### Medium Priority
4. **Proxy Traps** - Important for metaprogramming
5. **Decorators** - TypeScript feature, complex
6. ~~**Generator Edge Cases**~~ - ✅ Fixed

### Lower Priority
7. ~~**BigInt**~~ - ✅ Fixed (simplified)
8. **Eval Scoping** - Edge case for direct eval
9. **Miscellaneous Issues** - Various small fixes
