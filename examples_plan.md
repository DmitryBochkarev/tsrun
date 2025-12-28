# tsrun Examples Plan

This document outlines proposed example projects to demonstrate the capabilities of the `tsrun` Rust library for evaluating TypeScript code.

## CLI Tool Overview

**Name:** `tsrun`

A simple CLI that executes TypeScript files using the `tsrun` library. Each example project demonstrates different language features and use cases.

```bash
# Usage
tsrun <entry-point.ts>
tsrun examples/config-generator/main.ts
```

---

## Proposed Example Projects

### 1. Config Generator (Recommended Starting Example)

**Purpose:** Demonstrate the primary use case - generating configuration files from TypeScript.

**Features Showcased:**
- Object literals and interfaces
- Type annotations (parsed but stripped)
- JSON.stringify output
- Template literals
- Spread operator
- Default values

**Files:**
```
examples/config-generator/
├── main.ts          # Entry point
├── defaults.ts      # Default configuration values
└── types.ts         # TypeScript interfaces (for documentation)
```

**Sample Output:** JSON configuration object

---

### 2. Data Transformation Pipeline

**Purpose:** Show functional programming capabilities with array methods.

**Features Showcased:**
- Array methods (map, filter, reduce, flatMap)
- Arrow functions
- Destructuring
- Method chaining
- Spread operator in arrays

**Files:**
```
examples/data-pipeline/
├── main.ts          # Pipeline orchestration
├── data.ts          # Sample data array
└── transforms.ts    # Transformation functions
```

**Sample Output:** Transformed data array

---

### 3. Class-Based Domain Model

**Purpose:** Demonstrate OOP features with a simple domain model.

