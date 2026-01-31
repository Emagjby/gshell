#include <stdio.h>
#include <unistd.h>

#include "error.h"

void error_command_not_found(const char* command) {
    char buf[256];
    int len = snprintf(buf, sizeof(buf), "%s: command not found\n", command);
    write(2, buf, len);
}

void error_insufficient_arguments(const char* command) {
    char buf[256];
    int len = snprintf(buf, sizeof(buf), "%s: insufficient arguments provided\n", command);
    write(2, buf, len);
}

void error_generic(const char* message, const char* details) {
    char buf[512];
    int len = snprintf(buf, sizeof(buf), "%s\n\n\x1b[31m%s\x1b[0m\n", message, details);
    write(2, buf, len);
}

void error(ErrorType errorType, const char* details) {
    switch (errorType) {
        case ERROR_COMMAND_NOT_FOUND:
            error_command_not_found(details);
            break;
        case ERROR_INSUFFICIENT_ARGUMENTS:
            error_insufficient_arguments(details);
            break;
        default:
            error_generic("An unknown error occurred:", details);
            break;
    }
}

