#include <string.h>
#include <stdlib.h>
#include <stdio.h>

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

static void history_load(void) {
    const char* histfile = getenv("HISTFILE");
    if (histfile) {
        linenoiseHistoryLoad(histfile);
    } else {
        // default to ~/.config/gshell/.history
        const char* home = getenv("HOME");
        if (home) {
            char path[1024];
            snprintf(path, sizeof(path), "%s/.config/gshell/.history", home);
            linenoiseHistoryLoad(path);
        }
    }
}

void history_init(void) {
    linenoiseHistorySetMaxLen(HISTORY_SIZE);
    history_load();
}

void history_save(void) {
    const char* histfile = getenv("HISTFILE");
    if (histfile) {
        linenoiseHistorySave(histfile);
    } else {
        // default to ~/.config/gshell/.history
        const char* home = getenv("HOME");
        if (home) {
            char path[1024];
            snprintf(path, sizeof(path), "%s/.config/gshell/.history", home);
            linenoiseHistorySave(path);
        }
    }
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
