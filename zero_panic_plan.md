# Zero-Panic Policy Refactoring Plan

This document outlines the plan to eliminate all panic-prone patterns from the codebase and enforce a zero-panic policy using Clippy lints.

## Overview

**Goal:** Eliminate all potential runtime panics from production code, making the interpreter robust and predictable.

**Current State:** 43 clippy warnings when running with panic-related lints enabled.

**Enforcement:** Use Clippy with `deny` level for panic-prone lints in CI and development.

---

## Clippy Lints to Enable

Add the following to `Cargo.toml` or `lib.rs`:

```rust
// In lib.rs (recommended for fine-grained control)
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::string_slice)]
```

Or in `Cargo.toml`:

```toml
[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
indexing_slicing = "deny"
panic = "deny"
unreachable = "deny"
todo = "deny"
unimplemented = "deny"
string_slice = "deny"
```

### Clippy Configuration (`clippy.toml`)

Create `clippy.toml` to allow these patterns in tests:

```toml
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-indexing-slicing-in-tests = true
allow-panic-in-tests = true
```

---

## Current Violations Summary

| Category | Count | Files Affected |
|----------|-------|----------------|
| `unwrap_used` | 7 | lexer.rs, parser.rs, interpreter/mod.rs, builtins/array.rs, builtins/math.rs |
| `indexing_slicing` | 22 | interpreter/mod.rs, builtins/*.rs |
| `string_slice` | 9 | lexer.rs, builtins/string.rs, builtins/global.rs |
| `unreachable!` | 2 | parser.rs, interpreter/mod.rs |
| `panic!` | 0 (production) | - |
| `expect_used` | 0 | - |

---

## Refactoring Tasks by File

### Phase 1: High Priority (Core Interpreter)

#### 1. `src/interpreter/mod.rs` (10 violations)

| Line | Pattern | Current Code | Fix |
|------|---------|--------------|-----|
| 573 | indexing | `self.static_imports[self.static_import_index - 1]` | Use `.get()` with error propagation |
| 660 | indexing | `self.static_imports[self.static_import_index]` | Use `.get()` with error propagation |
| 801 | indexing | `&statements[index]` | Use `.get()` with error propagation |
| 1019 | unwrap | `case.test.as_ref().unwrap()` | Use `if let Some(test) = &case.test` pattern |
| 2652 | indexing | `&template.expressions[i]` | Use `.get()` with error propagation |
| 2787 | unwrap | `self.generator_context.as_mut().unwrap()` | Return error or use `ok_or()` |
| 2816 | unwrap | `self.generator_context.as_mut().unwrap()` | Return error or use `ok_or()` |
| 3176 | unreachable | `unreachable!()` | Return `JsError::internal_error()` |
| 3601 | slicing | `args[i..].to_vec()` | Use `.get(i..)` with `.unwrap_or_default()` |
| 3731 | slicing | `args[i..].to_vec()` | Use `.get(i..)` with `.unwrap_or_default()` |

**Refactoring Patterns:**

```rust
// Before:
let stmt = &statements[index];

// After:
let stmt = statements.get(index)
    .ok_or_else(|| JsError::internal_error("statement index out of bounds"))?;

// Before:
case.test.as_ref().unwrap()

// After:
let Some(test) = &case.test else {
    return Err(JsError::internal_error("non-default case missing test"));
};

// Before:
args[i..].to_vec()

// After:
args.get(i..).unwrap_or_default().to_vec()
```

#### 2. `src/lexer.rs` (3 violations)

| Line | Pattern | Current Code | Fix |
|------|---------|--------------|-----|
| 401 | string_slice | `self.source[self.current_pos..]` | Validate UTF-8 boundary or use chars() |
| 883 | string_slice | `self.source[base_offset..]` | Validate UTF-8 boundary or use chars() |
| 1018 | unwrap | `self.advance().unwrap()` | Use `if let Some(ch) = self.advance()` |

**Note:** The lexer string slicing is likely safe because we always advance by char boundaries, but we should document this invariant or switch to char-based iteration.

```rust
// Before:
if matches!(self.peek(), Some('+' | '-')) {
    num_str.push(self.advance().unwrap().1);
}

// After:
if matches!(self.peek(), Some('+' | '-')) {
    if let Some((_, ch)) = self.advance() {
        num_str.push(ch);
    }
}
```

#### 3. `src/parser.rs` (3 violations)

| Line | Pattern | Current Code | Fix |
|------|---------|--------------|-----|
| 669 | unreachable | `_ => unreachable!()` | Use `_ => return Err(...)` |
| 2464 | unwrap | `types.pop().unwrap()` | Use `types.pop().ok_or(...)` |
| 2481 | unwrap | `types.pop().unwrap()` | Use `types.pop().ok_or(...)` |

```rust
// Before:
if types.len() == 1 {
    Ok(types.pop().unwrap())
}

// After:
if types.len() == 1 {
    types.pop().ok_or_else(|| self.error("expected type"))
}
```

---

### Phase 2: Builtins

#### 4. `src/interpreter/builtins/string.rs` (6 violations)

| Line | Pattern | Current Code | Fix |
|------|---------|--------------|-----|
| 323 | string_slice | `s.as_str()[from_index..]` | Use chars-based approach |
| 360 | string_slice | `s.as_str()[..search_end]` | Use chars-based approach |
| 405 | string_slice | `s.as_str()[from_index..]` | Use chars-based approach |
| 425 | string_slice | `s.as_str()[position..]` | Use chars-based approach |
| 445 | string_slice | `s.as_str()[..end]` | Use chars-based approach |
| 476, 521, 571 | slicing | `chars[start..end]` | Use `.get()` with fallback |
| 875 | indexing | `chars[index]` | Use `.get()` with error |

**Strategy for string operations:**

The issue is JavaScript uses UTF-16 indices while Rust uses byte indices. We should:

1. Convert to `Vec<char>` for index operations
2. Use `.get()` for safe access
3. Return empty string/undefined for out-of-bounds

```rust
// Before:
match s.as_str()[from_index..].find(&search) {

// After:
let search_area = if from_index < s.len() {
    s.get(from_index..).unwrap_or("")
} else {
    ""
};
match search_area.find(&search) {
```

#### 5. `src/interpreter/builtins/array.rs` (7 violations)

| Line | Pattern | Current Code | Fix |
|------|---------|--------------|-----|
| 337 | indexing | `&args[0]` | Use `args.first()` |
| 616 | indexing | `args[1].clone()` | Use `args.get(1)` |
| 1219 | indexing | `elements[j]`, `elements[j+1]` | Use `.get()` pairs |
| 1662 | unwrap | `accumulator.clone().unwrap()` | Use `if let Some(acc)` or return error |
| 1948 | indexing | `elements[j-1]`, `elements[j]` | Use `.get()` pairs |

```rust
// Before:
if let JsValue::Number(n) = &args[0] {

// After:
if let Some(JsValue::Number(n)) = args.first() {

// Before:
(args[1].clone(), 0)

// After:
(args.get(1).cloned().unwrap_or(JsValue::Undefined), 0)
```

#### 6. `src/interpreter/builtins/number.rs` (3 violations)

| Line | Pattern | Current Code | Fix |
|------|---------|--------------|-----|
| 255 | indexing | `chars[(num % radix) as usize]` | Validate radix is 2-36, use `.get()` |
| 302, 303 | indexing | `parts[0]`, `parts[1]` | Use destructuring or `.get()` |

```rust
// Before:
let mantissa = parts[0].parse::<f64>().unwrap_or(0.0);
let exp: i32 = parts[1].parse().unwrap_or(0);

// After:
let (mantissa, exp) = match (parts.first(), parts.get(1)) {
    (Some(m), Some(e)) => (
        m.parse::<f64>().unwrap_or(0.0),
        e.parse::<i32>().unwrap_or(0)
    ),
    _ => (0.0, 0),
};
```

#### 7. `src/interpreter/builtins/symbol.rs` (2 violations)

| Line | Pattern | Current Code | Fix |
|------|---------|--------------|-----|
| 237 | indexing | `&args[0]` | Use `args.first()` |
| 256 | indexing | `args[0]` | Use `args.first()` |

#### 8. `src/interpreter/builtins/date.rs` (1 violation)

| Line | Pattern | Current Code | Fix |
|------|---------|--------------|-----|
| 265 | indexing | `&args[0]` | Use `args.first()` |

#### 9. `src/interpreter/builtins/map.rs` (1 violation)

| Line | Pattern | Current Code | Fix |
|------|---------|--------------|-----|
| 278 | indexing | `entries[i]` | Use `.get(i)` |

#### 10. `src/interpreter/builtins/set.rs` (1 violation)

| Line | Pattern | Current Code | Fix |
|------|---------|--------------|-----|
| 199 | indexing | `entries[i]` | Use `.get(i)` |

#### 11. `src/interpreter/builtins/math.rs` (1 violation)

| Line | Pattern | Current Code | Fix |
|------|---------|--------------|-----|
| 423-425 | unwrap | `SystemTime::now().duration_since().unwrap()` | Use fallback seed value |

```rust
// Before:
let seed = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_nanos() as u64;

// After:
let seed = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_nanos() as u64)
    .unwrap_or(0x12345678); // Fallback seed if time is before epoch
```

#### 12. `src/interpreter/builtins/global.rs` (1 violation)

| Line | Pattern | Current Code | Fix |
|------|---------|--------------|-----|
| 204 | string_slice | `&s[..end]` | Use safe slicing |

---

## Helper Functions to Add

Create a helper module `src/interpreter/safe_access.rs` with common safe access patterns:

```rust
/// Safe argument access for builtin functions
pub trait SafeArgs {
    fn get_arg(&self, index: usize) -> JsValue;
    fn get_arg_or(&self, index: usize, default: JsValue) -> JsValue;
}

impl SafeArgs for Vec<JsValue> {
    fn get_arg(&self, index: usize) -> JsValue {
        self.get(index).cloned().unwrap_or(JsValue::Undefined)
    }

    fn get_arg_or(&self, index: usize, default: JsValue) -> JsValue {
        self.get(index).cloned().unwrap_or(default)
    }
}

/// Safe string slicing that handles UTF-8 boundaries
pub fn safe_str_slice(s: &str, start: usize, end: usize) -> &str {
    if start >= s.len() {
        return "";
    }
    let end = end.min(s.len());
    // Ensure we're at char boundaries
    let start = s.floor_char_boundary(start);
    let end = s.ceil_char_boundary(end);
    &s[start..end]
}
```

---

## Implementation Order

### Week 1: Foundation
1. [ ] Add clippy configuration to `Cargo.toml` and `clippy.toml`
2. [ ] Create helper module with safe access patterns
3. [ ] Add `JsError::internal_error()` variant if not present

### Week 2: Core Interpreter
4. [ ] Fix `src/interpreter/mod.rs` (10 violations)
5. [ ] Fix `src/lexer.rs` (3 violations)
6. [ ] Fix `src/parser.rs` (3 violations)

### Week 3: String Builtins
7. [ ] Fix `src/interpreter/builtins/string.rs` (6 violations)
8. [ ] Fix `src/interpreter/builtins/global.rs` (1 violation)

### Week 4: Other Builtins
9. [ ] Fix `src/interpreter/builtins/array.rs` (7 violations)
10. [ ] Fix `src/interpreter/builtins/number.rs` (3 violations)
11. [ ] Fix remaining builtins (symbol, date, map, set, math)

### Week 5: Verification & CI
12. [ ] Enable all lints as `deny`
13. [ ] Update CI to run clippy with zero-panic lints
14. [ ] Update CLAUDE.md with new lint requirements

---

## CI Integration

Add to `.github/workflows/ci.yml`:

```yaml
- name: Clippy (Zero Panic Policy)
  run: |
    cargo clippy -- \
      -D clippy::unwrap_used \
      -D clippy::expect_used \
      -D clippy::indexing_slicing \
      -D clippy::panic \
      -D clippy::unreachable \
      -D clippy::todo \
      -D clippy::unimplemented \
      -D clippy::string_slice
```

---

## Exceptions

Some patterns may need `#[allow(...)]` with documentation:

```rust
// Performance-critical hot path where bounds are proven
#[allow(clippy::indexing_slicing)]
// SAFETY: index is validated by the loop condition `i < len`
let element = &elements[i];
```

Each exception MUST include:
1. The `#[allow(...)]` attribute
2. A `// SAFETY:` or `// PANIC-SAFE:` comment explaining why it's safe
3. Proof that the index is within bounds

---

## Testing Strategy

1. **Property-based testing:** Add tests with arbitrary Unicode strings
2. **Boundary testing:** Test all slice operations with empty inputs, single chars, max lengths
3. **Fuzz testing:** Consider adding cargo-fuzz for parser and lexer

---

## References

- [Clippy Lints Documentation](https://rust-lang.github.io/rust-clippy/master/index.html)
- [Rust API Guidelines - Documentation](https://rust-lang.github.io/api-guidelines/documentation.html)
- [Issue #12754: Unified no-panic lint](https://github.com/rust-lang/rust-clippy/issues/12754)
