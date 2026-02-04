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

  static struct {
    char* input;
    TokenArray tokenArray;
    Command command;
    int saved_stdout;
  } state;

  for(;;) {
    // prepare state
    state.input = NULL;
    state.tokenArray = (TokenArray){0};
    state.command = (Command){0};
    state.saved_stdout = -1;

    // set panic recovery point
    if(setjmp(panic_env) != 0) {
      goto cleanup;
    }

    write_prompt();

    // get input
    state.input = get_input();
    if(!state.input) {
      goto cleanup;
    }

    // process input
    state.tokenArray = tokenize(state.input);
    state.command = parse(state.tokenArray);

    // handle redirections
    if(state.command.stdout_path) {
      state.saved_stdout = redirect_stdout(state.command.stdout_path);
    }

    // execute command
    execute(&state.command);

cleanup:
    if(state.saved_stdout != -1) {
      restore_stdout(state.saved_stdout);
    }

    free(state.input);
    free_token_array(&state.tokenArray);
    free_command(&state.command);
  }

  clear_screen();
  return 0;
}
