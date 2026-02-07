#include <string.h>
#include <stdlib.h>
#include <stdio.h>

#include "linenoise.h"
#include "rehash.h"
#include "dynbuf.h"
#include "linenoise_setup.h"
#include "helpers.h"

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
        complete_from_table(token, prefix, lc, command_table);
    }

    free(prefix);
}

void repl_linenoise_init(void) {
    linenoiseSetCompletionCallback(completion_callback);
}

char* repl_readline(const char* prompt) {
  return linenoise(prompt);
}
