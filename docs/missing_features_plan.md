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

### 8. For-in/For-of with Pre-declared Variables ✅ IMPLEMENTED

**Status:** Implemented on 2025-12-22

**Implementation:**
- Added `no_in` flag to Parser struct to control whether `in` is treated as a binary operator
- When parsing for-loop init expressions, set `no_in = true` to allow `for (x in obj)` syntax
- This enables `for (existingVar in object)` and `for (existingVar of iterable)` patterns
- Reset `no_in = false` inside brackets `[]` and parentheses `()` per ECMAScript spec
  - Array literals: `for (... [a in obj] ...)` - `in` works inside `[]`
  - Computed properties: `for (const key in { ['x' in empty]: 1 })` - `in` works inside `[]`
  - Member expressions: `obj[key in map]` - `in` works inside `[]`
  - Parenthesized expressions: `('a' in obj ? 2 : 0)` - `in` works inside `()`
- Tests added: `test_for_in_with_predeclared_var`, `test_for_in_with_predeclared_var_accumulate`,
  `test_for_of_with_predeclared_var`, `test_for_in_computed_property_with_in`, `test_for_loop_array_with_in`

**Note:** Member expressions in for-in left-hand side (e.g., `for (obj.key in source)`) are not supported.

---

## Priority 2 - Medium (Less Common Cases)

### 9. try/catch/finally Completion Values ✅ IMPLEMENTED (Previously #8)

**Status:** Implemented on 2025-12-22

**Implementation:**
- Empty catch blocks now set completion value to `undefined` (was keeping previous completion)
- `break` and `continue` statements now set completion value to `undefined` per ES spec UpdateEmpty
- These fixes ensure correct completion values in eval() for try/catch/finally blocks

**Changes:**
- Modified `compile_try()` in compile_stmt.rs to emit `LoadUndefined { dst: 0 }` for empty catch blocks when tracking completions
- Modified `compile_break()` and `compile_continue()` to emit `LoadUndefined { dst: 0 }` when tracking completions
- Tests added: `test_eval_try_catch_empty_completion`, `test_eval_try_catch_expression_completion`,
  `test_eval_try_catch_break_completion`, `test_eval_try_catch_continue_completion`

**Test262 Impact:** try statement tests improved from 82.2% to 84.8% pass rate

---

### 10. Promise.all/race Iterator Handling ✅ PARTIALLY IMPLEMENTED (Previously #9)

**Status:** Partially implemented on 2025-12-22.

**Implementation:**
- Modified `promise_all()` to call `Promise.resolve` on each element via `resolve_each_value()`
- Non-promise values are now wrapped in fulfilled promises before being processed
- Already-promise values are returned as-is (per spec optimization)
- Proper GC guard management to prevent premature collection of promise objects
- Tests added: `test_promise_resolve_accessible`, `test_promise_all_with_plain_values` now passes

**Note:** Full spec compliance would require:
- Using Symbol.iterator for proper iteration (currently only arrays supported)
- Calling the user-visible `Promise.resolve` (currently calls internal implementation)
- Iterator close on rejection
These are edge cases that rarely affect real-world code.

---

### 11. Strict Mode Parse-Time Errors ✅ IMPLEMENTED (Previously #10)

**Status:** Already implemented. Verified on 2025-12-21.

**Implementation:**
The interpreter always runs in strict mode. The parser enforces:
- Duplicate parameter names → SyntaxError (`check_duplicate_params`)
- `eval`/`arguments` as binding names → SyntaxError (`validate_binding_identifier`)
- `with` statement → Not supported (skipped in test262)
- Legacy octal literals (0777) → Invalid token in lexer
- Delete on unqualified identifier → SyntaxError in parser

Tests in `tests/interpreter/strict.rs`:
- `test_strict_no_duplicate_params`, `test_strict_no_duplicate_params_arrow`
- `test_strict_no_var_eval`, `test_strict_no_let_eval`, `test_strict_no_const_eval`
- `test_strict_no_var_arguments`, `test_strict_no_let_arguments`, etc.
- `test_strict_no_delete_variable`
- `test_strict_no_legacy_octal`, `test_strict_no_octal_escape`

