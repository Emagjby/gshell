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

    // write prompt
    write_prompt();

    // Read user input 
    char* input = get_input();
    if (input == NULL) {
      continue;
    }

    // Tokenize
    TokenArray tokenArray = tokenize(input);

    for(int i = 0; i < tokenArray.count; i++) {
      // Uncomment the following line to debug tokens
      printf("Token %d: Type=%d, Value='%s'\n", i, tokenArray.tokens[i].type, tokenArray.tokens[i].value ? tokenArray.tokens[i].value : "NULL");
    }

    // Parse
    Command command = parse(tokenArray);

    // Execute
    execute(&command);

    // Free resources
    if(setjmp(panic_env)) { continue; } // recover from panic
    free(input);
    free_command(&command);
  }

  clear_screen();
  return 0;
}
