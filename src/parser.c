#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#include "parser.h"
#include "argvec.h"
#include "tokenizer.h"
#include "dynbuf.h"

char* build_argument(TokenArray* tokens, int start, int end) {
    DynBuf dynbuf;
    dynbuf_init(&dynbuf);

    for (int i = start; i < end; i++) {
        Token token = tokens->tokens[i];
        if (token.type == TOKEN_TEXT) {
            dynbuf_append(&dynbuf, token.value);
        }
    }

    // allocate argument string
    char* arg = malloc(dynbuf.len + 1);
    if(!arg) {
        abort(); // Handle memory allocation failure
    }
    memcpy(arg, dynbuf.buf, dynbuf.len);
    arg[dynbuf.len] = '\0';
    
    // free dynamic buffer
    dynbuf_free(&dynbuf);

    return arg;
}

ArgVec parse(TokenArray tokens) {
    ArgVec argv;
    argv.count = 0;
    argv.cap = 8;
    argv.args = malloc(sizeof(char*) * argv.cap);
    if(!argv.args) {
        abort(); // Handle memory allocation failure
    }

    int index = 0;
    int start = index;

    for (; index < tokens.count; index++){
        Token token = tokens.tokens[index];

        if(token.type == TOKEN_WHITESPACE || token.type == TOKEN_EOL) {
            if(start == index) {
                start = index + 1;
                continue;
            }

            // build argument from tokens[start] to tokens[index - 1]
            char* arg = build_argument(&tokens, start, index);

            // append argument to argv
            append_arg(&argv, arg);
            free(arg);

            // update start to next token
            start = index + 1;

            continue;
        }

        if (token.type == TOKEN_TEXT) {
            // continue to next token
            continue;
        }
    } 

    // Handle last argument if any (just in case no trailing eol, which shouldn't happen)
    if (start < index) {
        char* arg = build_argument(&tokens, start, index);
        append_arg(&argv, arg);
        free(arg);
    }

    // Null-terminate the argument list
    append_arg_end(&argv);

    free_token_array(&tokens);
    return argv;
}
