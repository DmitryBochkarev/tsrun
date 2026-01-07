// tsrun.h - C API for TypeScript interpreter
// Thread safety: NOT thread-safe. Use one context per thread.

#ifndef TSRUN_H
#define TSRUN_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Version
// ============================================================================

#define TSRUN_VERSION_MAJOR 0
#define TSRUN_VERSION_MINOR 1
#define TSRUN_VERSION_PATCH 0

const char* tsrun_version(void);

// ============================================================================
// Opaque Types
// ============================================================================

typedef struct TsRunContext TsRunContext;
typedef struct TsRunValue TsRunValue;
typedef uint64_t TsRunOrderId;

// ============================================================================
// Result Types (all fallible operations return these)
// ============================================================================

// Result for operations returning a value
typedef struct {
    TsRunValue* value;    // NULL on error
    const char* error;    // NULL on success, valid until next tsrun_* call
} TsRunValueResult;

// Result for operations returning nothing
typedef struct {
    bool ok;
    const char* error;    // NULL on success
} TsRunResult;

// ============================================================================
// Value Types
// ============================================================================

typedef enum {
    TSRUN_TYPE_UNDEFINED = 0,
    TSRUN_TYPE_NULL,
    TSRUN_TYPE_BOOLEAN,
    TSRUN_TYPE_NUMBER,
    TSRUN_TYPE_STRING,
    TSRUN_TYPE_OBJECT,
    TSRUN_TYPE_SYMBOL,
} TsRunType;

// ============================================================================
// Step Result (mirrors Rust StepResult)
// ============================================================================

typedef enum {
    TSRUN_STEP_CONTINUE = 0,    // More instructions to execute
    TSRUN_STEP_COMPLETE,        // Execution finished
    TSRUN_STEP_NEED_IMPORTS,    // Waiting for modules
    TSRUN_STEP_SUSPENDED,       // Waiting for order fulfillment
    TSRUN_STEP_DONE,            // No active execution
    TSRUN_STEP_ERROR,           // Execution error
} TsRunStepStatus;

// Import request
typedef struct {
    const char* specifier;      // Original import specifier (e.g., "./foo")
    const char* resolved_path;  // Resolved absolute path
    const char* importer;       // Module that requested this (NULL for main)
} TsRunImportRequest;

// Order from JS to host
typedef struct {
    TsRunOrderId id;
    TsRunValue* payload;        // The order payload (owned by context)
} TsRunOrder;

// Step result with all possible data
typedef struct {
    TsRunStepStatus status;

    // For TSRUN_STEP_COMPLETE
    TsRunValue* value;

    // For TSRUN_STEP_NEED_IMPORTS
    TsRunImportRequest* imports;
    size_t import_count;

    // For TSRUN_STEP_SUSPENDED
    TsRunOrder* pending_orders;
    size_t pending_count;
    TsRunOrderId* cancelled_orders;
    size_t cancelled_count;

    // For TSRUN_STEP_ERROR
    const char* error;
} TsRunStepResult;

// ============================================================================
// Context Lifecycle
// ============================================================================

// Create a new interpreter context
TsRunContext* tsrun_new(void);

// Free an interpreter context (also frees all associated values)
void tsrun_free(TsRunContext* ctx);

// ============================================================================
// Execution - Step-based API
// ============================================================================

// Prepare code for execution
// path is optional (NULL for anonymous scripts, or "/path/to/module.ts" for modules)
TsRunResult tsrun_prepare(TsRunContext* ctx, const char* code, const char* path);

// Execute one step
// Returns step result - caller must call tsrun_step_result_free when done
TsRunStepResult tsrun_step(TsRunContext* ctx);

// Run until completion, needing imports, or suspension
// Equivalent to calling step() in a loop until non-Continue result
TsRunStepResult tsrun_run(TsRunContext* ctx);

// Free a step result (frees internal arrays, NOT the value)
void tsrun_step_result_free(TsRunStepResult* result);

// ============================================================================
// Module System
// ============================================================================

// Provide module source code in response to TSRUN_STEP_NEED_IMPORTS
TsRunResult tsrun_provide_module(TsRunContext* ctx, const char* path, const char* code);

// ============================================================================
// Order System (for async operations)
// ============================================================================

// Order response from host to JS
typedef struct {
    TsRunOrderId id;
    TsRunValue* value;      // Result value (NULL if error)
    const char* error;      // Error message (NULL if success)
} TsRunOrderResponse;

// Fulfill one or more orders
TsRunResult tsrun_fulfill_orders(TsRunContext* ctx,
                                  const TsRunOrderResponse* responses,
                                  size_t count);

