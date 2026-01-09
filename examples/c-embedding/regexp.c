// regexp.c - RegExp demonstration with custom PCRE2 provider
//
// Demonstrates:
// - Setting up a custom PCRE2-based regex provider
// - Basic pattern matching (test, exec)
// - Capture groups (numbered and nested)
// - All supported flags (g, i, m, s)
// - String methods (match, matchAll, replace, split, search)
// - Global iteration pattern
// - Error handling for invalid patterns
// - Catastrophic backtracking protection
//
// Prerequisites:
//   - PCRE2 library (apt install libpcre2-dev / brew install pcre2)
//   - Build with: make regexp

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tsrun.h"
#include "tsrun_console.h"
#include "regexp_provider.h"

// ============================================================================
// Helper Functions
// ============================================================================

// Run code and print result
static void eval_and_print(TsRunContext* ctx, const char* description, const char* code) {
    printf("\n--- %s ---\n", description);
    printf("> %s\n", code);

    TsRunResult prep = tsrun_prepare(ctx, code, NULL);
    if (!prep.ok) {
        printf("Prepare error: %s\n", prep.error);
        return;
    }

    TsRunStepResult result = tsrun_run(ctx);

    switch (result.status) {
        case TSRUN_STEP_COMPLETE:
            if (result.value) {
                if (tsrun_is_null(result.value)) {
                    printf("=> null\n");
                } else if (tsrun_is_undefined(result.value)) {
                    printf("=> undefined\n");
                } else if (tsrun_is_boolean(result.value)) {
                    printf("=> %s\n", tsrun_get_bool(result.value) ? "true" : "false");
                } else if (tsrun_is_number(result.value)) {
                    printf("=> %g\n", tsrun_get_number(result.value));
                } else if (tsrun_is_string(result.value)) {
                    printf("=> \"%s\"\n", tsrun_get_string(result.value));
                } else {
                    char* json = tsrun_json_stringify(ctx, result.value);
                    if (json) {
                        printf("=> %s\n", json);
                        tsrun_free_string(json);
                    } else {
                        printf("=> [object]\n");
                    }
                }
                tsrun_value_free(result.value);
            } else {
                printf("=> undefined\n");
            }
            break;
        case TSRUN_STEP_ERROR:
            printf("Error: %s\n", result.error);
            break;
        default:
            printf("Unexpected status: %d\n", result.status);
            break;
    }

    tsrun_step_result_free(&result);
}

// ============================================================================
// Demo Sections
// ============================================================================

static void demo_basic_matching(TsRunContext* ctx) {
    printf("\n");
    printf("========================================\n");
    printf("1. Basic Pattern Matching\n");
    printf("========================================\n");

    eval_and_print(ctx, "test() - simple match",
        "/hello/.test('hello world')");

    eval_and_print(ctx, "test() - no match",
        "/xyz/.test('hello world')");

    eval_and_print(ctx, "exec() - returns match array",
        "/world/.exec('hello world')");

    eval_and_print(ctx, "exec() - no match returns null",
        "/xyz/.exec('hello world')");

    eval_and_print(ctx, "exec() result has index property",
        "/world/.exec('hello world').index");

    eval_and_print(ctx, "exec() result has input property",
        "/world/.exec('hello world').input");

    eval_and_print(ctx, "RegExp constructor",
        "new RegExp('hello').test('hello world')");

    eval_and_print(ctx, "Pattern with special chars",
        "/\\d+/.test('abc123def')");
}

static void demo_capture_groups(TsRunContext* ctx) {
    printf("\n");
    printf("========================================\n");
    printf("2. Capture Groups\n");
    printf("========================================\n");

    eval_and_print(ctx, "Single capture group",
        "/hello (\\w+)/.exec('hello world')");

    eval_and_print(ctx, "Multiple capture groups",
        "/(\\w+)@(\\w+)\\.(\\w+)/.exec('user@example.com')");

    eval_and_print(ctx, "Nested capture groups",
        "/((\\d+)-(\\d+))-(\\d+)/.exec('123-456-7890')");

    eval_and_print(ctx, "Optional capture group (non-participating)",
        "/(a)(b)?(c)/.exec('ac')");

    eval_and_print(ctx, "Accessing specific capture",
        "const m = /(\\w+)@(\\w+)/.exec('user@host'); m[1] + ' at ' + m[2]");
}

