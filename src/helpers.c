#include <unistd.h>
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

#include "helpers.h"
#include "tokenize.h"

void clear_screen(void) {
    write(1, "\033[2J\033[H", 7);
}

void write_prompt(void) {
    char prompt[2] = {'$', ' '};
    write(1, prompt, sizeof(prompt));
}

void builtin_type(char* command) {
    char buf[256];
    int len = snprintf(buf, sizeof(buf), "%s is a shell builtin\n", command);
    write(1, buf, len);
}

void unknown_type(char* command) {
    char buf[256];
    int len = snprintf(buf, sizeof(buf), "%s: not found\n", command);
    write(1, buf, len);
}

char* get_input(void){
    char *command = NULL;
    size_t cap = 0;

    ssize_t read = getline(&command, &cap, stdin);
    if (read > 0 && command[read - 1] == '\n') {
      command[read - 1] = '\0';
      read--;
    }

    if(read <= 0) {
        return NULL;
    }

    return command;
}

char** decompose_args(TokenArray tokens, int* out_count) {
    int count = 1; // include command itself

    for(int i = 1; i < tokens.count; i++) {
        if(tokens.tokens[i].type == TOKEN_ARGUMENT) {
            count++;
        }
    }

    char** args = malloc(sizeof(char*) * (count + 1));
    args[0] = tokens.tokens[0].value; // command itself

    int index = 1;
    for(int i = 1; i < tokens.count; i++) {
        if(tokens.tokens[i].type == TOKEN_ARGUMENT) {
            args[index++] = tokens.tokens[i].value;
        }
    }
    args[index] = NULL;

    *out_count = count;
    return args;
}

char* build_full_path(const char* directory, const char* command) {
    char buf[1024];
    snprintf(buf, sizeof(buf), "%s/%s", directory, command);

    char* full_path = strcpy(malloc(strlen(buf) + 1), buf);
    return full_path;
}
