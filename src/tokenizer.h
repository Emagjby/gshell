#ifndef TOKENIZER_H_
#define TOKENIZER_H_

#include <stdlib.h>

typedef enum {
    TOKEN_WHITESPACE,
    TOKEN_TEXT,
    TOKEN_REDIRECT_OUT,
    TOKEN_REDIRECT_APPEND,
    TOKEN_PIPE,
    TOKEN_EOL
} TokenType;

typedef struct {
    char* value;
    TokenType type;
} Token;

typedef struct {
    Token* tokens;
    size_t count;
    size_t cap;
} TokenArray;

TokenArray tokenize(const char* input);
void free_token_array(TokenArray* tokenArray);

#endif
