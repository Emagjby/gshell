#ifndef COMMAND_H_
#define COMMAND_H_

#include "argvec.h"

typedef struct {
    ArgVec argv;
    char* stdout_path;
    char* stdout_append;
} Command;

void free_command(Command* command);

#endif
