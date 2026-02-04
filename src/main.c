#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

#include "fs.h"
#include "error.h"
#include "execute.h"
#include "panic.h"
#include "helpers.h"
#include "tokenizer.h"
#include "parser.h"

int main(int argc, char *argv[]) {
  (void)argc;
  (void)argv;
  setbuf(stdout, NULL);

  clear_screen();
  for(;;) {
    // prepare state
    char* input = NULL;
    TokenArray tokenArray = {0};
    Command command = {0};

    // set panic recovery point
    if(setjmp(panic_env) != 0) {
      goto cleanup;
    }

    write_prompt();

    // get input
    input = get_input();
    if(!input) {
      goto cleanup;
    }

    // process input
    tokenArray = tokenize(input);
    command = parse(tokenArray);

    // handle redirection 
    int saved_stdout = -1;

    if(command.stdout_path) {
      saved_stdout = dup(STDOUT_FILENO);
      if(saved_stdout < 0){
        error(ERROR_FILE_OPERATION_FAILED, "Failed to save stdout");
      }

      redirect_stdout(command.stdout_path);
    }

    // execute command
    execute(&command);

cleanup:
    if(saved_stdout != -1) {
      restore_stdout(saved_stdout);
    }

    free(input);
    free_token_array(&tokenArray);
    free_command(&command);
  }

  clear_screen();
  return 0;
}
