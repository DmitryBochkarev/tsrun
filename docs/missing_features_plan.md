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

### 1. Function Property Descriptors ✅ IMPLEMENTED

**Status:** Implemented on 2025-12-21

**Implementation:**
- Modified `get_property_descriptor()` in value.rs to return correct attributes for function `name` and `length` properties:
  `{ writable: false, enumerable: false, configurable: true }`
- Updated `object_get_own_property_descriptor()` to use `get_property_descriptor()` which handles exotic function properties
- Tests added: `test_function_name_descriptor`, `test_function_length_descriptor`, `test_builtin_function_name_descriptor`,
  `test_builtin_function_length_descriptor`, `test_arrow_function_name_descriptor`, `test_arrow_function_length_descriptor`

---

### 2. Iterator Close Protocol ✅ IMPLEMENTED

**Status:** Implemented on 2025-12-21

**Implementation:**
- Added `IteratorClose` opcode to call iterator's `return()` method
- Added `PushIterTry`/`PopIterTry` opcodes for exception handling in for-of loops
- Compiler emits `IteratorClose` before break/return statements inside for-of loops
- For-of body is wrapped in implicit try-catch that closes iterator on exceptions
- Tests added: `test_iterator_close_on_break`, `test_iterator_close_on_return`, `test_iterator_close_on_throw`

**Note:** Destructuring, spread, and yield* iterator close still needs implementation.

---

### 3. Symbol.isConcatSpreadable ✅ IMPLEMENTED

**Status:** Implemented on 2025-12-21

**Implementation:**
- Modified `array_concat()` to check `[Symbol.isConcatSpreadable]` property
- If explicitly `false`, the object is not spread (even arrays)
- If explicitly `true`, array-like objects are spread using their length property
- If undefined, only arrays are spread (default behavior)
- Tests added: `test_array_concat_is_concat_spreadable_false`, `test_array_concat_is_concat_spreadable_true`,
  `test_array_concat_non_array_without_spreadable`

---

## Priority 1 - High (Common Usage Patterns)

### 4. Default Parameter TDZ (Temporal Dead Zone)

**Impact:** Function default parameters that reference themselves or later parameters should throw ReferenceError. This is a semantic correctness issue.

**Current Behavior:**
**Status:** Already implemented! Verified on 2025-12-21.

The bytecode VM correctly tracks TDZ for parameters. Both self-reference (`x = x`) and forward reference (`x = y`) correctly throw ReferenceError.

Tests added: `test_default_param_tdz_self_reference`, `test_default_param_tdz_forward_reference`, `test_default_param_can_reference_earlier_param`

---

### 5. Generator Methods in Object Literals ✅ IMPLEMENTED

**Status:** Implemented on 2025-12-21

**Implementation:**
- Modified `parse_property()` to check for `*` before method name
- Added support for `async *` (async generator methods)
- Updated `peek_is_property_name()` to recognize `*` as indicating a method follows
- Tests added: `test_generator_method_in_object_literal`, `test_generator_method_with_params`, `test_generator_method_computed_name`, `test_async_generator_method_in_object_literal`

---

### 6. Object.defineProperty with Array Indices ✅ IMPLEMENTED

**Status:** Implemented on 2025-12-21

**Implementation:**
- Modified `object_define_property()` to detect numeric keys (both `PropertyKey::Index` and parseable string keys)
- For arrays, extends the `elements` vector to include the index
- Fixed `object_keys()` to avoid duplicate keys when both elements and properties contain the same index
- Tests added: `test_object_define_property_on_array`, `test_define_property_array_access`, `test_define_property_array_object_keys`, `test_define_property_array_multiple_indices`

---

### 7. Unicode Escapes in Identifiers ✅ IMPLEMENTED

**Status:** Implemented on 2025-12-21

**Implementation:**
- Modified `scan_identifier()` in lexer.rs to handle `\uNNNN` and `\u{N...}` unicode escape sequences
- Added `scan_unicode_escape_in_identifier()` helper method to decode escape sequences
- Identifiers containing unicode escapes that spell out a reserved word remain identifiers (not keywords)
- Added `is_id_start_char()` and `is_id_continue_char()` helpers for decoded character validation
- Tests added: `test_unicode_escape_identifier`, `test_unicode_escape_identifier_mixed`, `test_unicode_escape_keyword_becomes_identifier`, `test_unicode_escape_braced_form`, `test_unicode_escape_braced_longer`
- Interpreter tests: `test_unicode_escape_in_property_name`, `test_unicode_escape_identifier_basic`, `test_unicode_escape_identifier_mixed`, `test_unicode_escape_in_member_access`

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
1. ~~**Function property descriptors** - High test impact, low complexity~~ ✅ DONE
2. ~~**Symbol.isConcatSpreadable** - Localized change~~ ✅ DONE
3. ~~**Generator methods in objects** - Parser addition~~ ✅ DONE

### Phase 2: Core Fixes (P1)
4. ~~**Iterator close protocol** - Important for resource management~~ ✅ DONE
5. ~~**Default parameter TDZ** - Semantic correctness~~ ✅ ALREADY IMPLEMENTED
6. ~~**Object.defineProperty arrays** - Common usage~~ ✅ DONE

### Phase 3: Parser Improvements (P1-P2)
7. ~~**Unicode escapes in identifiers** - Lexer refactor~~ ✅ DONE
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
