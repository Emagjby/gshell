#ifndef FS_H_
#define FS_H_

char* check_path_directories(const char* command);
void run_program(const char* path, char** argv);

int redirect_stdout(const char* path);
void restore_stdout(int saved_stdout);

#endif
