#include <string.h>
#include <stdlib.h>
#include <unistd.h>

#include "command.h"
#include "error.h"
#include "fs.h"
#include "commands.h"

void execute(Command* command) {
  ArgVec argv = command->argv;
  if(argv.count == 0 || argv.args == NULL || argv.args[0] == NULL) {
    return;
  }
  char* toExec = argv.args[0]; 

  if(strcmp(toExec, "exit") == 0) {
    exit(0);
  } else if (strcmp(toExec, "echo") == 0) {
    echo_command(argv);
  } else if (strcmp(toExec, "clear") == 0) {
    clear_command();
    return;
  } else if (strcmp(toExec, "type") == 0) {
    type_command(argv);
  } else if (strcmp(toExec, "pwd") == 0) {
    pwd_command();
  } else if (strcmp(toExec, "cd") == 0) {
    cd_command(argv);
  } else {
    char* found = check_path_directories(toExec);

    if (found) {
      run_command(argv, found);
      free(found);
    } else {
      error(ERROR_COMMAND_NOT_FOUND, *argv.args);
    }
  }
}
