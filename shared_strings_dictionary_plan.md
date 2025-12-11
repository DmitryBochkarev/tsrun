# Shared String Dictionary Implementation Plan

## Overview

This document describes an optimization to share string instances between the parser and interpreter. Currently, each identifier occurrence creates a new `JsString` (`Rc<str>`) allocation. With a shared dictionary, identical strings will share the same `Rc<str>` instance, reducing memory allocations and improving cache locality.

## Current Architecture

### String Flow (Before)

```
Lexer                    Parser                      AST                    Interpreter
─────                    ──────                      ───                    ───────────
"length" (String) ──────► JsString::from("length") ──► Identifier.name ────► env lookup
"length" (String) ──────► JsString::from("length") ──► Identifier.name ────► env lookup
"length" (String) ──────► JsString::from("length") ──► Identifier.name ────► env lookup
     │                         │                           │
     └─── 3 String allocs ─────┴─── 3 Rc<str> allocs ──────┘
```

**Problem:** Each `"length"` creates a separate `Rc<str>` instance.

### String Types

| Location | Type | Definition |
|----------|------|------------|
| Lexer tokens | `String` | `TokenKind::Identifier(String)` |
| AST nodes | `JsString` | `Rc<str>` wrapper |
| Runtime values | `JsString` | Same `Rc<str>` wrapper |
| Property keys | `PropertyKey::String(JsString)` | Enum variant |
| Environment bindings | `FxHashMap<JsString, Binding>` | Keys are `JsString` |

## Proposed Architecture

### String Flow (After)

```
Lexer                    Parser + Dict                AST                    Interpreter + Dict
─────                    ────────────                 ───                    ──────────────────
"length" (String) ──────► dict.get("length") ────────► Identifier.name ────► dict.get("length")
"length" (String) ──────► dict.get("length") ────────► Identifier.name ────► dict.get("length")
"length" (String) ──────► dict.get("length") ────────► Identifier.name ────► dict.get("length")
     │                         │                           │                        │
     └─── 3 String allocs ─────┴─── 1 Rc<str> (shared) ────┴────────────────────────┘
```

**Solution:** All `"length"` occurrences share the same `Rc<str>` instance.

### Core Type: StringDict

```rust
/// A dictionary for deduplicating JsString instances.
///
/// Strings inserted into the dictionary are stored once and subsequent
/// requests for the same string return a cheap clone of the existing instance.
pub struct StringDict {
    /// Map from string content to shared JsString instance.
    /// Using Box<str> as key to avoid double-indirection through Rc.
    strings: FxHashMap<Box<str>, JsString>,
}

impl StringDict {
    /// Create an empty dictionary.
    pub fn new() -> Self {
        Self {
            strings: FxHashMap::default(),
        }
    }

    /// Create a dictionary pre-populated with common strings.
    pub fn with_common_strings() -> Self {
        let mut dict = Self::new();
        for s in COMMON_STRINGS {
            dict.get_or_insert(s);
        }
        dict
    }

    /// Get an existing string or insert a new one.
    /// Returns a cheap clone of the shared JsString instance.
    pub fn get_or_insert(&mut self, s: &str) -> JsString {
        if let Some(existing) = self.strings.get(s) {
            return existing.cheap_clone();
        }
        let js_str = JsString::from(s);
        self.strings.insert(s.into(), js_str.cheap_clone());
        js_str
    }

    /// Get an existing string without inserting.
    /// Returns None if the string is not in the dictionary.
    pub fn get(&self, s: &str) -> Option<JsString> {
        self.strings.get(s).map(|s| s.cheap_clone())
    }

    /// Insert a JsString that was created elsewhere.
    /// If the string already exists, returns the existing instance.
    pub fn insert(&mut self, js_str: JsString) -> JsString {
        if let Some(existing) = self.strings.get(js_str.as_str()) {
            return existing.cheap_clone();
        }
        self.strings.insert(js_str.as_str().into(), js_str.cheap_clone());
        js_str
    }

    /// Number of unique strings in the dictionary.
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

impl Default for StringDict {
    fn default() -> Self {
        Self::new()
    }
}
```

