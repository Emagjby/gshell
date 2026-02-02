#include <stdlib.h>
#include <string.h>

#include "argvec.h"

void double_argvec_capacity(ArgVec* argv) {
    argv->cap *= 2;
    argv->args = realloc(argv->args, sizeof(char*) * argv->cap);
}

void append_arg(ArgVec* argv, const char* arg) {
    if (argv->count >= argv->cap) {
        double_argvec_capacity(argv);
    }

    argv->args[argv->count++] = strcpy(malloc(strlen(arg) + 1), arg);
}

void append_arg_end(ArgVec* argv) {
    if (argv->count >= argv->cap) {
        double_argvec_capacity(argv);
    }

    argv->args[argv->count] = NULL;
}

void free_argvec(ArgVec* argv) {
    for (int i = 0; i < argv->count; i++) {
        free(argv->args[i]);
    }
    free(argv->args);
    argv->args = NULL;
    argv->count = 0;
    argv->cap = 0;
}
