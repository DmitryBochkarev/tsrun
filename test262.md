# Test262 Conformance Testing

This document describes how to run the [Test262](https://github.com/tc39/test262) ECMAScript conformance test suite against the typescript-eval interpreter.

## Setup

The test262 suite is included as a git submodule. Initialize it with:

```bash
git submodule update --init --depth 1
```

Or clone directly:

```bash
git clone --depth 1 https://github.com/tc39/test262 test262
```

## Running Tests

Build the test runner:

```bash
cargo build --release --bin test262-runner
```

### Basic Usage

```bash
# Run all tests in a directory
./target/release/test262-runner language/types

# Run with verbose output (shows each test)
./target/release/test262-runner --verbose language/literals/numeric

# Stop on first failure (good for debugging)
./target/release/test262-runner --verbose --stop-on-fail language/statements

# List tests without running
./target/release/test262-runner --list language/expressions/addition

# Filter by pattern within directory
./target/release/test262-runner --filter "array" built-ins/Array
```

### Options

| Option | Description |
|--------|-------------|
| `--test262-dir <PATH>` | Path to test262 directory (default: `./test262`) |
| `--filter <PATTERN>` | Filter tests by path pattern |
| `--features <LIST>` | Only run tests requiring these features (comma-separated) |
| `--skip-features <LIST>` | Skip tests requiring these features (comma-separated) |
| `--verbose` / `-v` | Show detailed output for each test |
| `--stop-on-fail` | Stop on first failure |
| `--list` | List matching tests without running |
| `--strict-only` | Only run strict mode variants |
| `--non-strict-only` | Only run non-strict mode variants |

### Strict Mode Only

**Important:** The typescript-eval interpreter runs all code in strict mode. This means:

- Tests marked `noStrict` will fail (they rely on sloppy mode behavior)
- Tests marked `onlyStrict` are the most relevant for this interpreter
- The `--strict-only` flag is recommended for accurate pass rates

For meaningful results, run with strict mode only:

```bash
./target/release/test262-runner --strict-only language/types
```

## Current Test Results

Results as of the current implementation (run with `--strict-only` flag):

### Language Features

| Category | Pass | Fail | Skip | Pass Rate |
|----------|------|------|------|-----------|
| `language/function-code` | 88 | 20 | 0 | 81.5% |
| `language/identifiers` | 182 | 86 | 0 | 67.9% |
| `language/types` | 68 | 36 | 0 | 65.4% |
| `language/literals/string` | 32 | 29 | 0 | 52.5% |
| `language/literals/numeric` | 80 | 75 | 0 | 51.6% |
| `language/arguments-object` | 72 | 74 | 60 | 49.3% |
| `language/comments` | 12 | 17 | 1 | 41.4% |
| `language/expressions` | 2729 | 5351 | 2477 | 33.8% |
| `language/statements` | 1991 | 4308 | 2539 | 31.6% |
| `language/block-scope` | 34 | 111 | 0 | 23.4% |

### Built-in Objects

| Category | Pass | Fail | Skip | Pass Rate |
|----------|------|------|------|-----------|
| `built-ins/Math` | 135 | 192 | 0 | 41.3% |
| `built-ins/Number` | 110 | 224 | 1 | 32.9% |
| `built-ins/Object` | 1093 | 2285 | 22 | 32.4% |
| `built-ins/RegExp` | 340 | 799 | 739 | 29.9% |
| `built-ins/String` | 348 | 858 | 3 | 28.9% |
| `built-ins/Set` | 46 | 141 | 195 | 24.6% |
| `built-ins/Function` | 101 | 319 | 1 | 24.0% |
| `built-ins/Map` | 33 | 155 | 15 | 17.6% |
| `built-ins/Promise` | 39 | 235 | 364 | 14.2% |
| `built-ins/Symbol` | 10 | 82 | 0 | 10.9% |

## Skipped Features

The test runner automatically skips tests requiring features we don't support:

### Not Implemented
- `BigInt` - Arbitrary precision integers
- `WeakRef`, `WeakMap`, `WeakSet` - Weak references
- `FinalizationRegistry` - Cleanup callbacks
- `Atomics`, `SharedArrayBuffer` - Shared memory
- `TypedArray`, `ArrayBuffer`, `DataView` - Binary data
- `Temporal` - Date/time proposal
- `ShadowRealm` - Isolated realms
- `decorators` - Class decorators proposal

### Intentionally Unsupported
- `eval` - Dynamic code execution
- `with` statement - Deprecated feature
- `tail-call-optimization` - Requires special runtime support
- `Intl` - Internationalization API

### Parser Limitations
- `import-assertions`, `import-attributes` - Import attributes
- `json-modules` - JSON imports
- `regexp-lookbehind` - RegExp lookbehind assertions
- `regexp-named-groups` - RegExp named capture groups
- `regexp-unicode-property-escapes` - Unicode property escapes
- `top-level-await` - Module-level await

## Common Failure Patterns

### 1. Missing `eval()` function
Many tests use `eval()` to test whitespace/unicode handling:
```javascript
// Test expects eval to be available
if (eval("1\u0009+\u00091") !== 2) { ... }
```

### 2. Numeric literal edge cases
Some numeric literal formats like `.1` (leading dot) may not be fully supported:
```javascript
// Test for leading dot notation
.1 === 0.1
```

### 3. Unicode identifiers
Tests for extended unicode identifier characters:
```javascript
// Unicode escape in identifier
var \u0078 = 1;
```

### 4. Strict mode edge cases
Some strict mode behaviors differ slightly from spec:
```javascript
// Strict mode this binding
"use strict";
function f() { return this; }  // Should return undefined
```

### 5. Property descriptors
Object property descriptor behaviors (enumerable, configurable, writable):
```javascript
Object.defineProperty(obj, 'x', { value: 1, writable: false });
```

## Adding Custom Skip Features

To skip additional features:

```bash
./target/release/test262-runner --skip-features "Promise,async-functions" language/
```

To run only tests requiring specific features:

```bash
./target/release/test262-runner --features "arrow-function" language/expressions/arrow-function
```

## Test File Format

Test262 tests use YAML frontmatter for metadata:

```javascript
/*---
description: Test description
features: [arrow-function, const]
flags: [onlyStrict]
negative:
  phase: parse
  type: SyntaxError
includes: [propertyHelper.js]
---*/

// Test code here
```

### Flags

| Flag | Description |
|------|-------------|
| `onlyStrict` | Run only in strict mode |
| `noStrict` | Run only in non-strict mode |
| `module` | Interpret as ES module |
| `raw` | Don't add harness files |
| `async` | Async test (requires `print()` callback) |

### Negative Tests

Tests expected to throw errors specify:
- `phase`: `parse`, `resolution`, or `runtime`
- `type`: Expected error constructor name

## Debugging Failures

To investigate a specific failure:

```bash
# Run single test with verbose output
./target/release/test262-runner --verbose --stop-on-fail \
  test262/test/language/expressions/addition/S11.6.1_A1.js

# View the test file
cat test262/test/language/expressions/addition/S11.6.1_A1.js
```

## Harness Files

The test runner automatically loads these harness files before each test:
- `harness/sta.js` - Test262Error class and $DONOTEVALUATE
- `harness/assert.js` - Assertion functions (assert, assert.sameValue, etc.)

Additional harness files are loaded based on the `includes` metadata.

## Performance

Release build is recommended for running large test suites:

```bash
# Debug build: ~1.19s for 485 tests
./target/debug/test262-runner language/expressions/assignment

# Release build: ~0.28s for 485 tests
./target/release/test262-runner language/expressions/assignment
```

## Contributing

When adding new features:

1. Run relevant test262 tests to check compliance
2. Document any intentional deviations from spec
3. Update this file with new pass rates
