// async_orders.c - Async order handling example
//
// Demonstrates:
// - Native functions that create pending orders
// - Step-based execution with TSRUN_STEP_SUSPENDED
// - Processing orders from JavaScript
// - Fulfilling orders with responses
// - Error handling in async operations

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tsrun.h"
#include "tsrun_console.h"

// ============================================================================
// Native async functions that create pending orders
// ============================================================================

// Native function: dbQuery(table, id) - creates a pending order
static TsRunValue* native_db_query(
    TsRunContext* ctx,
    TsRunValue* this_arg,
    TsRunValue** args,
    size_t argc,
    void* userdata,
    const char** error_out
) {
    (void)this_arg;
    (void)userdata;

    // Get arguments
    const char* table = "unknown";
    double id = 0;

    if (argc >= 1 && tsrun_is_string(args[0])) {
        table = tsrun_get_string(args[0]);
    }
    if (argc >= 2 && tsrun_is_number(args[1])) {
        id = tsrun_get_number(args[1]);
    }

    // Create payload object with order details
    TsRunValueResult obj_r = tsrun_object_new(ctx);
    if (!obj_r.value) {
        *error_out = "Failed to create payload object";
        return NULL;
    }

    // Set payload properties
    TsRunValue* type_val = tsrun_string(ctx, "db_query");
    TsRunValue* table_val = tsrun_string(ctx, table);
    TsRunValue* id_val = tsrun_number(ctx, id);

    tsrun_set(ctx, obj_r.value, "type", type_val);
    tsrun_set(ctx, obj_r.value, "table", table_val);
    tsrun_set(ctx, obj_r.value, "id", id_val);

    // Create pending order - this will cause the interpreter to suspend
    TsRunOrderId order_id;
    TsRunValueResult pending = tsrun_create_pending_order(ctx, obj_r.value, &order_id);

    // Clean up temporary values
    tsrun_value_free(type_val);
    tsrun_value_free(table_val);
    tsrun_value_free(id_val);
    tsrun_value_free(obj_r.value);

    if (!pending.value) {
        *error_out = pending.error ? pending.error : "Failed to create pending order";
        return NULL;
    }

    return pending.value;
}

// Native function: httpFetch(url) - creates a pending order
static TsRunValue* native_http_fetch(
    TsRunContext* ctx,
    TsRunValue* this_arg,
    TsRunValue** args,
    size_t argc,
    void* userdata,
    const char** error_out
) {
    (void)this_arg;
    (void)userdata;

    const char* url = "http://unknown";
    if (argc >= 1 && tsrun_is_string(args[0])) {
        url = tsrun_get_string(args[0]);
    }

    // Create payload
    TsRunValueResult obj_r = tsrun_object_new(ctx);
    if (!obj_r.value) {
        *error_out = "Failed to create payload object";
        return NULL;
    }

    TsRunValue* type_val = tsrun_string(ctx, "http_fetch");
    TsRunValue* url_val = tsrun_string(ctx, url);

    tsrun_set(ctx, obj_r.value, "type", type_val);
    tsrun_set(ctx, obj_r.value, "url", url_val);

    TsRunOrderId order_id;
    TsRunValueResult pending = tsrun_create_pending_order(ctx, obj_r.value, &order_id);

    tsrun_value_free(type_val);
    tsrun_value_free(url_val);
    tsrun_value_free(obj_r.value);

    if (!pending.value) {
        *error_out = pending.error ? pending.error : "Failed to create pending order";
        return NULL;
    }

    return pending.value;
}

// Native function: delay(ms) - creates a pending order
static TsRunValue* native_delay(
    TsRunContext* ctx,
    TsRunValue* this_arg,
    TsRunValue** args,
    size_t argc,
    void* userdata,
    const char** error_out
) {
    (void)this_arg;
    (void)userdata;

    double ms = 0;
    if (argc >= 1 && tsrun_is_number(args[0])) {
        ms = tsrun_get_number(args[0]);
    }

    TsRunValueResult obj_r = tsrun_object_new(ctx);
    if (!obj_r.value) {
        *error_out = "Failed to create payload object";
        return NULL;
    }

    TsRunValue* type_val = tsrun_string(ctx, "timeout");
    TsRunValue* ms_val = tsrun_number(ctx, ms);

    tsrun_set(ctx, obj_r.value, "type", type_val);
    tsrun_set(ctx, obj_r.value, "ms", ms_val);

    TsRunOrderId order_id;
    TsRunValueResult pending = tsrun_create_pending_order(ctx, obj_r.value, &order_id);

    tsrun_value_free(type_val);
    tsrun_value_free(ms_val);
    tsrun_value_free(obj_r.value);

    if (!pending.value) {
        *error_out = pending.error ? pending.error : "Failed to create pending order";
        return NULL;
    }

    return pending.value;
}

