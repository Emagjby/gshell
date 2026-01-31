#ifndef HELPERS_H_
#define HELPERS_H_

void clear_screen(void);
void write_prompt(void);
char* get_input(void);

// type command helpers
void builtin_type(char* command);
void unknown_type(char* command);

#endif