### Common Strings to Pre-populate

```rust
/// Strings that appear frequently in JavaScript code and runtime.
const COMMON_STRINGS: &[&str] = &[
    // Object properties
    "length", "prototype", "constructor", "__proto__",
    "name", "message", "stack",

    // Property descriptors
    "value", "writable", "enumerable", "configurable",
    "get", "set",

    // Common methods
    "toString", "valueOf", "hasOwnProperty", "toJSON",

    // Array iteration
    "next", "done", "return", "throw",

    // Type names
    "undefined", "null", "boolean", "number", "string",
    "object", "function", "symbol",

    // Built-in constructors
    "Object", "Array", "String", "Number", "Boolean",
    "Function", "Error", "TypeError", "ReferenceError",
    "SyntaxError", "RangeError", "Map", "Set", "Date",
    "RegExp", "Promise", "Symbol",

    // Common identifiers
    "this", "arguments", "callee", "caller",

    // Internal properties
    "__super__", "__fields__", "__private__",

    // Console
    "log", "error", "warn", "info", "debug",

    // Math
    "PI", "E", "abs", "floor", "ceil", "round", "max", "min",

    // Common variable names (optional - may not be worth it)
    "i", "j", "k", "x", "y", "n", "s", "v", "key", "val",
    "arr", "obj", "fn", "cb", "err", "res", "req",
];
```

## Implementation Phases

### Phase 1: Create StringDict Module

**File: `src/string_dict.rs`** (new file)

```rust
//! String dictionary for deduplicating JsString instances.

use rustc_hash::FxHashMap;
use crate::value::{JsString, CheapClone};

// ... StringDict implementation as shown above ...
```

**File: `src/lib.rs`** (add module)

```rust
pub mod string_dict;
pub use string_dict::StringDict;
```

**Estimated changes:** ~100 lines, 2 files

### Phase 2: Integrate with Parser

**File: `src/parser.rs`**

Add dictionary to Parser struct:

```rust
pub struct Parser<'src> {
    lexer: Lexer<'src>,
    current: Token,
    previous: Token,
    string_dict: &'src mut StringDict,  // <-- Add this
}

impl<'src> Parser<'src> {
    pub fn new(source: &'src str, string_dict: &'src mut StringDict) -> Self {
        // ...
    }
}
```

Update `parse_identifier()`:

```rust
fn parse_identifier(&mut self) -> Result<Identifier, JsError> {
    match &self.current.kind {
        TokenKind::Identifier(name) => {
            // Before: let name = JsString::from(name.as_str());
            // After:
            let name = self.string_dict.get_or_insert(name.as_str());
            let span = self.current.span;
            self.advance();
            Ok(Identifier { name, span })
        }
        // ... rest unchanged
    }
}
```

Update all other string creation sites in parser:
- `parse_identifier_name()` (~line 3198)
- `parse_string_literal()` (~line 3293)
- `parse_property_name()` (~line 3261)
- `keyword_to_js_string()` (~line 3460)

**File: `src/parser.rs` changes summary:**

| Function | Line | Change |
|----------|------|--------|
| `Parser::new()` | ~130 | Add `string_dict` parameter |
| `parse_identifier()` | ~3168 | Use `string_dict.get_or_insert()` |
| `parse_identifier_name()` | ~3201 | Use `string_dict.get_or_insert()` |
| `parse_string_literal()` | ~3296 | Use `string_dict.get_or_insert()` |
| `parse_property_name()` | ~3268 | Use `string_dict.get_or_insert()` |
| `keyword_to_js_string()` | ~3461 | Use `string_dict.get_or_insert()` |

**Estimated changes:** ~50 lines, 1 file

### Phase 3: Update Public API

**File: `src/lib.rs`**

Update `parse()` function signature:

```rust
pub fn parse(source: &str, string_dict: &mut StringDict) -> Result<Program, JsError> {
    let mut parser = Parser::new(source, string_dict);
    parser.parse_program()
}
```

Update `Runtime` struct:

