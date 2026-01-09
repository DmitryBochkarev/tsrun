// event_system.c - Event emitter pattern with deferred promises
//
// Demonstrates:
// - Creating deferred promises with tsrun_create_order_promise()
// - Resolving promises from C when events fire
// - Multiple subscribers waiting on same event type
// - Promise rejection for error/timeout scenarios
// - Event queue management in C
//
// This shows how to implement an event-driven system where JavaScript
// code can await events that are dispatched from C.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tsrun.h"
#include "tsrun_console.h"

// ============================================================================
// Event Subscription Storage
// ============================================================================

#define MAX_SUBSCRIPTIONS 64

typedef struct {
    TsRunOrderId order_id;
    TsRunValue* promise;
    char event_name[64];
    int active;
} Subscription;

static Subscription subscriptions[MAX_SUBSCRIPTIONS];
static int subscription_count = 0;
static TsRunOrderId next_order_id = 1;

static void reset_subscriptions(void) {
    for (int i = 0; i < MAX_SUBSCRIPTIONS; i++) {
        if (subscriptions[i].active && subscriptions[i].promise) {
            tsrun_value_free(subscriptions[i].promise);
        }
        subscriptions[i].active = 0;
        subscriptions[i].promise = NULL;
    }
    subscription_count = 0;
}

// ============================================================================
// Native Functions for Event System
// ============================================================================

// subscribe(eventName) -> Promise
// Creates a promise that resolves when the named event fires
static TsRunValue* native_subscribe(
    TsRunContext* ctx,
    TsRunValue* this_arg,
    TsRunValue** args,
    size_t argc,
    void* userdata,
    const char** error_out
) {
    (void)this_arg;
    (void)userdata;

    if (argc < 1 || !tsrun_is_string(args[0])) {
        *error_out = "subscribe() requires an event name string";
        return NULL;
    }

    const char* event_name = tsrun_get_string(args[0]);

    // Find a free subscription slot
    int slot = -1;
    for (int i = 0; i < MAX_SUBSCRIPTIONS; i++) {
        if (!subscriptions[i].active) {
            slot = i;
            break;
        }
    }

    if (slot < 0) {
        *error_out = "Too many subscriptions";
        return NULL;
    }

    // Generate an order ID for this subscription
    TsRunOrderId order_id = next_order_id++;

    // Create a deferred promise for this subscription
    TsRunValueResult promise_result = tsrun_create_order_promise(ctx, order_id);
    if (!promise_result.value) {
        *error_out = promise_result.error ? promise_result.error : "Failed to create promise";
        return NULL;
    }

    // Store the subscription
    subscriptions[slot].order_id = order_id;
    subscriptions[slot].promise = tsrun_value_dup(ctx, promise_result.value);
    strncpy(subscriptions[slot].event_name, event_name, sizeof(subscriptions[slot].event_name) - 1);
    subscriptions[slot].event_name[sizeof(subscriptions[slot].event_name) - 1] = '\0';
    subscriptions[slot].active = 1;
    subscription_count++;

    printf("  [C] Subscribed to '%s' (order %llu)\n", event_name, (unsigned long long)order_id);

    return promise_result.value;
}

// getSubscriptionCount() -> number
// Returns the current number of active subscriptions
static TsRunValue* native_get_subscription_count(
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
    (void)error_out;

    return tsrun_number(ctx, subscription_count);
}

// ============================================================================
// C-side Event Emission
// ============================================================================

// Emit an event from C, resolving all subscribed promises
static int emit_event(TsRunContext* ctx, const char* event_name, TsRunValue* data) {
    int resolved = 0;

    printf("  [C] Emitting event '%s'\n", event_name);

    for (int i = 0; i < MAX_SUBSCRIPTIONS; i++) {
        if (subscriptions[i].active && strcmp(subscriptions[i].event_name, event_name) == 0) {
            // Resolve the promise with the event data
            TsRunResult result = tsrun_resolve_promise(ctx, subscriptions[i].promise, data);
            if (result.ok) {
                printf("  [C] Resolved subscription (order %llu)\n",
                       (unsigned long long)subscriptions[i].order_id);
                resolved++;
            } else {
                printf("  [C] Failed to resolve: %s\n", result.error);
            }

            // Clean up this subscription
            tsrun_value_free(subscriptions[i].promise);
            subscriptions[i].promise = NULL;
            subscriptions[i].active = 0;
            subscription_count--;
        }
    }

    return resolved;
}

