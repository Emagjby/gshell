#ifndef REDIRECT_H_
#define REDIRECT_H_

#include <stdlib.h>

#include "command.h"
#include "tokenizer.h"

void handle_redirect(Command* command, TokenArray* tokens, size_t* index, size_t* start, Token* redirect_token);

#endif
