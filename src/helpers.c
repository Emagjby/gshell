#include <unistd.h>
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

#include "helpers.h"
#include "error.h"

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

int is_builtin_command(const char* command) {
    const char* builtins[] = {"cd", "exit", "clear", "type", "echo", "pwd", NULL};
    for (int i = 0; builtins[i] != NULL; i++) {
        if (strcmp(command, builtins[i]) == 0) {
            return 1;
        }
    }
    return 0;
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

char* build_full_path(const char* directory, const char* command) {
    char buf[1024];
    snprintf(buf, sizeof(buf), "%s/%s", directory, command);

    char* full_path = strcpy(malloc(strlen(buf) + 1), buf);
    return full_path;
}

void handle_home(char** path) {
    if((*path)[0] == '~') {
        const char* home = getenv("HOME");
        if(home) {
            char buf[1024];
            snprintf(buf, sizeof(buf), "%s%s", home, (*path) + 1);
            free(*path);
            *path = strcpy(malloc(strlen(buf) + 1), buf);
            return;
        }
        error(ERROR_ENVIRONMENT_VARIABLE_NOT_SET, "HOME");
    }
}
