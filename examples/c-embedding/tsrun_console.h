// tsrun_console.h - Default console implementation for tsrun
//
// Provides a ready-to-use console callback that writes to stdout/stderr.
// Include this header and link tsrun_console.c to use.
//
// Usage:
//   #include "tsrun.h"
//   #include "tsrun_console.h"
//
//   TsRunContext* ctx = tsrun_new();
//
//   // Option 1: Use default stdout/stderr
//   tsrun_set_console(ctx, tsrun_console_stdio, NULL);
//
//   // Option 2: Use custom streams
//   TsRunConsoleStreams streams = { my_log_file, my_err_file };
//   tsrun_set_console(ctx, tsrun_console_stdio, &streams);

#ifndef TSRUN_CONSOLE_H
#define TSRUN_CONSOLE_H

#include "tsrun.h"
#include <stdio.h>

#ifdef __cplusplus
extern "C" {
#endif

// Configuration for tsrun_console_stdio.
// If NULL is passed as userdata, defaults are used.
typedef struct {
    FILE* out;  // Stream for log/info/debug (default: stdout)
    FILE* err;  // Stream for warn/error (default: stderr)
} TsRunConsoleStreams;

// Console callback that writes to stdio streams.
//
// userdata can be:
// - NULL: uses stdout for log/info/debug, stderr for warn/error
// - TsRunConsoleStreams*: uses specified streams (NULL members use defaults)
//
// For TSRUN_CONSOLE_CLEAR, prints "--- Console cleared ---" to the out stream.
void tsrun_console_stdio(
    TsRunConsoleLevel level,
    const char* message,
    size_t message_len,
    void* userdata
);

#ifdef __cplusplus
}
#endif

#endif // TSRUN_CONSOLE_H