static void demo_flags(TsRunContext* ctx) {
    printf("\n");
    printf("========================================\n");
    printf("3. Regex Flags\n");
    printf("========================================\n");

    // Case insensitive (i)
    eval_and_print(ctx, "Case sensitive (default)",
        "/hello/.test('HELLO')");

    eval_and_print(ctx, "Case insensitive (i flag)",
        "/hello/i.test('HELLO')");

    // Multiline (m)
    eval_and_print(ctx, "^ without multiline",
        "/^world/.test('hello\\nworld')");

    eval_and_print(ctx, "^ with multiline (m flag)",
        "/^world/m.test('hello\\nworld')");

    eval_and_print(ctx, "$ with multiline",
        "/hello$/m.test('hello\\nworld')");

    // DotAll (s)
    eval_and_print(ctx, ". without dotAll (doesn't match newline)",
        "/hello.world/.test('hello\\nworld')");

    eval_and_print(ctx, ". with dotAll (s flag)",
        "/hello.world/s.test('hello\\nworld')");

    // Global (g) - affects iteration
    eval_and_print(ctx, "match() without global - first match only",
        "'abcabc'.match(/a/)");

    eval_and_print(ctx, "match() with global - all matches",
        "'abcabc'.match(/a/g)");

    // Flag properties
    eval_and_print(ctx, "Check flag properties",
        "const r = /test/gims; [r.global, r.ignoreCase, r.multiline, r.dotAll]");
}

static void demo_string_methods(TsRunContext* ctx) {
    printf("\n");
    printf("========================================\n");
    printf("4. String Methods with RegExp\n");
    printf("========================================\n");

    // match()
    eval_and_print(ctx, "match() - find pattern",
        "'hello world'.match(/o/)");

    eval_and_print(ctx, "match() - global finds all",
        "'hello world'.match(/o/g)");

    eval_and_print(ctx, "match() - with captures (non-global)",
        "'hello world'.match(/(\\w+) (\\w+)/)");

    // matchAll()
    eval_and_print(ctx, "matchAll() - iterate all matches with captures",
        "[...'a1b2c3'.matchAll(/(\\w)(\\d)/g)].map(m => m[0])");

    // search()
    eval_and_print(ctx, "search() - find index",
        "'hello world'.search(/world/)");

    eval_and_print(ctx, "search() - not found",
        "'hello world'.search(/xyz/)");

    // split()
    eval_and_print(ctx, "split() - by pattern",
        "'a1b2c3'.split(/\\d/)");

    eval_and_print(ctx, "split() - by pattern with limit",
        "'a,b;c d'.split(/[,;\\s]/)");

    // replace()
    eval_and_print(ctx, "replace() - first match only",
        "'hello hello'.replace(/hello/, 'hi')");

    eval_and_print(ctx, "replace() - global replaces all",
        "'hello hello'.replace(/hello/g, 'hi')");

    eval_and_print(ctx, "replace() - with capture reference",
        "'John Smith'.replace(/(\\w+) (\\w+)/, '$2, $1')");

    eval_and_print(ctx, "replace() - with callback function",
        "'hello world'.replace(/\\w+/g, s => s.toUpperCase())");

    eval_and_print(ctx, "replace() - callback with captures",
        "'font-size'.replace(/-([a-z])/g, (_, c) => c.toUpperCase())");

    // replaceAll()
    eval_and_print(ctx, "replaceAll() - requires global flag",
        "'a1b2c3'.replace(/\\d/g, 'X')");
}

