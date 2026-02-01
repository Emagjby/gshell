#ifndef TOKENIZE_H_
#define TOKENIZE_H_

typedef enum {
    TOKEN_COMMAND,
    TOKEN_ARGUMENT,
    TOKEN_EMPTY
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

void tokenize(char* input, TokenArray* tokenArray);
void freeTokenArray(TokenArray* tokenArray);
 
#endif 
