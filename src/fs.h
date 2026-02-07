#ifndef FS_H_
#define FS_H_

typedef enum {
    REDIRECT_OUT, 
    REDIRECT_APPEND
} RedirectType;

int count_dirs(const char* path_env);
void free_directories(char** directories, int count);
char** decompose_path(const char* path_env);
char* check_path_directories(const char* command);
void run_program(const char* path, char** argv);

int redirect_stdout(const char* path, RedirectType type);
void restore_stdout(int saved_stdout);
int redirect_stderr(const char* path, RedirectType type);
void restore_stderr(int saved_stderr);

#endif
