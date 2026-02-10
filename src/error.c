#include <unistd.h>

#include "panic.h"
#include "error.h"
#include "dynbuf.h"

void error_command_not_found(const char* command) {
    DynBuf dynbuf;
    dynbuf_init(&dynbuf);

    dynbuf_append(&dynbuf, command);
    dynbuf_append(&dynbuf, ": command not found\n");

    write(2, dynbuf.buf, dynbuf.len);
    dynbuf_free(&dynbuf);
}

void error_insufficient_arguments(const char* command) {
    DynBuf dynbuf;
    dynbuf_init(&dynbuf);

    dynbuf_append(&dynbuf, command);
    dynbuf_append(&dynbuf, ": insufficient arguments provided\n");

    write(2, dynbuf.buf, dynbuf.len);

    dynbuf_free(&dynbuf);
}

void error_generic(const char* message, const char* details) {
    DynBuf dynbuf;
    dynbuf_init(&dynbuf);

    dynbuf_append(&dynbuf, message);
    dynbuf_append(&dynbuf, "\n\x1b[31m");
    dynbuf_append(&dynbuf, details);
    dynbuf_append(&dynbuf, "\x1b[0m\n\n");
    
    write(2, dynbuf.buf, dynbuf.len);

    dynbuf_free(&dynbuf);
}

void error_no_such_directory(const char* directory) {
    DynBuf dynbuf;
    dynbuf_init(&dynbuf);

    dynbuf_append(&dynbuf, "cd: ");
    dynbuf_append(&dynbuf, directory);
    dynbuf_append(&dynbuf, ": No such file or directory\n");

    write(2, dynbuf.buf, dynbuf.len);

    dynbuf_free(&dynbuf);
}

static void dispatch_error(ErrorType errorType, const char* details) {
    switch (errorType) {
        case ERROR_COMMAND_NOT_FOUND:
            error_command_not_found((char*)details);
            break;
        case ERROR_INSUFFICIENT_ARGUMENTS:
            error_insufficient_arguments(details);
            break;
        case ERROR_CD_NO_SUCH_DIRECTORY:
            error_no_such_directory((char*)details);
            break;
        case ERROR_PARSING_FAILED:
            error_generic("Parsing failed:", details);
            break;
        case ERROR_ENVIRONMENT_VARIABLE_NOT_SET:
            error_generic("Environment variable not set: ", details);
            break;
        case ERROR_UNTERMINATED_QUOTE:
            error_generic("Unterminated quote detected in input:", details);
            break;
        case ERROR_TOKENIZATION_FAILED:
            error_generic("Tokenization failed:", details);
            break;
        case ERROR_FILE_OPERATION_FAILED:
            error_generic("File operation failed:", details);
            break;
        case ERROR_EXECUTE_ERROR:
             error_generic("Execution error:", details);
             break;
        case ERROR_INVALID_ARGUMENT:
             error_generic("Invalid argument:", details);
             break;
        case ERROR_HISTORY_LOAD:
            error_generic("Failed to load history:", details);
            break;
        default:
            error_generic("An unknown error occurred:", details);
            break;
    }
}

void error_no_panic(ErrorType errorType, const char* details) {
    dispatch_error(errorType, details);
}

void error(ErrorType errorType, const char* details) {
    dispatch_error(errorType, details);
    panic();
}
