#ifndef PANIC_H_
#define PANIC_H_

#include <setjmp.h>

extern jmp_buf panic_env;

void panic(void);

#endif 
