#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

// #include "tokenize.h"
// #include "execute.h"
#include "helpers.h"
#include "tokenizer.h"
// #include "fs.h"

int main(int argc, char *argv[]) {
  (void)argc;
  (void)argv;
  setbuf(stdout, NULL);

  clear_screen();
  for(;;) {
    write_prompt();

    // Read user input 
    char* input = get_input();
    if (input == NULL) {
      continue;
    }

    TokenArray tokenArray = tokenize(input);

    // // Execute
    // execute(&tokenArray);

    // Free resources
    free_token_array(&tokenArray);
    free(input);
  }

  clear_screen();
  return 0;
}
