// tsrun_console.c - Default console implementation for tsrun

#include "tsrun_console.h"
#include <stdio.h>

void tsrun_console_stdio(
    TsRunConsoleLevel level,
    const char* message,
    size_t message_len,
    void* userdata
) {
    // Determine output streams
    FILE* out_stream = stdout;
    FILE* err_stream = stderr;

    if (userdata != NULL) {
        TsRunConsoleStreams* streams = (TsRunConsoleStreams*)userdata;
        if (streams->out != NULL) {
            out_stream = streams->out;
        }
        if (streams->err != NULL) {
            err_stream = streams->err;
        }
    }

    // Select stream based on level
    FILE* stream;
    switch (level) {
        case TSRUN_CONSOLE_LOG:
        case TSRUN_CONSOLE_INFO:
        case TSRUN_CONSOLE_DEBUG:
            stream = out_stream;
            break;
        case TSRUN_CONSOLE_WARN:
        case TSRUN_CONSOLE_ERROR:
            stream = err_stream;
            break;
        case TSRUN_CONSOLE_CLEAR:
            fprintf(out_stream, "--- Console cleared ---\n");
            fflush(out_stream);
            return;
        default:
            stream = out_stream;
            break;
    }

    // Write message (message_len bytes, not null-terminated)
    if (message_len > 0) {
        fwrite(message, 1, message_len, stream);
    }
    fputc('\n', stream);
    fflush(stream);
}
