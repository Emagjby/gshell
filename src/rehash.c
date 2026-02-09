#include <dirent.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "rehash.h"
#include "error.h"
#include "fs.h"

char** command_table_mut = NULL;
const char* const* command_table = NULL;

static void free_command_table(void) {
    if (command_table_mut) {
        for (int i = 0; command_table_mut[i] != NULL; i++) {
            free(command_table_mut[i]);
        }
        free(command_table_mut);
        command_table_mut = NULL;
    }
}

void rehash_command_table(void) {
    command_table = NULL;
    free_command_table();

    char* path_env = getenv("PATH");
    if (!path_env) {
        static const char* empty[] = {NULL};
        command_table = empty;
        error(ERROR_ENVIRONMENT_VARIABLE_NOT_SET, "PATH environment variable not found");
    }

    char** directories = decompose_path(path_env);
    int count = count_dirs(path_env);

    size_t cap = 128;
    size_t index = 0;
    command_table_mut = malloc(sizeof(char*) * cap);
    if(!command_table_mut) {
        free_directories(directories, count);
        return;
    }

    for (int i = 0; directories[i] != NULL; i++) {
        DIR* dp = opendir(directories[i]);
        if(!dp) continue;

        struct dirent* entry;
        while ((entry = readdir(dp)) != NULL) {
            if (entry->d_type == DT_REG || entry->d_type == DT_LNK) {
                if (index + 1 >= cap) {
                    cap *= 2;

                    char** tmp = realloc(command_table_mut, sizeof(char*) * cap);
                    if (tmp == NULL) {
                        closedir(dp);
                        command_table_mut[index] = NULL;
                        command_table = (const char* const*)command_table_mut;
                        free_directories(directories, count);
                        return;
                    }
                    command_table_mut = tmp;
                }

                command_table_mut[index++] = strdup(entry->d_name);
            }
        }
        closedir(dp);
    }
    command_table_mut[index] = NULL;

    command_table = (const char* const*)command_table_mut;

    free_directories(directories, count);
}
