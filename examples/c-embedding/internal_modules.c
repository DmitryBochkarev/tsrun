// internal_modules.c - Creating importable native modules
//
// Demonstrates:
// - Creating internal modules with native functions
// - Adding value exports (constants)
// - Multiple modules with different specifiers
// - Importing and using native modules from JavaScript
//
// This shows how to create Node.js-style native modules that can be
// imported using ES module syntax: import { add } from "myapp:math";

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include "tsrun.h"
#include "tsrun_console.h"

// ============================================================================
// Math module functions
// ============================================================================

static TsRunValue* math_add(
    TsRunContext* ctx,
    TsRunValue* this_arg,
    TsRunValue** args,
    size_t argc,
    void* userdata,
    const char** error_out
) {
    (void)this_arg;
    (void)userdata;

    double a = (argc > 0 && tsrun_is_number(args[0])) ? tsrun_get_number(args[0]) : 0.0;
    double b = (argc > 1 && tsrun_is_number(args[1])) ? tsrun_get_number(args[1]) : 0.0;

    return tsrun_number(ctx, a + b);
}

static TsRunValue* math_multiply(
    TsRunContext* ctx,
    TsRunValue* this_arg,
    TsRunValue** args,
    size_t argc,
    void* userdata,
    const char** error_out
) {
    (void)this_arg;
    (void)userdata;

    double a = (argc > 0 && tsrun_is_number(args[0])) ? tsrun_get_number(args[0]) : 0.0;
    double b = (argc > 1 && tsrun_is_number(args[1])) ? tsrun_get_number(args[1]) : 1.0;

    return tsrun_number(ctx, a * b);
}

static TsRunValue* math_pow(
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
        *error_out = "pow() requires 2 arguments";
        return NULL;
    }

    double base = tsrun_is_number(args[0]) ? tsrun_get_number(args[0]) : 0.0;
    double exp = tsrun_is_number(args[1]) ? tsrun_get_number(args[1]) : 0.0;

    double result = 1.0;
    for (int i = 0; i < (int)exp; i++) {
        result *= base;
    }

    return tsrun_number(ctx, result);
}

// ============================================================================
// String module functions
// ============================================================================

static TsRunValue* string_uppercase(
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
        *error_out = "uppercase() requires a string argument";
        return NULL;
    }

    const char* input = tsrun_get_string(args[0]);
    size_t len = strlen(input);

    char* result = malloc(len + 1);
    if (!result) {
        *error_out = "Memory allocation failed";
        return NULL;
    }

    for (size_t i = 0; i < len; i++) {
        result[i] = (char)toupper((unsigned char)input[i]);
    }
    result[len] = '\0';

    TsRunValue* ret = tsrun_string(ctx, result);
    free(result);
    return ret;
}

static TsRunValue* string_reverse(
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
        *error_out = "reverse() requires a string argument";
        return NULL;
    }

    const char* input = tsrun_get_string(args[0]);
    size_t len = strlen(input);

    char* result = malloc(len + 1);
    if (!result) {
        *error_out = "Memory allocation failed";
        return NULL;
    }

    for (size_t i = 0; i < len; i++) {
        result[i] = input[len - 1 - i];
    }
    result[len] = '\0';

    TsRunValue* ret = tsrun_string(ctx, result);
    free(result);
    return ret;
}

static TsRunValue* string_repeat(
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
        *error_out = "repeat() requires 2 arguments (string, count)";
        return NULL;
    }

    if (!tsrun_is_string(args[0]) || !tsrun_is_number(args[1])) {
        *error_out = "repeat() requires (string, number) arguments";
        return NULL;
    }

    const char* input = tsrun_get_string(args[0]);
    int count = (int)tsrun_get_number(args[1]);
    size_t len = strlen(input);

    if (count <= 0) {
        return tsrun_string(ctx, "");
    }

    size_t result_len = len * count;
    char* result = malloc(result_len + 1);
    if (!result) {
        *error_out = "Memory allocation failed";
        return NULL;
    }

    for (int i = 0; i < count; i++) {
        memcpy(result + i * len, input, len);
    }
    result[result_len] = '\0';

    TsRunValue* ret = tsrun_string(ctx, result);
    free(result);
    return ret;
}

// ============================================================================
// Module setup
// ============================================================================

static void setup_modules(TsRunContext* ctx) {
    // Create "myapp:math" module
    TsRunInternalModule* math_mod = tsrun_internal_module_new("myapp:math");

    // Add function exports
    tsrun_internal_module_add_function(math_mod, "add", math_add, 2, NULL);
    tsrun_internal_module_add_function(math_mod, "multiply", math_multiply, 2, NULL);
    tsrun_internal_module_add_function(math_mod, "pow", math_pow, 2, NULL);

    // Add value exports (constants)
    tsrun_internal_module_add_value(math_mod, "PI", tsrun_number(ctx, 3.14159265358979));
    tsrun_internal_module_add_value(math_mod, "E", tsrun_number(ctx, 2.71828182845905));

    // Register the module
    TsRunResult result = tsrun_register_internal_module(ctx, math_mod);
    if (!result.ok) {
        printf("Failed to register myapp:math module: %s\n", result.error);
    }

    // Create "myapp:string" module
    TsRunInternalModule* string_mod = tsrun_internal_module_new("myapp:string");

    tsrun_internal_module_add_function(string_mod, "uppercase", string_uppercase, 1, NULL);
    tsrun_internal_module_add_function(string_mod, "reverse", string_reverse, 1, NULL);
    tsrun_internal_module_add_function(string_mod, "repeat", string_repeat, 2, NULL);

    // Add string constants
    tsrun_internal_module_add_value(string_mod, "EMPTY", tsrun_string(ctx, ""));
    tsrun_internal_module_add_value(string_mod, "NEWLINE", tsrun_string(ctx, "\n"));

    result = tsrun_register_internal_module(ctx, string_mod);
    if (!result.ok) {
        printf("Failed to register myapp:string module: %s\n", result.error);
    }
}

