#include <string.h>
#include <stdlib.h>
#include <unistd.h>

#include "error.h"
#include "fs.h"
#include "commands.h"

void execute(ArgVec argv) {
  if(argv.count == 0 || argv.args == NULL || argv.args[0] == NULL) {
    free_argvec(&argv);
    return;
  }
  char* toExec = argv.args[0]; 

  if(strcmp(toExec, "exit") == 0) {
    exit(0);
  } else if (strcmp(toExec, "echo") == 0) {
    echo_command(argv);
  } else if (strcmp(toExec, "clear") == 0) {
    clear_command();
    free_argvec(&argv);
    return;
  } else if (strcmp(toExec, "type") == 0) {
    type_command(argv);
  } else if (strcmp(toExec, "pwd") == 0) {
    pwd_command();
    free_argvec(&argv);
    return;
  } else if (strcmp(toExec, "cd") == 0) {
    cd_command(argv);
  } else {
    char* found = check_path_directories(toExec);

    if (found) {
      run_command(argv, found);
      free(found);
      return;
    }

    error(ERROR_COMMAND_NOT_FOUND, argv.args[0]);
  }
}
