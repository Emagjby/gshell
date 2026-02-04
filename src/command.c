#include <stdlib.h>

#include "command.h"

void free_command(Command* command) {
    if(!command) {
        return;
    }
    free_argvec(&command->argv);
    if (command->stdout_path) {
        free(command->stdout_path);
        command->stdout_path = NULL;
    }
    if (command->stdout_append) {
        free(command->stdout_append);
        command->stdout_append = NULL;
    }
}
