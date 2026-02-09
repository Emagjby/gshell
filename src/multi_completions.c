#include <sys/stat.h>
#include <dirent.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

#include "multi_completions.h"
#include "helpers.h"
#include "rehash.h"
#include "fs.h"

int check_multi_completions(char* buf, char*** items, size_t* out_count) {
    // Skip leading spaces
    char* start = buf;
    while(*start == ' ') start++;

    char* token_start = strrchr(start, ' ');
    char* token = token_start ? token_start + 1 : start;

    int is_first_token = (token_start == NULL);

    *items = NULL;
    *out_count = 0;
    if(token[0] == '\0' && is_first_token) {
        return (*out_count = 0);
    } else if(is_first_token) {
        // Check builtins and command_table for matches
        int count = 0;
        for(int i = 0; builtins[i]; i++) {
            if(strncmp(token, builtins[i], strlen(token)) == 0) {
                count++;
            }
        }
        for(int i = 0; command_table && command_table[i]; i++) {
            if(strncmp(token, command_table[i], strlen(token)) == 0) {
                count++;
            }
        }

        if(count > 1) {
            *items = malloc(sizeof(char*) * (count + 1));
            if(*items == NULL) {
                *out_count = 0;
                return 0; 
            }
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
            (*items)[index] = NULL; // Null-terminate the array
            dedupe(items, out_count);

            if(*out_count == 1) {
                free((*items)[0]);
                free(*items);
                *items = NULL;
                return 0;
            }

            return 1;
        }
    } else {
        *items = list_dir(".", out_count);
        if(*items == NULL) {
            *out_count = 0;
            return 0;
        }
        int write = 0;

        for (int read = 0; (*items)[read] != NULL; read++) {
            if (strncmp(token, (*items)[read], strlen(token)) == 0) {
                (*items)[write++] = (*items)[read];
            } else {
                free((*items)[read]);
            }
        }

        (*items)[write] = NULL;
        *out_count = write;

        if (write == 0) {
            free(*items);
            *items = NULL;
            return 0;
        }

        return 1;
    }

    return 0;
}
