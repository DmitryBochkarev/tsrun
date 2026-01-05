# tsrun

A TypeScript interpreter written in Rust, designed for embedding in applications.

## Overview

tsrun executes TypeScript code directly without transpilation to JavaScript. It uses a register-based bytecode VM for efficient execution. Type annotations are parsed but stripped at runtime (no type checking).

**Primary use case:** Configuration and manifest generation where you want TypeScript's syntax and module system without a Node.js dependency.

## Features

- **ES Modules** - Full import/export support with step-based module loading
- **Async/Await** - Promises, async functions, Promise.all/race/allSettled
- **Classes** - Inheritance, static blocks, private fields, getters/setters
- **Generators** - function*, yield, yield*, for...of iteration
- **Destructuring** - Arrays, objects, function parameters, rest/spread
- **Built-ins** - Array, String, Object, Map, Set, Date, RegExp, JSON, Math, Proxy, Reflect, Symbol
- **Embeddable** - Rust and C APIs for integration into host applications
- **no_std compatible** - Can run in environments without the standard library

## Installation

### CLI

```bash
cargo install tsrun
```

### Library (Rust)

```toml
[dependencies]
tsrun = "0.1"
```

### C/C++ Embedding

```bash
cargo build --release --features c-api
# Produces: target/release/libtsrun.so (Linux), .dylib (macOS), .dll (Windows)
```

## Quick Start

### CLI

```bash
# Run a TypeScript file
tsrun script.ts

# With ES modules
tsrun main.ts  # automatically resolves imports
```

### Rust Library

```rust
use tsrun::{Interpreter, StepResult};

fn main() -> Result<(), tsrun::JsError> {
    let mut interp = Interpreter::new();

    // Prepare code for execution
    interp.prepare("1 + 2 * 3", None)?;

    // Step until completion
    loop {
        match interp.step()? {
            StepResult::Continue => continue,
            StepResult::Complete(value) => {
                println!("Result: {}", value.as_number().unwrap()); // 7.0
                break;
            }
            _ => break,
        }
    }
    Ok(())
}
```

### C Embedding

```c
#include "tsrun.h"

int main() {
    TsRunContext* ctx = tsrun_new();

    tsrun_prepare(ctx, "1 + 2 * 3", NULL);
    TsRunStepResult result = tsrun_run(ctx);

    if (result.status == TSRUN_STEP_COMPLETE) {
        printf("Result: %g\n", tsrun_get_number(result.value));
        tsrun_value_free(result.value);
    }

    tsrun_step_result_free(&result);
    tsrun_free(ctx);
    return 0;
}
```

## Rust API

### Basic Execution

```rust
use tsrun::{Interpreter, StepResult, JsValue};

let mut interp = Interpreter::new();
interp.prepare(r#"
    const greeting = "Hello";
    const target = "World";
    `${greeting}, ${target}!`
"#, None)?;

loop {
    match interp.step()? {
        StepResult::Continue => continue,
        StepResult::Complete(value) => {
            assert_eq!(value.as_str(), Some("Hello, World!"));
            break;
        }
        _ => break,
    }
}
```

### ES Module Loading

The interpreter uses step-based execution that pauses when imports are needed:

```rust
use tsrun::{Interpreter, StepResult, ModulePath};

let mut interp = Interpreter::new();

// Main module with imports
interp.prepare(r#"
    import { add } from "./math.ts";
    export const result = add(2, 3);
"#, Some(ModulePath::new("/main.ts")))?;

loop {
    match interp.step()? {
        StepResult::Continue => continue,
        StepResult::NeedImports(imports) => {
            for import in imports {
                // Load module source from filesystem, network, etc.
                let source = match import.resolved_path.as_str() {
                    "/math.ts" => "export function add(a: number, b: number) { return a + b; }",
                    _ => panic!("Unknown module"),
                };
                interp.provide_module(import.resolved_path, source)?;
            }
        }
        StepResult::Complete(value) => {
            println!("Done: {}", value);
            break;
        }
        _ => break,
    }
}
```

### Working with Values

```rust
use tsrun::{Interpreter, api, JsValue};
use serde_json::json;

let mut interp = Interpreter::new();
let guard = api::create_guard(&interp);

// Create values from JSON
let user = api::create_from_json(&mut interp, &guard, &json!({
    "name": "Alice",
    "age": 30,
    "tags": ["admin", "developer"]
}))?;

// Read properties
let name = api::get_property(&user, "name")?;
assert_eq!(name.as_str(), Some("Alice"));

// Modify properties
api::set_property(&user, "email", JsValue::from("alice@example.com"))?;

// Call methods
let tags = api::get_property(&user, "tags")?;
let joined = api::call_method(&mut interp, &guard, &tags, "join", &[JsValue::from(", ")])?;
assert_eq!(joined.as_str(), Some("admin, developer"));
```

### Async/Await with Orders

For async operations, the interpreter pauses with pending "orders" that the host fulfills:

```rust
use tsrun::{Interpreter, StepResult, OrderResponse, RuntimeValue, api};

let mut interp = Interpreter::new();

// Code that uses the order system for async I/O
interp.prepare(r#"
    import { request } from "eval:internal";
    const response = await request({ url: "/api/users" });
    response.data
"#, Some("/main.ts".into()))?;

loop {
    match interp.step()? {
        StepResult::Continue => continue,
        StepResult::Suspended { pending, cancelled } => {
            let mut responses = Vec::new();
            for order in pending {
                // Examine order.payload to determine what to do
                // Create response value
                let response = api::create_response_object(&mut interp, &serde_json::json!({
                    "data": [{"id": 1, "name": "Alice"}]
                }))?;
                responses.push(OrderResponse {
                    id: order.id,
                    result: Ok(response),
                });
            }
            interp.fulfill_orders(responses);
        }
        StepResult::Complete(value) => {
            println!("Result: {}", value);
            break;
        }
        _ => break,
    }
}
```

