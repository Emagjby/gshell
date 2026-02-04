#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

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

    // process input & execute
    tokenArray = tokenize(input);
    for(int i = 0; i < tokenArray.count; i++) {
      printf("Token %d: Type %d, Value '%s'\n", i, tokenArray.tokens[i].type, tokenArray.tokens[i].value ? tokenArray.tokens[i].value : "NULL");
    }
    command = parse(tokenArray);
    execute(&command);

cleanup:
    free(input);
    free_token_array(&tokenArray);
    free_command(&command);
  }

  clear_screen();
  return 0;
}
