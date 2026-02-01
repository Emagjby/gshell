#ifndef COMMANDS_H_
#define COMMANDS_H_

void type_command(TokenArray* tokenArray);
void clear_command();
void echo_command(TokenArray* tokenArray);
void run_command(TokenArray* tokenArray, char* path);
void pwd_command(void);

#endif
