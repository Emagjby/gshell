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

TokenType categorizeToken() {
  return TOKEN_TEXT;
}

void postprocess_dq_token(char* token) {
  int read_index = 0;

  DynBuf dynbuf;
  dynbuf_init(&dynbuf);

  while(token[read_index] != '\0') {
    if(token[read_index] == '\\') {
      read_index++; // consume '\'
      if(token[read_index] == '\\') {
        dynbuf_append(&dynbuf, "\\");
        read_index++;
      } else if (token[read_index] == '"') {
        dynbuf_append(&dynbuf, "\"");
        read_index++;
      } else {
        // unrecognized escape, treat literally
        dynbuf_append(&dynbuf, "\\");
        if(token[read_index] != '\0') {
          char temp[2] = {token[read_index], '\0'};
          dynbuf_append(&dynbuf, temp);
          read_index++;
        }
      }
    } else {
      char temp[2] = {token[read_index], '\0'};
      dynbuf_append(&dynbuf, temp);
      read_index++;
    }
  }

  // finalize processed token
  free(token);
  token = malloc(dynbuf.len + 1);
  if(!token) {
    abort(); // Handle memory allocation failure
  }
  memcpy(token, dynbuf.buf, dynbuf.len);
  token[dynbuf.len] = '\0';
  dynbuf_free(&dynbuf);
}

/**
 * Split an input string into tokens (text, whitespace)
 * and return them as a TokenArray.
 *
 * @param input Null-terminated string to tokenize.
 * @returns A TokenArray containing the parsed tokens; the array is terminated by a token
 *          with type `TOKEN_EOL` and `value == NULL`.
 *
 * On encountering an unterminated single or double quote or an internal tokenization
 * failure, the function frees any allocated tokens and reports an error via `error(...)`
 * with `ERROR_UNTERMINATED_QUOTE` or `ERROR_TOKENIZATION_FAILED` respectively.
 */
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
    if(input[index] == '\'') {
      index++;
      start = index;
      while(input[index] != '\'' && input[index] != '\0') {
        index++;
      } // go to final ''\'
      if(input[index] == '\0') {
        free_token_array(&tokenArray);
        error(ERROR_UNTERMINATED_QUOTE, "Single quote not terminated");
      }

      // determine length
      int length = index - start;

      // build token
      Token token;
      token.value = malloc(length + 1);
      strncpy(token.value, &input[start], length);
      token.value[length] = '\0';
      token.type = categorizeToken();

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
            free_token_array(&tokenArray);
            error(ERROR_UNTERMINATED_QUOTE, "Double quote not terminated after escape");
          }
        }
        index++;
      } // go to final '"'
      if(input[index] == '\0') {
        free_token_array(&tokenArray);
        error(ERROR_UNTERMINATED_QUOTE, "Double quote not terminated");
      }

      // determine length
      int length = index - start;

      // build token
      Token token;
      token.value = malloc(length + 1);
      strncpy(token.value, &input[start], length);
      token.value[length] = '\0';
      token.type = categorizeToken();

      // postprocess token for escape sequences
      postprocess_dq_token(token.value);

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
    } else if (input[index] == '\\') {
      // include next literal
      index++; // consume '\'

      if(input[index] == '\0') {
        free_token_array(&tokenArray);
        error(ERROR_TOKENIZATION_FAILED, "Escape character at end of input");
      } else {
        // build token
        Token token;
        token.value = malloc(2);
        token.value[0] = input[index];
        token.value[1] = '\0';
        token.type = categorizeToken();

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
            && input[index + 1] != '\\') {
        index++;
      } // go to end of token
      
      // determine length
      int length = index - start + 1;

      // build token
      Token token;
      token.value = malloc(length + 1);
      strncpy(token.value, &input[start], length);
      token.value[length] = '\0';
      token.type = categorizeToken();

      // append token
      append_token(&tokenArray, token);

      start = index + 1;
      continue;
    }

    free_token_array(&tokenArray);
    error(ERROR_TOKENIZATION_FAILED, "An unknown tokenization error occurred");
  }

  append_token(&tokenArray, (Token){NULL, TOKEN_EOL});
  return tokenArray;
}
