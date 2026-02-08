#include <sys/stat.h>
#include <dirent.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

#include "multi_completions.h"
#include "dynbuf.h"
#include "helpers.h"
#include "rehash.h"
#include "fs.h"

void dedupe(char*** items, int* out_count) {
    if(*items == NULL) return;

    int count = 0;
    for(int i = 0; (*items)[i] != NULL; i++) {
        count++;
    }

    *out_count = count;
    char** unique_items = malloc(sizeof(char*) * (count + 1));
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

int check_multi_completions(char* buf, char*** items, int* out_count) {
    // Skip leading spaces
    char* start = buf;
    while(*start == ' ') start++;

    char* token_start = strrchr(start, ' ');
    char* token = token_start ? token_start + 1 : start;

    int is_first_token = (token_start == NULL);

    if(token[0] == '\0' && is_first_token) {
        *items = NULL;
        return (*out_count = 0);
    } else if(is_first_token) {
        // Check builtins and command_table for matches
        int count = 0;
        for(int i = 0; builtins[i]; i++) {
            if(strncmp(token, builtins[i], strlen(token)) == 0) {
                count++;
            }
        }
        for(int i = 0; command_table[i]; i++) {
            if(strncmp(token, command_table[i], strlen(token)) == 0) {
                count++;
            }
        }

        if(count > 1) {
            *items = malloc(sizeof(char*) * (count + 1));
            int index = 0;
            for(int i = 0; builtins[i]; i++) {
                if(strncmp(token, builtins[i], strlen(token)) == 0) {
                    (*items)[index++] = strdup(builtins[i]);
                }
            }
            for(int i = 0; command_table[i]; i++) {
                if(strncmp(token, command_table[i], strlen(token)) == 0) {
                    (*items)[index++] = strdup(command_table[i]);
                }
            }
            dedupe(items, out_count);

            if(*out_count == 1) {
                // Only one unique completion, free the array and return 0 to indicate no multi-completions
                free(*items);
                *items = NULL;
                return 0;
            }

            (*items)[index] = NULL; // Null-terminate the array
            return 1;
        }
    } else {
        *items = list_dir(".", out_count);

        if(*out_count > 1) {
            // Filter items based on the token
            int filtered_count = 0;
            for(int i = 0; (*items)[i] != NULL; i++) {
                if(strncmp(token, (*items)[i], strlen(token)) == 0) {
                    filtered_count++;
                } else {
                    free((*items)[i]);
                    (*items)[i] = NULL;
                }
            }

            if(filtered_count == 0) {
                free(*items);
                *items = NULL;
                return 0;
            }

            *out_count = filtered_count;
            return 1;
        } else {
            free(*items);
            *items = NULL;
            return 0;
        }
    }

    return 0;
}
