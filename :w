#include <string.h>
#include <stdlib.h>

#include "history.h"
#include "linenoise.h"

static void remove_leading_whitespace(char* line) {
    char* start = line;
    while (*start && (*start == ' ' || *start == '\t')) {
        start++;
    }
    if (start != line) {
        memmove(line, start, strlen(start) + 1);
    }
}

void history_init(void) {
    linenoiseHistorySetMaxLen(HISTORY_SIZE);
}

void history_add(const char* line) {
    char* trimmed_line = strdup(line);
    if(!line) {
        return;
    }
    remove_leading_whitespace(trimmed_line);
    linenoiseHistoryAdd(trimmed_line);
    free(trimmed_line);
}
