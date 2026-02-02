#ifndef DYNBUF_H_
#define DYNBUF_H_

typedef struct {
    char* buf;
    size_t len;
    size_t cap;
} DynBuf;

void dynbuf_init(DynBuf* dynbuf);
void dynbuf_free(DynBuf* dynbuf);
void dynbuf_append(DynBuf* dynbuf, const char* data);

#endif
