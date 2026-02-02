#include <stdlib.h>
#include <string.h>

#include "dynbuf.h"

void dynbuf_init(DynBuf* dynbuf) {
    dynbuf->buf = (char*)malloc(256);
    dynbuf->len = 0;
    dynbuf->cap = 256;
}

void dynbuf_free(DynBuf* dynbuf) {
    free(dynbuf->buf);
    dynbuf->buf = NULL;
    dynbuf->len = 0;
    dynbuf->cap = 0;
}

void double_dynbuf_capacity(DynBuf* dynbuf) {
    size_t new_cap = dynbuf->cap * 2;
    char* new_buf = (char*)realloc(dynbuf->buf, new_cap);
    if (new_buf) {
        dynbuf->buf = new_buf;
        dynbuf->cap = new_cap;
    }
}

void dynbuf_append(DynBuf* dynbuf, const char* data) {
    size_t data_len = strlen(data);
    while (dynbuf->len + data_len >= dynbuf->cap) {
        double_dynbuf_capacity(dynbuf);
    }
    memcpy(dynbuf->buf + dynbuf->len, data, data_len);
    dynbuf->len += data_len;
    dynbuf->buf[dynbuf->len] = '\0'; 
}
