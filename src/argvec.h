#ifndef ARGVEC_H_
#define ARGVEC_H_

#include <stdlib.h>

typedef struct {
    char** args;
    size_t count;
    size_t cap;
} ArgVec;

void double_argvec_capacity(ArgVec* argv);
void append_arg(ArgVec* argv, const char* arg);
void append_arg_end(ArgVec* argv);
void free_argvec(ArgVec* argv);

#endif
