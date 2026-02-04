#ifndef TOKENIZER_H_
#define TOKENIZER_H_

typedef enum {
    TOKEN_WHITESPACE,
    TOKEN_TEXT,
    TOKEN_REDIRECT_OUT,
    TOKEN_EOL
} TokenType;

typedef struct {
    char* value;
    TokenType type;
} Token;

typedef struct {
    Token* tokens;
    int count;
    int cap;
} TokenArray;

TokenArray tokenize(const char* input);
void free_token_array(TokenArray* tokenArray);

#endif
