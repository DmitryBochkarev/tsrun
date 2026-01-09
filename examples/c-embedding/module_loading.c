// module_loading.c - Module system example
//
// Demonstrates:
// - Loading ES modules with imports/exports
// - Step-based execution with TSRUN_STEP_NEED_IMPORTS
// - Providing module source code
// - Accessing module exports

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tsrun.h"
#include "tsrun_console.h"

// ============================================================================
// Simulated file system (in real code, you'd read from disk)
// ============================================================================

typedef struct {
    const char* path;
    const char* content;
} VirtualFile;

static const VirtualFile virtual_fs[] = {
    {
        "math.ts",
        "export const PI = 3.14159265358979;\n"
        "export const E = 2.71828182845905;\n"
        "\n"
        "export function square(x: number): number {\n"
        "    return x * x;\n"
        "}\n"
        "\n"
        "export function cube(x: number): number {\n"
        "    return x * x * x;\n"
        "}\n"
        "\n"
        "export function factorial(n: number): number {\n"
        "    if (n <= 1) return 1;\n"
        "    return n * factorial(n - 1);\n"
        "}\n"
    },
    {
        "utils.ts",
        "export function range(start: number, end: number): number[] {\n"
        "    const result: number[] = [];\n"
        "    for (let i = start; i < end; i++) {\n"
        "        result.push(i);\n"
        "    }\n"
        "    return result;\n"
        "}\n"
        "\n"
        "export function sum(arr: number[]): number {\n"
        "    return arr.reduce((a, b) => a + b, 0);\n"
        "}\n"
        "\n"
        "export function formatNumber(n: number, decimals: number = 2): string {\n"
        "    return n.toFixed(decimals);\n"
        "}\n"
    },
    {
        "config.ts",
        "interface Config {\n"
        "    appName: string;\n"
        "    version: string;\n"
        "    debug: boolean;\n"
        "    maxItems: number;\n"
        "}\n"
        "\n"
        "const config: Config = {\n"
        "    appName: 'MyApp',\n"
        "    version: '1.0.0',\n"
        "    debug: true,\n"
        "    maxItems: 100\n"
        "};\n"
        "\n"
        "export default config;\n"
        "\n"
        "export const environment: string = 'development';\n"
    },
    { NULL, NULL }
};

static const char* load_virtual_file(const char* path) {
    for (const VirtualFile* f = virtual_fs; f->path != NULL; f++) {
        if (strcmp(f->path, path) == 0) {
            return f->content;
        }
    }
    return NULL;
}

// ============================================================================
// Module loading loop
// ============================================================================

static TsRunStepResult run_with_modules(TsRunContext* ctx) {
    TsRunStepResult result = tsrun_run(ctx);

    while (result.status == TSRUN_STEP_NEED_IMPORTS) {
        printf("\n--- Module loader: %zu imports requested ---\n", result.import_count);

        for (size_t i = 0; i < result.import_count; i++) {
            const char* path = result.imports[i].resolved_path;
            const char* specifier = result.imports[i].specifier;
            const char* importer = result.imports[i].importer;

            printf("  Import: '%s' -> '%s'", specifier, path);
            if (importer) {
                printf(" (from %s)", importer);
            }
            printf("\n");

            const char* source = load_virtual_file(path);
            if (!source) {
                printf("  ERROR: Module not found: %s\n", path);
                // In real code, you'd handle this error properly
                tsrun_step_result_free(&result);
                result.status = TSRUN_STEP_ERROR;
                result.error = "Module not found";
                return result;
            }

            TsRunResult provide = tsrun_provide_module(ctx, path, source);
            if (!provide.ok) {
                printf("  ERROR: Failed to provide module: %s\n", provide.error);
                tsrun_step_result_free(&result);
                result.status = TSRUN_STEP_ERROR;
                result.error = provide.error;
                return result;
            }
            printf("  Loaded: %s (%zu bytes)\n", path, strlen(source));
        }

        tsrun_step_result_free(&result);
        result = tsrun_run(ctx);
    }

    return result;
}

