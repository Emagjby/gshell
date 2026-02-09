#ifndef COMPLETITION_FLOW_H_
#define COMPLETITION_FLOW_H_

#include <stddef.h>

char** apply_completion_flow(
        const char* buf, 
        size_t candidates_len, 
        char** candidates, 
        size_t* out_count,
        int* has_lcps
);

#endif