// ============================================================================
// Helper to run code with module support
// ============================================================================

static void run_code(const char* title, const char* code) {
    printf("\n========================================\n");
    printf("%s\n", title);
    printf("========================================\n");
    printf("\nCode:\n%s\n", code);
    printf("\n--- Output ---\n");

    TsRunContext* ctx = tsrun_new();
    tsrun_set_console(ctx, tsrun_console_stdio, NULL);

    // Set up internal modules before running code
    setup_modules(ctx);

    // Prepare and run
    TsRunResult prep = tsrun_prepare(ctx, code, "/main.ts");
    if (!prep.ok) {
        printf("Prepare error: %s\n", prep.error);
        tsrun_free(ctx);
        return;
    }

    TsRunStepResult result = tsrun_run(ctx);

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
    }

    tsrun_step_result_free(&result);
    tsrun_free(ctx);
}

// ============================================================================
// Examples
// ============================================================================

static void example_basic_import(void) {
    run_code(
        "Example 1: Basic module import",
        "import { add, multiply, PI } from 'myapp:math';\n"
        "\n"
        "const sum = add(10, 20);\n"
        "const product = multiply(5, 6);\n"
        "const circumference = multiply(2, multiply(PI, 5));\n"
        "\n"
        "console.log('Sum:', sum);\n"
        "console.log('Product:', product);\n"
        "console.log('Circumference (r=5):', circumference);\n"
        "\n"
        "({ sum, product, circumference });\n"
    );
}

static void example_string_module(void) {
    run_code(
        "Example 2: String module functions",
        "import { uppercase, reverse, repeat } from 'myapp:string';\n"
        "\n"
        "const text = 'hello world';\n"
        "\n"
        "console.log('Original:', text);\n"
        "console.log('Uppercase:', uppercase(text));\n"
        "console.log('Reversed:', reverse(text));\n"
        "console.log('Repeated 3x:', repeat(text, 3));\n"
        "\n"
        "uppercase(text);\n"
    );
}

static void example_namespace_import(void) {
    run_code(
        "Example 3: Namespace import",
        "import * as math from 'myapp:math';\n"
        "import * as str from 'myapp:string';\n"
        "\n"
        "// Use qualified names\n"
        "const result = math.add(math.PI, math.E);\n"
        "console.log('PI + E =', result);\n"
        "\n"
        "const greeting = str.uppercase('hello');\n"
        "console.log('Greeting:', greeting);\n"
        "\n"
        "result;\n"
    );
}

static void example_combined_usage(void) {
    run_code(
        "Example 4: Combined module usage",
        "import { pow, PI } from 'myapp:math';\n"
        "import { uppercase, reverse } from 'myapp:string';\n"
        "\n"
        "// Calculate area of circle with radius 10\n"
        "const radius = 10;\n"
        "const area = PI * pow(radius, 2);\n"
        "console.log(`Area of circle (r=${radius}):`, area);\n"
        "\n"
        "// Play with strings\n"
        "const name = 'typescript';\n"
        "const processed = uppercase(reverse(name));\n"
        "console.log('Processed name:', processed);\n"
        "\n"
        "({ area, processed });\n"
    );
}

static void example_constants(void) {
    run_code(
        "Example 5: Using exported constants",
        "import { PI, E } from 'myapp:math';\n"
        "import { EMPTY, NEWLINE } from 'myapp:string';\n"
        "\n"
        "console.log('Math constants:');\n"
        "console.log('  PI =', PI);\n"
        "console.log('  E =', E);\n"
        "\n"
        "console.log('String constants:');\n"
        "console.log('  EMPTY is empty:', EMPTY === '');\n"
        "console.log('  NEWLINE:', JSON.stringify(NEWLINE));\n"
        "\n"
        "({ PI, E });\n"
    );
}

// ============================================================================
// Main
// ============================================================================

int main(void) {
    printf("tsrun C API - Internal Modules Example\n");
    printf("======================================\n\n");
    printf("This example demonstrates how to create native modules\n");
    printf("that can be imported from JavaScript using ES module syntax.\n");
    printf("\n");
    printf("Two modules are created:\n");
    printf("  - 'myapp:math'   - Math functions and constants\n");
    printf("  - 'myapp:string' - String manipulation functions\n");

    example_basic_import();
    example_string_module();
    example_namespace_import();
    example_combined_usage();
    example_constants();

    printf("\nDone!\n");
    return 0;
}