// ============================================================================
// Example 1: Simple import
// ============================================================================

static void example_simple_import(void) {
    printf("\n========================================\n");
    printf("Example 1: Simple import\n");
    printf("========================================\n");

    TsRunContext* ctx = tsrun_new();
    tsrun_set_console(ctx, tsrun_console_stdio, NULL);

    const char* code =
        "import { PI, square, factorial } from './math.ts';\n"
        "\n"
        "interface MathResult {\n"
        "    pi: number;\n"
        "    squared5: number;\n"
        "    fact6: number;\n"
        "}\n"
        "\n"
        "const result: MathResult = {\n"
        "    pi: PI,\n"
        "    squared5: square(5),\n"
        "    fact6: factorial(6)\n"
        "};\n"
        "result;\n";

    printf("\nMain module:\n%s\n", code);

    TsRunResult prep = tsrun_prepare(ctx, code, "/main.ts");
    if (!prep.ok) {
        printf("Prepare error: %s\n", prep.error);
        tsrun_free(ctx);
        return;
    }

    TsRunStepResult result = run_with_modules(ctx);

    if (result.status == TSRUN_STEP_COMPLETE && result.value) {
        printf("\n--- Result ---\n");
        char* json = tsrun_json_stringify(ctx, result.value);
        if (json) {
            printf("%s\n", json);
            tsrun_free_string(json);
        }
        tsrun_value_free(result.value);
    } else if (result.status == TSRUN_STEP_ERROR) {
        printf("Error: %s\n", result.error);
    }

    tsrun_step_result_free(&result);
    tsrun_free(ctx);
}

// ============================================================================
// Example 2: Multiple imports
// ============================================================================

static void example_multiple_imports(void) {
    printf("\n========================================\n");
    printf("Example 2: Multiple imports\n");
    printf("========================================\n");

    TsRunContext* ctx = tsrun_new();
    tsrun_set_console(ctx, tsrun_console_stdio, NULL);

    const char* code =
        "import { square, cube } from './math.ts';\n"
        "import { range, sum, formatNumber } from './utils.ts';\n"
        "\n"
        "// Calculate sum of squares from 1 to 5\n"
        "const numbers: number[] = range(1, 6);\n"
        "const squares: number[] = numbers.map(square);\n"
        "const total: number = sum(squares);\n"
        "\n"
        "`Sum of squares 1-5: ${formatNumber(total, 0)}`;\n";

    printf("\nMain module:\n%s\n", code);

    TsRunResult prep = tsrun_prepare(ctx, code, "/main.ts");
    if (!prep.ok) {
        printf("Prepare error: %s\n", prep.error);
        tsrun_free(ctx);
        return;
    }

    TsRunStepResult result = run_with_modules(ctx);

    if (result.status == TSRUN_STEP_COMPLETE && result.value) {
        printf("\n--- Result ---\n");
        printf("%s\n", tsrun_get_string(result.value));
        tsrun_value_free(result.value);
    } else if (result.status == TSRUN_STEP_ERROR) {
        printf("Error: %s\n", result.error);
    }

    tsrun_step_result_free(&result);
    tsrun_free(ctx);
}

// ============================================================================
// Example 3: Default export
// ============================================================================

static void example_default_export(void) {
    printf("\n========================================\n");
    printf("Example 3: Default export\n");
    printf("========================================\n");

    TsRunContext* ctx = tsrun_new();
    tsrun_set_console(ctx, tsrun_console_stdio, NULL);

    const char* code =
        "import config, { environment } from './config.ts';\n"
        "\n"
        "interface AppInfo {\n"
        "    app: string;\n"
        "    version: string;\n"
        "    env: string;\n"
        "    debug: boolean;\n"
        "}\n"
        "\n"
        "const info: AppInfo = {\n"
        "    app: config.appName,\n"
        "    version: config.version,\n"
        "    env: environment,\n"
        "    debug: config.debug\n"
        "};\n"
        "info;\n";

    printf("\nMain module:\n%s\n", code);

    TsRunResult prep = tsrun_prepare(ctx, code, "/main.ts");
    if (!prep.ok) {
        printf("Prepare error: %s\n", prep.error);
        tsrun_free(ctx);
        return;
    }

    TsRunStepResult result = run_with_modules(ctx);

    if (result.status == TSRUN_STEP_COMPLETE && result.value) {
        printf("\n--- Result ---\n");
        char* json = tsrun_json_stringify(ctx, result.value);
        if (json) {
            printf("%s\n", json);
            tsrun_free_string(json);
        }
        tsrun_value_free(result.value);
    } else if (result.status == TSRUN_STEP_ERROR) {
        printf("Error: %s\n", result.error);
    }

    tsrun_step_result_free(&result);
    tsrun_free(ctx);
}

