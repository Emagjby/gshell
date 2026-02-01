#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

#include "tokenize.h"
#include "execute.h"
#include "helpers.h"

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
    
    // Tokenize
    TokenArray tokenArray;
    tokenize(input, &tokenArray);

    // Execute
    execute(&tokenArray);

    // Free resources
    freeTokenArray(&tokenArray);
  }

  clear_screen();
  return 0;
}
