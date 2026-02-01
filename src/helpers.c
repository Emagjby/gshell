#include <unistd.h>
#include <stdlib.h>
#include <stdio.h>

#include "helpers.h"

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
        command[0] = '\0';
    }

    return command;
}


int is_empty(char* input) {
    if(input[0] == '\0') {
        free(input);
        return 1;
    }
    return 0;
}
