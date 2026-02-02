#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

// #include "tokenize.h"
// #include "execute.h"
#include "panic.h"
#include "helpers.h"
#include "tokenizer.h"
#include "parser.h"
// #include "fs.h"

int main(int argc, char *argv[]) {
  (void)argc;
  (void)argv;
  setbuf(stdout, NULL);

  clear_screen();
  for(;;) {
    if(setjmp(panic_env)) { continue; } // recover from panic

    // write prompt
    write_prompt();

    // Read user input 
    char* input = get_input();
    if (input == NULL) {
      continue;
    }

    // Tokenize
    TokenArray tokenArray = tokenize(input);

    // Parse
    ArgVec argVec = parse(tokenArray);

    for(int i = 0; i < argVec.count; i++) {
      printf("arg[%d]: %s\n", i, argVec.args[i]);
    }

    // // Execute
    // execute(&tokenArray);

    // Free resources
    free(input);
  }

  clear_screen();
  return 0;
}
