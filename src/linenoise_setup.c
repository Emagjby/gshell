#include <string.h>
#include <stdlib.h>
#include <stdio.h>

#include "linenoise.h"
#include "dynbuf.h"
#include "linenoise_setup.h"
#include "helpers.h"

static void completion_callback(const char* buf, linenoiseCompletions* lc) {
    // Skip leading spaces
    const char* start = buf;
    while(*start == ' ') start++;

    const char* token_start = strrchr(start, ' ');
    const char* token = token_start ? token_start + 1 : start;

    int is_first_token = (token_start == NULL);

    size_t prefix_len = token - buf;
    char* prefix = strndup(buf, prefix_len);


    for(int i = 0; builtins[i]; i++) {
        if(is_first_token) {
            if(strncmp(token, builtins[i], strlen(token)) == 0) {
                DynBuf dynbuf;
                dynbuf_init(&dynbuf);
                
                dynbuf_append(&dynbuf, prefix);

                dynbuf_append(&dynbuf, builtins[i]);
                dynbuf_append(&dynbuf, " ");

                linenoiseAddCompletion(lc, dynbuf.buf);
                dynbuf_free(&dynbuf);
            }
        } 
    }

    free(prefix);
}

void repl_linenoise_init(void) {
    linenoiseSetCompletionCallback(completion_callback);
}

char* repl_readline(const char* prompt) {
  return linenoise(prompt);
}