---

### 12. Template Literal Invalid Escapes ✅ IMPLEMENTED (Previously #11)

**Status:** Implemented on 2025-12-21

**Implementation:**
- Modified `scan_template_literal()` and `scan_template_continuation()` in lexer.rs
- Invalid hex escapes (`\xZZ`) now return `TokenKind::Invalid`
- Invalid unicode escapes (`\u00GG`, `\u{ZZZZ}`) now return `TokenKind::Invalid`
- Octal escapes after `\0` (like `\01`) now return `TokenKind::Invalid`
- Tests added: `test_template_invalid_hex_escape`, `test_template_invalid_unicode_escape`, `test_template_invalid_unicode_brace_escape`

**Note:** This is stricter than ES2018+ spec which allows invalid escapes in tagged templates.
For full compliance, we would need to track escape validity and only reject in untagged templates.

---

## Priority 3 - Low (New APIs / Rare Cases)

### 13. Map.groupBy / Object.groupBy ✅ IMPLEMENTED (Previously #12)

**Status:** Implemented on 2025-12-22

**Implementation:**
- Added `object_group_by()` in `builtins/object.rs` - groups array items by string keys
- Added `map_group_by()` in `builtins/map.rs` - groups array items by any key type (using SameValueZero)
- Object.groupBy returns an object with null prototype
- Map.groupBy returns a Map instance
- Both support callbacks with (item, index) arguments
- Tests added: `test_object_groupby_*` (8 tests), `test_map_groupby_*` (6 tests)

**Note:** Currently only supports arrays as input. Full spec compliance would use Symbol.iterator.

---

### 14. JSON.isRawJSON (Previously #13)

**Impact:** New ES2024 API for raw JSON handling.

**Implementation:** Add `json_is_raw_json()` and `JSON.rawJSON()` support.

**Estimated Complexity:** Medium - new JSON functionality

---

### 15. String.prototype.at Surrogate Handling (Previously #14)

**Impact:** Unicode surrogate pair handling in `at()`.

**Current Behavior:**
```javascript
'\uD800\uDC00'.at(0);  // Returns first code unit instead of surrogate pair
```

**Implementation:** Review string indexing for proper UTF-16 handling.

**Estimated Complexity:** Medium - string implementation

---

### 16. Symbol.species ✅ PARTIALLY IMPLEMENTED (Previously #15)

**Status:** Partially implemented on 2025-12-22

**Implementation:**
- Added `register_species_getter()` helper to Interpreter
- Added `Symbol.species` getter to Array, Promise, Map, Set, and RegExp constructors
- The getter returns `this` per ECMAScript spec
- Tests added: `test_array_symbol_species`, `test_promise_symbol_species`, `test_map_symbol_species`,
  `test_set_symbol_species`, `test_regexp_symbol_species`, `test_symbol_species_is_symbol`

**Note:** Full spec compliance would require:
- Modifying methods like `map`, `filter`, `slice`, `then` to use SpeciesConstructor
- This allows subclasses to control which constructor is used for derived objects
- Current implementation provides the getter, which is sufficient for most real-world use cases

---

### 17. Symbol.unscopables (Previously #16)

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
8. ~~**For-in/for-of with pre-declared variables** - Parser fix~~ ✅ DONE
9. ~~**Strict mode parse errors** - Parser awareness~~ ✅ ALREADY IMPLEMENTED
10. ~~**Template literal escapes** - Lexer validation~~ ✅ DONE

### Phase 4: Edge Cases (P2)
11. ~~**try/catch completion values** - Complex but correctness~~ ✅ DONE
12. ~~**Promise iterator handling** - Promise refactor~~ ✅ PARTIALLY DONE

### Phase 5: New APIs (P3)
13. ~~**Map.groupBy/Object.groupBy** - New ES2024~~ ✅ DONE
14. ~~**Symbol.species** - Subclassing support~~ ✅ PARTIALLY DONE
15. **JSON.isRawJSON** - New JSON API

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
