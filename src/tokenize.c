#include <stdio.h>
#include <unistd.h>
#include <stdlib.h>
#include <string.h>

#include "tokenize.h"
#include "helpers.h"
#include "fs.h"

TokenType categorizeToken(const char* value) {
  if(value[0] == '\0'){
    return TOKEN_EMPTY;
  }
  if(is_builtin_command(value)){
    return TOKEN_COMMAND;
  }
  return TOKEN_ARGUMENT;
}

Token consumeToken(const char* input, int* index) {
  // Skip whitespaces
  while (input[*index] == ' ') {
    (*index)++;
  }

  int start = *index;
  while (input[*index] != ' ' && input[*index] != '\0') {
    (*index)++;
  }
  int length = *index - start;

  Token token;
  token.value = malloc(length + 1);

  strncpy(token.value, &input[start], length);
  token.value[length] = '\0';
  token.type = categorizeToken(token.value);

  return token;
}


void tokenize(char* input, TokenArray* tokenArray) {
  tokenArray->count = 0;
  tokenArray->cap = 8;
  tokenArray->tokens = malloc(sizeof(Token) * tokenArray->cap);

  int i = 0;
  while (input[i] != '\0') {
    Token consumed = consumeToken(input, &i);
    if(consumed.type == TOKEN_EMPTY) {
      continue;
    }
    
    if (tokenArray->count >= tokenArray->cap) {
      tokenArray->cap *= 2;
      tokenArray->tokens = realloc(tokenArray->tokens, sizeof(Token) * tokenArray->cap);
    }

    tokenArray->tokens[tokenArray->count++] = consumed;
  }
  free(input);
}

void freeTokenArray(TokenArray* tokenArray) {
  for (int i = 0; i < tokenArray->count; i++) {
    free(tokenArray->tokens[i].value);
  }
  free(tokenArray->tokens);
}