```rust
pub struct Runtime {
    interpreter: Interpreter,
    env_arena: EnvironmentArena,
    string_dict: StringDict,  // <-- Add this
    // ...
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            interpreter: Interpreter::new(),
            env_arena: EnvironmentArena::new(),
            string_dict: StringDict::with_common_strings(),  // <-- Pre-populate
            // ...
        }
    }

    pub fn eval(&mut self, source: &str) -> Result<RuntimeResult, JsError> {
        let program = parse(source, &mut self.string_dict)?;
        // ... pass string_dict to interpreter
    }
}
```

**Estimated changes:** ~30 lines, 1 file

### Phase 4: Integrate with Interpreter

**File: `src/interpreter/mod.rs`**

Add dictionary reference to Interpreter:

```rust
pub struct Interpreter<'dict> {
    // ... existing fields
    string_dict: &'dict mut StringDict,
}
```

Use dictionary for PropertyKey creation:

```rust
// Before (creates new JsString each time):
obj.set_property(PropertyKey::from("length"), value);

// After (reuses existing JsString):
let key = PropertyKey::String(self.string_dict.get_or_insert("length"));
obj.set_property(key, value);
```

**High-impact locations in interpreter:**

| Location | Approx Line | Current Code | Frequency |
|----------|-------------|--------------|-----------|
| Property access | Various | `PropertyKey::from("length")` | ~61 sites |
| Builtin registration | ~200-400 | `env.define("undefined", ...)` | ~20 sites |
| Method calls | Various | `PropertyKey::from("toString")` | ~30 sites |

**Estimated changes:** ~150 lines, 1 file

### Phase 5: Update Builtin Modules

**Files: `src/interpreter/builtins/*.rs`**

Each builtin file creates property keys for method registration:

```rust
// Before:
proto.set_property(PropertyKey::from("push"), push_fn);

// After:
proto.set_property(
    PropertyKey::String(string_dict.get_or_insert("push")),
    push_fn
);
```

**Files to update:**

| File | Estimated PropertyKey::from calls |
|------|-----------------------------------|
| `array.rs` | ~40 |
| `string.rs` | ~35 |
| `object.rs` | ~15 |
| `number.rs` | ~10 |
| `math.rs` | ~30 |
| `date.rs` | ~25 |
| `regexp.rs` | ~10 |
| `map.rs` | ~10 |
| `set.rs` | ~10 |
| `function.rs` | ~5 |
| `error.rs` | ~10 |
| `symbol.rs` | ~5 |
| `json.rs` | ~5 |
| `console.rs` | ~5 |
| `global.rs` | ~10 |

**Total:** ~225 sites

**Estimated changes:** ~300 lines across 15 files

### Phase 6: Add Helper Methods

To reduce verbosity, add helper methods:

**File: `src/value.rs`**

```rust
impl PropertyKey {
    /// Create a string property key using the dictionary.
    pub fn from_dict(s: &str, dict: &mut StringDict) -> Self {
        PropertyKey::String(dict.get_or_insert(s))
    }
}
```

**File: `src/interpreter/mod.rs`**

```rust
impl<'dict> Interpreter<'dict> {
    /// Get a JsString from the dictionary.
    fn str(&mut self, s: &str) -> JsString {
        self.string_dict.get_or_insert(s)
    }

    /// Create a PropertyKey from the dictionary.
    fn key(&mut self, s: &str) -> PropertyKey {
        PropertyKey::String(self.string_dict.get_or_insert(s))
    }
}

// Usage:
obj.set_property(self.key("length"), value);
```

**Estimated changes:** ~20 lines, 2 files

### Phase 7: Update Tests

**File: `tests/interpreter/main.rs`**

Update test helper:

```rust
pub fn eval(source: &str) -> JsValue {
    let mut runtime = Runtime::new();
    runtime.eval_simple(source).expect("eval failed")
}
```

No changes needed if `Runtime::new()` handles dictionary internally.

**Parser tests in `src/parser.rs`:**

```rust
#[cfg(test)]
mod tests {
    fn parse(source: &str) -> Result<Program, JsError> {
        let mut dict = StringDict::new();
        super::parse(source, &mut dict)
    }

    // ... existing tests should work unchanged
}
```

