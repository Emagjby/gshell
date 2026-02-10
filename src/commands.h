#ifndef COMMANDS_H_
#define COMMANDS_H_

#include "argvec.h"

void type_command(ArgVec argv);
void clear_command(void);
void echo_command(ArgVec argv);
void run_command(ArgVec argv, char* path);
void pwd_command(void);
void cd_command(ArgVec argv);
void history_command(ArgVec argv);

#endif
