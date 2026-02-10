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
#include "pipeline.h"

int main(int argc, char *argv[]) {
  (void)argc;
  (void)argv;
  setbuf(stdout, NULL);

  static struct {
    char* input;
    TokenArray tokenArray;
    Pipeline pipeline;
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
    state.pipeline = (Pipeline){0};
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
    state.pipeline = parse(state.tokenArray);

    // execute command
    execute_pipeline(&state.pipeline);

cleanup:
    rehash_command_table();

    if(state.saved_stdout != -1) { restore_stdout(state.saved_stdout); }
    if(state.saved_stderr != -1) { restore_stderr(state.saved_stderr); }

    free(state.input);
    free_token_array(&state.tokenArray);
    free_pipeline(&state.pipeline);
  }

  clear_screen();
  return 0;
}
