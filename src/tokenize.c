#include <stdio.h>
#include <unistd.h>
#include <stdlib.h>
#include <string.h>

#include "tokenize.h"

TokenType categorizeToken(const char* value, int* count) {
  if( strcmp(value, "exit") == 0 ||
      strcmp(value, "echo") == 0 ||
      strcmp(value, "clear") == 0 ||
      strcmp(value, "type") == 0) {
    return TOKEN_COMMAND;
  } 
  if (*count == 0) {
    return TOKEN_UNKNOWN;
  } 
  return TOKEN_ARGUMENT;
}

Token consumeToken(const char* input, int* index, int* count) {
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
  token.type = categorizeToken(token.value, count);

  return token;
}


void tokenize(char* input, TokenArray* tokenArray) {
  tokenArray->count = 0;
  tokenArray->cap = 8;
  tokenArray->tokens = malloc(sizeof(Token) * tokenArray->cap);

  int i = 0;
  while (input[i] != '\0') {
    Token consumed = consumeToken(input, &i, &tokenArray->count);
    
    if (tokenArray->count >= tokenArray->cap) {
      tokenArray->cap *= 2;
      tokenArray->tokens = realloc(tokenArray->tokens, sizeof(Token) * tokenArray->cap);
    }

    tokenArray->tokens[tokenArray->count++] = consumed;

    if (consumed.type == TOKEN_UNKNOWN) {
      // if unknown token, no need to continue tokenizing
      break;
    }
  }
  free(input);
}

void freeTokenArray(TokenArray* tokenArray) {
  for (int i = 0; i < tokenArray->count; i++) {
    free(tokenArray->tokens[i].value);
  }
  free(tokenArray->tokens);
}
