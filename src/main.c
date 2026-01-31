#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

#include "tokenize.h"
#include "execute.h"

void clear_screen() {
  write(1, "\033[2J\033[H", 7);
}

int main(int argc, char *argv[]) {
  (void)argc;
  (void)argv;
  setbuf(stdout, NULL);

  clear_screen();
  for(;;) {
    // Display prompt
    char prompt[2] = {'$', ' '};
    write(1, prompt, sizeof(prompt));

    // Read user input & remove \n
    char *command = NULL;
    size_t cap = 0;

    ssize_t read = getline(&command, &cap, stdin);
    if (read > 0 && command[read - 1] == '\n') {
      command[read - 1] = '\0';
      read--;
    }

    // Tokenize
    TokenArray tokenArray;
    tokenize(command, &tokenArray);

    // Execute
    execute(&tokenArray);
  }

  clear_screen();
  return 0;
}
