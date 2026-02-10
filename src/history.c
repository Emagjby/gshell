#include "history.h"
#include "linenoise.h"

void history_init(void) {
    linenoiseHistorySetMaxLen(HISTORY_SIZE);
}

void history_add(const char* line) {
    linenoiseHistoryAdd(line);
}
