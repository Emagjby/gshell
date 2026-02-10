#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#include "parser.h"
#include "pipeline.h"
#include "redirect.h"
#include "argvec.h"
#include "tokenizer.h"
#include "dynbuf.h"

char* build_argument(TokenArray* tokens, size_t start, size_t end) {
    DynBuf dynbuf;
    dynbuf_init(&dynbuf);

    for (size_t i = start; i < end; i++) {
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

Command parse_command(TokenArray* tokens, size_t start, size_t end) {
    ArgVec argv;
    argv.count = 0;
    argv.cap = 8;
    argv.args = malloc(sizeof(char*) * argv.cap);
    if(!argv.args) {
        abort(); // Handle memory allocation failure
    }

    Command command;
    command.stdout_path = NULL;
    command.stderr_path = NULL;
    command.stdout_append = NULL;
    command.stderr_append = NULL;

    size_t index = start;

    for (; index < end; index++){
        Token token = tokens->tokens[index];

        if(token.type == TOKEN_REDIRECT_APPEND || token.type == TOKEN_REDIRECT_OUT) {
            if (start < index) {
                char* arg = build_argument(tokens, start, index);
                append_arg(&argv, arg);
                free(arg);
            }

            handle_redirect(&command, tokens, &index, &start, &token);
            continue;
        }

        if(token.type == TOKEN_WHITESPACE || token.type == TOKEN_EOL || token.type == TOKEN_PIPE) {
            if(start == index) {
                start = index + 1;
                continue;
            }

            // build argument from tokens[start] to tokens[index - 1]
            char* arg = build_argument(tokens, start, index);

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
        char* arg = build_argument(tokens, start, index);
        append_arg(&argv, arg);
        free(arg);
    }

    // Null-terminate the argument list
    append_arg_end(&argv);

    command.argv = argv;

    return command;
}

Pipeline parse(TokenArray tokens) {
    // initialize pipeline with empty commands
    Pipeline pipeline;
    pipeline.count = 0;
    pipeline.commands = NULL;

    // walk over tokenarray
    size_t index = 0;
    size_t start = index;
    for(; index < tokens.count; index++) {
        Token token = tokens.tokens[index];

        if(token.type == TOKEN_PIPE || token.type == TOKEN_EOL) {
            // skip empty command 
            if(start == index) {
                start = index + 1;
                continue;
            }
            // parse command from tokens[start] to tokens[index - 1]
            Command command = parse_command(&tokens, start, index);

            // append command to pipeline
            append_command(&pipeline, command);

            // update start to next token
            start = index + 1;
        }
    }

    // Handle last command if any (just in case no trailing eol, which shouldn't happen)
    if(start < index) {
        Command command = parse_command(&tokens, start, index);
        append_command(&pipeline, command);
    }

    return pipeline;
}
