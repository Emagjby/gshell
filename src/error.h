#ifndef ERROR_H_
#define ERROR_H_

typedef enum {
    ERROR_COMMAND_NOT_FOUND,
} ErrorType;

void error(ErrorType errorType, const char* details);

#endif