// Reject all subscriptions for an event (e.g., for timeout or error)
static int reject_event(TsRunContext* ctx, const char* event_name, const char* error_msg) {
    int rejected = 0;

    printf("  [C] Rejecting event '%s': %s\n", event_name, error_msg);

    for (int i = 0; i < MAX_SUBSCRIPTIONS; i++) {
        if (subscriptions[i].active && strcmp(subscriptions[i].event_name, event_name) == 0) {
            TsRunResult result = tsrun_reject_promise(ctx, subscriptions[i].promise, error_msg);
            if (result.ok) {
                printf("  [C] Rejected subscription (order %llu)\n",
                       (unsigned long long)subscriptions[i].order_id);
                rejected++;
            }

            tsrun_value_free(subscriptions[i].promise);
            subscriptions[i].promise = NULL;
            subscriptions[i].active = 0;
            subscription_count--;
        }
    }

    return rejected;
}

// ============================================================================
// Setup and Execution
// ============================================================================

static void setup_event_functions(TsRunContext* ctx) {
    TsRunValueResult subscribe_fn = tsrun_native_function(ctx, "subscribe", native_subscribe, 1, NULL);
    TsRunValueResult count_fn = tsrun_native_function(ctx, "getSubscriptionCount", native_get_subscription_count, 0, NULL);

    if (subscribe_fn.value) {
        tsrun_set_global(ctx, "subscribe", subscribe_fn.value);
        tsrun_value_free(subscribe_fn.value);
    }
    if (count_fn.value) {
        tsrun_set_global(ctx, "getSubscriptionCount", count_fn.value);
        tsrun_value_free(count_fn.value);
    }
}

// Run code, process events, and continue until completion
static void run_with_events(
    const char* title,
    const char* code,
    void (*event_simulator)(TsRunContext* ctx)
) {
    printf("\n========================================\n");
    printf("%s\n", title);
    printf("========================================\n");
    printf("\nCode:\n%s\n", code);
    printf("\n--- Execution ---\n");

    TsRunContext* ctx = tsrun_new();
    tsrun_set_console(ctx, tsrun_console_stdio, NULL);
    reset_subscriptions();
    setup_event_functions(ctx);

    TsRunResult prep = tsrun_prepare(ctx, code, "/main.ts");
    if (!prep.ok) {
        printf("Prepare error: %s\n", prep.error);
        tsrun_free(ctx);
        return;
    }

    // Initial run
    TsRunStepResult result = tsrun_run(ctx);

    // Event loop: run -> handle suspension -> emit events -> continue
    int iterations = 0;
    while (result.status == TSRUN_STEP_SUSPENDED && iterations < 10) {
        printf("\n--- Suspended with %zu pending orders ---\n", result.pending_count);

        // Simulate events from C side
        if (event_simulator) {
            event_simulator(ctx);
        }

        // Continue execution
        tsrun_step_result_free(&result);
        result = tsrun_run(ctx);
        iterations++;
    }

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
        } else {
            printf("undefined\n");
        }
    } else if (result.status == TSRUN_STEP_ERROR) {
        printf("\n--- Error ---\n%s\n", result.error);
    } else if (result.status == TSRUN_STEP_SUSPENDED) {
        printf("\n--- Still suspended (max iterations reached) ---\n");
    }

    tsrun_step_result_free(&result);
    reset_subscriptions();
    tsrun_free(ctx);
}

// ============================================================================
// Event Simulators for Examples
// ============================================================================

static int example1_iteration = 0;

static void example1_simulator(TsRunContext* ctx) {
    example1_iteration++;
    if (example1_iteration == 1) {
        // First suspension: emit the login event
        TsRunValue* data = tsrun_string(ctx, "alice@example.com");
        emit_event(ctx, "login", data);
        tsrun_value_free(data);
    }
}

static int example2_iteration = 0;

static void example2_simulator(TsRunContext* ctx) {
    example2_iteration++;

    // Emit events one at a time to demonstrate multiple subscribers
    if (example2_iteration == 1) {
        TsRunValueResult obj = tsrun_object_new(ctx);
        TsRunValue* msg = tsrun_string(ctx, "Hello from server!");
        tsrun_set(ctx, obj.value, "text", msg);
        tsrun_value_free(msg);

        emit_event(ctx, "message", obj.value);
        tsrun_value_free(obj.value);
    } else if (example2_iteration == 2) {
        TsRunValueResult obj = tsrun_object_new(ctx);
        TsRunValue* msg = tsrun_string(ctx, "Second message!");
        tsrun_set(ctx, obj.value, "text", msg);
        tsrun_value_free(msg);

        emit_event(ctx, "message", obj.value);
        tsrun_value_free(obj.value);
    }
}

