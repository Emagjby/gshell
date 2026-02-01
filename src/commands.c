#include <unistd.h>
#include <string.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>

#include "fs.h"
#include "execute.h"
#include "helpers.h"
#include "commands.h"
#include "error.h"

void type_command(TokenArray* tokenArray) {
  // For now only supports a single argument
  if (tokenArray->count < 2) {
    error(ERROR_INSUFFICIENT_ARGUMENTS, tokenArray->tokens[0].value);
    return;
  }

  TokenType type = tokenArray->tokens[1].type;
  char* command = tokenArray->tokens[1].value;
  switch(type) {
    case TOKEN_COMMAND:      
      builtin_type(command);
      break;
    default:
      char* found = check_path_directories(command);
      if (found) {
        char* full_path = build_full_path(found, command);
        char buf[256];
        int len = snprintf(buf, sizeof(buf), "%s is %s\n", command, full_path);
        write(1, buf, len);
        free(full_path);
      } else {
        unknown_type(command);
      }
      break;
  }
}

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

void run_command(TokenArray* tokenArray, char* path) {
  int argCount = 0;
  char** args = decompose_args(*tokenArray, &argCount);

  char* full_path = build_full_path(path, tokenArray->tokens[0].value);

  run_program(full_path, args);
  free(full_path);
  free(args);
}

void pwd_command(void) {
  char cwd[PATH_MAX];
  getcwd(cwd, sizeof(cwd));
  write(1, cwd, strlen(cwd));
  write(1, "\n", 1);
}

void cd_command(TokenArray* tokenArray) {
  if (tokenArray->count < 2) {
    error(ERROR_INSUFFICIENT_ARGUMENTS, tokenArray->tokens[0].value);
    return;
  }

  char* path = tokenArray->tokens[1].value;
  if (chdir(path) != 0) {
    error(ERROR_CD_NO_SUCH_DIRECTORY, path);
  }
}
