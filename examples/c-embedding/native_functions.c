// native_functions.c - Registering C functions callable from JavaScript
//
// Demonstrates:
// - Creating native functions
// - Exposing them as globals
// - Handling arguments and return values
// - Error handling in native functions
// - Using userdata for state

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include "tsrun.h"

// ============================================================================
// Simple native function: add two numbers
// ============================================================================

static TsRunValue* native_add(
    TsRunContext* ctx,
    TsRunValue* this_arg,
    TsRunValue** args,
    size_t argc,
    void* userdata,
    const char** error_out
) {
    (void)this_arg;
    (void)userdata;

    if (argc < 2) {
        *error_out = "add() requires 2 arguments";
        return NULL;
    }

    if (!tsrun_is_number(args[0]) || !tsrun_is_number(args[1])) {
        *error_out = "add() arguments must be numbers";
        return NULL;
    }

    double a = tsrun_get_number(args[0]);
    double b = tsrun_get_number(args[1]);

    return tsrun_number(ctx, a + b);
}

// ============================================================================
// Native function with string manipulation
// ============================================================================

static TsRunValue* native_greet(
    TsRunContext* ctx,
    TsRunValue* this_arg,
    TsRunValue** args,
    size_t argc,
    void* userdata,
    const char** error_out
) {
    (void)this_arg;
    (void)userdata;

    const char* name = "World";
    if (argc > 0 && tsrun_is_string(args[0])) {
        name = tsrun_get_string(args[0]);
    }

    // Build greeting string
    char buffer[256];
    snprintf(buffer, sizeof(buffer), "Hello, %s!", name);

    return tsrun_string(ctx, buffer);
}

// ============================================================================
// Native function that returns an object
// ============================================================================

static TsRunValue* native_point(
    TsRunContext* ctx,
    TsRunValue* this_arg,
    TsRunValue** args,
    size_t argc,
    void* userdata,
    const char** error_out
) {
    (void)this_arg;
    (void)userdata;

    double x = (argc > 0 && tsrun_is_number(args[0])) ? tsrun_get_number(args[0]) : 0.0;
    double y = (argc > 1 && tsrun_is_number(args[1])) ? tsrun_get_number(args[1]) : 0.0;

    TsRunValueResult obj_r = tsrun_object_new(ctx);
    if (!obj_r.value) {
        *error_out = "Failed to create object";
        return NULL;
    }

    TsRunValue* x_val = tsrun_number(ctx, x);
    TsRunValue* y_val = tsrun_number(ctx, y);

    tsrun_set(ctx, obj_r.value, "x", x_val);
    tsrun_set(ctx, obj_r.value, "y", y_val);

    tsrun_value_free(x_val);
    tsrun_value_free(y_val);

    return obj_r.value;
}

// ============================================================================
// Native function that uses userdata (stateful)
// ============================================================================

typedef struct {
    int call_count;
    double total;
} AccumulatorState;

static TsRunValue* native_accumulate(
    TsRunContext* ctx,
    TsRunValue* this_arg,
    TsRunValue** args,
    size_t argc,
    void* userdata,
    const char** error_out
) {
    (void)this_arg;

    AccumulatorState* state = (AccumulatorState*)userdata;

    if (argc > 0 && tsrun_is_number(args[0])) {
        double value = tsrun_get_number(args[0]);
        state->total += value;
    }

    state->call_count++;

    // Return current state as object
    TsRunValueResult obj_r = tsrun_object_new(ctx);
    if (!obj_r.value) {
        *error_out = "Failed to create result object";
        return NULL;
    }

    TsRunValue* count_val = tsrun_number(ctx, (double)state->call_count);
    TsRunValue* total_val = tsrun_number(ctx, state->total);

    tsrun_set(ctx, obj_r.value, "count", count_val);
    tsrun_set(ctx, obj_r.value, "total", total_val);

    tsrun_value_free(count_val);
    tsrun_value_free(total_val);

    return obj_r.value;
}

// ============================================================================
// Native function that calls back into JS
// ============================================================================

static TsRunValue* native_map_array(
    TsRunContext* ctx,
    TsRunValue* this_arg,
    TsRunValue** args,
    size_t argc,
    void* userdata,
    const char** error_out
) {
    (void)this_arg;
    (void)userdata;

    if (argc < 2) {
        *error_out = "mapArray(arr, fn) requires 2 arguments";
        return NULL;
    }

    TsRunValue* arr = args[0];
    TsRunValue* fn = args[1];

    if (!tsrun_is_array(arr)) {
        *error_out = "First argument must be an array";
        return NULL;
    }

    if (!tsrun_is_function(fn)) {
        *error_out = "Second argument must be a function";
        return NULL;
    }

    // Create result array
    TsRunValueResult result_r = tsrun_array_new(ctx);
    if (!result_r.value) {
        *error_out = "Failed to create result array";
        return NULL;
    }

    size_t len = tsrun_array_len(arr);
    for (size_t i = 0; i < len; i++) {
        TsRunValueResult elem_r = tsrun_array_get(ctx, arr, i);
        if (!elem_r.value) continue;

        TsRunValue* idx = tsrun_number(ctx, (double)i);
        TsRunValue* call_args[] = { elem_r.value, idx };

        TsRunValueResult mapped_r = tsrun_call(ctx, fn, NULL, call_args, 2);

        tsrun_value_free(elem_r.value);
        tsrun_value_free(idx);

        if (mapped_r.value) {
            tsrun_array_push(ctx, result_r.value, mapped_r.value);
            tsrun_value_free(mapped_r.value);
        }
    }

    return result_r.value;
}

