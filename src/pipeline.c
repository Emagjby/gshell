#include "pipeline.h"

void append_command(Pipeline* pipeline, Command command) {
    // Pipeline can have preexisting commands, so we need to append to the array
    size_t new_count = pipeline->count + 1;
    Command** new_commands = realloc(pipeline->commands, new_count * sizeof(Command*));
    if (!new_commands) {
        abort(); // Handle memory allocation failure
    }

    // Allocate memory for the new command and copy the command data
    Command* new_command = malloc(sizeof(Command));
    if (!new_command) {
        abort(); // Handle memory allocation failure
    }
    *new_command = command; 
    
    new_commands[pipeline->count] = new_command; 
    pipeline->commands = new_commands; 
    pipeline->count = new_count; 
}

void free_pipeline(Pipeline* pipeline) {
    if (!pipeline) {
        return;
    }

    for (size_t i = 0; i < pipeline->count; i++) {
        if (!pipeline->commands[i]) {
            continue;
        }
        free_command(pipeline->commands[i]);
        free(pipeline->commands[i]);
        pipeline->commands[i] = NULL;
    }

    free(pipeline->commands);
    pipeline->commands = NULL;
    pipeline->count = 0;
}
