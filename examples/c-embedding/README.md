# C Embedding Examples

This directory contains examples of embedding the tsrun TypeScript interpreter in C/C++ applications.

## Overview

tsrun provides a C API that allows you to:

- Evaluate TypeScript/JavaScript code
- Inspect and manipulate values
- Register native C functions callable from JS
- Handle ES module imports
- Process async operations via the order system

## Building

### 1. Build the Rust library with C API support

```bash
cd ../..  # Go to project root
cargo build --release --features c-api
```

This produces:
- `target/release/libtsrun.so` (Linux)
- `target/release/libtsrun.dylib` (macOS)
- `target/release/libtsrun.a` (static library)

### 2. Build the examples

```bash
cd examples/c-embedding
make all
```

### 3. Run the examples

```bash
make run-all
# Or individual examples:
make run-basic
make run-native
make run-modules
make run-async
```

## Examples

### `basic.c` - Basic Usage

Demonstrates fundamental operations:
- Creating and freeing interpreter contexts
- Evaluating expressions
- Inspecting return values (type checking, extracting primitives)
- Working with objects and arrays
- JSON serialization

```c
TsRunContext* ctx = tsrun_new();

TsRunResult prep = tsrun_prepare(ctx, "1 + 2 * 3", NULL);
TsRunStepResult result = tsrun_run(ctx);

if (result.status == TSRUN_STEP_COMPLETE) {
    printf("Result: %f\n", tsrun_get_number(result.value));
    tsrun_value_free(result.value);
}

tsrun_step_result_free(&result);
tsrun_free(ctx);
```

### `native_functions.c` - Native Functions

Demonstrates registering C functions that can be called from JavaScript:
- Simple functions (add, greet)
- Functions returning objects
- Stateful functions using userdata
- Functions that call back into JS

```c
static TsRunValue* native_add(TsRunContext* ctx, TsRunValue* this_arg,
                              TsRunValue** args, size_t argc,
                              void* userdata, const char** error_out) {
    double a = tsrun_get_number(args[0]);
    double b = tsrun_get_number(args[1]);
    return tsrun_number(ctx, a + b);
}

// Register and use
TsRunValueResult fn = tsrun_native_function(ctx, "add", native_add, 2, NULL);
tsrun_set_global(ctx, "add", fn.value);
// Now callable from JS: add(10, 20)
```

### `module_loading.c` - Module System

Demonstrates handling ES module imports:
- Step-based execution with `TSRUN_STEP_NEED_IMPORTS`
- Providing module source code
- Accessing module exports from C

```c
TsRunStepResult result = tsrun_run(ctx);

while (result.status == TSRUN_STEP_NEED_IMPORTS) {
    for (size_t i = 0; i < result.import_count; i++) {
        const char* path = result.imports[i].resolved_path;
        const char* source = load_file(path);
        tsrun_provide_module(ctx, path, source);
    }
    tsrun_step_result_free(&result);
    result = tsrun_run(ctx);
}
```

### `async_orders.c` - Async Operations

Demonstrates the order system for async operations:
- Handling `TSRUN_STEP_SUSPENDED` status
- Processing orders (requests from JS)
- Fulfilling orders with responses
- Handling cancellation

```c
while (result.status == TSRUN_STEP_SUSPENDED) {
    for (size_t i = 0; i < result.pending_count; i++) {
        TsRunOrder* order = &result.pending_orders[i];
        // Examine order->payload to determine what to do
        // Create response...
    }
    tsrun_fulfill_orders(ctx, responses, count);
    result = tsrun_run(ctx);
}
```

## API Reference

See `tsrun.h` for the complete API. Key types:

### Result Types

```c
typedef struct {
    TsRunValue* value;    // NULL on error
    const char* error;    // NULL on success
} TsRunValueResult;

typedef struct {
    bool ok;
    const char* error;
} TsRunResult;
```

### Step Status

```c
typedef enum {
    TSRUN_STEP_CONTINUE,     // More to execute
    TSRUN_STEP_COMPLETE,     // Finished with value
    TSRUN_STEP_NEED_IMPORTS, // Waiting for modules
    TSRUN_STEP_SUSPENDED,    // Waiting for order fulfillment
    TSRUN_STEP_DONE,         // No active execution
    TSRUN_STEP_ERROR,        // Execution error
} TsRunStepStatus;
```

### Value Types

```c
typedef enum {
    TSRUN_TYPE_UNDEFINED,
    TSRUN_TYPE_NULL,
    TSRUN_TYPE_BOOLEAN,
    TSRUN_TYPE_NUMBER,
    TSRUN_TYPE_STRING,
    TSRUN_TYPE_OBJECT,
    TSRUN_TYPE_SYMBOL,
} TsRunType;
```

## Memory Management

- Call `tsrun_value_free()` on all values when done
- Call `tsrun_step_result_free()` on step results (but NOT before freeing the value)
- Call `tsrun_free_string()` on strings returned by `tsrun_json_stringify()`
- Call `tsrun_free_strings()` on string arrays from `tsrun_keys()`

## Thread Safety

**The library is NOT thread-safe.** Use one `TsRunContext` per thread.

## Error Handling

All fallible operations return result structs. Check the `error` field or `ok` status:

```c
TsRunValueResult result = tsrun_json_parse(ctx, invalid_json);
if (!result.value) {
    printf("Error: %s\n", result.error);
}
```

Error strings are valid until the next tsrun_* call on the same context.