// Create a pending order that will suspend the interpreter.
// Use in native callbacks to perform async operations.
// The payload is accessible via order.payload in the step result.
// The order_id_out receives the ID to use when fulfilling the order.
// Returns a value that MUST be returned from the native callback.
TsRunValueResult tsrun_create_pending_order(TsRunContext* ctx,
                                             TsRunValue* payload,
                                             TsRunOrderId* order_id_out);

// Create a promise for deferred order fulfillment
// Use when you want to return a promise that will be resolved later
TsRunValueResult tsrun_create_order_promise(TsRunContext* ctx, TsRunOrderId order_id);

// Resolve a promise created with tsrun_create_order_promise
TsRunResult tsrun_resolve_promise(TsRunContext* ctx, TsRunValue* promise, TsRunValue* value);

// Reject a promise
TsRunResult tsrun_reject_promise(TsRunContext* ctx, TsRunValue* promise, const char* error);

// ============================================================================
// Value Inspection
// ============================================================================

TsRunType tsrun_typeof(const TsRunValue* val);
bool tsrun_is_undefined(const TsRunValue* val);
bool tsrun_is_null(const TsRunValue* val);
bool tsrun_is_nullish(const TsRunValue* val);
bool tsrun_is_boolean(const TsRunValue* val);
bool tsrun_is_number(const TsRunValue* val);
bool tsrun_is_string(const TsRunValue* val);
bool tsrun_is_object(const TsRunValue* val);
bool tsrun_is_array(const TsRunValue* val);
bool tsrun_is_function(const TsRunValue* val);

// Extract primitive values (undefined behavior if wrong type - check first!)
bool tsrun_get_bool(const TsRunValue* val);
double tsrun_get_number(const TsRunValue* val);
const char* tsrun_get_string(const TsRunValue* val);  // Valid until value freed
size_t tsrun_get_string_len(const TsRunValue* val);

// ============================================================================
// Value Creation
// ============================================================================

TsRunValue* tsrun_undefined(TsRunContext* ctx);
TsRunValue* tsrun_null(TsRunContext* ctx);
TsRunValue* tsrun_boolean(TsRunContext* ctx, bool b);
TsRunValue* tsrun_number(TsRunContext* ctx, double n);
TsRunValue* tsrun_string(TsRunContext* ctx, const char* s);
TsRunValue* tsrun_string_len(TsRunContext* ctx, const char* s, size_t len);

// Create object/array from JSON string
TsRunValueResult tsrun_json_parse(TsRunContext* ctx, const char* json);

// Create empty object/array
TsRunValueResult tsrun_object_new(TsRunContext* ctx);
TsRunValueResult tsrun_array_new(TsRunContext* ctx);

// ============================================================================
// Value Memory Management
// ============================================================================

// Values returned by the API are "guarded" - they won't be garbage collected
// until you explicitly free them. Always free values when done.
void tsrun_value_free(TsRunValue* val);

// Duplicate a value handle (both handles must be freed separately)
TsRunValue* tsrun_value_dup(const TsRunValue* val);

// ============================================================================
// Object/Array Operations
// ============================================================================

// Property access
TsRunValueResult tsrun_get(TsRunContext* ctx, TsRunValue* obj, const char* key);
TsRunResult tsrun_set(TsRunContext* ctx, TsRunValue* obj, const char* key, TsRunValue* val);
bool tsrun_has(TsRunContext* ctx, TsRunValue* obj, const char* key);
TsRunResult tsrun_delete(TsRunContext* ctx, TsRunValue* obj, const char* key);

// Get property keys (caller must free returned array with tsrun_free_strings)
char** tsrun_keys(TsRunContext* ctx, TsRunValue* obj, size_t* count_out);
void tsrun_free_strings(char** strings, size_t count);

// Array operations
size_t tsrun_array_len(const TsRunValue* arr);
TsRunValueResult tsrun_array_get(TsRunContext* ctx, TsRunValue* arr, size_t index);
TsRunResult tsrun_array_set(TsRunContext* ctx, TsRunValue* arr, size_t index, TsRunValue* val);
TsRunResult tsrun_array_push(TsRunContext* ctx, TsRunValue* arr, TsRunValue* val);

// ============================================================================
// Function Calls
// ============================================================================

// Call a function
TsRunValueResult tsrun_call(TsRunContext* ctx,
                             TsRunValue* func,
                             TsRunValue* this_arg,  // NULL for undefined
                             TsRunValue** args,
                             size_t argc);

// Call a method on an object
TsRunValueResult tsrun_call_method(TsRunContext* ctx,
                                    TsRunValue* obj,
                                    const char* method,
                                    TsRunValue** args,
                                    size_t argc);

// ============================================================================
// Global Access
// ============================================================================