**Estimated changes:** ~20 lines, 2 files

## Summary of Changes

| Phase | Files | Lines Changed | Description |
|-------|-------|---------------|-------------|
| 1 | 2 | ~100 | Create StringDict module |
| 2 | 1 | ~50 | Integrate with Parser |
| 3 | 1 | ~30 | Update public API |
| 4 | 1 | ~150 | Integrate with Interpreter |
| 5 | 15 | ~300 | Update builtin modules |
| 6 | 2 | ~20 | Add helper methods |
| 7 | 2 | ~20 | Update tests |
| **Total** | **~20** | **~670** | |

## Performance Expectations

### Memory

- **Before:** N occurrences of "length" = N × `Rc<str>` allocations
- **After:** N occurrences of "length" = 1 × `Rc<str>` + N × `Rc` clones (cheap)

**Expected reduction:** 30-50% fewer string allocations for typical code

### Speed

- **Dictionary lookup:** O(1) hash lookup per string
- **Cheap clone:** O(1) reference count increment
- **Trade-off:** Small overhead for lookup, saved allocation time

**Expected impact:** Neutral to slight improvement (allocation savings offset lookup cost)

### Cache Locality

- All strings in dictionary stored contiguously (in HashMap)
- Frequently used strings more likely to be in cache

## Testing Strategy

### Unit Tests for StringDict

```rust
#[test]
fn test_string_dict_deduplication() {
    let mut dict = StringDict::new();
    let s1 = dict.get_or_insert("hello");
    let s2 = dict.get_or_insert("hello");

    // Same Rc pointer
    assert!(Rc::ptr_eq(&s1.0, &s2.0));
}

#[test]
fn test_string_dict_different_strings() {
    let mut dict = StringDict::new();
    let s1 = dict.get_or_insert("hello");
    let s2 = dict.get_or_insert("world");

    // Different Rc pointers
    assert!(!Rc::ptr_eq(&s1.0, &s2.0));
}

#[test]
fn test_common_strings_preloaded() {
    let dict = StringDict::with_common_strings();
    assert!(dict.get("length").is_some());
    assert!(dict.get("prototype").is_some());
}
```

### Integration Tests

Existing interpreter tests should pass unchanged - the optimization is internal.

### Benchmarking

```rust
// Before/after comparison
fn bench_parse_large_file() {
    let source = include_str!("large_test.ts");
    // Measure: time, memory allocations
}

fn bench_eval_many_property_accesses() {
    let source = r#"
        let obj = { length: 1 };
        for (let i = 0; i < 10000; i++) {
            obj.length;
        }
    "#;
    // Measure: time
}
```

## Migration Path

1. **Phase 1-3:** Core infrastructure (can be done in one PR)
2. **Phase 4-5:** Interpreter integration (separate PR, larger scope)
3. **Phase 6-7:** Polish and testing (final PR)

Each phase can be tested independently. The optimization is backward-compatible - existing behavior is preserved, just with better memory sharing.

## Future Enhancements

### String Interning for Runtime Strings

Currently, runtime string concatenation creates new strings:

```javascript
let s = "hello" + " world";  // Creates new JsString
```

Could extend dictionary to intern runtime strings:

```rust
impl Interpreter {
    fn concat_strings(&mut self, a: &JsString, b: &JsString) -> JsString {
        let result = format!("{}{}", a, b);
        self.string_dict.get_or_insert(&result)
    }
}
```

**Trade-off:** More memory sharing vs. dictionary growth during execution.

### Weak References for Cleanup

If memory becomes a concern, could use weak references:

```rust
pub struct StringDict {
    strings: FxHashMap<Box<str>, Weak<str>>,
}
```

Entries with no strong references would be cleaned up. More complex but allows garbage collection of unused strings.

## Conclusion

The shared string dictionary provides a clean, low-risk optimization that:

1. Reduces memory allocations for duplicate strings
2. Maintains the existing `JsString` API
3. Can be implemented incrementally
4. Has minimal performance overhead
5. Is easy to test and debug

The implementation touches ~20 files with ~670 lines changed, primarily mechanical updates to pass the dictionary through the pipeline.
