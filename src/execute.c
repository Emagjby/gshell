#include <string.h>
#include <stdlib.h>
#include <unistd.h>

#include "error.h"
#include "tokenize.h"
#include "fs.h"
#include "commands.h"

void execute(TokenArray* tokenArray) {
  char* toExec = tokenArray->tokens[0].value;

  if(strcmp(toExec, "exit") == 0) {
    write(1, "\n", 1);
    exit(0);
  } else if (strcmp(toExec, "echo") == 0) {
    echo_command(tokenArray);
  } else if (strcmp(toExec, "clear") == 0) {
    clear_command();
  } else if (strcmp(toExec, "type") == 0) {
    type_command(tokenArray);
  } else if (strcmp(toExec, "pwd") == 0) {
    pwd_command();
  } else if (strcmp(toExec, "cd") == 0) {
    cd_command(tokenArray);
  } else {
    char* found = check_path_directories(toExec);

    if (found) {
      run_command(tokenArray, found);
      free(found);
      return;
    }

    error(ERROR_COMMAND_NOT_FOUND, tokenArray->tokens[0].value);
  }
}
