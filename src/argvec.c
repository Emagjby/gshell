#include <stdlib.h>
#include <string.h>

#include "argvec.h"

void double_argvec_capacity(ArgVec* argv) {
    int new_cap = argv->cap ? argv->cap * 2 : 8;
    char** new_args = realloc(argv->args, sizeof(char*) * new_cap);
    if(!new_args) {
        abort(); // Handle memory allocation failure
    }
    argv->args = new_args;
    argv->cap = new_cap;
}

void append_arg(ArgVec* argv, const char* arg) {
    if (argv->count >= argv->cap) {
        double_argvec_capacity(argv);
    }

    char* copy = malloc(strlen(arg) + 1);
    if(!copy) {
        abort(); // Handle memory allocation failure
    }
    strcpy(copy, arg);
    argv->args[argv->count++] = copy;
}

void append_arg_end(ArgVec* argv) {
    if (argv->count >= argv->cap) {
        double_argvec_capacity(argv);
    }

    argv->args[argv->count] = NULL;
}

void free_argvec(ArgVec* argv) {
    for (size_t i = 0; i < argv->count; i++) {
        free(argv->args[i]);
    }
    free(argv->args);
    argv->args = NULL;
    argv->count = 0;
    argv->cap = 0;
}
