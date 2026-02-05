#ifndef REDIRECT_H_
#define REDIRECT_H_

#include "command.h"
#include "tokenizer.h"

void handle_redirect(Command* command, TokenArray* tokens, int* index, int* start, Token* redirect_token);

#endif
