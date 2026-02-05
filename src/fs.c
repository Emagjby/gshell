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
            int length = (path_env[i] == ':')
                ? (i - start)
                : (i - start + 1);

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

static int redirect_fd(int target_fd, const char* path, RedirectType append) {
    int saved_fd = dup(target_fd);
    if(saved_fd < 0){
      error(ERROR_FILE_OPERATION_FAILED, "Failed to save file descriptor");
    }

    int flags = O_WRONLY | O_CREAT | (append == REDIRECT_APPEND ? O_APPEND : O_TRUNC);
    int fd = open(path, flags, 0644);
    if (fd < 0) {
        close(saved_fd);
        error(ERROR_FILE_OPERATION_FAILED, "Failed to open file for redirection");
    }

    if(dup2(fd, target_fd) < 0) {
        close(fd);
        close(saved_fd);
        error(ERROR_FILE_OPERATION_FAILED, "Failed to redirect file descriptor");
    }

    close(fd);
    return saved_fd;
}

void restore_fd(int saved_fd, int target_fd) {
    if(dup2(saved_fd, target_fd) < 0) {
        close(saved_fd);
        error(ERROR_FILE_OPERATION_FAILED, "Failed to restore file descriptor");
    }

    close(saved_fd);
}

int redirect_stdout(const char* path, RedirectType append) {
    return redirect_fd(STDOUT_FILENO, path, append);
}

void restore_stdout(int saved_stdout) {
    restore_fd(saved_stdout, STDOUT_FILENO);
}

int redirect_stderr(const char* path, RedirectType append) {
    return redirect_fd(STDERR_FILENO, path, append);
}

void restore_stderr(int saved_stderr) {
    restore_fd(saved_stderr, STDERR_FILENO);
}