static void demo_global_iteration(TsRunContext* ctx) {
    printf("\n");
    printf("========================================\n");
    printf("5. Global Iteration Pattern\n");
    printf("========================================\n");

    eval_and_print(ctx, "Classic while loop with exec()",
        "const text = 'a1b23c456';\n"
        "const pattern = /(\\d+)/g;\n"
        "const results: string[] = [];\n"
        "let match: RegExpExecArray | null;\n"
        "while ((match = pattern.exec(text)) !== null) {\n"
        "    results.push(match[0]);\n"
        "}\n"
        "results.join(', ')");

    eval_and_print(ctx, "Extract all URLs",
        "const html = '<a href=\"http://a.com\">A</a> <a href=\"http://b.com\">B</a>';\n"
        "const urls: string[] = [];\n"
        "const re = /href=\"([^\"]+)\"/g;\n"
        "let m: RegExpExecArray | null;\n"
        "while ((m = re.exec(html)) !== null) {\n"
        "    urls.push(m[1]);\n"
        "}\n"
        "urls");

    eval_and_print(ctx, "Parse key=value pairs",
        "const params = 'name=John&age=30&city=NYC';\n"
        "const pairs: { key: string; value: string }[] = [];\n"
        "const re = /(\\w+)=(\\w+)/g;\n"
        "let m: RegExpExecArray | null;\n"
        "while ((m = re.exec(params)) !== null) {\n"
        "    pairs.push({ key: m[1], value: m[2] });\n"
        "}\n"
        "pairs");
}

static void demo_error_handling(TsRunContext* ctx) {
    printf("\n");
    printf("========================================\n");
    printf("6. Error Handling\n");
    printf("========================================\n");

    eval_and_print(ctx, "Invalid pattern - unmatched parenthesis",
        "try {\n"
        "    new RegExp('(abc');\n"
        "} catch (e) {\n"
        "    'Error: ' + e.message;\n"
        "}");

    eval_and_print(ctx, "Invalid pattern - invalid escape",
        "try {\n"
        "    new RegExp('\\\\c');\n"
        "} catch (e) {\n"
        "    'Error: ' + e.message;\n"
        "}");

    eval_and_print(ctx, "Invalid pattern - bad quantifier",
        "try {\n"
        "    new RegExp('a{3,1}');\n"
        "} catch (e) {\n"
        "    'Error: ' + e.message;\n"
        "}");

    eval_and_print(ctx, "Unsupported sticky flag",
        "try {\n"
        "    new RegExp('abc', 'y');\n"
        "} catch (e) {\n"
        "    'Error: ' + e.message;\n"
        "}");
}

static void demo_backtracking_protection(TsRunContext* ctx) {
    printf("\n");
    printf("========================================\n");
    printf("7. Catastrophic Backtracking Protection\n");
    printf("========================================\n");

    printf("\nThe PCRE2 provider has a match limit to prevent\n");
    printf("catastrophic backtracking from hanging the program.\n");

    // This is a classic catastrophic backtracking pattern
    // (a+)+ on a string of 'a's followed by something that doesn't match
    eval_and_print(ctx, "Catastrophic pattern detection",
        "try {\n"
        "    // Pattern (a+)+ on 'aaaaaaaaaaaaaaaaaaaaaaaaaab'\n"
        "    // causes exponential backtracking\n"
        "    const evil = /(a+)+$/.test('aaaaaaaaaaaaaaaaaaaaaaaaaab');\n"
        "    'Should not reach here: ' + evil;\n"
        "} catch (e) {\n"
        "    'Protected: ' + e.message;\n"
        "}");

    eval_and_print(ctx, "Another backtracking bomb",
        "try {\n"
        "    // Alternation with overlapping patterns\n"
        "    /^(a|aa)+$/.test('aaaaaaaaaaaaaaaaaaaaab');\n"
        "} catch (e) {\n"
        "    'Protected: ' + e.message;\n"
        "}");

    printf("\nNote: Normal patterns work fine, only pathological\n");
    printf("patterns that would otherwise hang are caught.\n");

    eval_and_print(ctx, "Normal complex pattern works",
        "/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$/.test('user@example.com')");
}

