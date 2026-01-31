#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>

#include "error.h"
#include "execute.h"

void clear_command() {
  write(1, "\x1b[H\x1b[2J", 7);
}

void echo_command(TokenArray* tokenArray) {
  for(int i = 1; i < tokenArray->count; i++) {
    write(1, tokenArray->tokens[i].value, strlen(tokenArray->tokens[i].value));
    if (i < tokenArray->count - 1) {
      write(1, " ", 1);
    }
  }
  write(1, "\n", 1);
}

void execute(TokenArray* tokenArray) {
  char* toExec = tokenArray->tokens[0].value;
  char* args[ tokenArray->count ];

  if(strcmp(toExec, "exit") == 0) {
    write(1, "\n", 1);
    exit(0);
  } else if (strcmp(toExec, "echo") == 0) {
    echo_command(tokenArray);
  } else if (strcmp(toExec, "clear") == 0) {
    clear_command();
  } else {
    error(ERROR_COMMAND_NOT_FOUND, tokenArray->tokens[0].value);
  }
}