TsRunValueResult tsrun_get_global(TsRunContext* ctx, const char* name);
TsRunResult tsrun_set_global(TsRunContext* ctx, const char* name, TsRunValue* val);

// ============================================================================
// Module Exports
// ============================================================================

// Get an export from the main module (after execution completes)
TsRunValueResult tsrun_get_export(TsRunContext* ctx, const char* name);

// Get all export names (caller frees with tsrun_free_strings)
char** tsrun_get_export_names(TsRunContext* ctx, size_t* count_out);

// ============================================================================
// Native Functions
// ============================================================================

// Callback signature for native functions
// Return NULL to return undefined, set *error_out on error
typedef TsRunValue* (*TsRunNativeFn)(
    TsRunContext* ctx,
    TsRunValue* this_arg,
    TsRunValue** args,
    size_t argc,
    void* userdata,
    const char** error_out
);

// Create a native function that can be called from JS
TsRunValueResult tsrun_native_function(
    TsRunContext* ctx,
    const char* name,
    TsRunNativeFn func,
    size_t arity,
    void* userdata
);

// ============================================================================
// JSON Serialization
// ============================================================================

// Serialize value to JSON string (caller frees with tsrun_free_string)
char* tsrun_json_stringify(TsRunContext* ctx, TsRunValue* val);
void tsrun_free_string(char* s);

// ============================================================================
// Internal Modules (for extending the interpreter)
// ============================================================================

typedef struct TsRunInternalModule TsRunInternalModule;

// Create an internal module builder
TsRunInternalModule* tsrun_internal_module_new(const char* specifier);

// Add a native function export
void tsrun_internal_module_add_function(
    TsRunInternalModule* module,
    const char* name,
    TsRunNativeFn func,
    size_t arity,
    void* userdata
);

// Add a value export
void tsrun_internal_module_add_value(
    TsRunInternalModule* module,
    const char* name,
    TsRunValue* value
);

// Register the module with a context (takes ownership of module)
TsRunResult tsrun_register_internal_module(TsRunContext* ctx, TsRunInternalModule* module);

// ============================================================================
// Custom RegExp Provider
// ============================================================================

// Regex match result (for captures)
typedef struct {
    intptr_t start;  // Start byte offset (-1 if group didn't participate)
    intptr_t end;    // End byte offset (-1 if group didn't participate)
} TsRunRegexCapture;

// Regex match result
typedef struct {
    size_t start;                   // Byte offset where match starts
    size_t end;                     // Byte offset where match ends (exclusive)
    TsRunRegexCapture* captures;    // Array of capture groups (NULL if none)
    size_t capture_count;           // Number of capture groups
} TsRunRegexMatch;

// Callback: Compile a regex pattern
// Returns opaque handle on success, NULL on error (set *error_out)
typedef void* (*TsRunRegexCompileFn)(
    void* userdata,
    const char* pattern,
    const char* flags,
    const char** error_out
);

// Callback: Test if regex matches (1=match, 0=no match, -1=error)
typedef int (*TsRunRegexIsMatchFn)(
    void* userdata,
    void* handle,
    const char* input,
    size_t input_len,
    const char** error_out
);

// Callback: Find first match at position (1=found, 0=not found, -1=error)
typedef int (*TsRunRegexFindFn)(
    void* userdata,
    void* handle,
    const char* input,
    size_t input_len,
    size_t start_pos,
    TsRunRegexMatch* match_out,
    const char** error_out
);

// Callback: Free a compiled regex handle
typedef void (*TsRunRegexFreeFn)(void* userdata, void* handle);

// Callback: Free captures array (may be NULL if not needed)
typedef void (*TsRunRegexFreeCapturesFn)(
    void* userdata,
    TsRunRegexCapture* captures,
    size_t count
);

// Bundle of regex callbacks
typedef struct {
    TsRunRegexCompileFn compile;
    TsRunRegexIsMatchFn is_match;
    TsRunRegexFindFn find;
    TsRunRegexFreeFn free;
    TsRunRegexFreeCapturesFn free_captures;  // May be NULL
    void* userdata;
} TsRunRegexCallbacks;

// Set a custom RegExp provider
// The callbacks must remain valid for the lifetime of the context
TsRunResult tsrun_set_regexp_provider(TsRunContext* ctx,
                                       const TsRunRegexCallbacks* callbacks);

// ============================================================================
// Debugging / Statistics
// ============================================================================

typedef struct {
    size_t total_objects;   // Total GcBox slots (including pooled)
    size_t pooled_objects;  // Objects in pool (available for reuse)
    size_t live_objects;    // Number of live objects
} TsRunGcStats;

TsRunGcStats tsrun_gc_stats(TsRunContext* ctx);

#ifdef __cplusplus
}
#endif

#endif // TSRUN_H
