# Missing Features Plan

This document analyzes test262 conformance test failures and prioritizes fixes based on real-world impact. The analysis was performed on 2025-12-21.

## Test262 Summary

| Category | Passed | Failed | Skipped | Pass Rate |
|----------|--------|--------|---------|-----------|
| language/expressions | 4916 | 3164 | 2477 | 60.8% |
| language/statements | 3832 | 2461 | 2539 | 60.8% |
| language/types | 86 | 18 | 0 | 82.7% |

---

## Priority 0 - Critical (High Real-World Impact)

### 1. Function Property Descriptors

**Impact:** Libraries and frameworks frequently check function metadata (name, length). Incorrect descriptors break introspection, debugging tools, and framework validation.

**Current Behavior:**
```javascript
const f = function foo(a, b) {};
Object.getOwnPropertyDescriptor(f, 'name').writable;     // true (should be false)
Object.getOwnPropertyDescriptor(f, 'name').enumerable;   // true (should be false)
Object.getOwnPropertyDescriptor(f, 'length').writable;   // true (should be false)
Object.getOwnPropertyDescriptor(f, 'length').enumerable; // true (should be false)
```

**Expected Behavior:**
- `name` property: `{ writable: false, enumerable: false, configurable: true }`
- `length` property: `{ writable: false, enumerable: false, configurable: true }`

**Affected Areas:** All native functions (Array methods, String methods, etc.), user-defined functions, arrow functions, class methods.

**Test Patterns:**
```
Error: obj['name'] descriptor should not be enumerable; obj['name'] descriptor should not be writable
Error: obj['length'] descriptor should not be enumerable; obj['length'] descriptor should not be writable
```

**Implementation:**
1. In `create_native_fn()` and `create_interpreted_function()`, use `define_property_attributes()` to set correct descriptors
2. Create helper function for setting non-enumerable, non-writable properties
3. Apply to all builtin prototype methods

**Estimated Complexity:** Low - mechanical change across function creation paths

---

### 2. Iterator Close Protocol

**Impact:** Resource cleanup in custom iterators (file handles, database connections, streams). Affects generators, async iterators, and any code using early loop exit.

**Current Behavior:**
```javascript
const iter = {
  [Symbol.iterator]() {
    return {
      next() { return { value: 1, done: false }; },
      return() { cleanup(); return { done: true }; }  // Never called!
    };
  }
};
for (const x of iter) {
  break;  // Should call iter.return()
}
```

**Expected Behavior:** When a for-of loop exits early (break, throw, return), call the iterator's `return()` method if it exists.

**Test Patterns:**
```
Error: Expected SameValue(«0», «1») to be true  // return() call count should be 1
```

**Implementation:**
1. Add `IteratorClose` opcode
2. In for-of compilation, emit `IteratorClose` in:
   - Break path (before jump)
   - Exception handler
   - Early return path
3. Also needed for: destructuring, spread, yield*

**Estimated Complexity:** Medium - requires changes to loop compilation and exception handling

---

### 3. Symbol.isConcatSpreadable

**Impact:** Array subclassing, custom array-like objects, library interoperability. Used by frameworks that extend arrays.

**Current Behavior:**
```javascript
const arr = [1, 2, 3];
arr[Symbol.isConcatSpreadable] = false;
[].concat(arr);  // Returns [1, 2, 3] (spreads anyway)
```

**Expected Behavior:** When `[Symbol.isConcatSpreadable]` is `false`, treat the object as non-spreadable (wrap in array).

**Test Patterns:**
```
Error: Actual [1, 2, 3] and expected [true] should have the same contents
```

**Implementation:**
1. In `array_concat()`, check `[Symbol.isConcatSpreadable]` property
2. If false or undefined on non-arrays, don't spread
3. If true on array-likes, spread based on length property

**Estimated Complexity:** Low - localized change in array_concat

---

## Priority 1 - High (Common Usage Patterns)

### 4. Default Parameter TDZ (Temporal Dead Zone)

**Impact:** Function default parameters that reference themselves or later parameters should throw ReferenceError. This is a semantic correctness issue.

**Current Behavior:**
```javascript
function f(x = x) { return x; }  // Should throw ReferenceError
f();  // Returns undefined
```

