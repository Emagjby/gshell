#ifndef ARGVEC_H_
#define ARGVEC_H_

typedef struct {
    char** args;
    int count;
    int cap;
} ArgVec;

void double_argvec_capacity(ArgVec* argv);
void append_arg(ArgVec* argv, const char* arg);
void append_arg_end(ArgVec* argv);

#endif
