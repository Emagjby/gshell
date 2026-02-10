#include <stdlib.h>
#include <string.h>

#include "error.h"
#include "redirect.h"
#include "command.h"
#include "tokenizer.h"

void handle_redirect(Command* command, TokenArray* tokens, size_t* index, size_t* start, Token* redirect_token) {
    // find path token
    size_t path_index = *index + 1;
    while(path_index < tokens->count && tokens->tokens[path_index].type == TOKEN_WHITESPACE) {
        path_index++;
    }
    if(path_index >= tokens->count || tokens->tokens[path_index].type != TOKEN_TEXT) {
        error(ERROR_PARSING_FAILED, "Expected file path after redirect operator");
    }

    char* path = strdup(tokens->tokens[path_index].value);
    if(!path) {
        abort(); // Handle memory allocation failure
    }

    if(strcmp(redirect_token->value, "1>>") == 0 || strcmp(redirect_token->value, ">>") == 0) {
        // advance index to skip path token
        *index = path_index;

        // store redirect info in command
        if(command->stdout_path) {
            free(command->stdout_path); 
            command->stdout_path = NULL;
        }
        if(command->stdout_append) {
            free(command->stdout_append); 
            command->stdout_append = NULL;
        }
        command->stdout_append = path;

        // update start to next token
        *start = *index + 1; // skip whitespace token
    } else if(strcmp(redirect_token->value, "2>>") == 0) {
        // advance index to skip path token
        *index = path_index;

        // store redirect info in command
        if(command->stderr_path) {
            free(command->stderr_path); 
            command->stderr_path = NULL;
        }
        if(command->stderr_append) {
            free(command->stderr_append); 
            command->stderr_append = NULL;
        }
        command->stderr_append = path;

        // update start to next token
        *start = *index + 1; // skip whitespace token
    } else if(strcmp(redirect_token->value, ">") == 0 || strcmp(redirect_token->value, "1>") == 0) {
        // advance index to skip path token
        *index = path_index;

        // store redirect info in command
        if(command->stdout_path) {
            free(command->stdout_path); 
            command->stdout_path = NULL;
        }
        command->stdout_path = path;

        *start = *index + 1; // skip whitespace token
    } else if(strcmp(redirect_token->value, "2>") == 0) {
        // advance index to skip path token
        *index = path_index;

        // store redirect info in command
        if(command->stderr_path) {
            free(command->stderr_path); 
            command->stderr_path = NULL;
        }
        command->stderr_path = path;

        *start = *index + 1; // skip whitespace token
    } else {
        error(ERROR_PARSING_FAILED, "Invalid redirect operator");
    }
}
