# Missing Bytecode VM Features

This document analyzes test failures in the bytecode VM and categorizes them by feature area with implementation guidance.

**Total Tests:** 1764
**Passing:** 1527
**Failing:** 230
**Ignored:** 7

---

## Table of Contents

1. [Module System (Import/Export)](#1-module-system-importexport)
2. [Async Iteration (for-await-of)](#2-async-iteration-for-await-of)
3. [Private Class Members](#3-private-class-members)
4. [Proxy Handler Traps](#4-proxy-handler-traps)
5. [Decorators](#5-decorators)
6. [BigInt](#6-bigint)
7. [Static Class Features](#7-static-class-features)
8. [Generator Edge Cases](#8-generator-edge-cases)
9. [Eval Scope Handling](#9-eval-scope-handling)
10. [Closure Scoping in Loops](#10-closure-scoping-in-loops)
11. [Optional Chaining with Parentheses](#11-optional-chaining-with-parentheses)
12. [Super in Static Methods](#12-super-in-static-methods)
13. [Miscellaneous Issues](#13-miscellaneous-issues)

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

**Affected Tests (~40 tests):**
- `proxy::test_proxy_get_trap*`
- `proxy::test_proxy_set_trap*`
- `proxy::test_proxy_has_trap*`
- `proxy::test_proxy_delete_property_trap`
- `proxy::test_proxy_construct_trap*`
- `proxy::test_proxy_revocable*`
- `proxy::test_nested_proxies`
- `proxy::test_reflect_*` (several)
- `function::test_proxied_function_*`

**Error Patterns:**
- Get trap not being invoked
- Receiver not passed correctly to traps
- Nested proxy chains not working
- Revocable proxies not revoking

**Current State:**
Basic proxy creation works, but trap invocation is incomplete.

**Implementation Strategy:**

### Step 1: Fix Get Trap Invocation
The VM's property access needs to check for proxy:
```rust
fn get_property(&mut self, obj: JsValue, key: PropertyKey) -> Result<JsValue> {
    if let Some(proxy) = obj.as_proxy() {
        if let Some(get_trap) = proxy.handler.get("get") {
            // Call trap with (target, property, receiver)
            return self.call_function(get_trap, proxy.handler, vec![
                proxy.target.into(),
                key.to_value(),
                obj.into(), // receiver
            ]);
        }
    }
    // Fall through to normal property access
}
```

### Step 2: Fix Set Trap
Similar pattern for [[Set]]:
```rust
fn set_property(&mut self, obj: JsValue, key: PropertyKey, value: JsValue) -> Result<bool> {
    if let Some(proxy) = obj.as_proxy() {
        if let Some(set_trap) = proxy.handler.get("set") {
            let result = self.call_function(set_trap, proxy.handler, vec![
                proxy.target.into(),
                key.to_value(),
                value,
                obj.into(), // receiver
            ])?;
            return Ok(result.to_boolean());
        }
    }
    // Normal set
}
```

### Step 3: Revocable Proxies
Need to track revocation state:
```rust
struct ProxyData {
    target: Option<Gc<JsObject>>, // None if revoked
    handler: Option<Gc<JsObject>>, // None if revoked
    revoked: bool,
}
```

### Step 4: Invariant Checking
Some traps have invariants that must be enforced (e.g., get trap for non-configurable property).

**Complexity:** Medium-High
**Estimated Effort:** 2 days

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

**Error Message:** `BigInt is not supported`

**Affected Tests (3 tests):**
- `basics::test_bigint_literal`
- `basics::test_bigint_arithmetic`
- `basics::test_bigint_variable`

**Current State:**
The parser recognizes BigInt literals, but the VM throws TypeError.

**Implementation Strategy:**

### Step 1: Add BigInt Value Type
```rust
enum JsValue {
    // ... existing
    BigInt(BigInt), // Use num-bigint crate
}
```

### Step 2: Arithmetic Operations
BigInt operations:
- Cannot mix BigInt with Number
- Division is integer division
- No bitwise with Number

### Step 3: Comparison
BigInt can compare with Number:
```javascript
1n < 2 // true
1n == 1 // true
1n === 1 // false
```

### Step 4: Bytecode Changes
May need BigInt-specific opcodes or type checking in existing ops.

**Complexity:** Medium
**Estimated Effort:** 1 day
**Note:** Consider using `num-bigint` crate or similar.

---

## 7. Static Class Features

**Affected Tests (~8 tests):**
- `class::test_static_initialization_block` - returns 0 instead of 42
- `class::test_static_block_complex`
- `class::test_super_in_static_method` - super not valid in static context
- `class::test_super_in_static_getter`
- `class::test_super_in_static_setter`
- `class::test_super_computed_property`

**Error Patterns:**
1. Static blocks execute but don't modify static fields
2. `super` keyword not recognized in static methods

**Implementation Strategy:**

### Static Initialization Blocks
Currently returning 0 instead of expected value. The static block runs but field assignment may not work:
```javascript
class C {
    static value = 0;
    static { this.value = 42; }
}
```

**Fix:** Ensure `this` in static block refers to the class constructor.

### Super in Static Methods
Error: `'super' keyword is only valid inside a class method`

**Fix:** Static methods should also have access to super, referring to parent constructor:
```rust
// When compiling static method:
fn compile_static_method() {
    // Set up super reference to parent class constructor
    // Not parent prototype
}
```

**Complexity:** Medium
**Estimated Effort:** 1 day

---

## 8. Generator Edge Cases

**Affected Tests (~5 tests):**
- `generator::test_generator_throw_with_catch` - Error not caught inside generator
- `generator::test_yield_star_array` - Invalid array iterator
- `generator::test_generator_yield_star_with_array`

**Error Patterns:**

### Generator.throw()
Error: `RuntimeError: test` (not caught)

The generator's internal try/catch isn't catching errors injected via `.throw()`:
```javascript
function* gen() {
    try { yield 1; }
    catch (e) { yield "caught: " + e; }
}
const g = gen();
g.next();
g.throw("test"); // Should yield "caught: test", but throws instead
```

**Fix:** When processing `.throw()`, resume generator in catch block if one exists.

### yield* with Array
Error: `Invalid array iterator`

```javascript
function* gen() { yield* [1, 2, 3]; }
```

**Fix:** Ensure `yield*` correctly gets `Symbol.iterator` from array and iterates.

**Complexity:** Medium
**Estimated Effort:** 1 day

---

## 9. Eval Scope Handling

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

## 10. Closure Scoping in Loops

**Affected Tests (2 tests):**
- `control_flow::test_closure_capturing_loop_variable` - Returns "3,3,3" instead of "0,1,2"
- `control_flow::test_for_let_block_scope`

**Error Pattern:**
```javascript
let fns = [];
for (let i = 0; i < 3; i++) {
    fns.push(() => i);
}
fns.map(f => f()).join(",") // Expected: "0,1,2", Got: "3,3,3"
```

**Current State:**
The `let` binding in for-loop is not creating a new binding per iteration.

**Implementation Strategy:**

Per ES6, each iteration of `for (let i...)` creates a fresh `i` binding:
```rust
fn compile_for_with_let(&mut self, init: &VarDecl, ...) {
    // For each iteration:
    // 1. Create new block scope
    // 2. Copy previous i value
    // 3. Execute body
    // 4. Update copied binding
}
```

**Bytecode Approach:**
```
ForLetStart:
  CreateBlockScope        ; New scope for this iteration
  CopyBinding i          ; Copy i from previous iteration's scope
  ... body ...
  UpdateBinding i        ; Before next iteration
  PopScope
  Jump ForLetStart
```

**Complexity:** Medium
**Estimated Effort:** 0.5-1 day

---

## 11. Optional Chaining with Parentheses

**Affected Tests (3 tests):**
- `control_flow::test_optional_chain_double_optional_parenthesized`
- `control_flow::test_optional_call_preserves_this_parenthesized`
- `control_flow::test_optional_call_on_method_parenthesized`

**Error Pattern:**
```javascript
(obj?.method)?.() // TypeError: Cannot read properties of undefined
```

**Current State:**
Parenthesized optional chains don't preserve the short-circuit correctly.

**Implementation Strategy:**
The issue is that `(obj?.method)` returns `undefined` correctly, but the outer `?.()` then tries to access properties of the parenthesized result incorrectly.

Need to track that the entire parenthesized expression should short-circuit:
```rust
// When evaluating (a?.b)?.c:
// If a?.b short-circuits to undefined,
// the outer ?.c should also return undefined
```

**Complexity:** Low-Medium
**Estimated Effort:** 0.5 day

---

## 12. Super in Static Methods

**Error Message:** `'super' keyword is only valid inside a class method`

**Affected Tests:**
- `class::test_super_in_static_method`
- `class::test_super_in_static_getter`
- `class::test_super_in_static_setter`
- `class::test_super_property_assignment`

**Current State:**
Compiler rejects `super` in static methods, but it should work.

**Implementation Strategy:**

In static methods, `super` refers to the parent class (constructor), not prototype:
```javascript
class Parent { static greet() { return "hi"; } }
class Child extends Parent {
    static greet() { return super.greet() + "!"; } // Should work
}
```

**Fix in Compiler:**
```rust
fn is_valid_super_context(&self) -> bool {
    self.in_method || self.in_static_method || self.in_constructor
}

fn get_super_base(&self) -> SuperBase {
    if self.in_static_method {
        SuperBase::ParentConstructor
    } else {
        SuperBase::ParentPrototype
    }
}
```

**Complexity:** Low-Medium
**Estimated Effort:** 0.5 day

---

## 13. Miscellaneous Issues

### Function Constructor Rest Params
**Tests:** `function::test_function_constructor_rest_params*`
**Error:** Returns undefined, rest params not working in `new Function("...args", "...")`

### Symbol.hasInstance
**Tests:** `function::test_instanceof_uses_symbol_hasinstance`, `test_symbol_hasinstance_direct_call`
**Error:** Custom `[Symbol.hasInstance]` not consulted during `instanceof`

### Argument Evaluation Order
**Tests:** `function::test_call_args_not_evaluated_when_callee_throws`
**Error:** Arguments evaluated even when callee expression throws

### GC/Memory Leak
**Tests:** `gc::test_nested_for_loop_environments_collected`
**Error:** Loop environments not being collected properly

### Reflect.construct with newTarget
**Tests:** `proxy::test_reflect_construct_with_new_target`
**Error:** Third argument to `Reflect.construct` not handled

---

## Implementation Priority

### High Priority (Core Language Features)
1. **Module System** - Essential for real-world usage
2. **Private Class Members** - Required for modern JS/TS
3. **Closure Scoping in Loops** - Common pattern, semantics bug

### Medium Priority
4. **Async Iteration** - Needed for async patterns
5. **Proxy Traps** - Important for metaprogramming
6. **Static Class Features** - Class-based code correctness
7. **Super in Static Methods** - Related to static features

### Lower Priority
8. **Decorators** - TypeScript feature, complex
9. **BigInt** - Less common usage
10. **Generator Edge Cases** - Uncommon patterns
11. **Eval Scoping** - Edge case for direct eval

---

## Quick Wins

These can likely be fixed quickly:

1. **Super in static methods** - Just expand the valid context check
2. **Closure scoping in loops** - Well-understood pattern
3. **Optional chaining with parentheses** - Small fix in short-circuit logic
4. **Static initialization blocks** - Likely just `this` binding issue
5. **Argument evaluation order** - Evaluate callee first before args
