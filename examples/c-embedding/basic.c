// basic.c - Basic tsrun usage example
//
// Demonstrates:
// - Creating and freeing a context
// - Evaluating simple expressions
// - Inspecting return values
// - Working with objects and arrays

#include <stdio.h>
#include <stdlib.h>
#include "tsrun.h"
#include "tsrun_console.h"

// Helper to print value type
static const char* type_name(TsRunType t) {
    switch (t) {
        case TSRUN_TYPE_UNDEFINED: return "undefined";
        case TSRUN_TYPE_NULL:      return "null";
        case TSRUN_TYPE_BOOLEAN:   return "boolean";
        case TSRUN_TYPE_NUMBER:    return "number";
        case TSRUN_TYPE_STRING:    return "string";
        case TSRUN_TYPE_OBJECT:    return "object";
        case TSRUN_TYPE_SYMBOL:    return "symbol";
        default:                   return "unknown";
    }
}

// Helper to run code and print result
static void eval_and_print(TsRunContext* ctx, const char* code) {
    printf("\n> %s\n", code);

    TsRunResult prep = tsrun_prepare(ctx, code, NULL);
    if (!prep.ok) {
        printf("Prepare error: %s\n", prep.error);
        return;
    }

    TsRunStepResult result = tsrun_run(ctx);

    switch (result.status) {
        case TSRUN_STEP_COMPLETE: {
            TsRunValue* val = result.value;
            TsRunType t = tsrun_typeof(val);
            printf("Type: %s\n", type_name(t));

            switch (t) {
                case TSRUN_TYPE_UNDEFINED:
                    printf("Value: undefined\n");
                    break;
                case TSRUN_TYPE_NULL:
                    printf("Value: null\n");
                    break;
                case TSRUN_TYPE_BOOLEAN:
                    printf("Value: %s\n", tsrun_get_bool(val) ? "true" : "false");
                    break;
                case TSRUN_TYPE_NUMBER:
                    printf("Value: %g\n", tsrun_get_number(val));
                    break;
                case TSRUN_TYPE_STRING:
                    printf("Value: \"%s\"\n", tsrun_get_string(val));
                    break;
                case TSRUN_TYPE_OBJECT:
                    if (tsrun_is_array(val)) {
                        printf("Value: Array[%zu]\n", tsrun_array_len(val));
                    } else if (tsrun_is_function(val)) {
                        printf("Value: [Function]\n");
                    } else {
                        char* json = tsrun_json_stringify(ctx, val);
                        if (json) {
                            printf("Value: %s\n", json);
                            tsrun_free_string(json);
                        }
                    }
                    break;
                default:
                    printf("Value: [%s]\n", type_name(t));
                    break;
            }
            tsrun_value_free(val);
            break;
        }
        case TSRUN_STEP_ERROR:
            printf("Error: %s\n", result.error);
            break;
        default:
            printf("Unexpected status: %d\n", result.status);
            break;
    }

    tsrun_step_result_free(&result);
}

// Demonstrate working with objects
static void object_demo(TsRunContext* ctx) {
    printf("\n=== Object Demo ===\n");

    // Create an object from JSON
    TsRunValueResult obj_r = tsrun_json_parse(ctx, "{\"name\": \"Alice\", \"age\": 30}");
    if (!obj_r.value) {
        printf("JSON parse error: %s\n", obj_r.error);
        return;
    }
    TsRunValue* obj = obj_r.value;

    // Get properties
    TsRunValueResult name_r = tsrun_get(ctx, obj, "name");
    if (name_r.value) {
        printf("name = \"%s\"\n", tsrun_get_string(name_r.value));
        tsrun_value_free(name_r.value);
    }

    TsRunValueResult age_r = tsrun_get(ctx, obj, "age");
    if (age_r.value) {
        printf("age = %g\n", tsrun_get_number(age_r.value));
        tsrun_value_free(age_r.value);
    }

    // Set a property
    TsRunValue* city = tsrun_string(ctx, "New York");
    tsrun_set(ctx, obj, "city", city);
    tsrun_value_free(city);

    // Get all keys
    size_t key_count;
    char** keys = tsrun_keys(ctx, obj, &key_count);
    printf("Keys (%zu): ", key_count);
    for (size_t i = 0; i < key_count; i++) {
        printf("%s%s", keys[i], i < key_count - 1 ? ", " : "\n");
    }
    tsrun_free_strings(keys, key_count);

    // Stringify to JSON
    char* json = tsrun_json_stringify(ctx, obj);
    printf("JSON: %s\n", json);
    tsrun_free_string(json);

    tsrun_value_free(obj);
}

