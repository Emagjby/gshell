#include "linenoise.h"
#include "dynbuf.h"
#include "linenoise_setup.h"
#include "helpers.h"

#include <string.h>

static void completion_callback(const char* buf, linenoiseCompletions* lc) {
    const char* token = strrchr(buf, ' ');
    token = token ? token + 1 : buf;

    for(int i = 0; builtins[i]; i++) {
        if(strncmp(token, builtins[i], strlen(token)) == 0) {
            DynBuf dynbuf;
            dynbuf_init(&dynbuf);
            dynbuf_append(&dynbuf, builtins[i]);
            dynbuf_append(&dynbuf, " ");

            linenoiseAddCompletion(lc, dynbuf.buf);
            dynbuf_free(&dynbuf);
        }
    }
}

void repl_linenoise_init(void) {
    linenoiseSetCompletionCallback(completion_callback);
}

char* repl_readline(const char* prompt) {
  return linenoise(prompt);
}
