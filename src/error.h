#ifndef ERROR_H_
#define ERROR_H_

typedef enum {
    ERROR_COMMAND_NOT_FOUND,
    ERROR_INSUFFICIENT_ARGUMENTS,
    ERROR_CD_NO_SUCH_DIRECTORY,
    ERROR_ENVIRONMENT_VARIABLE_NOT_SET,
    ERROR_TOKENIZATION_FAILED,
    ERROR_UNTERMINATED_QUOTE,
    ERROR_PARSING_FAILED,
    ERROR_FILE_OPERATION_FAILED,
    ERROR_EXECUTE_ERROR,
    ERROR_INVALID_ARGUMENT
} ErrorType;

void error(ErrorType errorType, const char* details);
void error_no_panic(ErrorType errorType, const char* details);

#endif