// Native function: errorTest() - creates a pending order that will be rejected
static TsRunValue* native_error_test(
    TsRunContext* ctx,
    TsRunValue* this_arg,
    TsRunValue** args,
    size_t argc,
    void* userdata,
    const char** error_out
) {
    (void)this_arg;
    (void)args;
    (void)argc;
    (void)userdata;

    TsRunValueResult obj_r = tsrun_object_new(ctx);
    if (!obj_r.value) {
        *error_out = "Failed to create payload object";
        return NULL;
    }

    TsRunValue* type_val = tsrun_string(ctx, "error_test");
    tsrun_set(ctx, obj_r.value, "type", type_val);

    TsRunOrderId order_id;
    TsRunValueResult pending = tsrun_create_pending_order(ctx, obj_r.value, &order_id);

    tsrun_value_free(type_val);
    tsrun_value_free(obj_r.value);

    if (!pending.value) {
        *error_out = pending.error ? pending.error : "Failed to create pending order";
        return NULL;
    }

    return pending.value;
}

// ============================================================================
// Simulated async operations
// ============================================================================

static TsRunValue* simulate_db_query(TsRunContext* ctx, const char* table, int id) {
    printf("    [C] Simulating DB query: SELECT * FROM %s WHERE id = %d\n", table, id);

    char json[256];
    snprintf(json, sizeof(json),
        "{\"id\": %d, \"table\": \"%s\", \"data\": \"mock_data_%d\"}",
        id, table, id);

    TsRunValueResult result = tsrun_json_parse(ctx, json);
    return result.value;
}

static TsRunValue* simulate_http_fetch(TsRunContext* ctx, const char* url) {
    printf("    [C] Simulating HTTP fetch: %s\n", url);

    char json[512];
    snprintf(json, sizeof(json),
        "{\"status\": 200, \"url\": \"%s\", \"body\": \"Response from %s\"}",
        url, url);

    TsRunValueResult result = tsrun_json_parse(ctx, json);
    return result.value;
}

static TsRunValue* simulate_timeout(TsRunContext* ctx, double ms) {
    printf("    [C] Simulating timeout: %.0f ms\n", ms);
    return tsrun_undefined(ctx);
}

// ============================================================================
// Order processing
// ============================================================================