**Expected Behavior:** Parameters are in TDZ until initialized, so `x = x` should throw.

**Test Patterns:**
```
Error: Expected a ReferenceError to be thrown but no exception was thrown at all
```

**Implementation:**
1. In function parameter compilation, track which parameters are initialized
2. When compiling default value expressions, check references against initialized set
3. Throw ReferenceError for forward references

**Estimated Complexity:** Medium - requires parameter compilation refactor

---

### 5. Generator Methods in Object Literals

**Impact:** Common ES6 pattern for defining generator methods in objects.

**Current Behavior:**
```javascript
const obj = { *gen() { yield 1; } };  // SyntaxError
```

**Expected Behavior:** Parse and compile correctly.

**Test Patterns:**
```
SyntaxError: Unexpected Star, expected property name
```

**Implementation:**
1. In parser `parse_object_literal()`, check for `*` token before method name
2. If found, parse as generator method
3. Set `generator: true` on the parsed method

**Estimated Complexity:** Low - parser addition

---

### 6. Object.defineProperty with Array Indices

**Impact:** Many libraries use `Object.defineProperty` to set array elements with specific descriptors.

**Current Behavior:**
```javascript
const arr = [];
Object.defineProperty(arr, '0', { value: 42, writable: true, enumerable: true, configurable: true });
arr.hasOwnProperty('0');  // false (should be true)
```

**Expected Behavior:** Array elements defined via `defineProperty` should be reflected in `hasOwnProperty`.

**Implementation:**
1. In `object_define_property()`, detect numeric string keys
2. For arrays, ensure the array's internal storage is updated
3. Update array length if needed

**Estimated Complexity:** Medium - requires coordinating object properties with array exotic behavior

---

### 7. Escaped Keywords in Property Names

**Impact:** Obscure but valid syntax. Some minifiers/bundlers may produce this.

**Current Behavior:**
```javascript
const obj = { \u0063ase: 1 };  // SyntaxError (should parse as { case: 1 })
```

**Expected Behavior:** Unicode escapes in identifiers should be resolved before keyword check.

**Test Patterns:**
```
SyntaxError: Unexpected Invalid('\\'), expected property name
```

**Implementation:**
1. In lexer, when parsing identifiers, decode unicode escapes first
2. Then check if result is a reserved word
3. For property names, keywords are allowed

**Estimated Complexity:** Medium - lexer refactor for escape handling

---

## Priority 2 - Medium (Less Common Cases)

### 8. try/catch/finally Completion Values

**Impact:** Rare edge case where completion values matter (mainly eval).

**Current Behavior:**
```javascript
eval(`
  L: do {
    try { break L; }
    finally { }
  } while (false);
`);  // Returns some value (should be undefined)
```

**Test Patterns:**
```
Error: Expected SameValue(«"bad completion"», «undefined») to be true
```

**Implementation:**
1. In `compile_try_statement`, track completion value through all paths
2. Empty finally shouldn't override completion value
3. Break/continue in try with finally needs special handling

**Estimated Complexity:** High - complex control flow tracking

---

### 9. Promise.all/race Iterator Handling

**Impact:** Promise combinators not properly iterating inputs.

**Current Behavior:**
```javascript
let resolveCount = 0;
Promise.resolve = function(v) {
  resolveCount++;
  return { then: (f) => f(v) };
};
Promise.all([1, 2, 3]);
resolveCount;  // 0 (should be 3)
```

**Test Patterns:**
```
Error: callCount after call to all() Expected SameValue(«0», «1») to be true
```

**Implementation:**
1. In `promise_all()`, properly iterate using Symbol.iterator
2. Call `Promise.resolve` for each element
3. Handle iterator close on rejection

**Estimated Complexity:** Medium - promise implementation refactor

---

### 10. Strict Mode Parse-Time Errors

**Impact:** Certain constructs should fail at parse time in strict mode.

**Current Behavior:**
```javascript
"use strict";
function f(a, a) {}  // Should be SyntaxError, but parses
```

**Test Patterns:**
```
Expected SyntaxError in parse phase, got: Error: Test262: This statement should not be evaluated.
```

