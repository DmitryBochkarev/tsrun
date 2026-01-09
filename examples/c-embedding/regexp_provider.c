// regexp_provider.c - PCRE2-based RegExp provider implementation
//
// Implements the TsRunRegexCallbacks interface using PCRE2.

#define PCRE2_CODE_UNIT_WIDTH 8
#include <pcre2.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "regexp_provider.h"

// ============================================================================
// Internal State
// ============================================================================

// Global configuration (set by tsrun_pcre2_provider)
static uint32_t g_match_limit = TSRUN_PCRE2_DEFAULT_MATCH_LIMIT;

// Thread-local error buffer for returning error messages
static __thread char g_error_buffer[256];

// ============================================================================
// Helper Functions
// ============================================================================

// Format a PCRE2 error into the error buffer
static const char* format_pcre2_error(int errorcode) {
    pcre2_get_error_message(errorcode, (PCRE2_UCHAR*)g_error_buffer, sizeof(g_error_buffer));
    return g_error_buffer;
}

// Parse JS regex flags into PCRE2 options
// Returns 0 on success, -1 on error (unsupported flag)
static int parse_flags(const char* flags, uint32_t* options_out, int* is_global_out) {
    *options_out = PCRE2_UTF;  // Always enable UTF-8
    *is_global_out = 0;

    if (!flags) return 0;

    for (const char* p = flags; *p; p++) {
        switch (*p) {
            case 'i':
                *options_out |= PCRE2_CASELESS;
                break;
            case 'm':
                *options_out |= PCRE2_MULTILINE;
                break;
            case 's':
                *options_out |= PCRE2_DOTALL;
                break;
            case 'g':
                *is_global_out = 1;
                break;
            case 'u':
                // UTF-8 is already enabled by default
                break;
            case 'y':
                // Sticky flag not supported
                snprintf(g_error_buffer, sizeof(g_error_buffer),
                         "sticky flag (y) is not supported by PCRE2 provider");
                return -1;
            default:
                snprintf(g_error_buffer, sizeof(g_error_buffer),
                         "unknown regex flag: %c", *p);
                return -1;
        }
    }

    return 0;
}

// ============================================================================
// Compiled Regex Handle
// ============================================================================

// Our compiled regex handle wraps PCRE2 code plus metadata
typedef struct {
    pcre2_code* code;
    pcre2_match_context* match_ctx;
    int is_global;
    uint32_t capture_count;
} CompiledRegex;

// ============================================================================
// Callback Implementations
// ============================================================================

// Compile a regex pattern
static void* pcre2_compile_fn(
    void* userdata,
    const char* pattern,
    const char* flags,
    const char** error_out
) {
    (void)userdata;

    uint32_t options;
    int is_global;

    if (parse_flags(flags, &options, &is_global) < 0) {
        *error_out = g_error_buffer;
        return NULL;
    }

    int errorcode;
    PCRE2_SIZE erroroffset;

    pcre2_code* code = pcre2_compile(
        (PCRE2_SPTR)pattern,
        PCRE2_ZERO_TERMINATED,
        options,
        &errorcode,
        &erroroffset,
        NULL  // compile context
    );

    if (!code) {
        // Format error with offset info
        char err_msg[200];
        pcre2_get_error_message(errorcode, (PCRE2_UCHAR*)err_msg, sizeof(err_msg));
        snprintf(g_error_buffer, sizeof(g_error_buffer),
                 "regex compile error at offset %zu: %s", (size_t)erroroffset, err_msg);
        *error_out = g_error_buffer;
        return NULL;
    }

    // Get capture count
    uint32_t capture_count;
    pcre2_pattern_info(code, PCRE2_INFO_CAPTURECOUNT, &capture_count);

    // Create match context with limit
    pcre2_match_context* match_ctx = pcre2_match_context_create(NULL);
    if (match_ctx && g_match_limit > 0) {
        pcre2_set_match_limit(match_ctx, g_match_limit);
    }

    // Allocate and populate handle
    CompiledRegex* handle = malloc(sizeof(CompiledRegex));
    if (!handle) {
        pcre2_code_free(code);
        if (match_ctx) pcre2_match_context_free(match_ctx);
        *error_out = "out of memory";
        return NULL;
    }

    handle->code = code;
    handle->match_ctx = match_ctx;
    handle->is_global = is_global;
    handle->capture_count = capture_count;

    return handle;
}