static void demo_practical_examples(TsRunContext* ctx) {
    printf("\n");
    printf("========================================\n");
    printf("8. Practical Examples\n");
    printf("========================================\n");

    eval_and_print(ctx, "Email validation",
        "const emailRe = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$/;\n"
        "[\n"
        "    emailRe.test('user@example.com'),\n"
        "    emailRe.test('invalid-email'),\n"
        "    emailRe.test('user@sub.domain.org')\n"
        "]");

    eval_and_print(ctx, "Parse ISO date",
        "const dateStr = '2024-03-15T10:30:00Z';\n"
        "const m = dateStr.match(/(\\d{4})-(\\d{2})-(\\d{2})T(\\d{2}):(\\d{2}):(\\d{2})/);\n"
        "({ year: m[1], month: m[2], day: m[3], hour: m[4], min: m[5], sec: m[6] })");

    eval_and_print(ctx, "Slugify text",
        "'Hello World! This is a TEST'.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '')");

    eval_and_print(ctx, "Extract hashtags",
        "'Check out #typescript and #rust for #programming'.match(/#\\w+/g)");

    eval_and_print(ctx, "Mask credit card (keep last 4 digits)",
        "'4111-1111-1111-1111'.replace(/\\d(?=.{4,}$)/g, '*')");

    eval_and_print(ctx, "Validate password strength",
        "function checkPassword(pw: string): string {\n"
        "    if (pw.length < 8) return 'Too short';\n"
        "    if (!/[a-z]/.test(pw)) return 'Need lowercase';\n"
        "    if (!/[A-Z]/.test(pw)) return 'Need uppercase';\n"
        "    if (!/\\d/.test(pw)) return 'Need digit';\n"
        "    return 'Strong';\n"
        "}\n"
        "[checkPassword('weak'), checkPassword('Str0ngPass')]");
}

// ============================================================================
// Main
// ============================================================================

int main(void) {
    printf("tsrun C API - RegExp Example with PCRE2 Provider\n");
    printf("================================================\n\n");

    printf("This example demonstrates using a custom PCRE2-based\n");
    printf("RegExp provider with the tsrun interpreter.\n\n");

    printf("Features:\n");
    printf("  - PCRE2 regex engine with full pattern support\n");
    printf("  - UTF-8 enabled by default\n");
    printf("  - Configurable match limits for backtracking protection\n");
    printf("  - Supports flags: g (global), i (ignoreCase), m (multiline), s (dotAll)\n\n");

    // Create context
    TsRunContext* ctx = tsrun_new();
    if (!ctx) {
        fprintf(stderr, "Failed to create context\n");
        return 1;
    }

    // Set up console output
    tsrun_set_console(ctx, tsrun_console_stdio, NULL);

    // Configure and register PCRE2 provider
    TsRunPcre2Config config = {
        .match_limit = 100000  // Conservative limit for demo
    };
    TsRunRegexCallbacks callbacks = tsrun_pcre2_provider(&config);

    TsRunResult reg_result = tsrun_set_regexp_provider(ctx, &callbacks);
    if (!reg_result.ok) {
        fprintf(stderr, "Failed to set regexp provider: %s\n", reg_result.error);
        tsrun_free(ctx);
        return 1;
    }

    printf("PCRE2 provider registered with match_limit=%u\n", config.match_limit);

    // Run all demos
    demo_basic_matching(ctx);
    demo_capture_groups(ctx);
    demo_flags(ctx);
    demo_string_methods(ctx);
    demo_global_iteration(ctx);
    demo_error_handling(ctx);
    demo_backtracking_protection(ctx);
    demo_practical_examples(ctx);

    // Cleanup
    tsrun_free(ctx);

    printf("\n========================================\n");
    printf("Done!\n");
    return 0;
}
