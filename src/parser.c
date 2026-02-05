#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#include "parser.h"
#include "argvec.h"
#include "tokenizer.h"
#include "error.h"
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

Command parse(TokenArray tokens) {
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

    int index = 0;
    int start = index;

    for (; index < tokens.count; index++){
        Token token = tokens.tokens[index];

        if(token.type == TOKEN_REDIRECT_APPEND) {
            if (start < index) {
                char* arg = build_argument(&tokens, start, index);
                append_arg(&argv, arg);
                free(arg);
            }

            // find next non-whitespace token for path
            int path_index = index + 1;
            while(path_index < tokens.count && tokens.tokens[path_index].type == TOKEN_WHITESPACE) {
                path_index++;
            }
            if(path_index >= tokens.count || tokens.tokens[path_index].type != TOKEN_TEXT) {
                error(ERROR_PARSING_FAILED, "Expected file path after redirect operator");
            }

            Token path_token = tokens.tokens[path_index];

            if(strcmp(token.value, "1>>") == 0 || strcmp(token.value, ">>") == 0) {
                // set stdout_append in command
                char* path = malloc(strlen(path_token.value) + 1);
                if(!path) {
                    abort(); // Handle memory allocation failure
                }
                strcpy(path, path_token.value);

                // advance index to skip path token
                index = path_index;

                // store redirect info in command
                if(command.stdout_append) {
                    free(command.stdout_append); 
                    command.stdout_append = NULL;
                }
                command.stdout_append = path;

                // update start to next token
                start = index + 1; // skip whitespace token
                continue;
            } else if(strcmp(token.value, "2>>") == 0) {
                // set stderr_path in command
                char* path = malloc(strlen(path_token.value) + 1);
                if(!path) {
                    abort(); // Handle memory allocation failure
                }
                strcpy(path, path_token.value);

                // advance index to skip path token
                index = path_index;

                // store redirect info in command
                if(command.stderr_append) {
                    free(command.stderr_append); 
                    command.stderr_append = NULL;
                }
                command.stderr_append = path;

                // update start to next token
                start = index + 1; // skip whitespace token
                continue;
            } else {
                error(ERROR_PARSING_FAILED, "Invalid redirect operator");
            }
        }

        if(token.type == TOKEN_REDIRECT_OUT) {
            if (start < index) {
                char* arg = build_argument(&tokens, start, index);
                append_arg(&argv, arg);
                free(arg);
            }

            // find next non-whitespace token for path
            int path_index = index + 1;
            while(path_index < tokens.count && tokens.tokens[path_index].type == TOKEN_WHITESPACE) {
                path_index++;
            }
            if(path_index >= tokens.count || tokens.tokens[path_index].type != TOKEN_TEXT) {
                error(ERROR_PARSING_FAILED, "Expected file path after redirect operator");
            }

            Token path_token = tokens.tokens[path_index];

            if(strcmp(token.value, ">") == 0 || strcmp(token.value, "1>") == 0) {
                // set stdout_path in command
                char* path = malloc(strlen(path_token.value) + 1);
                if(!path) {
                    abort(); // Handle memory allocation failure
                }
                strcpy(path, path_token.value);

                // advance index to skip path token
                index = path_index;

                // store redirect info in command
                if(command.stdout_path) {
                    free(command.stdout_path); 
                    command.stdout_path = NULL;
                }
                command.stdout_path = path;
            } else if(strcmp(token.value, "2>") == 0) {
                // set stderr_path in command
                char* path = malloc(strlen(path_token.value) + 1);
                if(!path) {
                    abort(); // Handle memory allocation failure
                }
                strcpy(path, path_token.value);

                // advance index to skip path token
                index = path_index;

                // store redirect info in command
                if(command.stderr_path) {
                    free(command.stderr_path); 
                    command.stderr_path = NULL;
                }
                command.stderr_path = path;
            } else {
                error(ERROR_PARSING_FAILED, "Invalid redirect operator");
            }

            // update start to next token
            start = index + 1; // skip whitespace token
            continue;
        }

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

    command.argv = argv;

    return command;
}