// Test if regex matches (boolean result)
static int pcre2_is_match_fn(
    void* userdata,
    void* handle,
    const char* input,
    size_t input_len,
    const char** error_out
) {
    (void)userdata;

    CompiledRegex* re = (CompiledRegex*)handle;

    pcre2_match_data* match_data = pcre2_match_data_create_from_pattern(re->code, NULL);
    if (!match_data) {
        *error_out = "out of memory";
        return -1;
    }

    int rc = pcre2_match(
        re->code,
        (PCRE2_SPTR)input,
        input_len,
        0,              // start offset
        0,              // options
        match_data,
        re->match_ctx
    );

    pcre2_match_data_free(match_data);

    if (rc >= 0) {
        return 1;  // Match found
    } else if (rc == PCRE2_ERROR_NOMATCH) {
        return 0;  // No match
    } else if (rc == PCRE2_ERROR_MATCHLIMIT) {
        snprintf(g_error_buffer, sizeof(g_error_buffer),
                 "regex match limit exceeded (possible catastrophic backtracking)");
        *error_out = g_error_buffer;
        return -1;
    } else {
        *error_out = format_pcre2_error(rc);
        return -1;
    }
}

// Find first match at position with captures
static int pcre2_find_fn(
    void* userdata,
    void* handle,
    const char* input,
    size_t input_len,
    size_t start_pos,
    TsRunRegexMatch* match_out,
    const char** error_out
) {
    (void)userdata;

    CompiledRegex* re = (CompiledRegex*)handle;

    pcre2_match_data* match_data = pcre2_match_data_create_from_pattern(re->code, NULL);
    if (!match_data) {
        *error_out = "out of memory";
        return -1;
    }

    int rc = pcre2_match(
        re->code,
        (PCRE2_SPTR)input,
        input_len,
        start_pos,
        0,              // options
        match_data,
        re->match_ctx
    );

    if (rc == PCRE2_ERROR_NOMATCH) {
        pcre2_match_data_free(match_data);
        return 0;  // No match
    } else if (rc < 0) {
        pcre2_match_data_free(match_data);
        if (rc == PCRE2_ERROR_MATCHLIMIT) {
            snprintf(g_error_buffer, sizeof(g_error_buffer),
                     "regex match limit exceeded (possible catastrophic backtracking)");
            *error_out = g_error_buffer;
        } else {
            *error_out = format_pcre2_error(rc);
        }
        return -1;
    }

    // Get the ovector (array of start/end pairs)
    PCRE2_SIZE* ovector = pcre2_get_ovector_pointer(match_data);
    uint32_t ovector_count = pcre2_get_ovector_count(match_data);

    // Fill in match result
    match_out->start = ovector[0];
    match_out->end = ovector[1];

    // Allocate captures array (includes group 0 = full match)
    if (ovector_count > 0) {
        match_out->capture_count = ovector_count;
        match_out->captures = malloc(ovector_count * sizeof(TsRunRegexCapture));

        if (!match_out->captures) {
            pcre2_match_data_free(match_data);
            *error_out = "out of memory";
            return -1;
        }

        for (uint32_t i = 0; i < ovector_count; i++) {
            PCRE2_SIZE start = ovector[2 * i];
            PCRE2_SIZE end = ovector[2 * i + 1];

            // PCRE2 uses PCRE2_UNSET for non-participating groups
            if (start == PCRE2_UNSET) {
                match_out->captures[i].start = -1;
                match_out->captures[i].end = -1;
            } else {
                match_out->captures[i].start = (intptr_t)start;
                match_out->captures[i].end = (intptr_t)end;
            }
        }
    } else {
        match_out->captures = NULL;
        match_out->capture_count = 0;
    }

    pcre2_match_data_free(match_data);
    return 1;  // Match found
}

// Free a compiled regex
static void pcre2_free_fn(void* userdata, void* handle) {
    (void)userdata;

    CompiledRegex* re = (CompiledRegex*)handle;
    if (re) {
        if (re->code) pcre2_code_free(re->code);
        if (re->match_ctx) pcre2_match_context_free(re->match_ctx);
        free(re);
    }
}

// Free captures array
static void pcre2_free_captures_fn(
    void* userdata,
    TsRunRegexCapture* captures,
    size_t count
) {
    (void)userdata;
    (void)count;

    free(captures);
}

// ============================================================================
// Public API
// ============================================================================

TsRunRegexCallbacks tsrun_pcre2_provider(const TsRunPcre2Config* config) {
    // Apply configuration
    if (config && config->match_limit > 0) {
        g_match_limit = config->match_limit;
    } else {
        g_match_limit = TSRUN_PCRE2_DEFAULT_MATCH_LIMIT;
    }

    TsRunRegexCallbacks callbacks = {
        .compile = pcre2_compile_fn,
        .is_match = pcre2_is_match_fn,
        .find = pcre2_find_fn,
        .free = pcre2_free_fn,
        .free_captures = pcre2_free_captures_fn,
        .userdata = NULL
    };

    return callbacks;
}
