// regexp_provider.h - PCRE2-based RegExp provider for tsrun
//
// This header provides a custom RegExp implementation using PCRE2.
// It implements all the callbacks required by tsrun_set_regexp_provider().
//
// Features:
// - Full PCRE2 regex syntax
// - UTF-8 support enabled by default
// - Capture groups with proper indexing
// - Configurable match limits for backtracking protection
// - Support for flags: i (ignoreCase), m (multiline), s (dotAll), g (global)
//
// Limitations:
// - Sticky flag (y) is not supported and will return an error
//
// Usage:
//   TsRunPcre2Config config = { .match_limit = 500000 };
//   TsRunRegexCallbacks callbacks = tsrun_pcre2_provider(&config);
//   tsrun_set_regexp_provider(ctx, &callbacks);
//
// Prerequisites:
//   - PCRE2 library (libpcre2-8)
//   - Link with -lpcre2-8

#ifndef REGEXP_PROVIDER_H
#define REGEXP_PROVIDER_H

#include "tsrun.h"

#ifdef __cplusplus
extern "C" {
#endif

// Configuration for the PCRE2 provider
typedef struct {
    // Maximum number of match attempts before aborting (backtracking protection).
    // Set to 0 to use the default (1,000,000).
    // PCRE2's built-in default is 10,000,000.
    uint32_t match_limit;
} TsRunPcre2Config;

// Create a PCRE2-based regex provider.
//
// Parameters:
//   config - Configuration options, or NULL for defaults
//
// Returns:
//   TsRunRegexCallbacks struct ready to pass to tsrun_set_regexp_provider()
//
// The returned callbacks use static storage for userdata, so only one
// PCRE2 provider configuration can be active at a time per process.
// If you need multiple configurations, create separate contexts.
TsRunRegexCallbacks tsrun_pcre2_provider(const TsRunPcre2Config* config);

// Default match limit (1,000,000)
#define TSRUN_PCRE2_DEFAULT_MATCH_LIMIT 1000000

#ifdef __cplusplus
}
#endif

#endif // REGEXP_PROVIDER_H
