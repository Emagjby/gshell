#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>
#include "execute.h"

void execute(TokenArray* tokenArray) {
  char* toExec = tokenArray->tokens[0].value;
  if(strcmp(toExec, "exit") == 0) {
    // No need to free memory here as the program is exiting
    write(1, "\n", 1);
    exit(0);
  }

  char buf[128];
  int len = snprintf(buf, sizeof(buf), "%s: command not found\n", tokenArray->tokens[0].value);
  write(1, buf, len);

  for(int i = 0; i < tokenArray->count; i++) {
    free(tokenArray->tokens[i].value);
  }
  free(tokenArray->tokens);
}
