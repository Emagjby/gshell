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
#include "history.h"

int main(int argc, char *argv[]) {
  (void)argc;
  (void)argv;
  setbuf(stdout, NULL);

  static struct {
    char* input;
    TokenArray tokenArray;
    Pipeline pipeline;
    char* prompt;
    int saved_stdout;
    int saved_stderr;
  } state;
  repl_linenoise_init();
  history_init();

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

    history_add(state.input);

    // process input
    state.tokenArray = tokenize(state.input);
    state.pipeline = parse(state.tokenArray);

    // handle redirections and execute
    state.saved_stdout = save_stdout();
    if(state.saved_stdout < 0) {
      state.saved_stdout = -1;
    }
    state.saved_stderr = save_stderr();
    if(state.saved_stderr < 0) {
      state.saved_stderr = -1;
    }

    execute_pipeline(&state.pipeline);

cleanup:
    if(state.saved_stdout != -1) {
      restore_stdout(state.saved_stdout);
      state.saved_stdout = -1;
    }
    if(state.saved_stderr != -1) {
      restore_stderr(state.saved_stderr);
      state.saved_stderr = -1;
    }
    rehash_command_table();

    free(state.input);
    free_token_array(&state.tokenArray);
    free_pipeline(&state.pipeline);
  }

  clear_screen();
  history_save();
  return 0;
}
