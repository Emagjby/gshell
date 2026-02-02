#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include <string.h>

#include "tokenizer.h"
#include "error.h"

void free_token_array(TokenArray* tokenArray) {
  for (int i = 0; i < tokenArray->count; i++) {
    free(tokenArray->tokens[i].value);
  }
  free(tokenArray->tokens);
}

void double_token_array_capacity(TokenArray* tokenArray) {
  tokenArray->cap *= 2;
  tokenArray->tokens = realloc(tokenArray->tokens, sizeof(Token) * tokenArray->cap);
}

void append_token(TokenArray* tokenArray, Token token) {
  if (tokenArray->count >= tokenArray->cap) {
    double_token_array_capacity(tokenArray);
  }
  tokenArray->tokens[tokenArray->count++] = token;
}

TokenType categorizeToken(const char* value) {
  return TOKEN_TEXT;
}

TokenArray tokenize(const char* input) {
  int index = 0;
  while(input[index] == ' ') {
    index++;
  } // skip leading whitespace
  int start = index;

  TokenArray tokenArray;
  tokenArray.count = 0;
  tokenArray.cap = 8;
  tokenArray.tokens = malloc(sizeof(Token) * tokenArray.cap);

  for(;input[index] != '\0'; index++) {
    if(input[index] == '\'') {
      index++;
      start = index;
      while(input[index] != '\'' && input[index] != '\0') {
        index++;
      } // go to final ''\'
      if(input[index] == '\0') {
        error(ERROR_UNTERMINATED_QUOTE, "Single quote not terminated");
      }

      // determine length
      int length = index - start;

      // build token
      Token token;
      token.value = malloc(length + 1);
      strncpy(token.value, &input[start], length);
      token.value[length] = '\0';
      token.type = categorizeToken(token.value);

      // append token
      append_token(&tokenArray, token);      

      start = index + 1;
      continue;
    } else if (input[index] == ' ') {
      if (start != index) {
        // build  prev token
      }
      while(input[index + 1] == ' ') {
        index++;
      } // consume whitespaces
      
      // build whitespace token
      Token token;
      token.value = NULL;
      token.type = TOKEN_WHITESPACE;

      // append token
      append_token(&tokenArray, token);

      start = index + 1;
      continue; 
    } else {
      while(input[index + 1] != ' ' 
            && input[index + 1] != '\0'
            && input[index + 1] != '\'') {
        index++;
      } // go to end of token
      
      // determine length
      int length = index - start + 1;

      // build token
      Token token;
      token.value = malloc(length + 1);
      strncpy(token.value, &input[start], length);
      token.value[length] = '\0';
      token.type = categorizeToken(token.value);

      // append token
      append_token(&tokenArray, token);

      start = index + 1;
      continue;
    }

    error(ERROR_TOKENIZATION_FAILED, "An unknown tokenization error occurred");
  }

  return tokenArray;
}
