#define _POSIX_C_SOURCE 200809L
#include <sys/wait.h>

#include <stdio.h>
#include <fcntl.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "dynbuf.h"
#include "error.h"
#include "fs.h"

int check_directory(const char* directory, const char* command) {
    DynBuf dynbuf; 
    dynbuf_init(&dynbuf);

    dynbuf_append(&dynbuf, directory);
    dynbuf_append(&dynbuf, "/");
    dynbuf_append(&dynbuf, command);

    if (access(dynbuf.buf, X_OK) == 0) {
        dynbuf_free(&dynbuf);
        return 1; 
    } else {
        dynbuf_free(&dynbuf);
        return 0;
    }
}

int count_dirs(const char* path_env) {
    int count = 1; 
    for (int i = 0; path_env[i] != '\0'; i++) {
        if (path_env[i] == ':') {
            count++;
        }
    }
    return count;
}

char** decompose_path(const char* path_env) {
    int start = 0;
    int count = count_dirs(path_env);
    int index = 0;
    char** directories = malloc(sizeof(char*) * (count + 1));

    for (int i = 0; path_env[i] != '\0'; i++) {
        if (path_env[i] == ':' || path_env[i + 1] == '\0') {
            int length = i - start;
            char* dir = malloc(length + 1);
            strncpy(dir, &path_env[start], length);
            dir[length] = '\0';
            start = i + 1;

            // Append to directories array
            directories[index++] = dir;
        }
    }
    directories[index] = NULL;

    return directories;
}

void free_directories(char** directories, int count) {
    for (int i = 0; i < count; i++) {
        free(directories[i]);
    }
    free(directories);
}

char* check_path_directories(const char* command) {
    char* path_env = getenv("PATH");
    if (path_env == NULL) {
        return NULL;
    }

    char** directories = decompose_path(path_env);
    char* result = NULL;

    for(int count = 0; directories[count] != NULL; count++) {
        if(check_directory(directories[count], command)) {
            result = strcpy(malloc(strlen(directories[count]) + 1), directories[count]);
            break;
        }
    }

    free_directories(directories, count_dirs(path_env));
    return result;
}

void run_program(const char* path, char** argv){
    pid_t pid = fork();

    if (pid == 0) {
        execv(path, argv);
        perror("execv failed");
        exit(EXIT_FAILURE);
    } else if (pid > 0) {
        waitpid(pid, NULL, 0);
    } else {
        perror("fork");
    }
}

int redirect_stdout(const char* path) {
    int saved_stdout = dup(STDOUT_FILENO);
    if(saved_stdout < 0){
      error(ERROR_FILE_OPERATION_FAILED, "Failed to save stdout");
    }

    int fd = open(path, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd < 0) {
        close(saved_stdout);
        error(ERROR_FILE_OPERATION_FAILED, "Failed to open file for redirection");
    }

    if(dup2(fd, STDOUT_FILENO) < 0) {
        close(fd);
        close(saved_stdout);
        error(ERROR_FILE_OPERATION_FAILED, "Failed to redirect stdout");
    }

    close(fd);
    return saved_stdout;
}

void restore_stdout(int saved_stdout) {
    if(dup2(saved_stdout, STDOUT_FILENO) < 0) {
        close(saved_stdout);
        error(ERROR_FILE_OPERATION_FAILED, "Failed to restore stdout");
    }

    close(saved_stdout);
}
