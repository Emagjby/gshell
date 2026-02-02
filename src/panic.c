#include <stdio.h>
#include <setjmp.h>

jmp_buf panic_env;

void panic(void) {
    longjmp(panic_env, 1);
}
