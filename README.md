# tsrun

[![crates.io](https://img.shields.io/crates/v/tsrun.svg)](https://crates.io/crates/tsrun)
[![docs.rs](https://docs.rs/tsrun/badge.svg)](https://docs.rs/tsrun)

A minimal TypeScript runtime in Rust for embedding in applications.

## Overview

tsrun is designed for configuration files where you want the full benefits of TypeScript in your editor: autocompletion, type checking, and error highlighting. The runtime executes TypeScript directly without transpilation, using a register-based bytecode VM.

**Why TypeScript for configs?**
- IDE autocompletion for your config schema
- Catch errors before runtime with type checking in your editor
- Native support for enums, interfaces, and type annotations
- No Node.js dependency - embed directly in your application

## Features

### TypeScript Support

> **Note:** Types are parsed for IDE support but **not checked at runtime**. Type annotations, interfaces, and generics are stripped during execution. Use your editor's TypeScript language server for type checking.

- **Enums** - Native support with numeric and string enums, including reverse mappings
- **Type Annotations** - Full parsing of types, interfaces, type aliases, and generics
- **Decorators** - Class, method, property, and parameter decorators
- **Namespaces** - TypeScript namespace declarations
- **Parameter Properties** - `constructor(public x: number)` syntax support
- **Type Assertions** - Both `x as T` and `<T>x` syntaxes

### JavaScript Features
- **ES Modules** - Full import/export support with step-based module loading
- **Async/Await** - Promises, async functions, Promise.all/race/allSettled
- **Classes** - Inheritance, static blocks, private fields, getters/setters
- **Generators** - function*, yield, yield*, for...of iteration
- **Destructuring** - Arrays, objects, function parameters, rest/spread
- **eval()** - Dynamic code evaluation
- **Built-ins** - Array, String, Object, Map, Set, Date, RegExp, JSON, Math, Proxy, Reflect, Symbol

### Embedding
- **Minimal Runtime** - Small footprint, no Node.js dependency
- **Rust & C APIs** - Full integration support for host applications
- **WASM Support** - Run in browsers, Node.js, Go (wazero), and other WASM runtimes
- **no_std Compatible** - Can run in environments without the standard library

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
    import { order } from "tsrun:host";
    const response = await order({ url: "/api/users" });
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

## WASM

The interpreter compiles to WebAssembly with a C-style FFI, enabling use across multiple runtimes: browsers, Node.js, Go (via wazero), and others.

### Building

```bash
cd examples/wasm-playground
./build.sh              # Build WASM module
./build.sh --test       # Build and run browser tests
```

### Go Embedding (wazero)

```go
import "github.com/example/tsrun-go/tsrun"

ctx := context.Background()
rt, _ := tsrun.New(ctx, tsrun.ConsoleOption(func(level tsrun.ConsoleLevel, msg string) {
    fmt.Printf("[%s] %s\n", level, msg)
}))
defer rt.Close(ctx)

interp, _ := rt.NewContext(ctx)
defer interp.Free(ctx)

interp.Prepare(ctx, `console.log("Hello from Go!")`, "/main.ts")
result, _ := interp.Run(ctx)
```

See [examples/go-wazero/](examples/go-wazero/) for complete examples including async operations, modules, and native functions.

### Browser/Node.js API

```javascript
import init, { TsRunner, STEP_CONTINUE, STEP_COMPLETE, STEP_ERROR } from './pkg/tsrun.js';

await init();
const runner = new TsRunner();

// Status constants (functions that return values)
const Status = {
    CONTINUE: STEP_CONTINUE(),
    COMPLETE: STEP_COMPLETE(),
    ERROR: STEP_ERROR()
};

// Prepare and run
runner.prepare('console.log("Hello!"); 1 + 2', 'script.ts');

while (true) {
    const result = runner.step();

    // Display console output
    for (const entry of result.console_output) {
        console.log(`[${entry.level}] ${entry.message}`);
    }

    if (result.status === Status.COMPLETE) {
        console.log('Result:', result.value);
        break;
    } else if (result.status === Status.ERROR) {
        console.error('Error:', result.error);
        break;
    }
}
```

### Async Operations

TypeScript code can use `order` for async operations that the JavaScript host fulfills:

```typescript
import { order } from "tsrun:host";

function fetch(url: string): Promise<any> {
    return order({ type: "fetch", url });
}

const [user, posts] = await Promise.all([
    fetch("/api/users/1"),
    fetch("/api/posts")
]);
```

```javascript
// In the step loop, handle STEP_SUSPENDED status:
if (result.status === STEP_SUSPENDED()) {
    const orders = runner.get_pending_orders();
    // orders = [{ id: 1, payload: { type: "fetch", url: "/api/users/1" } }, ...]

    const responses = await Promise.all(orders.map(async order => {
        const data = await realFetch(order.payload.url);
        return { id: order.id, result: data };
    }));

    runner.fulfill_orders(responses);
}
```

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
| `wasm` | WebAssembly target support | No |

```toml
# Minimal build without regex
[dependencies]
tsrun = { version = "0.1", default-features = false, features = ["std"] }

# With C API
[dependencies]
tsrun = { version = "0.1", features = ["c-api"] }
```

```bash
# Build for WASM
cargo build --target wasm32-unknown-unknown --features wasm --no-default-features
```

## Use Case Examples

> **Note:** The type annotations in these examples provide IDE autocompletion and editor-based type checking, but tsrun does not validate types at runtime. Passing a wrong type will not throw an error - it will simply execute with whatever value is provided.

### Kubernetes Deployment Configuration

Generate type-safe Kubernetes manifests with IDE autocompletion:

```typescript
interface DeploymentConfig {
    name: string;
    image: string;
    replicas: number;
    port: number;
}

function deployment(config: DeploymentConfig) {
    return {
        apiVersion: "apps/v1",
        kind: "Deployment",
        metadata: { name: config.name },
        spec: {
            replicas: config.replicas,
            selector: { matchLabels: { app: config.name } },
            template: {
                metadata: { labels: { app: config.name } },
                spec: {
                    containers: [{
                        name: config.name,
                        image: config.image,
                        ports: [{ containerPort: config.port }]
                    }]
                }
            }
        }
    };
}

deployment({ name: "api", image: "myapp:v1.2.0", replicas: 3, port: 8080 })
```

### Game Item Configuration

Define game items with enums and computed loot tables:

```typescript
enum Rarity { Common, Rare, Epic, Legendary }

interface Item {
    name: string;
    rarity: Rarity;
    basePrice: number;
    effects?: string[];
}

function createLootTable(items: Item[]) {
    return items.map(item => ({
        ...item,
        dropWeight: item.rarity === Rarity.Legendary ? 1 :
                    item.rarity === Rarity.Epic ? 5 :
                    item.rarity === Rarity.Rare ? 15 : 50,
        sellPrice: Math.floor(item.basePrice * (1 + item.rarity * 0.5))
    }));
}

createLootTable([
    { name: "Iron Sword", rarity: Rarity.Common, basePrice: 100 },
    { name: "Dragon Scale", rarity: Rarity.Legendary, basePrice: 5000,
      effects: ["Fire Resistance", "+50 Defense"] }
])
// Result: [{ dropWeight: 50, sellPrice: 100, ... }, { dropWeight: 1, sellPrice: 12500, ... }]
```

### API Router Configuration

Configure REST endpoints with typed middleware and rate limits:

```typescript
interface Route {
    method: "GET" | "POST" | "PUT" | "DELETE";
    path: string;
    handler: string;
    middleware?: string[];
    rateLimit?: { requests: number; window: string };
}

const routes: Route[] = [
    {
        method: "GET",
        path: "/users/:id",
        handler: "users::get",
        middleware: ["auth", "cache"]
    },
    {
        method: "POST",
        path: "/users",
        handler: "users::create",
        middleware: ["auth", "validate"],
        rateLimit: { requests: 10, window: "1m" }
    },
    {
        method: "DELETE",
        path: "/users/:id",
        handler: "users::delete",
        middleware: ["auth", "admin"]
    }
];

routes
```

### Build Tool Configuration

Create plugin-based build configurations like webpack or vite:

```typescript
interface Plugin {
    name: string;
    options?: Record<string, any>;
}

interface BuildConfig {
    entry: string;
    output: { path: string; filename: string };
    plugins: Plugin[];
    minify: boolean;
}

const config: BuildConfig = {
    entry: "./src/index.ts",
    output: {
        path: "./dist",
        filename: "[name].[hash].js"
    },
    plugins: [
        { name: "typescript", options: { target: "ES2022" } },
        { name: "minify", options: { dropConsole: true } },
        { name: "bundle-analyzer" }
    ],
    minify: true
};

config
```

### Validation Schema

Define form validation schemas with discriminated unions:

```typescript
type Rule =
    | { type: "required" }
    | { type: "minLength"; value: number }
    | { type: "maxLength"; value: number }
    | { type: "pattern"; regex: string; message: string }
    | { type: "email" };

interface FieldSchema {
    name: string;
    label: string;
    rules: Rule[];
}

const userSchema: FieldSchema[] = [
    {
        name: "email",
        label: "Email Address",
        rules: [
            { type: "required" },
            { type: "email" }
        ]
    },
    {
        name: "password",
        label: "Password",
        rules: [
            { type: "required" },
            { type: "minLength", value: 8 },
            { type: "pattern", regex: "[A-Z]", message: "Must contain uppercase" }
        ]
    }
];

userSchema
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

- **No runtime type checking** - Types are parsed and stripped for IDE support, not validated at runtime
- **Strict mode only** - All code runs in strict mode
- **Single-threaded** - One interpreter instance per thread

## License

MIT
