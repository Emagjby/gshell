#ifndef PIPELINE_H_
#define PIPELINE_H_

#include <stdlib.h>

#include "command.h"

typedef struct {
    Command** commands; 
    size_t count;
} Pipeline;

void append_command(Pipeline* pipeline, Command command);
void free_pipeline(Pipeline* pipeline);

#endif 