static TsRunStepResult process_orders(TsRunContext* ctx, TsRunStepResult result) {
    while (result.status == TSRUN_STEP_SUSPENDED) {
        printf("\n--- Order processor: %zu pending, %zu cancelled ---\n",
               result.pending_count, result.cancelled_count);

        if (result.pending_count == 0) {
            // No orders to process, continue running
            tsrun_step_result_free(&result);
            result = tsrun_run(ctx);
            continue;
        }

        // Handle cancelled orders first
        for (size_t i = 0; i < result.cancelled_count; i++) {
            printf("  Cancelled order: %llu\n",
                   (unsigned long long)result.cancelled_orders[i]);
        }

        // Prepare responses for pending orders
        TsRunOrderResponse* responses = malloc(
            result.pending_count * sizeof(TsRunOrderResponse)
        );

        for (size_t i = 0; i < result.pending_count; i++) {
            TsRunOrder* order = &result.pending_orders[i];
            responses[i].id = order->id;
            responses[i].error = NULL;

            printf("\n  Processing order #%llu:\n", (unsigned long long)order->id);

            // Get order type
            TsRunValueResult type_r = tsrun_get(ctx, order->payload, "type");
            if (!type_r.value || !tsrun_is_string(type_r.value)) {
                responses[i].value = NULL;
                responses[i].error = "Order missing 'type' field";
                if (type_r.value) tsrun_value_free(type_r.value);
                continue;
            }

            const char* type = tsrun_get_string(type_r.value);
            printf("    Type: %s\n", type);

            if (strcmp(type, "db_query") == 0) {
                TsRunValueResult table_r = tsrun_get(ctx, order->payload, "table");
                TsRunValueResult id_r = tsrun_get(ctx, order->payload, "id");

                const char* table = "unknown";
                int id = 0;

                if (table_r.value && tsrun_is_string(table_r.value)) {
                    table = tsrun_get_string(table_r.value);
                }
                if (id_r.value && tsrun_is_number(id_r.value)) {
                    id = (int)tsrun_get_number(id_r.value);
                }

                responses[i].value = simulate_db_query(ctx, table, id);

                if (table_r.value) tsrun_value_free(table_r.value);
                if (id_r.value) tsrun_value_free(id_r.value);

            } else if (strcmp(type, "http_fetch") == 0) {
                TsRunValueResult url_r = tsrun_get(ctx, order->payload, "url");

                const char* url = "http://unknown";
                if (url_r.value && tsrun_is_string(url_r.value)) {
                    url = tsrun_get_string(url_r.value);
                }

                responses[i].value = simulate_http_fetch(ctx, url);

                if (url_r.value) tsrun_value_free(url_r.value);

            } else if (strcmp(type, "timeout") == 0) {
                TsRunValueResult ms_r = tsrun_get(ctx, order->payload, "ms");

                double ms = 0;
                if (ms_r.value && tsrun_is_number(ms_r.value)) {
                    ms = tsrun_get_number(ms_r.value);
                }

                responses[i].value = simulate_timeout(ctx, ms);

                if (ms_r.value) tsrun_value_free(ms_r.value);

            } else if (strcmp(type, "error_test") == 0) {
                responses[i].value = NULL;
                responses[i].error = "Simulated error for testing";
                printf("    [C] Returning error\n");

            } else {
                responses[i].value = NULL;
                responses[i].error = "Unknown order type";
                printf("    [C] Unknown type: %s\n", type);
            }

            tsrun_value_free(type_r.value);
        }

        // Fulfill all orders
        TsRunResult fulfill = tsrun_fulfill_orders(ctx, responses, result.pending_count);
        if (!fulfill.ok) {
            printf("Failed to fulfill orders: %s\n", fulfill.error);
        }

        // Free response values
        for (size_t i = 0; i < result.pending_count; i++) {
            if (responses[i].value) {
                tsrun_value_free(responses[i].value);
            }
        }
        free(responses);

        // Continue execution
        tsrun_step_result_free(&result);
        result = tsrun_run(ctx);
    }

    return result;
}

// ============================================================================
// Helper to set up context with native async functions
// ============================================================================

static void setup_async_functions(TsRunContext* ctx) {
    // Register native async functions as globals
    TsRunValueResult dbQuery = tsrun_native_function(ctx, "dbQuery", native_db_query, 2, NULL);
    TsRunValueResult httpFetch = tsrun_native_function(ctx, "httpFetch", native_http_fetch, 1, NULL);
    TsRunValueResult delay = tsrun_native_function(ctx, "delay", native_delay, 1, NULL);
    TsRunValueResult errorTest = tsrun_native_function(ctx, "errorTest", native_error_test, 0, NULL);

    if (dbQuery.value) {
        tsrun_set_global(ctx, "dbQuery", dbQuery.value);
        tsrun_value_free(dbQuery.value);
    }
    if (httpFetch.value) {
        tsrun_set_global(ctx, "httpFetch", httpFetch.value);
        tsrun_value_free(httpFetch.value);
    }
    if (delay.value) {
        tsrun_set_global(ctx, "delay", delay.value);
        tsrun_value_free(delay.value);
    }
    if (errorTest.value) {
        tsrun_set_global(ctx, "errorTest", errorTest.value);
        tsrun_value_free(errorTest.value);
    }
}

// ============================================================================
// Helper to run async code
// ============================================================================

