#include <sys/ioctl.h>
#include <unistd.h>
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

#include "helpers.h"
#include "error.h"
#include "dynbuf.h"

const char* builtins[] = {"cd", "exit", "clear", "type", "echo", "pwd", "history", NULL};

void clear_screen(void) {
    write(STDOUT_FILENO, "\033[2J\033[H", 7);
}

void write_prompt(void) {
    char prompt[2] = {'$', ' '};
    write(STDOUT_FILENO, prompt, sizeof(prompt));
}

void builtin_type(char* command) {
    DynBuf dynbuf;
    dynbuf_init(&dynbuf);

    dynbuf_append(&dynbuf, command);
    dynbuf_append(&dynbuf, " is a shell builtin\n");

    write(STDOUT_FILENO, dynbuf.buf, dynbuf.len);
    dynbuf_free(&dynbuf);
}

int is_builtin_command(const char* command) {
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

    write(STDOUT_FILENO, dynbuf.buf, dynbuf.len);
    dynbuf_free(&dynbuf);
}

void handle_home(char** path) {
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

        free(*path);
        error(ERROR_ENVIRONMENT_VARIABLE_NOT_SET, "HOME");
    }
}

static int ln_get_term_width(void) {
    struct winsize ws;
    if (ioctl(STDOUT_FILENO, TIOCGWINSZ, &ws) == -1) {
        return 80; // default width
    }

    if (ws.ws_col == 0)
        return 80;

    return ws.ws_col;
}

int cmp_str(const void* a, const void* b) {
    const char* str_a = *(const char**)a;
    const char* str_b = *(const char**)b;
    return strcmp(str_a, str_b);
}

void print_ln_grid(char **items, size_t count) {
    if (count == 0) return;

    qsort(items, count, sizeof(char*), cmp_str);

    size_t max = 0;
    for (size_t i = 0; i < count; i++) {
        size_t len = strlen(items[i]);
        if (len > max) max = len;
    }

    int term_width = ln_get_term_width();

    int col_width = max + 2; 
    int cols = term_width / col_width;
    if (cols < 1) cols = 1;

    char pad_buf[256];
    memset(pad_buf, ' ', sizeof(pad_buf));

    for (size_t i = 0; i < count; i++) {
        size_t slen = strlen(items[i]);
        write(STDOUT_FILENO, items[i], slen);

        size_t pad = col_width - slen;
        while(pad > 0){
            int chunk = pad < (int)sizeof(pad_buf) ? pad : (int)sizeof(pad_buf);
            write(STDOUT_FILENO, pad_buf, chunk);
            pad -= chunk;
        }

        if ((i + 1) % cols == 0)
            write(STDOUT_FILENO, "\r\n", 2);
    }

    if (count % cols != 0)
        write(STDOUT_FILENO, "\r\n", 2);
}

void dedupe(char*** items, size_t* out_count) {
    if(*items == NULL) return;

    int count = 0;
    for(int i = 0; (*items)[i] != NULL; i++) {
        count++;
    }

    *out_count = count;
    char** unique_items = malloc(sizeof(char*) * (count + 1));
    if(!unique_items) return;
    int index = 0;
    for(int i = 0; (*items)[i] != NULL; i++) {
        int is_duplicate = 0;
        for(int j = 0; j < index; j++) {
            if(strcmp((*items)[i], unique_items[j]) == 0) {
                is_duplicate = 1;
                *out_count = *out_count - 1;
                break;
            }
        }
        if(!is_duplicate) {
            unique_items[index++] = (*items)[i];
        } else {
            free((*items)[i]);
        }
    }
    unique_items[index] = NULL;

    free(*items);
    *items = unique_items;
}

int is_number(const char* str) {
    if (*str == '\0') return 0; 
    for (const char* p = str; *p; p++) {
        if (*p < '0' || *p > '9') {
            return 0; 
        }
    }
    return 1; 
}