// Demonstrate working with arrays
static void array_demo(TsRunContext* ctx) {
    printf("\n=== Array Demo ===\n");

    // Create an array from JSON
    TsRunValueResult arr_r = tsrun_json_parse(ctx, "[10, 20, 30]");
    if (!arr_r.value) {
        printf("JSON parse error: %s\n", arr_r.error);
        return;
    }
    TsRunValue* arr = arr_r.value;

    printf("Length: %zu\n", tsrun_array_len(arr));

    // Access elements
    for (size_t i = 0; i < tsrun_array_len(arr); i++) {
        TsRunValueResult elem_r = tsrun_array_get(ctx, arr, i);
        if (elem_r.value) {
            printf("arr[%zu] = %g\n", i, tsrun_get_number(elem_r.value));
            tsrun_value_free(elem_r.value);
        }
    }

    // Push a new element
    TsRunValue* val = tsrun_number(ctx, 40);
    tsrun_array_push(ctx, arr, val);
    tsrun_value_free(val);

    printf("After push, length: %zu\n", tsrun_array_len(arr));

    // Call array method (join)
    TsRunValue* sep = tsrun_string(ctx, ", ");
    TsRunValue* args[] = { sep };
    TsRunValueResult joined_r = tsrun_call_method(ctx, arr, "join", args, 1);
    tsrun_value_free(sep);

    if (joined_r.value) {
        printf("Joined: \"%s\"\n", tsrun_get_string(joined_r.value));
        tsrun_value_free(joined_r.value);
    }

    tsrun_value_free(arr);
}

// Demonstrate globals
static void globals_demo(TsRunContext* ctx) {
    printf("\n=== Globals Demo ===\n");

    // Set a global variable
    TsRunValue* greeting = tsrun_string(ctx, "Hello from C!");
    tsrun_set_global(ctx, "myGreeting", greeting);
    tsrun_value_free(greeting);

    // Access it from JS
    eval_and_print(ctx, "myGreeting");

    // Define a function in JS and call it from C
    eval_and_print(ctx, "function add(a: number, b: number): number { return a + b; }");

    TsRunValueResult add_r = tsrun_get_global(ctx, "add");
    if (add_r.value && tsrun_is_function(add_r.value)) {
        TsRunValue* a = tsrun_number(ctx, 100);
        TsRunValue* b = tsrun_number(ctx, 200);
        TsRunValue* args[] = { a, b };

        TsRunValueResult sum_r = tsrun_call(ctx, add_r.value, NULL, args, 2);
        if (sum_r.value) {
            printf("add(100, 200) = %g\n", tsrun_get_number(sum_r.value));
            tsrun_value_free(sum_r.value);
        }

        tsrun_value_free(a);
        tsrun_value_free(b);
        tsrun_value_free(add_r.value);
    }
}

int main(void) {
    printf("tsrun C API - Basic Example\n");
    printf("Version: %s\n", tsrun_version());

    // Create interpreter context
    TsRunContext* ctx = tsrun_new();
    if (!ctx) {
        fprintf(stderr, "Failed to create context\n");
        return 1;
    }
    tsrun_set_console(ctx, tsrun_console_stdio, NULL);

    // Basic expressions
    printf("\n=== Basic Expressions ===\n");
    eval_and_print(ctx, "1 + 2 * 3");
    eval_and_print(ctx, "\"Hello, \" + \"World!\"");
    eval_and_print(ctx, "Math.sqrt(16)");
    eval_and_print(ctx, "[1, 2, 3].map((x: number): number => x * 2)");
    eval_and_print(ctx, "({ x: 10, y: 20 })");

    // Object manipulation
    object_demo(ctx);

    // Array manipulation
    array_demo(ctx);

    // Globals
    globals_demo(ctx);

    // GC stats
    TsRunGcStats stats = tsrun_gc_stats(ctx);
    printf("\n=== GC Stats ===\n");
    printf("Total objects: %zu\n", stats.total_objects);
    printf("Pooled objects: %zu\n", stats.pooled_objects);
    printf("Live objects: %zu\n", stats.live_objects);

    // Cleanup
    tsrun_free(ctx);

    printf("\nDone!\n");
    return 0;
}
