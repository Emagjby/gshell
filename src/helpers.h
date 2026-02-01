#ifndef HELPERS_H_
#define HELPERS_H_

#include "tokenize.h"

void clear_screen(void);
void write_prompt(void);
char* get_input(void);
int is_builtin_command(char* command);

// type command helpers
void builtin_type(char* command);
void unknown_type(char* command);

// run command helpers
char** decompose_args(TokenArray tokens, int* out_count);
char* build_full_path(const char* directory, const char* command);

#endif