### Accessing Module Exports

```rust
use tsrun::{Interpreter, StepResult, api};

let mut interp = Interpreter::new();
interp.prepare(r#"
    export const VERSION = "1.0.0";
    export const CONFIG = { debug: true };
"#, Some("/config.ts".into()))?;

// Run to completion
loop {
    match interp.step()? {
        StepResult::Continue => continue,
        StepResult::Complete(_) => break,
        _ => break,
    }
}

// Access exports
let version = api::get_export(&interp, "VERSION");
assert_eq!(version.unwrap().as_str(), Some("1.0.0"));

let export_names = api::get_export_names(&interp);
assert!(export_names.contains(&"VERSION".to_string()));
assert!(export_names.contains(&"CONFIG".to_string()));
```

## C API

See [examples/c-embedding/](examples/c-embedding/) for complete examples.

### Native Functions

Register C functions callable from JavaScript:

```c
static TsRunValue* native_add(TsRunContext* ctx, TsRunValue* this_arg,
                              TsRunValue** args, size_t argc,
                              void* userdata, const char** error_out) {
    double a = tsrun_get_number(args[0]);
    double b = tsrun_get_number(args[1]);
    return tsrun_number(ctx, a + b);
}

// Register
TsRunValueResult fn = tsrun_native_function(ctx, "add", native_add, 2, NULL);
tsrun_set_global(ctx, "add", fn.value);

// Use from JS: add(10, 20) -> 30
```

### Module Loading

```c
TsRunStepResult result = tsrun_run(ctx);

while (result.status == TSRUN_STEP_NEED_IMPORTS) {
    for (size_t i = 0; i < result.import_count; i++) {
        const char* path = result.imports[i].resolved_path;
        const char* source = load_from_filesystem(path);
        tsrun_provide_module(ctx, path, source);
    }
    tsrun_step_result_free(&result);
    result = tsrun_run(ctx);
}
```

## Feature Flags

| Flag | Description | Default |
|------|-------------|---------|
| `std` | Full standard library support | Yes |
| `regex` | Regular expression support (requires `std`) | Yes |
| `console` | Console.log builtin | Yes |
| `c-api` | C FFI for embedding (requires `std`) | No |

```toml
# Minimal build without regex
[dependencies]
tsrun = { version = "0.1", default-features = false, features = ["std"] }

# With C API
[dependencies]
tsrun = { version = "0.1", features = ["c-api"] }
```

## TypeScript Examples

### Classes with Inheritance

```typescript
class Entity {
    static #count = 0;
    #id: number;

    constructor() {
        this.#id = ++Entity.#count;
    }

    get id() { return this.#id; }
    static getCount() { return Entity.#count; }
}

class User extends Entity {
    constructor(public name: string, public email: string) {
        super();
    }

    toJSON() {
        return { id: this.id, name: this.name, email: this.email };
    }
}

const user = new User("Alice", "alice@example.com");
JSON.stringify(user.toJSON());
```

### Generators

```typescript
function* fibonacci(): Generator<number> {
    let [a, b] = [0, 1];
    while (true) {
        yield a;
        [a, b] = [b, a + b];
    }
}

function* take<T>(gen: Generator<T>, n: number): Generator<T> {
    for (const value of gen) {
        if (n-- <= 0) return;
        yield value;
    }
}

const first10 = [...take(fibonacci(), 10)];
// [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]
```

### Async/Await

```typescript
async function fetchUserWithPosts(userId: number) {
    const [user, posts] = await Promise.all([
        fetchUser(userId),
        fetchUserPosts(userId)
    ]);
    return { user, posts };
}

async function processAll<T, R>(
    items: T[],
    fn: (item: T) => Promise<R>
): Promise<R[]> {
    const results: R[] = [];
    for (const item of items) {
        results.push(await fn(item));
    }
    return results;
}
```

### Config Generation

```typescript
import { DEFAULT_CONFIG } from "./defaults";

const envOverrides = {
    production: {
        database: { host: "prod-db.example.com", ssl: true },
        logging: { level: "warn" }
    },
    development: {
        database: { host: "localhost" },
        logging: { level: "debug" }
    }
};

function buildConfig(env: string) {
    return {
        ...DEFAULT_CONFIG,
        ...(envOverrides[env] || {})
    };
}

JSON.stringify(buildConfig("production"), null, 2);
```

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_array_map

# Run with output
cargo test -- --nocapture
```

### Test262 Conformance

```bash
git submodule update --init --depth 1
cargo build --release --bin test262-runner
./target/release/test262-runner --strict-only language/types
```

## Performance

The interpreter uses a register-based bytecode VM:
- Fewer instructions than stack-based VMs
- Better cache locality
- Efficient state capture for async/generators

Release builds use LTO and are optimized for size (`opt-level = "z"`).

## Limitations

- **No type checking** - Types are parsed and stripped, not validated
- **Strict mode only** - All code runs in strict mode
- **No eval()** - Dynamic code evaluation is not supported
- **Single-threaded** - One interpreter instance per thread

## License

MIT
