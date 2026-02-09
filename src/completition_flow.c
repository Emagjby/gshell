#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#include "completition_flow.h"
#include "helpers.h"

void remove_trailing_whitespaces(char*** items) {
    for(int i = 0; (*items)[i] != NULL; i++) {
        size_t len = strlen((*items)[i]);
        while(len > 0 && (*items)[i][len - 1] == ' ') {
            (*items)[i][len - 1] = '\0';
            len--;
        }
    }
}

int filter_by_lcp(char **candidates, const char *lcp) {
    int w = 0;
    size_t len = strlen(lcp);

    for (int r = 0; candidates[r]; r++) {
        if (strncmp(candidates[r], lcp, len) == 0) {
            candidates[w++] = candidates[r];
        } else {
            free(candidates[r]);
        }
    }

    candidates[w] = NULL;
    return w; // new count
}

int cmp_str_len(const void* a, const void* b) {
    const char* str_a = *(const char**)a;
    const char* str_b = *(const char**)b;
    return strcmp(str_a, str_b);
}

char** apply_completion_flow(
        const char* buf, 
        size_t candidates_len, 
        char** candidates, 
        size_t* out_count,
        int* has_lcps
) {
    // Copy candidates to a new array that we can modify without affecting the original
    *out_count = candidates_len;
    char** copy = malloc(sizeof(char*) * (candidates_len + 1));

    for(size_t i = 0; i < candidates_len; i++) {
        copy[i] = strdup(candidates[i]);
    }
    copy[candidates_len] = NULL; // Null-terminate the array

    // dedupe the candidates to remove duplicates
    dedupe(&copy, out_count);

    // apply lcp calculation to the candidates
    int filtered = filter_by_lcp(copy, buf);
    *out_count = filtered;
    if(filtered > 0) {
        *has_lcps = 1;
    } else {
        *has_lcps = 0;
    }

    if(*has_lcps) {
        qsort(copy, *out_count, sizeof(char*), cmp_str_len);
        remove_trailing_whitespaces(&copy);
    }

    return copy;
}
