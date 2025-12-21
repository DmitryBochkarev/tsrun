# Class Implementation Fix Plan

This document outlines the missing class features and provides a prioritized fix plan based on Test262 conformance testing.

## Current Status

**Test262 Class Expression Results:**
- Total tests: 4,059
- Pass rate (with all features): 49.0%
- Pass rate (excluding private fields): 57.3%

**Major Categories of Failures:**
| Issue | ~Failures | Priority |
|-------|-----------|----------|
| Private class members (#field, #method) | 714 | P1 - High (common in modern code) |
| Method enumerability wrong | 140 | P1 - High (easy fix) |
| Function name inference missing | 80+ | P2 - Medium |
| Class fields (public) issues | ~100 | P2 - Medium |
| ToPrimitive coercion in computed keys | ~87 | P3 - Low (edge case) |
| TDZ violations not caught | ~34 | P2 - Medium |
| Parser accepting invalid syntax | ~72 | P3 - Low |
| Unicode escapes in identifiers | ~50 | P3 - Low |

---

## P1 (High Priority) Fixes

### 1. Class Methods Should Be Non-Enumerable

**Problem:** Class methods are currently defined with `enumerable: true` instead of `enumerable: false`.

**Test Failures:** ~140 tests with error "obj['m'] descriptor should not be enumerable"

**Root Cause:** `src/interpreter/bytecode_vm.rs:3587`
```rust
// Current code - uses set_property which defaults to enumerable=true
proto.borrow_mut().set_property(prop_key, method_val);
```

**Fix Location:** `src/interpreter/bytecode_vm.rs` - `Op::DefineMethod` handler (line ~3531)

**Solution:**
```rust
// Use define_property with correct attributes
let method_prop = Property::with_attributes(
    method_val,
    true,   // writable
    false,  // enumerable (methods should NOT be enumerable)
    true,   // configurable
);
proto.borrow_mut().define_property(prop_key, method_prop);
```

**Also update:**
- `Op::DefineMethodComputed` handler (line ~3662)
- `Op::DefineAccessor` handler (line ~3594)
- `Op::DefineAccessorComputed` handler

**Estimated Effort:** Small - single file change

---

### 2. Private Class Members

**Problem:** Private fields and methods (#identifier) are parsed but not fully implemented.

**Test Failures:** 714 tests related to private members

**Current Implementation Status:**
- Parser: Supports `#identifier` syntax (see `ObjectPropertyKey::PrivateIdentifier`)
- AST: Has `PrivateIdentifier` variant
- Compiler: Has code for private fields/methods but incomplete
- Runtime: Missing proper brand checks and private storage

**Missing Pieces:**

#### 2a. Private Field Storage

Private fields need a separate storage mechanism (WeakMap-like) that is keyed by instance.

**Current approach (incomplete):** Uses `class_brand` for tracking but doesn't properly store/retrieve values.

**Files to modify:**
- `src/value.rs` - Add `PrivateFields` storage to `JsObject`
- `src/interpreter/bytecode_vm.rs` - Implement `GetPrivateField`, `SetPrivateField` operations
- `src/compiler/compile_stmt.rs` - Fix private field initialization

#### 2b. Private Method Storage

Similar to fields, but methods are stored once per class, not per instance.

**Files to modify:**
- `src/compiler/bytecode.rs` - Add opcodes for private method access
- `src/interpreter/bytecode_vm.rs` - Implement private method lookup

#### 2c. Brand Checking

Before accessing private members, must verify the object has the class's brand.

```javascript
class C {
  #x = 1;
  getX(obj) { return obj.#x; }  // Must throw if obj doesn't have C's brand
}
```

**Estimated Effort:** Large - significant architectural changes

---

## P2 (Medium Priority) Fixes

### 3. Function Name Inference

**Problem:** Functions assigned through destructuring don't get proper names.

**Test Failures:** ~80 tests with error `Expected SameValue(«""», «"arrow"»)`

**Example:**
```javascript
class C {
  *method([arrow = () => {}]) {
    // arrow.name should be 'arrow', but is ''
  }
}
```

**Root Cause:** When compiling default parameter values that are anonymous functions, we don't assign the binding name to the function's `name` property.

**Files to modify:**
- `src/compiler/compile_pattern.rs` - Add name assignment during pattern compilation
- `src/interpreter/bytecode_vm.rs` - Potentially add `SetFunctionName` opcode

**Spec Reference:** ES2023 14.1.20 Runtime Semantics: NamedEvaluation

**Estimated Effort:** Medium - requires tracking function creation context

---

### 4. Class Fields (Public) Issues

**Problem:** Some class field edge cases don't work correctly.

**Current Status:** Basic public fields work, but these fail:
- Computed field names
- Field initializer ordering relative to super()
- Fields with decorators

**Files to check:**
- `src/compiler/compile_stmt.rs` - `compile_instance_field_initializer`
- Field initialization order in constructors

**Estimated Effort:** Medium

---

### 5. TDZ (Temporal Dead Zone) Violations

**Problem:** Accessing class name before initialization should throw ReferenceError.

**Test Failures:** ~34 tests expecting ReferenceError

**Example:**
```javascript
let C = class C {
  static x = C;  // Should work - C is in scope
};

// But this should fail:
class C extends C {}  // ReferenceError - C used before declaration
```

**Current Issue:** Some TDZ cases throw TypeError instead of ReferenceError.

**Files to modify:**
- `src/compiler/compile_stmt.rs` - Class compilation
- TDZ tracking for class bindings

**Estimated Effort:** Medium

---

## P3 (Low Priority) Fixes

### 6. ToPrimitive Coercion in Computed Property Keys

**Problem:** Using an object without valueOf/toString as a computed key should throw TypeError.

**Test Failures:** ~87 tests expecting TypeError

**Example:**
```javascript
let badKey = Object.create(null);  // No toString/valueOf
class C {
  [badKey]() {}  // Should throw TypeError
}
```

**Files to modify:**
- `src/interpreter/bytecode_vm.rs` - Property key evaluation
- Add proper ToPrimitive conversion with error handling

**Estimated Effort:** Small

---

### 7. Computed Getters/Setters ✅ FIXED

**Problem:** Computed property names for getters/setters not supported in object literals.

**Status:** Fixed! Computed getters/setters now work in both classes (already worked) and object literals.

**Example:**
```javascript
let key = "foo";
let obj = {
  get [key]() { return 1; }  // Now works!
};
```

**Fix:** Modified `compile_accessor_property` in `src/compiler/compile_expr.rs` to handle `ObjectPropertyKey::Computed` using the existing `Op::DefineAccessorComputed` opcode.

---

### 8. Unicode Escapes in Identifiers

**Problem:** Unicode escape sequences in identifier names not fully supported.

**Test Failures:** ~50 tests with error `Unexpected Invalid('\\')`

**Example:**
```javascript
class C {
  \u0066oo() {}  // Should define method 'foo'
}
```

**Files to modify:**
- `src/lexer.rs` - Unicode escape handling in identifiers
- `src/parser.rs` - Identifier parsing

**Estimated Effort:** Medium

---

### 9. Parser Accepting Invalid Syntax

**Problem:** Some syntactically invalid code is not rejected during parsing.

**Test Failures:** ~72 tests expecting SyntaxError in parse phase

**Examples to investigate:**
- Duplicate class members
- Invalid generator/async combinations
- Reserved words in wrong contexts

**Files to modify:**
- `src/parser.rs` - Add validation checks

**Estimated Effort:** Small per issue

---

## Implementation Order

Recommended order based on impact and effort:

1. **Method enumerability** (P1, Small) - Quick win, fixes 140 tests
2. **ToPrimitive coercion** (P3, Small) - Quick fix for edge cases
3. **Function name inference** (P2, Medium) - Common in real code
4. **Class fields fixes** (P2, Medium) - Very common in modern TS
5. **Private fields/methods** (P1, Large) - ✅ Already implemented!
6. **Computed accessors** (P3, Medium) - ✅ DONE
7. **Unicode escapes** (P3, Medium) - Edge case
8. **Parser validation** (P3, Small per issue) - Ongoing

---

## Quick Wins

These can be fixed quickly for immediate test262 improvement:

### Fix 1: Method Enumerability (140 tests)

```rust
// In src/interpreter/bytecode_vm.rs, Op::DefineMethod handler
// Replace:
proto.borrow_mut().set_property(prop_key, method_val);

// With:
proto.borrow_mut().define_property(
    prop_key,
    Property::with_attributes(method_val, true, false, true)
);
```

### Fix 2: Static Method Enumerability

Same fix applies to static methods:
```rust
// Replace:
class_obj.borrow_mut().set_property(prop_key, method_val);

// With:
class_obj.borrow_mut().define_property(
    prop_key,
    Property::with_attributes(method_val, true, false, true)
);
```

---

## Test Commands

```bash
# Run all class expression tests
./target/release/test262-runner --strict-only language/expressions/class

# Run without private member tests (to see other issues)
./target/release/test262-runner --strict-only \
  --skip-features class-fields-private,class-static-fields-private,class-methods-private,class-static-methods-private \
  language/expressions/class

# Run specific category
./target/release/test262-runner --strict-only --verbose language/expressions/class/accessor-name-inst

# Test with stop on first failure for debugging
./target/release/test262-runner --strict-only --verbose --stop-on-fail language/expressions/class
```

---

## References

- [ECMAScript Class Definitions](https://tc39.es/ecma262/#sec-class-definitions)
- [Private Class Fields Proposal](https://github.com/tc39/proposal-class-fields)
- [Test262 Class Tests](https://github.com/tc39/test262/tree/main/test/language/expressions/class)
