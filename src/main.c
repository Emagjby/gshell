#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

#include "fs.h"
#include "linenoise_setup.h"
#include "execute.h"
#include "panic.h"
#include "helpers.h"
#include "tokenizer.h"
#include "parser.h"
#include "rehash.h"

int main(int argc, char *argv[]) {
  (void)argc;
  (void)argv;
  setbuf(stdout, NULL);

  static struct {
    char* input;
    TokenArray tokenArray;
    Command command;
    int saved_stdout;
    int saved_stderr;
    char* prompt;
  } state;
  repl_linenoise_init();

  clear_screen();

  rehash_command_table();
  for(;;) {
    // prepare state
    state.input = NULL;
    state.tokenArray = (TokenArray){0};
    state.command = (Command){0};
    state.saved_stdout = -1;
    state.saved_stderr = -1;

    // TODO: support custom prompt
    state.prompt = "$ ";

    // set panic recovery point
    if(setjmp(panic_env) != 0) {
      goto cleanup;
    }

    // get input
    state.input = repl_readline(state.prompt);
    if(!state.input) {
      goto cleanup;
    }

    // process input
    state.tokenArray = tokenize(state.input);
    state.command = parse(state.tokenArray);

    // handle redirections
    if(state.command.stdout_path) {
      state.saved_stdout = redirect_stdout(state.command.stdout_path, REDIRECT_OUT);
    }

    if(state.command.stderr_path) {
      state.saved_stderr = redirect_stderr(state.command.stderr_path, REDIRECT_OUT);
    }

    if(state.command.stdout_append) {
      state.saved_stdout = redirect_stdout(state.command.stdout_append, REDIRECT_APPEND);
    }

    if(state.command.stderr_append) {
      state.saved_stderr = redirect_stderr(state.command.stderr_append, REDIRECT_APPEND);
    }

    // execute command
    execute(&state.command);

cleanup:
    rehash_command_table();

    if(state.saved_stdout != -1) { restore_stdout(state.saved_stdout); }
    if(state.saved_stderr != -1) { restore_stderr(state.saved_stderr); }

    free(state.input);
    free_token_array(&state.tokenArray);
    free_command(&state.command);
  }

  clear_screen();
  return 0;
}