// ============================================================================
// Helper to run code and print result
// ============================================================================

static void eval_and_print(TsRunContext* ctx, const char* code) {
    printf("\n> %s\n", code);

    TsRunResult prep = tsrun_prepare(ctx, code, NULL);
    if (!prep.ok) {
        printf("Prepare error: %s\n", prep.error);
        return;
    }

    TsRunStepResult result = tsrun_run(ctx);

    if (result.status == TSRUN_STEP_COMPLETE && result.value) {
        if (tsrun_is_number(result.value)) {
            printf("=> %g\n", tsrun_get_number(result.value));
        } else if (tsrun_is_string(result.value)) {
            printf("=> \"%s\"\n", tsrun_get_string(result.value));
        } else {
            char* json = tsrun_json_stringify(ctx, result.value);
            if (json) {
                printf("=> %s\n", json);
                tsrun_free_string(json);
            }
        }
        tsrun_value_free(result.value);
    } else if (result.status == TSRUN_STEP_ERROR) {
        printf("Error: %s\n", result.error);
    }

    tsrun_step_result_free(&result);
}

// ============================================================================
// Main
// ============================================================================

int main(void) {
    printf("tsrun C API - Native Functions Example\n\n");

    TsRunContext* ctx = tsrun_new();
    if (!ctx) {
        fprintf(stderr, "Failed to create context\n");
        return 1;
    }

    // Register native_add
    printf("=== Registering native functions ===\n");

    TsRunValueResult add_fn = tsrun_native_function(ctx, "nativeAdd", native_add, 2, NULL);
    if (add_fn.value) {
        tsrun_set_global(ctx, "nativeAdd", add_fn.value);
        tsrun_value_free(add_fn.value);
        printf("Registered: nativeAdd(a, b)\n");
    }

    // Register native_greet
    TsRunValueResult greet_fn = tsrun_native_function(ctx, "greet", native_greet, 1, NULL);
    if (greet_fn.value) {
        tsrun_set_global(ctx, "greet", greet_fn.value);
        tsrun_value_free(greet_fn.value);
        printf("Registered: greet(name)\n");
    }

    // Register native_point
    TsRunValueResult point_fn = tsrun_native_function(ctx, "createPoint", native_point, 2, NULL);
    if (point_fn.value) {
        tsrun_set_global(ctx, "createPoint", point_fn.value);
        tsrun_value_free(point_fn.value);
        printf("Registered: createPoint(x, y)\n");
    }

    // Register native_accumulate with state
    AccumulatorState acc_state = { .call_count = 0, .total = 0.0 };
    TsRunValueResult acc_fn = tsrun_native_function(ctx, "accumulate", native_accumulate, 1, &acc_state);
    if (acc_fn.value) {
        tsrun_set_global(ctx, "accumulate", acc_fn.value);
        tsrun_value_free(acc_fn.value);
        printf("Registered: accumulate(value) [stateful]\n");
    }

    // Register native_map_array
    TsRunValueResult map_fn = tsrun_native_function(ctx, "mapArray", native_map_array, 2, NULL);
    if (map_fn.value) {
        tsrun_set_global(ctx, "mapArray", map_fn.value);
        tsrun_value_free(map_fn.value);
        printf("Registered: mapArray(arr, fn)\n");
    }

    // Test the native functions
    printf("\n=== Testing native functions ===\n");

    eval_and_print(ctx, "nativeAdd(10, 20)");
    eval_and_print(ctx, "nativeAdd(3.14, 2.86)");

    eval_and_print(ctx, "greet()");
    eval_and_print(ctx, "greet('Alice')");

    eval_and_print(ctx, "createPoint(100, 200)");
    eval_and_print(ctx, "const p: { x: number; y: number } = createPoint(5, 10); p.x + p.y");

    printf("\n=== Testing stateful accumulator ===\n");
    eval_and_print(ctx, "accumulate(10)");
    eval_and_print(ctx, "accumulate(20)");
    eval_and_print(ctx, "accumulate(30)");
    printf("Final state from C: count=%d, total=%g\n", acc_state.call_count, acc_state.total);

    printf("\n=== Testing callback into JS ===\n");
    eval_and_print(ctx, "mapArray([1, 2, 3], (x: number): number => x * x)");
    eval_and_print(ctx, "mapArray(['a', 'b', 'c'], (s: string, i: number): string => s + i)");

    printf("\n=== Error handling ===\n");
    eval_and_print(ctx, "nativeAdd(1)");  // Too few args
    eval_and_print(ctx, "nativeAdd('a', 'b')");  // Wrong types

    tsrun_free(ctx);
    printf("\nDone!\n");
    return 0;
}