static void run_async_code(const char* title, const char* code) {
    printf("\n========================================\n");
    printf("%s\n", title);
    printf("========================================\n");
    printf("\nCode:\n%s\n", code);

    TsRunContext* ctx = tsrun_new();
    tsrun_set_console(ctx, tsrun_console_stdio, NULL);

    // Set up native async functions
    setup_async_functions(ctx);

    // Run the code
    TsRunResult prep = tsrun_prepare(ctx, code, "/main.ts");
    if (!prep.ok) {
        printf("Prepare error: %s\n", prep.error);
        tsrun_free(ctx);
        return;
    }

    TsRunStepResult result = tsrun_run(ctx);
    result = process_orders(ctx, result);

    if (result.status == TSRUN_STEP_COMPLETE) {
        printf("\n--- Result ---\n");
        if (result.value) {
            if (tsrun_is_string(result.value)) {
                printf("%s\n", tsrun_get_string(result.value));
            } else if (tsrun_is_number(result.value)) {
                printf("%g\n", tsrun_get_number(result.value));
            } else {
                char* json = tsrun_json_stringify(ctx, result.value);
                if (json) {
                    printf("%s\n", json);
                    tsrun_free_string(json);
                }
            }
            tsrun_value_free(result.value);
        } else {
            printf("undefined\n");
        }
    } else if (result.status == TSRUN_STEP_ERROR) {
        printf("\n--- Error ---\n%s\n", result.error);
    } else if (result.status == TSRUN_STEP_DONE) {
        printf("\n--- Done (no result) ---\n");
    }

    tsrun_step_result_free(&result);
    tsrun_free(ctx);
}

// ============================================================================
// Examples
// ============================================================================

static void example_basic_async(void) {
    run_async_code(
        "Example 1: Basic sync call (order creates immediate suspension)",
        "interface DbResult {\n"
        "    id: number;\n"
        "    table: string;\n"
        "    data: string;\n"
        "}\n"
        "\n"
        "declare function dbQuery(table: string, id: number): DbResult;\n"
        "\n"
        "console.log('Starting...');\n"
        "\n"
        "const user: DbResult = dbQuery('users', 42);\n"
        "console.log('Got user:', JSON.stringify(user));\n"
        "\n"
        "user;\n"
    );
}

static void example_multiple_calls(void) {
    run_async_code(
        "Example 2: Multiple sequential calls",
        "interface DbResult {\n"
        "    id: number;\n"
        "    table: string;\n"
        "    data: string;\n"
        "}\n"
        "\n"
        "interface HttpResponse {\n"
        "    status: number;\n"
        "    url: string;\n"
        "    body: string;\n"
        "}\n"
        "\n"
        "declare function dbQuery(table: string, id: number): DbResult;\n"
        "declare function httpFetch(url: string): HttpResponse;\n"
        "\n"
        "console.log('Fetching data...');\n"
        "\n"
        "const user: DbResult = dbQuery('users', 1);\n"
        "console.log('User:', JSON.stringify(user));\n"
        "\n"
        "const posts: DbResult = dbQuery('posts', 100);\n"
        "console.log('Posts:', JSON.stringify(posts));\n"
        "\n"
        "const config: HttpResponse = httpFetch('https://api.example.com/config');\n"
        "console.log('Config:', JSON.stringify(config));\n"
        "\n"
        "({ user, posts, config });\n"
    );
}

static void example_error_handling(void) {
    run_async_code(
        "Example 3: Error handling",
        "declare function errorTest(): never;\n"
        "\n"
        "console.log('Attempting operation that will fail...');\n"
        "\n"
        "try {\n"
        "    const result: never = errorTest();\n"
        "    console.log('Result:', result);\n"
        "} catch (e: unknown) {\n"
        "    const error = e as Error;\n"
        "    console.log('Caught error:', error.message);\n"
        "}\n"
        "\n"
        "'Error was handled';\n"
    );
}

static void example_loop(void) {
    run_async_code(
        "Example 4: Orders in a loop",
        "interface DbResult {\n"
        "    id: number;\n"
        "    table: string;\n"
        "    data: string;\n"
        "}\n"
        "\n"
        "declare function dbQuery(table: string, id: number): DbResult;\n"
        "\n"
        "const results: DbResult[] = [];\n"
        "\n"
        "for (let i: number = 1; i <= 3; i++) {\n"
        "    console.log(`Fetching item ${i}...`);\n"
        "    const item: DbResult = dbQuery('items', i);\n"
        "    results.push(item);\n"
        "}\n"
        "\n"
        "console.log('All items fetched!');\n"
        "results;\n"
    );
}

// ============================================================================
// Main
// ============================================================================

int main(void) {
    printf("tsrun C API - Async Orders Example\n");
    printf("===================================\n\n");
    printf("This example demonstrates how the host (C code) handles\n");
    printf("async operations from JavaScript via the order system.\n");
    printf("\n");
    printf("Native functions create pending orders, which cause the\n");
    printf("interpreter to suspend. The host then fulfills these orders\n");
    printf("and the interpreter resumes with the results.\n");

    example_basic_async();
    example_multiple_calls();
    example_error_handling();
    example_loop();

    printf("\nDone!\n");
    return 0;
}
