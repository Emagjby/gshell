#include <string.h>
#include <stdlib.h>
#include <stdio.h>

#include "linenoise.h"
#include "rehash.h"
#include "dynbuf.h"
#include "linenoise_setup.h"
#include "helpers.h"
#include "fs.h"

static void collapse_slashes(char* s) {
    char* w = s;
    for(char* r = s; *r; r++) {
        if (*r == '/' && w > s && w[-1] == '/') {
            continue;
        }
        *w++ = *r;
    }
    *w = 0;
}

static void complete_from_filesystem(const char* token, const char* prefix, linenoiseCompletions* lc) {
    const char* last_slash = strrchr(token, '/');

    int dir_allocated = 0;
    const char* dir;
    if (!last_slash) {
        dir = ".";
    } else if(last_slash == token) {
        dir = "/";
    } else {
        dir = strndup(token, last_slash - token);
        dir_allocated = 1;
    }

    const char* file_prefix = last_slash ? last_slash + 1 : token;

    size_t count = -1;
    char** items = list_dir(dir, &count);
    if(items) {
        for(int i = 0; items[i]; i++) {
            const char* name = items[i];
            if (strcmp(dir, ".") != 0) {
                size_t dir_len = strlen(dir);
                if (strncmp(items[i], dir, dir_len) == 0 && items[i][dir_len] == '/') {
                    name = items[i] + dir_len + 1;
                }
            }
            if(strncmp(file_prefix, name, strlen(file_prefix)) == 0) {
                DynBuf dynbuf;
                dynbuf_init(&dynbuf);
                
                dynbuf_append(&dynbuf, prefix);
                if(last_slash) {
                    dynbuf_append(&dynbuf, dir);
                    if(dir[strlen(dir) - 1] != '/') {
                        dynbuf_append(&dynbuf, "/");
                    }
                }

                dynbuf_append(&dynbuf, name);
                size_t name_len = strlen(name);
                if(name_len == 0 || name[name_len - 1] != '/') {
                    dynbuf_append(&dynbuf, " ");
                }

                collapse_slashes(dynbuf.buf);

                linenoiseAddCompletion(lc, dynbuf.buf);
                dynbuf_free(&dynbuf);
            }
            free(items[i]);
        }
        free(items);
    }

    if(dir_allocated) {
        free((char*)dir);
    }
}

static void complete_from_table(const char* token, const char* prefix, linenoiseCompletions* lc, const char* const* table) { for(int i = 0; table[i]; i++) {
        if(strncmp(token, table[i], strlen(token)) == 0) {
            DynBuf dynbuf;
            dynbuf_init(&dynbuf);
            
            dynbuf_append(&dynbuf, prefix);

            dynbuf_append(&dynbuf, table[i]);
            dynbuf_append(&dynbuf, " ");

            linenoiseAddCompletion(lc, dynbuf.buf);
            dynbuf_free(&dynbuf);
        }
    }
}

static void completion_callback(const char* buf, linenoiseCompletions* lc) {
    // Skip leading spaces
    const char* start = buf;
    while(*start == ' ') start++;

    const char* token_start = strrchr(start, ' ');
    const char* token = token_start ? token_start + 1 : start;

    int is_first_token = (token_start == NULL);

    size_t prefix_len = token - buf;
    char* prefix = strndup(buf, prefix_len);

    if(is_first_token) {
        complete_from_table(token, prefix, lc, builtins);
        if(command_table) complete_from_table(token, prefix, lc, command_table);
    } else {
        complete_from_filesystem(token, prefix, lc);    
    }

    free(prefix);
}

void repl_linenoise_init(void) {
    linenoiseSetCompletionCallback(completion_callback);
}

char* repl_readline(const char* prompt) {
  return linenoise(prompt);
}
