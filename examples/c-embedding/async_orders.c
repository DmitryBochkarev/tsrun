// async_orders.c - Async order handling example
//
// Demonstrates:
// - JavaScript using `await order()` from tsrun:host
// - Step-based execution with TSRUN_STEP_SUSPENDED
// - Processing orders (requests from JS)
// - Fulfilling orders with responses
// - Error handling in async operations

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tsrun.h"
#include "tsrun_console.h"

// ============================================================================
// Simulated async operations (host-side implementations)
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
    (void)ctx;
    return NULL; // Returns undefined
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
            tsrun_run(&result, ctx);
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

            // Get order type from payload
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
        tsrun_run(&result, ctx);
    }

    return result;
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

    // Run the code
    TsRunResult prep = tsrun_prepare(ctx, code, "/main.ts");
    if (!prep.ok) {
        printf("Prepare error: %s\n", prep.error);
        tsrun_free(ctx);
        return;
    }

    TsRunStepResult result;
    tsrun_run(&result, ctx);
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
// Examples - Using order() from tsrun:host
// ============================================================================

static void example_basic_async(void) {
    run_async_code(
        "Example 1: Basic async operation",
        "import { order } from 'tsrun:host';\n"
        "\n"
        "interface DbResult {\n"
        "    id: number;\n"
        "    table: string;\n"
        "    data: string;\n"
        "}\n"
        "\n"
        "// Helper function that creates an order for DB query\n"
        "async function dbQuery(table: string, id: number): Promise<DbResult> {\n"
        "    return await order({ type: 'db_query', table, id });\n"
        "}\n"
        "\n"
        "console.log('Starting...');\n"
        "\n"
        "const user = await dbQuery('users', 42);\n"
        "console.log('Got user:', JSON.stringify(user));\n"
        "\n"
        "user;\n"
    );
}

static void example_multiple_calls(void) {
    run_async_code(
        "Example 2: Multiple sequential calls",
        "import { order } from 'tsrun:host';\n"
        "\n"
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
        "async function dbQuery(table: string, id: number): Promise<DbResult> {\n"
        "    return await order({ type: 'db_query', table, id });\n"
        "}\n"
        "\n"
        "async function httpFetch(url: string): Promise<HttpResponse> {\n"
        "    return await order({ type: 'http_fetch', url });\n"
        "}\n"
        "\n"
        "console.log('Fetching data...');\n"
        "\n"
        "const user = await dbQuery('users', 1);\n"
        "console.log('User:', JSON.stringify(user));\n"
        "\n"
        "const posts = await dbQuery('posts', 100);\n"
        "console.log('Posts:', JSON.stringify(posts));\n"
        "\n"
        "const config = await httpFetch('https://api.example.com/config');\n"
        "console.log('Config:', JSON.stringify(config));\n"
        "\n"
        "({ user, posts, config });\n"
    );
}

static void example_error_handling(void) {
    run_async_code(
        "Example 3: Error handling",
        "import { order } from 'tsrun:host';\n"
        "\n"
        "console.log('Attempting operation that will fail...');\n"
        "\n"
        "try {\n"
        "    const result = await order({ type: 'error_test' });\n"
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
        "import { order } from 'tsrun:host';\n"
        "\n"
        "interface DbResult {\n"
        "    id: number;\n"
        "    table: string;\n"
        "    data: string;\n"
        "}\n"
        "\n"
        "async function dbQuery(table: string, id: number): Promise<DbResult> {\n"
        "    return await order({ type: 'db_query', table, id });\n"
        "}\n"
        "\n"
        "const results: DbResult[] = [];\n"
        "\n"
        "for (let i: number = 1; i <= 3; i++) {\n"
        "    console.log(`Fetching item ${i}...`);\n"
        "    const item = await dbQuery('items', i);\n"
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
    printf("JavaScript code uses `await order({ type: ... })` from tsrun:host.\n");
    printf("The interpreter suspends, and the host fulfills orders with\n");
    printf("simulated responses (DB queries, HTTP fetches, etc.).\n");

    example_basic_async();
    example_multiple_calls();
    example_error_handling();
    example_loop();

    printf("\nDone!\n");
    return 0;
}
