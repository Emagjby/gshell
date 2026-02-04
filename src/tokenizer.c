#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include <string.h>

#include "tokenizer.h"
#include "dynbuf.h"
#include "error.h"

void free_token_array(TokenArray* tokenArray) {
  for (int i = 0; i < tokenArray->count; i++) {
    free(tokenArray->tokens[i].value);
  }
  free(tokenArray->tokens);
}

void double_token_array_capacity(TokenArray* tokenArray) {
  int new_cap = tokenArray->cap * 2;
  Token* new_tokens = realloc(tokenArray->tokens, sizeof(Token) * new_cap);
  if(!new_tokens) {
    abort(); // Handle memory allocation failure
  }
  tokenArray->tokens = new_tokens;
  tokenArray->cap = new_cap;
}

void append_token(TokenArray* tokenArray, Token token) {
  if (tokenArray->count >= tokenArray->cap) {
    double_token_array_capacity(tokenArray);
  }
  tokenArray->tokens[tokenArray->count++] = token;
}

char* postprocess_dq(char* arg) {
  int read_index = 0;

  DynBuf dynbuf;
  dynbuf_init(&dynbuf);

  while(arg[read_index] != '\0') {
    if(arg[read_index] == '\\') {
      read_index++;
      switch(arg[read_index]) {
        case '"':
          dynbuf_append(&dynbuf, "\"");
          break;
        case '\\':
          dynbuf_append(&dynbuf, "\\");
          break;
        default: {
          // unrecognized escape, treat literally
          char temp[3] = {'\\', arg[read_index], '\0'};
          dynbuf_append(&dynbuf, temp);
          break;
        }
      }
    } else {
      char temp[2] = {arg[read_index], '\0'};
      dynbuf_append(&dynbuf, temp);
    }
    read_index++;
  }

  // allocate processed string
  char* processed = malloc(dynbuf.len + 1);
  if(!processed) {
    abort(); // Handle memory allocation failure
  }
  memcpy(processed, dynbuf.buf, dynbuf.len);
  processed[dynbuf.len] = '\0';

  // free dynamic buffer and original arg
  dynbuf_free(&dynbuf);
  free(arg);

  return processed;
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
  if(!tokenArray.tokens) {
    abort(); // Handle memory allocation failure
  }

  for(;input[index] != '\0'; index++) {
    // first check for special tokens
    if(input[index] == '>') {
      // build redirect token
      Token token;
      token.value = malloc(2);
      if(!token.value) {
        abort(); // Handle memory allocation failure
      }
      token.value[0] = '>';
      token.value[1] = '\0';
      token.type = TOKEN_REDIRECT_OUT;

      // append token
      append_token(&tokenArray, token);

      start = index + 1;
      continue;
    }
    if(input[index] >= '0' && input[index] <= '9' && index == start) {
      int temp_index = index + 1;

      while(input[temp_index] >= '0' && input[temp_index] <= '9' && input[temp_index] != '\0') {
        temp_index++;
      }

      if(input[temp_index] == '>') {
        // build redirect token
        int length = temp_index - index + 1;
        Token token;
        token.value = malloc(length + 1);
        if(!token.value) {
          abort(); // Handle memory allocation failure
        }
        strncpy(token.value, &input[index], length);
        token.value[length] = '\0';
        token.type = TOKEN_REDIRECT_OUT;

        // append token
        append_token(&tokenArray, token);

        index = temp_index; // advance main index
        start = index + 1;
        continue;
      }
    }

    // otherwise, process normal tokens
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
      if(!token.value) {
        abort(); // Handle memory allocation failure
      }
      strncpy(token.value, &input[start], length);
      token.value[length] = '\0';
      token.type = TOKEN_TEXT;

      // append token
      append_token(&tokenArray, token);      

      start = index + 1;
      continue;
    } else if (input[index] == '"') {
      index++;
      start = index;
      while(input[index] != '"' && input[index] != '\0') {
        if(input[index] == '\\') {
          index++; // skip escape character
          if(input[index] == '\0') {
            error(ERROR_UNTERMINATED_QUOTE, "Double quote not terminated after escape");
          }
        }
        index++;
      } // go to final '"'
      if(input[index] == '\0') {
        error(ERROR_UNTERMINATED_QUOTE, "Double quote not terminated");
      }

      // postprocess token for escape sequences
      int length = index - start;
      char* arg = malloc(length + 1);
      if(!arg) {
        abort(); // Handle memory allocation failure
      }
      strncpy(arg, &input[start], length);
      arg[length] = '\0';
      char* token_value = postprocess_dq(arg);

      // determine length
      length = strlen(token_value);

      // build token
      Token token;
      token.value = malloc(length + 1);
      if(!token.value) {
        abort(); // Handle memory allocation failure
      }
      strcpy(token.value, token_value);
      token.type = TOKEN_TEXT;

      // append token
      append_token(&tokenArray, token);      

      free(token_value);

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
    } else if (input[index] == '\\') {
      // include next literal
      index++; // consume '\'

      if(input[index] == '\0') {
        error(ERROR_TOKENIZATION_FAILED, "Escape character at end of input");
      } else {
        // build token
        Token token;
        token.value = malloc(2);
        if(!token.value) {
          abort(); // Handle memory allocation failure
        }
        token.value[0] = input[index];
        token.value[1] = '\0';
        token.type = TOKEN_TEXT;

        // append token
        append_token(&tokenArray, token);

        start = index + 1;
        continue;
      }
    } else {
      while(input[index + 1] != ' ' 
            && input[index + 1] != '\0'
            && input[index + 1] != '\''
            && input[index + 1] != '"'
            && input[index + 1] != '\\'
            && input[index + 1] != '>') {
        index++;
      } // go to end of token
      
      // determine length
      int length = index - start + 1;

      // build token
      Token token;
      token.value = malloc(length + 1);
      if(!token.value) {
        abort(); // Handle memory allocation failure
      }
      strncpy(token.value, &input[start], length);
      token.value[length] = '\0';
      token.type = TOKEN_TEXT;

      // append token
      append_token(&tokenArray, token);

      start = index + 1;
      continue;
    }

    error(ERROR_TOKENIZATION_FAILED, "An unknown tokenization error occurred");
  }

  append_token(&tokenArray, (Token){NULL, TOKEN_EOL});
  return tokenArray;
}