// ============================================================================
// Example 4: Accessing exports from C
// ============================================================================

static void example_access_exports(void) {
    printf("\n========================================\n");
    printf("Example 4: Accessing exports from C\n");
    printf("========================================\n");

    TsRunContext* ctx = tsrun_new();
    tsrun_set_console(ctx, tsrun_console_stdio, NULL);

    const char* code =
        "export const VERSION = '2.0.0';\n"
        "export const MAX_SIZE = 1024;\n"
        "\n"
        "export function greet(name: string): string {\n"
        "    return `Hello, ${name}!`;\n"
        "}\n"
        "\n"
        "export default class Calculator {\n"
        "    add(a: number, b: number): number {\n"
        "        return a + b;\n"
        "    }\n"
        "}\n"
        "\n"
        "console.log('Module initialized');\n";

    printf("\nModule with exports:\n%s\n", code);

    TsRunResult prep = tsrun_prepare(ctx, code, "/calculator.ts");
    if (!prep.ok) {
        printf("Prepare error: %s\n", prep.error);
        tsrun_free(ctx);
        return;
    }

    TsRunStepResult result = run_with_modules(ctx);

    if (result.status == TSRUN_STEP_COMPLETE) {
        printf("\n--- Accessing exports from C ---\n");

        // List all exports
        size_t export_count;
        char** exports = tsrun_get_export_names(ctx, &export_count);
        printf("Exports (%zu): ", export_count);
        for (size_t i = 0; i < export_count; i++) {
            printf("%s%s", exports[i], i < export_count - 1 ? ", " : "\n");
        }
        tsrun_free_strings(exports, export_count);

        // Get VERSION
        TsRunValueResult version_r = tsrun_get_export(ctx, "VERSION");
        if (version_r.value) {
            printf("VERSION = \"%s\"\n", tsrun_get_string(version_r.value));
            tsrun_value_free(version_r.value);
        }

        // Get MAX_SIZE
        TsRunValueResult size_r = tsrun_get_export(ctx, "MAX_SIZE");
        if (size_r.value) {
            printf("MAX_SIZE = %g\n", tsrun_get_number(size_r.value));
            tsrun_value_free(size_r.value);
        }

        // Call greet function
        TsRunValueResult greet_r = tsrun_get_export(ctx, "greet");
        if (greet_r.value && tsrun_is_function(greet_r.value)) {
            TsRunValue* name = tsrun_string(ctx, "World");
            TsRunValue* args[] = { name };
            TsRunValueResult greeting_r = tsrun_call(ctx, greet_r.value, NULL, args, 1);
            if (greeting_r.value) {
                printf("greet('World') = \"%s\"\n", tsrun_get_string(greeting_r.value));
                tsrun_value_free(greeting_r.value);
            }
            tsrun_value_free(name);
            tsrun_value_free(greet_r.value);
        }

        if (result.value) {
            tsrun_value_free(result.value);
        }
    } else if (result.status == TSRUN_STEP_ERROR) {
        printf("Error: %s\n", result.error);
    }

    tsrun_step_result_free(&result);
    tsrun_free(ctx);
}

// ============================================================================
// Main
// ============================================================================

int main(void) {
    printf("tsrun C API - Module Loading Example\n");

    example_simple_import();
    example_multiple_imports();
    example_default_export();
    example_access_exports();

    printf("\nDone!\n");
    return 0;
}