**Implementation:**
1. Track strict mode state in parser
2. In function parameter parsing, detect duplicate names
3. Also check: `with`, `eval`/`arguments` binding, octal literals

**Estimated Complexity:** Medium - parser needs strict mode awareness

---

### 11. Template Literal Invalid Escapes

**Impact:** Invalid escape sequences in template literals should be syntax errors (unless tagged).

**Current Behavior:**
```javascript
`\xZZ`;  // Parses (should be SyntaxError)
```

**Implementation:**
1. In template literal parsing, validate escape sequences
2. `\xHH` - exactly 2 hex digits
3. `\uHHHH` or `\u{...}` - valid unicode
4. Throw SyntaxError for invalid escapes (except in tagged templates)

**Estimated Complexity:** Low - lexer validation

---

## Priority 3 - Low (New APIs / Rare Cases)

### 12. Map.groupBy / Object.groupBy

**Impact:** New ES2024 API, easily polyfillable.

**Test Patterns:**
```
TypeError: Not a function
```

**Implementation:**
1. Add `map_group_by()` native function
2. Add `object_group_by()` native function
3. Both take iterable and callback

**Estimated Complexity:** Low - new builtin functions

---

### 13. JSON.isRawJSON

**Impact:** New ES2024 API for raw JSON handling.

**Implementation:** Add `json_is_raw_json()` and `JSON.rawJSON()` support.

**Estimated Complexity:** Medium - new JSON functionality

---

### 14. String.prototype.at Surrogate Handling

**Impact:** Unicode surrogate pair handling in `at()`.

**Current Behavior:**
```javascript
'\uD800\uDC00'.at(0);  // Returns first code unit instead of surrogate pair
```

**Implementation:** Review string indexing for proper UTF-16 handling.

**Estimated Complexity:** Medium - string implementation

---

### 15. Symbol.species

**Impact:** Subclassing built-ins (Array, Promise, Map, etc.).

**Current Behavior:**
```javascript
Promise[Symbol.species];  // undefined (should be Promise)
```

**Implementation:**
1. Add `Symbol.species` getter to all built-in constructors
2. Use in methods that create new instances (`map`, `filter`, `then`, etc.)

**Estimated Complexity:** Medium - affects many builtin methods

---

### 16. Symbol.unscopables

**Impact:** Only affects `with` statement (deprecated).

**Implementation:** Add `Array.prototype[Symbol.unscopables]` object.

**Estimated Complexity:** Low - but rarely used

---

## Implementation Roadmap

### Phase 1: Quick Wins (P0)
1. **Function property descriptors** - High test impact, low complexity
2. **Symbol.isConcatSpreadable** - Localized change
3. **Generator methods in objects** - Parser addition

### Phase 2: Core Fixes (P1)
4. **Iterator close protocol** - Important for resource management
5. **Default parameter TDZ** - Semantic correctness
6. **Object.defineProperty arrays** - Common usage

### Phase 3: Parser Improvements (P1-P2)
7. **Escaped keywords in properties** - Lexer refactor
8. **Strict mode parse errors** - Parser awareness
9. **Template literal escapes** - Lexer validation

### Phase 4: Edge Cases (P2)
10. **try/catch completion values** - Complex but correctness
11. **Promise iterator handling** - Promise refactor

### Phase 5: New APIs (P3)
12. **Map.groupBy/Object.groupBy** - New ES2024
13. **Symbol.species** - Subclassing support
14. **JSON.isRawJSON** - New JSON API

---

## Test Commands

```bash
# Run test262 on specific area
./target/release/test262-runner --strict-only language/expressions/object

# Verbose output for debugging
./target/release/test262-runner --strict-only --verbose --stop-on-fail language/statements/for-of

# Check specific builtin
./target/release/test262-runner --strict-only built-ins/Array/prototype/concat
```

---

## Notes

- Many failures are related to property descriptor checks - fixing function descriptors will have cascading benefits
- Iterator protocol issues affect multiple areas: for-of, destructuring, spread, generators
- Parser improvements for strict mode will catch many "expected SyntaxError" failures
- Some features like BigInt and TypedArray are intentionally skipped (see test262.md)
