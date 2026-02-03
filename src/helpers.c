#include <unistd.h>
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

#include "helpers.h"
#include "argvec.h"
#include "error.h"
#include "dynbuf.h"

void clear_screen(void) {
    write(1, "\033[2J\033[H", 7);
}

void write_prompt(void) {
    char prompt[2] = {'$', ' '};
    write(1, prompt, sizeof(prompt));
}

void builtin_type(char* command) {
    DynBuf dynbuf;
    dynbuf_init(&dynbuf);

    dynbuf_append(&dynbuf, command);
    dynbuf_append(&dynbuf, " is a shell builtin\n");

    write(1, dynbuf.buf, dynbuf.len);
    dynbuf_free(&dynbuf);
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
    DynBuf dynbuf;
    dynbuf_init(&dynbuf);

    dynbuf_append(&dynbuf, directory);
    dynbuf_append(&dynbuf, "/");
    dynbuf_append(&dynbuf, command);

    char* full_path = strcpy(malloc(dynbuf.len + 1), dynbuf.buf);
    dynbuf_free(&dynbuf);
    return full_path;
}

void unknown_type(char* command) {
    DynBuf dynbuf;
    dynbuf_init(&dynbuf);

    dynbuf_append(&dynbuf, command);
    dynbuf_append(&dynbuf, ": not found\n");

    write(1, dynbuf.buf, dynbuf.len);
    dynbuf_free(&dynbuf);
}

void handle_home(char** path, ArgVec* argv) {
    if((*path)[0] == '~') {
        const char* home = getenv("HOME");
        if(home) {
            DynBuf dynbuf;
            dynbuf_init(&dynbuf);

            dynbuf_append(&dynbuf, home);
            dynbuf_append(&dynbuf, (*path) + 1);

            free(*path);
            *path = strcpy(malloc(strlen(dynbuf.buf) + 1), dynbuf.buf);

            dynbuf_free(&dynbuf);
            return;
        }

        free_argvec(argv);
        free(*path);
        error(ERROR_ENVIRONMENT_VARIABLE_NOT_SET, "HOME");
    }
}
