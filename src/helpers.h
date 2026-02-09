#ifndef HELPERS_H_
#define HELPERS_H_

#include <stddef.h>

extern const char* builtins[];

void clear_screen(void);
void write_prompt(void);
char* get_input(void);
int is_builtin_command(const char* command);
void print_ln_grid(char** items, size_t count);
int cmp_str(const void* a, const void* b);
void dedupe(char*** items, size_t* out_count);

// type command helpers
void builtin_type(char* command);
void unknown_type(char* command);

// run command helpers
char* build_full_path(const char* directory, const char* command);

// cd command helpers
void handle_home(char** path);

#endif
