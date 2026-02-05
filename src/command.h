#ifndef COMMAND_H_
#define COMMAND_H_

#include "argvec.h"

typedef struct {
    ArgVec argv;
    char* stdout_path;
    char* stderr_path;
    char* stdout_append;
    char* stderr_append;
} Command;

void free_command(Command* command);

#endif