**Features Showcased:**
- Classes with inheritance
- Private fields (#field)
- Static methods and fields
- Getters and setters
- Constructor parameters
- super() calls

**Files:**
```
examples/domain-model/
├── main.ts          # Usage example
├── entity.ts        # Base Entity class
├── user.ts          # User extends Entity
└── order.ts         # Order with items
```

**Sample Output:** Serialized domain objects

---

### 4. Math/Algorithm Showcase

**Purpose:** Demonstrate numeric operations and algorithms.

**Features Showcased:**
- Math object methods
- Number methods
- Recursion
- Control flow (for, while, if)
- Bitwise operators

**Files:**
```
examples/algorithms/
├── main.ts          # Run all algorithms
├── fibonacci.ts     # Recursive and iterative
├── sorting.ts       # QuickSort implementation
├── primes.ts        # Prime number sieve
└── statistics.ts    # Mean, median, std deviation
```

**Sample Output:** Algorithm results

---

### 5. Async/Await Demo

**Purpose:** Show async programming capabilities.

**Features Showcased:**
- async/await syntax
- Promise.all, Promise.race
- Promise chaining
- Error handling in async code
- Dynamic import()

**Files:**
```
examples/async-demo/
├── main.ts          # Async orchestration
├── fetcher.ts       # Simulated async data fetch
└── processor.ts     # Async data processing
```

**Sample Output:** Results from async operations

---

### 6. Generator Functions

**Purpose:** Demonstrate iterator and generator support.

**Features Showcased:**
- function* syntax
- yield and yield*
- for...of iteration
- Custom iterables
- Lazy evaluation

**Files:**
```
examples/generators/
├── main.ts          # Generator usage
├── range.ts         # Range generator
├── fibonacci.ts     # Infinite fibonacci sequence
└── tree.ts          # Tree traversal generator
```

**Sample Output:** Generated sequences

---

### 7. RegExp Text Processing

**Purpose:** Show regular expression capabilities.

**Features Showcased:**
- RegExp constructor and literals
- test(), exec() methods
- String match(), replace(), split()
- Global and case-insensitive flags
- Capture groups

**Files:**
```
examples/text-processing/
├── main.ts          # Text processing demo
├── parser.ts        # Simple markup parser
├── validator.ts     # Email/URL validation
└── formatter.ts     # String formatting
```

**Sample Output:** Processed text results

---

### 8. Date/Time Utilities

**Purpose:** Demonstrate Date object functionality.

**Features Showcased:**
- Date constructor variations
- Date getters and setters
- Date arithmetic
- ISO string formatting
- UTC methods

**Files:**
```
examples/datetime/
├── main.ts          # Date utility demo
├── calendar.ts      # Simple calendar generator
└── duration.ts      # Duration calculations
```

**Sample Output:** Formatted dates and calendar data

---

### 9. Map/Set Collections Demo

**Purpose:** Show Map and Set data structures.

**Features Showcased:**
- Map and Set construction
- Iteration with forEach and for...of
- Set operations (union, intersection)
- Using objects as Map keys
- Size property

**Files:**
```
examples/collections/
├── main.ts          # Collections demo
├── graph.ts         # Graph using Map<node, Set<neighbor>>
└── counter.ts       # Word frequency counter
```

**Sample Output:** Collection operation results

---

### 10. TypeScript Enum and Namespace

**Purpose:** Demonstrate TypeScript-specific features.

**Features Showcased:**
- enum declarations (compiled to objects)
- const enum (inlined values)
- namespace declarations
- Namespace merging
- Export from namespace

**Files:**
```
examples/typescript-features/
├── main.ts          # Feature demo
├── enums.ts         # Various enum patterns
└── namespaces.ts    # Namespace organization
```

**Sample Output:** Enum values and namespace exports

---

### 11. Error Handling Showcase

**Purpose:** Demonstrate try/catch/finally and custom errors.

**Features Showcased:**
- try/catch/finally blocks
- throw statement
- Error types (TypeError, RangeError, etc.)
- Error stack traces
- Custom error patterns

**Files:**
```
examples/error-handling/
├── main.ts          # Error handling demo
├── validators.ts    # Validation with errors
└── safe-ops.ts      # Safe operation wrappers
```

**Sample Output:** Caught and handled errors

---

### 12. JSON Processing

**Purpose:** Show JSON parsing and stringification.

**Features Showcased:**
- JSON.parse with reviver
- JSON.stringify with replacer and space
- Deep object manipulation
- Schema validation pattern

**Files:**
```
examples/json-processing/
├── main.ts          # JSON processing demo
├── schema.ts        # Schema validation
└── transform.ts     # JSON transformations
```

**Sample Output:** Processed JSON data

---

## Implementation Priority

**Phase 1 - Core Examples (Essential):**
1. Config Generator - Primary use case
2. Data Transformation Pipeline - Functional programming
3. Class-Based Domain Model - OOP support

**Phase 2 - Language Features:**
4. Math/Algorithm Showcase
5. Async/Await Demo
6. Generator Functions

**Phase 3 - Built-ins:**
7. RegExp Text Processing
8. Date/Time Utilities
9. Map/Set Collections

**Phase 4 - TypeScript Specific:**
10. TypeScript Enum and Namespace
11. Error Handling Showcase
12. JSON Processing

---

## CLI Implementation Notes

The `tsrun` CLI needs to:

1. **Parse command-line arguments** - Entry point file path
2. **Read the entry file** - Load TypeScript source
3. **Handle imports** - Resolve relative imports from the same directory
4. **Execute with Runtime** - Use the `RuntimeResult` loop pattern
5. **Output results** - Print the final value or exports to stdout
6. **Handle errors** - Display error messages with source locations

### Module Resolution Strategy

For these examples, use simple relative path resolution:
- `import { x } from "./file"` resolves to `./file.ts` in same directory
- No node_modules resolution needed
- No package.json support needed

### Example CLI Loop

```rust
let mut runtime = Runtime::new();
let mut result = runtime.eval(&source)?;

loop {
    match result {
        RuntimeResult::Complete(value) => {
            println!("{}", format_value(&value));
            break;
        }
        RuntimeResult::ImportAwaited { slot, specifier } => {
            let module_source = read_module(&specifier)?;
            let module = runtime.eval_module(&module_source)?;
            slot.set_success(module);
        }
        RuntimeResult::AsyncAwaited { slot, .. } => {
            // For demo: resolve immediately with undefined
            slot.set_success(runtime.create_undefined());
        }
    }
    result = runtime.continue_eval()?;
}
```

---

## Directory Structure

```
tsrun/
├── Cargo.toml
├── src/
│   └── main.rs
└── examples/
    ├── config-generator/
    ├── data-pipeline/
    ├── domain-model/
    ├── algorithms/
    ├── async-demo/
    ├── generators/
    ├── text-processing/
    ├── datetime/
    ├── collections/
    ├── typescript-features/
    ├── error-handling/
    └── json-processing/
```

---

## Success Criteria

Each example should:
1. Run successfully with `tsrun examples/<name>/main.ts`
2. Produce meaningful output demonstrating the features
3. Use idiomatic TypeScript patterns
4. Include type annotations for documentation
5. Be self-contained (no external dependencies)
6. Serve as documentation for library capabilities