static int example3_iteration = 0;

static void example3_simulator(TsRunContext* ctx) {
    example3_iteration++;

    // Simulate a timeout scenario - reject instead of resolve
    if (example3_iteration == 1) {
        reject_event(ctx, "timeout-test", "Operation timed out after 5000ms");
    }
}

static int example4_iteration = 0;

static void example4_simulator(TsRunContext* ctx) {
    example4_iteration++;

    // Emit multiple events for the race condition
    if (example4_iteration == 1) {
        // Emit 'fast' first - it should win the race
        TsRunValue* data = tsrun_string(ctx, "fast-result");
        emit_event(ctx, "fast", data);
        tsrun_value_free(data);

        // Emit 'slow' after - but it was already cancelled by Promise.race
        data = tsrun_string(ctx, "slow-result");
        emit_event(ctx, "slow", data);
        tsrun_value_free(data);
    }
}

// ============================================================================
// Examples
// ============================================================================

static void example_basic_subscription(void) {
    example1_iteration = 0;
    run_with_events(
        "Example 1: Basic event subscription",
        "declare function subscribe(event: string): Promise<any>;\n"
        "\n"
        "console.log('Waiting for login event...');\n"
        "\n"
        "// Subscribe and await the event\n"
        "const userEmail = await subscribe('login');\n"
        "console.log('User logged in:', userEmail);\n"
        "\n"
        "userEmail;\n",
        example1_simulator
    );
}

static void example_multiple_subscribers(void) {
    example2_iteration = 0;
    run_with_events(
        "Example 2: Sequential event handling",
        "declare function subscribe(event: string): Promise<any>;\n"
        "\n"
        "console.log('Waiting for messages...');\n"
        "\n"
        "// Wait for first message\n"
        "const msg1 = await subscribe('message');\n"
        "console.log('Message 1:', msg1.text);\n"
        "\n"
        "// Wait for second message\n"
        "const msg2 = await subscribe('message');\n"
        "console.log('Message 2:', msg2.text);\n"
        "\n"
        "({ msg1, msg2 });\n",
        example2_simulator
    );
}

static void example_error_handling(void) {
    example3_iteration = 0;
    run_with_events(
        "Example 3: Error handling (timeout simulation)",
        "declare function subscribe(event: string): Promise<any>;\n"
        "\n"
        "console.log('Starting operation with timeout...');\n"
        "\n"
        "try {\n"
        "    const result = await subscribe('timeout-test');\n"
        "    console.log('Got result:', result);\n"
        "} catch (e) {\n"
        "    console.log('Caught error:', e.message);\n"
        "}\n"
        "\n"
        "'Error handled gracefully';\n",
        example3_simulator
    );
}

static void example_promise_race(void) {
    example4_iteration = 0;
    run_with_events(
        "Example 4: Promise.race pattern",
        "declare function subscribe(event: string): Promise<any>;\n"
        "\n"
        "console.log('Racing between fast and slow events...');\n"
        "\n"
        "// Race between two events\n"
        "const winner = await Promise.race([\n"
        "    subscribe('fast'),\n"
        "    subscribe('slow')\n"
        "]);\n"
        "\n"
        "console.log('Winner:', winner);\n"
        "winner;\n",
        example4_simulator
    );
}

// ============================================================================
// Main
// ============================================================================

int main(void) {
    printf("tsrun C API - Event System Example\n");
    printf("==================================\n\n");
    printf("This example demonstrates how to implement an event-driven\n");
    printf("architecture using deferred promises. JavaScript code can\n");
    printf("await events that are dispatched from C code.\n");
    printf("\n");
    printf("Key APIs used:\n");
    printf("  - tsrun_create_order_promise() - Create a deferred promise\n");
    printf("  - tsrun_resolve_promise()      - Resolve when event fires\n");
    printf("  - tsrun_reject_promise()       - Reject on error/timeout\n");

    example_basic_subscription();
    example_multiple_subscribers();
    example_error_handling();
    example_promise_race();

    printf("\nDone!\n");
    return 0;
}
